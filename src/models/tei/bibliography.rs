use serde::{Deserialize, Serialize};

use super::attributes::{GlobalAttributes, TargetAttributes, TypedAttributes};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiblStruct {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "@status", default)]
    pub status: Option<String>,

    #[serde(default)]
    pub analytic: Option<Analytic>,

    #[serde(rename = "monogr", default)]
    pub monographs: Vec<Monogr>,

    #[serde(default)]
    pub series: Option<Series>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "ptr", default)]
    pub pointers: Vec<Ptr>,

    #[serde(rename = "note", default)]
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analytic {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "title", default)]
    pub titles: Vec<Title>,

    #[serde(rename = "author", default)]
    pub authors: Vec<Author>,

    #[serde(rename = "editor", default)]
    pub editors: Vec<Author>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "ptr", default)]
    pub pointers: Vec<Ptr>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monogr {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "title", default)]
    pub titles: Vec<Title>,

    #[serde(rename = "author", default)]
    pub authors: Vec<Author>,

    #[serde(rename = "editor", default)]
    pub editors: Vec<Author>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "ptr", default)]
    pub pointers: Vec<Ptr>,

    #[serde(default)]
    pub imprint: Option<Imprint>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Series {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "title", default)]
    pub titles: Vec<Title>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(default)]
    pub imprint: Option<Imprint>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imprint {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "publisher", default)]
    pub publishers: Vec<Publisher>,

    #[serde(rename = "pubPlace", default)]
    pub publication_places: Vec<PlaceName>,

    #[serde(rename = "date", default)]
    pub dates: Vec<Date>,

    #[serde(rename = "biblScope", default)]
    pub bibliographic_scopes: Vec<BiblScope>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Title {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "@level", default)]
    pub level: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "@role", default)]
    pub role: Option<String>,

    #[serde(rename = "persName", default)]
    pub person_names: Vec<PersonName>,

    #[serde(rename = "orgName", default)]
    pub organization_names: Vec<OrgName>,

    #[serde(rename = "email", default)]
    pub emails: Vec<Email>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "affiliation", default)]
    pub affiliations: Vec<Affiliation>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonName {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "forename", default)]
    pub forenames: Vec<NamePart>,

    #[serde(rename = "surname", default)]
    pub surnames: Vec<NamePart>,

    #[serde(rename = "genName", default)]
    pub generation_names: Vec<NamePart>,

    #[serde(rename = "addName", default)]
    pub additional_names: Vec<NamePart>,

    #[serde(rename = "roleName", default)]
    pub role_names: Vec<NamePart>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamePart {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Affiliation {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@key", default)]
    pub key: Option<String>,

    #[serde(rename = "orgName", default)]
    pub organization_names: Vec<OrgName>,

    #[serde(default)]
    pub address: Option<Address>,

    #[serde(rename = "note", default)]
    pub notes: Vec<Note>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "addrLine", default)]
    pub address_lines: Vec<TextElement>,

    #[serde(rename = "settlement", default)]
    pub settlements: Vec<TextElement>,

    #[serde(rename = "region", default)]
    pub regions: Vec<TextElement>,

    #[serde(rename = "postCode", default)]
    pub post_codes: Vec<TextElement>,

    #[serde(rename = "country", default)]
    pub countries: Vec<Country>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Country {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@key", default)]
    pub key: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgName {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "@key", default)]
    pub key: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Idno {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ptr {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub target: TargetAttributes,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Date {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "@when", default)]
    pub when: Option<String>,

    #[serde(rename = "@notBefore", default)]
    pub not_before: Option<String>,

    #[serde(rename = "@notAfter", default)]
    pub not_after: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiblScope {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@unit", default)]
    pub unit: Option<String>,

    #[serde(rename = "@from", default)]
    pub from_value: Option<String>,

    #[serde(rename = "@to", default)]
    pub to_value: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Publisher {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceName {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextElement {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}
