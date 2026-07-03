use serde::Serialize;

use super::publication::PublicationDate;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Literature {
    Scientific(ScientificLiterature),
    ProjectReport(LiteratureCore),
    Grey(LiteratureCore),
    Survey(LiteratureCore),
    Book(LiteratureCore),
    BookChapter(LiteratureCore),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScientificLiterature {
    pub core: LiteratureCore,
    pub doi: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiteratureCore {
    pub title: Option<String>,
    pub publishing_date: Option<PublicationDate>,
    pub issn: Option<String>,
    pub isbn: Option<String>,
}
