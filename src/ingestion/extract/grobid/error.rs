use std::path::PathBuf;

use reqwest::StatusCode;
use thiserror::Error;

use crate::models::paths::{pdf::PdfPath, tei_xml::TeiXmlPath};

#[derive(Debug, Error)]
pub enum GrobidError {
    #[error("PDF path has no file name: {0}")]
    MissingFileName(PdfPath),

    #[error("PDF path has no file stem: {0}")]
    MissingFileStem(PdfPath),

    #[error("failed to read PDF at {path}: {source}")]
    ReadPdf {
        path: PdfPath,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create GROBID output directory at {path}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write TEI XML at {path}: {source}")]
    WriteTeiXml {
        path: TeiXmlPath,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to call GROBID: {0}")]
    Request(#[from] reqwest::Error),

    #[error("GROBID returned {status}: {body}")]
    UnsuccessfulResponse { status: StatusCode, body: String },
}
