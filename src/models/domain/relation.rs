use serde::Serialize;

use super::{
    author::Author,
    institution::{Department, Institution},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Authoring {
    pub author: Author,
    pub affiliations: Vec<Affiliation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Affiliation {
    pub institution: Institution,
    pub department: Option<Department>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Publication {
    pub publisher: Institution,
}
