//! Arcana data CLI — structured data operations for AI skills and agents.
//!
//! Usage:
//!   arcana-data context [--missions] [--status] [--achievements] [--memory]
//!   arcana-data read <path>
//!   arcana-data mission update <id> [--progress N] [--status S] ...
//!   arcana-data mission create [--file <path>]
//!   arcana-data mission update-menu [--countdown JSON] [--progress JSON]
//!   arcana-data status update <key=value>...
//!   arcana-data achievement update <id> --status <s> [--progress-detail "..."]...
//!   arcana-data pack list|validate|scaffold|write|enable|disable
//!   arcana-data changelog write --skill <s> --summary "..." [--file <path>]
//!   arcana-data memory update [--file <path>]
//!   arcana-data json export --output <directory> [--runtime <directory>]
//!   arcana-data json import --input <directory> [--runtime <directory>]

use arcana_lib::application::ArcanaRuntime;
use arcana_lib::domain::SyncedRepositorySnapshot;
use arcana_lib::models::achievement::{AchievementFile, AchievementProgressFile, PackManifest};
use arcana_lib::models::mission::MissionFile;
use arcana_lib::models::skill::SkillFile;
use arcana_lib::models::status::{MetricDefinitionFile, StatusValueFile};
use arcana_lib::services;
use arcana_lib::storage::date_utils::{epoch_days_to_civil, today_epoch_days};
use arcana_lib::storage::json_store::{read_json_file, resolve_data_dir};
use arcana_lib::storage::settings::{expand_tilde, load_settings};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// CLI structure
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "arcana-data", about = "Arcana data operations CLI")]
struct Cli {
    /// Output compact JSON (no pretty-print)
    #[arg(long, global = true)]
    compact: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Read current Arcana context (missions, status, achievements, memory)
    Context {
        /// Show missions section
        #[arg(long)]
        missions: bool,
        /// Show status metrics section
        #[arg(long)]
        status: bool,
        /// Show achievement progress section
        #[arg(long)]
        achievements: bool,
        /// Show mission memory section
        #[arg(long)]
        memory: bool,
        /// Only show active missions (exclude proposed)
        #[arg(long)]
        active_only: bool,
        /// Filter achievements by pack ID prefix
        #[arg(long)]
        pack: Option<String>,
    },
    /// Read a file relative to data/ directory (sandboxed)
    Read {
        /// Relative path under data/ (e.g. "packs/programmer/achievements.json")
        path: String,
    },
    /// Mission operations
    Mission {
        #[command(subcommand)]
        action: MissionAction,
    },
    /// Update status metrics
    Status {
        #[command(subcommand)]
        action: StatusAction,
    },
    /// Update achievement progress
    Achievement {
        #[command(subcommand)]
        action: AchievementAction,
    },
    /// Content pack operations
    Pack {
        #[command(subcommand)]
        action: PackAction,
    },
    /// Write a changelog entry
    Changelog {
        #[command(subcommand)]
        action: ChangelogAction,
    },
    /// Update mission memory
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Initialize a fresh Arcana data directory from data-example/
    Init {
        /// Deprecated: init is non-interactive by default
        #[arg(long)]
        non_interactive: bool,
    },
    /// Convert between the new SQLite runtime and a canonical JSON directory
    Json {
        #[command(subcommand)]
        action: JsonAction,
    },
}

#[derive(Subcommand)]
enum JsonAction {
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

#[derive(Subcommand)]
enum MissionAction {
    /// Update an existing mission's fields
    Update {
        /// Mission ID
        id: String,
        #[arg(long)]
        progress: Option<u32>,
        #[arg(long)]
        status: Option<String>,
        /// Difficulty: S, A, B, C, or D (S = hardest)
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        short_desc: Option<String>,
        #[arg(long)]
        deadline: Option<String>,
        #[arg(long)]
        completed_at: Option<String>,
        #[arg(long)]
        linked_achievement_id: Option<String>,
        #[arg(long)]
        parent_id: Option<String>,
        /// AI metadata as JSON string
        #[arg(long)]
        ai_metadata: Option<String>,
    },
    /// Update main menu display config
    UpdateMenu {
        /// Countdown widget JSON: {"mission_id": "...", "label": "..."} or "null" to clear
        #[arg(long)]
        countdown: Option<String>,
        /// Hints array JSON: [{"mission_id":"...","short_desc":"..."},...] or "null" to clear
        #[arg(long)]
        hints: Option<String>,
        /// Progress widget JSON: {"mission_id": "...", "label": "..."} or "null" to clear
        #[arg(long)]
        progress: Option<String>,
    },
    /// Create a new mission (reads JSON from stdin or --file)
    Create {
        /// Read JSON from file instead of stdin
        #[arg(long)]
        file: Option<String>,
    },
    /// Permanently delete a mission (also clears any main_menu references to it)
    Delete {
        /// Mission ID
        id: String,
    },
}

#[derive(Subcommand)]
enum StatusAction {
    /// Update metric values: set via key=value pairs and/or remove via --remove
    Update {
        /// Metric updates as key=value pairs (e.g. weight_kg=75.2 sleep_hours=7)
        metrics: Vec<String>,
        /// Metric IDs to remove (can be repeated). Sets the metric back to "not measured".
        #[arg(long = "remove", value_name = "METRIC_ID")]
        remove: Vec<String>,
    },
}

#[derive(Subcommand)]
enum AchievementAction {
    /// Update achievement progress
    Update {
        /// Achievement ID (e.g. "programmer::rust_proficient")
        id: String,
        /// Status: "tracked" or "achieved"
        #[arg(long)]
        status: String,
        /// Progress detail entries to append (repeatable)
        #[arg(long)]
        progress_detail: Vec<String>,
        /// Optional note
        #[arg(long)]
        note: Option<String>,
        /// Mark as potentially incomplete
        #[arg(long)]
        may_be_incomplete: bool,
    },
}

#[derive(Subcommand)]
enum PackAction {
    /// List content packs and whether they are enabled
    List,
    /// Validate a content pack's manifest, achievements, and skills
    Validate {
        /// Pack ID / directory name under data/packs/
        id: String,
    },
    /// Create a pack directory with valid starter JSON files
    Scaffold {
        /// Pack ID / directory name under data/packs/
        id: String,
        /// Display name. Defaults to a title-cased form of the ID.
        #[arg(long)]
        name: Option<String>,
        /// Pack description
        #[arg(long, default_value = "")]
        description: String,
        /// Pack author
        #[arg(long, default_value = "Arcana")]
        author: String,
        /// Pack tags. Repeat --tag for multiple tags.
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// Write one or more pack JSON files after validating the full pack
    Write {
        /// Pack ID / directory name under data/packs/
        id: String,
        /// Path to manifest.json payload
        #[arg(long)]
        manifest: Option<String>,
        /// Path to achievements.json payload
        #[arg(long)]
        achievements: Option<String>,
        /// Path to skills.json payload
        #[arg(long)]
        skills: Option<String>,
        /// Enable the pack after a successful write
        #[arg(long)]
        enable: bool,
    },
    /// Add a valid pack to loaded_packs.json
    Enable {
        /// Pack ID / directory name under data/packs/
        id: String,
    },
    /// Remove a pack from loaded_packs.json
    Disable {
        /// Pack ID / directory name under data/packs/
        id: String,
    },
}

#[derive(Subcommand)]
enum ChangelogAction {
    /// Write a changelog entry (reads changes JSON array from stdin or --file)
    Write {
        /// Skill name: "velvet-room", "phan-site", or "agent"
        #[arg(long)]
        skill: String,
        /// Human-readable summary
        #[arg(long)]
        summary: String,
        /// Read changes JSON from file instead of stdin
        #[arg(long)]
        file: Option<String>,
    },
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Update mission memory (reads JSON from stdin or --file)
    Update {
        /// Read JSON from file instead of stdin
        #[arg(long)]
        file: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Json { action } => cmd_json(action),
        command => {
            let data_dir = match resolve_data_dir() {
                Ok(directory) => directory,
                Err(error) => {
                    eprintln!("Fatal: {error}");
                    std::process::exit(1);
                }
            };
            dispatch_legacy_command(&data_dir, command)
        }
    };

    print_result(result, cli.compact);
}

fn dispatch_legacy_command(data_dir: &Path, command: Commands) -> Result<String, String> {
    match command {
        Commands::Context {
            missions,
            status,
            achievements,
            memory,
            active_only,
            pack,
        } => cmd_context(
            data_dir,
            missions,
            status,
            achievements,
            memory,
            active_only,
            pack,
        ),
        Commands::Read { path } => cmd_read(data_dir, &path),
        Commands::Mission { action } => cmd_mission(data_dir, action),
        Commands::Status { action } => cmd_status(data_dir, action),
        Commands::Achievement { action } => cmd_achievement(data_dir, action),
        Commands::Pack { action } => cmd_pack(data_dir, action),
        Commands::Changelog { action } => cmd_changelog(data_dir, action),
        Commands::Memory { action } => cmd_memory(data_dir, action),
        Commands::Init { non_interactive } => cmd_init(data_dir, non_interactive),
        Commands::Json { .. } => unreachable!("JSON commands are dispatched before legacy data"),
    }
}

fn print_result(result: Result<String, String>, compact: bool) {
    match result {
        Ok(output) => {
            if compact {
                // Re-parse and compact if it's valid JSON, otherwise print as-is
                if let Ok(v) = serde_json::from_str::<Value>(&output) {
                    println!("{}", serde_json::to_string(&v).unwrap_or(output));
                } else {
                    println!("{output}");
                }
            } else {
                println!("{output}");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_json(action: JsonAction) -> Result<String, String> {
    match action {
        JsonAction::Export { output, runtime } => {
            let runtime = runtime_from_cli(runtime)?;
            let output = absolute_cli_path(output)?;
            let snapshot = runtime
                .export_json_to_new_directory(&output)
                .map_err(|error| error.to_string())?;
            json_command_output("export", runtime.runtime_dir(), &output, &snapshot)
        }
        JsonAction::Import { input, runtime } => {
            let runtime = runtime_from_cli(runtime)?;
            let input = absolute_cli_path(input)?;
            let snapshot = runtime
                .import_json_from_directory(&input)
                .map_err(|error| error.to_string())?;
            json_command_output("import", runtime.runtime_dir(), &input, &snapshot)
        }
    }
}

fn runtime_from_cli(runtime_dir: Option<PathBuf>) -> Result<ArcanaRuntime, String> {
    match runtime_dir {
        Some(path) => {
            let absolute = absolute_cli_path(path)?;
            ArcanaRuntime::new(absolute).map_err(|error| error.to_string())
        }
        None => ArcanaRuntime::from_settings(&load_settings()).map_err(|error| error.to_string()),
    }
}

fn absolute_cli_path(path: PathBuf) -> Result<PathBuf, String> {
    let path = path.to_str().map(expand_tilde).unwrap_or(path);
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|directory| directory.join(path))
        .map_err(|error| format!("cannot resolve relative path: {error}"))
}

fn json_command_output(
    operation: &str,
    runtime_dir: &Path,
    directory: &Path,
    snapshot: &SyncedRepositorySnapshot,
) -> Result<String, String> {
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
    serde_json::to_string_pretty(&json!({
        "operation": format!("json_{operation}"),
        "runtime_dir": runtime_dir,
        "directory": absolute_cli_path(directory.to_path_buf())?,
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
    }))
    .map_err(|error| format!("failed to serialize JSON command result: {error}"))
}

// ---------------------------------------------------------------------------
// File lock helper
// ---------------------------------------------------------------------------

fn with_write_lock<F>(data_dir: &Path, f: F) -> Result<String, String>
where
    F: FnOnce() -> Result<String, String>,
{
    let lock_path = data_dir.join(".write.lock");
    let lock_file =
        File::create(&lock_path).map_err(|e| format!("Cannot create lock file: {e}"))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| format!("Cannot acquire write lock: {e}"))?;
    let result = f();
    let _ = lock_file.unlock();
    result
}

fn read_input(file: Option<&str>) -> Result<String, String> {
    if let Some(path) = file {
        if !Path::new(path).exists() {
            return Err(format!("File not found: {path}"));
        }
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))
    } else {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("Failed to read stdin: {e}"))?;
        Ok(buf)
    }
}

// ---------------------------------------------------------------------------
// context
// ---------------------------------------------------------------------------

fn cmd_context(
    data_dir: &Path,
    missions: bool,
    status: bool,
    achievements: bool,
    memory: bool,
    active_only: bool,
    pack: Option<String>,
) -> Result<String, String> {
    let show_all = !missions && !status && !achievements && !memory;
    let mut sections: HashMap<&str, Value> = HashMap::new();

    if show_all || missions {
        let missions_path = data_dir.join("missions.json");
        if missions_path.exists() {
            let file: MissionFile = read_json_file(&missions_path)?;
            let filtered: Vec<_> = file
                .missions
                .iter()
                .filter(|m| {
                    if active_only {
                        m.status == "active"
                    } else {
                        m.status == "active" || m.status == "proposed"
                    }
                })
                .collect();
            sections.insert(
                "missions",
                json!({
                    "items": serde_json::to_value(&filtered).unwrap_or_default(),
                    "main_menu": serde_json::to_value(&file.main_menu).unwrap_or_default(),
                }),
            );
        }
    }

    if show_all || status {
        let status_path = data_dir.join("status.json");
        if status_path.exists() {
            let values: StatusValueFile = read_json_file(&status_path)?;
            let mut status_section = json!({"metrics": values.metrics});

            let defs_path = data_dir.join("status_metric_definitions.json");
            if defs_path.exists() {
                let defs: MetricDefinitionFile = read_json_file(&defs_path)?;
                let summary: Vec<Value> = defs
                    .metrics
                    .iter()
                    .map(|m| {
                        json!({"id": m.id, "name": m.name, "unit": m.unit, "group": m.group, "description": m.description})
                    })
                    .collect();
                status_section["definitions"] = json!(summary);

                let dims: Vec<Value> = defs
                    .dimensions
                    .iter()
                    .filter(|d| d.enabled)
                    .map(|d| {
                        json!({
                            "id": d.id,
                            "name": d.name,
                            "level_titles": d.level_titles,
                            "level_thresholds": d.level_thresholds,
                            "metric_count": d.metrics.len(),
                        })
                    })
                    .collect();
                if !dims.is_empty() {
                    status_section["dimensions"] = json!(dims);
                }
            }

            sections.insert("status", status_section);
        }
    }

    if show_all || achievements {
        let progress_path = data_dir.join("achievement_progress.json");
        if progress_path.exists() {
            let progress: AchievementProgressFile = read_json_file(&progress_path)?;
            if !progress.achievements.is_empty() {
                let filtered: HashMap<_, _> = if let Some(ref pack_id) = pack {
                    let prefix = format!("{pack_id}::");
                    progress
                        .achievements
                        .into_iter()
                        .filter(|(k, _)| k.starts_with(&prefix))
                        .collect()
                } else {
                    progress.achievements
                };
                sections.insert(
                    "achievements",
                    serde_json::to_value(&filtered).unwrap_or_default(),
                );
            }
        }
    }

    if show_all || memory {
        let memory_path = data_dir.join("mission_memory.json");
        if memory_path.exists() {
            let mem: Value = read_json_file(&memory_path)?;
            sections.insert("memory", mem);
        }
    }

    if sections.is_empty() {
        Ok(json!({"message": "No data files found."}).to_string())
    } else {
        Ok(serde_json::to_string_pretty(&sections).unwrap_or_default())
    }
}

// ---------------------------------------------------------------------------
// read
// ---------------------------------------------------------------------------

fn cmd_read(data_dir: &Path, path: &str) -> Result<String, String> {
    services::file_access::read_sandboxed_file(data_dir, path)
}

// ---------------------------------------------------------------------------
// mission
// ---------------------------------------------------------------------------

fn cmd_mission(data_dir: &Path, action: MissionAction) -> Result<String, String> {
    match action {
        MissionAction::Update {
            id,
            progress,
            status,
            difficulty,
            title,
            description,
            short_desc,
            deadline,
            completed_at,
            linked_achievement_id,
            parent_id,
            ai_metadata,
        } => {
            let mut updates = serde_json::Map::new();
            if let Some(p) = progress {
                updates.insert("progress".into(), json!(p));
            }
            if let Some(s) = status {
                updates.insert("status".into(), json!(s));
            }
            if let Some(d) = difficulty {
                updates.insert("difficulty".into(), json!(d));
            }
            if let Some(t) = title {
                updates.insert("title".into(), json!(t));
            }
            if let Some(d) = description {
                updates.insert("description".into(), json!(d));
            }
            if let Some(s) = short_desc {
                updates.insert("short_desc".into(), json!(s));
            }
            if let Some(d) = deadline {
                updates.insert("deadline".into(), json!(d));
            }
            if let Some(c) = completed_at {
                updates.insert("completed_at".into(), json!(c));
            }
            if let Some(a) = linked_achievement_id {
                updates.insert("linked_achievement_id".into(), json!(a));
            }
            if let Some(p) = parent_id {
                updates.insert("parent_id".into(), json!(p));
            }
            if let Some(meta) = ai_metadata {
                let parsed: Value = serde_json::from_str(&meta)
                    .map_err(|e| format!("Invalid ai_metadata JSON: {e}"))?;
                updates.insert("ai_metadata".into(), parsed);
            }

            let input = json!({"mission_id": id, "updates": updates});
            with_write_lock(data_dir, || {
                services::mission::update_mission(data_dir, &input)
            })
        }
        MissionAction::UpdateMenu {
            countdown,
            hints,
            progress,
        } => {
            let mut menu = serde_json::Map::new();
            if let Some(c) = countdown {
                let val: Value = if c == "null" {
                    Value::Null
                } else {
                    serde_json::from_str(&c).map_err(|e| format!("Invalid countdown JSON: {e}"))?
                };
                menu.insert("countdown".into(), val);
            }
            if let Some(h) = hints {
                let val: Value = if h == "null" {
                    Value::Null
                } else {
                    serde_json::from_str(&h).map_err(|e| format!("Invalid hints JSON: {e}"))?
                };
                menu.insert("hints".into(), val);
            }
            if let Some(p) = progress {
                let val: Value = if p == "null" {
                    Value::Null
                } else {
                    serde_json::from_str(&p).map_err(|e| format!("Invalid progress JSON: {e}"))?
                };
                menu.insert("progress".into(), val);
            }

            let input = json!({"main_menu": menu});
            with_write_lock(data_dir, || {
                services::mission::update_mission(data_dir, &input)
            })
        }
        MissionAction::Create { file } => {
            let input_str = read_input(file.as_deref())?;
            let input: Value =
                serde_json::from_str(&input_str).map_err(|e| format!("Invalid JSON: {e}"))?;
            with_write_lock(data_dir, || {
                services::mission::create_mission(data_dir, &input)
            })
        }
        MissionAction::Delete { id } => with_write_lock(data_dir, || {
            services::mission::delete_mission(data_dir, &id)
        }),
    }
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(data_dir: &Path, action: StatusAction) -> Result<String, String> {
    match action {
        StatusAction::Update { metrics, remove } => {
            if metrics.is_empty() && remove.is_empty() {
                return Err("Provide at least one key=value update or --remove <id>".into());
            }
            let mut map = serde_json::Map::new();
            for kv in &metrics {
                let parts: Vec<&str> = kv.splitn(2, '=').collect();
                if parts.len() != 2 {
                    return Err(format!("Invalid metric format '{kv}', expected key=value"));
                }
                let val: f64 = parts[1].parse().map_err(|_| {
                    format!("Invalid number '{}' for metric '{}'", parts[1], parts[0])
                })?;
                map.insert(parts[0].to_string(), json!(val));
            }
            let input = json!({"metrics": map, "remove": remove});
            with_write_lock(data_dir, || {
                services::status::update_status(data_dir, &input)
            })
        }
    }
}

// ---------------------------------------------------------------------------
// achievement
// ---------------------------------------------------------------------------

fn cmd_achievement(data_dir: &Path, action: AchievementAction) -> Result<String, String> {
    match action {
        AchievementAction::Update {
            id,
            status,
            progress_detail,
            note,
            may_be_incomplete,
        } => {
            let mut input = json!({
                "achievement_id": id,
                "status": status,
            });
            if !progress_detail.is_empty() {
                input["progress_detail"] = json!(progress_detail);
            }
            if let Some(n) = note {
                input["note"] = json!(n);
            }
            if may_be_incomplete {
                input["may_be_incomplete"] = json!(true);
            }
            with_write_lock(data_dir, || {
                services::achievement::update_achievement(data_dir, &input)
            })
        }
    }
}

// ---------------------------------------------------------------------------
// pack
// ---------------------------------------------------------------------------

fn cmd_pack(data_dir: &Path, action: PackAction) -> Result<String, String> {
    match action {
        PackAction::List => cmd_pack_list(data_dir),
        PackAction::Validate { id } => {
            validate_pack_id(&id)?;
            let summary = validate_pack_on_disk(data_dir, &id)?;
            Ok(serde_json::to_string_pretty(&summary).unwrap_or_default())
        }
        PackAction::Scaffold {
            id,
            name,
            description,
            author,
            tags,
        } => {
            validate_pack_id(&id)?;
            with_write_lock(data_dir, || {
                cmd_pack_scaffold(data_dir, &id, name, description, author, tags)
            })
        }
        PackAction::Write {
            id,
            manifest,
            achievements,
            skills,
            enable,
        } => {
            validate_pack_id(&id)?;
            with_write_lock(data_dir, || {
                cmd_pack_write(data_dir, &id, manifest, achievements, skills, enable)
            })
        }
        PackAction::Enable { id } => {
            validate_pack_id(&id)?;
            with_write_lock(data_dir, || cmd_pack_enable(data_dir, &id))
        }
        PackAction::Disable { id } => {
            validate_pack_id(&id)?;
            with_write_lock(data_dir, || cmd_pack_disable(data_dir, &id))
        }
    }
}

fn validate_pack_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Pack ID cannot be empty".into());
    }
    if id == "." || id == ".." {
        return Err("Pack ID cannot be '.' or '..'".into());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(format!(
            "Invalid pack ID '{id}'. Use lowercase ASCII letters, digits, '_' or '-'."
        ));
    }
    Ok(())
}

fn pack_dir(data_dir: &Path, id: &str) -> std::path::PathBuf {
    data_dir.join("packs").join(id)
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    read_json_file(path)
}

fn read_json_value_from_arg(path: &str) -> Result<Value, String> {
    let input = read_input(Some(path))?;
    serde_json::from_str(&input).map_err(|e| format!("Invalid JSON in {path}: {e}"))
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize {}: {e}", path.display()))?;
    std::fs::write(path, content).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

fn read_pack_file_or_arg(
    pack_dir: &Path,
    file_name: &str,
    arg: Option<&String>,
) -> Result<Value, String> {
    if let Some(path) = arg {
        read_json_value_from_arg(path)
    } else {
        let path = pack_dir.join(file_name);
        if !path.exists() {
            return Err(format!(
                "Missing {file_name}. Provide --{} <path> or scaffold the pack first.",
                file_name.trim_end_matches(".json")
            ));
        }
        read_json_value(&path)
    }
}

fn value_as<T: serde::de::DeserializeOwned>(value: &Value, label: &str) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("Invalid {label}: {e}"))
}

fn validate_pack_values(
    id: &str,
    manifest_value: &Value,
    achievements_value: &Value,
    skills_value: &Value,
) -> Result<Value, String> {
    let manifest: PackManifest = value_as(manifest_value, "manifest.json")?;
    let achievement_file: AchievementFile = value_as(achievements_value, "achievements.json")?;
    let skill_file: SkillFile = value_as(skills_value, "skills.json")?;

    if manifest.id != id {
        return Err(format!(
            "manifest.id '{}' must equal pack directory '{}'",
            manifest.id, id
        ));
    }

    let prefix = format!("{id}::");
    let mut achievement_ids = HashSet::new();
    for achievement in &achievement_file.achievements {
        if !achievement.id.starts_with(&prefix) {
            return Err(format!(
                "Achievement '{}' must start with '{}'",
                achievement.id, prefix
            ));
        }
        if !achievement_ids.insert(achievement.id.clone()) {
            return Err(format!("Duplicate achievement id '{}'", achievement.id));
        }
    }

    for achievement in &achievement_file.achievements {
        for prereq in &achievement.prerequisites {
            if !achievement_ids.contains(prereq) {
                return Err(format!(
                    "Achievement '{}' references unknown prerequisite '{}'",
                    achievement.id, prereq
                ));
            }
        }
    }
    if let Some(cycle) = detect_pack_prerequisite_cycle(&achievement_file.achievements) {
        return Err(cycle);
    }

    let mut skill_ids = HashSet::new();
    for skill in &skill_file.skills {
        if !skill.id.starts_with(&prefix) {
            return Err(format!("Skill '{}' must start with '{}'", skill.id, prefix));
        }
        if !skill_ids.insert(skill.id.clone()) {
            return Err(format!("Duplicate skill id '{}'", skill.id));
        }

        let expected_thresholds = skill.max_level.saturating_sub(1) as usize;
        if skill.level_thresholds.len() != expected_thresholds {
            return Err(format!(
                "Skill '{}': level_thresholds count ({}) != max_level - 1 ({})",
                skill.id,
                skill.level_thresholds.len(),
                expected_thresholds
            ));
        }

        let mut previous_points = 0;
        for (i, threshold) in skill.level_thresholds.iter().enumerate() {
            let expected_level = i as u32 + 2;
            if threshold.level != expected_level {
                return Err(format!(
                    "Skill '{}': level_thresholds[{i}].level must be {expected_level}, got {}",
                    skill.id, threshold.level
                ));
            }
            if i > 0 && threshold.points_required <= previous_points {
                return Err(format!(
                    "Skill '{}': level_thresholds[{i}].points_required {} must be greater than previous {}",
                    skill.id, threshold.points_required, previous_points
                ));
            }
            previous_points = threshold.points_required;

            for key in &threshold.required_key_achievements {
                if !achievement_ids.contains(key) {
                    return Err(format!(
                        "Skill '{}': level {} required_key_achievement '{}' not found",
                        skill.id, threshold.level, key
                    ));
                }
            }
        }

        let mut node_ids = HashSet::new();
        for node in &skill.nodes {
            if !node_ids.insert(node.node_id.clone()) {
                return Err(format!(
                    "Skill '{}': duplicate node_id '{}'",
                    skill.id, node.node_id
                ));
            }
            if !achievement_ids.contains(&node.achievement_id) {
                return Err(format!(
                    "Skill '{}': node '{}' references unknown achievement '{}'",
                    skill.id, node.node_id, node.achievement_id
                ));
            }
        }
    }

    Ok(json!({
        "pack_id": id,
        "valid": true,
        "manifest": {
            "name": manifest.name,
            "version": manifest.version,
            "author": manifest.author,
            "tags": manifest.tags,
        },
        "achievement_count": achievement_file.achievements.len(),
        "skill_count": skill_file.skills.len(),
    }))
}

fn detect_pack_prerequisite_cycle(
    achievements: &[arcana_lib::models::achievement::AchievementDef],
) -> Option<String> {
    let adjacency: HashMap<&str, Vec<&str>> = achievements
        .iter()
        .map(|a| {
            (
                a.id.as_str(),
                a.prerequisites.iter().map(|p| p.as_str()).collect(),
            )
        })
        .collect();
    let mut state: HashMap<&str, u8> = adjacency.keys().map(|&id| (id, 0)).collect();

    for &start in adjacency.keys() {
        if state.get(start) == Some(&2) {
            continue;
        }

        let mut stack = vec![(start, 0usize)];
        state.insert(start, 1);
        while let Some((node, index)) = stack.last_mut() {
            let next_nodes = adjacency.get(*node).map(|v| v.as_slice()).unwrap_or(&[]);
            if *index < next_nodes.len() {
                let next = next_nodes[*index];
                *index += 1;
                match state.get(next).copied().unwrap_or(0) {
                    0 => {
                        state.insert(next, 1);
                        stack.push((next, 0));
                    }
                    1 => {
                        return Some(format!("Prerequisite cycle detected involving '{}'", next));
                    }
                    _ => {}
                }
            } else {
                let finished = *node;
                state.insert(finished, 2);
                stack.pop();
            }
        }
    }

    None
}

fn validate_pack_on_disk(data_dir: &Path, id: &str) -> Result<Value, String> {
    let dir = pack_dir(data_dir, id);
    if !dir.is_dir() {
        return Err(format!("Pack '{id}' does not exist at {}", dir.display()));
    }
    let manifest = read_json_value(&dir.join("manifest.json"))?;
    let achievements = read_json_value(&dir.join("achievements.json"))?;
    let skills = read_json_value(&dir.join("skills.json"))?;
    validate_pack_values(id, &manifest, &achievements, &skills)
}

fn read_loaded_pack_ids(data_dir: &Path) -> Result<Vec<String>, String> {
    let path = data_dir.join("loaded_packs.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let value: Value = read_json_file(&path)?;
    let packs = value
        .get("packs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "loaded_packs.json: 'packs' must be an array".to_string())?;
    packs
        .iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or_else(|| "loaded_packs.json: every pack id must be a string".to_string())
        })
        .collect()
}

fn write_loaded_pack_ids(data_dir: &Path, packs: &[String]) -> Result<(), String> {
    let path = data_dir.join("loaded_packs.json");
    write_json_value(&path, &json!({"version": 1, "packs": packs}))
}

fn cmd_pack_list(data_dir: &Path) -> Result<String, String> {
    let loaded: HashSet<String> = read_loaded_pack_ids(data_dir)?.into_iter().collect();
    let packs_dir = data_dir.join("packs");
    let mut packs = Vec::new();

    if packs_dir.is_dir() {
        for entry in std::fs::read_dir(&packs_dir)
            .map_err(|e| format!("Cannot read {}: {e}", packs_dir.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            let manifest_path = path.join("manifest.json");
            let manifest = if manifest_path.exists() {
                read_json_value(&manifest_path).unwrap_or_else(|e| json!({"error": e}))
            } else {
                Value::Null
            };
            packs.push(json!({
                "id": id,
                "enabled": loaded.contains(&id),
                "manifest": manifest,
            }));
        }
    }

    packs.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(serde_json::to_string_pretty(&json!({ "packs": packs })).unwrap_or_default())
}

fn default_pack_name(id: &str) -> String {
    id.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn cmd_pack_scaffold(
    data_dir: &Path,
    id: &str,
    name: Option<String>,
    description: String,
    author: String,
    tags: Vec<String>,
) -> Result<String, String> {
    let dir = pack_dir(data_dir, id);
    if dir.exists() {
        return Err(format!("Pack '{id}' already exists at {}", dir.display()));
    }

    let manifest = json!({
        "id": id,
        "name": name.unwrap_or_else(|| default_pack_name(id)),
        "description": description,
        "version": "1.0.0",
        "author": author,
        "tags": tags,
    });
    let achievements = json!({
        "version": 1,
        "achievements": [],
    });
    let skills = json!({
        "version": 1,
        "skills": [],
    });
    let summary = validate_pack_values(id, &manifest, &achievements, &skills)?;

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    write_json_value(&dir.join("manifest.json"), &manifest)?;
    write_json_value(&dir.join("achievements.json"), &achievements)?;
    write_json_value(&dir.join("skills.json"), &skills)?;

    Ok(serde_json::to_string_pretty(&json!({
        "message": "Pack scaffolded.",
        "pack": summary,
    }))
    .unwrap_or_default())
}

fn cmd_pack_write(
    data_dir: &Path,
    id: &str,
    manifest_arg: Option<String>,
    achievements_arg: Option<String>,
    skills_arg: Option<String>,
    enable: bool,
) -> Result<String, String> {
    if manifest_arg.is_none() && achievements_arg.is_none() && skills_arg.is_none() {
        return Err("Provide at least one of --manifest, --achievements, or --skills".into());
    }

    let dir = pack_dir(data_dir, id);
    let manifest = read_pack_file_or_arg(&dir, "manifest.json", manifest_arg.as_ref())?;
    let achievements = read_pack_file_or_arg(&dir, "achievements.json", achievements_arg.as_ref())?;
    let skills = read_pack_file_or_arg(&dir, "skills.json", skills_arg.as_ref())?;
    let summary = validate_pack_values(id, &manifest, &achievements, &skills)?;

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    let writes = [
        ("manifest.json", manifest_arg.as_ref(), &manifest),
        (
            "achievements.json",
            achievements_arg.as_ref(),
            &achievements,
        ),
        ("skills.json", skills_arg.as_ref(), &skills),
    ];

    let mut backups: Vec<(&str, Option<Vec<u8>>)> = Vec::new();
    for (file_name, arg, _) in &writes {
        if arg.is_some() {
            backups.push((*file_name, std::fs::read(dir.join(file_name)).ok()));
        }
    }

    for (file_name, arg, value) in &writes {
        if arg.is_some() {
            write_json_value(&dir.join(file_name), value)?;
        }
    }

    if let Err(e) = validate_pack_on_disk(data_dir, id) {
        for (file_name, backup) in backups {
            let path = dir.join(file_name);
            if let Some(bytes) = backup {
                let _ = std::fs::write(path, bytes);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
        return Err(format!(
            "Pack write rolled back after validation failure: {e}"
        ));
    }

    let enabled = if enable {
        enable_pack_id(data_dir, id)?;
        true
    } else {
        read_loaded_pack_ids(data_dir)?.contains(&id.to_string())
    };

    Ok(serde_json::to_string_pretty(&json!({
        "message": "Pack written.",
        "enabled": enabled,
        "pack": summary,
    }))
    .unwrap_or_default())
}

fn enable_pack_id(data_dir: &Path, id: &str) -> Result<(), String> {
    validate_pack_on_disk(data_dir, id)?;
    let mut packs = read_loaded_pack_ids(data_dir)?;
    if !packs.contains(&id.to_string()) {
        packs.push(id.to_string());
        write_loaded_pack_ids(data_dir, &packs)?;
    }
    Ok(())
}

fn cmd_pack_enable(data_dir: &Path, id: &str) -> Result<String, String> {
    enable_pack_id(data_dir, id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "message": "Pack enabled.",
        "pack_id": id,
        "loaded_packs": read_loaded_pack_ids(data_dir)?,
    }))
    .unwrap_or_default())
}

fn cmd_pack_disable(data_dir: &Path, id: &str) -> Result<String, String> {
    let mut packs = read_loaded_pack_ids(data_dir)?;
    let before = packs.len();
    packs.retain(|pack_id| pack_id != id);
    if packs.len() != before {
        write_loaded_pack_ids(data_dir, &packs)?;
    }
    Ok(serde_json::to_string_pretty(&json!({
        "message": "Pack disabled.",
        "pack_id": id,
        "loaded_packs": packs,
    }))
    .unwrap_or_default())
}

// ---------------------------------------------------------------------------
// changelog
// ---------------------------------------------------------------------------

fn cmd_changelog(data_dir: &Path, action: ChangelogAction) -> Result<String, String> {
    match action {
        ChangelogAction::Write {
            skill,
            summary,
            file,
        } => {
            let changes_str = read_input(file.as_deref())?;
            let changes: Value = serde_json::from_str(&changes_str)
                .map_err(|e| format!("Invalid changes JSON: {e}"))?;
            let input = json!({"summary": summary, "changes": changes});
            with_write_lock(data_dir, || {
                services::changelog::write_changelog(data_dir, &skill, &input)
            })
        }
    }
}

// ---------------------------------------------------------------------------
// memory
// ---------------------------------------------------------------------------

fn cmd_memory(data_dir: &Path, action: MemoryAction) -> Result<String, String> {
    match action {
        MemoryAction::Update { file } => {
            let input_str = read_input(file.as_deref())?;
            let input: Value =
                serde_json::from_str(&input_str).map_err(|e| format!("Invalid JSON: {e}"))?;
            services::memory::update_mission_memory(data_dir, &input)
        }
    }
}

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------

fn onboarding_deadline() -> Result<String, String> {
    let deadline_days = today_epoch_days()? + 7;
    let (year, month, day) = epoch_days_to_civil(deadline_days);
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn apply_onboarding_menu_defaults(missions: &mut Value) -> Result<(), String> {
    let deadline = onboarding_deadline()?;
    let mission_list = missions
        .get_mut("missions")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "data-example/missions.json is missing missions[]".to_string())?;

    let guide = mission_list
        .iter_mut()
        .find(|m| m.get("id").and_then(|v| v.as_str()) == Some("ai_20260430_arcana_guide"))
        .ok_or_else(|| {
            "data-example/missions.json is missing Arcana Awakening mission".to_string()
        })?;
    guide["deadline"] = json!(deadline);

    missions["main_menu"] = json!({
        "countdown": {
            "mission_id": "ai_20260430_arcana_guide",
            "label": "指引完成"
        },
        "hints": [
            { "mission_id": "ai_20260430_status_calibration" },
            { "mission_id": "ai_20260430_phantom_thieves_hq" }
        ],
        "progress": {
            "mission_id": "ai_20260430_arcana_guide",
            "label": "Arcana的新手指引**完成**了吗？"
        }
    });

    Ok(())
}

fn write_initialized_missions(src: &Path, dest: &Path) -> Result<(), String> {
    let mut missions: Value = read_json_file(src)?;
    apply_onboarding_menu_defaults(&mut missions)?;
    let content = serde_json::to_string_pretty(&missions)
        .map_err(|e| format!("Failed to serialize missions.json: {e}"))?;
    std::fs::write(dest, content).map_err(|e| format!("Failed to write missions.json: {e}"))
}

fn cmd_init(data_dir: &Path, _non_interactive: bool) -> Result<String, String> {
    // Locate data-example/ relative to the binary's location or working directory.
    // Try a few candidate paths to work both from repo root and from target/
    let candidates = [
        std::env::current_dir()
            .unwrap_or_default()
            .join("data-example"),
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(Path::new("."))
            .join("../../../data-example"),
    ];
    let example_dir = candidates
        .iter()
        .find(|p| p.exists())
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .ok_or_else(|| {
            "Cannot find data-example/ directory. Run this command from the repo root.".to_string()
        })?;

    println!("=== Arcana Init ===");
    println!("Setting up your data directory at: {}", data_dir.display());
    println!();

    // Copy flat files from data-example/, skipping any that already exist
    let flat_files = [
        "status.json",
        "status_metric_definitions.json",
        "loaded_packs.json",
        "achievement_progress.json",
        "gallery_sources.json",
        "item_sources.json",
        "missions.json",
        "mission_archive.json",
        "weather.json",
    ];
    for name in &flat_files {
        let dest = data_dir.join(name);
        if dest.exists() {
            println!("  skip  {name} (already exists)");
            continue;
        }
        let src = example_dir.join(name);
        if *name == "missions.json" {
            write_initialized_missions(&src, &dest)?;
        } else {
            std::fs::copy(&src, &dest).map_err(|e| format!("Failed to copy {name}: {e}"))?;
        }
        println!("  wrote {name}");
    }

    // Copy arcana pack
    let pack_dest = data_dir.join("packs").join("arcana");
    if pack_dest.exists() {
        println!("  skip  packs/arcana/ (already exists)");
    } else {
        std::fs::create_dir_all(&pack_dest)
            .map_err(|e| format!("Failed to create packs/arcana/: {e}"))?;
        let pack_src = example_dir.join("packs").join("arcana");
        for entry in std::fs::read_dir(&pack_src)
            .map_err(|e| format!("Cannot read data-example/packs/arcana/: {e}"))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_name = entry.file_name();
            std::fs::copy(entry.path(), pack_dest.join(&file_name)).map_err(|e| {
                format!(
                    "Failed to copy packs/arcana/{}: {e}",
                    file_name.to_string_lossy()
                )
            })?;
        }
        println!("  wrote packs/arcana/");
    }

    println!();
    println!(
        "Done! Data directory initialized at: {}",
        data_dir.display()
    );
    println!();
    println!("Next step: configure your AI provider, then run the app.");
    println!("  Recommended: set ANTHROPIC_API_KEY or equivalent in your agent config.");
    println!("  See docs/architecture.md for agent setup instructions.");

    Ok("Init complete.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_conversion_commands() {
        let export = Cli::try_parse_from([
            "arcana-data",
            "json",
            "export",
            "--output",
            "snapshot",
            "--runtime",
            "runtime",
        ])
        .unwrap();
        assert!(matches!(
            export.command,
            Commands::Json {
                action: JsonAction::Export { .. }
            }
        ));

        let import =
            Cli::try_parse_from(["arcana-data", "json", "import", "--input", "snapshot"]).unwrap();
        assert!(matches!(
            import.command,
            Commands::Json {
                action: JsonAction::Import { .. }
            }
        ));
    }

    #[test]
    fn json_cli_exports_and_imports_without_git_files() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        ArcanaRuntime::new(&runtime_dir)
            .unwrap()
            .initialize()
            .unwrap();
        let json_dir = directory.path().join("json");

        let export = cmd_json(JsonAction::Export {
            output: json_dir.clone(),
            runtime: Some(runtime_dir.clone()),
        })
        .unwrap();
        let export: Value = serde_json::from_str(&export).unwrap();
        assert_eq!(export["operation"], "json_export");
        assert_eq!(export["summary"]["packs"], 1);
        assert!(json_dir.join("arcana.json").is_file());
        assert!(!json_dir.join(".gitattributes").exists());

        let imported_runtime_dir = directory.path().join("imported-runtime");
        let import = cmd_json(JsonAction::Import {
            input: json_dir,
            runtime: Some(imported_runtime_dir.clone()),
        })
        .unwrap();
        let import: Value = serde_json::from_str(&import).unwrap();
        assert_eq!(import["operation"], "json_import");
        assert_eq!(import["summary"]["enabled_packs"], 1);
        assert!(imported_runtime_dir.join("arcana.sqlite3").is_file());
    }
}
