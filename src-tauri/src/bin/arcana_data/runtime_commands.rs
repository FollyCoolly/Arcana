use super::contract::{CliError, RepositoryOperation};
use arcana_lib::application::ArcanaRuntime;
use arcana_lib::domain::SyncedRepositorySnapshot;
use arcana_lib::storage::settings::{expand_tilde, load_settings};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum JsonAction {
    /// Export SQLite data to a new directory without running Git
    Export {
        /// New output directory; an existing path is never overwritten
        #[arg(long, value_name = "DIRECTORY")]
        output: PathBuf,
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY")]
        runtime: Option<PathBuf>,
    },
    /// Create or replace SQLite data from a complete JSON directory without Git
    Import {
        /// Input directory containing arcana.json, packs/, and optional data files
        #[arg(long, value_name = "DIRECTORY")]
        input: PathBuf,
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY")]
        runtime: Option<PathBuf>,
    },
}

pub fn execute_init(runtime_dir: Option<PathBuf>) -> Result<Value, CliError> {
    let runtime = runtime_from_cli(runtime_dir)?;
    runtime
        .initialize()
        .map_err(|error| CliError::from_repository(error, RepositoryOperation::Initialize))?;
    Ok(json!({
        "operation": "init",
        "runtime_dir": runtime.runtime_dir(),
        "database": runtime.database_path()
    }))
}

pub fn execute_json(action: JsonAction) -> Result<Value, CliError> {
    match action {
        JsonAction::Export { output, runtime } => {
            let runtime = runtime_from_cli(runtime)?;
            let output = absolute_cli_path(output, "json export")?;
            let snapshot = runtime
                .export_json_to_new_directory(&output)
                .map_err(|error| {
                    CliError::from_repository(error, RepositoryOperation::JsonExport)
                })?;
            Ok(json_command_output(
                "export",
                runtime.runtime_dir(),
                &output,
                &snapshot,
            ))
        }
        JsonAction::Import { input, runtime } => {
            let runtime = runtime_from_cli(runtime)?;
            let input = absolute_cli_path(input, "json import")?;
            let snapshot = runtime
                .import_json_from_directory(&input)
                .map_err(|error| {
                    CliError::from_repository(error, RepositoryOperation::JsonImport)
                })?;
            Ok(json_command_output(
                "import",
                runtime.runtime_dir(),
                &input,
                &snapshot,
            ))
        }
    }
}

pub(super) fn runtime_from_cli(runtime_dir: Option<PathBuf>) -> Result<ArcanaRuntime, CliError> {
    match runtime_dir {
        Some(path) => {
            let absolute = absolute_cli_path(path, "runtime path")?;
            ArcanaRuntime::new(absolute)
                .map_err(|error| CliError::from_repository(error, RepositoryOperation::Initialize))
        }
        None => ArcanaRuntime::from_settings(&load_settings())
            .map_err(|error| CliError::from_repository(error, RepositoryOperation::Initialize)),
    }
}

fn absolute_cli_path(path: PathBuf, operation: &str) -> Result<PathBuf, CliError> {
    let path = path.to_str().map(expand_tilde).unwrap_or(path);
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|error| {
            CliError::invalid_command_input(
                operation,
                format!("cannot resolve relative path: {error}"),
                json!({}),
            )
        })
}

fn json_command_output(
    operation: &str,
    runtime_dir: &Path,
    directory: &Path,
    snapshot: &SyncedRepositorySnapshot,
) -> Value {
    let record_count = snapshot
        .records
        .values()
        .map(|file| file.records.len())
        .sum::<usize>();
    let achievement_state_count = snapshot
        .achievement_states
        .as_ref()
        .map(|file| file.states.len())
        .unwrap_or_default();
    let mission_count = snapshot
        .missions
        .as_ref()
        .map(|file| file.missions.len())
        .unwrap_or_default();
    let memory_count = snapshot
        .assistant_memory
        .as_ref()
        .map(|file| file.memories.len())
        .unwrap_or_default();
    let asset_count = snapshot
        .packs
        .values()
        .map(|pack| pack.assets.len())
        .sum::<usize>();
    json!({
        "operation": format!("json_{operation}"),
        "runtime_dir": runtime_dir,
        "directory": directory,
        "summary": {
            "packs": snapshot.packs.len(),
            "enabled_packs": snapshot.manifest.enabled_pack_ids.len(),
            "record_namespaces": snapshot.records.len(),
            "records": record_count,
            "achievement_states": achievement_state_count,
            "missions": mission_count,
            "assistant_memories": memory_count,
            "assets": asset_count
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_commands_export_and_import_without_git_files() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        let initialized = execute_init(Some(runtime_dir.clone())).unwrap();
        assert_eq!(initialized["operation"], "init");

        let json_dir = directory.path().join("json");
        let exported = execute_json(JsonAction::Export {
            output: json_dir.clone(),
            runtime: Some(runtime_dir),
        })
        .unwrap();
        assert_eq!(exported["operation"], "json_export");
        assert!(json_dir.join("arcana.json").is_file());
        assert!(!json_dir.join(".gitattributes").exists());

        let imported_runtime = directory.path().join("imported-runtime");
        let imported = execute_json(JsonAction::Import {
            input: json_dir,
            runtime: Some(imported_runtime.clone()),
        })
        .unwrap();
        assert_eq!(imported["operation"], "json_import");
        assert_eq!(imported["summary"]["enabled_packs"], 1);
        assert!(imported_runtime.join("arcana.sqlite3").is_file());
    }
}
