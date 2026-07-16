use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};
use rootcause::{prelude::Report, report};

use crate::{
    domain::DocumentWithChunks,
    identity::{self, DocumentKind},
    models::{
        chunk::{Abstract, Chunk, Figure, Image, Text},
        generated::{
            Acceptance, Authorship, Citation, Cited, Citing, Contribution,
            Document as TypedbDocument, Entity, Institution, InstitutionEntity, Journal, Person,
            PersonEntity, PublicationEventRelation, PublicationVenue, Relation, ResearchPaper,
            Submission, Venue as PublicationVenueRole, Work as ContributionWork,
            Work as PublicationWork,
        },
    },
};

use super::tree::{Node, child, children_named, descendant, parse_tree, text};

pub fn parse(xml: &str) -> Result<DocumentWithChunks, Report> {
    parse_with_optional_pdf_hash(xml, None)
}

pub fn parse_with_pdf_hash(
    xml: &str,
    pdf_hash: impl Into<String>,
) -> Result<DocumentWithChunks, Report> {
    parse_with_optional_pdf_hash(xml, Some(pdf_hash.into()))
}

fn parse_with_optional_pdf_hash(
    xml: &str,
    pdf_hash: Option<String>,
) -> Result<DocumentWithChunks, Report> {
    let root = parse_tree(xml)?;
    let header = descendant(&root, "teiHeader");
    let body = descendant(&root, "body");
    let source = header.and_then(|node| descendant(node, "biblStruct"));
    let analytic = source.and_then(|node| child(node, "analytic"));
    let abstract_node = header.and_then(|node| descendant(node, "abstract"));
    let back = descendant(&root, "back");

    let title = header
        .and_then(|node| descendant(node, "titleStmt"))
        .and_then(|node| {
            children_named(node, "title")
                .into_iter()
                .find(|title| title.attributes.get("type").map(String::as_str) == Some("main"))
                .or_else(|| child(node, "title"))
        })
        .map(text);
    let abstract_text = abstract_node.map(text);
    let doi = source
        .and_then(|node| identifier(node, "DOI"))
        .map(|value| identity::normalize_doi(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    let pdf_hash = pdf_hash
        .map(|value| identity::normalize_pdf_hash(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    let document_id = identity::document_id(
        DocumentKind::ResearchPaper,
        doi.as_deref(),
        pdf_hash.as_deref(),
        None,
        None,
    )
    .map_err(|error| report!("{error}"))?;
    let people = analytic
        .map(|node| {
            children_named(node, "author")
                .into_iter()
                .enumerate()
                .map(|(index, author)| person(author, &document_id, index))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|error| report!("{error}"))?
        .unwrap_or_default();
    let (submission_date, acceptance_date, submission_note) =
        source.map(article_dates).unwrap_or_default();
    let acknowledgements = back.and_then(|node| back_matter(node, "acknowledgement"));
    let conflicts = back.and_then(|node| back_matter(node, "conflict"));
    let contributions = back.and_then(|node| back_matter(node, "contribution"));
    let chunks = parse_chunks(
        &root,
        abstract_node,
        body,
        back,
        acknowledgements.as_deref(),
        conflicts.as_deref(),
        contributions.as_deref(),
    );

    let document = TypedbDocument::ResearchPaper(ResearchPaper {
        entity_id: document_id.clone(),
        pdf_hash,
        title,
        doi,
        abstract_text: abstract_text.clone(),
        acknowledgements: acknowledgements.clone(),
        conflicts: conflicts.clone(),
        contributions: contributions.clone(),
    });

    let venue = source
        .map(|source| publication_venue(source, &document_id))
        .transpose()?
        .flatten();
    let mut entities = people
        .clone()
        .into_iter()
        .map(Entity::Person)
        .collect::<Vec<_>>();
    entities.extend(affiliation_entities(analytic, &document_id)?);
    if let Some(venue) = &venue {
        entities.push(Entity::PublicationVenue(venue.clone()));
    }

    let mut relations = people
        .iter()
        .map(|person| {
            Relation::Contribution(Contribution::Authorship(Authorship {
                author: Some(Box::new(person_value(person))),
                authored_work: Some(contribution_work(&document)),
            }))
        })
        .collect::<Vec<_>>();
    let has_people = !people.is_empty();
    for (index, reference) in back
        .into_iter()
        .flat_map(|node| node.descendants_named("biblStruct"))
        .enumerate()
    {
        let reference_id = reference
            .attributes
            .get("id")
            .cloned()
            .unwrap_or_else(|| format!("reference-{index}"));
        let cited = cited_document(reference, &document_id, &reference_id)?;
        let cited_role = citation_cited(&cited);
        entities.push(Entity::Document(cited));
        relations.push(Relation::Citation(Citation {
            citing: citation_citing(&document),
            cited: cited_role,
        }));
    }

    if submission_date.is_some() || submission_note.is_some() {
        relations.push(Relation::PublicationEvent(
            PublicationEventRelation::Submission(Submission {
                publisher: None,
                venue: venue.as_ref().map(venue_value),
                work: publication_work(&document),
                submission_date,
                submission_note,
            }),
        ));
    }
    if acceptance_date.is_some() {
        relations.push(Relation::PublicationEvent(
            PublicationEventRelation::Acceptance(Acceptance {
                publisher: None,
                venue: venue.as_ref().map(venue_value),
                work: publication_work(&document),
                acceptance_date,
            }),
        ));
    }

    let TypedbDocument::ResearchPaper(document_data) = &document else {
        unreachable!("TEI parser creates research paper documents")
    };
    if !has_model_data(document_data, has_people, !relations.is_empty(), &chunks) {
        return Err(report!(
            "GROBID response contained no data that fits the domain model"
        ));
    }

    Ok(DocumentWithChunks {
        document,
        entities,
        relations,
        chunks,
    })
}

fn has_model_data(
    document: &ResearchPaper,
    has_people: bool,
    has_relations: bool,
    chunks: &[Chunk],
) -> bool {
    [
        document.title.as_deref(),
        document.doi.as_deref(),
        document.abstract_text.as_deref(),
        document.acknowledgements.as_deref(),
        document.conflicts.as_deref(),
        document.contributions.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.is_empty())
        || has_people
        || has_relations
        || !chunks.is_empty()
}

fn parse_chunks(
    root: &Node,
    abstract_node: Option<&Node>,
    body: Option<&Node>,
    back: Option<&Node>,
    acknowledgements: Option<&str>,
    conflicts: Option<&str>,
    contributions: Option<&str>,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    if let Some(text) = abstract_node.map(text).filter(|text| !text.is_empty()) {
        push_abstract_chunk(&mut chunks, None, &text);
    }

    if let Some(body) = body {
        for div in body.descendants_named("div") {
            let heading = child(div, "head")
                .map(text)
                .filter(|value| !value.is_empty());
            let paragraphs = div.descendants_named("p");

            if paragraphs.is_empty() {
                if let Some(heading) = heading.as_deref() {
                    push_text_chunk(&mut chunks, Some(heading), "");
                }
                continue;
            }

            for paragraph in paragraphs {
                let paragraph = text(paragraph);
                if !paragraph.is_empty() {
                    push_text_chunk(&mut chunks, heading.as_deref(), &paragraph);
                }
            }
        }
    }

    for value in [acknowledgements, conflicts, contributions] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            push_text_chunk(&mut chunks, None, value);
        }
    }

    if let Some(back) = back {
        for div in children_named(back, "div") {
            if div.attributes.get("type").map(String::as_str) == Some("other") {
                let value = text(div);
                if !value.is_empty() {
                    push_text_chunk(&mut chunks, None, &value);
                }
            }
        }
    }

    for figure in root.descendants_named("figure") {
        let heading = child(figure, "head")
            .map(text)
            .filter(|value| !value.is_empty());
        push_figure_chunk(&mut chunks, heading.as_deref());

        for _image in figure
            .descendants_named("graphic")
            .into_iter()
            .chain(figure.descendants_named("media"))
        {
            push_image_chunk(&mut chunks, heading.as_deref());
        }
    }

    chunks
}

fn push_text_chunk(chunks: &mut Vec<Chunk>, section_heading: Option<&str>, text: &str) {
    chunks.push(Chunk::Text(Text {
        bounding_boxes: Vec::new(),
        index: chunks.len(),
        section_heading: section_heading.map(str::to_owned),
        document_hash: String::new(),
        text: text.to_owned(),
    }));
}

fn push_abstract_chunk(chunks: &mut Vec<Chunk>, section_heading: Option<&str>, text: &str) {
    chunks.push(Chunk::Abstract(Abstract {
        bounding_boxes: Vec::new(),
        index: chunks.len(),
        section_heading: section_heading.map(str::to_owned),
        document_hash: String::new(),
        text: text.to_owned(),
    }));
}

fn push_figure_chunk(chunks: &mut Vec<Chunk>, section_heading: Option<&str>) {
    chunks.push(Chunk::Figure(Figure {
        bounding_boxes: Vec::new(),
        index: chunks.len(),
        section_heading: section_heading.map(str::to_owned),
        document_hash: String::new(),
    }));
}

fn push_image_chunk(chunks: &mut Vec<Chunk>, section_heading: Option<&str>) {
    chunks.push(Chunk::Image(Image {
        bounding_boxes: Vec::new(),
        index: chunks.len(),
        section_heading: section_heading.map(str::to_owned),
        document_hash: String::new(),
    }));
}

fn person(
    node: &Node,
    source_document_id: &str,
    author_index: usize,
) -> Result<PersonEntity, Report> {
    let name = child(node, "persName");
    let orcid = identifier(node, "ORCID")
        .map(|value| identity::normalize_orcid(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    let entity_id = identity::person_id(orcid.as_deref(), source_document_id, author_index)
        .map_err(|error| report!("{error}"))?;
    Ok(PersonEntity::Person(Person {
        entity_id,
        given_name: name.and_then(|name| child(name, "forename")).map(text),
        family_name: name.and_then(|name| child(name, "surname")).map(text),
        orcid,
    }))
}

fn person_value(person: &PersonEntity) -> Person {
    match person {
        PersonEntity::Person(person) => person.clone(),
    }
}

fn affiliation_entities(
    analytic: Option<&Node>,
    source_document_id: &str,
) -> Result<Vec<Entity>, Report> {
    let mut entities = Vec::new();
    let mut affiliation_index = 0;
    for author in analytic
        .into_iter()
        .flat_map(|node| children_named(node, "author"))
    {
        for affiliation in author.descendants_named("affiliation") {
            let affiliation_key = affiliation
                .attributes
                .get("key")
                .cloned()
                .unwrap_or_else(|| {
                    let key = format!("affiliation-{affiliation_index}");
                    affiliation_index += 1;
                    key
                });
            let ror = identifier(affiliation, "ROR")
                .map(|value| identity::normalize_ror(&value))
                .transpose()
                .map_err(|error| report!("{error}"))?;
            let entity_id =
                identity::institution_id(ror.as_deref(), source_document_id, &affiliation_key)
                    .map_err(|error| report!("{error}"))?;
            entities.push(Entity::Institution(InstitutionEntity::Institution(
                Institution { entity_id, ror },
            )));
        }
    }
    Ok(entities)
}

fn publication_venue(
    source: &Node,
    source_document_id: &str,
) -> Result<Option<PublicationVenue>, Report> {
    let monograph = child(source, "monogr");
    let venue_name = monograph
        .and_then(|node| child(node, "title"))
        .map(text)
        .filter(|value| !value.is_empty());
    let issn = identifier(source, "ISSN")
        .or_else(|| monograph.and_then(|node| identifier(node, "ISSN")))
        .map(|value| identity::normalize_issn(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    if venue_name.is_none() && issn.is_none() {
        return Ok(None);
    }

    let entity_id = identity::venue_id(issn.as_deref(), source_document_id, "primary")
        .map_err(|error| report!("{error}"))?;
    let kind = source
        .attributes
        .get("type")
        .map(String::as_str)
        .unwrap_or_default();
    if kind.eq_ignore_ascii_case("conference") {
        Ok(Some(PublicationVenue::Conference(
            crate::models::generated::Conference {
                entity_id,
                venue_name,
                issn,
            },
        )))
    } else {
        Ok(Some(PublicationVenue::Journal(Journal {
            entity_id,
            venue_name,
            issn,
        })))
    }
}

fn venue_value(venue: &PublicationVenue) -> Box<dyn PublicationVenueRole> {
    match venue {
        PublicationVenue::Journal(venue) => Box::new(venue.clone()),
        PublicationVenue::Conference(venue) => Box::new(venue.clone()),
    }
}

fn cited_document(
    reference: &Node,
    source_document_id: &str,
    reference_id: &str,
) -> Result<TypedbDocument, Report> {
    let analytic = child(reference, "analytic");
    let monograph = child(reference, "monogr");
    let title = analytic
        .and_then(|node| child(node, "title"))
        .or_else(|| monograph.and_then(|node| child(node, "title")))
        .map(text)
        .filter(|value| !value.is_empty());
    let doi = identifier(reference, "DOI")
        .map(|value| identity::normalize_doi(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    let isbn = identifier(reference, "ISBN")
        .map(|value| identity::normalize_isbn(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    let issn = identifier(reference, "ISSN")
        .map(|value| identity::normalize_issn(&value))
        .transpose()
        .map_err(|error| report!("{error}"))?;
    let kind = reference
        .attributes
        .get("type")
        .map(String::as_str)
        .unwrap_or_default();

    if kind.eq_ignore_ascii_case("book") || (analytic.is_none() && isbn.is_some()) {
        let entity_id = identity::document_id(
            DocumentKind::Book,
            isbn.as_deref(),
            None,
            Some(source_document_id),
            Some(reference_id),
        )
        .map_err(|error| report!("{error}"))?;
        Ok(TypedbDocument::Book(crate::models::generated::Book {
            entity_id,
            pdf_hash: None,
            title,
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
            isbn,
            issn,
        }))
    } else if kind.eq_ignore_ascii_case("report") {
        let entity_id = identity::document_id(
            DocumentKind::Report,
            None,
            None,
            Some(source_document_id),
            Some(reference_id),
        )
        .map_err(|error| report!("{error}"))?;
        Ok(TypedbDocument::Report(crate::models::generated::Report {
            entity_id,
            pdf_hash: None,
            title,
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
        }))
    } else {
        let entity_id = identity::document_id(
            DocumentKind::ResearchPaper,
            doi.as_deref(),
            None,
            Some(source_document_id),
            Some(reference_id),
        )
        .map_err(|error| report!("{error}"))?;
        Ok(TypedbDocument::ResearchPaper(ResearchPaper {
            entity_id,
            pdf_hash: None,
            title,
            abstract_text: None,
            acknowledgements: None,
            conflicts: None,
            contributions: None,
            doi,
        }))
    }
}

fn contribution_work(document: &TypedbDocument) -> Box<dyn ContributionWork> {
    match document {
        TypedbDocument::Book(document) => Box::new(document.clone()),
        TypedbDocument::ResearchPaper(document) => Box::new(document.clone()),
        TypedbDocument::Report(document) => Box::new(document.clone()),
    }
}

fn publication_work(document: &TypedbDocument) -> Box<dyn PublicationWork> {
    match document {
        TypedbDocument::Book(document) => Box::new(document.clone()),
        TypedbDocument::ResearchPaper(document) => Box::new(document.clone()),
        TypedbDocument::Report(document) => Box::new(document.clone()),
    }
}

fn citation_citing(document: &TypedbDocument) -> Box<dyn Citing> {
    match document {
        TypedbDocument::Book(document) => Box::new(document.clone()),
        TypedbDocument::ResearchPaper(document) => Box::new(document.clone()),
        TypedbDocument::Report(document) => Box::new(document.clone()),
    }
}

fn citation_cited(document: &TypedbDocument) -> Box<dyn Cited> {
    match document {
        TypedbDocument::Book(document) => Box::new(document.clone()),
        TypedbDocument::ResearchPaper(document) => Box::new(document.clone()),
        TypedbDocument::Report(document) => Box::new(document.clone()),
    }
}

fn identifier(node: &Node, kind: &str) -> Option<String> {
    node.descendants_named("idno")
        .into_iter()
        .find(|id| {
            id.attributes
                .get("type")
                .is_some_and(|value| value.eq_ignore_ascii_case(kind))
        })
        .map(text)
        .filter(|value| !value.is_empty())
}

fn article_dates(
    source: &Node,
) -> (
    Option<DateTime<FixedOffset>>,
    Option<DateTime<FixedOffset>>,
    Option<String>,
) {
    let submission_note = children_named(source, "note")
        .into_iter()
        .find(|note| note.attributes.get("type").map(String::as_str) == Some("submission"))
        .map(text);
    let (submission_date, acceptance_date) = submission_note
        .as_deref()
        .map(split_submission_dates)
        .unwrap_or_default();
    (submission_date, acceptance_date, submission_note)
}

fn split_submission_dates(
    note: &str,
) -> (Option<DateTime<FixedOffset>>, Option<DateTime<FixedOffset>>) {
    let submission_date = note
        .split_once("Received:")
        .map(|(_, value)| {
            value
                .split_once("Accepted:")
                .map_or(value, |(date, _)| date)
        })
        .map(str::trim)
        .and_then(parse_datetime);
    let acceptance_date = note
        .split_once("Accepted:")
        .map(|(_, value)| value.trim())
        .and_then(parse_datetime);
    (submission_date, acceptance_date)
}

fn parse_datetime(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok().or_else(|| {
        let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
        let datetime = date.and_hms_opt(0, 0, 0)?;
        FixedOffset::east_opt(0)?
            .from_local_datetime(&datetime)
            .single()
    })
}

fn back_matter(back: &Node, kind: &str) -> Option<String> {
    children_named(back, "div")
        .into_iter()
        .find(|div| div.attributes.get("type").map(String::as_str) == Some(kind))
        .map(text)
        .filter(|value| !value.is_empty())
}
