use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Document {
    pub title: Option<String>,
    pub authors: Vec<Author>,
    pub doi: Option<String>,
    pub dates: ArticleDates,
    pub funding: Vec<Funding>,
    pub abstract_text: Option<String>,
    pub sections: Vec<Section>,
    pub figures: Vec<Figure>,
    pub acknowledgements: Option<String>,
    pub conflicts: Option<String>,
    pub contributions: Option<String>,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Author {
    pub given_name: Option<String>,
    pub middle_names: Vec<String>,
    pub surname: Option<String>,
    pub email: Option<String>,
    pub orcid: Option<String>,
    pub affiliations: Vec<Affiliation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Affiliation {
    pub key: Option<String>,
    pub organizations: Vec<Organization>,
    pub address: Option<Address>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Organization {
    pub kind: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Address {
    pub settlement: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ArticleDates {
    pub submission_date: Option<String>,
    pub acceptance_date: Option<String>,
    /// The original GROBID note, retained when its dates cannot be split reliably.
    pub submission_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Funding {
    pub name: Option<String>,
    pub reference: Option<String>,
    pub project: Option<String>,
    pub grant_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub heading: Option<String>,
    pub paragraphs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Figure {
    pub id: Option<String>,
    /// GROBID uses `type="table"` for tables represented as figures.
    pub kind: Option<String>,
    pub heading: Option<String>,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub images: Vec<FigureImage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FigureImage {
    /// The image URL/path when the TEI source includes one.
    pub url: Option<String>,
    pub media_type: Option<String>,
    /// GROBID page coordinates, when image bytes or a URL are unavailable.
    pub coordinates: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reference {
    pub id: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<Author>,
    pub doi: Option<String>,
    pub journal: Option<String>,
    pub publication_year: Option<String>,
    pub volume: Option<String>,
    pub pages: Option<String>,
    pub external_urls: Vec<String>,
}
