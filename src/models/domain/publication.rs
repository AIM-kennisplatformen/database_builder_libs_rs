use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct PublicationIds {
    pub doi: Option<String>,
    pub isbn: Option<String>,
    pub issn: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct PublicationDetails {
    pub identifiers: PublicationIds,
    pub publisher: Option<String>,
    pub journal: Option<String>,
    pub publishing_date: Option<PublicationDate>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct PublicationDate {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
}
