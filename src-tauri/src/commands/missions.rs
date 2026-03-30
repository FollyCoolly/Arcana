use crate::models::mission::*;
use crate::storage::date_utils::{days_from_civil, parse_date, today_epoch_days};
use crate::storage::json_store::{read_json_file, resolve_data_dir};

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
        .map(|m| {
            let days_remaining = m
                .deadline
                .as_deref()
                .and_then(|d| compute_days_remaining(d).ok());
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
                days_remaining,
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
            progress: None,
        });
    }

    let file: MissionFile = read_json_file(&missions_path)?;

    let countdown = file
        .main_menu
        .countdown
        .and_then(|ref_data| resolve_countdown(&file.missions, ref_data));

    let progress = file
        .main_menu
        .progress
        .and_then(|ref_data| resolve_progress(&file.missions, ref_data));

    Ok(MainMenuMissionData {
        countdown,
        progress,
    })
}

fn resolve_countdown(missions: &[Mission], ref_data: MainMenuRef) -> Option<CountdownDisplay> {
    let mission = missions
        .iter()
        .find(|m| m.id == ref_data.mission_id && m.status == "active")?;

    let deadline = mission.deadline.as_ref()?;
    let days_remaining = compute_days_remaining(deadline).ok()?;

    if days_remaining < 0 {
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

fn compute_days_remaining(deadline: &str) -> Result<i64, String> {
    let (year, month, day) = parse_date(deadline)?;
    let epoch_base = days_from_civil(1970, 1, 1);
    let deadline_days = days_from_civil(year, month, day) - epoch_base;
    let today_days = today_epoch_days()?;
    Ok(deadline_days - today_days)
}
