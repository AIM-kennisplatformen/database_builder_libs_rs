use typedb_driver::{Addresses, Credentials, DriverOptions, DriverTlsConfig};

use crate::stores::typedb::error::TypedbStoreError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedbConfig {
    pub address: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub tls: bool,
    pub schema: String,
}

impl TypedbConfig {
    pub fn new(
        address: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        tls: bool,
        schema: impl Into<String>,
    ) -> Self {
        Self {
            address: address.into(),
            database: database.into(),
            username: username.into(),
            password: password.into(),
            tls,
            schema: schema.into(),
        }
    }

    pub fn addresses(&self) -> Result<Addresses, TypedbStoreError> {
        match Addresses::try_from_address_str(&self.address) {
            Ok(addresses) => Ok(addresses),
            Err(source) => Err(TypedbStoreError::InvalidAddress {
                address: self.address.clone(),
                source: Box::new(source),
            }),
        }
    }

    pub fn credentials(&self) -> Credentials {
        Credentials::new(&self.username, &self.password)
    }

    pub fn driver_options(&self) -> DriverOptions {
        let tls_config = if self.tls {
            DriverTlsConfig::default()
        } else {
            DriverTlsConfig::disabled()
        };

        DriverOptions::new(tls_config)
    }
}
