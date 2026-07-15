use serde::{Deserialize, Serialize};

use crate::models::relations::contribution::Contributor;
use crate::models::{typedb_entity, typedb_relation_role};

#[typedb_entity]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum PersonEntity {
    Person(Person),
}

#[typedb_entity]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Person {
    pub entity_id: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub orcid: Option<String>,
}

#[typedb_relation_role(name = "person")]
impl Contributor for Person {}
