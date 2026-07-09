use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Project {
    pub name: Option<String>,
    pub number: Option<i64>,
}
