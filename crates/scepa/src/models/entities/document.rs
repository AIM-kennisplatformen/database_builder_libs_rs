use serde::{Deserialize, Serialize};

use crate::models::relations::{
    contribution::Work as ContributionWork, publication_event::Work as PublicationWork,
};
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Document {
    Book(Book),
    ResearchPaper(ResearchPaper),
    Report(Report),
}

#[typedb_entity]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Book {
    pub entity_id: String,
    pub pdf_hash: Option<String>,
    pub title: Option<String>,
    pub abstract_text: Option<String>,
    pub acknowledgements: Option<String>,
    pub conflicts: Option<String>,
    pub contributions: Option<String>,
    pub isbn: Option<String>,
    pub issn: Option<String>,
}

#[typedb_relation_role(name = "book")]
impl ContributionWork for Book {}

#[typedb_relation_role(name = "book")]
impl PublicationWork for Book {}

#[typedb_entity]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct ResearchPaper {
    pub entity_id: String,
    pub pdf_hash: Option<String>,
    pub title: Option<String>,
    pub abstract_text: Option<String>,
    pub acknowledgements: Option<String>,
    pub conflicts: Option<String>,
    pub contributions: Option<String>,
    pub doi: Option<String>,
}

#[typedb_relation_role(name = "research-paper")]
impl ContributionWork for ResearchPaper {}

#[typedb_relation_role(name = "research-paper")]
impl PublicationWork for ResearchPaper {}

#[typedb_entity]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Report {
    pub entity_id: String,
    pub pdf_hash: Option<String>,
    pub title: Option<String>,
    pub abstract_text: Option<String>,
    pub acknowledgements: Option<String>,
    pub conflicts: Option<String>,
    pub contributions: Option<String>,
}

#[typedb_relation_role(name = "report")]
impl ContributionWork for Report {}

#[typedb_relation_role(name = "report")]
impl PublicationWork for Report {}
