use crate::models::mission::{
    CountdownDisplay, HintDisplay, MainMenuHintRef, MainMenuMissionData, MainMenuRef, Mission,
    MissionData, MissionFile, MissionResponse, ProgressDisplay,
};
use crate::storage::date_utils::{days_from_civil, parse_date, today_epoch_days};
use crate::storage::json_store::{read_json_file, resolve_data_dir, write_json_file};
use crate::storage::validate::validate_data_file;

#[tauri::command]
pub fn load_missions() -> Result<MissionData, String> {
    let data_dir = resolve_data_dir()?;
    let missions_path = data_dir.join("missions.json");

    if !missions_path.exists() {
        return Ok(MissionData { missions: vec![] });
    }

    let file: MissionFile = read_json_file(&missions_path)?;

    let missions = file
        .missions
        .into_iter()
        .filter(|m| m.status != "rejected")
        .map(|m| {
            let days_remaining = m
                .deadline
                .as_deref()
                .and_then(|d| compute_days_remaining(d).ok());
            let difficulty = m
                .ai_metadata
                .as_ref()
                .and_then(|meta| meta["difficulty_tier"].as_str())
                .map(|s| s.to_string());
            MissionResponse {
                id: m.id,
                title: m.title,
                description: m.description,
                status: m.status,
                progress: m.progress,
                deadline: m.deadline,
                linked_achievement_id: m.linked_achievement_id,
                created_at: m.created_at,
                completed_at: m.completed_at,
                parent_id: m.parent_id,
                days_remaining,
                difficulty,
            }
        })
        .collect();

    Ok(MissionData { missions })
}

#[tauri::command]
pub fn load_main_menu_missions() -> Result<MainMenuMissionData, String> {
    let data_dir = resolve_data_dir()?;
    let missions_path = data_dir.join("missions.json");

    if !missions_path.exists() {
        return Ok(MainMenuMissionData {
            countdown: None,
            hints: vec![],
            progress: None,
        });
    }

    let file: MissionFile = read_json_file(&missions_path)?;

    let countdown = file
        .main_menu
        .countdown
        .and_then(|ref_data| resolve_countdown(&file.missions, ref_data));

    let hints = resolve_hints(&file.missions, &file.main_menu.hints);

    let progress = file
        .main_menu
        .progress
        .and_then(|ref_data| resolve_progress(&file.missions, ref_data));

    Ok(MainMenuMissionData {
        countdown,
        hints,
        progress,
    })
}

fn resolve_countdown(missions: &[Mission], ref_data: MainMenuRef) -> Option<CountdownDisplay> {
    let mission = missions
        .iter()
        .find(|m| m.id == ref_data.mission_id && m.status == "active")?;

    let deadline = mission.deadline.as_ref()?;
    let days_remaining = compute_days_remaining(deadline).ok()?;

    if !(0..=99).contains(&days_remaining) {
        return None;
    }

    Some(CountdownDisplay {
        label: ref_data.label,
        days_remaining,
    })
}

fn resolve_progress(missions: &[Mission], ref_data: MainMenuRef) -> Option<ProgressDisplay> {
    let mission = missions
        .iter()
        .find(|m| m.id == ref_data.mission_id && m.status == "active")?;

    Some(ProgressDisplay {
        label: ref_data.label,
        progress: mission.progress.unwrap_or(0),
    })
}

#[tauri::command]
pub fn update_mission_status(id: String, new_status: String) -> Result<(), String> {
    let valid = ["proposed", "active", "completed", "archived", "rejected"];
    if !valid.contains(&new_status.as_str()) {
        return Err(format!(
            "Invalid status '{}'. Must be one of: {:?}",
            new_status, valid
        ));
    }

    let data_dir = resolve_data_dir()?;
    let missions_path = data_dir.join("missions.json");

    let mut file: MissionFile = read_json_file(&missions_path)?;

    let mission = file
        .missions
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("Mission '{}' not found", id))?;

    mission.status = new_status;
    write_json_file(&missions_path, &file)?;

    // Post-write validation (shared rules with agent)
    let written: serde_json::Value = read_json_file(&missions_path)?;
    if let Err(e) = validate_data_file("missions.json", &written) {
        // This should not happen since status was pre-checked, but guard against drift
        return Err(format!("Post-write validation failed: {e}"));
    }

    Ok(())
}

fn resolve_hints(missions: &[Mission], hints: &[MainMenuHintRef]) -> Vec<HintDisplay> {
    hints
        .iter()
        .filter_map(|h| {
            missions
                .iter()
                .find(|m| m.id == h.mission_id && m.status == "active")
                .map(|m| HintDisplay {
                    short_desc: m
                        .short_desc
                        .clone()
                        .unwrap_or_else(|| m.title.clone()),
                })
        })
        .take(2)
        .collect()
}

fn compute_days_remaining(deadline: &str) -> Result<i64, String> {
    let (year, month, day) = parse_date(deadline)?;
    let epoch_base = days_from_civil(1970, 1, 1);
    let deadline_days = days_from_civil(year, month, day) - epoch_base;
    let today_days = today_epoch_days()?;
    Ok(deadline_days - today_days)
}
