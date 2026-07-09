use std::collections::HashMap;

use crate::{
    ingestion::{
        enrich::institution_kind::classify_institution_kind,
        extract::ror::source::{RorMatch, RorSource},
    },
    models::domain::{Institution, Paper},
};

/// Resolves each institution's ROR (Research Organization Registry) id and
/// kind (university, government, etc.) by name. This only tags institutions;
/// deduplicating entities that turn out to share a ror-id (typos, naming
/// variants of the same organization) is a separate, later step over
/// already-exported data, not something this pass does.
///
/// Lookups are cached per paper so repeated affiliations to the same
/// institution within one paper only hit the ROR API once. A failed or
/// missing lookup is treated the same as "no match": it must never fail the
/// whole paper's export over a flaky external API or an institution ROR
/// simply doesn't know about, and the institution's kind still gets
/// classified from its name alone in that case.
pub async fn enrich_institutions(paper: &mut Paper, ror: &RorSource) {
    let mut cache: HashMap<String, Option<RorMatch>> = HashMap::new();

    for authoring in &mut paper.graph.authorings {
        for affiliation in &mut authoring.affiliations {
            enrich(&mut affiliation.institution, ror, &mut cache).await;
        }
    }

    for publication in &mut paper.graph.publications {
        enrich(&mut publication.publisher, ror, &mut cache).await;
    }

    for funding in &mut paper.graph.fundings {
        enrich(&mut funding.funder, ror, &mut cache).await;
    }

    for citation in &mut paper.graph.citations {
        for authoring in &mut citation.authorings {
            for affiliation in &mut authoring.affiliations {
                enrich(&mut affiliation.institution, ror, &mut cache).await;
            }
        }
    }
}

async fn enrich(
    institution: &mut Institution,
    ror: &RorSource,
    cache: &mut HashMap<String, Option<RorMatch>>,
) {
    let Some(name) = institution.name.clone().filter(|name| !name.is_empty()) else {
        return;
    };

    let matched = if let Some(cached) = cache.get(&name) {
        cached.clone()
    } else {
        let matched = ror.match_organization(&name).await.ok().flatten();
        cache.insert(name.clone(), matched.clone());
        matched
    };

    let ror_types = matched.as_ref().map_or(&[][..], |matched| &matched.types);
    institution.kind = classify_institution_kind(&name, ror_types);
    institution.ror_id = matched.map(|matched| matched.ror_id);
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
                fundings: Vec::new(),
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

        enrich_institutions(&mut paper, &ror).await;

        assert_eq!(
            paper.graph.authorings[0].affiliations[0].institution.ror_id,
            None
        );
    }

    #[tokio::test]
    async fn classifies_kind_from_name_even_when_the_endpoint_is_unreachable() {
        let mut paper = paper_with_institutions(vec!["Some University"]);
        let ror = RorSource::new(RorConfig::new("http://localhost:0"));

        enrich_institutions(&mut paper, &ror).await;

        assert_eq!(
            paper.graph.authorings[0].affiliations[0].institution.kind,
            InstitutionKind::University
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

        enrich_institutions(&mut paper, &ror).await;

        assert_eq!(
            paper.graph.authorings[0].affiliations[0].institution.ror_id,
            None
        );
        assert_eq!(
            paper.graph.authorings[0].affiliations[0].institution.kind,
            InstitutionKind::Institution
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

        enrich_institutions(&mut paper, &ror).await;

        assert_eq!(paper.graph.publications[0].publisher.ror_id, None);
    }
}
