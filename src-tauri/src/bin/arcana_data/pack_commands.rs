use super::contract::{CliError, RepositoryOperation};
use super::record_commands::parse_json_input;
use super::runtime_commands::runtime_from_cli;
use arcana_lib::application::{PackCommands, PackContent};
use clap::Subcommand;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PackAction {
    /// List every installed Pack and its enabled state
    List,
    /// Show one Pack's structured content and asset metadata
    Show { pack_id: String },
    /// Print a minimal valid Pack content document without opening SQLite
    Scaffold {
        pack_id: String,
        /// Human-readable Pack name
        #[arg(long)]
        name: String,
    },
    /// Validate Pack content against the current repository without writing
    Validate {
        /// JSON Pack content; reads stdin when omitted
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Insert or replace structured Pack content while preserving assets
    Write {
        /// JSON Pack content; reads stdin when omitted
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
    /// Import or replace one Pack asset from a local file
    AssetPut {
        pack_id: String,
        /// Portable path inside the Pack, for example assets/cards/cooking.webp
        asset_path: String,
        /// Local file whose bytes will be stored in SQLite
        #[arg(long, value_name = "FILE")]
        file: PathBuf,
    },
    /// Delete one Pack asset; referenced card images cannot be deleted
    AssetDelete { pack_id: String, asset_path: String },
    /// Enable a Pack without enabling its parent
    Enable { pack_id: String },
    /// Disable a Pack without disabling its children
    Disable { pack_id: String },
}

enum PreparedPackAction {
    List,
    Show(String),
    Scaffold(PackContent),
    Validate(PackContent),
    Write(PackContent),
    AssetPut {
        pack_id: String,
        asset_path: String,
        content: Vec<u8>,
    },
    AssetDelete {
        pack_id: String,
        asset_path: String,
    },
    Enable(String),
    Disable(String),
}

pub fn execute_pack(runtime_dir: Option<PathBuf>, action: PackAction) -> Result<Value, CliError> {
    let action = prepare_pack_action(action)?;
    if let PreparedPackAction::Scaffold(content) = action {
        return serde_json::to_value(content).map_err(|error| {
            CliError::invalid_command_input(
                "scaffold Pack",
                format!("failed to serialize scaffold: {error}"),
                json!({}),
            )
        });
    }

    let runtime = runtime_from_cli(runtime_dir)?;
    if !runtime.database_path().exists() {
        return Err(CliError::runtime_not_initialized(&runtime.database_path()));
    }
    let operation = match &action {
        PreparedPackAction::AssetPut { .. } | PreparedPackAction::AssetDelete { .. } => {
            RepositoryOperation::PackAsset
        }
        _ => RepositoryOperation::Pack,
    };
    runtime
        .with_repository(|repository| {
            let mut commands = PackCommands::new(repository);
            match action {
                PreparedPackAction::List => Ok(json!({ "packs": commands.list()? })),
                PreparedPackAction::Show(pack_id) => {
                    Ok(json!({ "pack": commands.show(&pack_id)? }))
                }
                PreparedPackAction::Validate(content) => {
                    Ok(json!({ "validation": commands.validate(content)? }))
                }
                PreparedPackAction::Write(content) => {
                    Ok(json!({ "pack": commands.write(content)? }))
                }
                PreparedPackAction::AssetPut {
                    pack_id,
                    asset_path,
                    content,
                } => Ok(json!({
                    "pack_id": pack_id,
                    "asset": commands.put_asset(&pack_id, asset_path, content)?
                })),
                PreparedPackAction::AssetDelete {
                    pack_id,
                    asset_path,
                } => {
                    commands.delete_asset(&pack_id, &asset_path)?;
                    Ok(json!({
                        "pack_id": pack_id,
                        "deleted_asset_path": asset_path
                    }))
                }
                PreparedPackAction::Enable(pack_id) => {
                    Ok(json!({ "pack": commands.set_enabled(&pack_id, true)? }))
                }
                PreparedPackAction::Disable(pack_id) => {
                    Ok(json!({ "pack": commands.set_enabled(&pack_id, false)? }))
                }
                PreparedPackAction::Scaffold(_) => unreachable!("handled before runtime access"),
            }
        })
        .map_err(|error| CliError::from_repository(error, operation))
}

fn prepare_pack_action(action: PackAction) -> Result<PreparedPackAction, CliError> {
    match action {
        PackAction::List => Ok(PreparedPackAction::List),
        PackAction::Show { pack_id } => Ok(PreparedPackAction::Show(pack_id)),
        PackAction::Scaffold { pack_id, name } => PackContent::scaffold(pack_id, name)
            .map(PreparedPackAction::Scaffold)
            .map_err(|error| CliError::from_repository(error, RepositoryOperation::Pack)),
        PackAction::Validate { file } => Ok(PreparedPackAction::Validate(parse_json_input(
            file.as_deref(),
            "validate Pack",
        )?)),
        PackAction::Write { file } => Ok(PreparedPackAction::Write(parse_json_input(
            file.as_deref(),
            "write Pack",
        )?)),
        PackAction::AssetPut {
            pack_id,
            asset_path,
            file,
        } => {
            let content = std::fs::read(&file).map_err(|error| {
                CliError::invalid_command_input(
                    "put Pack asset",
                    format!("failed to read asset file: {error}"),
                    json!({ "source": file }),
                )
            })?;
            Ok(PreparedPackAction::AssetPut {
                pack_id,
                asset_path,
                content,
            })
        }
        PackAction::AssetDelete {
            pack_id,
            asset_path,
        } => Ok(PreparedPackAction::AssetDelete {
            pack_id,
            asset_path,
        }),
        PackAction::Enable { pack_id } => Ok(PreparedPackAction::Enable(pack_id)),
        PackAction::Disable { pack_id } => Ok(PreparedPackAction::Disable(pack_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arcana_lib::application::ArcanaRuntime;

    #[test]
    fn scaffold_does_not_require_initialized_runtime() {
        let output = execute_pack(
            None,
            PackAction::Scaffold {
                pack_id: "cooking".to_string(),
                name: "Cooking".to_string(),
            },
        )
        .unwrap();
        assert_eq!(output["manifest"]["schema_version"], 1);
        assert_eq!(output["manifest"]["id"], "cooking");
    }

    #[test]
    fn commands_write_validate_enable_and_manage_assets() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        ArcanaRuntime::new(&runtime_dir)
            .unwrap()
            .initialize()
            .unwrap();
        let content = PackContent::scaffold("cooking".to_string(), "Cooking".to_string()).unwrap();
        let content_file = directory.path().join("pack.json");
        std::fs::write(&content_file, serde_json::to_vec(&content).unwrap()).unwrap();

        let validated = execute_pack(
            Some(runtime_dir.clone()),
            PackAction::Validate {
                file: Some(content_file.clone()),
            },
        )
        .unwrap();
        assert_eq!(validated["validation"]["valid"], true);

        execute_pack(
            Some(runtime_dir.clone()),
            PackAction::Write {
                file: Some(content_file),
            },
        )
        .unwrap();
        let asset_file = directory.path().join("note.txt");
        std::fs::write(&asset_file, "hello").unwrap();
        execute_pack(
            Some(runtime_dir.clone()),
            PackAction::AssetPut {
                pack_id: "cooking".to_string(),
                asset_path: "assets/note.txt".to_string(),
                file: asset_file,
            },
        )
        .unwrap();
        assert_eq!(
            execute_pack(
                Some(runtime_dir.clone()),
                PackAction::Enable {
                    pack_id: "cooking".to_string(),
                },
            )
            .unwrap()["pack"]["enabled"],
            true
        );
        execute_pack(
            Some(runtime_dir.clone()),
            PackAction::AssetDelete {
                pack_id: "cooking".to_string(),
                asset_path: "assets/note.txt".to_string(),
            },
        )
        .unwrap();
        let shown = execute_pack(
            Some(runtime_dir),
            PackAction::Show {
                pack_id: "cooking".to_string(),
            },
        )
        .unwrap();
        assert!(shown["pack"]["assets"].as_array().unwrap().is_empty());
    }
}
