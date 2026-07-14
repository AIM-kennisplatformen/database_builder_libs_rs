use serde::{Deserialize, Serialize};

use crate::models::{typedb_relation, typedb_relation_role};

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Contribution {
    Contribution(BaseContribution),
    Authorship(Authorship),
    PeerReview(PeerReview),
}

#[typedb_relation_role]
pub trait Contributor: std::fmt::Debug {}

#[typedb_relation_role]
pub trait ContributedWork: std::fmt::Debug {}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct BaseContribution {
    pub contributor: Option<Box<dyn Contributor>>,
    pub contributed_work: Option<Box<dyn ContributedWork>>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Authorship {
    pub author: Option<Box<dyn Contributor>>,
    pub authored_work: Option<Box<dyn ContributedWork>>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct PeerReview {
    pub reviewer: Option<Box<dyn Contributor>>,
    pub reviewed_work: Option<Box<dyn ContributedWork>>,
}
