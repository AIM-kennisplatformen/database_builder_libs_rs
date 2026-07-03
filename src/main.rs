mod progress;

use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use clap::Parser;
use database_builder_scepa_rs::{ingestion::pipeline, models::paths::pdf::PdfPath};
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

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> ExitCode {
    match run_cli().await {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("{}", format_error_chain(&error));
            ExitCode::FAILURE
        }
    }
}

async fn run_cli() -> Result<ExitCode> {
    let args = Args::parse();
    let pdf_source = args.pdf_source;
    let source_display = pdf_source.display().to_string();

    let failure_count = process_pdf_source(pdf_source, args.tei_xml_dir, args.json_dir)
        .await
        .with_context(|| format!("processing PDF source {source_display}"))?;

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
) -> Result<usize> {
    let pdf_paths = collect_pdf_paths(pdf_source.clone())
        .with_context(|| format!("collecting PDF paths from {}", pdf_source.display()))?;

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

                pipeline::run_with_reporter(pdf_path, tei_xml_dir, json_dir, |_| {})
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

fn collect_pdf_paths(pdf_source: PathBuf) -> Result<Vec<PdfPath>> {
    if pdf_source.is_file() {
        let pdf_path = PdfPath::try_from(pdf_source.clone())
            .with_context(|| format!("validating PDF path {}", pdf_source.display()))?;

        return Ok(vec![pdf_path]);
    }

    let entries = fs::read_dir(&pdf_source)
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
