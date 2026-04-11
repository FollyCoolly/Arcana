use crate::models::achievement::AchievementProgressFile;
use crate::models::mission::MissionFile;
use crate::models::status::{MetricDefinitionFile, StatusValueFile};
use crate::storage::json_store::read_json_file;
use serde_json::{json, Value};
use std::path::Path;

pub fn get_context(data_dir: &Path) -> Result<String, String> {
    let mut sections = Vec::new();

    // Missions
    let missions_path = data_dir.join("missions.json");
    if missions_path.exists() {
        let file: MissionFile = read_json_file(&missions_path)?;
        let active: Vec<_> = file
            .missions
            .iter()
            .filter(|m| m.status == "active")
            .collect();
        let proposed: Vec<_> = file
            .missions
            .iter()
            .filter(|m| m.status == "proposed")
            .collect();
        sections.push(format!(
            "## Missions\nActive: {}\nProposed: {}\n\n{}",
            active.len(),
            proposed.len(),
            serde_json::to_string_pretty(
                &file
                    .missions
                    .iter()
                    .filter(|m| m.status == "active" || m.status == "proposed")
                    .collect::<Vec<_>>()
            )
            .unwrap_or_default()
        ));
        sections.push(format!(
            "## Main Menu Config\n{}",
            serde_json::to_string_pretty(&file.main_menu).unwrap_or_default()
        ));
    }

    // Status metrics
    let status_path = data_dir.join("status.json");
    if status_path.exists() {
        let values: StatusValueFile = read_json_file(&status_path)?;
        sections.push(format!(
            "## Status Metrics\n{}",
            serde_json::to_string_pretty(&values.metrics).unwrap_or_default()
        ));
    }

    // Metric definitions
    let defs_path = data_dir.join("status_metric_definitions.json");
    if defs_path.exists() {
        let defs: MetricDefinitionFile = read_json_file(&defs_path)?;
        let summary: Vec<Value> = defs
            .metrics
            .iter()
            .map(|m| {
                json!({"id": m.id, "name": m.name, "unit": m.unit, "group": m.group, "description": m.description})
            })
            .collect();
        sections.push(format!(
            "## Metric Definitions\n{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        ));

        // Dimensions summary
        if !defs.dimensions.is_empty() {
            let dim_summary: Vec<Value> = defs
                .dimensions
                .iter()
                .filter(|d| d.enabled)
                .map(|d| {
                    json!({
                        "id": d.id,
                        "name": d.name,
                        "level_titles": d.level_titles,
                        "level_thresholds": d.level_thresholds,
                        "metric_count": d.metrics.len(),
                    })
                })
                .collect();
            sections.push(format!(
                "## Dimensions\n{}",
                serde_json::to_string_pretty(&dim_summary).unwrap_or_default()
            ));
        }
    }

    // Achievement progress
    let progress_path = data_dir.join("achievement_progress.json");
    if progress_path.exists() {
        let progress: AchievementProgressFile = read_json_file(&progress_path)?;
        if !progress.achievements.is_empty() {
            sections.push(format!(
                "## Achievement Progress\n{}",
                serde_json::to_string_pretty(&progress.achievements).unwrap_or_default()
            ));
        }
    }

    // Mission memory
    let memory_path = data_dir.join("mission_memory.json");
    if memory_path.exists() {
        let memory: Value = read_json_file(&memory_path)?;
        sections.push(format!(
            "## Mission Memory\n{}",
            serde_json::to_string_pretty(&memory).unwrap_or_default()
        ));
    }

    if sections.is_empty() {
        Ok("No data files found. This appears to be a fresh setup.".into())
    } else {
        Ok(sections.join("\n\n"))
    }
}
