use thiserror::Error;

use crate::ingestion::{
    export::json::JsonExportError, extract::grobid::error::GrobidError,
    parse::tei::error::ParseError as TeiParseError,
};

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("PDF path has no file stem: {path}")]
    MissingPdfFileStem { path: std::path::PathBuf },

    #[error("failed to extract TEI XML with GROBID: {source}")]
    ExtractError {
        #[from]
        source: GrobidError,
    },

    #[error("failed to write TEI XML: {source}")]
    TeiXmlExportError {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TEI XML: {source}")]
    ParseError {
        #[from]
        source: TeiParseError,
    },

    #[error("failed to export domain paper as JSON: {source}")]
    JsonExportError {
        #[from]
        source: JsonExportError,
    },
}
