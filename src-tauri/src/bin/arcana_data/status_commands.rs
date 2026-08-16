use super::batch_commands::execute_mutation;
use super::contract::{CliError, RepositoryOperation};
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{
    MutationOperation, StatusCommands, StatusPositionInput, StatusSelectionInput,
};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum StatusAction {
    /// List Dimensions from enabled Packs and all five local selections
    ListDimensions,
    /// Evaluate one Dimension, or every enabled Dimension when omitted
    Evaluate {
        /// Exact Dimension ID, for example fitness::physical
        dimension_id: Option<String>,
    },
    /// Set or explicitly clear one local Status display position
    Select {
        /// Display position from 0 to 4
        position: u8,
        /// Enabled Dimension ID; omit only together with --clear
        dimension_id: Option<String>,
        /// Clear the selected position instead of assigning a Dimension
        #[arg(long)]
        clear: bool,
    },
}

#[derive(Debug)]
enum PreparedStatusAction {
    ListDimensions,
    Evaluate(Option<String>),
    Select { position: u8, dimension_id: String },
    Clear { position: u8 },
}

pub fn execute_status(
    runtime_dir: Option<PathBuf>,
    action: StatusAction,
    dry_run: bool,
) -> Result<Value, CliError> {
    let action = prepare_status_action(action)?;
    let action = match action {
        PreparedStatusAction::Select {
            position,
            dimension_id,
        } => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::StatusSelect(StatusSelectionInput {
                    position,
                    dimension_id,
                }),
                RepositoryOperation::Status,
                dry_run,
            )
        }
        PreparedStatusAction::Clear { position } => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::StatusClear(StatusPositionInput { position }),
                RepositoryOperation::Status,
                dry_run,
            )
        }
        action @ (PreparedStatusAction::ListDimensions | PreparedStatusAction::Evaluate(_)) => {
            action
        }
    };
    if dry_run {
        return Err(CliError::invalid_command_input(
            "read Status",
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
            let mut commands = StatusCommands::new(repository);
            match action {
                PreparedStatusAction::ListDimensions => Ok(json!(commands.list_dimensions()?)),
                PreparedStatusAction::Evaluate(dimension_id) => Ok(json!({
                    "evaluations": commands.evaluate(dimension_id.as_deref())?
                })),
                _ => unreachable!("mutations return before read dispatch"),
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Status))
}

fn prepare_status_action(action: StatusAction) -> Result<PreparedStatusAction, CliError> {
    match action {
        StatusAction::ListDimensions => Ok(PreparedStatusAction::ListDimensions),
        StatusAction::Evaluate { dimension_id } => Ok(PreparedStatusAction::Evaluate(dimension_id)),
        StatusAction::Select {
            position,
            dimension_id,
            clear,
        } => match (dimension_id, clear) {
            (Some(dimension_id), false) => Ok(PreparedStatusAction::Select {
                position,
                dimension_id,
            }),
            (None, true) => Ok(PreparedStatusAction::Clear { position }),
            (Some(_), true) => Err(CliError::invalid_command_input(
                "select Status Dimension",
                "dimension_id and --clear cannot be used together",
                json!({ "position": position }),
            )),
            (None, false) => Err(CliError::invalid_command_input(
                "select Status Dimension",
                "dimension_id is required unless --clear is used",
                json!({ "position": position }),
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_lib::application::ArcanaRuntime;

    #[test]
    fn empty_basic_runtime_lists_and_evaluates_no_dimensions() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        ArcanaRuntime::new(&runtime_dir)
            .unwrap()
            .initialize()
            .unwrap();

        let listed = execute_status(
            Some(runtime_dir.clone()),
            StatusAction::ListDimensions,
            false,
        )
        .unwrap();
        assert_eq!(listed["dimensions"], json!([]));
        assert_eq!(listed["selections"], json!([]));
        let evaluated = execute_status(
            Some(runtime_dir),
            StatusAction::Evaluate { dimension_id: None },
            false,
        )
        .unwrap();
        assert_eq!(evaluated["evaluations"], json!([]));
    }

    #[test]
    fn select_requires_exactly_one_assignment_mode() {
        let missing = prepare_status_action(StatusAction::Select {
            position: 0,
            dimension_id: None,
            clear: false,
        })
        .unwrap_err();
        assert_eq!(missing.code, "invalid_command_input");

        let conflicting = prepare_status_action(StatusAction::Select {
            position: 0,
            dimension_id: Some("fitness::physical".to_string()),
            clear: true,
        })
        .unwrap_err();
        assert_eq!(conflicting.code, "invalid_command_input");
    }
}
