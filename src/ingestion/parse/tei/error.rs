use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to read TEI XML file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to deserialize TEI XML: {0}")]
    Deserialization(#[from] quick_xml::DeError),

    #[error("File is empty or missing root tag")]
    EmptyDocument,
}
