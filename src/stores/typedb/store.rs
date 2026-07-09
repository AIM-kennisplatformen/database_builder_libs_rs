use core::future::Future;
use std::sync::Arc;

use anyhow::{Context, Result};
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

    pub async fn connect(self, config: &TypedbConfig) -> Result<TypedbStore<TypedbConnected>> {
        self.connect_inner(config).await
    }

    async fn connect_inner(self, config: &TypedbConfig) -> Result<TypedbStore<TypedbConnected>> {
        let driver_options = config
            .driver_options()
            .context("building TypeDB driver options")?;

        let driver = TypeDBDriver::new(&config.address, config.credentials(), driver_options)
            .await
            .map_err(|source| TypedbStoreError::Connect {
                address: config.address.clone(),
                source: Box::new(source),
            })
            .with_context(|| format!("connecting to TypeDB at `{}`", config.address))?;

        let database = if config.wipe_database {
            let database = Self::recreate_database(&driver, &config.database)
                .await
                .with_context(|| format!("recreating TypeDB database `{}`", config.database))?;

            Self::ensure_schema(&driver, database.name(), &config.schema)
                .await
                .with_context(|| {
                    format!(
                        "ensuring TypeDB schema is applied to database `{}`",
                        database.name()
                    )
                })?;

            database
        } else {
            Self::get_or_create_database(&driver, &config.database, &config.schema)
                .await
                .with_context(|| format!("preparing TypeDB database `{}`", config.database))?
        };

        Ok(TypedbStore {
            state: TypedbConnected { driver, database },
        })
    }

    async fn get_or_create_database(
        driver: &TypeDBDriver,
        database: &str,
        schema: &str,
    ) -> Result<Arc<Database>> {
        let databases = driver.databases();

        let exists = databases
            .contains(database)
            .await
            .map_err(|source| TypedbStoreError::CheckDatabase {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| format!("checking whether TypeDB database `{database}` exists"))?;

        if !exists {
            databases
                .create(database)
                .await
                .map_err(|source| TypedbStoreError::CreateDatabase {
                    database: database.to_owned(),
                    source: Box::new(source),
                })
                .with_context(|| format!("creating TypeDB database `{database}`"))?;
        }

        let created_database = databases
            .get(database)
            .await
            .map_err(|source| TypedbStoreError::OpenDatabase {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| format!("opening TypeDB database `{database}`"))?;

        // Only a brand-new database needs its schema defined: applying the
        // same `define` schema again on every connect against an existing
        // database is unnecessary and would fail once it already owns those
        // types.
        if !exists {
            Self::ensure_schema(driver, created_database.name(), schema)
                .await
                .with_context(|| {
                    format!(
                        "ensuring TypeDB schema is applied to database `{}`",
                        created_database.name()
                    )
                })?;
        }

        Ok(created_database)
    }

    async fn recreate_database(driver: &TypeDBDriver, database: &str) -> Result<Arc<Database>> {
        let databases = driver.databases();

        let exists = databases
            .contains(database)
            .await
            .map_err(|source| TypedbStoreError::CheckDatabase {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| format!("checking whether TypeDB database `{database}` exists"))?;

        if exists {
            let existing_database = databases
                .get(database)
                .await
                .map_err(|source| TypedbStoreError::OpenDatabase {
                    database: database.to_owned(),
                    source: Box::new(source),
                })
                .with_context(|| format!("opening TypeDB database `{database}` before deletion"))?;

            existing_database
                .delete()
                .await
                .map_err(|source| TypedbStoreError::DeleteDatabase {
                    database: database.to_owned(),
                    source: Box::new(source),
                })
                .with_context(|| format!("deleting TypeDB database `{database}`"))?;
        }

        databases
            .create(database)
            .await
            .map_err(|source| TypedbStoreError::CreateDatabase {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| format!("creating TypeDB database `{database}`"))?;

        databases
            .get(database)
            .await
            .map_err(|source| TypedbStoreError::OpenDatabase {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| format!("opening TypeDB database `{database}`"))
    }

    async fn ensure_schema(driver: &TypeDBDriver, database: &str, schema: &str) -> Result<()> {
        let transaction = driver
            .transaction(database, TransactionType::Schema)
            .await
            .map_err(|source| TypedbStoreError::OpenSchemaTransaction {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| {
                format!("opening TypeDB schema transaction for database `{database}`")
            })?;

        transaction
            .query(schema)
            .await
            .map_err(|source| TypedbStoreError::ApplySchema {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| format!("applying TypeDB schema to database `{database}`"))?;

        transaction
            .commit()
            .await
            .map_err(|source| TypedbStoreError::CommitSchema {
                database: database.to_owned(),
                source: Box::new(source),
            })
            .with_context(|| {
                format!("committing TypeDB schema transaction for database `{database}`")
            })?;

        Ok(())
    }
}

impl Connect for TypedbStore<TypedbDisconnected> {
    type Config = TypedbConfig;
    type Connected = TypedbStore<TypedbConnected>;
    type Error = anyhow::Error;

    fn connect(
        self,
        config: &Self::Config,
    ) -> impl Future<Output = Result<Self::Connected, Self::Error>> + Send {
        self.connect_inner(config)
    }
}

impl TypedbStore<TypedbConnected> {
    pub async fn transaction(&self, transaction_type: TransactionType) -> Result<Transaction> {
        let database = self.state.database.name();
        let transaction_type_name = Self::transaction_type_name(transaction_type);

        self.state
            .driver
            .transaction(database, transaction_type)
            .await
            .map_err(|source| TypedbStoreError::OpenTransaction {
                database: database.to_owned(),
                transaction_type: transaction_type_name,
                source: Box::new(source),
            })
            .with_context(|| {
                format!(
                    "opening {transaction_type_name} TypeDB transaction for database `{database}`"
                )
            })
    }

    pub async fn read_transaction(&self) -> Result<Transaction> {
        self.transaction(TransactionType::Read)
            .await
            .context("opening read TypeDB transaction")
    }

    pub async fn write_transaction(&self) -> Result<Transaction> {
        self.transaction(TransactionType::Write)
            .await
            .context("opening write TypeDB transaction")
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
    type Output = Result<TypedbStore<TypedbDisconnected>>;

    fn disconnect(self) -> Self::Output {
        self.state
            .driver
            .force_close()
            .map_err(|source| TypedbStoreError::CloseDriver {
                source: Box::new(source),
            })
            .context("closing TypeDB driver")?;

        Ok(TypedbStore::new())
    }
}
