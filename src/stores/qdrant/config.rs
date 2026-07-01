use qdrant_client::qdrant::Distance;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
    pub vector_dimension: u64,
    pub distance: Distance,
    pub api_key: String,
}

impl QdrantConfig {
    pub fn new(
        url: impl Into<String>,
        collection: impl Into<String>,
        vector_dimension: u64,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            collection: collection.into(),
            vector_dimension,
            distance: Distance::Cosine,
            api_key: api_key.into(),
        }
    }
}
