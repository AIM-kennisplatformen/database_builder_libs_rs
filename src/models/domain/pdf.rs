use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct PdfExtractionData {
    pub properties: Option<PdfProperties>,
    pub bounding_boxes: Vec<BoundingBox>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BoundingBox {
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PdfProperties {
    pub page_count: Option<u32>,
    pub file_size: Option<u64>,
}
