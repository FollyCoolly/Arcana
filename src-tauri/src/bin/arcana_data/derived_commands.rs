use super::contract::{CliError, RepositoryOperation};
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::DerivedValueCommands;
use chrono::{Local, NaiveDate};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum DerivedAction {
    /// List and evaluate all DerivedValues from enabled Packs
    List {
        /// Evaluation date in YYYY-MM-DD form; defaults to today
        #[arg(long, value_name = "DATE")]
        as_of: Option<String>,
    },
    /// Evaluate one DerivedValue by exact ID
    Evaluate {
        /// Exact DerivedValue ID, for example identity.game_days
        id: String,
        /// Evaluation date in YYYY-MM-DD form; defaults to today
        #[arg(long, value_name = "DATE")]
        as_of: Option<String>,
    },
}

pub fn execute_derived(
    runtime_dir: Option<PathBuf>,
    action: DerivedAction,
) -> Result<Value, CliError> {
    let action = match action {
        DerivedAction::List { as_of } => PreparedDerivedAction::List(parse_as_of(as_of)?),
        DerivedAction::Evaluate { id, as_of } => {
            PreparedDerivedAction::Evaluate(id, parse_as_of(as_of)?)
        }
    };
    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    runtime
        .with_repository(|repository| {
            let commands = DerivedValueCommands::new(repository);
            match action {
                PreparedDerivedAction::List(as_of_date) => Ok(json!({
                    "values": commands.list_on(as_of_date)?
                })),
                PreparedDerivedAction::Evaluate(id, as_of_date) => {
                    Ok(json!(commands.evaluate_on(&id, as_of_date)?))
                }
            }
        })
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::DerivedValue))
}

enum PreparedDerivedAction {
    List(NaiveDate),
    Evaluate(String, NaiveDate),
}

fn parse_as_of(value: Option<String>) -> Result<NaiveDate, CliError> {
    match value {
        Some(value) => NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
            CliError::invalid_command_input(
                "evaluate DerivedValue",
                "--as-of must be a valid YYYY-MM-DD date",
                json!({ "as_of": value }),
            )
        }),
        None => Ok(Local::now().date_naive()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_lib::application::{ArcanaRuntime, RecordCommands, SetScalarRecord};
    use serde_json::json;

    #[test]
    fn evaluates_basic_game_days_on_an_explicit_date() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        let runtime = ArcanaRuntime::new(&runtime_dir).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                RecordCommands::new(repository).set_scalar(SetScalarRecord {
                    definition_id: "identity.birth_date".to_string(),
                    value: json!("2026-08-01"),
                    effective_at: None,
                })?;
                Ok(())
            })
            .unwrap();

        let value = execute_derived(
            Some(runtime_dir),
            DerivedAction::Evaluate {
                id: "identity.game_days".to_string(),
                as_of: Some("2026-08-17".to_string()),
            },
        )
        .unwrap();
        assert_eq!(value["value"], 16.0);
        assert_eq!(value["as_of_date"], "2026-08-17");
    }
}
