use std::{fs, path::Path};

use reqwest::{StatusCode, multipart};
use rootcause::prelude::{Report, ResultExt};

use super::error::{GrobidError, GrobidRetryableError};

#[derive(Clone, Debug)]
pub struct GrobidClient {
    pub url: String,
    pub reqwest_client: reqwest::Client,
}

impl GrobidClient {
    pub fn new(url: String, reqwest_client: reqwest::Client) -> Self {
        Self {
            url,
            reqwest_client,
        }
    }

    pub async fn extract_pdf_to_tei_xml(&self, pdf_path: &Path) -> Result<String, Report> {
        let pdf_bytes = fs::read(pdf_path).context(format!(
            "failed to read PDF `{}` before sending it to GROBID",
            pdf_path.display()
        ))?;
        let file_name = pdf_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| GrobidError::MissingFileName(pdf_path.to_path_buf()))
            .context("failed to determine the PDF file name for GROBID")?;

        let endpoint = format!(
            "{}/api/processFulltextDocument",
            self.url.trim_end_matches('/')
        );
        let pdf_part = multipart::Part::bytes(pdf_bytes)
            .file_name(file_name.to_owned())
            .mime_str("application/pdf")
            .context("failed to create the GROBID PDF request part")?;
        let form = multipart::Form::new().part("input", pdf_part);
        let response = self
            .reqwest_client
            .post(endpoint)
            .multipart(form)
            .send()
            .await
            .map_err(|error| {
                if error.is_connect() {
                    GrobidError::Retryable(GrobidRetryableError::Connection(error))
                } else {
                    GrobidError::Request(error)
                }
            })
            .context("failed to send PDF to GROBID")?;

        let status = response.status();
        if !status.is_success() {
            if status == StatusCode::SERVICE_UNAVAILABLE {
                return Err(Err::<String, _>(GrobidError::Retryable(
                    GrobidRetryableError::Unavailable { status },
                ))
                .context("GROBID is currently unavailable")
                .unwrap_err()
                .into());
            }

            let body = response.text().await.unwrap_or_default();
            return Err(Err::<String, _>(GrobidError::Rejected { status, body })
                .context("GROBID rejected the PDF")
                .unwrap_err()
                .into());
        }

        Ok(response
            .text()
            .await
            .context("failed to read the TEI XML response from GROBID")?)
    }
}
