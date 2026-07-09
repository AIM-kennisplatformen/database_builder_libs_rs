use std::collections::HashMap;

use crate::{
    ingestion::extract::ror::source::RorSource,
    models::domain::{Institution, Paper},
};

/// Resolves each institution's ROR (Research Organization Registry) id by
/// name. This only tags institutions with a ror-id; deduplicating entities
/// that turn out to share one (typos, naming variants of the same
/// organization) is a separate, later step over already-exported data, not
/// something this pass does.
///
/// Lookups are cached per paper so repeated affiliations to the same
/// institution within one paper only hit the ROR API once. A failed or
/// missing lookup is treated the same as "no match": it must never fail the
/// whole paper's export over a flaky external API or an institution ROR
/// simply doesn't know about.
pub async fn resolve_institution_ror_ids(paper: &mut Paper, ror: &RorSource) {
    let mut cache: HashMap<String, Option<String>> = HashMap::new();

    for authoring in &mut paper.graph.authorings {
        for affiliation in &mut authoring.affiliations {
            resolve(&mut affiliation.institution, ror, &mut cache).await;
        }
    }

    for publication in &mut paper.graph.publications {
        resolve(&mut publication.publisher, ror, &mut cache).await;
    }

    for citation in &mut paper.graph.citations {
        for authoring in &mut citation.authorings {
            for affiliation in &mut authoring.affiliations {
                resolve(&mut affiliation.institution, ror, &mut cache).await;
            }
        }
    }
}

async fn resolve(
    institution: &mut Institution,
    ror: &RorSource,
    cache: &mut HashMap<String, Option<String>>,
) {
    let Some(name) = institution
        .name
        .as_deref()
        .filter(|name| !name.is_empty())
    else {
        return;
    };

    if let Some(cached) = cache.get(name) {
        institution.ror_id = cached.clone();
        return;
    }

    let ror_id = ror
        .match_organization(name)
        .await
        .ok()
        .flatten()
        .map(|matched| matched.ror_id);

    cache.insert(name.to_owned(), ror_id.clone());
    institution.ror_id = ror_id;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ingestion::extract::ror::config::RorConfig,
        models::domain::{Affiliation, Author, Authoring, InstitutionKind, Publication},
    };

    fn paper_with_institutions(names: Vec<&str>) -> Paper {
        use crate::models::domain::{
            DocumentContent, Literature, LiteratureCore, PaperGraph, PaperMetadata,
            PdfExtractionData, ScientificLiterature, SourceHash,
        };

        Paper {
            source: SourceHash::from_bytes(b"ror-test"),
            graph: PaperGraph {
                literature: Literature::Scientific(ScientificLiterature {
                    core: LiteratureCore {
                        title: None,
                        publishing_date: None,
                        issn: None,
                        isbn: None,
                    },
                    doi: None,
                }),
                authorings: names
                    .into_iter()
                    .map(|name| Authoring {
                        author: Author {
                            forename: None,
                            surname: None,
                        },
                        affiliations: vec![Affiliation {
                            institution: Institution {
                                name: Some(name.to_owned()),
                                kind: InstitutionKind::Institution,
                                ror_id: None,
                            },
                            department: None,
                        }],
                    })
                    .collect(),
                publications: Vec::new(),
                citations: Vec::new(),
            },
            metadata: PaperMetadata::default(),
            content: DocumentContent::default(),
            extraction_data: PdfExtractionData::default(),
        }
    }

    #[tokio::test]
    async fn leaves_ror_id_none_when_the_endpoint_is_unreachable() {
        let mut paper = paper_with_institutions(vec!["Some University"]);
        let ror = RorSource::new(RorConfig::new("http://localhost:0"));

        resolve_institution_ror_ids(&mut paper, &ror).await;

        assert_eq!(
            paper.graph.authorings[0].affiliations[0].institution.ror_id,
            None
        );
    }

    #[tokio::test]
    async fn skips_institutions_with_no_name() {
        let mut paper = paper_with_institutions(vec![]);
        paper.graph.authorings.push(Authoring {
            author: Author {
                forename: None,
                surname: None,
            },
            affiliations: vec![Affiliation {
                institution: Institution {
                    name: None,
                    kind: InstitutionKind::Institution,
                    ror_id: None,
                },
                department: None,
            }],
        });
        let ror = RorSource::new(RorConfig::new("http://localhost:0"));

        resolve_institution_ror_ids(&mut paper, &ror).await;

        assert_eq!(
            paper.graph.authorings[0].affiliations[0].institution.ror_id,
            None
        );
    }

    #[tokio::test]
    async fn resolves_publishers_too() {
        let mut paper = paper_with_institutions(vec![]);
        paper.graph.publications.push(Publication {
            publisher: Institution {
                name: Some("Some Publisher".to_owned()),
                kind: InstitutionKind::Institution,
                ror_id: None,
            },
        });
        let ror = RorSource::new(RorConfig::new("http://localhost:0"));

        resolve_institution_ror_ids(&mut paper, &ror).await;

        assert_eq!(paper.graph.publications[0].publisher.ror_id, None);
    }
}
