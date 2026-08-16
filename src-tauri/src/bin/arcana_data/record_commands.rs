use super::contract::{CliError, RepositoryOperation};
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{
    AddCollectionItem, AppendEvent, CorrectCollectionItem, CorrectEvent, CreateEmptyRecord,
    DeleteEvent, IncrementScalarRecord, QueryRecords, RecordCommands, RemoveCollectionItem,
    SetScalarRecord,
};
use arcana_lib::domain::RecordKind;
use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum RecordAction {
    /// Get the current Record value for one exact definition ID
    Get {
        /// RecordDefinition ID, for example identity.nickname
        definition_id: String,
    },
    /// Query active definitions, supplying Packs, and optional current values
    Query {
        /// Match one exact RecordDefinition ID
        #[arg(long)]
        definition_id: Option<String>,
        /// Match the namespace before the dot in a RecordDefinition ID
        #[arg(long)]
        namespace: Option<String>,
        /// Match definitions supplied by this enabled Pack
        #[arg(long)]
        pack: Option<String>,
        /// Match one Record kind
        #[arg(long)]
        kind: Option<RecordKindArg>,
        /// Match definitions with or without a current Record value
        #[arg(long, value_name = "BOOL")]
        has_value: Option<bool>,
    },
    /// Set a scalar Record from JSON on stdin or --file
    Set {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Increment a numeric scalar Record from JSON on stdin or --file
    Increment {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Correct a scalar Record from JSON on stdin or --file
    Correct {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Create an explicitly empty collection Record
    CreateEmptyCollection { definition_id: String },
    /// Create an explicitly empty event Record
    CreateEmptyEvent { definition_id: String },
    /// Add a collection item from JSON on stdin or --file
    AddItem {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Replace a collection item's fields from JSON on stdin or --file
    CorrectItem {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Remove a collection item from JSON on stdin or --file
    RemoveItem {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Append an event from JSON on stdin or --file
    AppendEvent {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Replace an event from JSON on stdin or --file
    CorrectEvent {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Delete an event from JSON on stdin or --file
    DeleteEvent {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Delete the current Record value while retaining its definition
    Delete { definition_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RecordKindArg {
    Scalar,
    Collection,
    Event,
}

impl From<RecordKindArg> for RecordKind {
    fn from(value: RecordKindArg) -> Self {
        match value {
            RecordKindArg::Scalar => Self::Scalar,
            RecordKindArg::Collection => Self::Collection,
            RecordKindArg::Event => Self::Event,
        }
    }
}

#[derive(Debug)]
enum PreparedRecordAction {
    Get(String),
    Query(QueryRecords),
    Set(SetScalarRecord),
    Increment(IncrementScalarRecord),
    Correct(SetScalarRecord),
    CreateEmptyCollection(CreateEmptyRecord),
    CreateEmptyEvent(CreateEmptyRecord),
    AddItem(AddCollectionItem),
    CorrectItem(CorrectCollectionItem),
    RemoveItem(RemoveCollectionItem),
    AppendEvent(AppendEvent),
    CorrectEvent(CorrectEvent),
    DeleteEvent(DeleteEvent),
    Delete(String),
}

pub fn execute_record(
    runtime_dir: Option<PathBuf>,
    action: RecordAction,
) -> Result<Value, CliError> {
    let action = prepare_record_action(action)?;
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository(|repository| {
            let mut commands = RecordCommands::new(repository);
            match action {
                PreparedRecordAction::Get(definition_id) => {
                    Ok(json!({ "record": commands.get(&definition_id)? }))
                }
                PreparedRecordAction::Query(query) => {
                    Ok(json!({ "entries": commands.query(query)? }))
                }
                PreparedRecordAction::Set(command) => {
                    Ok(json!({ "record": commands.set_scalar(command)? }))
                }
                PreparedRecordAction::Increment(command) => {
                    Ok(json!({ "record": commands.increment_scalar(command)? }))
                }
                PreparedRecordAction::Correct(command) => {
                    Ok(json!({ "record": commands.correct_scalar(command)? }))
                }
                PreparedRecordAction::CreateEmptyCollection(command) => Ok(json!({
                    "record": commands.create_empty_collection(command)?
                })),
                PreparedRecordAction::CreateEmptyEvent(command) => {
                    Ok(json!({ "record": commands.create_empty_event(command)? }))
                }
                PreparedRecordAction::AddItem(command) => {
                    Ok(json!({ "record": commands.add_collection_item(command)? }))
                }
                PreparedRecordAction::CorrectItem(command) => Ok(json!({
                    "record": commands.correct_collection_item(command)?
                })),
                PreparedRecordAction::RemoveItem(command) => Ok(json!({
                    "record": commands.remove_collection_item(command)?
                })),
                PreparedRecordAction::AppendEvent(command) => {
                    Ok(json!({ "record": commands.append_event(command)? }))
                }
                PreparedRecordAction::CorrectEvent(command) => {
                    Ok(json!({ "record": commands.correct_event(command)? }))
                }
                PreparedRecordAction::DeleteEvent(command) => {
                    Ok(json!({ "record": commands.delete_event(command)? }))
                }
                PreparedRecordAction::Delete(definition_id) => {
                    commands.delete(&definition_id)?;
                    Ok(json!({ "deleted_definition_id": definition_id }))
                }
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Record))
}

fn prepare_record_action(action: RecordAction) -> Result<PreparedRecordAction, CliError> {
    match action {
        RecordAction::Get { definition_id } => Ok(PreparedRecordAction::Get(definition_id)),
        RecordAction::Query {
            definition_id,
            namespace,
            pack,
            kind,
            has_value,
        } => Ok(PreparedRecordAction::Query(QueryRecords {
            definition_id,
            namespace,
            pack_id: pack,
            kind: kind.map(Into::into),
            has_value,
        })),
        RecordAction::Set { file } => Ok(PreparedRecordAction::Set(parse_json_input(
            file.as_deref(),
            "set scalar",
        )?)),
        RecordAction::Increment { file } => Ok(PreparedRecordAction::Increment(parse_json_input(
            file.as_deref(),
            "increment scalar",
        )?)),
        RecordAction::Correct { file } => Ok(PreparedRecordAction::Correct(parse_json_input(
            file.as_deref(),
            "correct scalar",
        )?)),
        RecordAction::CreateEmptyCollection { definition_id } => Ok(
            PreparedRecordAction::CreateEmptyCollection(CreateEmptyRecord { definition_id }),
        ),
        RecordAction::CreateEmptyEvent { definition_id } => {
            Ok(PreparedRecordAction::CreateEmptyEvent(CreateEmptyRecord {
                definition_id,
            }))
        }
        RecordAction::AddItem { file } => Ok(PreparedRecordAction::AddItem(parse_json_input(
            file.as_deref(),
            "add collection item",
        )?)),
        RecordAction::CorrectItem { file } => Ok(PreparedRecordAction::CorrectItem(
            parse_json_input(file.as_deref(), "correct collection item")?,
        )),
        RecordAction::RemoveItem { file } => Ok(PreparedRecordAction::RemoveItem(
            parse_json_input(file.as_deref(), "remove collection item")?,
        )),
        RecordAction::AppendEvent { file } => Ok(PreparedRecordAction::AppendEvent(
            parse_json_input(file.as_deref(), "append event")?,
        )),
        RecordAction::CorrectEvent { file } => Ok(PreparedRecordAction::CorrectEvent(
            parse_json_input(file.as_deref(), "correct event")?,
        )),
        RecordAction::DeleteEvent { file } => Ok(PreparedRecordAction::DeleteEvent(
            parse_json_input(file.as_deref(), "delete event")?,
        )),
        RecordAction::Delete { definition_id } => Ok(PreparedRecordAction::Delete(definition_id)),
    }
}

pub(super) fn parse_json_input<T>(
    file: Option<&std::path::Path>,
    operation: &str,
) -> Result<T, CliError>
where
    T: serde::de::DeserializeOwned,
{
    let (input, source) = read_input(file, operation)?;
    serde_json::from_str(&input).map_err(|error| {
        CliError::invalid_command_input(
            operation,
            format!("invalid JSON: {error}"),
            json!({
                "source": source,
                "line": error.line(),
                "column": error.column()
            }),
        )
    })
}

fn read_input(
    file: Option<&std::path::Path>,
    operation: &str,
) -> Result<(String, String), CliError> {
    match file {
        Some(path) => std::fs::read_to_string(path)
            .map(|input| (input, path.display().to_string()))
            .map_err(|error| {
                CliError::invalid_command_input(
                    operation,
                    format!("failed to read command input: {error}"),
                    json!({ "source": path }),
                )
            }),
        None => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| {
                    CliError::invalid_command_input(
                        operation,
                        format!("failed to read stdin: {error}"),
                        json!({ "source": "stdin" }),
                    )
                })?;
            Ok((input, "stdin".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_lib::application::ArcanaRuntime;
    use arcana_lib::domain::{
        ArcanaRepository, ArcanaRepositoryTransaction, FieldDefinition, Pack, PackManifest,
        RecordDefinition, RecordDefinitionFile, ScalarRecordDefinition, StructuredRecordDefinition,
        ValueType, SCHEMA_VERSION,
    };
    use serde::Serialize;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn record_test_pack() -> Pack {
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
                        fields: BTreeMap::from([(
                            "kind".to_string(),
                            FieldDefinition {
                                value_type: ValueType::String,
                                required: true,
                                unit: None,
                            },
                        )]),
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
                        fields: BTreeMap::from([(
                            "title".to_string(),
                            FieldDefinition {
                                value_type: ValueType::String,
                                required: true,
                                unit: None,
                            },
                        )]),
                    }),
                ],
            }),
            dimensions: None,
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn write_command_file<T: Serialize>(directory: &Path, name: &str, value: &T) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        path
    }

    #[test]
    fn record_commands_cover_scalar_collection_event_query_and_delete() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        let runtime = ArcanaRuntime::new(&runtime_dir).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut transaction = repository.begin_transaction()?;
                transaction.put_pack(record_test_pack())?;
                transaction.set_pack_enabled("stats", true)?;
                transaction.commit()
            })
            .unwrap();

        let set_file = write_command_file(
            directory.path(),
            "set.json",
            &SetScalarRecord {
                definition_id: "stats.count".to_string(),
                value: json!(2),
                effective_at: None,
            },
        );
        execute_record(
            Some(runtime_dir.clone()),
            RecordAction::Set {
                file: Some(set_file),
            },
        )
        .unwrap();

        let increment_file = write_command_file(
            directory.path(),
            "increment.json",
            &IncrementScalarRecord {
                definition_id: "stats.count".to_string(),
                delta: json!(3),
                effective_at: None,
            },
        );
        let incremented = execute_record(
            Some(runtime_dir.clone()),
            RecordAction::Increment {
                file: Some(increment_file),
            },
        )
        .unwrap();
        assert_eq!(incremented["record"]["value"], 5);

        let queried = execute_record(
            Some(runtime_dir.clone()),
            RecordAction::Query {
                definition_id: None,
                namespace: Some("stats".to_string()),
                pack: Some("stats".to_string()),
                kind: Some(RecordKindArg::Scalar),
                has_value: Some(true),
            },
        )
        .unwrap();
        assert_eq!(queried["entries"].as_array().unwrap().len(), 1);

        execute_record(
            Some(runtime_dir.clone()),
            RecordAction::CreateEmptyCollection {
                definition_id: "stats.projects".to_string(),
            },
        )
        .unwrap();
        let add_item_file = write_command_file(
            directory.path(),
            "add-item.json",
            &AddCollectionItem {
                definition_id: "stats.projects".to_string(),
                item_id: "arcana".to_string(),
                fields: BTreeMap::from([("title".to_string(), json!("Arcana"))]),
            },
        );
        execute_record(
            Some(runtime_dir.clone()),
            RecordAction::AddItem {
                file: Some(add_item_file),
            },
        )
        .unwrap();

        execute_record(
            Some(runtime_dir.clone()),
            RecordAction::CreateEmptyEvent {
                definition_id: "stats.activities".to_string(),
            },
        )
        .unwrap();
        let append_event_file = write_command_file(
            directory.path(),
            "append-event.json",
            &AppendEvent {
                definition_id: "stats.activities".to_string(),
                event_id: "walk".to_string(),
                occurred_at: "2026-08-15T08:00:00+08:00".to_string(),
                fields: BTreeMap::from([("kind".to_string(), json!("walk"))]),
            },
        );
        execute_record(
            Some(runtime_dir.clone()),
            RecordAction::AppendEvent {
                file: Some(append_event_file),
            },
        )
        .unwrap();

        execute_record(
            Some(runtime_dir.clone()),
            RecordAction::Delete {
                definition_id: "stats.count".to_string(),
            },
        )
        .unwrap();
        let fetched = execute_record(
            Some(runtime_dir),
            RecordAction::Get {
                definition_id: "stats.count".to_string(),
            },
        )
        .unwrap();
        assert!(fetched["record"].is_null());
    }

    #[test]
    fn invalid_json_is_structured_before_runtime_access() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("invalid.json");
        std::fs::write(&input, "{not json").unwrap();
        let error = prepare_record_action(RecordAction::Set { file: Some(input) }).unwrap_err();
        assert_eq!(error.code, "invalid_command_input");
        assert_eq!(error.details["operation"], "set scalar");
        assert!(error.details["line"].as_u64().is_some());
    }
}
