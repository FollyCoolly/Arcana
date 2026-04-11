use crate::models::status::{MetricDefinitionFile, StatusValueFile};
use crate::storage::json_store::{read_json_file, write_and_validate};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

pub fn update_status(data_dir: &Path, input: &Value) -> Result<String, String> {
    let status_path = data_dir.join("status.json");
    let mut file: StatusValueFile = read_json_file(&status_path)?;

    let defs_path = data_dir.join("status_metric_definitions.json");
    let defs: MetricDefinitionFile = read_json_file(&defs_path)?;
    let valid_ids: HashSet<&str> = defs.metrics.iter().map(|m| m.id.as_str()).collect();

    let updates = input["metrics"]
        .as_object()
        .ok_or("'metrics' must be an object")?;

    let mut changes = Vec::new();
    for (id, val) in updates {
        if id.starts_with("sys_") {
            return Err(format!(
                "Cannot write system metric '{id}': sys_ metrics are read-only"
            ));
        }
        if !valid_ids.contains(id.as_str()) {
            return Err(format!(
                "Unknown metric ID: '{id}'. Valid IDs: {valid_ids:?}"
            ));
        }
        let new_val = val
            .as_f64()
            .ok_or(format!("Value for '{id}' must be a number"))?;
        let old_val = file.metrics.get(id).copied();
        file.metrics.insert(id.clone(), new_val);
        changes.push(format!("{id}: {old_val:?} → {new_val}"));
    }

    write_and_validate(&status_path, &file, "status.json")?;
    Ok(format!("Updated status.json:\n- {}", changes.join("\n- ")))
}
