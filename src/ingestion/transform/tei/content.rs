use crate::models::{
    domain::{
        DocumentContent, Figure as DomainFigure, Section, StructuredReference, Table as DomainTable,
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
            collect_references_from_blocks(&back.content, &mut content.references);
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
        bounding_boxes: Vec::new(),
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
        bounding_boxes: Vec::new(),
    }
}

fn collect_references_from_blocks(blocks: &[Block], references: &mut Vec<StructuredReference>) {
    for block in blocks {
        match block {
            Block::Division(division) => {
                collect_references_from_blocks(&division.content, references);
            }
            Block::ListBibl(list) => {
                collect_references_from_list(list, references);
            }
            _ => {}
        }
    }
}

fn collect_references_from_list(list: &ListBibl, references: &mut Vec<StructuredReference>) {
    for bibl in &list.bibliographic_structures {
        references.push(structured_reference_from_bibl(bibl, references.len() + 1));
    }
}

fn structured_reference_from_bibl(bibl: &BiblStruct, index: usize) -> StructuredReference {
    let analytic = bibl.analytic.as_ref();
    let monograph = bibl.monographs.first();
    let title = analytic
        .and_then(|analytic| first_title_text(&analytic.titles))
        .or_else(|| monograph.and_then(|monograph| first_title_text(&monograph.titles)));
    let year = monograph
        .and_then(|monograph| monograph.imprint.as_ref())
        .and_then(|imprint| {
            imprint
                .dates
                .iter()
                .find_map(publication_date_from_tei_date)
        })
        .map(|date| date.year);
    let identifiers = publication_ids_from_bibl(bibl);

    StructuredReference {
        id: bibl
            .global
            .xml_id
            .clone()
            .unwrap_or_else(|| format!("ref-{index}")),
        title,
        authors: analytic
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
            }),
        journal: monograph.and_then(journal_title),
        year,
        identifiers,
    }
}
