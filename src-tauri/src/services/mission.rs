use crate::models::mission::{Mission, MissionArchiveFile, MissionFile};
use crate::storage::json_store::{read_json_file, write_json_file};
use crate::storage::validate::validate_data_file;
use serde_json::Value;
use std::fs;
use std::path::Path;

const CURRENT_STATUSES: &[&str] = &["proposed", "active"];
const ARCHIVE_STATUSES: &[&str] = &["completed", "archived", "rejected"];
const VALID_STATUSES: &[&str] = &["proposed", "active", "completed", "archived", "rejected"];

fn current_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("missions.json")
}

fn archive_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("mission_archive.json")
}

fn read_current(data_dir: &Path) -> Result<MissionFile, String> {
    let path = current_path(data_dir);
    if path.exists() {
        read_json_file(&path)
    } else {
        Ok(MissionFile {
            version: 1,
            missions: Vec::new(),
            main_menu: Default::default(),
        })
    }
}

fn read_archive(data_dir: &Path) -> Result<MissionArchiveFile, String> {
    let path = archive_path(data_dir);
    if path.exists() {
        read_json_file(&path)
    } else {
        Ok(MissionArchiveFile {
            version: 1,
            missions: Vec::new(),
        })
    }
}

fn is_current_status(status: &str) -> bool {
    CURRENT_STATUSES.contains(&status)
}

fn is_archive_status(status: &str) -> bool {
    ARCHIVE_STATUSES.contains(&status)
}

fn remove_main_menu_refs(file: &mut MissionFile, id: &str) {
    if file
        .main_menu
        .countdown
        .as_ref()
        .is_some_and(|cd| cd.mission_id == id)
    {
        file.main_menu.countdown = None;
    }
    if file
        .main_menu
        .progress
        .as_ref()
        .is_some_and(|pg| pg.mission_id == id)
    {
        file.main_menu.progress = None;
    }
    file.main_menu.hints.retain(|h| h.mission_id != id);
}

fn normalize_split(file: &mut MissionFile, archive: &mut MissionArchiveFile) {
    let mut index = 0;
    while index < file.missions.len() {
        if is_archive_status(&file.missions[index].status) {
            let mission = file.missions.remove(index);
            remove_main_menu_refs(file, &mission.id);
            archive.missions.retain(|m| m.id != mission.id);
            archive.missions.push(mission);
        } else {
            index += 1;
        }
    }

    let mut index = 0;
    while index < archive.missions.len() {
        if is_current_status(&archive.missions[index].status) {
            let mission = archive.missions.remove(index);
            file.missions.retain(|m| m.id != mission.id);
            file.missions.push(mission);
        } else {
            index += 1;
        }
    }
}

fn restore_file(path: &Path, backup: &Option<Vec<u8>>) {
    if let Some(bytes) = backup {
        let _ = fs::write(path, bytes);
    } else {
        let _ = fs::remove_file(path);
    }
}

fn write_mission_files(
    data_dir: &Path,
    file: &MissionFile,
    archive: &MissionArchiveFile,
) -> Result<(), String> {
    let current_value =
        serde_json::to_value(file).map_err(|e| format!("Failed to serialize missions: {e}"))?;
    validate_data_file("missions.json", &current_value)
        .map_err(|e| format!("Validation failed: {e}"))?;

    let archive_value = serde_json::to_value(archive)
        .map_err(|e| format!("Failed to serialize mission archive: {e}"))?;
    validate_data_file("mission_archive.json", &archive_value)
        .map_err(|e| format!("Validation failed: {e}"))?;

    let current_path = current_path(data_dir);
    let archive_path = archive_path(data_dir);
    let current_backup = fs::read(&current_path).ok();
    let archive_backup = fs::read(&archive_path).ok();

    if let Err(e) = write_json_file(&current_path, file) {
        restore_file(&current_path, &current_backup);
        return Err(e);
    }

    if let Err(e) = write_json_file(&archive_path, archive) {
        restore_file(&current_path, &current_backup);
        restore_file(&archive_path, &archive_backup);
        return Err(e);
    }

    Ok(())
}

fn apply_mission_updates(
    mission: &mut Mission,
    id: &str,
    updates: &serde_json::Map<String, Value>,
    changes: &mut Vec<String>,
) -> Result<(), String> {
    let mut warnings = Vec::new();
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
                changes.push(format!("{id}.progress: {old:?} -> {:?}", mission.progress));
            }
            "status" => {
                let old = mission.status.clone();
                if let Some(s) = val.as_str() {
                    if !VALID_STATUSES.contains(&s) {
                        return Err(format!("Invalid status '{s}'"));
                    }
                    mission.status = s.to_string();
                    changes.push(format!("{id}.status: {old} -> {s}"));
                }
            }
            "difficulty" => {
                let old = mission.difficulty.clone();
                if val.is_null() {
                    mission.difficulty = None;
                    changes.push(format!("{id}.difficulty: {old:?} -> cleared"));
                } else if let Some(s) = val.as_str() {
                    let valid = ["S", "A", "B", "C", "D"];
                    if !valid.contains(&s) {
                        return Err(format!(
                            "Invalid difficulty '{s}', must be one of {valid:?}"
                        ));
                    }
                    mission.difficulty = Some(s.to_string());
                    changes.push(format!("{id}.difficulty: {old:?} -> {s:?}"));
                }
            }
            "completed_at" => {
                if let Some(s) = val.as_str() {
                    mission.completed_at = Some(s.to_string());
                    changes.push(format!("{id}.completed_at: set to {s}"));
                }
            }
            "short_desc" => {
                let old = mission.short_desc.clone();
                if val.is_null() {
                    mission.short_desc = None;
                    changes.push(format!("{id}.short_desc: {old:?} -> cleared"));
                } else if let Some(s) = val.as_str() {
                    mission.short_desc = Some(s.to_string());
                    changes.push(format!("{id}.short_desc: {old:?} -> {s:?}"));
                }
            }
            "deadline" => {
                let old = mission.deadline.clone();
                if let Some(s) = val.as_str() {
                    mission.deadline = Some(s.to_string());
                    changes.push(format!("{id}.deadline: {old:?} -> {s}"));
                }
            }
            "title" => {
                if let Some(s) = val.as_str() {
                    let old = mission.title.clone();
                    mission.title = s.to_string();
                    changes.push(format!("{id}.title: {old} -> {s}"));
                }
            }
            "description" => {
                if let Some(s) = val.as_str() {
                    mission.description = Some(s.to_string());
                    changes.push(format!("{id}.description: updated"));
                }
            }
            "linked_achievement_id" => {
                if val.is_null() {
                    mission.linked_achievement_id = None;
                    changes.push(format!("{id}.linked_achievement_id: cleared"));
                } else if let Some(s) = val.as_str() {
                    mission.linked_achievement_id = Some(s.to_string());
                    changes.push(format!("{id}.linked_achievement_id: set to {s}"));
                }
            }
            "parent_id" => {
                if val.is_null() {
                    mission.parent_id = None;
                    changes.push(format!("{id}.parent_id: cleared"));
                } else if let Some(s) = val.as_str() {
                    mission.parent_id = Some(s.to_string());
                    changes.push(format!("{id}.parent_id: set to {s}"));
                }
            }
            "ai_metadata" => {
                mission.ai_metadata = Some(val.clone());
                changes.push(format!("{id}.ai_metadata: updated"));
            }
            other => {
                warnings.push(format!("unknown field '{other}' ignored"));
            }
        }
    }
    if !warnings.is_empty() {
        changes.push(format!("warnings: {}", warnings.join(", ")));
    }
    Ok(())
}

pub fn update_mission(data_dir: &Path, input: &Value) -> Result<String, String> {
    let mut file = read_current(data_dir)?;
    let mut archive = read_archive(data_dir)?;
    normalize_split(&mut file, &mut archive);
    let mut changes = Vec::new();

    if let Some(id) = input["mission_id"].as_str() {
        if let Some(updates) = input["updates"].as_object() {
            if let Some(index) = file.missions.iter().position(|m| m.id == id) {
                apply_mission_updates(&mut file.missions[index], id, updates, &mut changes)?;
                if is_archive_status(&file.missions[index].status) {
                    let mission = file.missions.remove(index);
                    remove_main_menu_refs(&mut file, id);
                    archive.missions.retain(|m| m.id != id);
                    archive.missions.push(mission);
                    changes.push(format!("{id}: moved to mission_archive.json"));
                }
            } else if let Some(index) = archive.missions.iter().position(|m| m.id == id) {
                apply_mission_updates(&mut archive.missions[index], id, updates, &mut changes)?;
                if is_current_status(&archive.missions[index].status) {
                    let mission = archive.missions.remove(index);
                    file.missions.retain(|m| m.id != id);
                    file.missions.push(mission);
                    changes.push(format!("{id}: restored to missions.json"));
                }
            } else {
                return Err(format!("Mission '{id}' not found"));
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
        if menu.get("hints").is_some() {
            if menu["hints"].is_null() {
                file.main_menu.hints = vec![];
                changes.push("main_menu.hints: cleared".into());
            } else {
                file.main_menu.hints =
                    serde_json::from_value(menu["hints"].clone()).unwrap_or_default();
                changes.push("main_menu.hints: updated".into());
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

    write_mission_files(data_dir, &file, &archive)?;
    Ok(format!(
        "Updated mission files:\n- {}",
        changes.join("\n- ")
    ))
}

pub fn create_mission(data_dir: &Path, input: &Value) -> Result<String, String> {
    let mut file = read_current(data_dir)?;
    let mut archive = read_archive(data_dir)?;
    normalize_split(&mut file, &mut archive);

    let id = input["id"].as_str().ok_or("Missing required field 'id'")?;
    let title = input["title"]
        .as_str()
        .ok_or("Missing required field 'title'")?;
    let status = input["status"]
        .as_str()
        .ok_or("Missing required field 'status'")?;

    if !VALID_STATUSES.contains(&status) {
        return Err(format!("Invalid status '{status}'"));
    }

    // Check uniqueness
    if file.missions.iter().any(|m| m.id == id) || archive.missions.iter().any(|m| m.id == id) {
        return Err(format!("Mission '{id}' already exists"));
    }

    if let Some(d) = input["difficulty"].as_str() {
        let valid = ["S", "A", "B", "C", "D"];
        if !valid.contains(&d) {
            return Err(format!(
                "Invalid difficulty '{d}', must be one of {valid:?}"
            ));
        }
    }

    let progress = if let Some(p) = input["progress"].as_u64() {
        if p > 100 {
            return Err(format!("progress must be 0-100, got {p}"));
        }
        Some(p as u32)
    } else {
        None
    };

    let mission = Mission {
        id: id.to_string(),
        title: title.to_string(),
        description: input["description"].as_str().map(|s| s.to_string()),
        status: status.to_string(),
        progress,
        difficulty: input["difficulty"].as_str().map(|s| s.to_string()),
        deadline: input["deadline"].as_str().map(|s| s.to_string()),
        short_desc: input["short_desc"].as_str().map(|s| s.to_string()),
        linked_achievement_id: input["linked_achievement_id"]
            .as_str()
            .map(|s| s.to_string()),
        created_at: input["created_at"].as_str().map(|s| s.to_string()),
        completed_at: None,
        parent_id: input["parent_id"].as_str().map(|s| s.to_string()),
        ai_metadata: input.get("ai_metadata").cloned(),
    };

    if is_archive_status(status) {
        archive.missions.push(mission);
    } else {
        file.missions.push(mission);
    }
    write_mission_files(data_dir, &file, &archive)?;
    Ok(format!("Created mission '{id}' with status '{status}'"))
}

pub fn delete_mission(data_dir: &Path, id: &str) -> Result<String, String> {
    let mut file = read_current(data_dir)?;
    let mut archive = read_archive(data_dir)?;
    normalize_split(&mut file, &mut archive);

    let original_current_len = file.missions.len();
    let original_archive_len = archive.missions.len();
    file.missions.retain(|m| m.id != id);
    archive.missions.retain(|m| m.id != id);

    if file.missions.len() == original_current_len && archive.missions.len() == original_archive_len
    {
        return Err(format!("Mission '{id}' not found"));
    }

    remove_main_menu_refs(&mut file, id);

    write_mission_files(data_dir, &file, &archive)?;
    Ok(format!("Deleted mission '{id}'"))
}
