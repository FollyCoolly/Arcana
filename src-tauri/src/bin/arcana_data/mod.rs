mod achievement_commands;
mod batch_commands;
mod context_commands;
mod contract;
mod memory_commands;
mod mission_commands;
mod pack_commands;
mod record_commands;
mod runtime_commands;
mod skill_commands;
mod status_commands;

use achievement_commands::{execute_achievement, AchievementAction};
use batch_commands::{execute_batch, BatchAction};
use clap::{error::ErrorKind, Parser, Subcommand};
use context_commands::{execute_context, ContextAction};
use contract::{capabilities, render_json, CliError, EXIT_SUCCESS};
use memory_commands::{execute_memory, MemoryAction};
use mission_commands::{execute_mission, MissionAction};
use pack_commands::{execute_pack, PackAction};
use record_commands::{execute_record, RecordAction};
use runtime_commands::{execute_init, execute_json, JsonAction};
use serde_json::Value;
use skill_commands::{execute_skill, SkillAction};
use status_commands::{execute_status, StatusAction};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "arcana-data", about = "Arcana SQLite data operations CLI")]
struct Cli {
    /// Output compact JSON instead of pretty JSON
    #[arg(long, global = true)]
    compact: bool,

    /// Validate mutations against current data and roll the transaction back
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Report the stable CLI and data schema capabilities without opening SQLite
    Capabilities,
    /// Initialize a fresh SQLite runtime with the basic Pack
    Init {
        /// Runtime directory that will contain arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY")]
        runtime: Option<PathBuf>,
    },
    /// Read a compact, consistent Agent context from the SQLite runtime
    Context {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: ContextAction,
    },
    /// Apply multiple user-state mutations in one atomic transaction
    Batch {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: BatchAction,
    },
    /// Read and update Records in the SQLite runtime
    Record {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: RecordAction,
    },
    /// Inspect, validate, and update Packs in the SQLite runtime
    Pack {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: PackAction,
    },
    /// List and evaluate Status Dimensions and manage five local display slots
    Status {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: StatusAction,
    },
    /// Query Achievement definitions and update minimal user states
    Achievement {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: AchievementAction,
    },
    /// List Arcana Skills with derived node, point, and level state
    Skill {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Manage accepted Missions and local MissionSuggestions
    Mission {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: MissionAction,
    },
    /// Manage synchronized AssistantMemory entries
    Memory {
        /// Runtime directory containing arcana.sqlite3
        #[arg(long, value_name = "DIRECTORY", global = true)]
        runtime: Option<PathBuf>,
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Convert between SQLite and a canonical JSON directory without Git
    Json {
        #[command(subcommand)]
        action: JsonAction,
    },
}

pub fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return EXIT_SUCCESS;
        }
        Err(error) => {
            let error = CliError::invalid_invocation(error.to_string());
            eprintln!(
                "{}",
                render_json(&serde_json::to_value(&error).unwrap(), false)
            );
            return error.exit_code();
        }
    };

    match execute(cli.command, cli.dry_run) {
        Ok(value) => {
            println!("{}", render_json(&value, cli.compact));
            EXIT_SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                render_json(&serde_json::to_value(&error).unwrap(), cli.compact)
            );
            error.exit_code()
        }
    }
}

fn execute(command: Commands, dry_run: bool) -> Result<Value, CliError> {
    match command {
        Commands::Capabilities => without_dry_run(dry_run, "capabilities", || Ok(capabilities())),
        Commands::Init { runtime } => without_dry_run(dry_run, "init", || execute_init(runtime)),
        Commands::Context { runtime, action } => {
            without_dry_run(dry_run, "context", || execute_context(runtime, action))
        }
        Commands::Batch { runtime, action } => execute_batch(runtime, action, dry_run),
        Commands::Record { runtime, action } => execute_record(runtime, action, dry_run),
        Commands::Pack { runtime, action } => {
            without_dry_run(dry_run, "pack", || execute_pack(runtime, action))
        }
        Commands::Status { runtime, action } => execute_status(runtime, action, dry_run),
        Commands::Achievement { runtime, action } => execute_achievement(runtime, action, dry_run),
        Commands::Skill { runtime, action } => {
            without_dry_run(dry_run, "skill", || execute_skill(runtime, action))
        }
        Commands::Mission { runtime, action } => execute_mission(runtime, action, dry_run),
        Commands::Memory { runtime, action } => execute_memory(runtime, action, dry_run),
        Commands::Json { action } => without_dry_run(dry_run, "json", || execute_json(action)),
    }
}

fn without_dry_run(
    dry_run: bool,
    command: &str,
    execute: impl FnOnce() -> Result<Value, CliError>,
) -> Result<Value, CliError> {
    if dry_run {
        return Err(CliError::invalid_command_input(
            command,
            "--dry-run is only supported by user-state mutations and batch apply",
            serde_json::json!({ "command": command }),
        ));
    }
    execute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_new_sqlite_command_families_are_exposed() {
        for removed in ["read", "changelog"] {
            assert!(Cli::try_parse_from(["arcana-data", removed]).is_err());
        }
        for incomplete in ["context", "record", "mission", "memory"] {
            assert!(Cli::try_parse_from(["arcana-data", incomplete]).is_err());
        }
        for retained in ["capabilities", "init"] {
            assert!(Cli::try_parse_from(["arcana-data", retained]).is_ok());
        }
    }

    #[test]
    fn parses_every_pack_command() {
        let commands: &[&[&str]] = &[
            &["pack", "list"],
            &["pack", "show", "cooking"],
            &["pack", "scaffold", "cooking", "--name", "Cooking"],
            &["pack", "validate", "--file", "pack.json"],
            &["pack", "write", "--file", "pack.json"],
            &[
                "pack",
                "asset-put",
                "cooking",
                "assets/card.webp",
                "--file",
                "card.webp",
            ],
            &["pack", "asset-delete", "cooking", "assets/card.webp"],
            &["pack", "enable", "cooking"],
            &["pack", "disable", "cooking"],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Pack { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }

    #[test]
    fn parses_context_summary() {
        assert!(matches!(
            Cli::try_parse_from(["arcana-data", "context", "summary"])
                .unwrap()
                .command,
            Commands::Context { .. }
        ));
    }

    #[test]
    fn parses_batch_apply_and_global_dry_run() {
        let cli = Cli::try_parse_from([
            "arcana-data",
            "batch",
            "apply",
            "--file",
            "batch.json",
            "--dry-run",
        ])
        .unwrap();
        assert!(cli.dry_run);
        assert!(matches!(cli.command, Commands::Batch { .. }));
    }

    #[test]
    fn parses_every_status_command() {
        let commands: &[&[&str]] = &[
            &["status", "list-dimensions"],
            &["status", "evaluate"],
            &["status", "evaluate", "fitness::physical"],
            &["status", "select", "0", "fitness::physical"],
            &["status", "select", "0", "--clear"],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Status { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }

    #[test]
    fn parses_every_achievement_command() {
        let commands: &[&[&str]] = &[
            &["achievement", "list"],
            &[
                "achievement",
                "list",
                "--pack",
                "cooking",
                "--status",
                "achieved",
            ],
            &["achievement", "state-set", "--file", "state.json"],
            &["achievement", "state-revoke", "cooking::first_dish"],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Achievement { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }

    #[test]
    fn parses_every_skill_command() {
        let commands: &[&[&str]] = &[
            &["skill", "list"],
            &[
                "skill",
                "list",
                "--skill-id",
                "cooking::general",
                "--pack",
                "cooking",
            ],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Skill { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }

    #[test]
    fn parses_every_mission_command() {
        let commands: &[&[&str]] = &[
            &["mission", "list"],
            &[
                "mission",
                "list",
                "--status",
                "active",
                "--parent-id",
                "parent-id",
            ],
            &["mission", "create", "--file", "mission.json"],
            &["mission", "update", "--file", "mission.json"],
            &["mission", "complete", "mission-id"],
            &["mission", "archive", "mission-id"],
            &["mission", "delete", "mission-id"],
            &["mission", "suggestion-list", "--status", "pending"],
            &["mission", "suggest", "--file", "suggestion.json"],
            &["mission", "accept", "suggestion-id"],
            &["mission", "reject", "suggestion-id"],
            &["mission", "suggestion-delete", "suggestion-id"],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Mission { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }

    #[test]
    fn parses_every_memory_command() {
        let commands: &[&[&str]] = &[
            &["memory", "list"],
            &[
                "memory",
                "list",
                "--memory-id",
                "memory-id",
                "--kind",
                "reminder",
            ],
            &["memory", "create", "--file", "memory.json"],
            &["memory", "update", "--file", "memory.json"],
            &["memory", "delete", "memory-id"],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Memory { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }

    #[test]
    fn parses_every_record_command() {
        let commands: &[&[&str]] = &[
            &["record", "get", "identity.nickname"],
            &["record", "query", "--namespace", "identity"],
            &["record", "set", "--file", "command.json"],
            &["record", "increment", "--file", "command.json"],
            &["record", "correct", "--file", "command.json"],
            &["record", "create-empty-collection", "stats.projects"],
            &["record", "create-empty-event", "stats.activities"],
            &["record", "add-item", "--file", "command.json"],
            &["record", "correct-item", "--file", "command.json"],
            &["record", "remove-item", "--file", "command.json"],
            &["record", "append-event", "--file", "command.json"],
            &["record", "correct-event", "--file", "command.json"],
            &["record", "delete-event", "--file", "command.json"],
            &["record", "delete", "identity.nickname"],
        ];
        for arguments in commands {
            let mut argv = vec!["arcana-data"];
            argv.extend_from_slice(arguments);
            assert!(
                matches!(
                    Cli::try_parse_from(argv).unwrap().command,
                    Commands::Record { .. }
                ),
                "failed to parse {arguments:?}"
            );
        }
    }
}
