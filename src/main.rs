use std::{io::Error, path::PathBuf, process::ExitCode};

use clap::Parser;
use database_builder_scepa_rs::ingestion::batch;

#[derive(Parser)]
struct Args {
    #[arg(value_name = "pdf-path-or-dir")]
    pdf_source: PathBuf,

    #[arg(value_name = "tei-xml-dir")]
    tei_xml_dir: PathBuf,

    #[arg(value_name = "json-dir")]
    json_dir: PathBuf,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ExitCode {
    match run_cli().await {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run_cli() -> Result<ExitCode, Error> {
    let args = Args::parse();

    let summary = batch::run_source(args.pdf_source, args.tei_xml_dir, args.json_dir).await?;
    if summary.has_failures() {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}
