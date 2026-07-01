use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalAttributes {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@n", default)]
    pub n: Option<String>,

    #[serde(rename = "xml:lang", default)]
    pub xml_lang: Option<String>,

    #[serde(rename = "xml:base", default)]
    pub xml_base: Option<String>,

    #[serde(rename = "xml:space", default)]
    pub xml_space: Option<XmlSpace>,

    #[serde(rename = "@ana", default)]
    pub ana: Option<String>,

    #[serde(rename = "@change", default)]
    pub change: Option<String>,

    #[serde(rename = "@facs", default)]
    pub facs: Option<String>,

    #[serde(rename = "@corresp", default)]
    pub corresp: Option<String>,

    #[serde(rename = "@synch", default)]
    pub synch: Option<String>,

    #[serde(rename = "@sameAs", default)]
    pub same_as: Option<String>,

    #[serde(rename = "@copyOf", default)]
    pub copy_of: Option<String>,

    #[serde(rename = "@next", default)]
    pub next: Option<String>,

    #[serde(rename = "@prev", default)]
    pub prev: Option<String>,

    #[serde(rename = "@exclude", default)]
    pub exclude: Option<String>,

    #[serde(rename = "@select", default)]
    pub select: Option<String>,

    #[serde(rename = "@rend", default)]
    pub rend: Option<String>,

    #[serde(rename = "@style", default)]
    pub style: Option<String>,

    #[serde(rename = "@rendition", default)]
    pub rendition: Option<String>,

    #[serde(rename = "@cert", default)]
    pub cert: Option<String>,

    #[serde(rename = "@resp", default)]
    pub resp: Option<String>,

    #[serde(rename = "@source", default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum XmlSpace {
    #[serde(rename = "default")]
    DefaultValue,
    Preserve,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedAttributes {
    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetAttributes {
    #[serde(rename = "@target", default)]
    pub target: Option<String>,

    #[serde(rename = "@ref", default)]
    pub reference: Option<String>,

    #[serde(rename = "@key", default)]
    pub key: Option<String>,
}
