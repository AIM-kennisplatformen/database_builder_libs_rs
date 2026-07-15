use serde::{Deserialize, Serialize};

use document::Document;
use institution::InstitutionEntity;
use person::PersonEntity;
use publication_venue::PublicationVenue;

use crate::models::typedb_model;

pub trait TypeDbEntity {
    fn typeql_type(&self) -> &'static str;
    fn entity_id(&self) -> &str;
    fn typeql_identity_pattern(&self, variable: &str) -> String;
    fn typeql_metadata_statements(&self) -> Vec<String>;

    fn typeql_insert_statement(&self, variable: &str) -> String;
}

pub mod document;
pub mod institution;
pub mod person;
pub mod publication_venue;

#[typedb_model(entity)]
#[derive(Debug, Serialize, Deserialize)]
pub enum Entity {
    Document(Document),
    Person(PersonEntity),
    Institution(InstitutionEntity),
    PublicationVenue(PublicationVenue),
}
