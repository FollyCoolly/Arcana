use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Deserialization types (from disk) ---

#[derive(Debug, Deserialize)]
pub struct MetricDefinitionFile {
    pub version: u32,
    pub metrics: Vec<MetricDefinition>,
    #[serde(default)]
    pub dimensions: Vec<DimensionDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct MetricDefinition {
    pub id: String,
    pub name: String,
    pub group: String,
    pub unit: String,
    pub value_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DimensionDefinition {
    pub id: String,
    pub name: String,
    pub level_titles: Vec<String>,
    pub level_thresholds: Vec<f64>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub metrics: HashMap<String, DimensionMetricConfig>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct DimensionMetricConfig {
    pub weight: f64,
    pub target_max: Option<f64>,
    pub target_min: Option<f64>,
    pub scoring_brackets: Option<Vec<ScoringBracket>>,
}

#[derive(Debug, Deserialize)]
pub struct ScoringBracket {
    pub min: f64,
    pub max: f64,
    pub score: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StatusValueFile {
    pub version: u32,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
pub struct UserProfile {
    pub username: String,
    pub birth_date: String,
}

// --- Serialization types (to frontend) ---

#[derive(Debug, Serialize)]
pub struct StatusMetric {
    pub id: String,
    pub name: String,
    pub group: String,
    pub unit: String,
    pub value_type: String,
    pub value: Option<f64>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DimensionMetricResult {
    pub metric_id: String,
    pub value: Option<f64>,
    pub contribution: Option<f64>,
    pub weight: f64,
}

#[derive(Debug, Serialize)]
pub struct DimensionData {
    pub id: String,
    pub name: String,
    pub level_titles: Vec<String>,
    pub level_thresholds: Vec<f64>,
    pub enabled: bool,
    pub score: Option<f64>,
    pub level: Option<u32>,
    pub level_title: Option<String>,
    pub metrics: Vec<DimensionMetricResult>,
}

#[derive(Debug, Serialize)]
pub struct StatusData {
    pub definition_version: u32,
    pub value_version: u32,
    pub username: String,
    pub game_days: Option<u64>,
    pub metrics: Vec<StatusMetric>,
    pub dimensions: Vec<DimensionData>,
    pub system_metrics: HashMap<String, f64>,
}
