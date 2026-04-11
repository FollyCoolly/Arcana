use std::collections::{HashMap, HashSet};

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

#[tauri::command]
pub fn load_status_data() -> Result<StatusData, String> {
    let data_dir = resolve_data_dir()?;
    let definitions_path = data_dir.join("status_metric_definitions.json");
    let values_path = data_dir.join("status.json");
    let user_profile_path = data_dir.join("user_profile.json");

    let definitions: MetricDefinitionFile = read_json_file(&definitions_path)?;
    let values: StatusValueFile = read_json_file(&values_path)?;
    let user_profile: UserProfile = read_json_file(&user_profile_path)?;

    let mut metric_ids = HashSet::new();
    for metric in &definitions.metrics {
        if !metric_ids.insert(metric.id.clone()) {
            return Err(format!(
                "Duplicate metric id found in definitions: {}",
                metric.id
            ));
        }
    }

    for value_id in values.metrics.keys() {
        if !metric_ids.contains(value_id) {
            return Err(format!(
                "Metric '{}' exists in status.json but is missing in status_metric_definitions.json",
                value_id
            ));
        }
    }

    let merged_metrics = definitions
        .metrics
        .into_iter()
        .filter(|metric| metric.value_type == "number")
        .map(|metric| StatusMetric {
            value: values.metrics.get(&metric.id).copied(),
            id: metric.id,
            name: metric.name,
            category: metric.category,
            group: metric.group,
            sub_group: metric.sub_group,
            unit: metric.unit,
            value_type: metric.value_type,
            target_max: metric.target_max,
            target_min: metric.target_min,
            body_parts: metric.body_parts.unwrap_or_default(),
            description: metric.description,
        })
        .collect();

    Ok(StatusData {
        definition_version: definitions.version,
        value_version: values.version,
        username: user_profile.username,
        game_days: Some(calculate_days_since(&user_profile.birth_date)?),
        bmi: calculate_bmi(&values.metrics),
        metrics: merged_metrics,
    })
}
