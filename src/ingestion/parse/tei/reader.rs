use crate::ingestion::parse::tei::error::ParseError;
use crate::models::paths::tei_xml::TeiXmlPath;
use crate::models::tei::TeiDocument;
use std::fs;

pub fn parse_tei_xml_path(path: &TeiXmlPath) -> Result<TeiDocument, ParseError> {
    let xml = fs::read_to_string(path.as_path())?;

    parse_tei_xml_str(&xml)
}

/// Parses an in-memory TEI XML string directly, without a round trip
/// through disk -- used by the metadata server, which only ever has GROBID's
/// output in memory for an ephemeral, not-yet-stored upload.
pub fn parse_tei_xml_str(xml: &str) -> Result<TeiDocument, ParseError> {
    Ok(quick_xml::de::from_str(xml)?)
}
