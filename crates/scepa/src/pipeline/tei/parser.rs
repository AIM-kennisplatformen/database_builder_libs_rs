use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};
use rootcause::prelude::Report;

use crate::{
    domain::DocumentWithChunks,
    models::{
        chunk::{Abstract, Chunk, Figure, Image, Text},
        entities::{
            Entity,
            document::{Document as TypedbDocument, ResearchPaper},
            person::{Person, PersonEntity},
        },
        relations::{
            Relation,
            contribution::{Authorship, Contribution, Work as ContributionWork},
            publication_event::{
                Acceptance, PublicationEventRelation, Submission, Work as PublicationWork,
            },
        },
    },
};

use super::tree::{Node, child, children_named, descendant, parse_tree, text};

pub fn parse(xml: &str) -> Result<DocumentWithChunks, Report> {
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
    let doi = source.and_then(|node| identifier(node, "DOI"));
    let people = analytic
        .map(|node| {
            children_named(node, "author")
                .into_iter()
                .map(person)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let (submission_date, acceptance_date, submission_note) =
        source.map(article_dates).unwrap_or_default();
    let acknowledgements = back.and_then(|node| back_matter(node, "acknowledgement"));
    let conflicts = back.and_then(|node| back_matter(node, "conflict"));
    let contributions = back.and_then(|node| back_matter(node, "contribution"));

    let document = TypedbDocument::ResearchPaper(ResearchPaper {
        title,
        doi,
        abstract_text: abstract_text.clone(),
        acknowledgements: acknowledgements.clone(),
        conflicts: conflicts.clone(),
        contributions: contributions.clone(),
    });

    let mut relations = people
        .iter()
        .map(|person| {
            Relation::Contribution(Contribution::Authorship(Authorship {
                author: Some(Box::new(person_value(person))),
                authored_work: Some(contribution_work(&document)),
            }))
        })
        .collect::<Vec<_>>();

    if submission_date.is_some() || submission_note.is_some() {
        relations.push(Relation::PublicationEvent(
            PublicationEventRelation::Submission(Submission {
                publisher: None,
                venue: None,
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
                venue: None,
                work: publication_work(&document),
                acceptance_date,
            }),
        ));
    }

    Ok(DocumentWithChunks {
        document,
        entities: people.into_iter().map(Entity::Person).collect(),
        relations,
        chunks: parse_chunks(
            &root,
            abstract_node,
            body,
            back,
            acknowledgements.as_deref(),
            conflicts.as_deref(),
            contributions.as_deref(),
        ),
    })
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

fn person(node: &Node) -> PersonEntity {
    let name = child(node, "persName");
    PersonEntity::Person(Person {
        given_name: name.and_then(|name| child(name, "forename")).map(text),
        family_name: name.and_then(|name| child(name, "surname")).map(text),
    })
}

fn person_value(person: &PersonEntity) -> Person {
    match person {
        PersonEntity::Person(person) => person.clone(),
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
