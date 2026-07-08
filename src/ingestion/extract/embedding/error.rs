use std::time::Duration;

use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("failed to call the embedding endpoint")]
    Request(#[from] reqwest::Error),

    #[error("embedding endpoint rejected the request: {body}")]
    UnsuccessfulResponse { status: StatusCode, body: String },

    #[error("embedding endpoint is rate limiting requests: {body}")]
    RateLimited {
        body: String,
        retry_after: Option<Duration>,
    },

    #[error("failed to decode embedding response")]
    Decode(#[source] serde_json::Error),

    #[error("expected {expected} embeddings, got {actual}")]
    ResponseCountMismatch { expected: usize, actual: usize },
}
