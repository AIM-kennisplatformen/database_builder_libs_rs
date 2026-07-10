use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use scepa_rs::{Config, Env, log};

const SOURCES_PATH: &str = "sources/pdfs";

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    dotenvy::dotenv().expect("Failed to load .env file");

    let env = Env::try_parse().context("Failed to parse .env file")?;

    let config = Config::try_from(env).context("Failed to create config from env")?;

    let pdf_source_dir = PathBuf::from(SOURCES_PATH);
    let pdf_paths = collect_file_paths(&pdf_source_dir)?;
    let progress = scepa_rs::progress::Progress::new(pdf_paths.len(), config.worker_count);

    log::setup_tracing(progress.log_writer()).context("Failed to setup logging environment")?;

    Ok(())
}

fn collect_file_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed to read PDF source directory `{}`", dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in `{}`", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to read file type for `{}`", path.display()))?;

        if file_type.is_file() {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}
