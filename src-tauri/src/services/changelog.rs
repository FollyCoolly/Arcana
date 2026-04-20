use crate::storage::date_utils::current_iso8601;
use crate::storage::json_store::{read_json_file, write_and_validate};
use serde_json::{json, Value};
use std::path::Path;

/// Write a changelog entry. `skill` is provided by the caller:
/// - Agent tools pass `"agent"`
/// - CLI callers pass their skill name (`"velvet-room"`, `"phan-site"`, etc.)
pub fn write_changelog(data_dir: &Path, skill: &str, input: &Value) -> Result<String, String> {
    let changelog_path = data_dir.join("ai_changelog.json");

    let mut entries: Vec<Value> = if changelog_path.exists() {
        let raw: Value = read_json_file(&changelog_path)?;
        if let Some(arr) = raw.as_array() {
            // Migration: legacy bare array format
            arr.clone()
        } else if let Some(obj) = raw.as_object() {
            obj.get("entries")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let entry = json!({
        "timestamp": current_iso8601(),
        "skill": skill,
        "summary": input["summary"],
        "changes": input["changes"],
    });

    entries.push(entry);

    while entries.len() > 200 {
        entries.remove(0);
    }

    let file = json!({
        "version": 1,
        "entries": entries,
    });

    write_and_validate(&changelog_path, &file, "ai_changelog.json")?;
    Ok("Changelog entry written.".into())
}
