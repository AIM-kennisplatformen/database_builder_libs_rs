use thiserror::Error;

use crate::ingestion::{
    export::{
        json::JsonExportError, qdrant::QdrantExportError, tei_xml::TeiXmlExportError,
        typedb::TypedbExportError,
    },
    extract::grobid::error::GrobidError,
    parse::tei::error::ParseError as TeiParseError,
};

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("PDF path has no file stem: {path}")]
    MissingPdfFileStem { path: std::path::PathBuf },

    #[error("failed to extract TEI XML with GROBID")]
    ExtractError {
        #[from]
        source: GrobidError,
    },

    #[error("failed to write TEI XML")]
    TeiXmlExportError {
        #[from]
        source: TeiXmlExportError,
    },

    #[error("failed to parse TEI XML")]
    ParseError {
        #[from]
        source: TeiParseError,
    },

    #[error("failed to export domain paper as JSON")]
    JsonExportError {
        #[from]
        source: JsonExportError,
    },

    #[error("failed to export domain paper to TypeDB")]
    TypedbExportError {
        #[from]
        source: TypedbExportError,
    },

    #[error("failed to export domain paper to Qdrant")]
    QdrantExportError {
        #[from]
        source: QdrantExportError,
    },
}
