use serde::Serialize;

use super::{
    content::DocumentContent, graph::PaperGraph, metadata::PaperMetadata, pdf::PdfExtractionData,
    source::SourceHash,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Paper {
    pub source: SourceHash,
    pub graph: PaperGraph,
    pub metadata: PaperMetadata,
    pub content: DocumentContent,
    pub extraction_data: PdfExtractionData,
}
