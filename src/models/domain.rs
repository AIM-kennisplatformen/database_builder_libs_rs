pub mod author;
pub mod content;
pub mod metadata;
pub mod paper;
pub mod pdf;
pub mod publication;

pub use author::{Affiliation, Author};
pub use content::{DocumentContent, Figure, Section, StructuredReference, Table};
pub use metadata::CoreMetadata;
pub use paper::Paper;
pub use pdf::{BoundingBox, PdfExtractionData, PdfProperties};
pub use publication::{PublicationContext, PublicationDate, PublicationIds};
