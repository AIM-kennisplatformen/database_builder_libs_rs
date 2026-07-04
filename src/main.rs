mod progress;

use std::{
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use database_builder_scepa_rs::{
    ingestion::{
        extract::grobid::{config::GrobidConfig, source::GrobidSource},
        pipeline,
    },
    models::paths::pdf::PdfPath,
    stores::{
        connection::Disconnect,
        typedb::{
            DOMAIN_SCHEMA,
            config::TypedbConfig,
            store::{TypedbConnected, TypedbStore},
        },
    },
};
use progress::ProgressBar;
use tokio::{runtime::Builder, task::JoinSet};

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
struct Env {
    #[arg(long, env = "PDF_SOURCE")]
    pdf_source: PathBuf,

    #[arg(long, env = "TEI_XML_DIR")]
    tei_xml_dir: PathBuf,

    #[arg(long, env = "JSON_DIR")]
    json_dir: PathBuf,

    #[arg(long, env = "GROBID_URL")]
    grobid_url: String,

    #[arg(long, env = "TYPEDB_ADDRESS")]
    typedb_address: String,

    #[arg(long, env = "TYPEDB_DATABASE")]
    typedb_database: String,

    #[arg(long, env = "TYPEDB_USERNAME")]
    typedb_username: String,

    #[arg(long, env = "TYPEDB_PASSWORD")]
    typedb_password: String,

    #[arg(long, env = "TYPEDB_TLS", action = clap::ArgAction::Set)]
    typedb_tls: bool,

    #[arg(
        long,
        env = "TYPEDB_WIPE_DATABASE",
        default_value_t = false,
        action = clap::ArgAction::Set
    )]
    typedb_wipe_database: bool,

    #[arg(long, env = "PARALLELISM")]
    parallelism: NonZeroUsize,

    #[arg(long, env = "WORKER_THREADS")]
    worker_threads: NonZeroUsize,
}

#[derive(Clone)]
struct RunConfig {
    pdf_source: PathBuf,
    tei_xml_dir: Arc<PathBuf>,
    json_dir: Arc<PathBuf>,
    grobid: Arc<GrobidSource>,
    typedb_store: Arc<TypedbStore<TypedbConnected>>,
    parallelism: usize,
}

fn main() -> ExitCode {
    match run_cli() {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{}", format_error_chain(&error));
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> Result<ExitCode> {
    dotenvy::dotenv().expect("Failed to load .env file");

    let env = Env::parse();
    let worker_threads = env.worker_threads.get();

    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .context("building Tokio runtime")?;

    runtime.block_on(run(env))
}

async fn run(env: Env) -> Result<ExitCode> {
    let grobid = Arc::new(GrobidSource::new(GrobidConfig {
        url: env.grobid_url,
    }));

    let typedb_config = TypedbConfig::new(
        env.typedb_address,
        env.typedb_database,
        env.typedb_username,
        env.typedb_password,
        env.typedb_tls,
        env.typedb_wipe_database,
        DOMAIN_SCHEMA,
    );

    let typedb_store = Arc::new(
        TypedbStore::new()
            .connect(&typedb_config)
            .await
            .context("connecting to TypeDB for domain export")?,
    );

    let config = RunConfig {
        pdf_source: env.pdf_source,
        tei_xml_dir: Arc::new(env.tei_xml_dir),
        json_dir: Arc::new(env.json_dir),
        grobid,
        typedb_store,
        parallelism: env.parallelism.get(),
    };

    let source_display = config.pdf_source.display().to_string();

    let failure_count = {
        process_pdf_source(config.clone())
            .await
            .with_context(|| format!("processing PDF source {source_display}"))?
    };
    if let Ok(typedb_store) = Arc::try_unwrap(config.typedb_store) {
        typedb_store
            .disconnect()
            .context("disconnecting from TypeDB after domain export")?;
    }

    if failure_count > 0 {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

async fn process_pdf_source(config: RunConfig) -> Result<usize> {
    let pdf_paths = collect_pdf_paths(&config.pdf_source)
        .with_context(|| format!("collecting PDF paths from {}", config.pdf_source.display()))?;

    let total = pdf_paths.len();
    let mut pdf_paths = pdf_paths.into_iter();
    let mut tasks = JoinSet::new();
    let mut progress = ProgressBar::new(total, config.parallelism.min(total));
    let mut failure_count = 0;

    loop {
        while tasks.len() < config.parallelism {
            let Some(pdf_path) = pdf_paths.next() else {
                break;
            };

            progress.start_document();

            let tei_xml_dir = config.tei_xml_dir.clone();
            let json_dir = config.json_dir.clone();
            let grobid = config.grobid.clone();
            let typedb_store = config.typedb_store.clone();

            tasks.spawn(async move {
                let document_path = pdf_path.as_path().to_path_buf();

                pipeline::run_with_reporter(
                    pdf_path,
                    tei_xml_dir.as_ref().as_path(),
                    json_dir.as_ref().as_path(),
                    grobid.as_ref(),
                    typedb_store.as_ref(),
                    |_| {},
                )
                .await
                .with_context(|| format!("processing PDF {}", document_path.display()))?;

                Ok::<PathBuf, anyhow::Error>(document_path)
            });
        }

        let Some(result) = tasks.join_next().await else {
            break;
        };

        progress.finish_document();

        match result {
            Ok(Ok(_)) => progress.render(),
            Ok(Err(error)) => {
                failure_count += 1;
                progress.error(&format_error_chain(&error));
            }
            Err(error) => {
                failure_count += 1;
                progress.worker_error(&error.to_string());
            }
        }
    }

    progress.finish(failure_count);

    Ok(failure_count)
}

fn collect_pdf_paths(pdf_source: &Path) -> Result<Vec<PdfPath>> {
    if pdf_source.is_file() {
        let pdf_path = PdfPath::try_from(pdf_source.to_path_buf())
            .with_context(|| format!("validating PDF path {}", pdf_source.display()))?;

        return Ok(vec![pdf_path]);
    }

    let entries = fs::read_dir(pdf_source)
        .with_context(|| format!("reading PDF directory {}", pdf_source.display()))?;

    let mut pdf_paths = Vec::new();

    for entry in entries {
        let path = entry
            .with_context(|| format!("reading entry in {}", pdf_source.display()))?
            .path();

        if path.is_file()
            && let Ok(pdf_path) = PdfPath::try_from(path)
        {
            pdf_paths.push(pdf_path);
        }
    }

    pdf_paths.sort_by(|left, right| left.as_path().cmp(right.as_path()));

    if pdf_paths.is_empty() {
        bail!(
            "PDF directory contains no PDF files: {}",
            pdf_source.display()
        );
    }

    Ok(pdf_paths)
}

fn format_error_chain(error: &anyhow::Error) -> String {
    use std::fmt::Write as _;

    let mut message = error.to_string();
    let mut causes = error.chain().skip(1).peekable();

    if causes.peek().is_some() {
        message.push_str("\nCaused by:");

        for (index, cause) in causes.enumerate() {
            let _ = write!(message, "\n    {index}: {cause}");
        }
    }

    message
}
