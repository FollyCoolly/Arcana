use crate::models::achievement::{
    AchievementFile, AchievementProgress, AchievementProgressFile, AchievementStatus,
    LoadedPacksFile,
};
use crate::storage::date_utils::current_iso8601;
use crate::storage::json_store::{read_json_file, write_and_validate};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub fn update_achievement(data_dir: &Path, input: &Value) -> Result<String, String> {
    let progress_path = data_dir.join("achievement_progress.json");

    let mut file: AchievementProgressFile = if progress_path.exists() {
        read_json_file(&progress_path)?
    } else {
        AchievementProgressFile {
            version: 1,
            achievements: HashMap::new(),
        }
    };

    let id = input["achievement_id"]
        .as_str()
        .ok_or("Missing 'achievement_id'")?;
    let new_status = input["status"].as_str().ok_or("Missing 'status'")?;

    // Validate achievement exists in loaded packs
    let packs_path = data_dir.join("loaded_packs.json");
    if packs_path.exists() {
        let packs: LoadedPacksFile = read_json_file(&packs_path)?;
        let pack_id = id.split("::").next().unwrap_or("");
        if !packs.packs.contains(&pack_id.to_string()) {
            return Err(format!(
                "Pack '{pack_id}' not in loaded_packs.json. Cannot track achievement '{id}'."
            ));
        }
        let ach_path = data_dir
            .join("packs")
            .join(pack_id)
            .join("achievements.json");
        if ach_path.exists() {
            let ach_file: AchievementFile = read_json_file(&ach_path)?;
            if !ach_file.achievements.iter().any(|a| a.id == id) {
                return Err(format!("Achievement '{id}' not found in pack '{pack_id}'."));
            }
        }
    }

    let now = current_iso8601();

    let entry = file
        .achievements
        .entry(id.to_string())
        .or_insert_with(|| AchievementProgress {
            status: AchievementStatus::Tracked,
            achieved_at: None,
            tracked_at: Some(now.clone()),
            note: None,
            progress_detail: Vec::new(),
            may_be_incomplete: None,
        });

    let old_status = format!("{:?}", entry.status);
    match new_status {
        "tracked" => {
            entry.status = AchievementStatus::Tracked;
            if entry.tracked_at.is_none() {
                entry.tracked_at = Some(now);
            }
        }
        "achieved" => {
            entry.status = AchievementStatus::Achieved;
            entry.achieved_at = Some(now);
        }
        _ => return Err(format!("Invalid status: '{new_status}'")),
    }

    if let Some(details) = input["progress_detail"].as_array() {
        for d in details {
            if let Some(s) = d.as_str() {
                entry.progress_detail.push(s.to_string());
            }
        }
    }
    if let Some(note) = input["note"].as_str() {
        entry.note = Some(note.to_string());
    }
    if let Some(inc) = input["may_be_incomplete"].as_bool() {
        entry.may_be_incomplete = Some(inc);
    }

    write_and_validate(&progress_path, &file, "achievement_progress.json")?;
    Ok(format!(
        "Updated achievement '{id}': {old_status} → {new_status}"
    ))
}
