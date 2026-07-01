use crate::models::tei::{
    bibliography::OrgName,
    text::{
        Block, Cell, Division, Figure as TeiFigure, Head, Inline, Item, Paragraph,
        Table as TeiTable,
    },
};

pub fn collect_division_text_chunks(division: &Division, chunks: &mut Vec<String>) {
    for block in &division.content {
        match block {
            Block::Paragraph(paragraph) => {
                if let Some(text) = paragraph_text(paragraph) {
                    chunks.push(text);
                }
            }
            Block::Division(division) => collect_division_text_chunks(division, chunks),
            Block::List(list) => {
                for item in &list.items {
                    if let Some(text) = item_text(item) {
                        chunks.push(text);
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn figure_caption(figure: &TeiFigure) -> Option<String> {
    let mut chunks = figure
        .heads
        .iter()
        .filter_map(head_text)
        .collect::<Vec<_>>();

    chunks.extend(figure.descriptions.iter().filter_map(paragraph_text));
    chunks.extend(figure.paragraphs.iter().filter_map(paragraph_text));

    normalized_opt(Some(&chunks.join(" ")))
}

pub fn table_raw_content(table: &TeiTable) -> Option<String> {
    let rows = table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .filter_map(cell_text)
                .collect::<Vec<_>>()
                .join("\t")
        })
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();

    normalized_opt(Some(&rows.join("\n")))
}

fn cell_text(cell: &Cell) -> Option<String> {
    inline_text(&cell.content)
}

pub fn head_text(head: &Head) -> Option<String> {
    inline_text(&head.content)
}

pub fn paragraph_text(paragraph: &Paragraph) -> Option<String> {
    inline_text(&paragraph.content)
}

pub fn item_text(item: &Item) -> Option<String> {
    inline_text(&item.content)
}

pub fn inline_text(inline: &[Inline]) -> Option<String> {
    let text = inline
        .iter()
        .filter_map(|node| match node {
            Inline::Text(text) => Some(text.clone()),
            Inline::Reference(reference) => inline_text(&reference.content),
            Inline::ReferencingString(rs) => inline_text(&rs.content),
            Inline::Highlighted(highlighted) => inline_text(&highlighted.content),
            Inline::Note(note) => paragraph_text(note),
            Inline::Formula(formula) => normalized_opt(formula.text.as_deref()),
            Inline::Figure(figure) => figure_caption(figure),
            Inline::Pointer(_) | Inline::LineBreak(_) | Inline::PageBreak(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ");

    normalized_opt(Some(&text))
}

pub fn org_name_text(org_name: &OrgName) -> Option<String> {
    normalized_opt(org_name.text.as_deref())
}

pub fn normalized_opt(text: Option<&str>) -> Option<String> {
    text.map(normalize_whitespace)
        .filter(|text| !text.is_empty())
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn push_unique(values: &mut Vec<String>, value: Option<String>) {
    let Some(value) = value else {
        return;
    };

    if !values.contains(&value) {
        values.push(value);
    }
}
