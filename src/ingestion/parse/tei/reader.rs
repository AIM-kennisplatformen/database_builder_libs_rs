use crate::ingestion::parse::tei::error::ParseError;
use crate::models::paths::tei_xml::TeiXmlPath;
use crate::models::tei::TeiDocument;
use std::fs::File;
use std::io::BufReader;

pub fn parse_tei_xml_path(path: &TeiXmlPath) -> Result<TeiDocument, ParseError> {
    let file = File::open(path.as_path())?;
    let reader = BufReader::new(file);

    let document = quick_xml::de::from_reader(reader)?;

    Ok(document)
}
