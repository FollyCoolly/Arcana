use super::contract::{CliError, RepositoryOperation};
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::ContextCommands;
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ContextAction {
    /// Build a compact Agent context from one SQLite transaction snapshot
    Summary,
}

pub fn execute_context(
    runtime_dir: Option<PathBuf>,
    action: ContextAction,
) -> Result<Value, CliError> {
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository(|repository| {
            let mut commands = ContextCommands::new(repository);
            match action {
                ContextAction::Summary => Ok(json!(commands.summary()?)),
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Context))
}
