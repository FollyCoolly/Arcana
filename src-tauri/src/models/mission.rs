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
pub struct MissionArchiveFile {
    pub version: u32,
    pub missions: Vec<Mission>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub difficulty: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub short_desc: Option<String>,
    #[serde(default)]
    pub linked_achievement_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub ai_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MainMenuConfig {
    #[serde(default)]
    pub countdown: Option<MainMenuRef>,
    #[serde(default)]
    pub hints: Vec<MainMenuHintRef>,
    #[serde(default)]
    pub progress: Option<MainMenuRef>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MainMenuRef {
    pub mission_id: String,
    pub label: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MainMenuHintRef {
    pub mission_id: String,
}
