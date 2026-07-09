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
        extract::{
            grobid::{config::GrobidConfig, source::GrobidSource},
            ror::{config::RorConfig, source::RorSource},
        },
        pipeline::{self, PipelineSources, PipelineStores},
        transform::embedding::{config::EmbeddingConfig, source::EmbeddingSource},
    },
    models::paths::pdf::PdfPath,
    stores::{
        connection::Disconnect,
        qdrant::{
            config::QdrantConfig,
            store::{QdrantConnected, QdrantStore},
        },
        studio::{config::StudioConfig, store::StudioStore},
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

    #[arg(long, env = "ROR_HOST", default_value = "https://api.ror.org")]
    ror_host: String,

    #[arg(long, env = "QDRANT_URL")]
    qdrant_url: String,

    #[arg(long, env = "QDRANT_COLLECTION")]
    qdrant_collection: String,

    #[arg(long, env = "QDRANT_VECTOR_SIZE")]
    qdrant_vector_size: u64,

    #[arg(long, env = "QDRANT_API_KEY", default_value = "")]
    qdrant_api_key: String,

    #[arg(
        long,
        env = "QDRANT_WIPE_COLLECTION",
        default_value_t = false,
        action = clap::ArgAction::Set
    )]
    qdrant_wipe_collection: bool,

    #[arg(
        long,
        env = "STUDIO_PDF_STORE_ENABLED",
        default_value_t = false,
        action = clap::ArgAction::Set
    )]
    studio_pdf_store_enabled: bool,

    #[arg(long, env = "STUDIO_BASE_URL", default_value = "")]
    studio_base_url: String,

    #[arg(long, env = "STUDIO_API_KEY", default_value = "")]
    studio_api_key: String,

    #[arg(long, env = "OPENAI_HOST")]
    openai_host: String,

    #[arg(long, env = "OPENAI_API_KEY")]
    openai_api_key: String,

    #[arg(long, env = "OPENAI_EMBEDDING_MODEL")]
    openai_embedding_model: String,

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
    ror_source: Arc<RorSource>,
    embedding_source: Arc<EmbeddingSource>,
    qdrant_store: Arc<QdrantStore<QdrantConnected>>,
    studio_store: Option<Arc<StudioStore>>,
    parallelism: usize,
}

fn main() -> ExitCode {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("installing default rustls CryptoProvider");

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

    let ror_source = Arc::new(RorSource::new(RorConfig::new(env.ror_host)));

    let embedding_source = Arc::new(EmbeddingSource::new(EmbeddingConfig::new(
        env.openai_host,
        env.openai_api_key,
        env.openai_embedding_model,
    )));

    let qdrant_config = QdrantConfig::new(
        env.qdrant_url,
        env.qdrant_collection,
        env.qdrant_vector_size,
        env.qdrant_api_key,
        env.qdrant_wipe_collection,
    );

    let qdrant_store = Arc::new(
        QdrantStore::new()
            .connect(&qdrant_config)
            .await
            .context("connecting to Qdrant for chunk export")?,
    );

    let studio_store = if env.studio_pdf_store_enabled {
        if env.studio_base_url.is_empty() || env.studio_api_key.is_empty() {
            bail!("STUDIO_PDF_STORE_ENABLED=true requires both STUDIO_BASE_URL and STUDIO_API_KEY");
        }
        Some(Arc::new(StudioStore::new(StudioConfig::new(
            env.studio_base_url,
            env.studio_api_key,
        ))))
    } else {
        None
    };

    let config = RunConfig {
        pdf_source: env.pdf_source,
        tei_xml_dir: Arc::new(env.tei_xml_dir),
        json_dir: Arc::new(env.json_dir),
        grobid,
        typedb_store,
        ror_source,
        embedding_source,
        qdrant_store,
        studio_store,
        parallelism: env.parallelism.get(),
    };

    config
        .qdrant_store
        .update_indexing_treshold(0)
        .await
        .context("disabling Qdrant indexing before chunk export")?;

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

    config
        .qdrant_store
        .update_indexing_treshold(20_000)
        .await
        .context("restoring Qdrant indexing after chunk export")?;

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
            let ror_source = config.ror_source.clone();
            let embedding_source = config.embedding_source.clone();
            let qdrant_store = config.qdrant_store.clone();
            let studio_store = config.studio_store.clone();

            tasks.spawn(async move {
                let document_path = pdf_path.as_path().to_path_buf();

                pipeline::run_with_reporter(
                    pdf_path,
                    tei_xml_dir.as_ref().as_path(),
                    json_dir.as_ref().as_path(),
                    PipelineSources {
                        grobid: grobid.as_ref(),
                        ror_source: ror_source.as_ref(),
                        embedding_source: embedding_source.as_ref(),
                    },
                    PipelineStores {
                        typedb_store: typedb_store.as_ref(),
                        qdrant_store: qdrant_store.as_ref(),
                        studio_store: studio_store.as_deref(),
                    },
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

    let mut pdf_paths = vec![];

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
