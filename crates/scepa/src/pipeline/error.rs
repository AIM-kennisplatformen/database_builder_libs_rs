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
