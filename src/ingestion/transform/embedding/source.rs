use std::time::Duration;

use reqwest::{Response, StatusCode, header::RETRY_AFTER};
use serde::{Deserialize, Serialize};

use crate::ingestion::transform::embedding::{config::EmbeddingConfig, error::EmbeddingError};

const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);
const MAX_RETRY_ATTEMPTS: u32 = 5;

#[derive(Debug)]
pub struct EmbeddingSource {
    pub config: EmbeddingConfig,
    pub client: reqwest::Client,
}

#[derive(Serialize)]
struct EmbeddingsRequest<'a> {
    model: &'a str,
    input: &'a [String],
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

impl EmbeddingSource {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Embeds `inputs` in a single batched request, returning one vector per
    /// input in the same order they were given. Retries with backoff on
    /// `429 Too Many Requests`, honoring the endpoint's requested delay
    /// (either a `Retry-After` header or a "retry after N sec" hint in the
    /// response body) when it provides one.
    pub async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if inputs.is_empty() {
            return Ok(vec![]);
        }

        let mut attempt = 1;
        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            match self.embed_once(inputs).await {
                Ok(vectors) => return Ok(vectors),
                Err(EmbeddingError::RateLimited { retry_after, .. })
                    if attempt < MAX_RETRY_ATTEMPTS =>
                {
                    tokio::time::sleep(retry_after.unwrap_or(retry_delay)).await;
                    attempt += 1;
                    retry_delay = next_retry_delay(retry_delay);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn embed_once(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let endpoint = format!("{}embeddings", self.config.host);
        let body = serde_json::to_vec(&EmbeddingsRequest {
            model: &self.config.model,
            input: inputs,
        })
        .map_err(EmbeddingError::Decode)?;

        let response = self
            .client
            .post(endpoint)
            .bearer_auth(&self.config.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status = response.status();

        match status {
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = retry_after_header(&response);
                let body = response.text().await.unwrap_or_default();
                let retry_after = retry_after.or_else(|| retry_after_from_body(&body));

                return Err(EmbeddingError::RateLimited { body, retry_after });
            }
            _ if !status.is_success() => {
                let body = response.text().await.unwrap_or_default();
                return Err(EmbeddingError::UnsuccessfulResponse { status, body });
            }
            _ => {}
        }

        let bytes = response.bytes().await?;
        let mut body: EmbeddingsResponse =
            serde_json::from_slice(&bytes).map_err(EmbeddingError::Decode)?;

        if body.data.len() != inputs.len() {
            return Err(EmbeddingError::ResponseCountMismatch {
                expected: inputs.len(),
                actual: body.data.len(),
            });
        }

        body.data.sort_by_key(|item| item.index);

        Ok(body.data.into_iter().map(|item| item.embedding).collect())
    }
}

fn next_retry_delay(current: Duration) -> Duration {
    (current * 2).min(MAX_RETRY_DELAY)
}

fn retry_after_header(response: &Response) -> Option<Duration> {
    response
        .headers()
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

/// Parses provider-specific messages like "Retry the request after 2 sec."
/// out of the response body, for endpoints that don't set `Retry-After`.
fn retry_after_from_body(body: &str) -> Option<Duration> {
    const MARKER: &str = "after ";

    let start = body.find(MARKER)? + MARKER.len();
    let rest = &body[start..];
    let digits_end = rest
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(rest.len());

    if digits_end == 0 {
        return None;
    }

    rest[..digits_end]
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_doubles_until_the_maximum() {
        assert_eq!(
            next_retry_delay(Duration::from_secs(1)),
            Duration::from_secs(2)
        );
        assert_eq!(next_retry_delay(MAX_RETRY_DELAY), MAX_RETRY_DELAY);
    }

    #[test]
    fn parses_the_retry_hint_out_of_the_nebius_style_body() {
        let body = r#"{"detail":"You exceeded your tokens quota for `Qwen/Qwen3-Embedding-8B`. Retry the request after 2 sec. If you need more quota, you can request it at https://tokenfactory.nebius.com"}"#;

        assert_eq!(retry_after_from_body(body), Some(Duration::from_secs(2)));
    }

    #[test]
    fn returns_none_when_the_body_has_no_retry_hint() {
        assert_eq!(retry_after_from_body("no hint here"), None);
        assert_eq!(retry_after_from_body("retry after soon"), None);
    }

    #[test]
    fn embeddings_response_deserializes_and_reorders_by_index() {
        // Providers aren't guaranteed to return embeddings in request order,
        // which is why `embed_once` sorts by `index` after deserializing.
        let body = r#"{"data":[
            {"embedding":[0.2,0.2],"index":1},
            {"embedding":[0.1,0.1],"index":0}
        ]}"#;

        let mut response: EmbeddingsResponse = serde_json::from_slice(body.as_bytes())
            .expect("well-formed embeddings response should deserialize");
        response.data.sort_by_key(|item| item.index);

        let vectors: Vec<Vec<f32>> = response
            .data
            .into_iter()
            .map(|item| item.embedding)
            .collect();

        assert_eq!(vectors, vec![vec![0.1, 0.1], vec![0.2, 0.2]]);
    }

    #[tokio::test]
    async fn embed_returns_immediately_for_empty_input_without_making_a_request() {
        let source = EmbeddingSource::new(EmbeddingConfig::new(
            "http://localhost:0",
            "unused-key",
            "unused-model",
        ));

        let result = source
            .embed(&[])
            .await
            .expect("empty input never makes a request");

        let expected: Vec<Vec<f32>> = vec![];
        assert_eq!(result, expected);
    }
}
