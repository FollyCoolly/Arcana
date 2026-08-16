use super::migrations::initialize_connection;
use crate::domain::{
    split_record_definition_id, CollectionItem, CollectionRecord, EventEntry, EventRecord, Record,
    RecordFile, RecordKind, RepositoryError, RepositoryErrorCode, RepositoryResult, ScalarRecord,
    Validate,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// SQLite adapter for mutable Record values only. Pack content and other
/// semantic user state are deliberately owned by the JSON repository.
pub struct RecordRepository {
    connection: Connection,
}

impl RecordRepository {
    pub fn open(path: impl AsRef<Path>) -> RepositoryResult<Self> {
        let connection = Connection::open(path).map_err(map_sqlite_error)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> RepositoryResult<Self> {
        let connection = Connection::open_in_memory().map_err(map_sqlite_error)?;
        Self::from_connection(connection)
    }

    fn from_connection(mut connection: Connection) -> RepositoryResult<Self> {
        initialize_connection(&mut connection).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("failed to initialize Arcana Record database: {error}"),
            )
        })?;
        Ok(Self { connection })
    }

    pub fn checkpoint_and_close(self) -> RepositoryResult<()> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .map_err(map_sqlite_error)?;
        self.connection
            .close()
            .map_err(|(_, error)| map_sqlite_error(error))
    }

    pub fn load_records(&self) -> RepositoryResult<BTreeMap<String, RecordFile>> {
        load_records(&self.connection)
    }

    pub fn get_record(&self, definition_id: &str) -> RepositoryResult<Option<Record>> {
        load_record(&self.connection, definition_id)
    }

    pub fn begin_transaction(&mut self) -> RepositoryResult<RecordRepositoryTransaction<'_>> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error)?;
        Ok(RecordRepositoryTransaction {
            transaction: Some(transaction),
            changed: false,
        })
    }
}

pub struct RecordRepositoryTransaction<'connection> {
    transaction: Option<Transaction<'connection>>,
    changed: bool,
}

impl RecordRepositoryTransaction<'_> {
    fn connection(&self) -> &Connection {
        self.transaction
            .as_ref()
            .expect("Record transaction is active")
    }

    pub fn get_record(&self, definition_id: &str) -> RepositoryResult<Option<Record>> {
        load_record(self.connection(), definition_id)
    }

    pub fn load_records(&self) -> RepositoryResult<BTreeMap<String, RecordFile>> {
        load_records(self.connection())
    }

    pub fn has_changes(&self) -> bool {
        self.changed
    }

    pub fn put_record(&mut self, record: &Record) -> RepositoryResult<()> {
        record.validate().map_err(RepositoryError::validation)?;
        write_record(self.connection(), record)?;
        self.changed = true;
        Ok(())
    }

    pub fn delete_record(&mut self, definition_id: &str) -> RepositoryResult<()> {
        let changed = self
            .connection()
            .execute(
                "DELETE FROM records WHERE definition_id = ?1",
                [definition_id],
            )
            .map_err(map_sqlite_error)?;
        require_changed(changed, "Record", definition_id)?;
        self.changed = true;
        Ok(())
    }

    pub fn replace_records(
        &mut self,
        records: &BTreeMap<String, RecordFile>,
    ) -> RepositoryResult<()> {
        for file in records.values() {
            file.validate().map_err(RepositoryError::validation)?;
        }
        self.connection()
            .execute("DELETE FROM records", [])
            .map_err(map_sqlite_error)?;
        for record in records.values().flat_map(|file| file.records.iter()) {
            write_record(self.connection(), record)?;
        }
        self.changed = true;
        Ok(())
    }

    pub fn commit(mut self) -> RepositoryResult<()> {
        for file in load_records(self.connection())?.values() {
            file.validate().map_err(RepositoryError::validation)?;
        }
        if self.changed {
            self.connection()
                .execute(
                    "UPDATE sync_state SET data_revision = data_revision + 1 WHERE singleton = 1",
                    [],
                )
                .map_err(map_sqlite_error)?;
        }
        self.transaction
            .take()
            .expect("Record transaction is active")
            .commit()
            .map_err(map_sqlite_error)
    }

    pub fn rollback(mut self) -> RepositoryResult<()> {
        self.transaction
            .take()
            .expect("Record transaction is active")
            .rollback()
            .map_err(map_sqlite_error)
    }
}

fn load_records(connection: &Connection) -> RepositoryResult<BTreeMap<String, RecordFile>> {
    let mut statement = connection
        .prepare("SELECT definition_id FROM records ORDER BY definition_id")
        .map_err(map_sqlite_error)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(map_sqlite_error)?;
    let mut files: BTreeMap<String, RecordFile> = BTreeMap::new();
    for id in ids {
        let id = id.map_err(map_sqlite_error)?;
        let record = load_record(connection, &id)?.ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("Record header '{id}' disappeared while reading"),
            )
        })?;
        let namespace = split_record_definition_id(&id)
            .map(|(namespace, _)| namespace.to_string())
            .ok_or_else(|| {
                RepositoryError::new(
                    RepositoryErrorCode::Storage,
                    format!("stored Record has invalid definition_id '{id}'"),
                )
            })?;
        files
            .entry(namespace.clone())
            .or_insert_with(|| RecordFile {
                namespace,
                records: Vec::new(),
            })
            .records
            .push(record);
    }
    Ok(files)
}

fn load_record(connection: &Connection, definition_id: &str) -> RepositoryResult<Option<Record>> {
    let kind = connection
        .query_row(
            "SELECT kind FROM records WHERE definition_id = ?1",
            [definition_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_sqlite_error)?;
    let Some(kind) = kind else {
        return Ok(None);
    };
    match kind.as_str() {
        "scalar" => {
            let row = connection
                .query_row(
                    "SELECT value_json, effective_at, recorded_at
                     FROM scalar_records WHERE definition_id = ?1",
                    [definition_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(map_sqlite_error)?
                .ok_or_else(|| {
                    RepositoryError::new(
                        RepositoryErrorCode::Storage,
                        format!("scalar Record '{definition_id}' has no payload"),
                    )
                })?;
            Ok(Some(Record::Scalar(ScalarRecord {
                definition_id: definition_id.to_string(),
                value: deserialize_json(&row.0, "scalar_records.value_json")?,
                effective_at: row.1,
                recorded_at: row.2,
            })))
        }
        "collection" => {
            let mut statement = connection
                .prepare(
                    "SELECT item_id, payload_json, recorded_at
                     FROM collection_items WHERE definition_id = ?1 ORDER BY item_id",
                )
                .map_err(map_sqlite_error)?;
            let rows = statement
                .query_map([definition_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(map_sqlite_error)?;
            let mut items = Vec::new();
            for row in rows {
                let (id, payload, recorded_at) = row.map_err(map_sqlite_error)?;
                items.push(CollectionItem {
                    id,
                    fields: deserialize_json(&payload, "collection_items.payload_json")?,
                    recorded_at,
                });
            }
            Ok(Some(Record::Collection(CollectionRecord {
                definition_id: definition_id.to_string(),
                items,
            })))
        }
        "event" => {
            let mut statement = connection
                .prepare(
                    "SELECT event_id, occurred_at, payload_json, recorded_at
                     FROM event_entries
                     WHERE definition_id = ?1 ORDER BY occurred_at, event_id",
                )
                .map_err(map_sqlite_error)?;
            let rows = statement
                .query_map([definition_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(map_sqlite_error)?;
            let mut events = Vec::new();
            for row in rows {
                let (id, occurred_at, payload, recorded_at) = row.map_err(map_sqlite_error)?;
                events.push(EventEntry {
                    id,
                    occurred_at,
                    fields: deserialize_json(&payload, "event_entries.payload_json")?,
                    recorded_at,
                });
            }
            Ok(Some(Record::Event(EventRecord {
                definition_id: definition_id.to_string(),
                events,
            })))
        }
        value => Err(RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("Record '{definition_id}' has unknown kind '{value}'"),
        )),
    }
}

fn write_record(connection: &Connection, record: &Record) -> RepositoryResult<()> {
    let definition_id = record.definition_id();
    let kind = record_kind_to_str(record.kind());
    let existing_kind = connection
        .query_row(
            "SELECT kind FROM records WHERE definition_id = ?1",
            [definition_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_sqlite_error)?;
    if existing_kind
        .as_deref()
        .is_some_and(|existing| existing != kind)
    {
        return Err(RepositoryError::new(
            RepositoryErrorCode::Conflict,
            format!("Record '{definition_id}' kind cannot be changed"),
        ));
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO records(definition_id, kind) VALUES (?1, ?2)",
            params![definition_id, kind],
        )
        .map_err(map_sqlite_error)?;
    for table in ["scalar_records", "collection_items", "event_entries"] {
        connection
            .execute(
                &format!("DELETE FROM {table} WHERE definition_id = ?1"),
                [definition_id],
            )
            .map_err(map_sqlite_error)?;
    }

    match record {
        Record::Scalar(record) => {
            connection
                .execute(
                    "INSERT INTO scalar_records(definition_id, value_json, effective_at, recorded_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        record.definition_id,
                        serialize_json(&record.value, "scalar Record value")?,
                        record.effective_at,
                        record.recorded_at
                    ],
                )
                .map_err(map_sqlite_error)?;
        }
        Record::Collection(record) => {
            for item in &record.items {
                connection
                    .execute(
                        "INSERT INTO collection_items(definition_id, item_id, payload_json, recorded_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            record.definition_id,
                            item.id,
                            serialize_json(&item.fields, "collection item payload")?,
                            item.recorded_at
                        ],
                    )
                    .map_err(map_sqlite_error)?;
            }
        }
        Record::Event(record) => {
            for event in &record.events {
                connection
                    .execute(
                        "INSERT INTO event_entries(
                            definition_id, event_id, occurred_at, payload_json, recorded_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            record.definition_id,
                            event.id,
                            event.occurred_at,
                            serialize_json(&event.fields, "event payload")?,
                            event.recorded_at
                        ],
                    )
                    .map_err(map_sqlite_error)?;
            }
        }
    }
    Ok(())
}

fn serialize_json(value: &impl Serialize, context: &str) -> RepositoryResult<String> {
    serde_json::to_string(value).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to serialize {context}: {error}"),
        )
    })
}

fn deserialize_json<T: DeserializeOwned>(json: &str, context: &str) -> RepositoryResult<T> {
    serde_json::from_str(json).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to deserialize {context}: {error}"),
        )
    })
}

fn require_changed(changed: usize, entity: &str, id: &str) -> RepositoryResult<()> {
    if changed == 0 {
        Err(RepositoryError::new(
            RepositoryErrorCode::NotFound,
            format!("{entity} '{id}' was not found"),
        ))
    } else {
        Ok(())
    }
}

fn map_sqlite_error(error: rusqlite::Error) -> RepositoryError {
    use rusqlite::ffi::ErrorCode;
    let code = match &error {
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
            ) =>
        {
            RepositoryErrorCode::Busy
        }
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::ConstraintViolation =>
        {
            RepositoryErrorCode::Conflict
        }
        _ => RepositoryErrorCode::Storage,
    };
    RepositoryError::new(code, format!("SQLite error: {error}"))
}

fn record_kind_to_str(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Scalar => "scalar",
        RecordKind::Collection => "collection",
        RecordKind::Event => "event",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stores_only_records_after_record_only_migration() {
        let mut repository = RecordRepository::open_in_memory().unwrap();
        let record = Record::Scalar(ScalarRecord {
            definition_id: "basic.nickname".to_string(),
            value: json!("Joker"),
            effective_at: None,
            recorded_at: "2026-08-16T00:00:00+08:00".to_string(),
        });
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_record(&record).unwrap();
        transaction.commit().unwrap();
        assert_eq!(
            repository.get_record("basic.nickname").unwrap(),
            Some(record)
        );

        let table_names: Vec<String> = repository
            .connection
            .prepare(
                "SELECT name FROM sqlite_schema
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            table_names,
            [
                "collection_items",
                "event_entries",
                "records",
                "scalar_records",
                "schema_migrations",
                "sync_state",
            ]
        );
    }
}
