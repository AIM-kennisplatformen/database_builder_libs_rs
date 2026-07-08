#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingConfig {
    pub host: String,
    pub api_key: String,
    pub model: String,
}

impl EmbeddingConfig {
    pub fn new(
        host: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}
