use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct Author {
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub affiliations: Vec<Affiliation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct Affiliation {
    pub laboratory: Option<String>,
    pub department: Option<String>,
    pub institution: Option<String>,
    pub settlement: Option<String>,
    pub country: Option<String>,
}
