use reqwest::multipart::{Form, Part};

use crate::stores::studio::{config::StudioConfig, error::StudioStoreError};

/// Client for Studio's PDF store (`PUT/GET/DELETE /api/pdf/{sha256}`).
///
/// Stateless HTTP client, not a real connection like TypedbStore/QdrantStore:
/// there is nothing to connect/disconnect up front, so it's constructed
/// synchronously and every call is independent.
#[derive(Debug)]
pub struct StudioStore {
    pub config: StudioConfig,
    pub client: reqwest::Client,
}

impl StudioStore {
    pub fn new(config: StudioConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Upload a PDF to Studio under its own sha256 content hash.
    ///
    /// Studio verifies the hash server-side against the uploaded bytes, so
    /// this is a content-addressed put: re-uploading the same PDF under its
    /// own hash is a safe no-op on Studio's side.
    pub async fn upload_pdf(&self, sha256: &str, bytes: Vec<u8>) -> Result<(), StudioStoreError> {
        let endpoint = format!(
            "{}/api/pdf/{sha256}",
            self.config.base_url.trim_end_matches('/')
        );

        let part = Part::bytes(bytes)
            .file_name(format!("{sha256}.pdf"))
            .mime_str("application/pdf")?;
        let form = Form::new().part("file", part);

        let response = self
            .client
            .put(endpoint)
            .bearer_auth(&self.config.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(StudioStoreError::UnsuccessfulResponse { status, body });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn upload_pdf_fails_with_a_request_error_when_studio_is_unreachable() {
        let store = StudioStore::new(StudioConfig::new("http://localhost:0", "unused-key"));

        let result = store
            .upload_pdf("a".repeat(64).as_str(), b"%PDF-1.4 test".to_vec())
            .await;

        assert!(matches!(result, Err(StudioStoreError::Request(_))));
    }
}
