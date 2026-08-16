#[cfg(not(test))]
#[path = "repository.rs"]
mod legacy_repository;
mod migrations;
mod record_repository;

#[cfg(not(test))]
pub(crate) use legacy_repository::read_legacy_v1_data;
pub use migrations::{
    initialize_connection, MigrationError, APPLICATION_ID, DATABASE_SCHEMA_VERSION,
};
pub use record_repository::{RecordRepository, RecordRepositoryTransaction};
