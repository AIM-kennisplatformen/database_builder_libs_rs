use std::{io::Error, path::PathBuf};

use crate::{
    ingestion::extract::grobid::{config::GrobidConfig, source::GrobidSource},
    models::paths::pdf::PdfPath,
};

pub async fn run(pdf_path: PdfPath, output_dir: PathBuf) -> Result<(), Error> {
	let grobid = GrobidSource::new(GrobidConfig {
        output_dir,
        url: "http://localhost:8070".into(),
    });

    let tei_xml_path = grobid.extract_pdf_to_tei_xml(pdf_path).await;

    match tei_xml_path {
        Ok(output_path) => {
            println!("Saved TEI XML to {}", output_path.display());
            Ok(())
        }
        Err(error) => {
            eprintln!("Failed to parse PDF with GROBID: {error}");
            Err(Error::other(error))
        }
    }
}
