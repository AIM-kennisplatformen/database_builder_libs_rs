#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingConfig {
    pub host: String,
    pub api_key: String,
    pub model: String,
}

impl EmbeddingConfig {
    /// Normalizes `host` to always end with `/`, so call sites can append a
    /// path directly (`format!("{host}embeddings")`) instead of trimming a
    /// trailing slash on every request.
    pub fn new(
        host: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let mut host = host.into();
        if !host.ends_with('/') {
            host.push('/');
        }

        Self {
            host,
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_trailing_slash_when_the_host_is_missing_one() {
        let config = EmbeddingConfig::new("https://api.example.com/v1", "key", "model");

        assert_eq!(config.host, "https://api.example.com/v1/");
    }

    #[test]
    fn leaves_a_host_that_already_ends_with_a_slash_unchanged() {
        let config = EmbeddingConfig::new("https://api.example.com/v1/", "key", "model");

        assert_eq!(config.host, "https://api.example.com/v1/");
    }
}
