use serde::{Deserialize, Serialize};

use contribution::Contribution;
use publication_event::PublicationEventRelation;

pub mod contribution;
pub mod publication_event;

#[derive(Debug, Serialize, Deserialize)]
pub enum Relation {
    Contribution(Contribution),
    PublicationEvent(PublicationEventRelation),
}
