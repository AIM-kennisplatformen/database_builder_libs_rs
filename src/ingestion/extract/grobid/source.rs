use super::config::GrobidConfig;

#[derive(Debug)]
pub struct GrobidSource {
    pub config: GrobidConfig,
    pub client: reqwest::Client,
}

impl GrobidSource {
    pub fn new(config: GrobidConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}
