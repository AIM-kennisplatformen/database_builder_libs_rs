use qdrant_client::QdrantError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QdrantStoreError {
    #[error("failed to build Qdrant client")]
    BuildClient {
        #[source]
        source: Box<QdrantError>,
    },
    #[error("failed to check whether Qdrant collection `{collection}` exists")]
    CheckCollection {
        collection: String,
        #[source]
        source: Box<QdrantError>,
    },
    #[error("failed to create Qdrant collection `{collection}`")]
    CreateCollection {
        collection: String,
        #[source]
        source: Box<QdrantError>,
    },
    #[error("failed to delete Qdrant collection `{collection}`")]
    DeleteCollection {
        collection: String,
        #[source]
        source: Box<QdrantError>,
    },
    #[error("expected vector dimension {expected}, got {actual}")]
    VectorDimensionMismatch { expected: u64, actual: usize },
    #[error("failed to upsert points into Qdrant collection `{collection}`")]
    UpsertPoints {
        collection: String,
        #[source]
        source: Box<QdrantError>,
    },
    #[error("failed to query Qdrant collection `{collection}`")]
    Query {
        collection: String,
        #[source]
        source: Box<QdrantError>,
    },
}
