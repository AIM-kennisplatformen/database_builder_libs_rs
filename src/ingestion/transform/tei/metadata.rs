use crate::models::{
    domain::PaperMetadata,
    tei::{
        bibliography::BiblStruct,
        document::TeiDocument,
        header::{Abstract, Funder, Keywords, ProfileDesc},
        text::{Block, ListOrg, Organization, TextPart},
    },
};

use super::{
    publication::first_title_text,
    text::{
        collect_division_text_chunks, item_text, normalized_opt, org_name_text, paragraph_text,
        push_unique,
    },
};

pub fn paper_metadata_from_tei(document: &TeiDocument) -> PaperMetadata {
    let abstract_text = document
        .header
        .profile_desc
        .as_ref()
        .and_then(|profile| profile.abstract_.as_ref())
        .and_then(abstract_text);

    let mut funding_statements = Vec::new();
    for funder in &document.header.file_desc.title_stmt.funders {
        push_unique(&mut funding_statements, funder_text(funder));
    }
    if let Some(text) = document.text.as_ref()
        && let Some(back) = text.back.as_ref()
    {
        collect_funding_from_text_part(back, &mut funding_statements);
    }

    PaperMetadata {
        abstract_text,
        keywords: document
            .header
            .profile_desc
            .as_ref()
            .map(keywords_from_profile)
            .unwrap_or_default(),
        funding_statements,
        acknowledgements: acknowledgements_from_tei(document),
        journal: None,
        volume: None,
        issue: None,
        pages: None,
    }
}

pub fn literature_title_from_tei(
    document: &TeiDocument,
    source_bibl: Option<&BiblStruct>,
) -> Option<String> {
    first_title_text(&document.header.file_desc.title_stmt.titles).or_else(|| {
        source_bibl
            .and_then(|bibl| bibl.analytic.as_ref())
            .and_then(|analytic| first_title_text(&analytic.titles))
    })
}

fn abstract_text(abstract_: &Abstract) -> Option<String> {
    let mut chunks = abstract_
        .paragraphs
        .iter()
        .filter_map(paragraph_text)
        .collect::<Vec<_>>();

    for division in &abstract_.divisions {
        collect_division_text_chunks(division, &mut chunks);
    }

    normalized_opt(Some(&chunks.join(" ")))
}

fn keywords_from_profile(profile: &ProfileDesc) -> Vec<String> {
    profile
        .text_class
        .as_ref()
        .map(|text_class| {
            let mut keywords = Vec::new();
            for keyword_block in &text_class.keywords {
                collect_keywords(keyword_block, &mut keywords);
            }
            keywords
        })
        .unwrap_or_default()
}

fn collect_keywords(keyword_block: &Keywords, keywords: &mut Vec<String>) {
    for term in &keyword_block.terms {
        push_unique(keywords, normalized_opt(term.text.as_deref()));
    }

    push_unique(keywords, normalized_opt(keyword_block.text.as_deref()));

    for list in &keyword_block.lists {
        for item in &list.items {
            push_unique(keywords, item_text(item));
        }
    }
}

fn acknowledgements_from_tei(document: &TeiDocument) -> Option<String> {
    let back = document.text.as_ref()?.back.as_ref()?;
    let mut chunks = Vec::new();

    for block in &back.content {
        if let Block::Division(division) = block
            && division.kind.as_deref() == Some("acknowledgement")
        {
            collect_division_text_chunks(division, &mut chunks);
        }
    }

    normalized_opt(Some(&chunks.join(" ")))
}

fn collect_funding_from_text_part(text_part: &TextPart, funding: &mut Vec<String>) {
    for block in &text_part.content {
        match block {
            Block::ListOrg(list_org) if list_org.typed.kind.as_deref() == Some("funding") => {
                collect_funding_from_list_org(list_org, funding);
            }
            Block::Division(division) => collect_funding_from_blocks(&division.content, funding),
            _ => {}
        }
    }
}

fn collect_funding_from_blocks(blocks: &[Block], funding: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::ListOrg(list_org) if list_org.typed.kind.as_deref() == Some("funding") => {
                collect_funding_from_list_org(list_org, funding);
            }
            Block::Division(division) => collect_funding_from_blocks(&division.content, funding),
            _ => {}
        }
    }
}

fn collect_funding_from_list_org(list_org: &ListOrg, funding: &mut Vec<String>) {
    for organization in &list_org.organizations {
        push_unique(funding, organization_text(organization));
    }
}

fn funder_text(funder: &Funder) -> Option<String> {
    let mut parts = funder
        .organization_names
        .iter()
        .filter_map(org_name_text)
        .collect::<Vec<_>>();

    if let Some(text) = normalized_opt(funder.text.as_deref()) {
        parts.push(text);
    }

    normalized_opt(Some(&parts.join(" ")))
}

fn organization_text(organization: &Organization) -> Option<String> {
    let mut parts = organization
        .organization_names
        .iter()
        .filter_map(org_name_text)
        .collect::<Vec<_>>();

    parts.extend(
        organization
            .identifiers
            .iter()
            .filter_map(|idno| normalized_opt(idno.text.as_deref())),
    );

    normalized_opt(Some(&parts.join(" ")))
}
