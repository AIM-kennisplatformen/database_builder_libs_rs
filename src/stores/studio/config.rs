#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StudioConfig {
    pub base_url: String,
    pub api_key: String,
}

impl StudioConfig {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
        }
    }
}
