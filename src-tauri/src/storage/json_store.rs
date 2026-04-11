use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    serde_json::from_str(&content).map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
}

pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

/// Write a typed value to a JSON file, then validate the result.
/// On validation failure, restore the original file content and return Err.
pub fn write_and_validate<T: Serialize>(
    path: &Path,
    data: &T,
    file_name: &str,
) -> Result<(), String> {
    let backup = fs::read(path).ok();
    write_json_file(path, data)?;

    let written: serde_json::Value =
        read_json_file(path).map_err(|e| format!("Failed to re-read after write: {e}"))?;
    if let Err(e) = super::validate::validate_data_file(file_name, &written) {
        if let Some(b) = backup {
            let _ = fs::write(path, b);
        } else {
            let _ = fs::remove_file(path);
        }
        return Err(format!("Validation failed: {e}"));
    }
    Ok(())
}

pub fn resolve_data_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Cannot resolve current dir: {}", e))?;
    let candidates = [cwd.join("data"), cwd.join("..").join("data")];

    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err("Cannot find data directory. Checked ./data and ../data".to_string())
}
