use crate::models::achievement::{AchievementFile, AchievementProgressFile, LoadedPacksFile};
use crate::models::mission::MissionFile;
use crate::models::status::{MetricDefinitionFile, StatusValueFile};
use crate::storage::json_store::{read_json_file, write_json_file};

use super::llm::ToolDef;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Tool trait
// ---------------------------------------------------------------------------

pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDef;
    fn execute(&self, input: &Value) -> Result<String, String>;
}

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    /// Global write lock: all data-mutating tools acquire this before writing.
    /// This prevents concurrent writes from the agent and Tauri commands.
    /// Shared via Arc so Tauri commands can also use it (see `data_write_lock()`).
    write_lock: Mutex<()>,
}

impl ToolRegistry {
    pub fn new(data_dir: &Path) -> Self {
        let dir = data_dir.to_path_buf();
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(GetContextTool(dir.clone())),
            Box::new(UpdateMissionTool(dir.clone())),
            Box::new(UpdateStatusTool(dir.clone())),
            Box::new(UpdateAchievementTool(dir.clone())),
            Box::new(WriteChangelogTool(dir.clone())),
            Box::new(ReadFileTool(dir.clone())),
        ];
        Self {
            tools,
            write_lock: Mutex::new(()),
        }
    }

    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    pub fn execute(&self, name: &str, input: &Value) -> Result<String, String> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.definition().name == name)
            .ok_or_else(|| format!("Unknown tool: {name}"))?;

        // Write tools acquire the lock; read-only tools skip it.
        let is_write = matches!(
            name,
            "update_mission" | "update_status" | "update_achievement" | "write_changelog"
        );

        if is_write {
            let _guard = self
                .write_lock
                .lock()
                .map_err(|e| format!("Write lock poisoned: {e}"))?;
            tool.execute(input)
        } else {
            tool.execute(input)
        }
    }
}

// ---------------------------------------------------------------------------
// get_context — read an overview of missions, status, achievements, memory
// ---------------------------------------------------------------------------

struct GetContextTool(PathBuf);

impl Tool for GetContextTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "get_context".into(),
            description: "Read RealityMod context: active missions, status metrics, achievement progress, and mission memory. Call this first to understand the user's current state.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    fn execute(&self, _input: &Value) -> Result<String, String> {
        let mut sections = Vec::new();

        // Missions
        let missions_path = self.0.join("missions.json");
        if missions_path.exists() {
            let file: MissionFile = read_json_file(&missions_path)?;
            let active: Vec<_> = file.missions.iter().filter(|m| m.status == "active").collect();
            let proposed: Vec<_> = file.missions.iter().filter(|m| m.status == "proposed").collect();
            sections.push(format!(
                "## Missions\nActive: {}\nProposed: {}\n\n{}",
                active.len(),
                proposed.len(),
                serde_json::to_string_pretty(
                    &file.missions.iter()
                        .filter(|m| m.status == "active" || m.status == "proposed")
                        .collect::<Vec<_>>()
                ).unwrap_or_default()
            ));
            // main_menu
            sections.push(format!(
                "## Main Menu Config\n{}",
                serde_json::to_string_pretty(&file.main_menu).unwrap_or_default()
            ));
        }

        // Status metrics
        let status_path = self.0.join("status.json");
        if status_path.exists() {
            let values: StatusValueFile = read_json_file(&status_path)?;
            sections.push(format!(
                "## Status Metrics\n{}",
                serde_json::to_string_pretty(&values.metrics).unwrap_or_default()
            ));
        }

        // Metric definitions (id + name + unit for AI to match)
        let defs_path = self.0.join("status_metric_definitions.json");
        if defs_path.exists() {
            let defs: MetricDefinitionFile = read_json_file(&defs_path)?;
            let summary: Vec<Value> = defs.metrics.iter().map(|m| {
                json!({"id": m.id, "name": m.name, "unit": m.unit, "description": m.description})
            }).collect();
            sections.push(format!(
                "## Metric Definitions\n{}",
                serde_json::to_string_pretty(&summary).unwrap_or_default()
            ));
        }

        // Achievement progress
        let progress_path = self.0.join("achievement_progress.json");
        if progress_path.exists() {
            let progress: AchievementProgressFile = read_json_file(&progress_path)?;
            if !progress.achievements.is_empty() {
                sections.push(format!(
                    "## Achievement Progress\n{}",
                    serde_json::to_string_pretty(&progress.achievements).unwrap_or_default()
                ));
            }
        }

        // Mission memory
        let memory_path = self.0.join("mission_memory.json");
        if memory_path.exists() {
            let memory: Value = read_json_file(&memory_path)?;
            sections.push(format!(
                "## Mission Memory\n{}",
                serde_json::to_string_pretty(&memory).unwrap_or_default()
            ));
        }

        if sections.is_empty() {
            Ok("No data files found. This appears to be a fresh setup.".into())
        } else {
            Ok(sections.join("\n\n"))
        }
    }
}

// ---------------------------------------------------------------------------
// read_file — read any file under data/ (for achievements, packs, etc.)
// ---------------------------------------------------------------------------

struct ReadFileTool(PathBuf);

impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "read_file".into(),
            description: "Read a file relative to the data directory. Use for reading pack achievements, skill definitions, or any data file. Path must be relative (e.g., 'packs/programmer/achievements.json').".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative path under data/ directory"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, input: &Value) -> Result<String, String> {
        let rel_path = input["path"].as_str().ok_or("Missing 'path'")?;

        let safe_path = sandbox_path(&self.0, rel_path)?;

        std::fs::read_to_string(&safe_path)
            .map_err(|e| format!("Failed to read {rel_path}: {e}"))
    }
}

// ---------------------------------------------------------------------------
// update_mission — update mission progress, status, or main_menu config
// ---------------------------------------------------------------------------

struct UpdateMissionTool(PathBuf);

impl Tool for UpdateMissionTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_mission".into(),
            description: "Update a mission's fields or the main_menu config in missions.json. Can update progress, status, completed_at, or the main_menu countdown/progress display.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mission_id": {
                        "type": "string",
                        "description": "Mission ID to update (omit if only updating main_menu)"
                    },
                    "updates": {
                        "type": "object",
                        "description": "Fields to update on the mission: progress (0-100), status (proposed/active/completed/archived/rejected), completed_at (ISO 8601)"
                    },
                    "main_menu": {
                        "type": "object",
                        "description": "Optional main_menu config update. Keys: countdown, progress. Each value: {mission_id, label} or null to clear.",
                        "properties": {
                            "countdown": {},
                            "progress": {}
                        }
                    }
                },
                "required": []
            }),
        }
    }

    fn execute(&self, input: &Value) -> Result<String, String> {
        let missions_path = self.0.join("missions.json");
        let mut file: MissionFile = read_json_file(&missions_path)?;
        let mut changes = Vec::new();

        // Update mission fields
        if let Some(id) = input["mission_id"].as_str() {
            let mission = file
                .missions
                .iter_mut()
                .find(|m| m.id == id)
                .ok_or_else(|| format!("Mission '{id}' not found"))?;

            if let Some(updates) = input["updates"].as_object() {
                for (key, val) in updates {
                    match key.as_str() {
                        "progress" => {
                            let old = mission.progress;
                            mission.progress = val.as_u64().map(|v| v as u32);
                            changes.push(format!("{id}.progress: {old:?} → {:?}", mission.progress));
                        }
                        "status" => {
                            let old = mission.status.clone();
                            if let Some(s) = val.as_str() {
                                let valid = ["proposed", "active", "completed", "archived", "rejected"];
                                if !valid.contains(&s) {
                                    return Err(format!("Invalid status '{s}'"));
                                }
                                mission.status = s.to_string();
                                changes.push(format!("{id}.status: {old} → {s}"));
                            }
                        }
                        "completed_at" => {
                            if let Some(s) = val.as_str() {
                                mission.completed_at = Some(s.to_string());
                                changes.push(format!("{id}.completed_at: set to {s}"));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update main_menu
        if let Some(menu) = input.get("main_menu") {
            if menu.get("countdown").is_some() {
                if menu["countdown"].is_null() {
                    file.main_menu.countdown = None;
                    changes.push("main_menu.countdown: cleared".into());
                } else {
                    file.main_menu.countdown = serde_json::from_value(menu["countdown"].clone()).ok();
                    changes.push(format!("main_menu.countdown: updated"));
                }
            }
            if menu.get("progress").is_some() {
                if menu["progress"].is_null() {
                    file.main_menu.progress = None;
                    changes.push("main_menu.progress: cleared".into());
                } else {
                    file.main_menu.progress = serde_json::from_value(menu["progress"].clone()).ok();
                    changes.push(format!("main_menu.progress: updated"));
                }
            }
        }

        if changes.is_empty() {
            return Ok("No changes made.".into());
        }

        write_json_file(&missions_path, &file)?;
        Ok(format!("Updated missions.json:\n- {}", changes.join("\n- ")))
    }
}

// ---------------------------------------------------------------------------
// update_status — update status metric values
// ---------------------------------------------------------------------------

struct UpdateStatusTool(PathBuf);

impl Tool for UpdateStatusTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_status".into(),
            description: "Update one or more status metric values in status.json. Use metric IDs from the metric definitions.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "metrics": {
                        "type": "object",
                        "description": "Map of metric_id → new numeric value, e.g. {\"weight_kg\": 75.2, \"bench_press_5rm_kg\": 90}",
                        "additionalProperties": { "type": "number" }
                    }
                },
                "required": ["metrics"]
            }),
        }
    }

    fn execute(&self, input: &Value) -> Result<String, String> {
        let status_path = self.0.join("status.json");
        let mut file: StatusValueFile = read_json_file(&status_path)?;

        // Validate metric IDs against definitions
        let defs_path = self.0.join("status_metric_definitions.json");
        let defs: MetricDefinitionFile = read_json_file(&defs_path)?;
        let valid_ids: std::collections::HashSet<&str> =
            defs.metrics.iter().map(|m| m.id.as_str()).collect();

        let updates = input["metrics"]
            .as_object()
            .ok_or("'metrics' must be an object")?;

        let mut changes = Vec::new();
        for (id, val) in updates {
            if !valid_ids.contains(id.as_str()) {
                return Err(format!("Unknown metric ID: '{id}'. Valid IDs: {valid_ids:?}"));
            }
            let new_val = val.as_f64().ok_or(format!("Value for '{id}' must be a number"))?;
            let old_val = file.metrics.get(id).copied();
            file.metrics.insert(id.clone(), new_val);
            changes.push(format!("{id}: {old_val:?} → {new_val}"));
        }

        write_json_file(&status_path, &file)?;
        Ok(format!("Updated status.json:\n- {}", changes.join("\n- ")))
    }
}

// ---------------------------------------------------------------------------
// update_achievement — track or achieve an achievement
// ---------------------------------------------------------------------------

struct UpdateAchievementTool(PathBuf);

impl Tool for UpdateAchievementTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_achievement".into(),
            description: "Update achievement progress. Set status to 'tracked' (partial progress) or 'achieved' (complete). Append to progress_detail as needed.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "achievement_id": {
                        "type": "string",
                        "description": "Achievement ID, e.g. 'programmer::rust_proficient'"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["tracked", "achieved"],
                        "description": "Achievement status"
                    },
                    "progress_detail": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "New progress entries to append (not replace)"
                    },
                    "note": {
                        "type": "string",
                        "description": "Optional note"
                    },
                    "may_be_incomplete": {
                        "type": "boolean",
                        "description": "Set true if user likely has unreported prior progress"
                    }
                },
                "required": ["achievement_id", "status"]
            }),
        }
    }

    fn execute(&self, input: &Value) -> Result<String, String> {
        let progress_path = self.0.join("achievement_progress.json");

        let mut file: AchievementProgressFile = if progress_path.exists() {
            read_json_file(&progress_path)?
        } else {
            AchievementProgressFile {
                version: 1,
                achievements: HashMap::new(),
            }
        };

        let id = input["achievement_id"]
            .as_str()
            .ok_or("Missing 'achievement_id'")?;
        let new_status = input["status"].as_str().ok_or("Missing 'status'")?;

        // Validate achievement exists in loaded packs
        let packs_path = self.0.join("loaded_packs.json");
        if packs_path.exists() {
            let packs: LoadedPacksFile = read_json_file(&packs_path)?;
            let pack_id = id.split("::").next().unwrap_or("");
            if !packs.packs.contains(&pack_id.to_string()) {
                return Err(format!(
                    "Pack '{pack_id}' not in loaded_packs.json. Cannot track achievement '{id}'."
                ));
            }
            // Validate achievement ID exists in pack
            let ach_path = self.0.join("packs").join(pack_id).join("achievements.json");
            if ach_path.exists() {
                let ach_file: AchievementFile = read_json_file(&ach_path)?;
                if !ach_file.achievements.iter().any(|a| a.id == id) {
                    return Err(format!("Achievement '{id}' not found in pack '{pack_id}'."));
                }
            }
        }

        let now = current_iso8601();

        // Get or create entry
        let entry = file.achievements.entry(id.to_string()).or_insert_with(|| {
            crate::models::achievement::AchievementProgress {
                status: crate::models::achievement::AchievementStatus::Tracked,
                achieved_at: None,
                tracked_at: Some(now.clone()),
                note: None,
                progress_detail: Vec::new(),
                may_be_incomplete: None,
            }
        });

        // Update status
        let old_status = format!("{:?}", entry.status);
        match new_status {
            "tracked" => {
                entry.status = crate::models::achievement::AchievementStatus::Tracked;
                if entry.tracked_at.is_none() {
                    entry.tracked_at = Some(now);
                }
            }
            "achieved" => {
                entry.status = crate::models::achievement::AchievementStatus::Achieved;
                entry.achieved_at = Some(now);
            }
            _ => return Err(format!("Invalid status: '{new_status}'")),
        }

        // Append progress_detail
        if let Some(details) = input["progress_detail"].as_array() {
            for d in details {
                if let Some(s) = d.as_str() {
                    entry.progress_detail.push(s.to_string());
                }
            }
        }

        if let Some(note) = input["note"].as_str() {
            entry.note = Some(note.to_string());
        }
        if let Some(inc) = input["may_be_incomplete"].as_bool() {
            entry.may_be_incomplete = Some(inc);
        }

        write_json_file(&progress_path, &file)?;
        Ok(format!(
            "Updated achievement '{id}': {old_status} → {new_status}"
        ))
    }
}

// ---------------------------------------------------------------------------
// write_changelog — append an entry to ai_changelog.json
// ---------------------------------------------------------------------------

struct WriteChangelogTool(PathBuf);

impl Tool for WriteChangelogTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "write_changelog".into(),
            description: "Append an entry to ai_changelog.json. MANDATORY after every data modification. Include old_value for rollback support.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "Human-readable summary of all changes"
                    },
                    "changes": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "file": { "type": "string" },
                                "type": { "type": "string" },
                                "target": { "type": "string" },
                                "field": { "type": "string" },
                                "old_value": {},
                                "new_value": {}
                            },
                            "required": ["file", "type", "target", "field", "new_value"]
                        },
                        "description": "List of individual changes with old_value for rollback"
                    }
                },
                "required": ["summary", "changes"]
            }),
        }
    }

    fn execute(&self, input: &Value) -> Result<String, String> {
        let changelog_path = self.0.join("ai_changelog.json");

        let mut entries: Vec<Value> = if changelog_path.exists() {
            read_json_file(&changelog_path)?
        } else {
            Vec::new()
        };

        let entry = json!({
            "timestamp": current_iso8601(),
            "skill": "agent",
            "summary": input["summary"],
            "changes": input["changes"],
        });

        entries.push(entry);

        // FIFO: max 200 entries
        while entries.len() > 200 {
            entries.remove(0);
        }

        write_json_file(&changelog_path, &entries)?;
        Ok("Changelog entry written.".into())
    }
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Validate that a relative path resolves to a location within the sandbox (data_dir).
/// Prevents path traversal, absolute paths, and symlink escapes.
fn sandbox_path(data_dir: &Path, rel_path: &str) -> Result<PathBuf, String> {
    // Reject absolute paths
    if Path::new(rel_path).is_absolute() {
        return Err("Absolute paths not allowed".into());
    }

    // Reject obvious traversal
    if rel_path.contains("..") {
        return Err("Path traversal not allowed".into());
    }

    let joined = data_dir.join(rel_path);

    // Canonicalize both to resolve symlinks, then verify containment
    let canon_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve data dir: {e}"))?;
    let canon_path = joined
        .canonicalize()
        .map_err(|_| format!("File not found: {rel_path}"))?;

    if !canon_path.starts_with(&canon_dir) {
        return Err("Access denied: path escapes data directory".into());
    }

    Ok(canon_path)
}

fn current_iso8601() -> String {
    // Use system time to produce ISO 8601 with timezone offset.
    // We avoid adding chrono as a dep; produce UTC timestamp.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Simple UTC formatting
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    // Days since epoch to Y-M-D (civil calendar)
    let (y, m, d) = epoch_days_to_civil(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

pub fn epoch_days_to_civil(days: i64) -> (i32, u32, u32) {
    // Inverse of days_from_civil, from Howard Hinnant's algorithm
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}
