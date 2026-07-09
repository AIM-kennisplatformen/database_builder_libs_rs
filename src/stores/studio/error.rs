use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StudioStoreError {
    #[error("failed to call Studio's PDF upload endpoint")]
    Request(#[from] reqwest::Error),

    #[error("Studio rejected the PDF upload with status {status}: {body}")]
    UnsuccessfulResponse { status: StatusCode, body: String },
}
