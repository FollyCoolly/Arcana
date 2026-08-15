use crate::domain::{
    ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction, Record,
    RecordDefinition, RepositoryError, RepositoryErrorCode, RepositoryResult, ScalarRecord,
    ValueType,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetScalarRecord {
    pub definition_id: String,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncrementScalarRecord {
    pub definition_id: String,
    pub delta: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_at: Option<String>,
}

pub struct RecordCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> RecordCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn get(&self, definition_id: &str) -> RepositoryResult<Option<Record>> {
        self.repository.get_record(definition_id)
    }

    pub fn set_scalar(&mut self, command: SetScalarRecord) -> RepositoryResult<Record> {
        self.set_scalar_at(command, now_rfc3339())
    }

    pub fn correct_scalar(&mut self, command: SetScalarRecord) -> RepositoryResult<Record> {
        self.set_scalar(command)
    }

    pub fn increment_scalar(&mut self, command: IncrementScalarRecord) -> RepositoryResult<Record> {
        self.increment_scalar_at(command, now_rfc3339())
    }

    pub fn delete(&mut self, definition_id: &str) -> RepositoryResult<()> {
        let mut transaction = self.repository.begin_transaction()?;
        transaction.delete_record(definition_id)?;
        transaction.commit()
    }

    pub(crate) fn set_scalar_at(
        &mut self,
        command: SetScalarRecord,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        let record = Record::Scalar(ScalarRecord {
            definition_id: command.definition_id,
            value: command.value,
            effective_at: command.effective_at,
            recorded_at,
        });
        let mut transaction = self.repository.begin_transaction()?;
        transaction.put_record(record.clone())?;
        transaction.commit()?;
        Ok(record)
    }

    pub(crate) fn increment_scalar_at(
        &mut self,
        command: IncrementScalarRecord,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let registry = snapshot.definition_registry()?;
        let value_type = match registry.get(&command.definition_id) {
            Some(RecordDefinition::Scalar(definition)) => definition.value_type,
            Some(_) => {
                return Err(command_error(format!(
                    "Record '{}' is not scalar and cannot be incremented",
                    command.definition_id
                )))
            }
            None => {
                return Err(RepositoryError::new(
                    RepositoryErrorCode::Unresolved,
                    format!(
                        "RecordDefinition '{}' is not supplied by an enabled Pack",
                        command.definition_id
                    ),
                ))
            }
        };
        if !matches!(value_type, ValueType::Number | ValueType::Integer) {
            return Err(command_error(format!(
                "Record '{}' has non-numeric type {:?}",
                command.definition_id, value_type
            )));
        }

        let existing = transaction
            .get_record(&command.definition_id)?
            .ok_or_else(|| {
                RepositoryError::new(
                    RepositoryErrorCode::NotFound,
                    format!("Record '{}' has no current value", command.definition_id),
                )
            })?;
        let Record::Scalar(existing) = existing else {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("Record '{}' is not stored as scalar", command.definition_id),
            ));
        };
        let value = increment_value(value_type, &existing.value, &command.delta)?;
        let record = Record::Scalar(ScalarRecord {
            definition_id: command.definition_id,
            value,
            effective_at: command.effective_at.or(existing.effective_at),
            recorded_at,
        });
        transaction.put_record(record.clone())?;
        transaction.commit()?;
        Ok(record)
    }
}

fn increment_value(
    value_type: ValueType,
    current: &Value,
    delta: &Value,
) -> RepositoryResult<Value> {
    match value_type {
        ValueType::Integer => {
            let current = exact_integer(current).ok_or_else(|| {
                command_error("stored integer Record contains a non-integer value")
            })?;
            let delta = exact_integer(delta)
                .ok_or_else(|| command_error("integer increment delta must be an integer"))?;
            let result = current
                .checked_add(delta)
                .ok_or_else(|| command_error("integer increment overflowed"))?;
            integer_json_value(result)
        }
        ValueType::Number => {
            let current = current
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| command_error("stored number Record is not finite"))?;
            let delta = delta
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| command_error("number increment delta must be numeric"))?;
            let result = current + delta;
            let number = Number::from_f64(result)
                .ok_or_else(|| command_error("number increment produced a non-finite value"))?;
            Ok(Value::Number(number))
        }
        _ => Err(command_error(
            "only number and integer Records can be incremented",
        )),
    }
}

fn exact_integer(value: &Value) -> Option<i128> {
    value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
}

fn integer_json_value(value: i128) -> RepositoryResult<Value> {
    let number = if let Ok(value) = i64::try_from(value) {
        Number::from(value)
    } else if let Ok(value) = u64::try_from(value) {
        Number::from(value)
    } else {
        return Err(command_error(
            "integer increment result exceeds the JSON integer range",
        ));
    };
    Ok(Value::Number(number))
}

fn command_error(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorCode::ValidationFailed, message)
}

fn now_rfc3339() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::BASIC_PACK_ID;
    use crate::domain::{
        ArcanaRepositoryTransaction, Pack, PackManifest, RecordDefinitionFile,
        ScalarRecordDefinition, SCHEMA_VERSION,
    };
    use crate::storage::sqlite::SqliteRepository;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn numeric_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "stats".to_string(),
                name: "Stats".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: vec![],
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "stats.count".to_string(),
                        name: "Count".to_string(),
                        description: None,
                        value_type: ValueType::Integer,
                        unit: None,
                    }),
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "stats.score".to_string(),
                        name: "Score".to_string(),
                        description: None,
                        value_type: ValueType::Number,
                        unit: None,
                    }),
                ],
            }),
            dimensions: None,
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn repository() -> SqliteRepository {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(numeric_pack()).unwrap();
        transaction.set_pack_enabled("stats", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    #[test]
    fn set_get_correct_and_delete_scalar() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        let first = commands
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "stats.count".to_string(),
                    value: json!(2),
                    effective_at: None,
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        assert_eq!(commands.get("stats.count").unwrap(), Some(first));

        let corrected = commands
            .correct_scalar(SetScalarRecord {
                definition_id: "stats.count".to_string(),
                value: json!(3),
                effective_at: None,
            })
            .unwrap();
        let Record::Scalar(corrected) = corrected else {
            panic!("stats.count must be scalar");
        };
        assert_eq!(corrected.value, json!(3));
        commands.delete("stats.count").unwrap();
        assert_eq!(commands.get("stats.count").unwrap(), None);
    }

    #[test]
    fn increment_reads_and_writes_inside_transaction() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        commands
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "stats.count".to_string(),
                    value: json!(10),
                    effective_at: Some("2026-08-15".to_string()),
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        let updated = commands
            .increment_scalar_at(
                IncrementScalarRecord {
                    definition_id: "stats.count".to_string(),
                    delta: json!(5),
                    effective_at: None,
                },
                "2026-08-15T20:31:00+08:00".to_string(),
            )
            .unwrap();
        assert_eq!(
            updated,
            Record::Scalar(ScalarRecord {
                definition_id: "stats.count".to_string(),
                value: json!(15),
                effective_at: Some("2026-08-15".to_string()),
                recorded_at: "2026-08-15T20:31:00+08:00".to_string(),
            })
        );
    }

    #[test]
    fn integer_increment_rejects_fractional_delta_without_changing_record() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        commands
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "stats.count".to_string(),
                    value: json!(10),
                    effective_at: None,
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        let before = commands.get("stats.count").unwrap();
        let error = commands
            .increment_scalar_at(
                IncrementScalarRecord {
                    definition_id: "stats.count".to_string(),
                    delta: json!(0.5),
                    effective_at: None,
                },
                "2026-08-15T20:31:00+08:00".to_string(),
            )
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
        assert_eq!(commands.get("stats.count").unwrap(), before);
    }

    #[test]
    fn increment_rejects_non_numeric_definition() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .put_pack(crate::application::basic_pack())
            .unwrap();
        transaction.set_pack_enabled(BASIC_PACK_ID, true).unwrap();
        transaction.commit().unwrap();
        let mut commands = RecordCommands::new(&mut repository);
        commands
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "identity.nickname".to_string(),
                    value: json!("Alice"),
                    effective_at: None,
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        let error = commands
            .increment_scalar_at(
                IncrementScalarRecord {
                    definition_id: "identity.nickname".to_string(),
                    delta: json!(1),
                    effective_at: None,
                },
                "2026-08-15T20:31:00+08:00".to_string(),
            )
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
    }
}
