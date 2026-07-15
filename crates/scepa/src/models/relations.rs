use serde::{Deserialize, Serialize};

use citation::Citation;
use contribution::Contribution;
use publication_event::PublicationEventRelation;

use crate::models::typedb_model;

pub(crate) trait TypeDbRelation {
    fn typeql_insert_statement(&self) -> String;
}

pub mod citation;
pub mod contribution;
pub mod publication_event;

#[typedb_model(relation)]
#[derive(Debug, Serialize, Deserialize)]
pub enum Relation {
    Contribution(Contribution),
    Citation(Citation),
    PublicationEvent(PublicationEventRelation),
}
