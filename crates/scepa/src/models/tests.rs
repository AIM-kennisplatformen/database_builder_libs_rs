use serde_json::json;

use super::entities::{
    document::{Book, Document, ResearchPaper},
    institution::{EducationInstitution, Institution, InstitutionEntity},
    person::{Person, PersonEntity},
    publication_venue::{Conference, Journal, PublicationVenue},
};
use super::relations::{
    contribution::{Authorship, PeerReview},
    publishing::Publishing,
};

#[test]
fn document_serializes_to_its_deserialization_envelope() {
    let document = Document::ResearchPaper(ResearchPaper {
        doi: Some("10.1038/s41586-020-2649-2".to_owned()),
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
fn authorship_and_peer_review_serialize_trait_object_participants() {
    let authorship = Authorship {
        author: Some(Box::new(Person {
            given_name: Some("Marie".to_owned()),
            family_name: Some("Curie".to_owned()),
        })),
        authored_work: Some(Box::new(ResearchPaper {
            doi: Some("10.1038/s41586-020-2649-2".to_owned()),
        })),
    };
    let peer_review = PeerReview {
        reviewer: Some(Box::new(Person {
            given_name: Some("Pierre".to_owned()),
            family_name: Some("Curie".to_owned()),
        })),
        reviewed_work: Some(Box::new(Book {
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
fn publishing_uses_trait_object_entity_participants() {
    let publishing = Publishing {
        publisher: Some(Box::new(EducationInstitution {})),
        venue: Some(Box::new(Journal {
            venue_name: Some("Nature".to_owned()),
        })),
        published_work: Box::new(ResearchPaper {
            doi: Some("10.1038/s41586-020-2649-2".to_owned()),
        }),
        version_number: Some("version-of-record".to_owned()),
        publication_date: Some("2020-08-26T00:00:00Z".to_owned()),
    };

    let serialized = serde_json::to_value(&publishing).unwrap();
    assert_eq!(
        serialized,
        json!({
            "publisher": {"type": "education-institution", "attrs": {}},
            "venue": {"type": "journal", "attrs": {"venue-name": "Nature"}},
            "published-work": {
                "type": "research-paper",
                "attrs": {"doi": "10.1038/s41586-020-2649-2"}
            },
            "version-number": "version-of-record",
            "publication-date": "2020-08-26T00:00:00Z"
        })
    );

    let deserialized: Publishing = serde_json::from_value(serialized.clone()).unwrap();
    assert_eq!(serde_json::to_value(deserialized).unwrap(), serialized);
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
    let publishing: Publishing = serde_json::from_value(json!({
        "published-work": {
            "type": "research-paper",
            "attrs": {"doi": "10.1038/s41586-020-2649-2"}
        }
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
        publishing,
        Publishing {
            publisher: None,
            venue: None,
            published_work: _,
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
