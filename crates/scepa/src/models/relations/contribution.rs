use serde::{Deserialize, Serialize};

use crate::models::entities::TypeDbEntity;
use crate::models::{typedb_relation, typedb_relation_role};

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub enum Contribution {
    Contribution(BaseContribution),
    Authorship(Authorship),
    PeerReview(PeerReview),
}

#[typedb_relation_role]
pub trait Contributor: std::fmt::Debug + TypeDbEntity {}

#[typedb_relation_role]
pub trait Work: std::fmt::Debug + TypeDbEntity {}

pub use Work as ContributedWork;

#[typedb_relation(name = "contribution")]
#[derive(Serialize, Deserialize, Debug)]
pub struct BaseContribution {
    pub contributor: Option<Box<dyn Contributor>>,
    pub work: Option<Box<dyn Work>>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct Authorship {
    pub author: Option<Box<dyn Contributor>>,
    pub authored_work: Option<Box<dyn Work>>,
}

#[typedb_relation]
#[derive(Serialize, Deserialize, Debug)]
pub struct PeerReview {
    pub reviewer: Option<Box<dyn Contributor>>,
    pub reviewed_work: Option<Box<dyn Work>>,
}
