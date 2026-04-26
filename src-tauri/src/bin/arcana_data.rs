//! Arcana data CLI — structured data operations for AI skills and agents.
//!
//! Usage:
//!   arcana-data context [--missions] [--status] [--achievements] [--memory]
//!   arcana-data read <path>
//!   arcana-data mission update <id> [--progress N] [--status S] ...
//!   arcana-data mission create < stdin
//!   arcana-data mission update-menu [--countdown JSON] [--progress JSON]
//!   arcana-data status update <key=value>...
//!   arcana-data achievement update <id> --status <s> [--progress-detail "..."]...
//!   arcana-data changelog write --skill <s> --summary "..." < stdin
//!   arcana-data memory update < stdin

use arcana_lib::models::achievement::AchievementProgressFile;
use arcana_lib::models::mission::MissionFile;
use arcana_lib::models::status::{MetricDefinitionFile, StatusValueFile};
use arcana_lib::services;
use arcana_lib::storage::json_store::{read_json_file, resolve_data_dir};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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
    /// Create a new mission (reads JSON from stdin)
    Create,
}

#[derive(Subcommand)]
enum StatusAction {
    /// Update metric values: key=value pairs
    Update {
        /// Metric updates as key=value pairs (e.g. weight_kg=75.2 sleep_hours=7)
        metrics: Vec<String>,
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
enum ChangelogAction {
    /// Write a changelog entry (reads changes JSON array from stdin)
    Write {
        /// Skill name: "velvet-room", "phan-site", or "agent"
        #[arg(long)]
        skill: String,
        /// Human-readable summary
        #[arg(long)]
        summary: String,
    },
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Update mission memory (reads JSON from stdin)
    Update,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let data_dir = match resolve_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Fatal: {e}");
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        Commands::Context {
            missions,
            status,
            achievements,
            memory,
            active_only,
            pack,
        } => cmd_context(
            &data_dir,
            missions,
            status,
            achievements,
            memory,
            active_only,
            pack,
        ),
        Commands::Read { path } => cmd_read(&data_dir, &path),
        Commands::Mission { action } => cmd_mission(&data_dir, action),
        Commands::Status { action } => cmd_status(&data_dir, action),
        Commands::Achievement { action } => cmd_achievement(&data_dir, action),
        Commands::Changelog { action } => cmd_changelog(&data_dir, action),
        Commands::Memory { action } => cmd_memory(&data_dir, action),
    };

    match result {
        Ok(output) => {
            if cli.compact {
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

fn read_stdin() -> Result<String, String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("Failed to read stdin: {e}"))?;
    Ok(buf)
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
        MissionAction::Create => {
            let stdin = read_stdin()?;
            let input: Value =
                serde_json::from_str(&stdin).map_err(|e| format!("Invalid JSON on stdin: {e}"))?;
            with_write_lock(data_dir, || {
                services::mission::create_mission(data_dir, &input)
            })
        }
    }
}

// ---------------------------------------------------------------------------
// status
// ---------------------------------------------------------------------------

fn cmd_status(data_dir: &Path, action: StatusAction) -> Result<String, String> {
    match action {
        StatusAction::Update { metrics } => {
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
            let input = json!({"metrics": map});
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
// changelog
// ---------------------------------------------------------------------------

fn cmd_changelog(data_dir: &Path, action: ChangelogAction) -> Result<String, String> {
    match action {
        ChangelogAction::Write { skill, summary } => {
            let stdin = read_stdin()?;
            let changes: Value = serde_json::from_str(&stdin)
                .map_err(|e| format!("Invalid changes JSON on stdin: {e}"))?;
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
        MemoryAction::Update => {
            let stdin = read_stdin()?;
            let input: Value =
                serde_json::from_str(&stdin).map_err(|e| format!("Invalid JSON on stdin: {e}"))?;
            services::memory::update_mission_memory(data_dir, &input)
        }
    }
}
