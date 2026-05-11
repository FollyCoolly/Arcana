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
            // Pass 1: collect raw contributions for filled metrics
            struct Row {
                metric_id: String,
                value: Option<f64>,
                filled_contrib: Option<f64>,
                weight: f64,
            }

            let mut rows: Vec<Row> = Vec::with_capacity(dim.metrics.len());
            let mut filled_count: usize = 0;
            let mut sum_filled_contrib: f64 = 0.0;

            for (metric_id, config) in &dim.metrics {
                let value = user_values
                    .get(metric_id)
                    .or_else(|| sys_metrics.get(metric_id))
                    .copied();

                let filled_contrib = value.map(|v| compute_contribution(v, config));
                if let Some(c) = filled_contrib {
                    filled_count += 1;
                    sum_filled_contrib += c;
                }

                rows.push(Row {
                    metric_id: metric_id.clone(),
                    value,
                    filled_contrib,
                    weight: config.weight,
                });
            }

            // Pass 2: assemble total_score. Missing metrics get an estimated
            // contribution = avg(filled_contrib) × (0.7 + 0.3 × completeness),
            // so users aren't punished by leaving a metric blank, but
            // completeness still matters. If no metrics are filled, the
            // dimension is unscored.
            let total = dim.metrics.len();
            let has_any_data = filled_count > 0;

            let (estimated_contrib, completeness) = if has_any_data && total > 0 {
                let avg = sum_filled_contrib / filled_count as f64;
                let c = filled_count as f64 / total as f64;
                let penalty = 0.7 + 0.3 * c;
                (avg * penalty, c)
            } else {
                (0.0, 0.0)
            };

            let mut total_score = 0.0;
            let mut metric_results = Vec::with_capacity(rows.len());
            for row in rows {
                let contribution = match row.filled_contrib {
                    Some(c) => {
                        total_score += c * row.weight;
                        Some(c)
                    }
                    None => {
                        if has_any_data {
                            total_score += estimated_contrib * row.weight;
                        }
                        None
                    }
                };

                metric_results.push(DimensionMetricResult {
                    metric_id: row.metric_id,
                    value: row.value,
                    contribution,
                    weight: row.weight,
                });
            }
            let _ = completeness; // reserved for future UI surfacing

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

#[cfg(test)]
mod tests {
    use super::*;

    fn bracket(min: f64, max: f64, score: f64) -> ScoringBracket {
        ScoringBracket { min, max, score }
    }

    fn health_dim() -> DimensionDefinition {
        let mut metrics = HashMap::new();
        metrics.insert(
            "bmi".to_string(),
            DimensionMetricConfig {
                weight: 1.0,
                target_max: None,
                target_min: None,
                scoring_brackets: Some(vec![
                    bracket(20.0, 23.0, 1.0),
                    bracket(23.0, 24.9, 0.85),
                ]),
            },
        );
        metrics.insert(
            "hr".to_string(),
            DimensionMetricConfig {
                weight: 2.0,
                target_max: None,
                target_min: None,
                scoring_brackets: Some(vec![
                    bracket(45.0, 56.0, 1.0),
                    bracket(56.0, 61.0, 0.9),
                ]),
            },
        );
        metrics.insert(
            "body_fat".to_string(),
            DimensionMetricConfig {
                weight: 1.0,
                target_max: None,
                target_min: None,
                scoring_brackets: Some(vec![bracket(10.0, 15.0, 1.0)]),
            },
        );
        DimensionDefinition {
            id: "health".to_string(),
            name: "Health".to_string(),
            level_titles: vec![
                "L1".to_string(),
                "L2".to_string(),
                "L3".to_string(),
                "L4".to_string(),
                "L5".to_string(),
            ],
            level_thresholds: vec![0.5, 1.0, 2.0, 3.5],
            enabled: true,
            metrics,
        }
    }

    #[test]
    fn missing_metric_gets_estimated_contribution() {
        // Two metrics filled (bmi=22 → 1.0, hr=58 → 0.9), body_fat missing.
        // avg_contrib = (1.0 + 0.9) / 2 = 0.95
        // completeness = 2/3
        // penalty = 0.7 + 0.3 * (2/3) = 0.9
        // estimated = 0.95 * 0.9 = 0.855
        // total = 1.0*1.0 + 0.9*2.0 + 0.855*1.0 = 3.655
        let mut values = HashMap::new();
        values.insert("bmi".to_string(), 22.0);
        values.insert("hr".to_string(), 58.0);
        let dims = compute_dimensions(&[health_dim()], &values, &HashMap::new());
        let score = dims[0].score.unwrap();
        assert!(
            (score - 3.655).abs() < 1e-6,
            "expected ~3.655, got {}",
            score
        );
        // body_fat's external contribution stays None (UI-visible marker)
        let bf = dims[0]
            .metrics
            .iter()
            .find(|m| m.metric_id == "body_fat")
            .unwrap();
        assert!(bf.contribution.is_none());
    }

    #[test]
    fn all_filled_behaves_like_plain_weighted_sum() {
        // bmi=22 → 1.0, hr=58 → 0.9, body_fat=12 → 1.0
        // total = 1.0 + 0.9*2.0 + 1.0 = 3.8
        let mut values = HashMap::new();
        values.insert("bmi".to_string(), 22.0);
        values.insert("hr".to_string(), 58.0);
        values.insert("body_fat".to_string(), 12.0);
        let dims = compute_dimensions(&[health_dim()], &values, &HashMap::new());
        let score = dims[0].score.unwrap();
        assert!((score - 3.8).abs() < 1e-6, "expected 3.8, got {}", score);
    }

    #[test]
    fn no_data_leaves_dimension_unscored() {
        let dims = compute_dimensions(&[health_dim()], &HashMap::new(), &HashMap::new());
        assert!(dims[0].score.is_none());
        assert!(dims[0].level.is_none());
    }
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
