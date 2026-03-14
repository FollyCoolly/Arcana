use std::collections::HashSet;

use crate::models::achievement::*;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

const VALID_DIFFICULTIES: &[&str] = &["beginner", "intermediate", "advanced", "expert", "legendary"];

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

            // Valid difficulty
            if !VALID_DIFFICULTIES.contains(&achievement.difficulty.as_str()) {
                return Err(format!(
                    "Achievement '{}' has invalid difficulty '{}'. Must be one of: {}",
                    achievement.id,
                    achievement.difficulty,
                    VALID_DIFFICULTIES.join(", ")
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
        progress: progress.unlocked,
    })
}
