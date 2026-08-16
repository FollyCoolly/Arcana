use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    serde_json::from_str(&content).map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))
}

/// Resolve the data directory with the following priority:
///   1. `ARCANA_DATA_DIR` environment variable
///   2. `data_dir` field in `~/.arcana/settings.json`
///   3. Default: `~/.arcana/data` (auto-created if missing)
///
/// This directory only contains configuration for external Items/Gallery
/// sources and Weather. User progress lives in the SQLite runtime.
pub fn resolve_data_dir() -> Result<PathBuf, String> {
    use super::settings;

    // 1. Environment variable override
    if let Ok(v) = std::env::var("ARCANA_DATA_DIR") {
        if !v.is_empty() {
            let dir = settings::expand_tilde(&v);
            if dir.is_dir() {
                return Ok(dir);
            }
            return Err(format!(
                "ARCANA_DATA_DIR points to '{}' which does not exist",
                dir.display()
            ));
        }
    }

    // 2. ~/.arcana/settings.json → data_dir
    let s = settings::load_settings();
    if let Some(ref configured) = s.data_dir {
        let dir = settings::expand_tilde(configured);
        if dir.is_dir() {
            return Ok(dir);
        }
        return Err(format!(
            "data_dir in settings.json points to '{}' which does not exist",
            dir.display()
        ));
    }

    // 3. Default: ~/.arcana/data (auto-create)
    let default = settings::default_data_dir()
        .ok_or("Cannot determine home directory for default data path")?;
    if !default.exists() {
        fs::create_dir_all(&default).map_err(|e| {
            format!(
                "Failed to create default data directory '{}': {}",
                default.display(),
                e
            )
        })?;
    }
    Ok(default)
}
