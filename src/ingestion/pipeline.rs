use std::path::PathBuf;

use crate::{
    ingestion::{
        error::PipelineError,
        export::{
            json::{json_path_for_tei_xml, write_paper_json},
            tei_xml::write_tei_xml,
        },
        extract::grobid::{config::GrobidConfig, source::GrobidSource},
        parse::tei::reader::parse_tei_xml_path,
        transform::tei::paper_from_tei,
    },
    models::paths::{pdf::PdfPath, tei_xml::TeiXmlPath},
};

pub async fn run_with_reporter<F>(
    pdf_path: PdfPath,
    tei_xml_dir: PathBuf,
    json_dir: PathBuf,
    mut report: F,
) -> Result<(), PipelineError>
where
    F: FnMut(&str),
{
    let grobid = GrobidSource::new(GrobidConfig {
        url: "http://localhost:8070".into(),
    });

    let file_stem = pdf_path
        .file_stem()
        .ok_or_else(|| PipelineError::MissingPdfFileStem {
            path: pdf_path.as_path().to_path_buf(),
        })?;
    let tei_xml_path = TeiXmlPath::filename_from_stem(file_stem, &tei_xml_dir);

    let tei_xml = match grobid.extract_pdf_to_tei_xml(&pdf_path).await {
        Ok(tei_xml) => {
            report("Extracted TEI XML with GROBID");
            tei_xml
        }
        Err(error) => {
            report(&format!("Failed to parse PDF with GROBID: {error}"));
            return Err(error.into());
        }
    };

    write_tei_xml(&tei_xml_path, &tei_xml)
        .map_err(|source| PipelineError::TeiXmlExportError { source })?;
    report(&format!("Saved TEI XML to {}", tei_xml_path.display()));

    let tei_document = match parse_tei_xml_path(&tei_xml_path) {
        Ok(document) => {
            report("Transformed TEI XML into tei document");
            document
        }
        Err(error) => {
            report(&format!("Failed to parse TEI XML: {error}"));
            return Err(error.into());
        }
    };

    let paper = paper_from_tei(&tei_document);
    report("Transformed tei document to domain paper");

    let json_path = json_path_for_tei_xml(&tei_xml_path, json_dir);
    write_paper_json(&paper, &json_path).map_err(|error| {
        report(&format!("Failed to export domain paper as JSON: {error}"));
        PipelineError::from(error)
    })?;
    report(&format!("Saved domain JSON to {}", json_path.display()));

    Ok(())
}
