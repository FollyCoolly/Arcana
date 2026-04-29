use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
pub struct ArcanaSettings {
    pub data_dir: Option<String>,
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
}
