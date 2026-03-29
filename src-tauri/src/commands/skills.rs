use std::collections::HashSet;

use crate::models::achievement::*;
use crate::models::skill::*;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

#[tauri::command]
pub fn load_skills() -> Result<SkillData, String> {
    let data_dir = resolve_data_dir()?;

    // 1. Read loaded packs list
    let loaded_packs_path = data_dir.join("loaded_packs.json");
    let loaded_packs: LoadedPacksFile = read_json_file(&loaded_packs_path)?;

    // 2. Read achievement progress → build set of unlocked IDs
    let progress_path = data_dir.join("achievement_progress.json");
    let progress: AchievementProgressFile = read_json_file(&progress_path)?;
    let unlocked_ids: HashSet<String> = progress.unlocked.keys().cloned().collect();

    // 3. For each pack: load manifest + achievements + skills, validate, compute levels
    let mut all_skills = Vec::new();

    for pack_id in &loaded_packs.packs {
        let pack_dir = data_dir.join("packs").join(pack_id);

        let skills_path = pack_dir.join("skills.json");
        // Skip packs without skills file
        if !skills_path.exists() {
            continue;
        }

        let manifest_path = pack_dir.join("manifest.json");
        let achievements_path = pack_dir.join("achievements.json");

        let manifest: PackManifest = read_json_file(&manifest_path)?;
        let skill_file: SkillFile = read_json_file(&skills_path)?;

        // Build set of valid achievement IDs in this pack (for validation)
        let pack_achievement_ids: HashSet<String> = if achievements_path.exists() {
            let ach_file: AchievementFile = read_json_file(&achievements_path)?;
            ach_file.achievements.iter().map(|a| a.id.clone()).collect()
        } else {
            HashSet::new()
        };

        let pack_prefix = format!("{}::", pack_id);

        for skill in skill_file.skills {
            // Validate: skill ID starts with pack_id::
            if !skill.id.starts_with(&pack_prefix) {
                return Err(format!(
                    "Skill '{}' in pack '{}' must start with '{}'",
                    skill.id, pack_id, pack_prefix
                ));
            }

            // Validate: level_thresholds count == max_level
            if skill.level_thresholds.len() as u32 != skill.max_level {
                return Err(format!(
                    "Skill '{}': level_thresholds count ({}) != max_level ({})",
                    skill.id,
                    skill.level_thresholds.len(),
                    skill.max_level
                ));
            }

            // Validate: points_required monotonically increasing
            for window in skill.level_thresholds.windows(2) {
                if window[1].points_required <= window[0].points_required {
                    return Err(format!(
                        "Skill '{}': level_thresholds points_required not monotonically increasing ({} >= {})",
                        skill.id, window[1].points_required, window[0].points_required
                    ));
                }
            }

            // Validate: node_id unique within skill
            let mut node_ids = HashSet::new();
            for node in &skill.nodes {
                if !node_ids.insert(&node.node_id) {
                    return Err(format!(
                        "Skill '{}': duplicate node_id '{}'",
                        skill.id, node.node_id
                    ));
                }

                // Validate: achievement_id references valid same-pack achievement
                if !pack_achievement_ids.contains(&node.achievement_id) {
                    return Err(format!(
                        "Skill '{}': node '{}' references unknown achievement '{}'",
                        skill.id, node.node_id, node.achievement_id
                    ));
                }
            }

            // Validate: required_key_achievements reference valid achievement IDs
            for threshold in &skill.level_thresholds {
                for key_ach in &threshold.required_key_achievements {
                    if !pack_achievement_ids.contains(key_ach) {
                        return Err(format!(
                            "Skill '{}': level {} required_key_achievement '{}' not found",
                            skill.id, threshold.level, key_ach
                        ));
                    }
                }
            }

            // Calculate level
            let total_points: u32 = skill
                .nodes
                .iter()
                .filter(|n| unlocked_ids.contains(&n.achievement_id))
                .map(|n| n.points)
                .sum();

            let max_points: u32 = skill.nodes.iter().map(|n| n.points).sum();

            let mut current_level: u32 = 0;
            let mut accumulated_keys: Vec<&str> = Vec::new();
            for threshold in &skill.level_thresholds {
                accumulated_keys.extend(
                    threshold
                        .required_key_achievements
                        .iter()
                        .map(|s| s.as_str()),
                );
                let all_keys_unlocked =
                    accumulated_keys.iter().all(|id| unlocked_ids.contains(*id));

                if total_points >= threshold.points_required && all_keys_unlocked {
                    current_level = threshold.level;
                } else {
                    break;
                }
            }

            let next_threshold = skill
                .level_thresholds
                .iter()
                .find(|t| t.level == current_level + 1)
                .cloned();

            all_skills.push(SkillWithLevel {
                skill,
                pack_id: pack_id.clone(),
                pack_name: manifest.name.clone(),
                current_level,
                current_points: total_points,
                max_points,
                next_threshold,
            });
        }
    }

    Ok(SkillData {
        skills: all_skills,
    })
}
