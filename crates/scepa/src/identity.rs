//! Deterministic identifiers for entities discovered during ingestion.
//!
//! This module is deliberately independent of the TEI parser and TypeDB
//! rendering.  Every caller therefore uses the same normalization rules when
//! it constructs an entity key or compares an identifier with one already in
//! the database.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IdentityError {
    #[error("{kind} must not be empty")]
    Empty { kind: &'static str },
    #[error("malformed {kind}: {value}")]
    Malformed { kind: &'static str, value: String },
    #[error("cannot construct a {kind} identity without a stable source-local identifier")]
    MissingSourceIdentity { kind: &'static str },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentKind {
    ResearchPaper,
    Book,
    Report,
}

impl DocumentKind {
    fn label(self) -> &'static str {
        match self {
            Self::ResearchPaper => "research paper",
            Self::Book => "book",
            Self::Report => "report",
        }
    }

    fn key_prefix(self) -> &'static str {
        match self {
            Self::ResearchPaper => "document",
            Self::Book => "document",
            Self::Report => "document",
        }
    }
}

pub fn normalize_doi(value: &str) -> Result<String, IdentityError> {
    let mut value = value.trim();
    let value_contains_url = value.starts_with("http") || value.contains("://");
    for prefix in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi:",
    ] {
        if value
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
        {
            value = value[prefix.len()..].trim();
            break;
        }
    }

    // GROBID occasionally puts a DOI URL, a DOI query parameter, or text
    // extracted immediately before the DOI into the idno value. Recover the
    // DOI itself when the prefix is recognizably URL-like or numeric noise.
    if let Some(start) = value
        .match_indices("10.")
        .find_map(|(start, _)| recoverable_doi_start(value, start).then_some(start))
    {
        value = &value[start..];
    }
    if value_contains_url {
        if let Some(end) = value.find("http://").or_else(|| value.find("https://")) {
            value = &value[..end];
        }
        if let Some((doi, _)) = value.split_once('?') {
            value = doi;
        }
        if let Some((doi, _)) = value.split_once('&') {
            value = doi;
        }
    }
    value = value.trim_end_matches('/').trim();

    if value.is_empty() {
        return Err(IdentityError::Empty { kind: "DOI" });
    }
    if !value.starts_with("10.")
        || !value.contains('/')
        || value.ends_with('/')
        || value.chars().any(char::is_whitespace)
    {
        return Err(malformed("DOI", value));
    }

    Ok(value.to_ascii_lowercase())
}

fn recoverable_doi_start(value: &str, start: usize) -> bool {
    if start == 0 {
        return true;
    }

    let prefix = &value[..start];
    prefix.starts_with('/')
        || prefix.starts_with("http")
        || prefix.ends_with('=')
        || prefix
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, '.' | '-'))
}

pub fn normalize_isbn(value: &str) -> Result<String, IdentityError> {
    let value = strip_label(value.trim(), "isbn");
    let normalized = remove_presentation_separators(value)?.to_ascii_uppercase();
    if normalized.len() == 10 {
        if !normalized[..9]
            .chars()
            .all(|character| character.is_ascii_digit())
            || !matches!(normalized.as_bytes()[9], b'0'..=b'9' | b'X')
            || !isbn10_checksum_is_valid(&normalized)
        {
            return Err(malformed("ISBN", normalized));
        }
    } else if normalized.len() == 13 {
        if !normalized
            .chars()
            .all(|character| character.is_ascii_digit())
            || !normalized.starts_with("97")
            || !isbn13_checksum_is_valid(&normalized)
        {
            return Err(malformed("ISBN", normalized));
        }
    } else {
        return Err(malformed("ISBN", normalized));
    }

    Ok(normalized)
}

pub fn normalize_issn(value: &str) -> Result<String, IdentityError> {
    let value = strip_label(value.trim(), "issn");
    let normalized = remove_presentation_separators(value)?.to_ascii_uppercase();
    if is_valid_issn(&normalized) {
        return Ok(normalized);
    }

    // GROBID sometimes joins the print and electronic ISSNs with labels and
    // punctuation. Keep the first valid ISSN instead of rejecting the whole
    // venue identifier.
    for candidate in issn_candidates(value) {
        if is_valid_issn(&candidate) {
            return Ok(candidate);
        }
    }

    Err(malformed("ISSN", normalized))
}

pub fn normalize_orcid(value: &str) -> Result<String, IdentityError> {
    let mut value = value.trim();
    for prefix in ["https://orcid.org/", "http://orcid.org/", "orcid:"] {
        if value
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
        {
            value = value[prefix.len()..].trim();
            break;
        }
    }
    let normalized = remove_presentation_separators(value)?.to_ascii_uppercase();
    if normalized.len() != 16
        || !normalized[..15]
            .chars()
            .all(|character| character.is_ascii_digit())
        || !matches!(normalized.as_bytes()[15], b'0'..=b'9' | b'X')
        || !orcid_checksum_is_valid(&normalized)
    {
        return Err(malformed("ORCID", normalized));
    }

    Ok(normalized)
}

pub fn normalize_pdf_hash(value: &str) -> Result<String, IdentityError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(IdentityError::Empty {
            kind: "PDF SHA-256",
        });
    }
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(malformed("PDF SHA-256", value));
    }
    Ok(value.to_ascii_lowercase())
}

/// Normalize a ROR-like authoritative identifier.  The database key keeps
/// the provider-neutral value; the `ror:` prefix added by `institution_id`
/// supplies the entity kind.
pub fn normalize_ror(value: &str) -> Result<String, IdentityError> {
    let mut value = value.trim();
    for prefix in ["https://ror.org/", "http://ror.org/", "ror:"] {
        if value
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
        {
            value = value[prefix.len()..].trim();
            break;
        }
    }
    if value.is_empty() {
        return Err(IdentityError::Empty { kind: "ROR" });
    }
    if value.chars().any(char::is_whitespace) {
        return Err(malformed("ROR", value));
    }
    Ok(value.to_ascii_lowercase())
}

pub fn document_id(
    kind: DocumentKind,
    strong_identifier: Option<&str>,
    pdf_hash: Option<&str>,
    source_document_id: Option<&str>,
    source_local_id: Option<&str>,
) -> Result<String, IdentityError> {
    let strong = match kind {
        DocumentKind::ResearchPaper => strong_identifier.map(normalize_doi).transpose()?,
        DocumentKind::Book => strong_identifier.map(normalize_isbn).transpose()?,
        DocumentKind::Report => strong_identifier
            .map(|value| normalize_source_part(value, "report identifier"))
            .transpose()?,
    };
    if let Some(strong) = strong {
        let namespace = match kind {
            DocumentKind::ResearchPaper => "doi",
            DocumentKind::Book => "isbn",
            DocumentKind::Report => "report",
        };
        return Ok(format!("{}:{}:{}", kind.key_prefix(), namespace, strong));
    }
    if let Some(pdf_hash) = pdf_hash {
        return Ok(format!(
            "{}:pdf-sha256:{}",
            kind.key_prefix(),
            normalize_pdf_hash(pdf_hash)?
        ));
    }
    source_local_key(
        kind.key_prefix(),
        kind.label(),
        source_document_id,
        source_local_id,
    )
}

pub fn person_id(
    orcid: Option<&str>,
    source_document_id: &str,
    author_index: usize,
) -> Result<String, IdentityError> {
    if let Some(orcid) = orcid.map(normalize_orcid).transpose()? {
        return Ok(format!("person:orcid:{orcid}"));
    }
    source_local_key(
        "person",
        "person",
        Some(source_document_id),
        Some(&format!("author-{author_index}")),
    )
}

pub fn institution_id(
    ror: Option<&str>,
    source_document_id: &str,
    affiliation_key: &str,
) -> Result<String, IdentityError> {
    if let Some(ror) = ror.map(normalize_ror).transpose()? {
        return Ok(format!("institution:ror:{ror}"));
    }
    source_local_key(
        "institution",
        "institution",
        Some(source_document_id),
        Some(affiliation_key),
    )
}

pub fn venue_id(
    issn: Option<&str>,
    source_document_id: &str,
    venue_key: &str,
) -> Result<String, IdentityError> {
    if let Some(issn) = issn.map(normalize_issn).transpose()? {
        return Ok(format!("publication-venue:issn:{issn}"));
    }
    source_local_key(
        "publication-venue",
        "publication venue",
        Some(source_document_id),
        Some(venue_key),
    )
}

fn source_local_key(
    prefix: &str,
    kind: &'static str,
    source_document_id: Option<&str>,
    source_local_id: Option<&str>,
) -> Result<String, IdentityError> {
    let source_document_id = source_document_id
        .map(|value| normalize_source_part(value, "source document ID"))
        .transpose()?;
    let source_local_id = source_local_id
        .map(|value| normalize_source_part(value, "source-local ID"))
        .transpose()?;
    match (source_document_id, source_local_id) {
        (Some(source_document_id), Some(source_local_id)) => Ok(format!(
            "{prefix}:source:{source_document_id}:{source_local_id}"
        )),
        _ => Err(IdentityError::MissingSourceIdentity { kind }),
    }
}

fn normalize_source_part(value: &str, kind: &'static str) -> Result<String, IdentityError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(IdentityError::Empty { kind });
    }
    if value.chars().any(char::is_whitespace) {
        return Err(malformed(kind, value));
    }
    Ok(value.to_owned())
}

fn strip_label<'a>(value: &'a str, label: &str) -> &'a str {
    value
        .get(..label.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(label))
        .and_then(|_| value.get(label.len()..))
        .map(str::trim)
        .unwrap_or(value)
}

fn remove_presentation_separators(value: &str) -> Result<String, IdentityError> {
    if value.trim().is_empty() {
        return Err(IdentityError::Empty { kind: "identifier" });
    }
    Ok(value
        .chars()
        .filter(|character| !matches!(character, '-' | ' ' | '\t' | '\n' | '\r'))
        .collect())
}

fn isbn10_checksum_is_valid(value: &str) -> bool {
    value
        .chars()
        .enumerate()
        .map(|(index, character)| {
            let digit = if character == 'X' {
                10
            } else {
                character.to_digit(10).unwrap_or(100)
            };
            (10 - index as u32) * digit
        })
        .sum::<u32>()
        .is_multiple_of(11)
}

fn isbn13_checksum_is_valid(value: &str) -> bool {
    value
        .chars()
        .enumerate()
        .map(|(index, character)| {
            character.to_digit(10).unwrap_or(100) * if index % 2 == 0 { 1 } else { 3 }
        })
        .sum::<u32>()
        .is_multiple_of(10)
}

fn issn_checksum_is_valid(value: &str) -> bool {
    let sum = value
        .chars()
        .take(7)
        .enumerate()
        .map(|(index, character)| character.to_digit(10).unwrap_or(100) * (8 - index as u32))
        .sum::<u32>();
    let check = if value.ends_with('X') {
        10
    } else {
        value
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .unwrap_or(100)
    };
    (sum + check).is_multiple_of(11)
}

fn is_valid_issn(value: &str) -> bool {
    value.len() == 8
        && value[..7]
            .chars()
            .all(|character| character.is_ascii_digit())
        && matches!(value.as_bytes()[7], b'0'..=b'9' | b'X')
        && issn_checksum_is_valid(value)
}

fn issn_candidates(value: &str) -> Vec<String> {
    let characters = value.to_ascii_uppercase().chars().collect::<Vec<_>>();
    let mut candidates = Vec::new();

    for start in 0..characters.len() {
        if !characters[start].is_ascii_digit()
            || (start > 0 && characters[start - 1].is_ascii_digit())
        {
            continue;
        }

        let mut candidate = String::new();
        let mut index = start;
        while index < characters.len() {
            let character = characters[index];
            if character.is_ascii_digit() || character == 'X' {
                candidate.push(character);
            } else if character == '-' || character.is_ascii_whitespace() {
                // Presentation separators may occur inside an ISSN.
            } else {
                break;
            }
            index += 1;
        }

        if candidate.len() == 8
            && (index == characters.len() || !characters[index].is_ascii_digit())
        {
            candidates.push(candidate);
        }
    }

    candidates
}

fn orcid_checksum_is_valid(value: &str) -> bool {
    let mut remainder = 0u32;
    for character in value.chars().take(15) {
        remainder = (remainder + character.to_digit(10).unwrap()) * 2 % 11;
    }
    let expected = (12 - remainder) % 11;
    let actual = if value.ends_with('X') {
        10
    } else {
        value.chars().last().and_then(|c| c.to_digit(10)).unwrap()
    };
    expected == actual
}

fn malformed(kind: &'static str, value: impl Into<String>) -> IdentityError {
    IdentityError::Malformed {
        kind,
        value: value.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_external_identifiers() {
        let cases = [
            (normalize_doi(" DOI:10.1234/ABC "), "10.1234/abc"),
            (normalize_doi("https://doi.org/10.1234/ABC"), "10.1234/abc"),
            (
                normalize_doi("https://www.aeaweb.org/articles?id=10.1257/aer.p20161101"),
                "10.1257/aer.p20161101",
            ),
            (
                normalize_doi("https:/doi.org/10.1016/j.erss.2022.102892"),
                "10.1016/j.erss.2022.102892",
            ),
            (normalize_doi("10.18352/jsi.186/"), "10.18352/jsi.186"),
            (
                normalize_doi("75111314310.1111/j.1549-0831.2009.00004.x"),
                "10.1111/j.1549-0831.2009.00004.x",
            ),
            (
                normalize_doi("2021.1925-474.110.5539/hes.v11n2p42"),
                "10.5539/hes.v11n2p42",
            ),
            (
                normalize_doi("/10.1016/j.energy.2015.07.073"),
                "10.1016/j.energy.2015.07.073",
            ),
            (normalize_isbn("978-0-306-40615-7"), "9780306406157"),
            (normalize_isbn("0-306-40615-2"), "0306406152"),
            (normalize_issn("2049-3630"), "20493630"),
            (normalize_issn("2668-7003, -L 2457-5011"), "26687003"),
            (normalize_issn("1947-5683 (Print) 1947-5691"), "19475683"),
            (normalize_issn("0027-8424, 1091-6490"), "00278424"),
            (
                normalize_orcid("https://orcid.org/0000-0002-1825-0097"),
                "0000000218250097",
            ),
        ];

        for (actual, expected) in cases {
            assert_eq!(actual.unwrap(), expected);
        }
    }

    #[test]
    fn rejects_empty_or_malformed_identifiers() {
        assert!(matches!(
            normalize_doi(" "),
            Err(IdentityError::Empty { .. })
        ));
        assert!(normalize_doi("not-a-doi").is_err());
        assert!(normalize_doi("not-a-doi10.1234/example").is_err());
        assert!(normalize_isbn("").is_err());
        assert!(normalize_isbn("9780306406158").is_err());
        assert!(normalize_issn("2049-3631").is_err());
        assert!(normalize_orcid("0000-0002-1825-0098").is_err());
        assert!(normalize_pdf_hash("abc").is_err());
    }

    #[test]
    fn shared_strong_identifiers_converge_to_one_key() {
        let cited = document_id(
            DocumentKind::ResearchPaper,
            Some("https://doi.org/10.X/ABC"),
            None,
            None,
            None,
        )
        .unwrap();
        let primary = document_id(
            DocumentKind::ResearchPaper,
            Some("doi:10.x/abc"),
            Some(&"a".repeat(64)),
            None,
            None,
        )
        .unwrap();
        assert_eq!(cited, "document:doi:10.x/abc");
        assert_eq!(cited, primary);
    }

    #[test]
    fn title_only_records_use_source_local_identity() {
        let first = document_id(
            DocumentKind::ResearchPaper,
            None,
            None,
            Some("document:doi:10.x/citing"),
            Some("ref-1"),
        )
        .unwrap();
        let second = document_id(
            DocumentKind::ResearchPaper,
            None,
            None,
            Some("document:doi:10.x/citing"),
            Some("ref-2"),
        )
        .unwrap();
        assert_ne!(first, second);
        assert_eq!(first, "document:source:document:doi:10.x/citing:ref-1");
    }

    #[test]
    fn local_fallbacks_include_stable_provenance() {
        assert_eq!(
            person_id(None, "document:pdf-sha256:abc", 3).unwrap(),
            "person:source:document:pdf-sha256:abc:author-3"
        );
        assert_eq!(
            institution_id(None, "document:pdf-sha256:abc", "aff-0").unwrap(),
            "institution:source:document:pdf-sha256:abc:aff-0"
        );
        assert_eq!(
            venue_id(Some("2049-3630"), "unused", "venue-0").unwrap(),
            "publication-venue:issn:20493630"
        );
    }
}
