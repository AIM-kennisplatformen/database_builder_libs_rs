#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RorConfig {
    pub host: String,
}

impl RorConfig {
    pub fn new(host: impl Into<String>) -> Self {
        Self { host: host.into() }
    }
}
