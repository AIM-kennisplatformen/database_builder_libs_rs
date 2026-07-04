use serde::Serialize;

use super::{
    literature::Literature,
    relation::{Authoring, Citation, Publication},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaperGraph {
    pub literature: Literature,
    pub authorings: Vec<Authoring>,
    pub publications: Vec<Publication>,
    pub citations: Vec<Citation>,
}
