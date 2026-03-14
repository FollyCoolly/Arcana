use serde::{Deserialize, Serialize};

// --- Deserialization structs (from JSON files) ---

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SkillFile {
    pub version: u32,
    pub skills: Vec<SkillDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub max_level: u32,
    pub level_thresholds: Vec<LevelThreshold>,
    pub nodes: Vec<SkillNode>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillNode {
    pub node_id: String,
    pub achievement_id: String,
    pub points: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LevelThreshold {
    pub level: u32,
    pub points_required: u32,
    #[serde(default)]
    pub required_key_achievements: Vec<String>,
}

// --- Response structs (sent to frontend) ---

#[derive(Debug, Serialize)]
pub struct SkillWithLevel {
    pub skill: SkillDef,
    pub pack_id: String,
    pub pack_name: String,
    pub current_level: u32,
    pub current_points: u32,
    pub max_points: u32,
    pub next_threshold: Option<LevelThreshold>,
}

#[derive(Debug, Serialize)]
pub struct SkillData {
    pub skills: Vec<SkillWithLevel>,
}
