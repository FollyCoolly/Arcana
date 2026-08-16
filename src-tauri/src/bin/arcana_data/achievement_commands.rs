use super::contract::{CliError, RepositoryOperation};
use super::record_commands::parse_json_input;
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{AchievementCommands, QueryAchievements, SetAchievementState};
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
) -> Result<Value, CliError> {
    let action = prepare_achievement_action(action)?;
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
                PreparedAchievementAction::StateSet(command) => Ok(json!({
                    "achievement_state": commands.set_state(command)?
                })),
                PreparedAchievementAction::StateRevoke(achievement_id) => Ok(json!({
                    "achievement_state": commands.revoke_state(&achievement_id)?
                })),
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
