use crate::models::{
    domain::{Paper, PdfExtractionData},
    tei::{bibliography::BiblStruct, document::TeiDocument},
};

use super::{
    authors::authors_from_tei, content::document_content_from_tei,
    metadata::core_metadata_from_tei, publication::publication_context_from_tei,
};

impl From<&TeiDocument> for Paper {
    fn from(document: &TeiDocument) -> Self {
        paper_from_tei(document)
    }
}

pub fn paper_from_tei(document: &TeiDocument) -> Paper {
    let source_bibl = source_bibl(document);

    Paper {
        core: core_metadata_from_tei(document, source_bibl),
        publication: publication_context_from_tei(document, source_bibl),
        authors: authors_from_tei(document, source_bibl),
        content: document_content_from_tei(document),
        extraction_data: PdfExtractionData::default(),
    }
}

fn source_bibl(document: &TeiDocument) -> Option<&BiblStruct> {
    document
        .header
        .file_desc
        .source_desc
        .bibliographic_structures
        .first()
}
