use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::models::achievement::*;
use crate::models::gallery::*;
use crate::models::skill::*;
use crate::models::status::*;
use crate::storage::date_utils::calculate_days_since;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

fn calculate_bmi(values: &HashMap<String, f64>) -> Option<f64> {
    let weight = values.get("weight_kg")?;
    let height_cm = values.get("height_cm")?;
    if *height_cm <= 0.0 {
        return None;
    }
    let height_m = height_cm / 100.0;
    Some(weight / (height_m * height_m))
}

fn compute_gallery_sys_metrics(data_dir: &Path) -> HashMap<String, f64> {
    let mut sys = HashMap::new();
    let sources_path = data_dir.join("gallery_sources.json");
    let source_file: GallerySourceFile = match read_json_file(&sources_path) {
        Ok(f) => f,
        Err(_) => return sys,
    };

    for source in &source_file.sources {
        let item_path = data_dir.join(&source.path);
        let count = read_json_file::<GalleryItemFile>(&item_path)
            .map(|f| f.items.len())
            .unwrap_or(0);

        let key = match source.media_type.as_str() {
            "anime" => "sys_anime_watched",
            "movie" => "sys_movies_watched",
            "book" => "sys_books_read",
            "game" => "sys_games_played",
            _ => continue,
        };
        *sys.entry(key.to_string()).or_insert(0.0) += count as f64;
    }
    sys
}

fn compute_skill_sys_metrics(data_dir: &Path) -> HashMap<String, f64> {
    let mut sys = HashMap::new();

    let loaded_packs: LoadedPacksFile = match read_json_file(&data_dir.join("loaded_packs.json")) {
        Ok(f) => f,
        Err(_) => return sys,
    };

    let progress: AchievementProgressFile =
        match read_json_file(&data_dir.join("achievement_progress.json")) {
            Ok(f) => f,
            Err(_) => return sys,
        };
    let unlocked_ids: HashSet<String> = progress.achievements.keys().cloned().collect();

    let mut lv_counts = [0u32; 6]; // index 1-5 used

    for pack_id in &loaded_packs.packs {
        let skills_path = data_dir.join("packs").join(pack_id).join("skills.json");
        let skill_file: SkillFile = match read_json_file(&skills_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        for skill in &skill_file.skills {
            let total_points: u32 = skill
                .nodes
                .iter()
                .filter(|n| unlocked_ids.contains(&n.achievement_id))
                .map(|n| n.points)
                .sum();

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

            let lv = (current_level as usize).min(5);
            // Count cumulatively: a lv3 skill counts for lv1, lv2, and lv3
            for l in 1..=lv {
                lv_counts[l] += 1;
            }
        }
    }

    for l in 1..=5 {
        sys.insert(format!("sys_skills_lv{}", l), lv_counts[l] as f64);
    }
    sys
}

fn compute_contribution(value: f64, config: &DimensionMetricConfig) -> f64 {
    // Range mode: both target_min and target_max define a healthy range
    if let (Some(t_min), Some(t_max)) = (config.target_min, config.target_max) {
        if t_min <= 0.0 || t_max <= 0.0 || t_max <= t_min {
            return 0.0;
        }
        if value >= t_min && value <= t_max {
            return 1.0;
        }
        if value < t_min {
            return (value / t_min).max(0.0);
        }
        // value > t_max
        return (t_max / value).max(0.0);
    }
    if let Some(target_max) = config.target_max {
        if target_max <= 0.0 {
            return 0.0;
        }
        return (value / target_max).min(1.0);
    }
    if let Some(target_min) = config.target_min {
        if value <= 0.0 {
            return 0.0;
        }
        return (target_min / value).min(1.0);
    }
    if let Some(ref brackets) = config.scoring_brackets {
        for bracket in brackets {
            if value >= bracket.min && value < bracket.max {
                return bracket.score;
            }
        }
        return 0.0;
    }
    // No scoring method: use raw value
    value
}

fn compute_dimensions(
    definitions: &[DimensionDefinition],
    user_values: &HashMap<String, f64>,
    sys_metrics: &HashMap<String, f64>,
) -> Vec<DimensionData> {
    definitions
        .iter()
        .map(|dim| {
            let mut total_score = 0.0;
            let mut has_any_data = false;
            let mut metric_results = Vec::new();

            for (metric_id, config) in &dim.metrics {
                let value = user_values
                    .get(metric_id)
                    .or_else(|| sys_metrics.get(metric_id))
                    .copied();

                let contribution = value.map(|v| {
                    has_any_data = true;
                    let c = compute_contribution(v, config);
                    total_score += c * config.weight;
                    c
                });

                metric_results.push(DimensionMetricResult {
                    metric_id: metric_id.clone(),
                    value,
                    contribution,
                    weight: config.weight,
                });
            }

            let (score, level, level_title) =
                if has_any_data && dim.level_thresholds.len() == 4 && dim.level_titles.len() == 5 {
                    let lv = if total_score >= dim.level_thresholds[3] {
                        5
                    } else if total_score >= dim.level_thresholds[2] {
                        4
                    } else if total_score >= dim.level_thresholds[1] {
                        3
                    } else if total_score >= dim.level_thresholds[0] {
                        2
                    } else {
                        1
                    };
                    (
                        Some(total_score),
                        Some(lv),
                        Some(dim.level_titles[(lv - 1) as usize].clone()),
                    )
                } else {
                    (None, None, None)
                };

            DimensionData {
                id: dim.id.clone(),
                name: dim.name.clone(),
                level_titles: dim.level_titles.clone(),
                level_thresholds: dim.level_thresholds.clone(),
                enabled: dim.enabled,
                score,
                level,
                level_title,
                metrics: metric_results,
            }
        })
        .collect()
}

#[tauri::command]
pub fn load_status_data() -> Result<StatusData, String> {
    let data_dir = resolve_data_dir()?;
    let definitions_path = data_dir.join("status_metric_definitions.json");
    let values_path = data_dir.join("status.json");
    let user_profile_path = data_dir.join("user_profile.json");

    let definitions: MetricDefinitionFile = read_json_file(&definitions_path)?;
    let values: StatusValueFile = read_json_file(&values_path)?;
    let user_profile: Option<UserProfile> = if user_profile_path.exists() {
        Some(read_json_file(&user_profile_path)?)
    } else {
        None
    };

    // Validate: no duplicate metric IDs
    let mut metric_ids = HashSet::new();
    for metric in &definitions.metrics {
        if !metric_ids.insert(metric.id.clone()) {
            return Err(format!(
                "Duplicate metric id found in definitions: {}",
                metric.id
            ));
        }
    }

    // Validate: no orphan values
    for value_id in values.metrics.keys() {
        if !metric_ids.contains(value_id) {
            return Err(format!(
                "Metric '{}' exists in status.json but is missing in status_metric_definitions.json",
                value_id
            ));
        }
    }

    // Compute system metrics
    let mut sys_metrics = compute_gallery_sys_metrics(&data_dir);
    sys_metrics.extend(compute_skill_sys_metrics(&data_dir));

    // BMI fallback: compute if height_cm and weight_kg exist but bmi is not in status.json
    if !values.metrics.contains_key("bmi") {
        if let Some(bmi) = calculate_bmi(&values.metrics) {
            sys_metrics.insert("bmi".to_string(), bmi);
        }
    }

    // game_days as system metric
    if let Some(profile) = &user_profile {
        if let Ok(days) = calculate_days_since(&profile.birth_date) {
            sys_metrics.insert("sys_game_days".to_string(), days as f64);
        }
    }

    // Compute dimensions
    let dimensions = compute_dimensions(&definitions.dimensions, &values.metrics, &sys_metrics);

    // Merge definitions with values
    let merged_metrics = definitions
        .metrics
        .into_iter()
        .filter(|metric| metric.value_type == "number")
        .map(|metric| StatusMetric {
            value: values
                .metrics
                .get(&metric.id)
                .copied()
                .or_else(|| sys_metrics.get(&metric.id).copied()),
            id: metric.id,
            name: metric.name,
            group: metric.group,
            unit: metric.unit,
            value_type: metric.value_type,
            description: metric.description,
        })
        .collect();

    Ok(StatusData {
        definition_version: definitions.version,
        value_version: values.version,
        username: user_profile
            .as_ref()
            .map(|profile| profile.username.clone())
            .unwrap_or_else(|| "Trickster".to_string()),
        game_days: user_profile
            .as_ref()
            .and_then(|profile| calculate_days_since(&profile.birth_date).ok()),
        metrics: merged_metrics,
        dimensions,
        system_metrics: sys_metrics,
    })
}
