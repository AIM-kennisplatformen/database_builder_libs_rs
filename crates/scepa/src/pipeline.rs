use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use rootcause::{
    compat::boxed_error::IntoBoxedError,
    option_ext::OptionExt,
    prelude::{Report, ResultExt},
    report,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::Instrument;

pub mod error;
pub mod source;
pub mod storage;
pub mod tei;
pub mod typedb;

use crate::{
    Config,
    pipeline::source::grobid::GrobidClient,
    progress::{Progress, ProgressEvent},
    typedb::{Connected, TypeDbDriver},
};

use self::typedb::typeql_queries;
use error::FailureCause;
use storage::PdfStorage;

pub const TOTAL_STEPS: usize = 3;
pub const PARSED_FILES_ARTIFACTS_DIR: &str = "log/parsed_files";
pub const RAW_TEI_ARTIFACTS_DIR: &str = "log/raw_tei";
pub const PARSED_TEI_ARTIFACTS_DIR: &str = "log/parsed_tei";
pub const KNOWN_FAILURES_PATH: &str = "known_failures.json";
pub const FIXED_FAILURES_PATH: &str = "fixed_failures.json";
pub const FAILED_PDFS_DIR: &str = "sources/retry_pdfs";

#[derive(Debug, Clone, Deserialize, Eq, Hash, Ord, PartialEq, Serialize, PartialOrd)]
struct KnownFailure {
    hash: String,
    /// The source file name is used to select files for a retry run.
    original_file_name: String,
    cause: FailureCause,
    error: String,
}

#[derive(Clone)]
pub struct KnownFailures {
    path: PathBuf,
    fixed_path: PathBuf,
    state: Arc<Mutex<FailureState>>,
}

struct FailureState {
    failures: HashSet<KnownFailure>,
    fixed_failures: HashSet<KnownFailure>,
}

impl KnownFailures {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, Report> {
        let path = path.into();
        let fixed_path = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(FIXED_FAILURES_PATH);
        Self::load_paths(path, fixed_path)
    }

    fn load_paths(path: PathBuf, fixed_path: PathBuf) -> Result<Self, Report> {
        let failures = match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).context(format!(
                "failed to parse known failures file \x60{}\x60",
                path.display()
            ))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashSet::new(),
            Err(error) => {
                return Err(error)
                    .context(format!(
                        "failed to read known failures file \x60{}\x60",
                        path.display()
                    ))
                    .map_err(Into::into);
            }
        };
        let fixed_failures = match fs::read_to_string(&fixed_path) {
            Ok(contents) => serde_json::from_str(&contents).context(format!(
                "failed to parse fixed failures file \x60{}\x60",
                fixed_path.display()
            ))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashSet::new(),
            Err(error) => {
                return Err(error)
                    .context(format!(
                        "failed to read fixed failures file \x60{}\x60",
                        fixed_path.display()
                    ))
                    .map_err(Into::into);
            }
        };

        Ok(Self {
            path,
            fixed_path,
            state: Arc::new(Mutex::new(FailureState {
                failures,
                fixed_failures,
            })),
        })
    }

    pub fn load_default() -> Result<Self, Report> {
        Self::load_paths(
            PathBuf::from(KNOWN_FAILURES_PATH),
            PathBuf::from(FIXED_FAILURES_PATH),
        )
    }

    fn contains(&self, hash: &str, cause: FailureCause) -> Result<bool, Report> {
        let state = self
            .state
            .lock()
            .map_err(|_| report!("known failures lock was poisoned"))?;
        Ok(state
            .failures
            .iter()
            .any(|failure| failure.hash == hash && failure.cause == cause))
    }

    fn record(
        &self,
        hash: &str,
        original_file_name: &str,
        cause: FailureCause,
        error: String,
    ) -> Result<(), Report> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| report!("known failures lock was poisoned"))?;
        let failure = KnownFailure {
            hash: hash.to_owned(),
            original_file_name: original_file_name.to_owned(),
            cause,
            error,
        };

        // A failure is identified by its PDF, source file, and pipeline
        // stage, not by the diagnostic text.
        if let Some(existing) = state
            .failures
            .iter()
            .find(|existing| existing.hash == hash && existing.cause == cause)
            .cloned()
        {
            if existing.original_file_name == failure.original_file_name {
                return Ok(());
            }
            state.failures.remove(&existing);
        }

        let mut values = state.failures.iter().cloned().collect::<Vec<_>>();
        values.push(failure.clone());
        values.sort();
        let contents = serde_json::to_string_pretty(&values)
            .context("failed to serialize known PDF failures")?;
        fs::write(&self.path, format!("{contents}\n")).context(format!(
            "failed to write known failures file \x60{}\x60",
            self.path.display()
        ))?;
        state.failures.insert(failure);
        Ok(())
    }

    pub fn retry_file_names(&self) -> Result<HashSet<String>, Report> {
        let state = self
            .state
            .lock()
            .map_err(|_| report!("known failures lock was poisoned"))?;
        Ok(state
            .failures
            .iter()
            .filter(|failure| failure.cause != FailureCause::Duplicate)
            .map(|failure| failure.original_file_name.clone())
            .collect())
    }

    fn mark_fixed(&self, hash: &str, cause: FailureCause) -> Result<bool, Report> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| report!("known failures lock was poisoned"))?;
        let Some(failure) = state
            .failures
            .iter()
            .find(|failure| failure.hash == hash && failure.cause == cause)
            .cloned()
        else {
            return Ok(false);
        };

        let mut failures = state.failures.clone();
        failures.remove(&failure);
        let mut fixed_failures = state.fixed_failures.clone();
        fixed_failures.insert(failure);

        // Write the archive first. If the second write fails, the known entry
        // remains on disk and can be moved again on the next run.
        write_failure_entries(&self.fixed_path, &fixed_failures, "fixed PDF failures")?;
        write_failure_entries(&self.path, &failures, "known PDF failures")?;

        state.failures = failures;
        state.fixed_failures = fixed_failures;
        Ok(true)
    }
}

fn write_failure_entries(
    path: &Path,
    failures: &HashSet<KnownFailure>,
    description: &str,
) -> Result<(), Report> {
    let mut values = failures.iter().cloned().collect::<Vec<_>>();
    values.sort();
    let contents = serde_json::to_string_pretty(&values)
        .context(format!("failed to serialize {description}"))?;
    fs::write(path, format!("{contents}\n")).context(format!(
        "failed to write {description} file \x60{}\x60",
        path.display()
    ))?;
    Ok(())
}

fn hash_pdf_file(path: &Path) -> Result<String, Report> {
    let bytes = fs::read(path).context(format!(
        "failed to read PDF \x60{}\x60 for hashing",
        path.display()
    ))?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn classify_failure(
    known_failures: &KnownFailures,
    hash: &str,
    original_file_name: &str,
    cause: FailureCause,
    report: Report,
) -> Result<Report, Report> {
    let error = format_failure(&report);
    if known_failures.contains(hash, cause)? {
        if let Err(record_error) =
            known_failures.record(hash, original_file_name, cause, error.clone())
        {
            tracing::error!(
                hash,
                cause = %cause,
                error = ?record_error,
                "failed to enrich known PDF failure"
            );
        }
        tracing::info!(
            hash,
            cause = %cause,
            "expected failure"
        );
        let report = report.into_cloneable();
        let source = report.clone().into_boxed_error();
        return Ok(report
            .context(error::ExpectedError {
                hash: hash.to_owned(),
                cause,
                source,
            })
            .into());
    }

    if let Err(record_error) = known_failures.record(hash, original_file_name, cause, error) {
        tracing::error!(
            hash,
            cause = %cause,
            error = ?record_error,
            "failed to record known PDF failure"
        );
    }
    Ok(report)
}

fn format_failure(report: &Report) -> String {
    report
        .iter_reports()
        .map(|report| report.format_current_context_unhooked().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Clone)]
pub struct PipelineSources {
    pub grobid: GrobidClient,
    pub pdf_storage: Arc<dyn PdfStorage>,
    pub typedb: Arc<TypeDbDriver<Connected>>,
    pub known_failures: KnownFailures,
}

impl PipelineSources {
    pub fn new(
        grobid: GrobidClient,
        pdf_storage: Arc<dyn PdfStorage>,
        typedb: Arc<TypeDbDriver<Connected>>,
        known_failures: KnownFailures,
    ) -> Self {
        Self {
            grobid,
            pdf_storage,
            typedb,
            known_failures,
        }
    }
}

pub async fn run(
    config: &Config,
    pdf_file: &Path,
    progress: &impl Progress,
    worker_id: usize,
    sources: PipelineSources,
) -> Result<(), Report> {
    let original_file_name = pdf_file
        .file_name()
        .context(format!(
            "failed to read file name for {}",
            pdf_file.display()
        ))?
        .to_string_lossy()
        .into_owned();
    let pdf_hash = match hash_pdf_file(pdf_file) {
        Ok(pdf_hash) => pdf_hash,
        Err(report) => {
            if config.save_debug_artifacts {
                preserve_failed_pdf(pdf_file).await;
            }
            return Err(report);
        }
    };
    let expected_grobid_failure = sources
        .known_failures
        .contains(&pdf_hash, FailureCause::GrobidExtraction)?;
    let expected_tei_failure = sources
        .known_failures
        .contains(&pdf_hash, FailureCause::TeiParsing)?;
    let expected_typedb_failure = sources
        .known_failures
        .contains(&pdf_hash, FailureCause::TypeDbExport)?;

    let span = tracing::info_span!(
        "pipeline",
        pdf = %pdf_file.display(),
        worker_id,
    );

    let result = async {
        tracing::info!("started PDF processing");
        if config.save_debug_artifacts {
            sources.pdf_storage.store_pdf(pdf_file, &pdf_hash).await?;
        }
        progress.report(ProgressEvent::FileStarted {
            worker_id,
            file_path: pdf_file.display().to_string(),
            total_steps: TOTAL_STEPS,
            message: "extracting from Grobid".to_owned(),
        });

        let tei_xml = match extract_tei(
            &sources.grobid,
            config.save_debug_artifacts,
            pdf_file,
            &pdf_hash,
            progress,
            worker_id,
        )
        .await
        {
            Ok(tei_xml) => tei_xml,
            Err(report) => {
                return Err(classify_failure(
                    &sources.known_failures,
                    &pdf_hash,
                    &original_file_name,
                    FailureCause::GrobidExtraction,
                    report,
                )?);
            }
        };
        let queries = {
            let document = match parse_tei(
                config.save_debug_artifacts,
                &pdf_hash,
                &tei_xml,
                progress,
                worker_id,
            ) {
                Ok(document) => document,
                Err(parse_report) => {
                    if parse_report
                        .downcast_current_context::<error::TeiParseFailure>()
                        .is_some()
                    {
                        return Err(classify_failure(
                            &sources.known_failures,
                            &pdf_hash,
                            &original_file_name,
                            FailureCause::TeiParsing,
                            parse_report,
                        )?);
                    }
                    return Err(parse_report);
                }
            };
            typeql_queries(&document)
        };
        if let Err(report) = export_to_typedb(&sources.typedb, queries, progress, worker_id).await {
            return Err(classify_failure(
                &sources.known_failures,
                &pdf_hash,
                &original_file_name,
                FailureCause::TypeDbExport,
                report,
            )?);
        }

        tracing::info!("completed PDF processing");
        Ok(())
    }
    .instrument(span)
    .await;

    if result.is_ok() {
        if expected_grobid_failure {
            log_fixed_failure(
                &sources.known_failures,
                &pdf_hash,
                FailureCause::GrobidExtraction,
            );
        }
        if expected_tei_failure {
            log_fixed_failure(&sources.known_failures, &pdf_hash, FailureCause::TeiParsing);
        }
        if expected_typedb_failure {
            log_fixed_failure(
                &sources.known_failures,
                &pdf_hash,
                FailureCause::TypeDbExport,
            );
        }
    } else if config.save_debug_artifacts {
        preserve_failed_pdf(pdf_file).await;
    }
    progress.report(ProgressEvent::FileFinished { worker_id });
    result
}

fn log_fixed_failure(known_failures: &KnownFailures, hash: &str, cause: FailureCause) {
    match known_failures.mark_fixed(hash, cause) {
        Ok(true) => tracing::warn!(
            hash,
            cause = %cause,
            "PDF previously expected to fail, but the full pipeline succeeded; moved failure to fixed_failures"
        ),
        Ok(false) => {}
        Err(error) => tracing::error!(
            hash,
            cause = %cause,
            error = ?error,
            "failed to move fixed PDF failure"
        ),
    }
}

async fn preserve_failed_pdf(pdf_file: &Path) {
    if let Err(error) = copy_failed_pdf(pdf_file, Path::new(FAILED_PDFS_DIR)).await {
        tracing::error!(
            pdf = %pdf_file.display(),
            error = ?error,
            "failed to preserve PDF for retry"
        );
    }
}

async fn copy_failed_pdf(pdf_file: &Path, directory: &Path) -> Result<PathBuf, Report> {
    let file_name = pdf_file.file_name().context(format!(
        "failed to read file name for {}",
        pdf_file.display()
    ))?;
    let artifact_path = directory.join(file_name);

    tokio::fs::create_dir_all(directory).await.context(format!(
        "failed to create failed PDF directory `{}`",
        directory.display()
    ))?;
    if pdf_file != artifact_path {
        tokio::fs::copy(pdf_file, &artifact_path)
            .await
            .context(format!(
                "failed to copy failed PDF `{}` to `{}`",
                pdf_file.display(),
                artifact_path.display()
            ))?;
    }
    tracing::debug!(artifact = %artifact_path.display(), "saved failed PDF for retry");
    Ok(artifact_path)
}

async fn extract_tei(
    grobid: &GrobidClient,
    save_debug_artifacts: bool,
    pdf_file: &Path,
    pdf_hash: &str,
    progress: &impl Progress,
    worker_id: usize,
) -> Result<String, Report> {
    progress.report(ProgressEvent::Step {
        worker_id,
        step: 1,
        message: Some("extracting TEI XML from Grobid".to_owned()),
    });

    let tei_xml = grobid.extract_pdf_to_tei_xml_with_retry(pdf_file).await?;

    if save_debug_artifacts {
        save_hashed_debug_artifact(RAW_TEI_ARTIFACTS_DIR, pdf_hash, ".tei.xml", &tei_xml)?;
    }

    Ok(tei_xml)
}

fn parse_tei(
    save_debug_artifacts: bool,
    pdf_hash: &str,
    tei_xml: &str,
    progress: &impl Progress,
    worker_id: usize,
) -> Result<crate::domain::DocumentWithChunks, Report> {
    progress.report(ProgressEvent::Step {
        worker_id,
        step: 2,
        message: Some("parsing TEI XML".to_owned()),
    });

    let document = tei::parse_with_pdf_hash(tei_xml, pdf_hash)
        .map_err(|parse_error| parse_error.context(error::TeiParseFailure))?;
    tracing::debug!(
        document = ?document.document,
        chunks = document.chunks.len(),
        "parsed TEI XML"
    );

    if save_debug_artifacts {
        let parsed_tei = serde_json::to_string_pretty(&document)
            .context("failed to serialize parsed TEI as JSON")?;
        save_hashed_debug_artifact(PARSED_TEI_ARTIFACTS_DIR, pdf_hash, ".json", &parsed_tei)?;
    }

    Ok(document)
}

async fn export_to_typedb(
    typedb: &TypeDbDriver<Connected>,
    queries: Vec<String>,
    progress: &impl Progress,
    worker_id: usize,
) -> Result<(), Report> {
    progress.report(ProgressEvent::Step {
        worker_id,
        step: 3,
        message: Some("exporting domain models to TypeDB".to_owned()),
    });

    typedb
        .export_queries(queries)
        .await
        .context("failed to export parsed domain models to TypeDB")?;

    Ok(())
}

pub fn save_debug_artifact(
    directory: &str,
    pdf_file: &Path,
    suffix: &str,
    content: &str,
) -> Result<PathBuf, Report> {
    let pdf_hash = hash_pdf_file(pdf_file)?;
    save_hashed_debug_artifact(directory, &pdf_hash, suffix, content)
}

fn save_hashed_debug_artifact(
    directory: &str,
    pdf_hash: &str,
    suffix: &str,
    content: &str,
) -> Result<PathBuf, Report> {
    let artifact_path = Path::new(directory).join(format!("{pdf_hash}{suffix}"));

    fs::create_dir_all(directory).context(format!(
        "failed to create debug artifact directory `{directory}`"
    ))?;
    fs::write(&artifact_path, content.as_bytes()).context(format!(
        "failed to write debug artifact `{}`",
        artifact_path.display()
    ))?;
    tracing::debug!(artifact = %artifact_path.display(), "saved debug artifact");
    Ok(artifact_path)
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DuplicateFileGroup {
    pub hash: String,
    pub original_file_names: Vec<String>,
}

pub fn duplicate_file_groups(pdf_files: &[PathBuf]) -> Result<Vec<DuplicateFileGroup>, Report> {
    let mut files_by_hash = std::collections::BTreeMap::<String, Vec<String>>::new();
    for pdf_file in pdf_files {
        let original_file_name = pdf_file
            .file_name()
            .context(format!(
                "failed to read file name for {}",
                pdf_file.display()
            ))?
            .to_string_lossy()
            .into_owned();
        files_by_hash
            .entry(hash_pdf_file(pdf_file)?)
            .or_default()
            .push(original_file_name);
    }

    Ok(files_by_hash
        .into_iter()
        .filter_map(|(hash, mut original_file_names)| {
            (original_file_names.len() > 1).then(|| {
                original_file_names.sort();
                DuplicateFileGroup {
                    hash,
                    original_file_names,
                }
            })
        })
        .collect())
}

pub fn log_duplicate_files(
    pdf_files: &[PathBuf],
    known_failures: &KnownFailures,
) -> Result<(), Report> {
    for group in duplicate_file_groups(pdf_files)? {
        let error = format!(
            "PDF has duplicate files: {}",
            group.original_file_names.join(", ")
        );
        let original_file_name = group
            .original_file_names
            .first()
            .context("duplicate PDF group has no file names")?;
        known_failures.record(
            &group.hash,
            original_file_name,
            FailureCause::Duplicate,
            error,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "scepa-known-failures-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ))
    }

    #[test]
    fn known_failures_are_persisted_as_sorted_hashes() {
        let path = temporary_path();
        let _ = fs::remove_file(&path);

        let failures = KnownFailures::load(&path).unwrap();
        failures
            .record(
                "b",
                "tei-failure.pdf",
                FailureCause::TeiParsing,
                "TEI parser failed".to_owned(),
            )
            .unwrap();
        failures
            .record(
                "a",
                "grobid-failure.pdf",
                FailureCause::GrobidExtraction,
                "GROBID returned 400: malformed table".to_owned(),
            )
            .unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[\n  {\n    \"hash\": \"a\",\n    \"original_file_name\": \"grobid-failure.pdf\",\n    \"cause\": \"grobid_extraction\",\n    \"error\": \"GROBID returned 400: malformed table\"\n  },\n  {\n    \"hash\": \"b\",\n    \"original_file_name\": \"tei-failure.pdf\",\n    \"cause\": \"tei_parsing\",\n    \"error\": \"TEI parser failed\"\n  }\n]\n"
        );
        let reloaded = KnownFailures::load(&path).unwrap();
        assert!(
            reloaded
                .contains("a", FailureCause::GrobidExtraction)
                .unwrap()
        );
        assert!(reloaded.contains("b", FailureCause::TeiParsing).unwrap());

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn classified_failures_persist_the_error_report() {
        let path = temporary_path();
        let _ = fs::remove_file(&path);

        let failures = KnownFailures::load(&path).unwrap();
        let report = rootcause::report!("GROBID returned 400: malformed table");
        let _ = classify_failure(
            &failures,
            "hash",
            "type-db-failure.pdf",
            FailureCause::TypeDbExport,
            report,
        )
        .unwrap();

        let stored: Vec<KnownFailure> =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(stored[0].cause, FailureCause::TypeDbExport);
        assert_eq!(stored[0].original_file_name, "type-db-failure.pdf");
        assert_eq!(stored[0].error, "GROBID returned 400: malformed table");

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn retry_file_names_only_include_active_pipeline_failures() {
        let path = temporary_path();
        let _ = fs::remove_file(&path);

        let failures = KnownFailures::load(&path).unwrap();
        failures
            .record(
                "pipeline-hash",
                "pipeline-failure.pdf",
                FailureCause::TeiParsing,
                "TEI parser failed".to_owned(),
            )
            .unwrap();
        failures
            .record(
                "duplicate-hash",
                "duplicate.pdf",
                FailureCause::Duplicate,
                "duplicate PDF".to_owned(),
            )
            .unwrap();

        assert_eq!(
            failures.retry_file_names().unwrap(),
            HashSet::from(["pipeline-failure.pdf".to_owned()])
        );

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn fixed_failures_are_removed_from_known_failures_and_archived() {
        let path = temporary_path();
        let fixed_path = path.with_file_name(format!(
            "scepa-fixed-failures-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&fixed_path);

        let failures = KnownFailures::load_paths(path.clone(), fixed_path.clone()).unwrap();
        failures
            .record(
                "hash",
                "tei-failure.pdf",
                FailureCause::TeiParsing,
                "TEI parser failed".to_owned(),
            )
            .unwrap();

        assert!(
            failures
                .mark_fixed("hash", FailureCause::TeiParsing)
                .unwrap()
        );
        assert!(!failures.contains("hash", FailureCause::TeiParsing).unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "[]\n");

        let fixed: Vec<KnownFailure> =
            serde_json::from_str(&fs::read_to_string(&fixed_path).unwrap()).unwrap();
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[0].hash, "hash");
        assert_eq!(fixed[0].original_file_name, "tei-failure.pdf");
        assert_eq!(fixed[0].cause, FailureCause::TeiParsing);
        assert_eq!(fixed[0].error, "TEI parser failed");
        assert!(
            !failures
                .mark_fixed("hash", FailureCause::TeiParsing)
                .unwrap()
        );

        fs::remove_file(path).unwrap();
        fs::remove_file(fixed_path).unwrap();
    }

    #[test]
    fn pdf_hash_is_sha256_of_file_bytes() {
        let first_path = temporary_path();
        let second_path = first_path.with_file_name("scepa-known-failures-second.pdf");
        fs::write(&first_path, b"pdf").unwrap();
        fs::write(&second_path, b"pdf").unwrap();

        let first_hash = hash_pdf_file(&first_path).unwrap();
        assert_eq!(first_hash, hash_pdf_file(&second_path).unwrap());
        assert_eq!(
            first_hash,
            "c35b21d6ca39aa7cc3b79a705d989f1a6e88b99ab43988d74048799e3db926a3"
        );

        fs::remove_file(first_path).unwrap();
        fs::remove_file(second_path).unwrap();
    }

    #[test]
    fn artifacts_are_named_by_pdf_hash() {
        let root = std::env::temp_dir().join(format!(
            "scepa-artifacts-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let source = root.join("original-name.pdf");
        let raw_tei = root.join("raw_tei");
        let parsed_tei = root.join("parsed_tei");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"pdf").unwrap();

        let hash = hash_pdf_file(&source).unwrap();
        let raw_tei = raw_tei.to_str().unwrap();
        let parsed_tei = parsed_tei.to_str().unwrap();

        let raw_tei_path = save_debug_artifact(raw_tei, &source, ".tei.xml", "<TEI/>").unwrap();
        let parsed_tei_path = save_debug_artifact(parsed_tei, &source, ".json", "{}").unwrap();

        assert_eq!(
            raw_tei_path.file_name().unwrap().to_str().unwrap(),
            format!("{hash}.tei.xml")
        );
        assert_eq!(
            parsed_tei_path.file_name().unwrap().to_str().unwrap(),
            format!("{hash}.json")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn failed_pdfs_are_copied_for_retry() {
        let root = std::env::temp_dir().join(format!(
            "scepa-failed-pdfs-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let source = root.join("original-name.pdf");
        let failed_directory = root.join("retry_pdfs");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"pdf").unwrap();

        let failed_path = copy_failed_pdf(&source, &failed_directory).await.unwrap();

        assert_eq!(failed_path, failed_directory.join("original-name.pdf"));
        assert_eq!(fs::read(&failed_path).unwrap(), b"pdf");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_file_groups_use_original_file_names() {
        let root = std::env::temp_dir().join(format!(
            "scepa-duplicates-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let first = root.join("first.pdf");
        let duplicate = root.join("duplicate.pdf");
        let unique = root.join("unique.pdf");
        fs::create_dir_all(&root).unwrap();
        fs::write(&first, b"same").unwrap();
        fs::write(&duplicate, b"same").unwrap();
        fs::write(&unique, b"different").unwrap();

        let groups = duplicate_file_groups(&[first, duplicate, unique]).unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].original_file_names,
            ["duplicate.pdf", "first.pdf"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_files_are_logged_as_known_failures() {
        let root = std::env::temp_dir().join(format!(
            "scepa-duplicate-failures-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let first = root.join("first.pdf");
        let duplicate = root.join("duplicate.pdf");
        let known_failures_path = root.join("known_failures.json");
        fs::create_dir_all(&root).unwrap();
        fs::write(&first, b"same").unwrap();
        fs::write(&duplicate, b"same").unwrap();

        let known_failures = KnownFailures::load(&known_failures_path).unwrap();
        log_duplicate_files(&[first, duplicate], &known_failures).unwrap();

        let stored: Vec<KnownFailure> =
            serde_json::from_str(&fs::read_to_string(&known_failures_path).unwrap()).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].cause, FailureCause::Duplicate);
        assert_eq!(stored[0].original_file_name, "duplicate.pdf");
        assert_eq!(
            stored[0].error,
            "PDF has duplicate files: duplicate.pdf, first.pdf"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn expected_error_retains_cause_and_original_report() {
        let report = rootcause::report!("actual TEI parser failure").into_cloneable();
        let source = report.clone().into_boxed_error();
        let report: Report = report
            .context(error::ExpectedError {
                hash: "hash".to_owned(),
                cause: FailureCause::TeiParsing,
                source,
            })
            .into();

        assert_eq!(
            report
                .downcast_current_context::<error::ExpectedError>()
                .unwrap()
                .cause,
            FailureCause::TeiParsing
        );
        assert_eq!(report.children().len(), 1);
    }
}
