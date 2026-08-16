use super::contract::{CliError, RepositoryOperation};
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{QuerySkills, SkillCommands};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum SkillAction {
    /// List enabled Skill definitions with derived nodes, points, and levels
    List {
        /// Match one exact Skill ID
        #[arg(long)]
        skill_id: Option<String>,
        /// Match one Pack ID
        #[arg(long)]
        pack: Option<String>,
    },
}

pub fn execute_skill(runtime_dir: Option<PathBuf>, action: SkillAction) -> Result<Value, CliError> {
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository(|repository| {
            let mut commands = SkillCommands::new(repository);
            match action {
                SkillAction::List { skill_id, pack } => Ok(json!({
                    "skills": commands.list(QuerySkills {
                        skill_id,
                        pack_id: pack,
                    })?
                })),
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Skill))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_lib::application::ArcanaRuntime;

    #[test]
    fn empty_basic_runtime_lists_no_skills() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        ArcanaRuntime::new(&runtime_dir)
            .unwrap()
            .initialize()
            .unwrap();

        let listed = execute_skill(
            Some(runtime_dir),
            SkillAction::List {
                skill_id: None,
                pack: None,
            },
        )
        .unwrap();
        assert_eq!(listed["skills"], json!([]));
    }
}
