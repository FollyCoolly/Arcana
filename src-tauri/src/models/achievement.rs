use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Deserialization structs (from JSON files) ---

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
    Legendary,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AchievementFile {
    pub version: u32,
    pub achievements: Vec<AchievementDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AchievementDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: Difficulty,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AchievementProgressFile {
    pub version: u32,
    pub unlocked: HashMap<String, UnlockInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnlockInfo {
    #[serde(default)]
    pub achieved_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct LoadedPacksFile {
    pub version: u32,
    pub packs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PackManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

// --- Response structs (sent to frontend) ---

#[derive(Debug, Serialize)]
pub struct AchievementData {
    pub packs: Vec<PackAchievements>,
    pub progress: HashMap<String, UnlockInfo>,
}

#[derive(Debug, Serialize)]
pub struct PackAchievements {
    pub pack_id: String,
    pub pack_name: String,
    pub achievements: Vec<AchievementDef>,
}
