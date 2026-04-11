use crate::services;

use super::llm::ToolDef;
use serde_json::{json, Value};
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
// Tool implementations — delegate to services
// ---------------------------------------------------------------------------

struct GetContextTool(PathBuf);
impl Tool for GetContextTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "get_context".into(),
            description: "Read RealityMod context: active missions, status metrics, achievement progress, and mission memory. Call this first to understand the user's current state.".into(),
            input_schema: json!({"type": "object", "properties": {}, "required": []}),
        }
    }
    fn execute(&self, _input: &Value) -> Result<String, String> {
        services::context::get_context(&self.0)
    }
}

struct ReadFileTool(PathBuf);
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "read_file".into(),
            description: "Read a file relative to the data directory. Path must be relative (e.g., 'packs/programmer/achievements.json').".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"path": {"type": "string", "description": "Relative path under data/ directory"}},
                "required": ["path"]
            }),
        }
    }
    fn execute(&self, input: &Value) -> Result<String, String> {
        let rel_path = input["path"].as_str().ok_or("Missing 'path'")?;
        services::file_access::read_sandboxed_file(&self.0, rel_path)
    }
}

struct UpdateMissionTool(PathBuf);
impl Tool for UpdateMissionTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_mission".into(),
            description: "Update a mission's fields or the main_menu config in missions.json."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "mission_id": {"type": "string", "description": "Mission ID to update"},
                    "updates": {"type": "object", "description": "Fields to update: progress (0-100), status, completed_at"},
                    "main_menu": {"type": "object", "description": "main_menu config update"}
                },
                "required": []
            }),
        }
    }
    fn execute(&self, input: &Value) -> Result<String, String> {
        services::mission::update_mission(&self.0, input)
    }
}

struct UpdateStatusTool(PathBuf);
impl Tool for UpdateStatusTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_status".into(),
            description: "Update one or more status metric values in status.json.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"metrics": {"type": "object", "description": "Map of metric_id → numeric value", "additionalProperties": {"type": "number"}}},
                "required": ["metrics"]
            }),
        }
    }
    fn execute(&self, input: &Value) -> Result<String, String> {
        services::status::update_status(&self.0, input)
    }
}

struct UpdateAchievementTool(PathBuf);
impl Tool for UpdateAchievementTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "update_achievement".into(),
            description: "Update achievement progress. Set status to 'tracked' or 'achieved'. Append to progress_detail.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "achievement_id": {"type": "string"},
                    "status": {"type": "string", "enum": ["tracked", "achieved"]},
                    "progress_detail": {"type": "array", "items": {"type": "string"}},
                    "note": {"type": "string"},
                    "may_be_incomplete": {"type": "boolean"}
                },
                "required": ["achievement_id", "status"]
            }),
        }
    }
    fn execute(&self, input: &Value) -> Result<String, String> {
        services::achievement::update_achievement(&self.0, input)
    }
}

struct WriteChangelogTool(PathBuf);
impl Tool for WriteChangelogTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "write_changelog".into(),
            description:
                "Append an entry to ai_changelog.json. MANDATORY after every data modification."
                    .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "summary": {"type": "string"},
                    "changes": {"type": "array", "items": {"type": "object"}}
                },
                "required": ["summary", "changes"]
            }),
        }
    }
    fn execute(&self, input: &Value) -> Result<String, String> {
        // Agent always uses "agent" as skill identifier
        services::changelog::write_changelog(&self.0, "agent", input)
    }
}
