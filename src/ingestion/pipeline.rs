use std::{io::Error, path::PathBuf};

use crate::{
    ingestion::{
        extract::grobid::{config::GrobidConfig, source::GrobidSource},
        parse::tei::reader::parse_tei_xml_path,
    },
    models::paths::pdf::PdfPath,
};

pub async fn run(pdf_path: PdfPath, tei_xml_dir: PathBuf) -> Result<(), Error> {
    let grobid = GrobidSource::new(GrobidConfig {
        output_dir: tei_xml_dir,
        url: "http://localhost:8070".into(),
    });

    let tei_xml_path = match grobid.extract_pdf_to_tei_xml(pdf_path).await {
        Ok(output_path) => {
            println!("Saved TEI XML to {}", output_path.display());
            output_path
        }
        Err(error) => {
            eprintln!("Failed to parse PDF with GROBID: {error}");
            return Err(Error::other(error));
        }
    };

    let tei_document = parse_tei_xml_path(&tei_xml_path);

    match tei_document {
        Ok(_) => {
            println!("Transformed TEI XML into tei document");
            Ok(())
        }
        Err(error) => {
            eprintln!("Failed to parse TEI XML: {error}");
            Err(Error::other(error))
        }
    }
}
