use serde::{Deserialize, Serialize};

use crate::models::{Document, Entity, Relation, chunk::Chunk};

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentWithChunks {
    pub document: Document,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub chunks: Vec<Chunk>,
}
