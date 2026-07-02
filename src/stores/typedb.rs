pub mod config;
pub mod error;
pub mod store;

pub const DOMAIN_SCHEMA: &str = include_str!("../../schemas/typedb/domain.tql");
