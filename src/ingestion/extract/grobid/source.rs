use reqwest::multipart;
use std::fs;

use crate::{
    ingestion::extract::grobid::{config::GrobidConfig, error::GrobidError},
    models::paths::pdf::PdfPath,
};

#[derive(Debug)]
pub struct GrobidSource {
    pub config: GrobidConfig,
    pub client: reqwest::Client,
}

impl GrobidSource {
    pub fn new(config: GrobidConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn extract_pdf_to_tei_xml(&self, pdf_path: &PdfPath) -> Result<String, GrobidError> {
        let file_name = pdf_path
            .as_path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| GrobidError::MissingFileName((*pdf_path).clone()))?;

        let pdf_bytes = fs::read(pdf_path.as_path()).map_err(|source| GrobidError::ReadPdf {
            path: (*pdf_path).clone(),
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

        Ok(response.text().await?)
    }
}
