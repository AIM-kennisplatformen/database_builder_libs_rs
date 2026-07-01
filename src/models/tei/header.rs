use serde::{Deserialize, Serialize};

use super::{
    attributes::{GlobalAttributes, TargetAttributes, TypedAttributes},
    bibliography::{Author, BiblStruct, Date, Idno, Note, OrgName, Publisher, TextElement, Title},
    text::{Division, ListBibl},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeiHeader {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "fileDesc")]
    pub file_desc: FileDesc,

    #[serde(rename = "encodingDesc", default)]
    pub encoding_desc: Option<EncodingDesc>,

    #[serde(rename = "profileDesc", default)]
    pub profile_desc: Option<ProfileDesc>,

    #[serde(rename = "revisionDesc", default)]
    pub revision_desc: Option<RevisionDesc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDesc {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "titleStmt")]
    pub title_stmt: TitleStmt,

    #[serde(rename = "editionStmt", default)]
    pub edition_stmt: Option<EditionStmt>,

    #[serde(rename = "extent", default)]
    pub extent: Option<TextElement>,

    #[serde(rename = "publicationStmt")]
    pub publication_stmt: PublicationStmt,

    #[serde(rename = "seriesStmt", default)]
    pub series_stmt: Option<SeriesStmt>,

    #[serde(rename = "notesStmt", default)]
    pub notes_stmt: Option<NotesStmt>,

    #[serde(rename = "sourceDesc")]
    pub source_desc: SourceDesc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitleStmt {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "title", default)]
    pub titles: Vec<Title>,

    #[serde(rename = "author", default)]
    pub authors: Vec<Author>,

    #[serde(rename = "editor", default)]
    pub editors: Vec<Author>,

    #[serde(rename = "sponsor", default)]
    pub sponsors: Vec<OrganizationLike>,

    #[serde(rename = "funder", default)]
    pub funders: Vec<Funder>,

    #[serde(rename = "principal", default)]
    pub principals: Vec<OrganizationLike>,

    #[serde(rename = "respStmt", default)]
    pub responsibility_statements: Vec<RespStmt>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Funder {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub target: TargetAttributes,

    #[serde(rename = "orgName", default)]
    pub organization_names: Vec<OrgName>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationLike {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "orgName", default)]
    pub organization_names: Vec<OrgName>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RespStmt {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "resp", default)]
    pub responsibilities: Vec<TextElement>,

    #[serde(rename = "name", default)]
    pub names: Vec<TextElement>,

    #[serde(rename = "persName", default)]
    pub person_names: Vec<super::bibliography::PersonName>,

    #[serde(rename = "orgName", default)]
    pub organization_names: Vec<OrgName>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditionStmt {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "edition", default)]
    pub editions: Vec<TextElement>,

    #[serde(rename = "respStmt", default)]
    pub responsibility_statements: Vec<RespStmt>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationStmt {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "publisher", default)]
    pub publishers: Vec<Publisher>,

    #[serde(rename = "distributor", default)]
    pub distributors: Vec<OrganizationLike>,

    #[serde(rename = "authority", default)]
    pub authorities: Vec<OrganizationLike>,

    #[serde(rename = "pubPlace", default)]
    pub publication_places: Vec<TextElement>,

    #[serde(rename = "address", default)]
    pub addresses: Vec<super::bibliography::Address>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "availability", default)]
    pub availability: Option<Availability>,

    #[serde(rename = "date", default)]
    pub dates: Vec<Date>,

    #[serde(rename = "p", default)]
    pub paragraphs: Vec<super::text::Paragraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Availability {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@status", default)]
    pub status: Option<String>,

    #[serde(rename = "licence", default)]
    pub licences: Vec<Licence>,

    #[serde(rename = "p", default)]
    pub paragraphs: Vec<super::text::Paragraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Licence {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub target: TargetAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesStmt {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "title", default)]
    pub titles: Vec<Title>,

    #[serde(rename = "idno", default)]
    pub identifiers: Vec<Idno>,

    #[serde(rename = "respStmt", default)]
    pub responsibility_statements: Vec<RespStmt>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesStmt {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "note", default)]
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDesc {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "biblStruct", default)]
    pub bibliographic_structures: Vec<BiblStruct>,

    #[serde(rename = "listBibl", default)]
    pub bibliographies: Vec<ListBibl>,

    #[serde(rename = "bibl", default)]
    pub bibliographic_citations: Vec<TextElement>,

    #[serde(rename = "p", default)]
    pub paragraphs: Vec<super::text::Paragraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodingDesc {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "appInfo", default)]
    pub app_info: Option<AppInfo>,

    #[serde(rename = "projectDesc", default)]
    pub project_desc: Option<DescriptionBlock>,

    #[serde(rename = "samplingDecl", default)]
    pub sampling_decl: Option<DescriptionBlock>,

    #[serde(rename = "editorialDecl", default)]
    pub editorial_decl: Option<DescriptionBlock>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "application", default)]
    pub applications: Vec<Application>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@version", default)]
    pub version: Option<String>,

    #[serde(rename = "@ident", default)]
    pub ident: Option<String>,

    #[serde(rename = "@when", default)]
    pub when: Option<String>,

    #[serde(rename = "desc", default)]
    pub descriptions: Vec<TextElement>,

    #[serde(rename = "label", default)]
    pub labels: Vec<Label>,

    #[serde(rename = "ref", default)]
    pub references: Vec<super::text::Reference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptionBlock {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "p", default)]
    pub paragraphs: Vec<super::text::Paragraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDesc {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "abstract", default)]
    pub abstract_: Option<Abstract>,

    #[serde(rename = "textClass", default)]
    pub text_class: Option<TextClass>,

    #[serde(rename = "langUsage", default)]
    pub lang_usage: Option<LangUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abstract {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "div", default)]
    pub divisions: Vec<Division>,

    #[serde(rename = "p", default)]
    pub paragraphs: Vec<super::text::Paragraph>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextClass {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "keywords", default)]
    pub keywords: Vec<Keywords>,

    #[serde(rename = "classCode", default)]
    pub class_codes: Vec<TextElement>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keywords {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(flatten, default)]
    pub typed: TypedAttributes,

    #[serde(rename = "term", default)]
    pub terms: Vec<TextElement>,

    #[serde(rename = "list", default)]
    pub lists: Vec<super::text::List>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangUsage {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "language", default)]
    pub languages: Vec<Language>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Language {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@ident", default)]
    pub ident: Option<String>,

    #[serde(rename = "@usage", default)]
    pub usage: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionDesc {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "change", default)]
    pub changes: Vec<Change>,

    #[serde(rename = "list", default)]
    pub lists: Vec<super::text::List>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    #[serde(flatten, default)]
    pub global: GlobalAttributes,

    #[serde(rename = "@when", default)]
    pub when: Option<String>,

    #[serde(rename = "@who", default)]
    pub who: Option<String>,

    #[serde(rename = "$text", default)]
    pub text: Option<String>,
}
