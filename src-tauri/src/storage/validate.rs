//! Shared data validation — pure logic, no I/O.
//!
//! Rules mirror `scripts/validate_data.py` so that Rust agent, Tauri commands,
//! and arcana-data CLI all enforce the same constraints.

use serde_json::Value;
use std::collections::HashSet;

const MISSION_STATUSES: &[&str] = &["proposed", "active", "completed", "archived", "rejected"];
const ACHIEVEMENT_STATUSES: &[&str] = &["tracked", "achieved"];
const CHANGELOG_SKILLS: &[&str] = &["velvet-room", "phan-site", "agent"];
const CHANGELOG_CHANGE_TYPES: &[&str] = &["add", "update", "delete"];

/// Validate a data file by name. Returns `Ok(())` or a human-readable error.
pub fn validate_data_file(file_name: &str, data: &Value) -> Result<(), String> {
    match file_name {
        "missions.json" => validate_missions(data),
        "achievement_progress.json" => validate_achievement_progress(data),
        "ai_changelog.json" => validate_changelog(data),
        "status.json" => validate_status(data),
        "mission_memory.json" => validate_mission_memory(data),
        _ => Ok(()), // unknown files pass through
    }
}

// ---------------------------------------------------------------------------
// missions.json
// ---------------------------------------------------------------------------

fn validate_missions(data: &Value) -> Result<(), String> {
    require_version(data, "missions.json")?;

    let missions = data
        .get("missions")
        .and_then(|v| v.as_array())
        .ok_or("missions.json: 'missions' must be an array")?;

    let mut seen_ids = HashSet::new();
    for (i, m) in missions.iter().enumerate() {
        let prefix = format!("missions.json: missions[{i}]");

        for field in &["id", "title", "status"] {
            if m.get(*field).and_then(|v| v.as_str()).is_none() {
                return Err(format!("{prefix}: missing required field '{field}'"));
            }
        }

        let id = m["id"].as_str().unwrap();
        if !seen_ids.insert(id) {
            return Err(format!("{prefix}: duplicate mission id '{id}'"));
        }

        let status = m["status"].as_str().unwrap();
        if !MISSION_STATUSES.contains(&status) {
            return Err(format!(
                "{prefix}: invalid status '{status}', must be one of {MISSION_STATUSES:?}"
            ));
        }

        if let Some(progress) = m.get("progress") {
            if !progress.is_null() {
                let p = progress
                    .as_u64()
                    .ok_or(format!("{prefix}: progress must be a non-negative integer"))?;
                if p > 100 {
                    return Err(format!("{prefix}: progress must be 0-100, got {p}"));
                }
            }
        }
    }

    // main_menu references
    if let Some(menu) = data.get("main_menu").and_then(|v| v.as_object()) {
        for widget in &["countdown", "progress"] {
            if let Some(w) = menu.get(*widget).and_then(|v| v.as_object()) {
                if let Some(ref_id) = w.get("mission_id").and_then(|v| v.as_str()) {
                    if !seen_ids.contains(ref_id) {
                        return Err(format!(
                            "missions.json: main_menu.{widget}.mission_id '{ref_id}' not found in missions"
                        ));
                    }
                }
            }
        }
        if let Some(hints) = menu.get("hints").and_then(|v| v.as_array()) {
            for (i, h) in hints.iter().enumerate() {
                if let Some(ref_id) = h.get("mission_id").and_then(|v| v.as_str()) {
                    if !seen_ids.contains(ref_id) {
                        return Err(format!(
                            "missions.json: main_menu.hints[{i}].mission_id '{ref_id}' not found in missions"
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// achievement_progress.json
// ---------------------------------------------------------------------------

fn validate_achievement_progress(data: &Value) -> Result<(), String> {
    require_version(data, "achievement_progress.json")?;

    let achievements = data
        .get("achievements")
        .and_then(|v| v.as_object())
        .ok_or("achievement_progress.json: 'achievements' must be an object")?;

    for (aid, entry) in achievements {
        let status = entry.get("status").and_then(|v| v.as_str()).ok_or(format!(
            "achievement_progress.json: achievements['{aid}'].status must be a string"
        ))?;
        if !ACHIEVEMENT_STATUSES.contains(&status) {
            return Err(format!(
                "achievement_progress.json: achievements['{aid}'].status '{status}' must be one of {ACHIEVEMENT_STATUSES:?}"
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// ai_changelog.json
// ---------------------------------------------------------------------------

fn validate_changelog(data: &Value) -> Result<(), String> {
    require_version(data, "ai_changelog.json")?;

    let entries = data
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or("ai_changelog.json: 'entries' must be an array")?;

    if entries.len() > 200 {
        return Err(format!(
            "ai_changelog.json: entries count {} exceeds max 200",
            entries.len()
        ));
    }

    for (i, entry) in entries.iter().enumerate() {
        let prefix = format!("ai_changelog.json: entries[{i}]");

        for field in &["timestamp", "skill", "changes"] {
            if entry.get(*field).is_none() {
                return Err(format!("{prefix}: missing required field '{field}'"));
            }
        }

        let skill = entry["skill"]
            .as_str()
            .ok_or(format!("{prefix}: 'skill' must be a string"))?;
        if !CHANGELOG_SKILLS.contains(&skill) {
            return Err(format!(
                "{prefix}: invalid skill '{skill}', must be one of {CHANGELOG_SKILLS:?}"
            ));
        }

        let changes = entry["changes"]
            .as_array()
            .ok_or(format!("{prefix}: 'changes' must be an array"))?;

        for (j, change) in changes.iter().enumerate() {
            let cprefix = format!("{prefix}.changes[{j}]");
            let ctype = change
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or(format!("{cprefix}: missing or invalid 'type'"))?;

            if !CHANGELOG_CHANGE_TYPES.contains(&ctype) {
                return Err(format!(
                    "{cprefix}: invalid type '{ctype}', must be one of {CHANGELOG_CHANGE_TYPES:?}"
                ));
            }

            if ctype == "update" && change.get("old_value").is_none() {
                return Err(format!(
                    "{cprefix}: 'update' type change must have 'old_value' for rollback"
                ));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// status.json
// ---------------------------------------------------------------------------

fn validate_status(data: &Value) -> Result<(), String> {
    require_version(data, "status.json")?;

    let metrics = data
        .get("metrics")
        .and_then(|v| v.as_object())
        .ok_or("status.json: 'metrics' must be an object")?;

    for (key, val) in metrics {
        if !val.is_number() {
            return Err(format!(
                "status.json: metrics['{key}'] must be a number, got {}",
                value_type_name(val)
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// mission_memory.json
// ---------------------------------------------------------------------------

fn validate_mission_memory(data: &Value) -> Result<(), String> {
    require_version(data, "mission_memory.json")?;

    if let Some(ctx) = data.get("conversation_context").and_then(|v| v.as_array()) {
        if ctx.len() > 20 {
            return Err(format!(
                "mission_memory.json: conversation_context has {} entries, max 20",
                ctx.len()
            ));
        }
    }

    if let Some(log) = data.get("completed_mission_log").and_then(|v| v.as_array()) {
        if log.len() > 50 {
            return Err(format!(
                "mission_memory.json: completed_mission_log has {} entries, max 50",
                log.len()
            ));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_version(data: &Value, file_name: &str) -> Result<(), String> {
    if data.get("version").is_none() {
        return Err(format!("{file_name}: missing 'version' field"));
    }
    Ok(())
}

fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_missions() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "Test", "status": "active", "progress": 50}
            ],
            "main_menu": {"countdown": null, "progress": null}
        });
        assert!(validate_data_file("missions.json", &data).is_ok());
    }

    #[test]
    fn missions_invalid_status() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "Test", "status": "invalid"}
            ]
        });
        assert!(validate_data_file("missions.json", &data).is_err());
    }

    #[test]
    fn missions_progress_out_of_range() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "Test", "status": "active", "progress": 150}
            ]
        });
        assert!(validate_data_file("missions.json", &data).is_err());
    }

    #[test]
    fn missions_duplicate_id() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "A", "status": "active"},
                {"id": "m1", "title": "B", "status": "proposed"}
            ]
        });
        assert!(validate_data_file("missions.json", &data).is_err());
    }

    #[test]
    fn missions_hints_bad_ref() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "Test", "status": "active"}
            ],
            "main_menu": {
                "hints": [{"mission_id": "nonexistent", "short_desc": "something"}]
            }
        });
        assert!(validate_data_file("missions.json", &data).is_err());
    }

    #[test]
    fn missions_hints_valid_ref() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "Test", "status": "active"}
            ],
            "main_menu": {
                "hints": [{"mission_id": "m1", "short_desc": "Do the thing"}]
            }
        });
        assert!(validate_data_file("missions.json", &data).is_ok());
    }

    #[test]
    fn missions_main_menu_bad_ref() {
        let data = json!({
            "version": 1,
            "missions": [
                {"id": "m1", "title": "Test", "status": "active"}
            ],
            "main_menu": {
                "countdown": {"mission_id": "nonexistent", "label": "X"}
            }
        });
        assert!(validate_data_file("missions.json", &data).is_err());
    }

    #[test]
    fn valid_achievement_progress() {
        let data = json!({
            "version": 1,
            "achievements": {
                "pack::a": {"status": "tracked"},
                "pack::b": {"status": "achieved"}
            }
        });
        assert!(validate_data_file("achievement_progress.json", &data).is_ok());
    }

    #[test]
    fn achievement_progress_bad_status() {
        let data = json!({
            "version": 1,
            "achievements": {
                "pack::a": {"status": "unknown"}
            }
        });
        assert!(validate_data_file("achievement_progress.json", &data).is_err());
    }

    #[test]
    fn valid_changelog() {
        let data = json!({
            "version": 1,
            "entries": [{
                "timestamp": "2026-04-09T00:00:00Z",
                "skill": "agent",
                "summary": "test",
                "changes": [{"file": "status.json", "type": "update", "target": "x", "field": "y", "old_value": 1, "new_value": 2}]
            }]
        });
        assert!(validate_data_file("ai_changelog.json", &data).is_ok());
    }

    #[test]
    fn changelog_bad_skill() {
        let data = json!({
            "version": 1,
            "entries": [{
                "timestamp": "2026-04-09T00:00:00Z",
                "skill": "invalid",
                "summary": "test",
                "changes": []
            }]
        });
        assert!(validate_data_file("ai_changelog.json", &data).is_err());
    }

    #[test]
    fn changelog_update_missing_old_value() {
        let data = json!({
            "version": 1,
            "entries": [{
                "timestamp": "2026-04-09T00:00:00Z",
                "skill": "agent",
                "summary": "test",
                "changes": [{"file": "f", "type": "update", "target": "t", "field": "f", "new_value": 1}]
            }]
        });
        assert!(validate_data_file("ai_changelog.json", &data).is_err());
    }

    #[test]
    fn valid_status() {
        let data = json!({"version": 1, "metrics": {"weight_kg": 75.2, "steps": 10000}});
        assert!(validate_data_file("status.json", &data).is_ok());
    }

    #[test]
    fn status_non_numeric() {
        let data = json!({"version": 1, "metrics": {"weight_kg": "not a number"}});
        assert!(validate_data_file("status.json", &data).is_err());
    }

    #[test]
    fn valid_mission_memory() {
        let data = json!({"version": 1, "conversation_context": [], "completed_mission_log": []});
        assert!(validate_data_file("mission_memory.json", &data).is_ok());
    }

    #[test]
    fn mission_memory_context_overflow() {
        let ctx: Vec<Value> = (0..21)
            .map(|i| json!({"date": "2026-01-01", "summary": format!("s{i}")}))
            .collect();
        let data = json!({"version": 1, "conversation_context": ctx});
        assert!(validate_data_file("mission_memory.json", &data).is_err());
    }

    #[test]
    fn unknown_file_passes() {
        let data = json!({"anything": true});
        assert!(validate_data_file("unknown.json", &data).is_ok());
    }
}
