use chrono::DateTime;
use serde_json::json;

use super::chunk::Chunk;
use super::entities::{
    Entity, TypeDbEntity,
    document::{Book, Document, ResearchPaper},
    institution::{EducationInstitution, Institution, InstitutionEntity},
    person::{Person, PersonEntity},
    publication_venue::{Conference, Journal, PublicationVenue},
};
use super::relations::{
    Relation,
    contribution::{Authorship, BaseContribution, Contribution, PeerReview},
    publication_event::{Publication, PublicationEventRelation, Submission},
};
use crate::pipeline::tei;

#[test]
fn document_serializes_to_its_deserialization_envelope() {
    let document = Document::ResearchPaper(ResearchPaper {
        pdf_hash: None,
        title: None,
        doi: Some("10.1038/s41586-020-2649-2".to_owned()),
        abstract_text: None,
        acknowledgements: None,
        conflicts: None,
        contributions: None,
    });

    let serialized = serde_json::to_value(&document).unwrap();
    assert_eq!(
        serialized,
        json!({
            "type": "research-paper",
            "attrs": {
                "doi": "10.1038/s41586-020-2649-2"
            }
        })
    );

    let deserialized: Document = serde_json::from_value(serialized.clone()).unwrap();
    assert_eq!(serde_json::to_value(deserialized).unwrap(), serialized);
}

#[test]
fn entity_hierarchies_round_trip_through_their_local_enums() {
    let documents = [
        json!({"type": "book", "attrs": {"isbn": "978-0-123456-47-2", "issn": "0028-0836"}}),
        json!({"type": "research-paper", "attrs": {"doi": "10.1038/s41586-020-2649-2"}}),
    ];
    for document in documents {
        let parsed: Document = serde_json::from_value(document.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), document);
    }

    let institutions = [
        "institution",
        "government-institution",
        "education-institution",
        "nonprofit-institution",
    ];
    for institution in institutions {
        let value = json!({"type": institution, "attrs": {}});
        let parsed: InstitutionEntity = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }

    let venues = [
        json!({"type": "journal", "attrs": {"venue-name": "Nature"}}),
        json!({"type": "conference", "attrs": {"venue-name": "NeurIPS"}}),
    ];
    for venue in venues {
        let parsed: PublicationVenue = serde_json::from_value(venue.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), venue);
    }

    let person = json!({
        "type": "person",
        "attrs": {"given-name": "Marie", "family-name": "Curie"}
    });
    let parsed: PersonEntity = serde_json::from_value(person.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), person);
}

#[test]
fn aggregate_typedb_enums_use_kebab_case_variant_names() {
    let entity = Entity::Person(PersonEntity::Person(Person {
        given_name: Some("Marie".to_owned()),
        family_name: Some("Curie".to_owned()),
    }));
    assert_eq!(
        serde_json::to_value(entity).unwrap(),
        json!({
            "person": {
                "type": "person",
                "attrs": {
                    "given-name": "Marie",
                    "family-name": "Curie"
                }
            }
        })
    );

    let relation = Relation::Contribution(Contribution::Contribution(BaseContribution {
        contributor: None,
        work: None,
    }));
    assert_eq!(
        serde_json::to_value(relation).unwrap(),
        json!({
            "contribution": {
                "type": "contribution"
            }
        })
    );
}

#[test]
fn authorship_and_peer_review_serialize_trait_object_participants() {
    let authorship = Authorship {
        author: Some(Box::new(Person {
            given_name: Some("Marie".to_owned()),
            family_name: Some("Curie".to_owned()),
        })),
        authored_work: Some(Box::new(ResearchPaper {
            pdf_hash: None,
            title: None,
            doi: Some("10.1038/s41586-020-2649-2".to_owned()),
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
        })),
    };
    let peer_review = PeerReview {
        reviewer: Some(Box::new(Person {
            given_name: Some("Pierre".to_owned()),
            family_name: Some("Curie".to_owned()),
        })),
        reviewed_work: Some(Box::new(Book {
            pdf_hash: None,
            title: None,
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
            isbn: Some("978-0-123456-47-3".to_owned()),
            issn: None,
        })),
    };

    let authorship_value = serde_json::to_value(&authorship).unwrap();
    assert_eq!(
        authorship_value,
        json!({
            "author": {
                "type": "person",
                "attrs": {
                    "given-name": "Marie",
                    "family-name": "Curie"
                }
            },
            "authored-work": {
                "type": "research-paper",
                "attrs": {"doi": "10.1038/s41586-020-2649-2"}
            }
        })
    );
    let parsed: Authorship = serde_json::from_value(authorship_value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), authorship_value);

    let peer_review_value = serde_json::to_value(&peer_review).unwrap();
    assert_eq!(
        peer_review_value,
        json!({
            "reviewer": {
                "type": "person",
                "attrs": {
                    "given-name": "Pierre",
                    "family-name": "Curie"
                }
            },
            "reviewed-work": {
                "type": "book",
                "attrs": {"isbn": "978-0-123456-47-3"}
            }
        })
    );
    let parsed: PeerReview = serde_json::from_value(peer_review_value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), peer_review_value);
}

#[test]
fn publication_uses_trait_object_entity_participants() {
    let publication = Publication {
        publisher: Some(Box::new(EducationInstitution {})),
        venue: Some(Box::new(Journal {
            venue_name: Some("Nature".to_owned()),
        })),
        work: Box::new(ResearchPaper {
            pdf_hash: None,
            title: None,
            doi: Some("10.1038/s41586-020-2649-2".to_owned()),
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
        }),
        version_number: Some("version-of-record".to_owned()),
        publication_date: Some(DateTime::parse_from_rfc3339("2020-08-26T00:00:00Z").unwrap()),
    };

    let serialized = serde_json::to_value(&publication).unwrap();
    assert_eq!(
        serialized,
        json!({
            "publisher": {"type": "education-institution", "attrs": {}},
            "venue": {"type": "journal", "attrs": {"venue-name": "Nature"}},
            "work": {
                "type": "research-paper",
                "attrs": {"doi": "10.1038/s41586-020-2649-2"}
            },
            "version-number": "version-of-record",
            "publication-date": "2020-08-26T00:00:00Z"
        })
    );

    let deserialized: Publication = serde_json::from_value(serialized.clone()).unwrap();
    assert_eq!(serde_json::to_value(deserialized).unwrap(), serialized);
}

#[test]
fn publication_event_subtypes_carry_their_schema_type() {
    let event = PublicationEventRelation::Submission(Submission {
        publisher: None,
        venue: None,
        work: Box::new(ResearchPaper {
            pdf_hash: None,
            title: None,
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
            doi: Some("10.1038/s41586-020-2649-2".to_owned()),
        }),
        submission_date: Some(DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z").unwrap()),
        submission_note: Some("Received: 2020-01-01".to_owned()),
    });

    let serialized = serde_json::to_value(event).unwrap();
    assert_eq!(
        serialized,
        json!({
            "type": "submission",
            "work": {
                "type": "research-paper",
                "attrs": {"doi": "10.1038/s41586-020-2649-2"}
            },
            "submission-date": "2020-01-01T00:00:00Z",
            "submission-note": "Received: 2020-01-01"
        })
    );
    let parsed: PublicationEventRelation = serde_json::from_value(serialized.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), serialized);
}

#[test]
fn unspecified_relation_roles_are_optional() {
    let authorship: Authorship = serde_json::from_value(json!({
        "authored-work": {
            "type": "book",
            "attrs": {"isbn": "978-0-123456-47-3"}
        }
    }))
    .unwrap();
    let publication: Publication = serde_json::from_value(json!({
        "work": {
            "type": "research-paper",
            "attrs": {"doi": "10.1038/s41586-020-2649-2"}
        },
        "publication-date": "2020-08-26T00:00:00Z"
    }))
    .unwrap();

    assert!(matches!(
        authorship,
        Authorship {
            author: None,
            authored_work: Some(_)
        }
    ));
    assert!(matches!(
        publication,
        Publication {
            publisher: None,
            venue: None,
            work: _,
            ..
        }
    ));
}

#[test]
fn empty_schema_entities_preserve_empty_attribute_maps() {
    let institution = InstitutionEntity::Institution(Institution {});
    let venue = PublicationVenue::Conference(Conference { venue_name: None });

    assert_eq!(
        serde_json::to_value(institution).unwrap(),
        json!({"type": "institution", "attrs": {}})
    );
    assert_eq!(
        serde_json::to_value(venue).unwrap(),
        json!({"type": "conference", "attrs": {}})
    );
}

#[test]
fn tei_parser_builds_type_db_entities_and_chunks_directly() {
    let model = tei::parse_with_pdf_hash(
        r#"
        <TEI>
          <teiHeader>
            <fileDesc>
              <titleStmt><title type="main">A study of energy poverty</title></titleStmt>
              <sourceDesc>
                <biblStruct>
                  <analytic>
                    <author>
                      <persName>
                        <forename>Ada</forename>
                        <forename type="middle">L</forename>
                        <surname>Lovelace</surname>
                      </persName>
                      <email>ada@example.org</email>
                      <idno type="ORCID">0000-0000-0000-0000</idno>
                      <affiliation key="aff0">
                        <orgName type="institution">Analytical Engine Institute</orgName>
                        <address><settlement>London</settlement><country>United Kingdom</country></address>
                      </affiliation>
                    </author>
                  </analytic>
                  <idno type="DOI">10.1234/example</idno>
                  <note type="submission">Received: 2024-01-01 Accepted: 2024-02-01</note>
                </biblStruct>
              </sourceDesc>
              <abstract>An abstract.</abstract>
            </fileDesc>
          </teiHeader>
          <text>
            <body><div><head>Introduction</head><p>The first paragraph.</p><p>The second paragraph.</p></div></body>
            <back><div type="acknowledgement">Thanks to the reviewers.</div></back>
          </text>
        </TEI>
        "#,
        "paper-hash",
    )
    .unwrap();

    assert_eq!(model.entities.len(), 1);
    assert!(matches!(
        &model.entities[0],
        Entity::Person(PersonEntity::Person(Person {
            given_name: Some(given_name),
            family_name: Some(family_name),
        })) if given_name == "Ada" && family_name == "Lovelace"
    ));
    assert_eq!(model.relations.len(), 3);
    assert!(matches!(&model.relations[0], Relation::Contribution(_)));
    assert!(matches!(&model.relations[1], Relation::PublicationEvent(_)));
    assert!(matches!(&model.relations[2], Relation::PublicationEvent(_)));
    assert_eq!(model.chunks.len(), 4);
    assert!(matches!(
        &model.chunks[0],
        Chunk::Abstract(abstract_chunk) if abstract_chunk.index == 0
            && abstract_chunk.bounding_boxes.is_empty()
            && abstract_chunk.document_hash.is_empty()
    ));
    assert!(matches!(
        &model.chunks[1],
        Chunk::Text(text_chunk) if text_chunk.index == 1
            && text_chunk.section_heading.as_deref() == Some("Introduction")
            && text_chunk.document_hash.is_empty()
    ));
    assert!(matches!(
        &model.chunks[3],
        Chunk::Text(text_chunk) if text_chunk.document_hash.is_empty()
    ));

    let document = serde_json::to_value(&model.document).unwrap();
    assert_eq!(document["type"], "research-paper");
    assert_eq!(document["attrs"]["pdf-hash"], "paper-hash");
    assert_eq!(document["attrs"]["title"], "A study of energy poverty");
    assert_eq!(document["attrs"]["abstract-text"], "An abstract.");
    assert!(document.get("chunks").is_none());

    assert!(
        model
            .document
            .typeql_insert_statement("entity")
            .contains(", has pdf-hash \"paper-hash\"")
    );
}
