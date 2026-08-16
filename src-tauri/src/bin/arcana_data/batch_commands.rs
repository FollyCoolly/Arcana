use super::contract::{CliError, RepositoryOperation};
use super::record_commands::parse_json_input;
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{MutationCommands, MutationOperation};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum BatchAction {
    /// Apply every operation in one transaction, or roll all of them back
    Apply {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

pub fn execute_batch(
    runtime_dir: Option<PathBuf>,
    action: BatchAction,
    dry_run: bool,
) -> Result<Value, CliError> {
    let batch = match action {
        BatchAction::Apply { file } => parse_json_input(file.as_deref(), "apply mutation batch")?,
    };
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository_result(|repository| {
            MutationCommands::new(repository).apply_batch(batch, dry_run)
        })
        .map(|result| json!(result))
        .map_err(CliError::from_batch)
}

pub(super) fn execute_mutation(
    runtime_dir: Option<PathBuf>,
    operation: MutationOperation,
    repository_operation: RepositoryOperation,
    dry_run: bool,
) -> Result<Value, CliError> {
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    let mut result = runtime
        .with_repository(|repository| {
            MutationCommands::new(repository).apply_one(operation, dry_run)
        })
        .map_err(|error| CliError::from_repository(error, repository_operation))?;
    if dry_run {
        result
            .as_object_mut()
            .expect("mutation results are JSON objects")
            .insert("dry_run".to_string(), json!(true));
    }
    Ok(result)
}
