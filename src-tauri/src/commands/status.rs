use std::collections::{HashMap, HashSet};

use crate::models::status::*;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

fn parse_birth_date(date_str: &str) -> Result<(i32, u32, u32), String> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Invalid birth_date '{}'. Expected format YYYY-MM-DD",
            date_str
        ));
    }

    let year = parts[0]
        .parse::<i32>()
        .map_err(|_| format!("Invalid year in birth_date '{}'", date_str))?;
    let month = parts[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid month in birth_date '{}'", date_str))?;
    let day = parts[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid day in birth_date '{}'", date_str))?;

    if !(1..=12).contains(&month) {
        return Err(format!("Invalid month '{}' in birth_date '{}'", month, date_str));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("Invalid day '{}' in birth_date '{}'", day, date_str));
    }

    Ok((year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe) as i64
}

fn calculate_game_days(birth_date: &str) -> Result<u64, String> {
    let (year, month, day) = parse_birth_date(birth_date)?;
    let birth_days = days_from_civil(year, month, day) - days_from_civil(1970, 1, 1);

    let now_duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System clock before UNIX_EPOCH: {}", e))?;
    let today_days = (now_duration.as_secs() / 86_400) as i64;

    let diff = today_days - birth_days;
    Ok(if diff > 0 { diff as u64 } else { 0 })
}

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
            return Err(format!("Duplicate metric id found in definitions: {}", metric.id));
        }

        if metric.value_type != "number" {
            return Err(format!(
                "Unsupported value_type '{}' for metric '{}'. Only 'number' is supported in MVP.",
                metric.value_type, metric.id
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
        game_days: Some(calculate_game_days(&user_profile.birth_date)?),
        bmi: calculate_bmi(&values.metrics),
        metrics: merged_metrics,
    })
}
