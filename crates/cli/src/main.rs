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
    pipeline::{self, PipelineSources},
};

mod progress;

use progress::ProgressBar;

const SOURCES_PATH: &str = "sources/pdfs";

#[derive(Parser)]
#[command(author, version, about)]
struct Env {
    #[arg(long, env = "SAVE_DEBUG_ARTIFACTS", default_value_t = false)]
    save_debug_artifacts: bool,

    #[arg(long, env = "WORKER_COUNT", default_value_t = 4)]
    worker_count: usize,

    #[arg(long, env = "GROBID_URL")]
    grobid_url: String,
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

    let config = Config::try_from(env).context("Failed to create config from env")?;

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_count)
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime")?
        .block_on(async_main(config))
}

async fn async_main(config: Config) -> Result<(), Report> {
    let pdf_source_dir = PathBuf::from(SOURCES_PATH);
    let pdf_paths = collect_file_paths(&pdf_source_dir)?;
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

    let sources = PipelineSources {
        grobid: pipeline::source::grobid::GrobidClient::new(
            config.grobid_url.clone(),
            reqwest::Client::new(),
        ),
    };

    run_workers(Arc::new(config), pdf_paths, progress, sources).await
}

async fn run_workers(
    config: Arc<Config>,
    pdf_paths: Vec<PathBuf>,
    progress: ProgressBar,
    sources: PipelineSources,
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

        workers.push(tokio::spawn(async move {
            let mut worker_errors = Vec::new();

            for pdf_path in worker_paths {
                let result = pipeline::run(
                    worker_config.as_ref(),
                    &pdf_path,
                    &worker_progress,
                    worker_id,
                    worker_sources.clone(),
                )
                .await;

                if let Err(error) = result {
                    let error =
                        error.context(format!("failed to process PDF `{}`", pdf_path.display()));
                    tracing::error!(
                        pdf = %pdf_path.display(),
                        worker_id,
                        error = ?error,
                        "pipeline failed",
                    );
                    worker_errors.push(error);
                }
            }

            worker_errors
        }));
    }

    let mut worker_errors = Vec::new();
    for worker in workers {
        worker_errors.extend(worker.await.context("worker task failed")?);
    }
    progress.finish();

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
