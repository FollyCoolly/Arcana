use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime};

pub const APPLICATION_ID: i32 = 0x4152_4341;
pub const DATABASE_SCHEMA_VERSION: i64 = 2;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: include_str!("migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "record_only",
        sql: include_str!("migrations/0002_record_only.sql"),
    },
];

#[derive(Debug)]
pub enum MigrationError {
    Sqlite(rusqlite::Error),
    UnknownDatabase { application_id: i32 },
    UnsupportedSqliteVersion(String),
    MissingMigrationTable,
    UnknownMigration { version: i64 },
    MigrationGap { expected: i64, actual: i64 },
    MigrationChanged { version: i64, name: String },
    ForeignKeyCheckFailed(String),
    IntegrityCheckFailed(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "SQLite migration failed: {error}"),
            Self::UnknownDatabase { application_id } => write!(
                f,
                "database is not an Arcana database (application_id={application_id:#x})"
            ),
            Self::UnsupportedSqliteVersion(version) => write!(
                f,
                "SQLite {version} is too old; Arcana requires SQLite 3.43.0 or newer"
            ),
            Self::MissingMigrationTable => {
                write!(f, "Arcana database is missing schema_migrations")
            }
            Self::UnknownMigration { version } => {
                write!(
                    f,
                    "database contains unsupported migration version {version}"
                )
            }
            Self::MigrationGap { expected, actual } => write!(
                f,
                "database migration history has a gap: expected {expected}, found {actual}"
            ),
            Self::MigrationChanged { version, name } => write!(
                f,
                "installed migration {version} ({name}) does not match this application"
            ),
            Self::ForeignKeyCheckFailed(result) => {
                write!(f, "SQLite foreign_key_check failed: {result}")
            }
            Self::IntegrityCheckFailed(result) => {
                write!(f, "SQLite integrity_check failed: {result}")
            }
        }
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for MigrationError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

pub fn initialize_connection(connection: &mut Connection) -> Result<(), MigrationError> {
    configure_connection(connection)?;
    ensure_supported_sqlite_version(connection)?;

    let application_id: i32 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let is_empty = database_is_empty(connection)?;
    match (application_id, is_empty) {
        (0, true) => {}
        (APPLICATION_ID, _) => ensure_migration_table_exists(connection)?,
        (application_id, _) => {
            return Err(MigrationError::UnknownDatabase { application_id });
        }
    }

    let applied = if is_empty {
        BTreeMap::new()
    } else {
        load_applied_migrations(connection)?
    };
    validate_applied_migrations(&applied)?;
    apply_pending_migrations(connection, &applied)?;

    let final_application_id: i32 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if final_application_id != APPLICATION_ID {
        return Err(MigrationError::UnknownDatabase {
            application_id: final_application_id,
        });
    }

    ensure_foreign_keys_clean(connection)?;
    ensure_integrity(connection)?;
    Ok(())
}

fn ensure_supported_sqlite_version(connection: &Connection) -> Result<(), MigrationError> {
    let version: String = connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))?;
    let parts: Vec<_> = version.split('.').take(3).map(str::parse::<u32>).collect();
    let supported = matches!(parts.as_slice(), [Ok(major), Ok(minor), Ok(patch)] if (*major, *minor, *patch) >= (3, 43, 0));
    if !supported {
        return Err(MigrationError::UnsupportedSqliteVersion(version));
    }
    Ok(())
}

fn configure_connection(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

fn database_is_empty(connection: &Connection) -> Result<bool, rusqlite::Error> {
    let object_count: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    Ok(object_count == 0)
}

fn ensure_migration_table_exists(connection: &Connection) -> Result<(), MigrationError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    if !exists {
        return Err(MigrationError::MissingMigrationTable);
    }
    Ok(())
}

fn load_applied_migrations(
    connection: &Connection,
) -> Result<BTreeMap<i64, (String, String)>, MigrationError> {
    let mut statement = connection
        .prepare("SELECT version, name, checksum FROM schema_migrations ORDER BY version")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut applied = BTreeMap::new();
    for row in rows {
        let (version, name, checksum) = row?;
        applied.insert(version, (name, checksum));
    }
    Ok(applied)
}

fn validate_applied_migrations(
    applied: &BTreeMap<i64, (String, String)>,
) -> Result<(), MigrationError> {
    for (index, (&version, (name, checksum))) in applied.iter().enumerate() {
        let expected_version = index as i64 + 1;
        if version != expected_version {
            return Err(MigrationError::MigrationGap {
                expected: expected_version,
                actual: version,
            });
        }
        let Some(migration) = MIGRATIONS
            .iter()
            .find(|migration| migration.version == version)
        else {
            return Err(MigrationError::UnknownMigration { version });
        };
        if name != migration.name || checksum != &migration_checksum(migration.sql) {
            return Err(MigrationError::MigrationChanged {
                version,
                name: name.clone(),
            });
        }
    }
    Ok(())
}

fn apply_pending_migrations(
    connection: &mut Connection,
    applied: &BTreeMap<i64, (String, String)>,
) -> Result<(), MigrationError> {
    for migration in MIGRATIONS {
        if applied.contains_key(&migration.version) {
            continue;
        }
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(migration.sql)?;
        ensure_foreign_keys_clean(&transaction)?;
        ensure_integrity(&transaction)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, name, checksum, applied_at) VALUES (?1, ?2, ?3, ?4)",
            (
                migration.version,
                migration.name,
                migration_checksum(migration.sql),
                now_rfc3339(),
            ),
        )?;
        transaction.commit()?;
    }
    Ok(())
}

fn ensure_foreign_keys_clean(connection: &Connection) -> Result<(), MigrationError> {
    let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
    let mut rows = statement.query([])?;
    if let Some(row) = rows.next()? {
        let table: String = row.get(0)?;
        let row_id: Option<i64> = row.get(1)?;
        let parent: String = row.get(2)?;
        return Err(MigrationError::ForeignKeyCheckFailed(format!(
            "table={table}, rowid={row_id:?}, parent={parent}"
        )));
    }
    Ok(())
}

fn ensure_integrity(connection: &Connection) -> Result<(), MigrationError> {
    let integrity: String =
        connection.query_row("PRAGMA integrity_check(1)", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(MigrationError::IntegrityCheckFailed(integrity));
    }
    Ok(())
}

fn migration_checksum(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_rfc3339() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_and_reopens_arcana_database() {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_connection(&mut connection).unwrap();
        initialize_connection(&mut connection).unwrap();

        let application_id: i32 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .unwrap();
        let version: i64 = connection
            .query_row("SELECT max(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(application_id, APPLICATION_ID);
        assert_eq!(version, DATABASE_SCHEMA_VERSION);
    }

    #[test]
    fn refuses_an_unrelated_database() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute("CREATE TABLE foreign_data(id INTEGER)", [])
            .unwrap();
        let error = initialize_connection(&mut connection).unwrap_err();
        assert!(matches!(error, MigrationError::UnknownDatabase { .. }));
    }

    #[test]
    fn detects_modified_migration_history() {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_connection(&mut connection).unwrap();
        connection
            .execute(
                "UPDATE schema_migrations SET checksum = 'changed' WHERE version = 1",
                [],
            )
            .unwrap();
        let error = initialize_connection(&mut connection).unwrap_err();
        assert!(matches!(error, MigrationError::MigrationChanged { .. }));
    }
}
