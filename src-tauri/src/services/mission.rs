use crate::models::mission::{Mission, MissionFile};
use crate::storage::json_store::{read_json_file, write_and_validate};
use serde_json::Value;
use std::path::Path;

pub fn update_mission(data_dir: &Path, input: &Value) -> Result<String, String> {
    let missions_path = data_dir.join("missions.json");
    let mut file: MissionFile = read_json_file(&missions_path)?;
    let mut changes = Vec::new();

    if let Some(id) = input["mission_id"].as_str() {
        let mission = file
            .missions
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| format!("Mission '{id}' not found"))?;

        if let Some(updates) = input["updates"].as_object() {
            for (key, val) in updates {
                match key.as_str() {
                    "progress" => {
                        let old = mission.progress;
                        if let Some(p) = val.as_u64() {
                            if p > 100 {
                                return Err(format!("progress must be 0-100, got {p}"));
                            }
                            mission.progress = Some(p as u32);
                        }
                        changes.push(format!("{id}.progress: {old:?} → {:?}", mission.progress));
                    }
                    "status" => {
                        let old = mission.status.clone();
                        if let Some(s) = val.as_str() {
                            let valid = ["proposed", "active", "completed", "archived", "rejected"];
                            if !valid.contains(&s) {
                                return Err(format!("Invalid status '{s}'"));
                            }
                            mission.status = s.to_string();
                            changes.push(format!("{id}.status: {old} → {s}"));
                        }
                    }
                    "completed_at" => {
                        if let Some(s) = val.as_str() {
                            mission.completed_at = Some(s.to_string());
                            changes.push(format!("{id}.completed_at: set to {s}"));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Update main_menu
    if let Some(menu) = input.get("main_menu") {
        if menu.get("countdown").is_some() {
            if menu["countdown"].is_null() {
                file.main_menu.countdown = None;
                changes.push("main_menu.countdown: cleared".into());
            } else {
                file.main_menu.countdown = serde_json::from_value(menu["countdown"].clone()).ok();
                changes.push("main_menu.countdown: updated".into());
            }
        }
        if menu.get("progress").is_some() {
            if menu["progress"].is_null() {
                file.main_menu.progress = None;
                changes.push("main_menu.progress: cleared".into());
            } else {
                file.main_menu.progress = serde_json::from_value(menu["progress"].clone()).ok();
                changes.push("main_menu.progress: updated".into());
            }
        }
    }

    if changes.is_empty() {
        return Ok("No changes made.".into());
    }

    write_and_validate(&missions_path, &file, "missions.json")?;
    Ok(format!(
        "Updated missions.json:\n- {}",
        changes.join("\n- ")
    ))
}

pub fn create_mission(data_dir: &Path, input: &Value) -> Result<String, String> {
    let missions_path = data_dir.join("missions.json");
    let mut file: MissionFile = if missions_path.exists() {
        read_json_file(&missions_path)?
    } else {
        MissionFile {
            version: 1,
            missions: Vec::new(),
            main_menu: Default::default(),
        }
    };

    let id = input["id"].as_str().ok_or("Missing required field 'id'")?;
    let title = input["title"]
        .as_str()
        .ok_or("Missing required field 'title'")?;
    let status = input["status"]
        .as_str()
        .ok_or("Missing required field 'status'")?;

    let valid_statuses = ["proposed", "active", "completed", "archived", "rejected"];
    if !valid_statuses.contains(&status) {
        return Err(format!("Invalid status '{status}'"));
    }

    // Check uniqueness
    if file.missions.iter().any(|m| m.id == id) {
        return Err(format!("Mission '{id}' already exists"));
    }

    let mission = Mission {
        id: id.to_string(),
        title: title.to_string(),
        description: input["description"].as_str().map(|s| s.to_string()),
        status: status.to_string(),
        progress: input["progress"].as_u64().map(|v| v as u32),
        deadline: input["deadline"].as_str().map(|s| s.to_string()),
        linked_achievement_id: input["linked_achievement_id"]
            .as_str()
            .map(|s| s.to_string()),
        created_at: input["created_at"].as_str().map(|s| s.to_string()),
        completed_at: None,
        ai_metadata: input.get("ai_metadata").cloned(),
    };

    file.missions.push(mission);
    write_and_validate(&missions_path, &file, "missions.json")?;
    Ok(format!("Created mission '{id}' with status '{status}'"))
}
