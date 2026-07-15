use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pipeline::source::grobid::GrobidError;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("failed to read file {path} from disk")]
    ReadPdf {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to extract TEI XML from PDF {path}")]
    ExtractTeiXml {
        path: String,
        #[source]
        source: GrobidError,
    },

    #[error("failed to write debug artifact at {path}")]
    WriteDebugArtifact {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
#[error("TEI parser failed")]
pub struct TeiParseFailure;

#[derive(
    Debug, Clone, Copy, Deserialize, Eq, Error, Hash, Ord, PartialEq, Serialize, PartialOrd,
)]
#[serde(rename_all = "snake_case")]
pub enum FailureCause {
    #[error("GROBID extraction")]
    GrobidExtraction,
    #[error("TEI parsing")]
    TeiParsing,
    #[error("TypeDB export")]
    TypeDbExport,
    #[error("duplicate PDF")]
    Duplicate,
}

#[derive(Debug, Error)]
#[error("expected {cause} failure for PDF with SHA-256 hash {hash}")]
pub struct ExpectedError {
    pub hash: String,
    pub cause: FailureCause,
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}
