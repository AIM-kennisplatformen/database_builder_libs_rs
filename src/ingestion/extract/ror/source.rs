use serde::Deserialize;

use crate::ingestion::extract::ror::{config::RorConfig, error::RorError};

#[derive(Debug)]
pub struct RorSource {
    pub config: RorConfig,
    pub client: reqwest::Client,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RorMatch {
    pub ror_id: String,
    pub name: String,
    /// ROR's own organization types (e.g. `"education"`, `"government"`,
    /// `"company"`), used as a coarse signal for classifying the matched
    /// institution's kind.
    pub types: Vec<String>,
}

impl RorSource {
    pub fn new(config: RorConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Matches a free-text organization name against ROR's affiliation
    /// endpoint, which is purpose-built for "large-scale programmatic
    /// matching of complex, unstructured text strings to ROR IDs" (unlike
    /// the plain keyword-search endpoint). Only the endpoint's own `chosen`
    /// flag is used to decide on a match, per ROR's guidance not to
    /// threshold on `score` directly.
    pub async fn match_organization(&self, name: &str) -> Result<Option<RorMatch>, RorError> {
        let endpoint = format!(
            "{}/v2/organizations",
            self.config.host.trim_end_matches('/')
        );

        let response = self
            .client
            .get(endpoint)
            .query(&[("affiliation", name)])
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RorError::UnsuccessfulResponse { status, body });
        }

        let bytes = response.bytes().await?;
        let body: RorAffiliationResponse =
            serde_json::from_slice(&bytes).map_err(RorError::Decode)?;

        Ok(chosen_match(body))
    }
}

fn chosen_match(response: RorAffiliationResponse) -> Option<RorMatch> {
    response
        .items
        .into_iter()
        .find(|item| item.chosen)
        .map(|item| RorMatch {
            ror_id: item.organization.id,
            name: item
                .organization
                .names
                .into_iter()
                .map(|name| name.value)
                .next()
                .unwrap_or_default(),
            types: item.organization.types,
        })
}

#[derive(Deserialize)]
struct RorAffiliationResponse {
    items: Vec<RorAffiliationItem>,
}

#[derive(Deserialize)]
struct RorAffiliationItem {
    chosen: bool,
    organization: RorOrganization,
}

#[derive(Deserialize)]
struct RorOrganization {
    id: String,
    names: Vec<RorName>,
    #[serde(default)]
    types: Vec<String>,
}

#[derive(Deserialize)]
struct RorName {
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_chosen_item_out_of_several_candidates() {
        let body = r#"{"items":[
            {"chosen":false,"organization":{"id":"https://ror.org/00000aaaa","names":[{"value":"Not This One"}],"types":["company"]}},
            {"chosen":true,"organization":{"id":"https://ror.org/00b30xv10","names":[{"value":"Fontys University of Applied Sciences"}],"types":["education"]}}
        ]}"#;
        let response: RorAffiliationResponse =
            serde_json::from_slice(body.as_bytes()).expect("well-formed response deserializes");

        let matched = chosen_match(response).expect("one item is chosen");

        assert_eq!(matched.ror_id, "https://ror.org/00b30xv10");
        assert_eq!(matched.name, "Fontys University of Applied Sciences");
        assert_eq!(matched.types, vec!["education".to_owned()]);
    }

    #[test]
    fn defaults_to_an_empty_types_list_when_the_field_is_missing() {
        let body = r#"{"items":[
            {"chosen":true,"organization":{"id":"https://ror.org/00b30xv10","names":[{"value":"Some Org"}]}}
        ]}"#;
        let response: RorAffiliationResponse =
            serde_json::from_slice(body.as_bytes()).expect("well-formed response deserializes");

        let matched = chosen_match(response).expect("one item is chosen");

        assert_eq!(matched.types, Vec::<String>::new());
    }

    #[test]
    fn returns_none_when_no_item_is_chosen() {
        let body = r#"{"items":[
            {"chosen":false,"organization":{"id":"https://ror.org/00000aaaa","names":[{"value":"Not This One"}]}}
        ]}"#;
        let response: RorAffiliationResponse =
            serde_json::from_slice(body.as_bytes()).expect("well-formed response deserializes");

        assert_eq!(chosen_match(response), None);
    }

    #[test]
    fn returns_none_for_an_empty_items_list() {
        let response: RorAffiliationResponse =
            serde_json::from_slice(b"{\"items\":[]}").expect("well-formed response deserializes");

        assert_eq!(chosen_match(response), None);
    }
}
