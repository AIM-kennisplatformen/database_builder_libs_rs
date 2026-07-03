mod progress;

use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
    process::ExitCode,
};

use clap::Parser;
use database_builder_scepa_rs::{
    ingestion::{error::PipelineError, pipeline},
    models::paths::pdf::PdfPath,
};
use progress::ProgressBar;
use tokio::task::JoinSet;

const PARALLELISM: usize = 10;

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

    let failure_count =
        process_pdf_source(args.pdf_source, args.tei_xml_dir, args.json_dir).await?;

    if failure_count > 0 {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

async fn process_pdf_source(
    pdf_source: PathBuf,
    tei_xml_dir: PathBuf,
    json_dir: PathBuf,
) -> Result<usize, Error> {
    let pdf_paths = collect_pdf_paths(pdf_source)?;

    let total = pdf_paths.len();
    let mut pdf_paths = pdf_paths.into_iter();
    let mut tasks = JoinSet::new();
    let mut progress = ProgressBar::new(total, PARALLELISM.min(total));
    let mut failure_count = 0;

    loop {
        while tasks.len() < PARALLELISM {
            let Some(pdf_path) = pdf_paths.next() else {
                break;
            };

            progress.start_document();
            let tei_xml_dir = tei_xml_dir.clone();
            let json_dir = json_dir.clone();
            tasks.spawn(async move {
                let document_path = pdf_path.as_path().to_path_buf();

                pipeline::run_with_reporter(pdf_path, tei_xml_dir, json_dir, |_| {}).await?;

                Ok::<PathBuf, PipelineError>(document_path)
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
                progress.error(&error.to_string());
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

fn collect_pdf_paths(pdf_source: PathBuf) -> Result<Vec<PdfPath>, Error> {
    if pdf_source.is_file() {
        return Ok(vec![
            PdfPath::try_from(pdf_source)
                .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?,
        ]);
    }

    let entries = fs::read_dir(&pdf_source).map_err(|source| {
        Error::new(
            source.kind(),
            format!(
                "failed to read PDF directory {}: {source}",
                pdf_source.display()
            ),
        )
    })?;

    let mut pdf_paths = Vec::new();

    for entry in entries {
        let path = entry
            .map_err(|source| {
                Error::new(
                    source.kind(),
                    format!("failed to read entry in {}: {source}", pdf_source.display()),
                )
            })?
            .path();

        if path.is_file()
            && let Ok(pdf_path) = PdfPath::try_from(path)
        {
            pdf_paths.push(pdf_path);
        }
    }

    pdf_paths.sort_by(|left, right| left.as_path().cmp(right.as_path()));

    if pdf_paths.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "PDF directory contains no PDF files: {}",
                pdf_source.display()
            ),
        ));
    }

    Ok(pdf_paths)
}
