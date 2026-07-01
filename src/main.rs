use std::{
    fs,
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

use clap::Parser;
use database_builder_scepa_rs::{ingestion::pipeline, models::paths::pdf::PdfPath};

#[derive(Parser)]
struct Args {
    #[arg(value_name = "pdf-path-or-dir")]
    pdf_source: PathBuf,

    #[arg(value_name = "tei-xml-dir")]
    tei_xml_dir: PathBuf,

    #[arg(value_name = "json-dir")]
    json_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    if args.pdf_source.is_dir() {
        for pdf_path in pdf_paths_in_dir(&args.pdf_source)? {
            pipeline::run(pdf_path, args.tei_xml_dir.clone(), args.json_dir.clone()).await?;
        }

        return Ok(());
    }

    let pdf_path = PdfPath::try_from(args.pdf_source)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;

    pipeline::run(pdf_path, args.tei_xml_dir, args.json_dir).await
}

fn pdf_paths_in_dir(pdf_dir: &Path) -> Result<Vec<PdfPath>, Error> {
    let entries = fs::read_dir(pdf_dir).map_err(|source| {
        Error::new(
            source.kind(),
            format!(
                "failed to read PDF directory {}: {source}",
                pdf_dir.display()
            ),
        )
    })?;

    let mut pdf_paths = Vec::new();

    for entry in entries {
        let path = entry
            .map_err(|source| {
                Error::new(
                    source.kind(),
                    format!("failed to read entry in {}: {source}", pdf_dir.display()),
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
            format!("PDF directory contains no PDF files: {}", pdf_dir.display()),
        ));
    }

    Ok(pdf_paths)
}
