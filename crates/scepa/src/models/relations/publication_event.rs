use serde::{Deserialize, Serialize};

use crate::models::{typedb_relation, typedb_relation_role};

#[typedb_relation_role]
pub trait Publisher: std::fmt::Debug {}

#[typedb_relation_role]
pub trait Venue: std::fmt::Debug {}

#[typedb_relation_role]
pub trait Work: std::fmt::Debug {}

pub use Work as PublishedWork;

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub enum PublicationEventRelation {
    Submission(Submission),
    Acceptance(Acceptance),
    Publication(Publication),
}

pub type PublishingRelation = PublicationEventRelation;

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct PublicationEvent {
    pub publisher: Option<Box<dyn Publisher>>,
    pub venue: Option<Box<dyn Venue>>,
    pub work: Box<dyn Work>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Submission {
    pub publisher: Option<Box<dyn Publisher>>,
    pub venue: Option<Box<dyn Venue>>,
    pub work: Box<dyn Work>,
    pub submission_date: Option<String>,
    pub submission_note: Option<String>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Acceptance {
    pub publisher: Option<Box<dyn Publisher>>,
    pub venue: Option<Box<dyn Venue>>,
    pub work: Box<dyn Work>,
    pub acceptance_date: Option<String>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Publication {
    pub publisher: Option<Box<dyn Publisher>>,
    pub venue: Option<Box<dyn Venue>>,
    pub work: Box<dyn Work>,
    pub publication_date: Option<String>,
    pub version_number: Option<String>,
}
