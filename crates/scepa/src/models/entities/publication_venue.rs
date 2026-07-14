use serde::{Deserialize, Serialize};

use crate::models::relations::publishing::Venue;
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "attrs")]
pub enum PublicationVenue {
    Journal(Journal),
    Conference(Conference),
}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct Journal {
    pub venue_name: Option<String>,
}

#[typedb_relation_role(name = "journal")]
impl Venue for Journal {}

#[typedb_entity]
#[derive(Serialize, Deserialize, Debug)]
pub struct Conference {
    pub venue_name: Option<String>,
}

#[typedb_relation_role(name = "conference")]
impl Venue for Conference {}
