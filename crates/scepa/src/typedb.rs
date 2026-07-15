use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use rootcause::prelude::{Report, ResultExt};
use tokio::time::sleep;
use typedb_driver::{
    Addresses, Credentials, DriverOptions, DriverTlsConfig, TransactionType, TypeDBDriver,
    answer::QueryAnswer,
};

pub const DOMAIN_SCHEMA: &str = include_str!("../../../domain.tql");

const EXPORT_MAX_ATTEMPTS: usize = 5;
const EXPORT_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDbConfig {
    pub address: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub tls: bool,
    pub wipe_database: bool,
}

impl TypeDbConfig {
    pub fn new(
        address: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        tls: bool,
        wipe_database: bool,
    ) -> Self {
        Self {
            address: address.into(),
            database: database.into(),
            username: username.into(),
            password: password.into(),
            tls,
            wipe_database,
        }
    }
}

impl Default for TypeDbConfig {
    fn default() -> Self {
        Self::new("127.0.0.1:1729", "scepa", "admin", "password", false, false)
    }
}

#[derive(Default)]
pub struct Disconnected;

pub struct Connected {
    driver: Arc<TypeDBDriver>,
    database: String,
}

#[derive(Default)]
pub struct TypeDbDriver<State = Disconnected> {
    state: State,
}

impl TypeDbDriver<Disconnected> {
    pub async fn connect(self, config: &TypeDbConfig) -> Result<TypeDbDriver<Connected>, Report> {
        let addresses = Addresses::try_from_address_str(&config.address)
            .context("failed to parse TypeDB address")?;
        let tls = if config.tls {
            DriverTlsConfig::enabled_with_native_root_ca()
        } else {
            DriverTlsConfig::disabled()
        };
        let driver = TypeDBDriver::new(
            addresses,
            Credentials::new(&config.username, &config.password),
            DriverOptions::new(tls),
        )
        .await
        .context("failed to connect to TypeDB")?;

        let database_exists = driver
            .databases()
            .contains(&config.database)
            .await
            .context("failed to check whether the TypeDB database exists")?;

        if config.wipe_database && database_exists {
            driver
                .databases()
                .get(&config.database)
                .await
                .context("failed to open the TypeDB database for recreation")?
                .delete()
                .await
                .context("failed to delete the TypeDB database")?;
        }

        if !database_exists || config.wipe_database {
            driver
                .databases()
                .create(&config.database)
                .await
                .context("failed to create the TypeDB database")?;
        }

        // TypeDB define queries are idempotent, so apply the current schema to
        // existing databases as well as newly created ones. This lets additive
        // schema changes (such as new role implementations) take effect
        // without requiring a destructive database wipe.
        apply_schema(&driver, &config.database).await?;

        Ok(TypeDbDriver {
            state: Connected {
                driver: Arc::new(driver),
                database: config.database.clone(),
            },
        })
    }
}

impl Clone for TypeDbDriver<Connected> {
    fn clone(&self) -> Self {
        Self {
            state: Connected {
                driver: Arc::clone(&self.state.driver),
                database: self.state.database.clone(),
            },
        }
    }
}

impl TypeDbDriver<Connected> {
    pub async fn count_matches(&self, query: &str) -> Result<usize, Report> {
        let transaction = self
            .state
            .driver
            .transaction(&self.state.database, TransactionType::Read)
            .await
            .context("failed to open a TypeDB read transaction")?;
        let answer = transaction
            .query(query)
            .await
            .context(format!("failed to execute TypeDB count query `{query}`"))?;
        let count = match answer {
            QueryAnswer::Ok(_) => 0,
            QueryAnswer::ConceptRowStream(_, mut rows) => {
                let mut count = 0;
                while let Some(row) = rows.next().await {
                    row.context(format!("failed to read TypeDB count query `{query}`"))?;
                    count += 1;
                }
                count
            }
            QueryAnswer::ConceptDocumentStream(_, mut documents) => {
                let mut count = 0;
                while let Some(document) = documents.next().await {
                    document.context(format!("failed to read TypeDB count query `{query}`"))?;
                    count += 1;
                }
                count
            }
        };
        transaction
            .close()
            .await
            .context("failed to close the TypeDB read transaction")?;
        Ok(count)
    }

    pub async fn export_queries(&self, queries: Vec<String>) -> Result<(), Report> {
        for attempt in 1..=EXPORT_MAX_ATTEMPTS {
            match self.export_queries_once(&queries).await {
                Ok(()) => return Ok(()),
                Err(error)
                    if attempt < EXPORT_MAX_ATTEMPTS && is_retryable_export_error(&error) =>
                {
                    let retry_delay = export_retry_delay(attempt);
                    tracing::warn!(
                        attempt,
                        max_attempts = EXPORT_MAX_ATTEMPTS,
                        retry_delay_ms = retry_delay.as_millis(),
                        "TypeDB export commit conflicted; retrying transaction"
                    );
                    sleep(retry_delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("export attempts always return or fail")
    }

    async fn export_queries_once(&self, queries: &[String]) -> Result<(), Report> {
        let transaction = self
            .state
            .driver
            .transaction(&self.state.database, TransactionType::Write)
            .await
            .context("failed to open a TypeDB write transaction")?;

        let query_started = Instant::now();
        for query in queries {
            let answer = transaction
                .query(&query)
                .await
                .context(format!("failed to execute TypeDB query `{query}`"))?;
            drain_answer(answer, query).await?;
        }
        tracing::info!(
            query_count = queries.len(),
            elapsed_ms = query_started.elapsed().as_millis(),
            "executed TypeDB query batch"
        );

        transaction
            .commit()
            .await
            .context("failed to commit the TypeDB export transaction")?;
        Ok(())
    }

    pub fn disconnect(self) -> Result<TypeDbDriver<Disconnected>, Report> {
        self.state
            .driver
            .force_close()
            .context("failed to close the TypeDB driver")?;
        Ok(TypeDbDriver::default())
    }
}

fn export_retry_delay(attempt: usize) -> Duration {
    EXPORT_RETRY_BASE_DELAY.saturating_mul(2u32.saturating_pow((attempt - 1) as u32))
}

fn is_retryable_export_error(error: &Report) -> bool {
    error.iter_reports().any(|report| {
        report
            .downcast_current_context::<typedb_driver::Error>()
            .is_some_and(|error| error.code() == "STC2")
    })
}

async fn apply_schema(driver: &TypeDBDriver, database: &str) -> Result<(), Report> {
    let transaction = driver
        .transaction(database, TransactionType::Schema)
        .await
        .context("failed to open a TypeDB schema transaction")?;
    transaction
        .query(DOMAIN_SCHEMA)
        .await
        .context("failed to apply the TypeDB domain schema")?;
    transaction
        .commit()
        .await
        .context("failed to commit the TypeDB domain schema")?;
    Ok(())
}

async fn drain_answer(answer: QueryAnswer, query: &str) -> Result<(), Report> {
    match answer {
        QueryAnswer::Ok(_) => {}
        QueryAnswer::ConceptRowStream(_, mut rows) => {
            while let Some(row) = rows.next().await {
                row.context(format!("failed to read TypeDB query result for `{query}`"))?;
            }
        }
        QueryAnswer::ConceptDocumentStream(_, mut documents) => {
            while let Some(document) = documents.next().await {
                document.context(format!("failed to read TypeDB query result for `{query}`"))?;
            }
        }
    }
    Ok(())
}
