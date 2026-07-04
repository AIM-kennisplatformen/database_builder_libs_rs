use serde::Serialize;

use super::pdf::BoundingBox;

#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct DocumentContent {
    pub sections: Vec<Section>,
    pub figures: Vec<Figure>,
    pub tables: Vec<Table>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default)]
pub struct Section {
    pub title: String,
    pub text_chunks: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct Figure {
    pub id: String,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub bounding_boxes: Vec<BoundingBox>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct Table {
    pub id: String,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub raw_content: Option<String>,
    pub bounding_boxes: Vec<BoundingBox>,
}
