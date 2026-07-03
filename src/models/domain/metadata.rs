use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct PaperMetadata {
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub funding_statements: Vec<String>,
    pub acknowledgements: Option<String>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
}
