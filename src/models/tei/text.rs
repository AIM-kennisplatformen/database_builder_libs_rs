use serde::{Deserialize, Serialize};

use super::{
    attributes::{GlobalAttributes, TargetAttributes, TypedAttributes},
    bibliography::{BiblStruct, Idno, OrgName, Ptr},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(default)]
    pub front: Option<TextPart>,

    #[serde(default)]
    pub body: Option<TextPart>,

    #[serde(default)]
    pub back: Option<TextPart>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPart {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@n", default)]
    pub n: Option<String>,

    #[serde(rename = "xml:lang", default)]
    pub xml_lang: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Block>,
}

pub type Front = TextPart;
pub type Body = TextPart;
pub type Back = TextPart;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    #[serde(rename = "div")]
    Division(Division),

    #[serde(rename = "p")]
    Paragraph(Paragraph),

    #[serde(rename = "head")]
    Head(Head),

    #[serde(rename = "list")]
    List(List),

    #[serde(rename = "listBibl")]
    ListBibl(ListBibl),

    #[serde(rename = "listOrg")]
    ListOrg(ListOrg),

    #[serde(rename = "figure")]
    Figure(Figure),

    #[serde(rename = "table")]
    Table(Table),

    #[serde(rename = "note")]
    Note(Paragraph),

    #[serde(rename = "ref")]
    Reference(Reference),

    #[serde(rename = "formula")]
    Formula(Formula),

    #[serde(rename = "$text")]
    Text(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Division {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@n", default)]
    pub n: Option<String>,

    #[serde(rename = "xml:lang", default)]
    pub xml_lang: Option<String>,

    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Block>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@n", default)]
    pub n: Option<String>,

    #[serde(rename = "xml:lang", default)]
    pub xml_lang: Option<String>,

    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@n", default)]
    pub n: Option<String>,

    #[serde(rename = "xml:lang", default)]
    pub xml_lang: Option<String>,

    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Inline {
    #[serde(rename = "p")]
    Paragraph(Box<Paragraph>),

    #[serde(rename = "ref")]
    Reference(Reference),

    #[serde(rename = "ptr")]
    Pointer(Ptr),

    #[serde(rename = "rs")]
    ReferencingString(ReferencingString),

    #[serde(rename = "hi")]
    Highlighted(Highlighted),

    #[serde(rename = "lb")]
    LineBreak(Break),

    #[serde(rename = "pb")]
    PageBreak(Break),

    #[serde(rename = "note")]
    Note(Box<Paragraph>),

    #[serde(rename = "formula")]
    Formula(Formula),

    #[serde(rename = "figure")]
    Figure(Box<Figure>),

    #[serde(rename = "$text")]
    Text(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,

    #[serde(rename = "@target", default)]
    pub target: Option<String>,

    #[serde(rename = "@ref", default)]
    pub reference: Option<String>,

    #[serde(rename = "@key", default)]
    pub key: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferencingString {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,

    #[serde(rename = "@target", default)]
    pub target: Option<String>,

    #[serde(rename = "@ref", default)]
    pub reference: Option<String>,

    #[serde(rename = "@key", default)]
    pub key: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Highlighted {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@type", default)]
    pub kind: Option<String>,

    #[serde(rename = "@subtype", default)]
    pub subtype: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Break {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@ed", default)]
    pub edition: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Formula {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "head", default)]
    pub heads: Vec<Head>,

    #[serde(rename = "item", default)]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@n", default)]
    pub n: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBibl {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "head", default)]
    pub heads: Vec<Head>,

    #[serde(rename = "biblStruct", default)]
    pub bibliographic_structures: Vec<BiblStruct>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListOrg {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "org", default)]
    pub organizations: Vec<Organization>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "orgName", default)]
    pub organization_names: Vec<OrgName>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Figure {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "head", default)]
    pub heads: Vec<Head>,

    #[serde(rename = "label", default)]
    pub labels: Vec<Label>,

    #[serde(rename = "figDesc", default)]
    pub descriptions: Vec<Paragraph>,

    #[serde(rename = "graphic", default)]
    pub graphics: Vec<Graphic>,

    #[serde(rename = "p", default)]
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graphic {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@url", default)]
    pub url: Option<String>,

    #[serde(flatten, default)]
    pub target: TargetAttributes,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "head", default)]
    pub heads: Vec<Head>,

    #[serde(rename = "row", default)]
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@role", default)]
    pub role: Option<String>,

    #[serde(rename = "cell", default)]
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    #[serde(rename = "xml:id", default)]
    pub xml_id: Option<String>,

    #[serde(rename = "@role", default)]
    pub role: Option<String>,

    #[serde(rename = "@cols", default)]
    pub columns: Option<String>,

    #[serde(rename = "@rows", default)]
    pub rows: Option<String>,

    #[serde(rename = "$value", default)]
    pub content: Vec<Inline>,
}
