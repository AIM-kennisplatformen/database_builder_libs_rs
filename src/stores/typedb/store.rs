use core::future::Future;
use std::sync::Arc;

use typedb_driver::{Database, Transaction, TransactionType, TypeDBDriver};

use crate::stores::{
    connection::{Connect, Disconnect},
    typedb::{config::TypedbConfig, error::TypedbStoreError},
};

pub struct TypedbConnected {
    driver: TypeDBDriver,
    database: Arc<Database>,
}

#[derive(Default)]
pub struct TypedbDisconnected;

#[derive(Default)]
pub struct TypedbStore<S = TypedbDisconnected> {
    state: S,
}

impl TypedbStore<TypedbDisconnected> {
    pub fn new() -> Self {
        Self {
            state: TypedbDisconnected,
        }
    }

    pub async fn connect(
        self,
        config: &TypedbConfig,
    ) -> Result<TypedbStore<TypedbConnected>, TypedbStoreError> {
        self.connect_inner(config).await
    }

    async fn connect_inner(
        self,
        config: &TypedbConfig,
    ) -> Result<TypedbStore<TypedbConnected>, TypedbStoreError> {
        let addresses = match config.addresses() {
            Ok(addresses) => addresses,
            Err(error) => return Err(error),
        };

        let driver =
            match TypeDBDriver::new(addresses, config.credentials(), config.driver_options()).await
            {
                Ok(driver) => driver,
                Err(source) => {
                    return Err(TypedbStoreError::Connect {
                        address: config.address.clone(),
                        source: Box::new(source),
                    });
                }
            };

        let database = match Self::get_or_create_database(&driver, &config.database).await {
            Ok(database) => database,
            Err(error) => return Err(error),
        };

        match Self::ensure_schema(&driver, database.name(), &config.schema).await {
            Ok(()) => {}
            Err(error) => return Err(error),
        }

        Ok(TypedbStore {
            state: TypedbConnected { driver, database },
        })
    }

    async fn get_or_create_database(
        driver: &TypeDBDriver,
        database: &str,
    ) -> Result<Arc<Database>, TypedbStoreError> {
        let databases = driver.databases();

        match databases.contains(database).await {
            Ok(true) => {}
            Ok(false) => match databases.create(database).await {
                Ok(()) => {}
                Err(source) => {
                    return Err(TypedbStoreError::CreateDatabase {
                        database: database.to_owned(),
                        source: Box::new(source),
                    });
                }
            },
            Err(source) => {
                return Err(TypedbStoreError::CheckDatabase {
                    database: database.to_owned(),
                    source: Box::new(source),
                });
            }
        }

        match databases.get(database).await {
            Ok(database) => Ok(database),
            Err(source) => Err(TypedbStoreError::OpenDatabase {
                database: database.to_owned(),
                source: Box::new(source),
            }),
        }
    }

    async fn ensure_schema(
        driver: &TypeDBDriver,
        database: &str,
        schema: &str,
    ) -> Result<(), TypedbStoreError> {
        let transaction = match driver.transaction(database, TransactionType::Schema).await {
            Ok(transaction) => transaction,
            Err(source) => {
                return Err(TypedbStoreError::OpenSchemaTransaction {
                    database: database.to_owned(),
                    source: Box::new(source),
                });
            }
        };

        match transaction.query(schema).await {
            Ok(_) => {}
            Err(source) => {
                return Err(TypedbStoreError::ApplySchema {
                    database: database.to_owned(),
                    source: Box::new(source),
                });
            }
        }

        match transaction.commit().await {
            Ok(()) => Ok(()),
            Err(source) => Err(TypedbStoreError::CommitSchema {
                database: database.to_owned(),
                source: Box::new(source),
            }),
        }
    }
}

impl Connect for TypedbStore<TypedbDisconnected> {
    type Config = TypedbConfig;
    type Connected = TypedbStore<TypedbConnected>;
    type Error = TypedbStoreError;

    fn connect(
        self,
        config: &Self::Config,
    ) -> impl Future<Output = Result<Self::Connected, Self::Error>> + Send {
        self.connect_inner(config)
    }
}

impl TypedbStore<TypedbConnected> {
    pub async fn transaction(
        &self,
        transaction_type: TransactionType,
    ) -> Result<Transaction, TypedbStoreError> {
        let database = self.state.database.name();
        let transaction_type_name = Self::transaction_type_name(transaction_type);

        match self
            .state
            .driver
            .transaction(database, transaction_type)
            .await
        {
            Ok(transaction) => Ok(transaction),
            Err(source) => Err(TypedbStoreError::OpenTransaction {
                database: database.to_owned(),
                transaction_type: transaction_type_name,
                source: Box::new(source),
            }),
        }
    }

    pub async fn read_transaction(&self) -> Result<Transaction, TypedbStoreError> {
        self.transaction(TransactionType::Read).await
    }

    pub async fn write_transaction(&self) -> Result<Transaction, TypedbStoreError> {
        self.transaction(TransactionType::Write).await
    }

    fn transaction_type_name(transaction_type: TransactionType) -> &'static str {
        match transaction_type {
            TransactionType::Read => "read",
            TransactionType::Write => "write",
            TransactionType::Schema => "schema",
        }
    }
}

impl Disconnect for TypedbStore<TypedbConnected> {
    type Output = Result<TypedbStore<TypedbDisconnected>, TypedbStoreError>;

    fn disconnect(self) -> Self::Output {
        match self.state.driver.force_close() {
            Ok(()) => {}
            Err(source) => {
                return Err(TypedbStoreError::CloseDriver {
                    source: Box::new(source),
                });
            }
        }

        Ok(TypedbStore::new())
    }
}
