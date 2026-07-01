use crate::models::{
    domain::{Affiliation as DomainAffiliation, Author as DomainAuthor},
    tei::{
        bibliography::{Affiliation as TeiAffiliation, Author as TeiAuthor, BiblStruct},
        document::TeiDocument,
    },
};

use super::text::normalized_opt;

pub fn authors_from_tei(
    document: &TeiDocument,
    source_bibl: Option<&BiblStruct>,
) -> Vec<DomainAuthor> {
    let source_authors = source_bibl
        .and_then(|bibl| bibl.analytic.as_ref())
        .map(|analytic| analytic.authors.as_slice())
        .unwrap_or_default();

    if !source_authors.is_empty() {
        return source_authors.iter().map(author_from_tei).collect();
    }

    document
        .header
        .file_desc
        .title_stmt
        .authors
        .iter()
        .map(author_from_tei)
        .collect()
}

pub fn author_from_tei(author: &TeiAuthor) -> DomainAuthor {
    let person_name = author.person_names.first();

    DomainAuthor {
        first_name: person_name.and_then(|name| {
            name.forenames
                .iter()
                .find(|part| part.typed.kind.as_deref() == Some("first"))
                .or_else(|| name.forenames.first())
                .and_then(|part| normalized_opt(part.text.as_deref()))
        }),
        middle_name: person_name.and_then(|name| {
            let middle = name
                .forenames
                .iter()
                .filter(|part| part.typed.kind.as_deref() == Some("middle"))
                .filter_map(|part| normalized_opt(part.text.as_deref()))
                .collect::<Vec<_>>()
                .join(" ");
            normalized_opt(Some(&middle))
        }),
        last_name: person_name.and_then(|name| {
            name.surnames
                .first()
                .and_then(|surname| normalized_opt(surname.text.as_deref()))
        }),
        affiliations: author
            .affiliations
            .iter()
            .map(affiliation_from_tei)
            .collect(),
    }
}

fn affiliation_from_tei(affiliation: &TeiAffiliation) -> DomainAffiliation {
    let mut domain = DomainAffiliation::default();

    for org in &affiliation.organization_names {
        let Some(text) = normalized_opt(org.text.as_deref()) else {
            continue;
        };

        match org.typed.kind.as_deref() {
            Some("laboratory") if domain.laboratory.is_none() => domain.laboratory = Some(text),
            Some("department") if domain.department.is_none() => domain.department = Some(text),
            Some("institution") if domain.institution.is_none() => domain.institution = Some(text),
            _ if domain.institution.is_none() => domain.institution = Some(text),
            _ => {}
        }
    }

    if let Some(address) = affiliation.address.as_ref() {
        domain.settlement = address
            .settlements
            .iter()
            .find_map(|settlement| normalized_opt(settlement.text.as_deref()));
        domain.country = address
            .countries
            .iter()
            .find_map(|country| normalized_opt(country.text.as_deref()));
    }

    domain
}
