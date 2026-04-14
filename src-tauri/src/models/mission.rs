use serde::{Deserialize, Serialize};

// --- Deserialization structs (from missions.json) ---

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct MissionFile {
    pub version: u32,
    pub missions: Vec<Mission>,
    #[serde(default)]
    pub main_menu: MainMenuConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct Mission {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: String,
    #[serde(default)]
    pub progress: Option<u32>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub linked_achievement_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub ai_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MainMenuConfig {
    #[serde(default)]
    pub countdown: Option<MainMenuRef>,
    #[serde(default)]
    pub progress: Option<MainMenuRef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MainMenuRef {
    pub mission_id: String,
    pub label: String,
}

// --- Response structs (sent to frontend) ---

#[derive(Debug, Serialize)]
pub struct MissionResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub progress: Option<u32>,
    pub deadline: Option<String>,
    pub linked_achievement_id: Option<String>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    pub days_remaining: Option<i64>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MissionData {
    pub missions: Vec<MissionResponse>,
}

#[derive(Debug, Serialize)]
pub struct MainMenuMissionData {
    pub countdown: Option<CountdownDisplay>,
    pub progress: Option<ProgressDisplay>,
}

#[derive(Debug, Serialize)]
pub struct CountdownDisplay {
    pub label: String,
    pub days_remaining: i64,
}

#[derive(Debug, Serialize)]
pub struct ProgressDisplay {
    pub label: String,
    pub progress: u32,
}
