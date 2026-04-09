use crate::storage::json_store::{read_json_file, write_and_validate};
use serde_json::{json, Value};
use std::path::Path;

pub fn update_mission_memory(data_dir: &Path, input: &Value) -> Result<String, String> {
    let memory_path = data_dir.join("mission_memory.json");

    let mut data: Value = if memory_path.exists() {
        read_json_file(&memory_path)?
    } else {
        json!({
            "version": 1,
            "focus_areas": [],
            "patterns": {"accepted_tags": [], "rejected_tags": [], "notes": ""},
            "conversation_context": [],
            "completed_mission_log": []
        })
    };

    let mut changes = Vec::new();

    // Replace last_generation
    if let Some(lg) = input.get("last_generation") {
        data["last_generation"] = lg.clone();
        changes.push("last_generation: updated");
    }

    // Replace focus_areas
    if let Some(fa) = input.get("focus_areas") {
        data["focus_areas"] = fa.clone();
        changes.push("focus_areas: replaced");
    }

    // Replace patterns
    if let Some(p) = input.get("patterns") {
        data["patterns"] = p.clone();
        changes.push("patterns: replaced");
    }

    // Append to conversation_context (FIFO, max 20)
    if let Some(new_entries) = input
        .get("append_conversation_context")
        .and_then(|v| v.as_array())
    {
        let ctx = data["conversation_context"]
            .as_array_mut()
            .ok_or("conversation_context must be an array")?;
        ctx.extend(new_entries.iter().cloned());
        while ctx.len() > 20 {
            ctx.remove(0);
        }
        changes.push("conversation_context: appended");
    }

    // Append to completed_mission_log (FIFO, max 50)
    if let Some(new_entries) = input
        .get("append_completed_mission_log")
        .and_then(|v| v.as_array())
    {
        let log = data["completed_mission_log"]
            .as_array_mut()
            .ok_or("completed_mission_log must be an array")?;
        log.extend(new_entries.iter().cloned());
        while log.len() > 50 {
            log.remove(0);
        }
        changes.push("completed_mission_log: appended");
    }

    if changes.is_empty() {
        // Read-only: return current state
        return Ok(serde_json::to_string_pretty(&data).unwrap_or_default());
    }

    write_and_validate(&memory_path, &data, "mission_memory.json")?;
    Ok(format!(
        "Updated mission_memory.json:\n- {}",
        changes.join("\n- ")
    ))
}
