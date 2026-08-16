use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ArcanaSettings {
    /// Legacy JSON v1 location. Only the not-yet-migrated UI and Rust Agent use
    /// it; the new data CLI ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
    /// Target Git synchronization repository. Not used by current commands yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_dir: Option<String>,
    /// SQLite/lock/backup directory used by the new data CLI and future UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_dir: Option<String>,
    /// Settings owned by other modules must survive a data-platform path update.
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Load settings from `~/.arcana/settings.json`.
/// Returns default settings if the file does not exist.
pub fn load_settings() -> ArcanaSettings {
    let Some(path) = settings_path() else {
        return ArcanaSettings::default();
    };
    if !path.exists() {
        return ArcanaSettings::default();
    }
    let Ok(content) = std::fs::read_to_string(&path) else {
        return ArcanaSettings::default();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Get user home directory without external crates.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Expand a leading `~` to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

/// Default data directory: `~/.arcana/data`.
pub fn default_data_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".arcana").join("data"))
}

/// Default target runtime directory. It remains separate from the Git repository.
pub fn default_runtime_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".arcana").join("runtime"))
}

fn settings_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".arcana").join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_absolute_path() {
        let p = expand_tilde("/usr/local/data");
        assert_eq!(p, PathBuf::from("/usr/local/data"));
    }

    #[test]
    fn expand_tilde_with_home() {
        let p = expand_tilde("~/some/path");
        if let Some(home) = home_dir() {
            assert_eq!(p, home.join("some/path"));
        }
    }

    #[test]
    fn expand_tilde_bare() {
        let p = expand_tilde("~");
        if let Some(home) = home_dir() {
            assert_eq!(p, home);
        }
    }

    #[test]
    fn target_paths_can_coexist_with_legacy_data_dir() {
        let settings: ArcanaSettings = serde_json::from_str(
            r#"{
                "data_dir": "~/old-data",
                "repository_dir": "~/arcana-user-data",
                "runtime_dir": "~/arcana-runtime",
                "weather_city": "Shanghai"
            }"#,
        )
        .unwrap();
        assert_eq!(settings.data_dir.as_deref(), Some("~/old-data"));
        assert_eq!(
            settings.repository_dir.as_deref(),
            Some("~/arcana-user-data")
        );
        assert_eq!(settings.runtime_dir.as_deref(), Some("~/arcana-runtime"));
        assert_eq!(
            settings.extra.get("weather_city"),
            Some(&serde_json::json!("Shanghai"))
        );
    }
}
