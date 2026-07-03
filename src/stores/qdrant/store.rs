use core::future::Future;

use anyhow::{Context, Result};
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        CreateCollectionBuilder, PointId, PointStruct, PointsOperationResponse, QueryPointsBuilder,
        QueryResponse, UpsertPointsBuilder, VectorParamsBuilder,
    },
};

use crate::stores::{
    connection::{Connect, Disconnect},
    qdrant::{config::QdrantConfig, error::QdrantStoreError},
};

pub struct QdrantConnected {
    client: Qdrant,
    collection: String,
    vector_dimension: u64,
}

#[derive(Default)]
pub struct QdrantDisconnected;

#[derive(Default)]
pub struct QdrantStore<S = QdrantDisconnected> {
    state: S,
}

impl QdrantStore<QdrantDisconnected> {
    pub fn new() -> Self {
        Self {
            state: QdrantDisconnected,
        }
    }

    pub async fn connect(self, config: &QdrantConfig) -> Result<QdrantStore<QdrantConnected>> {
        self.connect_inner(config).await
    }

    async fn connect_inner(self, config: &QdrantConfig) -> Result<QdrantStore<QdrantConnected>> {
        let client = Qdrant::from_url(&config.url)
            .api_key(config.api_key.as_str())
            .build()
            .map_err(|source| QdrantStoreError::BuildClient {
                source: Box::new(source),
            })
            .with_context(|| format!("building Qdrant client for `{}`", config.url))?;

        Self::ensure_collection(&client, config)
            .await
            .with_context(|| format!("preparing Qdrant collection `{}`", config.collection))?;

        Ok(QdrantStore {
            state: QdrantConnected {
                client,
                collection: config.collection.clone(),
                vector_dimension: config.vector_dimension,
            },
        })
    }

    async fn ensure_collection(client: &Qdrant, config: &QdrantConfig) -> Result<()> {
        let exists = client
            .collection_exists(&config.collection)
            .await
            .map_err(|source| QdrantStoreError::CheckCollection {
                collection: config.collection.clone(),
                source: Box::new(source),
            })
            .with_context(|| {
                format!(
                    "checking whether Qdrant collection `{}` exists",
                    config.collection
                )
            })?;

        if exists {
            return Ok(());
        }

        client
            .create_collection(
                CreateCollectionBuilder::new(&config.collection).vectors_config(
                    VectorParamsBuilder::new(config.vector_dimension, config.distance),
                ),
            )
            .await
            .map_err(|source| QdrantStoreError::CreateCollection {
                collection: config.collection.clone(),
                source: Box::new(source),
            })
            .with_context(|| format!("creating Qdrant collection `{}`", config.collection))?;

        Ok(())
    }
}

impl Connect for QdrantStore<QdrantDisconnected> {
    type Config = QdrantConfig;
    type Connected = QdrantStore<QdrantConnected>;
    type Error = anyhow::Error;

    fn connect(
        self,
        config: &Self::Config,
    ) -> impl Future<Output = Result<Self::Connected, Self::Error>> + Send {
        self.connect_inner(config)
    }
}

impl QdrantStore<QdrantConnected> {
    pub async fn upsert_point(
        &self,
        id: impl Into<PointId>,
        vector: Vec<f32>,
        payload: impl Into<Payload>,
    ) -> Result<PointsOperationResponse> {
        self.validate_vector(&vector).with_context(|| {
            format!(
                "validating point vector for Qdrant collection `{}`",
                self.state.collection
            )
        })?;

        self.upsert_points(vec![PointStruct::new(id, vector, payload)])
            .await
            .with_context(|| {
                format!(
                    "upserting point into Qdrant collection `{}`",
                    self.state.collection
                )
            })
    }

    pub async fn upsert_points(
        &self,
        points: impl Into<Vec<PointStruct>>,
    ) -> Result<PointsOperationResponse> {
        self.state
            .client
            .upsert_points(UpsertPointsBuilder::new(&self.state.collection, points).wait(true))
            .await
            .map_err(|source| QdrantStoreError::UpsertPoints {
                collection: self.state.collection.clone(),
                source: Box::new(source),
            })
            .with_context(|| {
                format!(
                    "upserting points into Qdrant collection `{}`",
                    self.state.collection
                )
            })
    }

    pub async fn query(&self, vector: Vec<f32>, limit: u64) -> Result<QueryResponse> {
        self.validate_vector(&vector).with_context(|| {
            format!(
                "validating query vector for Qdrant collection `{}`",
                self.state.collection
            )
        })?;

        self.state
            .client
            .query(
                QueryPointsBuilder::new(&self.state.collection)
                    .query(vector)
                    .limit(limit)
                    .with_payload(true),
            )
            .await
            .map_err(|source| QdrantStoreError::Query {
                collection: self.state.collection.clone(),
                source: Box::new(source),
            })
            .with_context(|| format!("querying Qdrant collection `{}`", self.state.collection))
    }

    fn validate_vector(&self, vector: &[f32]) -> Result<()> {
        if vector.len() as u64 != self.state.vector_dimension {
            return Err(QdrantStoreError::VectorDimensionMismatch {
                expected: self.state.vector_dimension,
                actual: vector.len(),
            }
            .into());
        }

        Ok(())
    }
}

impl Disconnect for QdrantStore<QdrantConnected> {
    type Output = QdrantStore<QdrantDisconnected>;

    fn disconnect(self) -> Self::Output {
        QdrantStore::new()
    }
}
