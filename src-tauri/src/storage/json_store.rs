use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
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
