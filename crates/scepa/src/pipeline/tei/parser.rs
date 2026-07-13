use std::collections::HashMap;

use rootcause::prelude::Report;

use super::{
    Address, Affiliation, ArticleDates, Author, Document, Figure, FigureImage, Funding,
    Organization, Reference, Section,
    tree::{Node, child, children_named, descendant, parse_tree, text},
};

pub fn parse(xml: &str) -> Result<Document, Report> {
    let root = parse_tree(xml)?;
    let header = descendant(&root, "teiHeader");
    let body = descendant(&root, "body");
    let source = header.and_then(|node| descendant(node, "biblStruct"));
    let analytic = source.and_then(|node| child(node, "analytic"));

    let title = header
        .and_then(|node| descendant(node, "titleStmt"))
        .and_then(|node| {
            children_named(node, "title")
                .into_iter()
                .find(|title| title.attributes.get("type").map(String::as_str) == Some("main"))
                .or_else(|| child(node, "title"))
        })
        .map(text);
    let authors = analytic
        .map(|node| {
            children_named(node, "author")
                .into_iter()
                .map(author)
                .collect()
        })
        .unwrap_or_default();
    let abstract_text = header
        .and_then(|node| descendant(node, "abstract"))
        .map(text);
    let doi = source.and_then(|node| identifier(node, "DOI"));
    let dates = source.map(article_dates).unwrap_or_default();
    let funding = parse_funding(&root);
    let back = descendant(&root, "back");
    let sections = body.map(parse_sections).unwrap_or_default();
    let figures = root
        .descendants_named("figure")
        .into_iter()
        .map(figure)
        .collect();
    let references = root
        .descendants_named("biblStruct")
        .into_iter()
        .filter(|node| node.attributes.contains_key("id"))
        .map(reference)
        .collect();

    Ok(Document {
        title,
        authors,
        doi,
        dates,
        funding,
        abstract_text,
        sections,
        figures,
        acknowledgements: back.and_then(|node| back_matter(node, "acknowledgement")),
        conflicts: back.and_then(|node| back_matter(node, "conflict")),
        contributions: back.and_then(|node| back_matter(node, "contribution")),
        references,
    })
}

fn figure(node: &Node) -> Figure {
    Figure {
        id: node.attributes.get("id").cloned(),
        kind: node.attributes.get("type").cloned(),
        heading: child(node, "head")
            .map(text)
            .filter(|value| !value.is_empty()),
        label: child(node, "label")
            .map(text)
            .filter(|value| !value.is_empty()),
        caption: child(node, "figDesc")
            .map(text)
            .filter(|value| !value.is_empty()),
        images: node
            .descendants_named("graphic")
            .into_iter()
            .chain(node.descendants_named("media"))
            .map(|image| FigureImage {
                url: image
                    .attributes
                    .get("url")
                    .or_else(|| image.attributes.get("target"))
                    .cloned(),
                media_type: image.attributes.get("type").cloned(),
                coordinates: image.attributes.get("coords").cloned(),
            })
            .collect(),
    }
}

fn parse_sections(body: &Node) -> Vec<Section> {
    body.descendants_named("div")
        .into_iter()
        .filter_map(|div| {
            let heading = child(div, "head").map(text);
            let paragraphs = div
                .descendants_named("p")
                .into_iter()
                .map(text)
                .collect::<Vec<_>>();
            (heading.is_some() || !paragraphs.is_empty()).then_some(Section {
                heading,
                paragraphs,
            })
        })
        .collect()
}

fn author(node: &Node) -> Author {
    let name = child(node, "persName");
    Author {
        given_name: name.and_then(|name| child(name, "forename")).map(text),
        middle_names: name
            .map(|name| {
                children_named(name, "forename")
                    .into_iter()
                    .filter(|forename| {
                        forename.attributes.get("type").map(String::as_str) == Some("middle")
                    })
                    .map(text)
                    .collect()
            })
            .unwrap_or_default(),
        surname: name.and_then(|name| child(name, "surname")).map(text),
        email: child(node, "email").map(text),
        orcid: identifier(node, "ORCID"),
        affiliations: children_named(node, "affiliation")
            .into_iter()
            .map(affiliation)
            .collect(),
    }
}

fn affiliation(node: &Node) -> Affiliation {
    Affiliation {
        key: node.attributes.get("key").cloned(),
        organizations: children_named(node, "orgName")
            .into_iter()
            .map(|org| Organization {
                kind: org.attributes.get("type").cloned(),
                name: text(org),
            })
            .collect(),
        address: child(node, "address").map(|address| Address {
            settlement: child(address, "settlement").map(text),
            region: child(address, "region").map(text),
            country: child(address, "country").map(text),
        }),
    }
}

fn reference(node: &Node) -> Reference {
    let analytic = child(node, "analytic");
    let monogr = child(node, "monogr");
    let imprint = monogr.and_then(|node| child(node, "imprint"));
    Reference {
        id: node.attributes.get("id").cloned(),
        title: analytic
            .and_then(|node| child(node, "title"))
            .or_else(|| monogr.and_then(|node| child(node, "title")))
            .map(text)
            .filter(|title| !title.is_empty()),
        authors: analytic
            .or(monogr)
            .map(|node| {
                children_named(node, "author")
                    .into_iter()
                    .map(author)
                    .collect()
            })
            .unwrap_or_default(),
        doi: identifier(node, "DOI"),
        journal: monogr
            .and_then(|node| {
                children_named(node, "title")
                    .into_iter()
                    .find(|title| title.attributes.get("level").map(String::as_str) == Some("j"))
            })
            .map(text),
        publication_year: imprint
            .and_then(|node| child(node, "date"))
            .and_then(publication_year),
        volume: imprint
            .and_then(|node| scope(node, "volume"))
            .map(text)
            .filter(|value| !value.is_empty()),
        pages: imprint
            .and_then(|node| scope(node, "page"))
            .and_then(page_range),
        external_urls: node
            .descendants_named("ptr")
            .into_iter()
            .chain(node.descendants_named("ref"))
            .filter_map(|link| link.attributes.get("target").cloned())
            .collect(),
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

fn scope<'a>(node: &'a Node, unit: &str) -> Option<&'a Node> {
    children_named(node, "biblScope")
        .into_iter()
        .find(|scope| scope.attributes.get("unit").map(String::as_str) == Some(unit))
}

fn page_range(scope: &Node) -> Option<String> {
    match (scope.attributes.get("from"), scope.attributes.get("to")) {
        (Some(from), Some(to)) => Some(format!("{from}-{to}")),
        (Some(from), None) => Some(from.clone()),
        _ => (!text(scope).is_empty()).then(|| text(scope)),
    }
}

fn publication_year(date: &Node) -> Option<String> {
    let value = date
        .attributes
        .get("when")
        .cloned()
        .unwrap_or_else(|| text(date));
    value
        .get(..4)
        .filter(|year| year.chars().all(|character| character.is_ascii_digit()))
        .map(str::to_owned)
}

fn article_dates(source: &Node) -> ArticleDates {
    let submission_note = children_named(source, "note")
        .into_iter()
        .find(|note| note.attributes.get("type").map(String::as_str) == Some("submission"))
        .map(text);
    let (submission_date, acceptance_date) = submission_note
        .as_deref()
        .map(split_submission_dates)
        .unwrap_or_default();
    ArticleDates {
        submission_date,
        acceptance_date,
        submission_note,
    }
}

fn split_submission_dates(note: &str) -> (Option<String>, Option<String>) {
    let submission_date = note
        .split_once("Received:")
        .map(|(_, value)| {
            value
                .split_once("Accepted:")
                .map_or(value, |(date, _)| date)
        })
        .map(str::trim)
        .map(str::to_owned)
        .filter(|value| !value.is_empty());
    let acceptance_date = note
        .split_once("Accepted:")
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    (submission_date, acceptance_date)
}

fn parse_funding(root: &Node) -> Vec<Funding> {
    let organizations = root
        .descendants_named("org")
        .into_iter()
        .filter_map(|org| {
            Some((
                org.attributes.get("id")?.clone(),
                (
                    child(org, "orgName").map(text),
                    identifier(org, "grant-number"),
                ),
            ))
        })
        .collect::<HashMap<_, _>>();

    root.descendants_named("funder")
        .into_iter()
        .map(|funder| {
            let reference = funder.attributes.get("ref").cloned();
            let linked = reference
                .as_deref()
                .and_then(|reference| organizations.get(reference.trim_start_matches('#')));
            Funding {
                name: child(funder, "orgName").map(text),
                reference,
                project: linked.and_then(|(project, _)| project.clone()),
                grant_number: linked.and_then(|(_, grant_number)| grant_number.clone()),
            }
        })
        .collect()
}

fn back_matter(back: &Node, kind: &str) -> Option<String> {
    children_named(back, "div")
        .into_iter()
        .find(|div| div.attributes.get("type").map(String::as_str) == Some(kind))
        .map(text)
        .filter(|value| !value.is_empty())
}
