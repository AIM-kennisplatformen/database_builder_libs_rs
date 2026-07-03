use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("failed to read TEI XML file")]
    Io(#[from] std::io::Error),

    #[error("failed to deserialize TEI XML")]
    Deserialization(#[from] quick_xml::DeError),

    #[error("file is empty or missing root tag")]
    EmptyDocument,
}
