use serde::{Deserialize, Serialize};

use crate::models::{typedb_relation, typedb_relation_role};

#[typedb_relation_role]
pub trait Publisher: std::fmt::Debug {}

#[typedb_relation_role]
pub trait Venue: std::fmt::Debug {}

#[typedb_relation_role]
pub trait PublishedWork: std::fmt::Debug {}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub enum PublishingRelation {
    Publishing(Publishing),
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Publishing {
    pub publisher: Option<Box<dyn Publisher>>,
    pub venue: Option<Box<dyn Venue>>,
    pub published_work: Box<dyn PublishedWork>,
    pub version_number: Option<String>,
    pub publication_date: Option<String>,
}
