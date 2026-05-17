use crate::models::mission::{
    CountdownDisplay, HintDisplay, MainMenuHintRef, MainMenuMissionData, MainMenuRef, Mission,
    MissionArchiveFile, MissionData, MissionFile, MissionResponse, ProgressDisplay,
};
use crate::services;
use crate::storage::date_utils::{days_from_civil, parse_date, today_epoch_days};
use crate::storage::json_store::{read_json_file, resolve_data_dir};
use serde_json::json;

#[tauri::command]
pub fn load_missions() -> Result<MissionData, String> {
    let data_dir = resolve_data_dir()?;
    let missions_path = data_dir.join("missions.json");
    let archive_path = data_dir.join("mission_archive.json");

    if !missions_path.exists() && !archive_path.exists() {
        return Ok(MissionData { missions: vec![] });
    }

    let file: MissionFile = if missions_path.exists() {
        read_json_file(&missions_path)?
    } else {
        MissionFile {
            version: 1,
            missions: vec![],
            main_menu: Default::default(),
        }
    };
    let archive: MissionArchiveFile = if archive_path.exists() {
        read_json_file(&archive_path)?
    } else {
        MissionArchiveFile {
            version: 1,
            missions: vec![],
        }
    };

    let missions = file
        .missions
        .into_iter()
        .chain(archive.missions)
        .filter(|m| m.status != "rejected")
        .map(|m| {
            let days_remaining = m
                .deadline
                .as_deref()
                .and_then(|d| compute_days_remaining(d).ok());
            let difficulty = m.difficulty.clone();
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
        short_desc: mission
            .short_desc
            .clone()
            .unwrap_or_else(|| mission.title.clone()),
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
    services::mission::update_mission(
        &data_dir,
        &json!({
            "mission_id": id,
            "updates": {
                "status": new_status,
            }
        }),
    )
    .map(|_| ())
}

fn resolve_hints(missions: &[Mission], hints: &[MainMenuHintRef]) -> Vec<HintDisplay> {
    hints
        .iter()
        .filter_map(|h| {
            missions
                .iter()
                .find(|m| m.id == h.mission_id && m.status == "active")
                .map(|m| HintDisplay {
                    short_desc: m.short_desc.clone().unwrap_or_else(|| m.title.clone()),
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
