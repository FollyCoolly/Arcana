use std::collections::{HashMap, HashSet};

use serde_json::json;

use crate::models::achievement::*;
use crate::services::achievement as achievement_service;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

#[tauri::command]
pub fn load_achievements() -> Result<AchievementData, String> {
    let data_dir = resolve_data_dir()?;

    // 1. Read loaded packs list
    let loaded_packs_path = data_dir.join("loaded_packs.json");
    let loaded_packs: LoadedPacksFile = read_json_file(&loaded_packs_path)?;

    // 2. Load each pack's manifest + achievements
    let mut packs = Vec::new();
    let mut all_achievement_ids = HashSet::new();

    for pack_id in &loaded_packs.packs {
        let pack_dir = data_dir.join("packs").join(pack_id);

        let manifest_path = pack_dir.join("manifest.json");
        let achievements_path = pack_dir.join("achievements.json");

        // Skip packs without achievements file
        if !achievements_path.exists() {
            continue;
        }

        let manifest: PackManifest = read_json_file(&manifest_path)?;
        let achievement_file: AchievementFile = read_json_file(&achievements_path)?;

        // Validate: manifest.id must match directory name
        if manifest.id != *pack_id {
            return Err(format!(
                "Pack '{}': manifest.id '{}' does not match directory name",
                pack_id, manifest.id
            ));
        }

        // Validate each achievement
        let pack_prefix = format!("{}::", pack_id);
        let mut pack_achievement_ids = HashSet::new();

        for achievement in &achievement_file.achievements {
            // ID must start with pack_id::
            if !achievement.id.starts_with(&pack_prefix) {
                return Err(format!(
                    "Achievement '{}' in pack '{}' must start with '{}'",
                    achievement.id, pack_id, pack_prefix
                ));
            }

            // No duplicates within pack
            if !pack_achievement_ids.insert(achievement.id.clone()) {
                return Err(format!(
                    "Duplicate achievement id '{}' in pack '{}'",
                    achievement.id, pack_id
                ));
            }

            // No duplicates across packs
            if !all_achievement_ids.insert(achievement.id.clone()) {
                return Err(format!(
                    "Duplicate achievement id '{}' across packs",
                    achievement.id
                ));
            }
        }

        // Validate prerequisites reference valid IDs within this pack
        for achievement in &achievement_file.achievements {
            for prereq in &achievement.prerequisites {
                if !pack_achievement_ids.contains(prereq) {
                    return Err(format!(
                        "Achievement '{}' references unknown prerequisite '{}'",
                        achievement.id, prereq
                    ));
                }
            }
        }

        // Validate prerequisite DAG has no cycles
        if let Some(cycle_msg) = detect_cycle(&achievement_file.achievements) {
            return Err(format!("Pack '{}': {}", pack_id, cycle_msg));
        }

        packs.push(PackAchievements {
            pack_id: pack_id.clone(),
            pack_name: manifest.name,
            achievements: achievement_file.achievements,
        });
    }

    // 3. Read achievement progress
    let progress_path = data_dir.join("achievement_progress.json");
    let progress: AchievementProgressFile = read_json_file(&progress_path)?;

    Ok(AchievementData {
        packs,
        progress: progress.achievements,
    })
}

/// Detect cycles in the prerequisite graph using iterative DFS.
/// Returns a message describing the first cycle found, or None if acyclic.
fn detect_cycle(achievements: &[AchievementDef]) -> Option<String> {
    // Build adjacency: id -> list of prerequisite ids
    let adj: HashMap<&str, Vec<&str>> = achievements
        .iter()
        .map(|a| {
            (
                a.id.as_str(),
                a.prerequisites.iter().map(|p| p.as_str()).collect(),
            )
        })
        .collect();

    // 0 = unvisited, 1 = in current path, 2 = fully visited
    let mut state: HashMap<&str, u8> = adj.keys().map(|&id| (id, 0u8)).collect();

    for &start in adj.keys() {
        if state[start] == 2 {
            continue;
        }

        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        *state.get_mut(start).unwrap() = 1;

        while let Some((node, idx)) = stack.last_mut() {
            let prereqs = adj.get(*node).map(|v| v.as_slice()).unwrap_or(&[]);
            if *idx < prereqs.len() {
                let next = prereqs[*idx];
                *idx += 1;
                match state[next] {
                    0 => {
                        *state.get_mut(next).unwrap() = 1;
                        stack.push((next, 0));
                    }
                    1 => {
                        return Some(format!("prerequisite cycle detected involving '{}'", next));
                    }
                    _ => {} // already fully visited
                }
            } else {
                let finished = *node;
                *state.get_mut(finished).unwrap() = 2;
                stack.pop();
            }
        }
    }

    None
}

/// Mark an achievement as achieved. Validates that all direct prerequisites
/// are already achieved before updating.
#[tauri::command]
pub fn set_achievement_achieved(achievement_id: String) -> Result<String, String> {
    let data_dir = resolve_data_dir()?;

    // Prerequisite check: every direct prereq must be in progress with status=achieved.
    let pack_id = achievement_id.split("::").next().unwrap_or("");
    let ach_path = data_dir
        .join("packs")
        .join(pack_id)
        .join("achievements.json");
    if ach_path.exists() {
        let ach_file: AchievementFile = read_json_file(&ach_path)?;
        let def = ach_file
            .achievements
            .iter()
            .find(|a| a.id == achievement_id)
            .ok_or_else(|| format!("Achievement '{achievement_id}' not found."))?;

        if !def.prerequisites.is_empty() {
            let progress_path = data_dir.join("achievement_progress.json");
            let progress: AchievementProgressFile = if progress_path.exists() {
                read_json_file(&progress_path)?
            } else {
                AchievementProgressFile {
                    version: 1,
                    achievements: HashMap::new(),
                }
            };

            let missing: Vec<String> = def
                .prerequisites
                .iter()
                .filter(|p| {
                    !matches!(
                        progress.achievements.get(p.as_str()).map(|e| &e.status),
                        Some(AchievementStatus::Achieved)
                    )
                })
                .cloned()
                .collect();

            if !missing.is_empty() {
                return Err(format!(
                    "Prerequisites not met: {}",
                    missing.join(", ")
                ));
            }
        }
    }

    achievement_service::update_achievement(
        &data_dir,
        &json!({
            "achievement_id": achievement_id,
            "status": "achieved",
        }),
    )
}

/// Lock (un-achieve) an achievement. Preserves tracked context if any,
/// otherwise removes the progress entry entirely.
#[tauri::command]
pub fn lock_achievement(achievement_id: String) -> Result<String, String> {
    let data_dir = resolve_data_dir()?;
    achievement_service::lock_achievement(&data_dir, &achievement_id)
}
