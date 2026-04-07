use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct MetricDefinitionFile {
    pub version: u32,
    pub metrics: Vec<MetricDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct MetricDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub group: String,
    #[serde(default)]
    pub sub_group: Option<String>,
    pub unit: String,
    pub value_type: String,
    pub target_max: Option<f64>,
    pub target_min: Option<f64>,
    pub body_parts: Option<HashMap<String, f64>>,
    pub description: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct StatusMetric {
    pub id: String,
    pub name: String,
    pub category: String,
    pub group: String,
    pub sub_group: Option<String>,
    pub unit: String,
    pub value_type: String,
    pub value: Option<f64>,
    pub target_max: Option<f64>,
    pub target_min: Option<f64>,
    pub body_parts: HashMap<String, f64>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusData {
    pub definition_version: u32,
    pub value_version: u32,
    pub username: String,
    pub game_days: Option<u64>,
    pub bmi: Option<f64>,
    pub metrics: Vec<StatusMetric>,
}
