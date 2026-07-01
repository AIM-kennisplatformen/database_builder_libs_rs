use std::{io::Error, path::PathBuf};

use clap::{
    Parser,
    builder::{PathBufValueParser, TypedValueParser},
};
use database_builder_scepa_rs::{ingestion::pipeline, models::paths::pdf::PdfPath};

#[derive(Parser)]
struct Args {
    #[arg(value_name = "pdf-path", value_parser = PathBufValueParser::new().try_map(PdfPath::try_from))]
    pdf_path: PdfPath,

    #[arg(value_name = "output-dir")]
    output_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    pipeline::run(args.pdf_path, args.output_dir).await
}
