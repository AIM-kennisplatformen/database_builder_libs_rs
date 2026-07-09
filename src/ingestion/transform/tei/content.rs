use crate::models::{
    domain::{
        Authoring, Citation, DocumentContent, Figure as DomainFigure, Literature, LiteratureCore,
        ScientificLiterature, Section, Table as DomainTable,
    },
    tei::{
        bibliography::BiblStruct,
        document::TeiDocument,
        text::{Block, Division, Figure as TeiFigure, ListBibl, Table as TeiTable},
    },
};

use super::{
    authors::reference_author_from_tei,
    publication::{
        first_title_text, journal_title, publication_date_from_tei_date, publication_ids_from_bibl,
    },
    text::{figure_caption, head_text, inline_text, paragraph_text, table_raw_content},
};

pub fn document_content_from_tei(document: &TeiDocument) -> DocumentContent {
    let mut content = DocumentContent::default();
    let mut figure_index = 1;
    let mut table_index = 1;

    if let Some(text) = document.text.as_ref() {
        if let Some(body) = text.body.as_ref() {
            collect_sections_from_blocks(&body.content, &mut content.sections);
            collect_figures_and_tables(
                &body.content,
                &mut content.figures,
                &mut content.tables,
                &mut figure_index,
                &mut table_index,
            );
        }

        if let Some(back) = text.back.as_ref() {
            collect_figures_and_tables(
                &back.content,
                &mut content.figures,
                &mut content.tables,
                &mut figure_index,
                &mut table_index,
            );
        }
    }

    content
}

pub fn citations_from_tei(document: &TeiDocument) -> Vec<Citation> {
    let mut citations = vec![];

    if let Some(text) = document.text.as_ref()
        && let Some(back) = text.back.as_ref()
    {
        collect_citations_from_blocks(&back.content, &mut citations);
    }

    citations
}

fn collect_sections_from_blocks(blocks: &[Block], sections: &mut Vec<Section>) {
    for block in blocks {
        match block {
            Block::Division(division) => {
                if let Some(section) = section_from_division(division) {
                    sections.push(section);
                }
                collect_sections_from_blocks(&division.content, sections);
            }
            Block::Paragraph(paragraph) => {
                if let Some(text) = paragraph_text(paragraph) {
                    sections.push(Section {
                        title: String::new(),
                        text_chunks: vec![text],
                    });
                }
            }
            _ => {}
        }
    }
}

fn section_from_division(division: &Division) -> Option<Section> {
    let title = division
        .content
        .iter()
        .find_map(|block| match block {
            Block::Head(head) => head_text(head),
            _ => None,
        })
        .unwrap_or_default();

    let text_chunks = division
        .content
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(paragraph) => paragraph_text(paragraph),
            _ => None,
        })
        .collect::<Vec<_>>();

    if title.is_empty() && text_chunks.is_empty() {
        None
    } else {
        Some(Section { title, text_chunks })
    }
}

fn collect_figures_and_tables(
    blocks: &[Block],
    figures: &mut Vec<DomainFigure>,
    tables: &mut Vec<DomainTable>,
    figure_index: &mut usize,
    table_index: &mut usize,
) {
    for block in blocks {
        match block {
            Block::Division(division) => collect_figures_and_tables(
                &division.content,
                figures,
                tables,
                figure_index,
                table_index,
            ),
            Block::Figure(figure) => {
                figures.push(figure_from_tei(figure, *figure_index));
                *figure_index += 1;
            }
            Block::Table(table) => {
                tables.push(table_from_tei(table, *table_index));
                *table_index += 1;
            }
            _ => {}
        }
    }
}

fn figure_from_tei(figure: &TeiFigure, index: usize) -> DomainFigure {
    DomainFigure {
        id: figure
            .global
            .xml_id
            .clone()
            .unwrap_or_else(|| format!("figure-{index}")),
        label: figure
            .labels
            .iter()
            .find_map(|label| inline_text(&label.content)),
        caption: figure_caption(figure),
        bounding_boxes: vec![],
    }
}

fn table_from_tei(table: &TeiTable, index: usize) -> DomainTable {
    DomainTable {
        id: table
            .global
            .xml_id
            .clone()
            .unwrap_or_else(|| format!("table-{index}")),
        label: None,
        caption: table.heads.iter().find_map(head_text),
        raw_content: table_raw_content(table),
        bounding_boxes: vec![],
    }
}

fn collect_citations_from_blocks(blocks: &[Block], citations: &mut Vec<Citation>) {
    for block in blocks {
        match block {
            Block::Division(division) => {
                collect_citations_from_blocks(&division.content, citations);
            }
            Block::ListBibl(list) => {
                collect_citations_from_list(list, citations);
            }
            _ => {}
        }
    }
}

fn collect_citations_from_list(list: &ListBibl, citations: &mut Vec<Citation>) {
    for bibl in &list.bibliographic_structures {
        citations.push(citation_from_bibl(bibl, citations.len() + 1));
    }
}

fn citation_from_bibl(bibl: &BiblStruct, index: usize) -> Citation {
    let analytic = bibl.analytic.as_ref();
    let monograph = bibl.monographs.first();
    let title = analytic
        .and_then(|analytic| first_title_text(&analytic.titles))
        .or_else(|| monograph.and_then(|monograph| first_title_text(&monograph.titles)));
    let publishing_date = monograph
        .and_then(|monograph| monograph.imprint.as_ref())
        .and_then(|imprint| {
            imprint
                .dates
                .iter()
                .find_map(publication_date_from_tei_date)
        });
    let identifiers = publication_ids_from_bibl(bibl);
    let authors: Vec<_> = analytic
        .map(|analytic| {
            analytic
                .authors
                .iter()
                .map(reference_author_from_tei)
                .collect()
        })
        .unwrap_or_else(|| {
            monograph
                .map(|monograph| {
                    monograph
                        .authors
                        .iter()
                        .map(reference_author_from_tei)
                        .collect()
                })
                .unwrap_or_default()
        });

    Citation {
        id: bibl
            .global
            .xml_id
            .clone()
            .unwrap_or_else(|| format!("ref-{index}")),
        cited: Literature::Scientific(ScientificLiterature {
            core: LiteratureCore {
                title,
                publishing_date,
                issn: identifiers.issn,
                isbn: identifiers.isbn,
            },
            doi: identifiers.doi,
        }),
        authorings: authors
            .into_iter()
            .map(|author| Authoring {
                author,
                affiliations: vec![],
            })
            .collect(),
        journal: monograph.and_then(journal_title),
    }
}
