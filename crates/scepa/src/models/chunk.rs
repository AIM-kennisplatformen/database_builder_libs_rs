use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Chunk {
    Text(Text),
    Abstract(Abstract),
    Figure(Figure),
    Image(Image),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub bounding_boxes: Vec<BoundingBox>,
    pub index: usize,
    pub section_heading: Option<String>,
    pub document_hash: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Abstract {
    pub bounding_boxes: Vec<BoundingBox>,
    pub index: usize,
    pub section_heading: Option<String>,
    pub document_hash: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    pub bounding_boxes: Vec<BoundingBox>,
    pub index: usize,
    pub section_heading: Option<String>,
    pub document_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub bounding_boxes: Vec<BoundingBox>,
    pub index: usize,
    pub section_heading: Option<String>,
    pub document_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
