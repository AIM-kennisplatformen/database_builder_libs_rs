use core::future::Future;

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

    pub async fn connect(
        self,
        config: &QdrantConfig,
    ) -> Result<QdrantStore<QdrantConnected>, QdrantStoreError> {
        self.connect_inner(config).await
    }

    async fn connect_inner(
        self,
        config: &QdrantConfig,
    ) -> Result<QdrantStore<QdrantConnected>, QdrantStoreError> {
        let client = match Qdrant::from_url(&config.url)
            .api_key(config.api_key.as_str())
            .build()
        {
            Ok(client) => client,
            Err(source) => {
                return Err(QdrantStoreError::BuildClient {
                    source: Box::new(source),
                });
            }
        };

        match Self::ensure_collection(&client, config).await {
            Ok(()) => {}
            Err(error) => return Err(error),
        }

        Ok(QdrantStore {
            state: QdrantConnected {
                client,
                collection: config.collection.clone(),
                vector_dimension: config.vector_dimension,
            },
        })
    }

    async fn ensure_collection(
        client: &Qdrant,
        config: &QdrantConfig,
    ) -> Result<(), QdrantStoreError> {
        match client.collection_exists(&config.collection).await {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(source) => {
                return Err(QdrantStoreError::CheckCollection {
                    collection: config.collection.clone(),
                    source: Box::new(source),
                });
            }
        };

        match client
            .create_collection(
                CreateCollectionBuilder::new(&config.collection).vectors_config(
                    VectorParamsBuilder::new(config.vector_dimension, config.distance),
                ),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(source) => Err(QdrantStoreError::CreateCollection {
                collection: config.collection.clone(),
                source: Box::new(source),
            }),
        }
    }
}

impl Connect for QdrantStore<QdrantDisconnected> {
    type Config = QdrantConfig;
    type Connected = QdrantStore<QdrantConnected>;
    type Error = QdrantStoreError;

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
    ) -> Result<PointsOperationResponse, QdrantStoreError> {
        match self.validate_vector(&vector) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }

        match self
            .upsert_points(vec![PointStruct::new(id, vector, payload)])
            .await
        {
            Ok(response) => Ok(response),
            Err(error) => Err(error),
        }
    }

    pub async fn upsert_points(
        &self,
        points: impl Into<Vec<PointStruct>>,
    ) -> Result<PointsOperationResponse, QdrantStoreError> {
        match self
            .state
            .client
            .upsert_points(UpsertPointsBuilder::new(&self.state.collection, points).wait(true))
            .await
        {
            Ok(response) => Ok(response),
            Err(source) => Err(QdrantStoreError::UpsertPoints {
                collection: self.state.collection.clone(),
                source: Box::new(source),
            }),
        }
    }

    pub async fn query(
        &self,
        vector: Vec<f32>,
        limit: u64,
    ) -> Result<QueryResponse, QdrantStoreError> {
        match self.validate_vector(&vector) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }

        match self
            .state
            .client
            .query(
                QueryPointsBuilder::new(&self.state.collection)
                    .query(vector)
                    .limit(limit)
                    .with_payload(true),
            )
            .await
        {
            Ok(response) => Ok(response),
            Err(source) => Err(QdrantStoreError::Query {
                collection: self.state.collection.clone(),
                source: Box::new(source),
            }),
        }
    }

    fn validate_vector(&self, vector: &[f32]) -> Result<(), QdrantStoreError> {
        if vector.len() as u64 != self.state.vector_dimension {
            return Err(QdrantStoreError::VectorDimensionMismatch {
                expected: self.state.vector_dimension,
                actual: vector.len(),
            });
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
