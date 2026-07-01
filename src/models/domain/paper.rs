use serde::Serialize;

use super::{
    author::Author, content::DocumentContent, metadata::CoreMetadata, pdf::PdfExtractionData,
    publication::PublicationContext,
};

#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct Paper {
    pub core: CoreMetadata,
    pub publication: PublicationContext,
    pub authors: Vec<Author>,
    pub content: DocumentContent,
    pub extraction_data: PdfExtractionData,
}
