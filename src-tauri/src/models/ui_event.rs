use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Deserialization structs (from JSON files) ---

#[derive(Debug, Deserialize, Serialize)]
pub struct UiEventsFile {
    pub version: u32,
    pub events: Vec<UiEvent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UiEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub data: Value,
}
