use std::path::PathBuf;

use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrobidError {
    #[error("PDF path has no file name: {0}")]
    MissingFileName(PathBuf),

    #[error("failed to read PDF at {path}")]
    ReadPdf {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to call GROBID")]
    Request(#[source] reqwest::Error),

    #[error("retryable GROBID failure")]
    Retryable(#[source] GrobidRetryableError),

    #[error("GROBID returned {status}: {body}")]
    Rejected { status: StatusCode, body: String },
}

#[derive(Debug, Error)]
pub enum GrobidRetryableError {
    #[error("failed to connect to GROBID")]
    Connection(#[source] reqwest::Error),

    #[error("GROBID returned {status}")]
    Unavailable { status: StatusCode },
}
