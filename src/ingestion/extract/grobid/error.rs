use reqwest::StatusCode;
use thiserror::Error;

use crate::models::paths::pdf::PdfPath;

#[derive(Debug, Error)]
pub enum GrobidError {
    #[error("PDF path has no file name: {0}")]
    MissingFileName(PdfPath),

    #[error("failed to read PDF at {path}: {source}")]
    ReadPdf {
        path: PdfPath,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to call GROBID: {0}")]
    Request(#[from] reqwest::Error),

    #[error("GROBID returned {status}: {body}")]
    UnsuccessfulResponse { status: StatusCode, body: String },
}
