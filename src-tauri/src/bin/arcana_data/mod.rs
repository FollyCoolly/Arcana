mod contract;
mod pack_commands;
mod record_commands;
mod runtime_commands;

use clap::{error::ErrorKind, Parser, Subcommand};
use contract::{capabilities, render_json, CliError, EXIT_SUCCESS};
use pack_commands::{execute_pack, PackAction};
use record_commands::{execute_record, RecordAction};
use runtime_commands::{execute_init, execute_json, JsonAction};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "arcana-data", about = "Arcana SQLite data operations CLI")]
struct Cli {
    /// Output compact JSON instead of pretty JSON
    #[arg(long, global = true)]
    compact: bool,

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

    match execute(cli.command) {
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

fn execute(command: Commands) -> Result<Value, CliError> {
    match command {
        Commands::Capabilities => Ok(capabilities()),
        Commands::Init { runtime } => execute_init(runtime),
        Commands::Record { runtime, action } => execute_record(runtime, action),
        Commands::Pack { runtime, action } => execute_pack(runtime, action),
        Commands::Json { action } => execute_json(action),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_new_sqlite_command_families_are_exposed() {
        for removed in [
            "context",
            "read",
            "mission",
            "status",
            "achievement",
            "changelog",
            "memory",
        ] {
            assert!(Cli::try_parse_from(["arcana-data", removed]).is_err());
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
