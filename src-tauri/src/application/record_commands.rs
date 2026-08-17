use crate::domain::{
    ArcanaRepository, ArcanaRepositoryTransaction, CollectionItem, CollectionRecord, EventEntry,
    EventRecord, Record, RecordDefinition, RepositoryError, RepositoryErrorCode, RepositoryResult,
    ScalarRecord, ValueType,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::BTreeMap;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateEmptyRecord {
    pub definition_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddCollectionItem {
    pub definition_id: String,
    pub item_id: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectCollectionItem {
    pub definition_id: String,
    pub item_id: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveCollectionItem {
    pub definition_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppendEvent {
    pub definition_id: String,
    pub event_id: String,
    pub occurred_at: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectEvent {
    pub definition_id: String,
    pub event_id: String,
    pub occurred_at: String,
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteEvent {
    pub definition_id: String,
    pub event_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryRecords {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<crate::domain::RecordKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_value: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordQueryEntry {
    pub definition_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<RecordDefinition>,
    pub supplied_by_pack_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<Record>,
}

pub(crate) enum RecordMutation {
    SetScalar(SetScalarRecord),
    IncrementScalar(IncrementScalarRecord),
    CreateEmptyCollection(CreateEmptyRecord),
    AddCollectionItem(AddCollectionItem),
    CorrectCollectionItem(CorrectCollectionItem),
    RemoveCollectionItem(RemoveCollectionItem),
    CreateEmptyEvent(CreateEmptyRecord),
    AppendEvent(AppendEvent),
    CorrectEvent(CorrectEvent),
    DeleteEvent(DeleteEvent),
    Delete(String),
}

pub(crate) enum RecordMutationResult {
    Record(Record),
    Deleted { definition_id: String },
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

    /// Query active RecordDefinitions together with their current values.
    /// Records whose definitions are no longer supplied by an enabled Pack are
    /// retained in the result with `definition = None` unless a Pack filter is
    /// present. Every filter is combined with logical AND.
    pub fn query(&self, query: QueryRecords) -> RepositoryResult<Vec<RecordQueryEntry>> {
        let snapshot = self.repository.load_synced_snapshot()?;
        let registry = snapshot.definition_registry()?;
        let mut records: BTreeMap<String, Record> = snapshot
            .records
            .values()
            .flat_map(|file| file.records.iter().cloned())
            .map(|record| (record.definition_id().to_string(), record))
            .collect();
        let mut entries = BTreeMap::new();

        for (definition_id, definition) in registry.iter() {
            let supplied_by_pack_ids = snapshot
                .manifest
                .enabled_pack_ids
                .iter()
                .filter(|pack_id| {
                    snapshot
                        .packs
                        .get(*pack_id)
                        .and_then(|pack| pack.record_definitions.as_ref())
                        .is_some_and(|file| {
                            file.definitions
                                .iter()
                                .any(|candidate| candidate.id() == definition_id)
                        })
                })
                .cloned()
                .collect();
            entries.insert(
                definition_id.to_string(),
                RecordQueryEntry {
                    definition_id: definition_id.to_string(),
                    definition: Some(definition.clone()),
                    supplied_by_pack_ids,
                    record: records.remove(definition_id),
                },
            );
        }

        for (definition_id, record) in records {
            entries.insert(
                definition_id.clone(),
                RecordQueryEntry {
                    definition_id,
                    definition: None,
                    supplied_by_pack_ids: Vec::new(),
                    record: Some(record),
                },
            );
        }

        Ok(entries
            .into_values()
            .filter(|entry| record_query_matches(entry, &query))
            .collect())
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

    pub fn create_empty_collection(
        &mut self,
        command: CreateEmptyRecord,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(
            RecordMutation::CreateEmptyCollection(command),
            now_rfc3339(),
        )?
        .into_record()
    }

    pub fn add_collection_item(&mut self, command: AddCollectionItem) -> RepositoryResult<Record> {
        self.add_collection_item_at(command, now_rfc3339())
    }

    pub fn correct_collection_item(
        &mut self,
        command: CorrectCollectionItem,
    ) -> RepositoryResult<Record> {
        self.correct_collection_item_at(command, now_rfc3339())
    }

    pub fn remove_collection_item(
        &mut self,
        command: RemoveCollectionItem,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::RemoveCollectionItem(command), now_rfc3339())?
            .into_record()
    }

    pub fn create_empty_event(&mut self, command: CreateEmptyRecord) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::CreateEmptyEvent(command), now_rfc3339())?
            .into_record()
    }

    pub fn append_event(&mut self, command: AppendEvent) -> RepositoryResult<Record> {
        self.append_event_at(command, now_rfc3339())
    }

    pub fn correct_event(&mut self, command: CorrectEvent) -> RepositoryResult<Record> {
        self.correct_event_at(command, now_rfc3339())
    }

    pub fn delete_event(&mut self, command: DeleteEvent) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::DeleteEvent(command), now_rfc3339())?
            .into_record()
    }

    pub fn delete(&mut self, definition_id: &str) -> RepositoryResult<()> {
        self.execute_record_mutation(
            RecordMutation::Delete(definition_id.to_string()),
            now_rfc3339(),
        )?;
        Ok(())
    }

    pub(crate) fn set_scalar_at(
        &mut self,
        command: SetScalarRecord,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::SetScalar(command), recorded_at)?
            .into_record()
    }

    pub(crate) fn increment_scalar_at(
        &mut self,
        command: IncrementScalarRecord,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::IncrementScalar(command), recorded_at)?
            .into_record()
    }

    pub(crate) fn add_collection_item_at(
        &mut self,
        command: AddCollectionItem,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::AddCollectionItem(command), recorded_at)?
            .into_record()
    }

    pub(crate) fn correct_collection_item_at(
        &mut self,
        command: CorrectCollectionItem,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::CorrectCollectionItem(command), recorded_at)?
            .into_record()
    }

    pub(crate) fn append_event_at(
        &mut self,
        command: AppendEvent,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::AppendEvent(command), recorded_at)?
            .into_record()
    }

    pub(crate) fn correct_event_at(
        &mut self,
        command: CorrectEvent,
        recorded_at: String,
    ) -> RepositoryResult<Record> {
        self.execute_record_mutation(RecordMutation::CorrectEvent(command), recorded_at)?
            .into_record()
    }

    fn execute_record_mutation(
        &mut self,
        mutation: RecordMutation,
        recorded_at: String,
    ) -> RepositoryResult<RecordMutationResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let result = apply_record_mutation(&mut transaction, mutation, &recorded_at)?;
        transaction.commit()?;
        Ok(result)
    }
}

impl RecordMutationResult {
    fn into_record(self) -> RepositoryResult<Record> {
        match self {
            Self::Record(record) => Ok(record),
            Self::Deleted { .. } => Err(command_error(
                "internal Record mutation result mismatch".to_string(),
            )),
        }
    }
}

pub(crate) fn apply_record_mutation<T>(
    transaction: &mut T,
    mutation: RecordMutation,
    recorded_at: &str,
) -> RepositoryResult<RecordMutationResult>
where
    T: ArcanaRepositoryTransaction,
{
    let result = match mutation {
        RecordMutation::SetScalar(command) => Record::Scalar(ScalarRecord {
            definition_id: command.definition_id,
            value: command.value,
            effective_at: command.effective_at,
            recorded_at: recorded_at.to_string(),
        }),
        RecordMutation::IncrementScalar(command) => {
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
            Record::Scalar(ScalarRecord {
                definition_id: command.definition_id,
                value: increment_value(value_type, &existing.value, &command.delta)?,
                effective_at: command.effective_at.or(existing.effective_at),
                recorded_at: recorded_at.to_string(),
            })
        }
        RecordMutation::CreateEmptyCollection(command) => {
            let record = Record::Collection(CollectionRecord {
                definition_id: command.definition_id,
                items: Vec::new(),
            });
            ensure_record_absent(transaction, record.definition_id())?;
            record
        }
        RecordMutation::AddCollectionItem(command) => {
            let mut collection = match transaction.get_record(&command.definition_id)? {
                Some(Record::Collection(collection)) => collection,
                Some(_) => return Err(kind_conflict(&command.definition_id, "collection")),
                None => CollectionRecord {
                    definition_id: command.definition_id.clone(),
                    items: Vec::new(),
                },
            };
            if collection
                .items
                .iter()
                .any(|item| item.id == command.item_id)
            {
                return Err(child_conflict(
                    "collection item",
                    &command.item_id,
                    &command.definition_id,
                ));
            }
            collection.items.push(CollectionItem {
                id: command.item_id,
                fields: command.fields,
                recorded_at: recorded_at.to_string(),
            });
            collection
                .items
                .sort_by(|left, right| left.id.cmp(&right.id));
            Record::Collection(collection)
        }
        RecordMutation::CorrectCollectionItem(command) => {
            let existing = required_record(
                transaction.get_record(&command.definition_id)?,
                &command.definition_id,
            )?;
            let Record::Collection(mut collection) = existing else {
                return Err(kind_conflict(&command.definition_id, "collection"));
            };
            let item = collection
                .items
                .iter_mut()
                .find(|item| item.id == command.item_id)
                .ok_or_else(|| {
                    child_not_found("collection item", &command.item_id, &command.definition_id)
                })?;
            item.fields = command.fields;
            item.recorded_at = recorded_at.to_string();
            Record::Collection(collection)
        }
        RecordMutation::RemoveCollectionItem(command) => {
            let existing = required_record(
                transaction.get_record(&command.definition_id)?,
                &command.definition_id,
            )?;
            let Record::Collection(mut collection) = existing else {
                return Err(kind_conflict(&command.definition_id, "collection"));
            };
            let original_len = collection.items.len();
            collection.items.retain(|item| item.id != command.item_id);
            if collection.items.len() == original_len {
                return Err(child_not_found(
                    "collection item",
                    &command.item_id,
                    &command.definition_id,
                ));
            }
            Record::Collection(collection)
        }
        RecordMutation::CreateEmptyEvent(command) => {
            let record = Record::Event(EventRecord {
                definition_id: command.definition_id,
                events: Vec::new(),
            });
            ensure_record_absent(transaction, record.definition_id())?;
            record
        }
        RecordMutation::AppendEvent(command) => {
            let mut event_record = match transaction.get_record(&command.definition_id)? {
                Some(Record::Event(event_record)) => event_record,
                Some(_) => return Err(kind_conflict(&command.definition_id, "event")),
                None => EventRecord {
                    definition_id: command.definition_id.clone(),
                    events: Vec::new(),
                },
            };
            if event_record
                .events
                .iter()
                .any(|event| event.id == command.event_id)
            {
                return Err(child_conflict(
                    "event",
                    &command.event_id,
                    &command.definition_id,
                ));
            }
            event_record.events.push(EventEntry {
                id: command.event_id,
                occurred_at: command.occurred_at,
                fields: command.fields,
                recorded_at: recorded_at.to_string(),
            });
            sort_events(&mut event_record.events);
            Record::Event(event_record)
        }
        RecordMutation::CorrectEvent(command) => {
            let existing = required_record(
                transaction.get_record(&command.definition_id)?,
                &command.definition_id,
            )?;
            let Record::Event(mut event_record) = existing else {
                return Err(kind_conflict(&command.definition_id, "event"));
            };
            let event = event_record
                .events
                .iter_mut()
                .find(|event| event.id == command.event_id)
                .ok_or_else(|| {
                    child_not_found("event", &command.event_id, &command.definition_id)
                })?;
            event.occurred_at = command.occurred_at;
            event.fields = command.fields;
            event.recorded_at = recorded_at.to_string();
            sort_events(&mut event_record.events);
            Record::Event(event_record)
        }
        RecordMutation::DeleteEvent(command) => {
            let existing = required_record(
                transaction.get_record(&command.definition_id)?,
                &command.definition_id,
            )?;
            let Record::Event(mut event_record) = existing else {
                return Err(kind_conflict(&command.definition_id, "event"));
            };
            let original_len = event_record.events.len();
            event_record
                .events
                .retain(|event| event.id != command.event_id);
            if event_record.events.len() == original_len {
                return Err(child_not_found(
                    "event",
                    &command.event_id,
                    &command.definition_id,
                ));
            }
            Record::Event(event_record)
        }
        RecordMutation::Delete(definition_id) => {
            transaction.delete_record(&definition_id)?;
            return Ok(RecordMutationResult::Deleted { definition_id });
        }
    };
    transaction.put_record(result.clone())?;
    Ok(RecordMutationResult::Record(result))
}

fn ensure_record_absent<T>(transaction: &T, definition_id: &str) -> RepositoryResult<()>
where
    T: ArcanaRepositoryTransaction,
{
    if transaction.get_record(definition_id)?.is_some() {
        return Err(RepositoryError::new(
            RepositoryErrorCode::Conflict,
            format!("Record '{definition_id}' already exists"),
        ));
    }
    Ok(())
}

fn record_query_matches(entry: &RecordQueryEntry, query: &QueryRecords) -> bool {
    if query
        .definition_id
        .as_deref()
        .is_some_and(|definition_id| entry.definition_id != definition_id)
    {
        return false;
    }
    if query.namespace.as_deref().is_some_and(|namespace| {
        crate::domain::split_record_definition_id(&entry.definition_id)
            .is_none_or(|(candidate, _)| candidate != namespace)
    }) {
        return false;
    }
    if query.pack_id.as_deref().is_some_and(|pack_id| {
        !entry
            .supplied_by_pack_ids
            .iter()
            .any(|candidate| candidate == pack_id)
    }) {
        return false;
    }
    let kind = entry
        .definition
        .as_ref()
        .map(RecordDefinition::kind)
        .or_else(|| entry.record.as_ref().map(Record::kind));
    if query.kind.is_some_and(|expected| kind != Some(expected)) {
        return false;
    }
    if query
        .has_value
        .is_some_and(|has_value| entry.record.is_some() != has_value)
    {
        return false;
    }
    true
}

fn required_record(record: Option<Record>, definition_id: &str) -> RepositoryResult<Record> {
    record.ok_or_else(|| {
        RepositoryError::new(
            RepositoryErrorCode::NotFound,
            format!("Record '{definition_id}' does not exist"),
        )
    })
}

fn kind_conflict(definition_id: &str, expected: &str) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::Conflict,
        format!("Record '{definition_id}' is not stored as {expected}"),
    )
}

fn child_conflict(kind: &str, id: &str, definition_id: &str) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::Conflict,
        format!("{kind} '{id}' already exists in Record '{definition_id}'"),
    )
}

fn child_not_found(kind: &str, id: &str, definition_id: &str) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::NotFound,
        format!("{kind} '{id}' does not exist in Record '{definition_id}'"),
    )
}

fn sort_events(events: &mut [EventEntry]) {
    events.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.id.cmp(&right.id))
    });
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
        ArcanaRepositoryTransaction, FieldDefinition, Pack, PackManifest, RecordDefinitionFile,
        ScalarRecordDefinition, StructuredRecordDefinition, SCHEMA_VERSION,
    };
    use crate::storage::DataRepository;
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
                    RecordDefinition::Event(StructuredRecordDefinition {
                        id: "stats.activities".to_string(),
                        name: "Activities".to_string(),
                        description: None,
                        fields: BTreeMap::from([
                            (
                                "distance".to_string(),
                                FieldDefinition {
                                    value_type: ValueType::Number,
                                    required: false,
                                    unit: Some("km".to_string()),
                                },
                            ),
                            (
                                "kind".to_string(),
                                FieldDefinition {
                                    value_type: ValueType::String,
                                    required: true,
                                    unit: None,
                                },
                            ),
                        ]),
                    }),
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "stats.count".to_string(),
                        name: "Count".to_string(),
                        description: None,
                        value_type: ValueType::Integer,
                        unit: None,
                    }),
                    RecordDefinition::Collection(StructuredRecordDefinition {
                        id: "stats.projects".to_string(),
                        name: "Projects".to_string(),
                        description: None,
                        fields: BTreeMap::from([
                            (
                                "rating".to_string(),
                                FieldDefinition {
                                    value_type: ValueType::Integer,
                                    required: false,
                                    unit: None,
                                },
                            ),
                            (
                                "title".to_string(),
                                FieldDefinition {
                                    value_type: ValueType::String,
                                    required: true,
                                    unit: None,
                                },
                            ),
                        ]),
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
            derived_values: None,
            dimensions: None,
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn repository() -> DataRepository {
        let mut repository = DataRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(numeric_pack()).unwrap();
        transaction.set_pack_enabled("stats", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    #[test]
    fn query_combines_active_definitions_suppliers_and_current_records() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        let empty_scalars = commands
            .query(QueryRecords {
                namespace: Some("stats".to_string()),
                kind: Some(crate::domain::RecordKind::Scalar),
                has_value: Some(false),
                ..QueryRecords::default()
            })
            .unwrap();
        assert_eq!(
            empty_scalars
                .iter()
                .map(|entry| entry.definition_id.as_str())
                .collect::<Vec<_>>(),
            vec!["stats.count", "stats.score"]
        );

        commands
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "stats.count".to_string(),
                    value: json!(2),
                    effective_at: None,
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        let entries = commands
            .query(QueryRecords {
                definition_id: Some("stats.count".to_string()),
                pack_id: Some("stats".to_string()),
                has_value: Some(true),
                ..QueryRecords::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].supplied_by_pack_ids, ["stats"]);
        assert!(entries[0].definition.is_some());
        assert!(entries[0].record.is_some());

        assert!(commands
            .query(QueryRecords {
                pack_id: Some("disabled_pack".to_string()),
                ..QueryRecords::default()
            })
            .unwrap()
            .is_empty());
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
        let mut repository = DataRepository::open_in_memory().unwrap();
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

    #[test]
    fn explicit_empty_collection_is_preserved_and_never_clears_existing_data() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        let empty = commands
            .create_empty_collection(CreateEmptyRecord {
                definition_id: "stats.projects".to_string(),
            })
            .unwrap();
        assert_eq!(
            empty,
            Record::Collection(CollectionRecord {
                definition_id: "stats.projects".to_string(),
                items: vec![],
            })
        );
        let error = commands
            .create_empty_collection(CreateEmptyRecord {
                definition_id: "stats.projects".to_string(),
            })
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Conflict);
        assert_eq!(commands.get("stats.projects").unwrap(), Some(empty));
    }

    #[test]
    fn collection_commands_sort_reject_duplicates_correct_and_preserve_empty_header() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        commands
            .add_collection_item_at(
                AddCollectionItem {
                    definition_id: "stats.projects".to_string(),
                    item_id: "project_b".to_string(),
                    fields: BTreeMap::from([("title".to_string(), json!("B"))]),
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        let added = commands
            .add_collection_item_at(
                AddCollectionItem {
                    definition_id: "stats.projects".to_string(),
                    item_id: "project_a".to_string(),
                    fields: BTreeMap::from([("title".to_string(), json!("A"))]),
                },
                "2026-08-15T20:31:00+08:00".to_string(),
            )
            .unwrap();
        let Record::Collection(added) = added else {
            panic!("stats.projects must be a collection");
        };
        assert_eq!(
            added
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["project_a", "project_b"]
        );

        let before_duplicate = commands.get("stats.projects").unwrap();
        let error = commands
            .add_collection_item_at(
                AddCollectionItem {
                    definition_id: "stats.projects".to_string(),
                    item_id: "project_a".to_string(),
                    fields: BTreeMap::from([("title".to_string(), json!("duplicate"))]),
                },
                "2026-08-15T20:32:00+08:00".to_string(),
            )
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Conflict);
        assert_eq!(commands.get("stats.projects").unwrap(), before_duplicate);

        let corrected = commands
            .correct_collection_item_at(
                CorrectCollectionItem {
                    definition_id: "stats.projects".to_string(),
                    item_id: "project_a".to_string(),
                    fields: BTreeMap::from([
                        ("rating".to_string(), json!(5)),
                        ("title".to_string(), json!("A+")),
                    ]),
                },
                "2026-08-15T20:33:00+08:00".to_string(),
            )
            .unwrap();
        let Record::Collection(corrected) = corrected else {
            panic!("stats.projects must be a collection");
        };
        assert_eq!(corrected.items[0].fields["title"], json!("A+"));
        assert_eq!(corrected.items[0].recorded_at, "2026-08-15T20:33:00+08:00");

        commands
            .remove_collection_item(RemoveCollectionItem {
                definition_id: "stats.projects".to_string(),
                item_id: "project_a".to_string(),
            })
            .unwrap();
        let empty = commands
            .remove_collection_item(RemoveCollectionItem {
                definition_id: "stats.projects".to_string(),
                item_id: "project_b".to_string(),
            })
            .unwrap();
        assert_eq!(
            empty,
            Record::Collection(CollectionRecord {
                definition_id: "stats.projects".to_string(),
                items: vec![],
            })
        );
        assert_eq!(commands.get("stats.projects").unwrap(), Some(empty));
    }

    #[test]
    fn invalid_collection_item_rolls_back_implicit_record_creation() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        let error = commands
            .add_collection_item_at(
                AddCollectionItem {
                    definition_id: "stats.projects".to_string(),
                    item_id: "project_a".to_string(),
                    fields: BTreeMap::new(),
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
        assert_eq!(commands.get("stats.projects").unwrap(), None);
    }

    #[test]
    fn event_commands_sort_reject_duplicates_correct_and_preserve_empty_header() {
        let mut repository = repository();
        let mut commands = RecordCommands::new(&mut repository);
        let empty = commands
            .create_empty_event(CreateEmptyRecord {
                definition_id: "stats.activities".to_string(),
            })
            .unwrap();
        assert_eq!(
            empty,
            Record::Event(EventRecord {
                definition_id: "stats.activities".to_string(),
                events: vec![],
            })
        );

        commands
            .append_event_at(
                AppendEvent {
                    definition_id: "stats.activities".to_string(),
                    event_id: "run_late".to_string(),
                    occurred_at: "2026-08-15T19:00:00+08:00".to_string(),
                    fields: BTreeMap::from([("kind".to_string(), json!("run"))]),
                },
                "2026-08-15T20:30:00+08:00".to_string(),
            )
            .unwrap();
        let appended = commands
            .append_event_at(
                AppendEvent {
                    definition_id: "stats.activities".to_string(),
                    event_id: "walk_early".to_string(),
                    occurred_at: "2026-08-15T08:00:00+08:00".to_string(),
                    fields: BTreeMap::from([("kind".to_string(), json!("walk"))]),
                },
                "2026-08-15T20:31:00+08:00".to_string(),
            )
            .unwrap();
        let Record::Event(appended) = appended else {
            panic!("stats.activities must be an event Record");
        };
        assert_eq!(
            appended
                .events
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            vec!["walk_early", "run_late"]
        );

        let before_duplicate = commands.get("stats.activities").unwrap();
        let error = commands
            .append_event_at(
                AppendEvent {
                    definition_id: "stats.activities".to_string(),
                    event_id: "run_late".to_string(),
                    occurred_at: "2026-08-15T21:00:00+08:00".to_string(),
                    fields: BTreeMap::from([("kind".to_string(), json!("run"))]),
                },
                "2026-08-15T20:32:00+08:00".to_string(),
            )
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Conflict);
        assert_eq!(commands.get("stats.activities").unwrap(), before_duplicate);

        let corrected = commands
            .correct_event_at(
                CorrectEvent {
                    definition_id: "stats.activities".to_string(),
                    event_id: "run_late".to_string(),
                    occurred_at: "2026-08-15T07:00:00+08:00".to_string(),
                    fields: BTreeMap::from([
                        ("distance".to_string(), json!(5.2)),
                        ("kind".to_string(), json!("run")),
                    ]),
                },
                "2026-08-15T20:33:00+08:00".to_string(),
            )
            .unwrap();
        let Record::Event(corrected) = corrected else {
            panic!("stats.activities must be an event Record");
        };
        assert_eq!(corrected.events[0].id, "run_late");
        assert_eq!(corrected.events[0].recorded_at, "2026-08-15T20:33:00+08:00");

        commands
            .delete_event(DeleteEvent {
                definition_id: "stats.activities".to_string(),
                event_id: "run_late".to_string(),
            })
            .unwrap();
        let empty = commands
            .delete_event(DeleteEvent {
                definition_id: "stats.activities".to_string(),
                event_id: "walk_early".to_string(),
            })
            .unwrap();
        assert_eq!(
            empty,
            Record::Event(EventRecord {
                definition_id: "stats.activities".to_string(),
                events: vec![],
            })
        );
        assert_eq!(commands.get("stats.activities").unwrap(), Some(empty));
    }
}
