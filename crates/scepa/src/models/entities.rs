use serde::{Deserialize, Serialize};

use document::Document;
use institution::InstitutionEntity;
use person::PersonEntity;
use publication_venue::PublicationVenue;

pub mod document;
pub mod institution;
pub mod person;
pub mod publication_venue;

#[derive(Debug, Serialize, Deserialize)]
pub enum Entity {
    Document(Document),
    Person(PersonEntity),
    Institution(InstitutionEntity),
    PublicationVenue(PublicationVenue),
}
