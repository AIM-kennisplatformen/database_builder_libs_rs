use typedb_driver::{Credentials, DriverOptions};

use crate::stores::typedb::error::TypedbStoreError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedbConfig {
    pub address: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub tls: bool,
    pub wipe_database: bool,
    pub schema: String,
}

impl TypedbConfig {
    pub fn new(
        address: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        tls: bool,
        wipe_database: bool,
        schema: impl Into<String>,
    ) -> Self {
        Self {
            address: address.into(),
            database: database.into(),
            username: username.into(),
            password: password.into(),
            tls,
            wipe_database,
            schema: schema.into(),
        }
    }

    pub fn credentials(&self) -> Credentials {
        Credentials::new(&self.username, &self.password)
    }

    /// No CA path is configurable here, so TLS (when enabled) always uses
    /// the platform's native root certificates.
    pub fn driver_options(&self) -> Result<DriverOptions, TypedbStoreError> {
        DriverOptions::new(self.tls, None).map_err(|source| TypedbStoreError::DriverOptions {
            source: Box::new(source),
        })
    }
}
