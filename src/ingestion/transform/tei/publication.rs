use crate::models::{
    domain::{PublicationContext, PublicationDate, PublicationIds},
    tei::{
        bibliography::{BiblScope, BiblStruct, Date as TeiDate, Idno, Monogr, Title},
        document::TeiDocument,
    },
};

use super::text::normalized_opt;

pub fn publication_context_from_tei(
    document: &TeiDocument,
    source_bibl: Option<&BiblStruct>,
) -> PublicationContext {
    let mut publication = source_bibl
        .map(publication_context_from_bibl)
        .unwrap_or_default();

    if publication.publisher.is_none() {
        publication.publisher = document
            .header
            .file_desc
            .publication_stmt
            .publishers
            .iter()
            .find_map(|publisher| normalized_opt(publisher.text.as_deref()));
    }

    publication
}

fn publication_context_from_bibl(bibl: &BiblStruct) -> PublicationContext {
    let monograph = bibl.monographs.first();
    let imprint = monograph.and_then(|monograph| monograph.imprint.as_ref());

    PublicationContext {
        identifiers: publication_ids_from_bibl(bibl),
        publisher: imprint.and_then(|imprint| {
            imprint
                .publishers
                .iter()
                .find_map(|publisher| normalized_opt(publisher.text.as_deref()))
        }),
        journal: monograph.and_then(journal_title),
        date: imprint.and_then(|imprint| {
            imprint
                .dates
                .iter()
                .find_map(publication_date_from_tei_date)
        }),
        volume: imprint.and_then(|imprint| scope_text(&imprint.bibliographic_scopes, "volume")),
        issue: imprint.and_then(|imprint| scope_text(&imprint.bibliographic_scopes, "issue")),
        pages: imprint.and_then(|imprint| scope_text(&imprint.bibliographic_scopes, "page")),
    }
}

pub fn publication_ids_from_bibl(bibl: &BiblStruct) -> PublicationIds {
    let mut ids = PublicationIds::default();

    apply_identifiers(&mut ids, &bibl.identifiers);

    if let Some(analytic) = bibl.analytic.as_ref() {
        apply_identifiers(&mut ids, &analytic.identifiers);
    }

    for monograph in &bibl.monographs {
        apply_identifiers(&mut ids, &monograph.identifiers);
    }

    if let Some(series) = bibl.series.as_ref() {
        apply_identifiers(&mut ids, &series.identifiers);
    }

    ids
}

fn apply_identifiers(ids: &mut PublicationIds, identifiers: &[Idno]) {
    for identifier in identifiers {
        let Some(value) = normalized_opt(identifier.text.as_deref()) else {
            continue;
        };

        match identifier
            .typed
            .kind
            .as_deref()
            .map(|kind| kind.to_ascii_uppercase())
            .as_deref()
        {
            Some("DOI") if ids.doi.is_none() => ids.doi = Some(value),
            Some("ISBN") if ids.isbn.is_none() => ids.isbn = Some(value),
            Some("ISSN") if ids.issn.is_none() => ids.issn = Some(value),
            _ => {}
        }
    }
}

pub fn first_title_text(titles: &[Title]) -> Option<String> {
    titles
        .iter()
        .find(|title| title.typed.kind.as_deref() == Some("main"))
        .or_else(|| titles.first())
        .and_then(|title| normalized_opt(title.text.as_deref()))
}

pub fn journal_title(monograph: &Monogr) -> Option<String> {
    monograph
        .titles
        .iter()
        .find(|title| title.level.as_deref() == Some("j"))
        .or_else(|| monograph.titles.first())
        .and_then(|title| normalized_opt(title.text.as_deref()))
}

fn scope_text(scopes: &[BiblScope], unit: &str) -> Option<String> {
    scopes
        .iter()
        .find(|scope| scope.unit.as_deref() == Some(unit))
        .and_then(|scope| {
            if let (Some(from), Some(to)) = (scope.from_value.as_deref(), scope.to_value.as_deref())
            {
                normalized_opt(Some(&format!("{from}-{to}")))
            } else {
                normalized_opt(scope.text.as_deref())
            }
        })
}

pub fn publication_date_from_tei_date(date: &TeiDate) -> Option<PublicationDate> {
    let source = date.when.as_deref().or(date.text.as_deref())?;
    publication_date_from_str(source)
}

fn publication_date_from_str(source: &str) -> Option<PublicationDate> {
    let source = source.trim();
    let year = first_year(source)?;
    let month = date_part(source, 5, 7).filter(|month| (1..=12).contains(month));
    let day = date_part(source, 8, 10).filter(|day| (1..=31).contains(day));

    Some(PublicationDate { year, month, day })
}

fn first_year(source: &str) -> Option<u16> {
    source
        .as_bytes()
        .windows(4)
        .find_map(|window| {
            if window.iter().all(u8::is_ascii_digit) {
                std::str::from_utf8(window)
                    .ok()
                    .and_then(|year| year.parse::<u16>().ok())
            } else {
                None
            }
        })
        .filter(|year| (1000..=3000).contains(year))
}

fn date_part(source: &str, start: usize, end: usize) -> Option<u8> {
    source
        .get(start..end)
        .and_then(|part| part.parse::<u8>().ok())
}
