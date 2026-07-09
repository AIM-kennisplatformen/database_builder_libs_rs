use std::collections::HashMap;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

/// "app-name:key,other-app:key" pairs authorizing machine clients (e.g.
/// upload_interface) to call this server's endpoints.
#[derive(Clone, Debug, Default)]
pub struct ApiKeys(HashMap<String, String>);

impl ApiKeys {
    pub fn parse(raw: &str) -> Self {
        let mut keys = HashMap::new();

        for entry in raw.split(',') {
            if let Some((name, key)) = entry.trim().split_once(':')
                && !name.is_empty()
                && !key.is_empty()
            {
                keys.insert(name.to_owned(), key.to_owned());
            }
        }

        Self(keys)
    }

    fn verify(&self, presented: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(_, key)| constant_time_eq(presented.as_bytes(), key.as_bytes()))
            .map(|(name, _)| name.as_str())
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Extractor authenticating a request via a static bearer API key, yielding
/// the calling application's configured name (for logging/auditing).
pub struct AuthorizedApp(pub String);

pub enum AuthError {
    Missing,
    Invalid,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let message = match self {
            AuthError::Missing => "missing or malformed Authorization header",
            AuthError::Invalid => "invalid API key",
        };

        (StatusCode::UNAUTHORIZED, message).into_response()
    }
}

impl<S> FromRequestParts<S> for AuthorizedApp
where
    S: Send + Sync,
    ApiKeys: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let keys = ApiKeys::from_ref(state);

        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AuthError::Missing)?;

        let presented = header.strip_prefix("Bearer ").ok_or(AuthError::Missing)?;

        keys.verify(presented)
            .map(|name| AuthorizedApp(name.to_owned()))
            .ok_or(AuthError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_multiple_app_key_pairs() {
        let keys = ApiKeys::parse("upload-interface:secret-one,scepa-rs:secret-two");

        assert_eq!(keys.verify("secret-one"), Some("upload-interface"));
        assert_eq!(keys.verify("secret-two"), Some("scepa-rs"));
        assert_eq!(keys.verify("wrong"), None);
    }

    #[test]
    fn parse_ignores_malformed_entries() {
        let keys = ApiKeys::parse("no-colon-here, :missing-name, missing-key:,valid:key");

        assert_eq!(keys.verify("key"), Some("valid"));
        assert!(keys.verify("no-colon-here").is_none());
    }

    #[test]
    fn empty_configuration_authorizes_nothing() {
        let keys = ApiKeys::parse("");

        assert!(keys.verify("anything").is_none());
    }
}
