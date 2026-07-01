use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct CoreMetadata {
    pub title: Option<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub funding_statements: Vec<String>,
    pub acknowledgements: Option<String>,
}
