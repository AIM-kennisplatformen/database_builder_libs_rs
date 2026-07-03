use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Author {
    pub forename: Option<String>,
    pub surname: Option<String>,
}
