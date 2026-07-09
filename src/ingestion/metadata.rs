use serde::{Deserialize, Serialize};

use crate::models::domain::{Author, Department, Institution, Literature, LiteratureCore, Paper};

/// Structured metadata shaped to match upload_interface's Field schema
/// (see its `src/backend/routers/field.py::FIELD_KEYS`), for autocompleting
/// upload_interface's metadata form from a live GROBID extraction instead
/// of a static pre-computed dataset.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UploadInterfaceMetadata {
    pub title: Option<String>,
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    pub authors: Vec<String>,
    pub doi: Option<String>,
    pub year: Option<String>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub issn: Option<String>,
    pub isbn: Option<String>,
    pub publisher: Option<String>,
    pub keywords: Vec<String>,
    pub affiliations: Vec<String>,
    pub acknowledgements: Option<String>,
    pub funding_statements: Vec<String>,
    pub literature_type: Option<String>,
}

impl UploadInterfaceMetadata {
    pub fn from_paper(paper: &Paper) -> Self {
        let (core, doi, literature_type) = literature_fields(&paper.graph.literature);

        let authors = paper
            .graph
            .authorings
            .iter()
            .filter_map(|authoring| author_name(&authoring.author))
            .collect();

        let affiliations = dedup_preserve_order(
            paper
                .graph
                .authorings
                .iter()
                .flat_map(|authoring| authoring.affiliations.iter())
                .filter_map(|affiliation| {
                    affiliation_text(affiliation.department.as_ref(), &affiliation.institution)
                })
                .collect(),
        );

        let publisher = paper
            .graph
            .publications
            .first()
            .and_then(|publication| publication.publisher.name.clone());

        Self {
            title: core.title.clone(),
            abstract_text: paper.metadata.abstract_text.clone(),
            authors,
            doi,
            year: core
                .publishing_date
                .as_ref()
                .map(|date| date.year.to_string()),
            journal: paper.metadata.journal.clone(),
            volume: paper.metadata.volume.clone(),
            issue: paper.metadata.issue.clone(),
            issn: core.issn.clone(),
            isbn: core.isbn.clone(),
            publisher,
            keywords: paper.metadata.keywords.clone(),
            affiliations,
            acknowledgements: paper.metadata.acknowledgements.clone(),
            funding_statements: paper.metadata.funding_statements.clone(),
            literature_type: Some(literature_type.to_owned()),
        }
    }
}

fn literature_fields(literature: &Literature) -> (&LiteratureCore, Option<String>, &'static str) {
    match literature {
        Literature::Scientific(scientific) => (
            &scientific.core,
            scientific.doi.clone(),
            "scientific-literature",
        ),
        Literature::ProjectReport(core) => (core, None, "project-reports"),
        Literature::Grey(core) => (core, None, "grey-literature"),
        Literature::Survey(core) => (core, None, "survey"),
        Literature::Book(core) => (core, None, "book"),
        Literature::BookChapter(core) => (core, None, "book-chapter"),
    }
}

fn author_name(author: &Author) -> Option<String> {
    match (author.forename.as_deref(), author.surname.as_deref()) {
        (Some(forename), Some(surname)) => Some(format!("{forename} {surname}")),
        (Some(name), None) | (None, Some(name)) => Some(name.to_owned()),
        (None, None) => None,
    }
}

fn affiliation_text(department: Option<&Department>, institution: &Institution) -> Option<String> {
    let parts: Vec<&str> = [
        department.and_then(|department| department.name.as_deref()),
        institution.name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn dedup_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::domain::{
        Affiliation, Authoring, DocumentContent, InstitutionKind, PaperGraph, PaperMetadata,
        PdfExtractionData, Publication, PublicationDate, ScientificLiterature, SourceHash,
    };

    fn paper_with_graph(graph: PaperGraph) -> Paper {
        Paper {
            source: SourceHash::from_bytes(b"metadata-test"),
            graph,
            metadata: PaperMetadata::default(),
            content: DocumentContent::default(),
            extraction_data: PdfExtractionData::default(),
        }
    }

    #[test]
    fn maps_literature_core_fields_and_year() {
        let paper = paper_with_graph(PaperGraph {
            literature: Literature::Scientific(ScientificLiterature {
                core: LiteratureCore {
                    title: Some("A Paper".to_owned()),
                    publishing_date: Some(PublicationDate {
                        year: 2021,
                        month: Some(6),
                        day: Some(1),
                    }),
                    issn: Some("1234-5678".to_owned()),
                    isbn: None,
                },
                doi: Some("10.1234/example".to_owned()),
            }),
            authorings: vec![],
            publications: vec![],
            citations: vec![],
            fundings: vec![],
        });

        let metadata = UploadInterfaceMetadata::from_paper(&paper);

        assert_eq!(metadata.title.as_deref(), Some("A Paper"));
        assert_eq!(metadata.year.as_deref(), Some("2021"));
        assert_eq!(metadata.issn.as_deref(), Some("1234-5678"));
        assert_eq!(metadata.doi.as_deref(), Some("10.1234/example"));
        assert_eq!(
            metadata.literature_type.as_deref(),
            Some("scientific-literature")
        );
    }

    #[test]
    fn joins_forename_and_surname_and_skips_nameless_authors() {
        let paper = paper_with_graph(PaperGraph {
            literature: Literature::Scientific(ScientificLiterature {
                core: LiteratureCore {
                    title: None,
                    publishing_date: None,
                    issn: None,
                    isbn: None,
                },
                doi: None,
            }),
            authorings: vec![
                Authoring {
                    author: Author {
                        forename: Some("Ada".to_owned()),
                        surname: Some("Lovelace".to_owned()),
                    },
                    affiliations: vec![],
                },
                Authoring {
                    author: Author {
                        forename: None,
                        surname: None,
                    },
                    affiliations: vec![],
                },
            ],
            publications: vec![],
            citations: vec![],
            fundings: vec![],
        });

        let metadata = UploadInterfaceMetadata::from_paper(&paper);

        assert_eq!(metadata.authors, vec!["Ada Lovelace".to_owned()]);
    }

    #[test]
    fn joins_department_and_institution_into_one_affiliation_string_and_dedupes() {
        let affiliation = Affiliation {
            institution: Institution {
                name: Some("Example University".to_owned()),
                kind: InstitutionKind::University,
                ror_id: None,
            },
            department: Some(Department {
                name: Some("Computer Science".to_owned()),
            }),
        };

        let paper = paper_with_graph(PaperGraph {
            literature: Literature::Scientific(ScientificLiterature {
                core: LiteratureCore {
                    title: None,
                    publishing_date: None,
                    issn: None,
                    isbn: None,
                },
                doi: None,
            }),
            authorings: vec![
                Authoring {
                    author: Author {
                        forename: None,
                        surname: Some("A".to_owned()),
                    },
                    affiliations: vec![affiliation.clone()],
                },
                Authoring {
                    author: Author {
                        forename: None,
                        surname: Some("B".to_owned()),
                    },
                    affiliations: vec![affiliation],
                },
            ],
            publications: vec![],
            citations: vec![],
            fundings: vec![],
        });

        let metadata = UploadInterfaceMetadata::from_paper(&paper);

        assert_eq!(
            metadata.affiliations,
            vec!["Computer Science, Example University".to_owned()]
        );
    }

    #[test]
    fn uses_the_first_publisher_name() {
        let paper = paper_with_graph(PaperGraph {
            literature: Literature::Scientific(ScientificLiterature {
                core: LiteratureCore {
                    title: None,
                    publishing_date: None,
                    issn: None,
                    isbn: None,
                },
                doi: None,
            }),
            authorings: vec![],
            publications: vec![Publication {
                publisher: Institution {
                    name: Some("Example Press".to_owned()),
                    kind: InstitutionKind::Institution,
                    ror_id: None,
                },
            }],
            citations: vec![],
            fundings: vec![],
        });

        let metadata = UploadInterfaceMetadata::from_paper(&paper);

        assert_eq!(metadata.publisher.as_deref(), Some("Example Press"));
    }

    #[test]
    fn serializes_the_abstract_field_under_its_python_keyword_name() {
        let metadata = UploadInterfaceMetadata {
            abstract_text: Some("summary".to_owned()),
            ..UploadInterfaceMetadata::default()
        };

        let value = serde_json::to_value(&metadata).unwrap();

        assert_eq!(value["abstract"], "summary");
        assert!(value.get("abstract_text").is_none());
    }
}
