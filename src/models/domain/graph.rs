use serde::Serialize;

use super::{
    literature::Literature,
    relation::{Authoring, Publication},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaperGraph {
    pub literature: Literature,
    pub authorings: Vec<Authoring>,
    pub publications: Vec<Publication>,
}
