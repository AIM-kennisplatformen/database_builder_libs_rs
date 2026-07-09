use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RorError {
    #[error("failed to call the ROR affiliation endpoint")]
    Request(#[from] reqwest::Error),

    #[error("ROR affiliation endpoint rejected the request: {body}")]
    UnsuccessfulResponse { status: StatusCode, body: String },

    #[error("failed to decode ROR affiliation response")]
    Decode(#[source] serde_json::Error),
}
