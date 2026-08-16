use super::batch_commands::execute_mutation;
use super::contract::{CliError, RepositoryOperation};
use super::record_commands::parse_json_input;
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{
    CreateMission, MissionCommands, MissionSuggestionTarget, MissionTarget, MutationOperation,
    QueryMissionSuggestions, QueryMissions, SuggestMission, UpdateMission,
};
use arcana_lib::domain::{MissionStatus, MissionSuggestionStatus};
use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum MissionAction {
    /// List accepted Missions, optionally using exact filters
    List {
        #[arg(long)]
        mission_id: Option<String>,
        #[arg(long)]
        status: Option<MissionStatusArg>,
        #[arg(long)]
        parent_id: Option<String>,
    },
    /// Create an active Mission from JSON on stdin or --file
    Create {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Replace a Mission's editable fields from JSON on stdin or --file
    Update {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Complete an active Mission using the current time
    Complete { mission_id: String },
    /// Archive an active or completed Mission
    Archive { mission_id: String },
    /// Permanently delete a Mission that has no children
    Delete { mission_id: String },
    /// List local MissionSuggestions using exact filters
    SuggestionList {
        #[arg(long)]
        suggestion_id: Option<String>,
        #[arg(long)]
        status: Option<MissionSuggestionStatusArg>,
    },
    /// Create a local pending MissionSuggestion from JSON on stdin or --file
    Suggest {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Atomically convert a local Suggestion into an active Mission
    Accept { suggestion_id: String },
    /// Retain a local Suggestion with rejected status
    Reject { suggestion_id: String },
    /// Permanently delete a local MissionSuggestion
    SuggestionDelete { suggestion_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MissionStatusArg {
    Active,
    Completed,
    Archived,
}

impl From<MissionStatusArg> for MissionStatus {
    fn from(value: MissionStatusArg) -> Self {
        match value {
            MissionStatusArg::Active => Self::Active,
            MissionStatusArg::Completed => Self::Completed,
            MissionStatusArg::Archived => Self::Archived,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MissionSuggestionStatusArg {
    Pending,
    Rejected,
}

impl From<MissionSuggestionStatusArg> for MissionSuggestionStatus {
    fn from(value: MissionSuggestionStatusArg) -> Self {
        match value {
            MissionSuggestionStatusArg::Pending => Self::Pending,
            MissionSuggestionStatusArg::Rejected => Self::Rejected,
        }
    }
}

enum PreparedMissionAction {
    List(QueryMissions),
    Create(CreateMission),
    Update(UpdateMission),
    Complete(String),
    Archive(String),
    Delete(String),
    SuggestionList(QueryMissionSuggestions),
    Suggest(SuggestMission),
    Accept(String),
    Reject(String),
    SuggestionDelete(String),
}

pub fn execute_mission(
    runtime_dir: Option<PathBuf>,
    action: MissionAction,
    dry_run: bool,
) -> Result<Value, CliError> {
    let action = prepare_mission_action(action)?;
    let action = match action {
        PreparedMissionAction::Create(command) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionCreate(command),
                RepositoryOperation::Mission,
                dry_run,
            )
        }
        PreparedMissionAction::Update(command) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionUpdate(command),
                RepositoryOperation::Mission,
                dry_run,
            )
        }
        PreparedMissionAction::Complete(mission_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionComplete(MissionTarget { mission_id }),
                RepositoryOperation::Mission,
                dry_run,
            )
        }
        PreparedMissionAction::Archive(mission_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionArchive(MissionTarget { mission_id }),
                RepositoryOperation::Mission,
                dry_run,
            )
        }
        PreparedMissionAction::Delete(mission_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionDelete(MissionTarget { mission_id }),
                RepositoryOperation::Mission,
                dry_run,
            )
        }
        PreparedMissionAction::Suggest(command) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionSuggest(command),
                RepositoryOperation::MissionSuggestion,
                dry_run,
            )
        }
        PreparedMissionAction::Accept(suggestion_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionAccept(MissionSuggestionTarget { suggestion_id }),
                RepositoryOperation::MissionSuggestion,
                dry_run,
            )
        }
        PreparedMissionAction::Reject(suggestion_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionReject(MissionSuggestionTarget { suggestion_id }),
                RepositoryOperation::MissionSuggestion,
                dry_run,
            )
        }
        PreparedMissionAction::SuggestionDelete(suggestion_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::MissionSuggestionDelete(MissionSuggestionTarget {
                    suggestion_id,
                }),
                RepositoryOperation::MissionSuggestion,
                dry_run,
            )
        }
        action @ (PreparedMissionAction::List(_) | PreparedMissionAction::SuggestionList(_)) => {
            action
        }
    };
    if dry_run {
        return Err(CliError::invalid_command_input(
            "read Missions",
            "--dry-run cannot be used with read-only commands",
            json!({}),
        ));
    }
    let operation = match &action {
        PreparedMissionAction::SuggestionList(_) => RepositoryOperation::MissionSuggestion,
        _ => RepositoryOperation::Mission,
    };
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository(|repository| {
            let mut commands = MissionCommands::new(repository);
            match action {
                PreparedMissionAction::List(query) => {
                    Ok(json!({ "missions": commands.list(query)? }))
                }
                PreparedMissionAction::SuggestionList(query) => {
                    Ok(json!({ "suggestions": commands.list_suggestions(query)? }))
                }
                _ => unreachable!("mutations return before read dispatch"),
            }
        })
        .map_err(|error| CliError::from_repository(error, operation))
}

fn prepare_mission_action(action: MissionAction) -> Result<PreparedMissionAction, CliError> {
    match action {
        MissionAction::List {
            mission_id,
            status,
            parent_id,
        } => Ok(PreparedMissionAction::List(QueryMissions {
            mission_id,
            status: status.map(Into::into),
            parent_id,
        })),
        MissionAction::Create { file } => Ok(PreparedMissionAction::Create(parse_json_input(
            file.as_deref(),
            "create Mission",
        )?)),
        MissionAction::Update { file } => Ok(PreparedMissionAction::Update(parse_json_input(
            file.as_deref(),
            "update Mission",
        )?)),
        MissionAction::Complete { mission_id } => Ok(PreparedMissionAction::Complete(mission_id)),
        MissionAction::Archive { mission_id } => Ok(PreparedMissionAction::Archive(mission_id)),
        MissionAction::Delete { mission_id } => Ok(PreparedMissionAction::Delete(mission_id)),
        MissionAction::SuggestionList {
            suggestion_id,
            status,
        } => Ok(PreparedMissionAction::SuggestionList(
            QueryMissionSuggestions {
                suggestion_id,
                status: status.map(Into::into),
            },
        )),
        MissionAction::Suggest { file } => Ok(PreparedMissionAction::Suggest(parse_json_input(
            file.as_deref(),
            "suggest Mission",
        )?)),
        MissionAction::Accept { suggestion_id } => Ok(PreparedMissionAction::Accept(suggestion_id)),
        MissionAction::Reject { suggestion_id } => Ok(PreparedMissionAction::Reject(suggestion_id)),
        MissionAction::SuggestionDelete { suggestion_id } => {
            Ok(PreparedMissionAction::SuggestionDelete(suggestion_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filters_map_to_typed_query() {
        let PreparedMissionAction::List(query) = prepare_mission_action(MissionAction::List {
            mission_id: Some("mission-id".to_string()),
            status: Some(MissionStatusArg::Completed),
            parent_id: Some("parent-id".to_string()),
        })
        .unwrap() else {
            panic!("expected Mission list query");
        };
        assert_eq!(query.mission_id.as_deref(), Some("mission-id"));
        assert_eq!(query.status, Some(MissionStatus::Completed));
        assert_eq!(query.parent_id.as_deref(), Some("parent-id"));
    }

    #[test]
    fn suggestion_filters_map_to_typed_query() {
        let PreparedMissionAction::SuggestionList(query) =
            prepare_mission_action(MissionAction::SuggestionList {
                suggestion_id: Some("suggestion-id".to_string()),
                status: Some(MissionSuggestionStatusArg::Rejected),
            })
            .unwrap()
        else {
            panic!("expected MissionSuggestion list query");
        };
        assert_eq!(query.suggestion_id.as_deref(), Some("suggestion-id"));
        assert_eq!(query.status, Some(MissionSuggestionStatus::Rejected));
    }
}
