use reqwest::multipart;
use std::fs;

use crate::{
    ingestion::extract::grobid::{error::GrobidError, source::GrobidSource},
    models::paths::{pdf::PdfPath, tei_xml::TeiXmlPath},
};

pub mod config;
pub mod error;
pub mod source;

impl GrobidSource {
    pub async fn extract_pdf_to_tei_xml(
        &self,
        pdf_path: PdfPath,
    ) -> Result<TeiXmlPath, GrobidError> {
        let file_name = pdf_path
            .as_path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| GrobidError::MissingFileName(pdf_path.clone()))?;

        let pdf_bytes = fs::read(pdf_path.as_path()).map_err(|source| GrobidError::ReadPdf {
            path: pdf_path.clone(),
            source,
        })?;
        let pdf_part = multipart::Part::bytes(pdf_bytes)
            .file_name(file_name.to_owned())
            .mime_str("application/pdf")?;
        let form = multipart::Form::new().part("input", pdf_part);

        let endpoint = format!(
            "{}/api/processFulltextDocument",
            self.config.url.trim_end_matches('/')
        );
        let response = self.client.post(endpoint).multipart(form).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GrobidError::UnsuccessfulResponse { status, body });
        }

        fs::create_dir_all(&self.config.output_dir).map_err(|error| {
            GrobidError::CreateOutputDir {
                path: self.config.output_dir.clone(),
                source: error,
            }
        })?;

        let file_name_without_extension = pdf_path
            .file_stem()
            .ok_or_else(|| GrobidError::MissingFileStem(pdf_path.clone()))?;
        let tei_xml_path =
            TeiXmlPath::filename_from_stem(file_name_without_extension, &self.config.output_dir);
        let tei_xml = response.text().await?;
        fs::write(tei_xml_path.as_path(), &tei_xml).map_err(|error| GrobidError::WriteTeiXml {
            path: tei_xml_path.clone(),
            source: error,
        })?;

        Ok(tei_xml_path)
    }
}
