use std::{io::Error, path::PathBuf};

use crate::{
    ingestion::{
        export::json::{json_path_for_tei_xml, write_paper_json},
        extract::grobid::{config::GrobidConfig, source::GrobidSource},
        parse::tei::reader::parse_tei_xml_path,
        transform::tei::paper_from_tei,
    },
    models::paths::pdf::PdfPath,
};

pub async fn run(pdf_path: PdfPath, tei_xml_dir: PathBuf, json_dir: PathBuf) -> Result<(), Error> {
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

    let tei_document = match parse_tei_xml_path(&tei_xml_path) {
        Ok(document) => {
            println!("Transformed TEI XML into tei document");
            document
        }
        Err(error) => {
            eprintln!("Failed to parse TEI XML: {error}");
            return Err(Error::other(error));
        }
    };

    let paper = paper_from_tei(&tei_document);
    println!("Transformed tei document to domain paper");

    let json_path = json_path_for_tei_xml(&tei_xml_path, json_dir);
    write_paper_json(&paper, &json_path).map_err(|error| {
        eprintln!("Failed to export domain paper as JSON: {error}");
        Error::other(error)
    })?;
    println!("Saved domain JSON to {}", json_path.display());

    Ok(())
}
