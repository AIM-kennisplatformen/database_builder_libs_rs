use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use rootcause::hooks::Hooks;
use rootcause::prelude::{Report, ResultExt};
use rootcause::report_collection::ReportCollection;
use rootcause_backtrace::BacktraceCollector;
use scepa_rs::{
    Config, log,
    pipeline::error::ExpectedError,
    pipeline::{self, PipelineSources},
    typedb::{TypeDbConfig, TypeDbDriver},
};

mod progress;

use progress::ProgressBar;

const SOURCES_PATH: &str = "sources/pdfs";
const RETRY_SOURCES_PATH: &str = "sources/retry_pdfs";

#[derive(Parser)]
#[command(author, version, about)]
struct Env {
    #[arg(long, env = "CLEAR_LOG", default_value_t = false)]
    clear_log: bool,

    #[arg(long, env = "SAVE_DEBUG_ARTIFACTS", default_value_t = false)]
    save_debug_artifacts: bool,

    #[arg(long, env = "WORKER_COUNT", default_value_t = 4)]
    worker_count: usize,

    #[arg(long, env = "GROBID_URL")]
    grobid_url: String,

    #[arg(long, env = "TYPEDB_ADDRESS", default_value = "127.0.0.1:1729")]
    typedb_address: String,

    #[arg(long, env = "TYPEDB_DATABASE", default_value = "scepa")]
    typedb_database: String,

    #[arg(long, env = "TYPEDB_USERNAME", default_value = "admin")]
    typedb_username: String,

    #[arg(long, env = "TYPEDB_PASSWORD", default_value = "password")]
    typedb_password: String,

    #[arg(long, env = "TYPEDB_TLS", default_value_t = false)]
    typedb_tls: bool,

    #[arg(long, env = "TYPEDB_WIPE_DATABASE", default_value_t = false)]
    typedb_wipe_database: bool,

    #[arg(long, env = "RETRY", default_value_t = false)]
    retry: bool,
}

impl TryFrom<Env> for Config {
    type Error = Report;

    fn try_from(env: Env) -> Result<Self, Self::Error> {
        Config::new(env.save_debug_artifacts, env.worker_count, env.grobid_url)
    }
}

fn main() -> Result<(), Report> {
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install rootcause backtrace hook");

    dotenvy::dotenv().expect("Failed to load .env file");

    let env = Env::try_parse().context("Failed to parse .env file")?;
    if env.clear_log {
        log::clear_log_dir().context("Failed to clear log directory")?;
    }

    let typedb_config = TypeDbConfig::new(
        env.typedb_address.clone(),
        env.typedb_database.clone(),
        env.typedb_username.clone(),
        env.typedb_password.clone(),
        env.typedb_tls,
        env.typedb_wipe_database,
    );

    let retry = env.retry;
    let config = Config::try_from(env).context("Failed to create config from env")?;

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_count)
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime")?
        .block_on(async_main(config, typedb_config, retry))
}

async fn async_main(
    config: Config,
    typedb_config: TypeDbConfig,
    retry: bool,
) -> Result<(), Report> {
    let pdf_source_dir = PathBuf::from(if retry {
        RETRY_SOURCES_PATH
    } else {
        SOURCES_PATH
    });
    let pdf_paths = collect_file_paths(&pdf_source_dir)?;
    let known_failures = pipeline::KnownFailures::load_default()?;
    pipeline::log_duplicate_files(&pdf_paths, &known_failures)?;
    let progress = ProgressBar::new(pdf_paths.len(), config.worker_count);
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();

    log::setup_tracing(progress.log_writer(), &rust_log)
        .context("Failed to setup logging environment")?;
    tracing::info!(
        pdf_count = pdf_paths.len(),
        worker_count = config.worker_count,
        save_debug_artifacts = config.save_debug_artifacts,
        "starting pipeline"
    );

    let typedb = Arc::new(
        TypeDbDriver::default()
            .connect(&typedb_config)
            .await
            .context("failed to connect to TypeDB")?,
    );
    let sources = PipelineSources::new(
        pipeline::source::grobid::GrobidClient::new(
            config.grobid_url.clone(),
            reqwest::Client::new(),
        ),
        Arc::new(pipeline::storage::LocalPdfStorage::new(
            pipeline::PARSED_FILES_ARTIFACTS_DIR,
        )),
        Arc::clone(&typedb),
        known_failures,
    );

    let result = run_workers(Arc::new(config), pdf_paths, progress, sources, retry).await;
    if let Ok(typedb) = Arc::try_unwrap(typedb) {
        typedb.disconnect()?;
    }
    result
}

async fn run_workers(
    config: Arc<Config>,
    pdf_paths: Vec<PathBuf>,
    progress: ProgressBar,
    sources: PipelineSources,
    retry: bool,
) -> Result<(), Report> {
    let mut workers = Vec::with_capacity(config.worker_count);

    for worker_id in 0..config.worker_count {
        let worker_paths = pdf_paths
            .iter()
            .skip(worker_id)
            .step_by(config.worker_count)
            .cloned()
            .collect::<Vec<_>>();
        let worker_config = Arc::clone(&config);
        let worker_progress = progress.clone();
        let worker_sources = sources.clone();
        let worker_retry = retry;

        workers.push(tokio::spawn(async move {
            let mut worker_errors = Vec::new();
            let mut expected_failures = 0;

            for pdf_path in worker_paths {
                let result = pipeline::run(
                    worker_config.as_ref(),
                    &pdf_path,
                    &worker_progress,
                    worker_id,
                    worker_sources.clone(),
                )
                .await;

                match result {
                    Ok(()) if worker_retry => {
                        if let Err(error) = remove_retry_pdf(&pdf_path).await {
                            let error = error.context(format!(
                                "failed to remove successfully processed retry PDF `{}`",
                                pdf_path.display()
                            ));
                            tracing::error!(
                                pdf = %pdf_path.display(),
                                worker_id,
                                error = ?error,
                                "retry PDF cleanup failed",
                            );
                            worker_errors.push(error);
                        }
                    }
                    Ok(()) => {}
                    Err(error) => {
                        if error.downcast_current_context::<ExpectedError>().is_some() {
                            expected_failures += 1;
                            continue;
                        }
                        let error = error
                            .context(format!("failed to process PDF `{}`", pdf_path.display()));
                        tracing::error!(
                            pdf = %pdf_path.display(),
                            worker_id,
                            error = ?error,
                            "pipeline failed",
                        );
                        worker_errors.push(error);
                    }
                }
            }

            (worker_errors, expected_failures)
        }));
    }

    let mut worker_errors = Vec::new();
    let mut expected_failures = 0;
    for worker in workers {
        let (errors, worker_expected_failures) = worker.await.context("worker task failed")?;
        worker_errors.extend(errors);
        expected_failures += worker_expected_failures;
    }
    progress.finish();
    tracing::info!(
        "expected failures: {expected_failures} / {}",
        pdf_paths.len()
    );

    if worker_errors.is_empty() {
        tracing::info!(processed_files = pdf_paths.len(), "pipeline finished");
        return Ok(());
    }

    let failure_count = worker_errors.len();
    tracing::warn!(
        processed_files = pdf_paths.len(),
        failure_count,
        "pipeline finished with failures"
    );
    let failures = worker_errors
        .into_iter()
        .map(Report::into_cloneable)
        .collect::<ReportCollection>();

    Err(failures
        .context(format!("{failure_count} pipeline failures"))
        .into())
}

async fn remove_retry_pdf(pdf_path: &Path) -> Result<(), Report> {
    tokio::fs::remove_file(pdf_path).await.context(format!(
        "failed to remove retry PDF `{}`",
        pdf_path.display()
    ))?;
    Ok(())
}

fn collect_file_paths(dir: &Path) -> Result<Vec<PathBuf>, Report> {
    let mut paths = Vec::new();

    for entry in fs::read_dir(dir).context(format!(
        "failed to read PDF source directory `{}`",
        dir.display()
    ))? {
        let entry = entry.context(format!("failed to read entry in `{}`", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .context(format!("failed to read file type for `{}`", path.display()))?;

        if file_type.is_file() {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}
