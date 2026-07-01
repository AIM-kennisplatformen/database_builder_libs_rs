use serde::{Deserialize, Serialize};

use super::{
    attributes::{GlobalAttributes, TypedAttributes},
    header::TeiHeader,
    text::Text,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "TEI")]
pub struct TeiDocument {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "@version", default)]
    pub version: Option<String>,

    #[serde(rename = "teiHeader")]
    pub header: TeiHeader,

    #[serde(default)]
    pub text: Option<Text>,

    #[serde(rename = "TEI", default)]
    pub children: Vec<TeiDocument>,
}
