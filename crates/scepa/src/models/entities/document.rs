use serde::{Deserialize, Serialize};

use crate::models::relations::{contribution::ContributedWork, publishing::PublishedWork};
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub enum Document {
    Book(Book),
    ResearchPaper(ResearchPaper),
}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct Book {
    pub isbn: Option<String>,
    pub issn: Option<String>,
}

#[typedb_relation_role(name = "book")]
impl ContributedWork for Book {}

#[typedb_relation_role(name = "book")]
impl PublishedWork for Book {}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct ResearchPaper {
    pub doi: Option<String>,
}

#[typedb_relation_role(name = "research-paper")]
impl ContributedWork for ResearchPaper {}

#[typedb_relation_role(name = "research-paper")]
impl PublishedWork for ResearchPaper {}
