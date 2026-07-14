use serde::{Deserialize, Serialize};

use document::Document;
use institution::InstitutionEntity;
use person::PersonEntity;
use publication_venue::PublicationVenue;

use crate::models::typedb_model;

pub trait TypeDbEntity {
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
