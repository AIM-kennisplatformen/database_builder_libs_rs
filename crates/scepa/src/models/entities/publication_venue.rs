use serde::{Deserialize, Serialize};

use crate::models::relations::publication_event::Venue;
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum PublicationVenue {
    Journal(Journal),
    Conference(Conference),
}

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Journal {
    pub entity_id: String,
    pub venue_name: Option<String>,
    pub issn: Option<String>,
}

#[typedb_relation_role(name = "journal")]
impl Venue for Journal {}

#[typedb_entity]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Conference {
    pub entity_id: String,
    pub venue_name: Option<String>,
    pub issn: Option<String>,
}

#[typedb_relation_role(name = "conference")]
impl Venue for Conference {}
