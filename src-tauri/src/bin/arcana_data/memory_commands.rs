use super::batch_commands::execute_mutation;
use super::contract::{CliError, RepositoryOperation};
use super::record_commands::parse_json_input;
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{
    AssistantMemoryTarget, CreateAssistantMemory, MemoryCommands, MutationOperation,
    QueryAssistantMemory, UpdateAssistantMemory,
};
use arcana_lib::domain::AssistantMemoryKind;
use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum MemoryAction {
    /// List synchronized AssistantMemory entries using exact filters
    List {
        #[arg(long)]
        memory_id: Option<String>,
        #[arg(long)]
        kind: Option<AssistantMemoryKindArg>,
    },
    /// Create AssistantMemory from JSON on stdin or --file
    Create {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Update AssistantMemory kind and content from JSON on stdin or --file
    Update {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Permanently delete AssistantMemory
    Delete { memory_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AssistantMemoryKindArg {
    Focus,
    Preference,
    Constraint,
    Habit,
    Summary,
    Reminder,
    Observation,
}

impl From<AssistantMemoryKindArg> for AssistantMemoryKind {
    fn from(value: AssistantMemoryKindArg) -> Self {
        match value {
            AssistantMemoryKindArg::Focus => Self::Focus,
            AssistantMemoryKindArg::Preference => Self::Preference,
            AssistantMemoryKindArg::Constraint => Self::Constraint,
            AssistantMemoryKindArg::Habit => Self::Habit,
            AssistantMemoryKindArg::Summary => Self::Summary,
            AssistantMemoryKindArg::Reminder => Self::Reminder,
            AssistantMemoryKindArg::Observation => Self::Observation,
        }
    }
}

enum PreparedMemoryAction {
    List(QueryAssistantMemory),
    Create(CreateAssistantMemory),
    Update(UpdateAssistantMemory),
    Delete(String),
}

pub fn execute_memory(
    runtime_dir: Option<PathBuf>,
    action: MemoryAction,
    dry_run: bool,
) -> Result<Value, CliError> {
    let action = prepare_memory_action(action)?;
    let action = match action {
        PreparedMemoryAction::Create(command) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MemoryCreate(command),
                RepositoryOperation::Memory,
                dry_run,
            )
        }
        PreparedMemoryAction::Update(command) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MemoryUpdate(command),
                RepositoryOperation::Memory,
                dry_run,
            )
        }
        PreparedMemoryAction::Delete(memory_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MemoryDelete(AssistantMemoryTarget { memory_id }),
                RepositoryOperation::Memory,
                dry_run,
            )
        }
        action @ PreparedMemoryAction::List(_) => action,
    };
    if dry_run {
        return Err(CliError::invalid_command_input(
            "list AssistantMemory",
            "--dry-run cannot be used with read-only commands",
            json!({}),
        ));
    }
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository(|repository| {
            let mut commands = MemoryCommands::new(repository);
            match action {
                PreparedMemoryAction::List(query) => {
                    Ok(json!({ "memories": commands.list(query)? }))
                }
                _ => unreachable!("mutations return before read dispatch"),
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Memory))
}

fn prepare_memory_action(action: MemoryAction) -> Result<PreparedMemoryAction, CliError> {
    match action {
        MemoryAction::List { memory_id, kind } => {
            Ok(PreparedMemoryAction::List(QueryAssistantMemory {
                memory_id,
                kind: kind.map(Into::into),
            }))
        }
        MemoryAction::Create { file } => Ok(PreparedMemoryAction::Create(parse_json_input(
            file.as_deref(),
            "create AssistantMemory",
        )?)),
        MemoryAction::Update { file } => Ok(PreparedMemoryAction::Update(parse_json_input(
            file.as_deref(),
            "update AssistantMemory",
        )?)),
        MemoryAction::Delete { memory_id } => Ok(PreparedMemoryAction::Delete(memory_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filters_map_to_typed_query() {
        let PreparedMemoryAction::List(query) = prepare_memory_action(MemoryAction::List {
            memory_id: Some("memory-id".to_string()),
            kind: Some(AssistantMemoryKindArg::Reminder),
        })
        .unwrap() else {
            panic!("expected AssistantMemory list query");
        };
        assert_eq!(query.memory_id.as_deref(), Some("memory-id"));
        assert_eq!(query.kind, Some(AssistantMemoryKind::Reminder));
    }
}
