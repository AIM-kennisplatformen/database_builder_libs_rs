use serde::{Deserialize, Serialize};

use crate::models::{
    chunk::Chunk,
    entities::{Entity, document::Document},
    relations::Relation,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentWithChunks {
    pub document: Document,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub chunks: Vec<Chunk>,
}
