use super::batch_commands::execute_mutation;
use super::contract::{CliError, RepositoryOperation};
use super::record_commands::parse_json_input;
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{
    AchievementCommands, AchievementTarget, MutationOperation, QueryAchievements,
    SetAchievementState,
};
use arcana_lib::domain::AchievementStatus;
use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum AchievementAction {
    /// List enabled Achievement definitions plus unresolved user states
    List {
        /// Match one exact Achievement ID
        #[arg(long)]
        achievement_id: Option<String>,
        /// Match one Pack ID
        #[arg(long)]
        pack: Option<String>,
        /// Match one persisted user state
        #[arg(long)]
        status: Option<AchievementStatusArg>,
        /// Match Achievements routed from this RecordDefinition
        #[arg(long)]
        related_record_definition_id: Option<String>,
    },
    /// Set tracked or achieved state from JSON on stdin or --file
    StateSet {
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Explicitly revoke any state, even when its definition is unavailable
    StateRevoke { achievement_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AchievementStatusArg {
    Tracked,
    Achieved,
}

impl From<AchievementStatusArg> for AchievementStatus {
    fn from(value: AchievementStatusArg) -> Self {
        match value {
            AchievementStatusArg::Tracked => Self::Tracked,
            AchievementStatusArg::Achieved => Self::Achieved,
        }
    }
}

enum PreparedAchievementAction {
    List(QueryAchievements),
    StateSet(SetAchievementState),
    StateRevoke(String),
}

pub fn execute_achievement(
    runtime_dir: Option<PathBuf>,
    action: AchievementAction,
    dry_run: bool,
) -> Result<Value, CliError> {
    let action = prepare_achievement_action(action)?;
    let action = match action {
        PreparedAchievementAction::StateSet(command) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::AchievementStateSet(command),
                RepositoryOperation::Achievement,
                dry_run,
            )
        }
        PreparedAchievementAction::StateRevoke(achievement_id) => {
            return execute_mutation(
                runtime_dir,
                MutationOperation::AchievementStateRevoke(AchievementTarget { achievement_id }),
                RepositoryOperation::Achievement,
                dry_run,
            )
        }
        action @ PreparedAchievementAction::List(_) => action,
    };
    if dry_run {
        return Err(CliError::invalid_command_input(
            "list Achievements",
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
            let mut commands = AchievementCommands::new(repository);
            match action {
                PreparedAchievementAction::List(query) => {
                    Ok(json!({ "achievements": commands.list(query)? }))
                }
                _ => unreachable!("mutations return before read dispatch"),
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Achievement))
}

fn prepare_achievement_action(
    action: AchievementAction,
) -> Result<PreparedAchievementAction, CliError> {
    match action {
        AchievementAction::List {
            achievement_id,
            pack,
            status,
            related_record_definition_id,
        } => Ok(PreparedAchievementAction::List(QueryAchievements {
            achievement_id,
            pack_id: pack,
            status: status.map(Into::into),
            related_record_definition_id,
        })),
        AchievementAction::StateSet { file } => Ok(PreparedAchievementAction::StateSet(
            parse_json_input(file.as_deref(), "set Achievement state")?,
        )),
        AchievementAction::StateRevoke { achievement_id } => {
            Ok(PreparedAchievementAction::StateRevoke(achievement_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_filters_map_to_typed_query() {
        let PreparedAchievementAction::List(query) =
            prepare_achievement_action(AchievementAction::List {
                achievement_id: Some("cooking::first_dish".to_string()),
                pack: Some("cooking".to_string()),
                status: Some(AchievementStatusArg::Achieved),
                related_record_definition_id: Some("cooking.dish_count".to_string()),
            })
            .unwrap()
        else {
            panic!("expected list query");
        };
        assert_eq!(query.pack_id.as_deref(), Some("cooking"));
        assert_eq!(query.status, Some(AchievementStatus::Achieved));
    }
}
