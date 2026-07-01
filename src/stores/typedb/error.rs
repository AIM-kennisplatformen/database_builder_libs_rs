use thiserror::Error;
use typedb_driver::Error as TypedbDriverError;

#[derive(Debug, Error)]
pub enum TypedbStoreError {
    #[error("invalid TypeDB address `{address}`")]
    InvalidAddress {
        address: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to connect to TypeDB at `{address}`")]
    Connect {
        address: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to check whether TypeDB database `{database}` exists")]
    CheckDatabase {
        database: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to create TypeDB database `{database}`")]
    CreateDatabase {
        database: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to open TypeDB database `{database}`")]
    OpenDatabase {
        database: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to open schema transaction for TypeDB database `{database}`")]
    OpenSchemaTransaction {
        database: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to apply schema to TypeDB database `{database}`")]
    ApplySchema {
        database: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to commit schema transaction for TypeDB database `{database}`")]
    CommitSchema {
        database: String,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to open {transaction_type} TypeDB transaction for database `{database}`")]
    OpenTransaction {
        database: String,
        transaction_type: &'static str,
        #[source]
        source: Box<TypedbDriverError>,
    },
    #[error("failed to close TypeDB driver")]
    CloseDriver {
        #[source]
        source: Box<TypedbDriverError>,
    },
}
