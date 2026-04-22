use crate::models::ui_event::{UiEvent, UiEventsFile};
use crate::storage::date_utils::current_iso8601;
use crate::storage::json_store::{read_json_file, write_json_file};
use serde_json::Value;
use std::path::Path;

const MAX_EVENTS: usize = 100;

fn events_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("ui_events.json")
}

fn load_or_create(data_dir: &Path) -> UiEventsFile {
    let path = events_path(data_dir);
    if path.exists() {
        read_json_file(&path).unwrap_or(UiEventsFile {
            version: 1,
            events: Vec::new(),
        })
    } else {
        UiEventsFile {
            version: 1,
            events: Vec::new(),
        }
    }
}

fn generate_event_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let nanos = now.subsec_nanos();
    // Use lower 16 bits of nanos as a pseudo-random suffix
    let suffix = format!("{:04x}", nanos & 0xFFFF);
    format!("evt_{secs}_{suffix}")
}

pub fn emit_event(data_dir: &Path, event_type: &str, data: Value) -> Result<(), String> {
    let mut file = load_or_create(data_dir);

    let event = UiEvent {
        id: generate_event_id(),
        event_type: event_type.to_string(),
        timestamp: current_iso8601(),
        data,
    };

    file.events.push(event);

    // FIFO eviction
    if file.events.len() > MAX_EVENTS {
        let drain_count = file.events.len() - MAX_EVENTS;
        file.events.drain(..drain_count);
    }

    write_json_file(&events_path(data_dir), &file)
}

pub fn consume_events(data_dir: &Path, event_type: Option<&str>) -> Result<Vec<UiEvent>, String> {
    let path = events_path(data_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut file: UiEventsFile = read_json_file(&path)?;

    let (consumed, remaining): (Vec<UiEvent>, Vec<UiEvent>) = match event_type {
        Some(t) => file.events.into_iter().partition(|e| e.event_type == t),
        None => (file.events, Vec::new()),
    };

    file.events = remaining;
    write_json_file(&path, &file)?;

    Ok(consumed)
}
