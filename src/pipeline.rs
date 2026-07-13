use std::{
    fs,
    path::{Path, PathBuf},
};

use rootcause::{
    option_ext::OptionExt,
    prelude::{Report, ResultExt},
};
use tracing::Instrument;

pub mod error;
pub mod source;
pub mod tei;

use crate::{Config, pipeline::source::grobid::GrobidClient, progress::Progress};

pub const TOTAL_STEPS: usize = 2;
pub const RAW_TEI_ARTIFACTS_DIR: &str = "log/raw_tei";
pub const PARSED_TEI_ARTIFACTS_DIR: &str = "log/parsed_tei";

#[derive(Clone)]
pub struct PipelineSources {
    pub grobid: GrobidClient,
}

pub async fn run(
    config: &Config,
    pdf_file: &Path,
    progress: &Progress,
    worker_id: usize,
    sources: PipelineSources,
) -> Result<(), Report> {
    let span = tracing::info_span!(
        "pipeline",
        pdf = %pdf_file.display(),
        worker_id,
    );

    let result = async {
        tracing::info!("started PDF processing");
        progress.start_file(worker_id, pdf_file, TOTAL_STEPS, "extracting from Grobid");

        let tei_xml = extract_tei(
            &sources.grobid,
            config.save_debug_artifacts,
            pdf_file,
            progress,
            worker_id,
        )
        .await?;
        let _document = parse_tei(
            config.save_debug_artifacts,
            pdf_file,
            &tei_xml,
            progress,
            worker_id,
        )?;

        tracing::info!("completed PDF processing");
        Ok(())
    }
    .instrument(span)
    .await;

    progress.finish_file(worker_id);
    result
}

async fn extract_tei(
    grobid: &GrobidClient,
    save_debug_artifacts: bool,
    pdf_file: &Path,
    progress: &Progress,
    worker_id: usize,
) -> Result<String, Report> {
    let tei_xml = grobid.extract_pdf_to_tei_xml_with_retry(pdf_file).await?;

    if save_debug_artifacts {
        save_debug_artifact(RAW_TEI_ARTIFACTS_DIR, pdf_file, ".tei.xml", &tei_xml)?;
    }
    progress.step(worker_id, 1, Some("extracted TEI XML".to_owned()));

    Ok(tei_xml)
}

fn parse_tei(
    save_debug_artifacts: bool,
    pdf_file: &Path,
    tei_xml: &str,
    progress: &Progress,
    worker_id: usize,
) -> Result<tei::Document, Report> {
    let document = tei::parse(tei_xml).context("failed to parse extracted TEI XML")?;
    tracing::debug!(
        title = ?document.title,
        authors = document.authors.len(),
        sections = document.sections.len(),
        references = document.references.len(),
        "parsed TEI XML"
    );

    if save_debug_artifacts {
        let parsed_tei = serde_json::to_string_pretty(&document)
            .context("failed to serialize parsed TEI as JSON")?;
        save_debug_artifact(PARSED_TEI_ARTIFACTS_DIR, pdf_file, ".json", &parsed_tei)?;
    }
    progress.step(worker_id, 2, Some("parsed TEI XML".to_owned()));

    Ok(document)
}

pub fn save_debug_artifact(
    directory: &str,
    pdf_file: &Path,
    suffix: &str,
    content: &str,
) -> Result<PathBuf, Report> {
    let mut artifact_name = pdf_file
        .file_stem()
        .context(format!(
            "failed to read file stem for {}",
            pdf_file.display()
        ))?
        .to_os_string();
    artifact_name.push(suffix);
    let artifact_path = Path::new(directory).join(artifact_name);

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
