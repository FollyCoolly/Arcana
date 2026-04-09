use rmcp::schemars;

use reality_mod_lib::services;
use reality_mod_lib::storage::json_store::resolve_data_dir;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ReadFileInput {
    /// Relative path under data/ directory (e.g., "packs/programmer/achievements.json")
    path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct UpdateMissionInput {
    /// Mission ID to update (omit if only updating main_menu)
    mission_id: Option<String>,
    /// Fields to update: progress (0-100), status (proposed/active/completed/archived/rejected), completed_at (ISO 8601)
    updates: Option<Value>,
    /// Main menu config update. Keys: countdown, progress. Each: {mission_id, label} or null to clear
    main_menu: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CreateMissionInput {
    /// Unique mission ID, e.g. ai_20260409_quest_slug
    id: String,
    /// Gamified quest-style title
    title: String,
    /// Initial status — typically "proposed"
    status: String,
    /// Detailed description
    description: Option<String>,
    /// Initial progress 0-100
    progress: Option<u32>,
    /// Deadline date YYYY-MM-DD
    deadline: Option<String>,
    /// Achievement ID to link (must exist in a loaded pack)
    linked_achievement_id: Option<String>,
    /// ISO 8601 creation timestamp
    created_at: Option<String>,
    /// AI generation metadata
    ai_metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct UpdateStatusInput {
    /// Map of metric_id to new numeric value, e.g. {"weight_kg": 75.2}
    metrics: HashMap<String, f64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct UpdateAchievementInput {
    /// Achievement ID, e.g. "programmer::rust_proficient"
    achievement_id: String,
    /// "tracked" (partial progress) or "achieved" (complete)
    status: String,
    /// New progress entries to append (never replaces existing)
    progress_detail: Option<Vec<String>>,
    /// Optional note
    note: Option<String>,
    /// Set true if user likely has unreported prior progress
    may_be_incomplete: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WriteChangelogInput {
    /// Skill name: "velvet-room", "phan-site", or "agent"
    skill: String,
    /// Human-readable summary of all changes
    summary: String,
    /// List of individual changes with old_value for rollback
    changes: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct UpdateMissionMemoryInput {
    /// Replace last_generation section
    last_generation: Option<Value>,
    /// Replace focus_areas array
    focus_areas: Option<Value>,
    /// Replace patterns object
    patterns: Option<Value>,
    /// Entries to APPEND to conversation_context (max 20 total, FIFO)
    append_conversation_context: Option<Vec<Value>>,
    /// Entries to APPEND to completed_mission_log (max 50 total, FIFO)
    append_completed_mission_log: Option<Vec<Value>>,
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RealityModServer {
    data_dir: PathBuf,
    write_lock: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

fn ok(text: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn err(e: String) -> ErrorData {
    ErrorData::internal_error(e, None)
}

#[tool_router]
impl RealityModServer {
    fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            write_lock: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Read RealityMod context: active missions, status metrics, achievement progress, and mission memory. Call this first to understand the user's current state.")]
    fn get_context(&self) -> Result<CallToolResult, ErrorData> {
        ok(services::context::get_context(&self.data_dir).map_err(err)?)
    }

    #[tool(description = "Read a file relative to the data/ directory. Sandboxed: rejects absolute paths, path traversal, and symlink escapes. Use for reading pack files, definitions, etc.")]
    fn read_file(
        &self,
        Parameters(input): Parameters<ReadFileInput>,
    ) -> Result<CallToolResult, ErrorData> {
        ok(services::file_access::read_sandboxed_file(&self.data_dir, &input.path).map_err(err)?)
    }

    #[tool(description = "Update a mission's fields or the main_menu config in missions.json. Can update progress (0-100), status (proposed/active/completed/archived/rejected), completed_at, or main_menu display.")]
    fn update_mission(
        &self,
        Parameters(input): Parameters<UpdateMissionInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().map_err(|e| err(e.to_string()))?;
        let val = serde_json::to_value(input).map_err(|e| err(e.to_string()))?;
        ok(services::mission::update_mission(&self.data_dir, &val).map_err(err)?)
    }

    #[tool(description = "Insert a new mission into missions.json. Used to create mission proposals. The id must be unique. Status is typically 'proposed'. Validates via missions.json schema rules.")]
    fn create_mission(
        &self,
        Parameters(input): Parameters<CreateMissionInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().map_err(|e| err(e.to_string()))?;
        let val = serde_json::to_value(input).map_err(|e| err(e.to_string()))?;
        ok(services::mission::create_mission(&self.data_dir, &val).map_err(err)?)
    }

    #[tool(description = "Update one or more status metric values in status.json. Validates metric IDs against status_metric_definitions.json. All values must be numeric.")]
    fn update_status(
        &self,
        Parameters(input): Parameters<UpdateStatusInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().map_err(|e| err(e.to_string()))?;
        let val = json!({"metrics": input.metrics});
        ok(services::status::update_status(&self.data_dir, &val).map_err(err)?)
    }

    #[tool(description = "Update achievement progress. Set status to 'tracked' (partial) or 'achieved' (complete). Appends to progress_detail; never replaces. Validates against loaded packs.")]
    fn update_achievement(
        &self,
        Parameters(input): Parameters<UpdateAchievementInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().map_err(|e| err(e.to_string()))?;
        let val = serde_json::to_value(input).map_err(|e| err(e.to_string()))?;
        ok(services::achievement::update_achievement(&self.data_dir, &val).map_err(err)?)
    }

    #[tool(description = "Append an entry to ai_changelog.json. MANDATORY after every data modification. Include old_value in change entries for rollback support. skill must be 'velvet-room', 'phan-site', or 'agent'.")]
    fn write_changelog(
        &self,
        Parameters(input): Parameters<WriteChangelogInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.write_lock.lock().map_err(|e| err(e.to_string()))?;
        let val = json!({"summary": input.summary, "changes": input.changes});
        ok(services::changelog::write_changelog(&self.data_dir, &input.skill, &val).map_err(err)?)
    }

    #[tool(description = "Update mission_memory.json — the AI's persistent cross-session memory. Supports: replacing focus_areas/patterns/last_generation, appending to conversation_context (max 20, FIFO) and completed_mission_log (max 50, FIFO). Changes do NOT require a changelog entry. If called with no update fields, returns current memory state.")]
    fn update_mission_memory(
        &self,
        Parameters(input): Parameters<UpdateMissionMemoryInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let val = serde_json::to_value(input).map_err(|e| err(e.to_string()))?;
        ok(services::memory::update_mission_memory(&self.data_dir, &val).map_err(err)?)
    }
}

#[tool_handler]
impl ServerHandler for RealityModServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("realitymod", env!("CARGO_PKG_VERSION")))
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // CRITICAL: all logging to stderr — stdout is the JSON-RPC channel
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    let data_dir = match resolve_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[mcp-server] Fatal: {e}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "[mcp-server] Starting. data_dir = {}",
        data_dir.display()
    );

    let server = RealityModServer::new(data_dir);
    let service = match server.serve(rmcp::transport::stdio()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[mcp-server] Failed to start: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = service.waiting().await {
        eprintln!("[mcp-server] Error: {e}");
    }
}
