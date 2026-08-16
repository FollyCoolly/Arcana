use crate::domain::{
    DashboardMissionSelections, MissionSuggestion, RepositoryError, RepositoryErrorCode,
    RepositoryResult, StatusDimensionSelection, Validate,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mission_suggestions: Vec<MissionSuggestion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_dimension_selection: Vec<StatusDimensionSelection>,
    #[serde(default, skip_serializing_if = "DashboardMissionSelections::is_empty")]
    pub dashboard_mission_selections: DashboardMissionSelections,
}

impl LocalState {
    pub fn validate(&self) -> RepositoryResult<()> {
        for suggestion in &self.mission_suggestions {
            suggestion.validate().map_err(RepositoryError::validation)?;
        }
        if !self
            .mission_suggestions
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
        {
            return Err(validation_error(
                "local MissionSuggestions must be unique and sorted by id",
            ));
        }
        for selection in &self.status_dimension_selection {
            selection.validate().map_err(RepositoryError::validation)?;
        }
        let mut positions = self
            .status_dimension_selection
            .iter()
            .map(|selection| selection.position)
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions.dedup();
        if positions.len() != self.status_dimension_selection.len() {
            return Err(validation_error(
                "local Status Dimension positions must be unique",
            ));
        }
        let mut dimensions = self
            .status_dimension_selection
            .iter()
            .map(|selection| selection.dimension_id.as_str())
            .collect::<Vec<_>>();
        dimensions.sort_unstable();
        dimensions.dedup();
        if dimensions.len() != self.status_dimension_selection.len() {
            return Err(validation_error(
                "local Status Dimension ids must be unique",
            ));
        }
        for selection in self.dashboard_mission_selections.values() {
            selection.validate().map_err(RepositoryError::validation)?;
        }
        Ok(())
    }

    pub fn normalize(&mut self) {
        self.mission_suggestions
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.status_dimension_selection
            .sort_by_key(|selection| selection.position);
    }
}

pub fn read_local_state(path: &Path) -> RepositoryResult<LocalState> {
    if !path.exists() {
        return Ok(LocalState::default());
    }
    let content = fs::read_to_string(path).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to read local state '{}': {error}", path.display()),
        )
    })?;
    let mut state: LocalState = serde_json::from_str(&content).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::ValidationFailed,
            format!("invalid local state '{}': {error}", path.display()),
        )
    })?;
    state.normalize();
    state.validate()?;
    Ok(state)
}

pub fn write_local_state(path: &Path, state: &LocalState) -> RepositoryResult<()> {
    let mut state = state.clone();
    state.normalize();
    state.validate()?;
    if state == LocalState::default() {
        if path.exists() {
            fs::remove_file(path).map_err(|error| {
                RepositoryError::new(
                    RepositoryErrorCode::Storage,
                    format!("failed to remove local state '{}': {error}", path.display()),
                )
            })?;
        }
        return Ok(());
    }
    let bytes = serde_json::to_vec_pretty(&state).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to serialize local state: {error}"),
        )
    })?;
    atomic_replace(path, &bytes)
}

fn atomic_replace(path: &Path, content: &[u8]) -> RepositoryResult<()> {
    let parent = path.parent().ok_or_else(|| {
        RepositoryError::new(
            RepositoryErrorCode::ValidationFailed,
            format!("local state path has no parent: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to create local state directory: {error}"),
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                "local state file name must be UTF-8",
            )
        })?;
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".{file_name}.arcana-temp-{}-{suffix}",
        std::process::id()
    ));
    let backup = parent.join(format!(
        ".{file_name}.arcana-backup-{}-{suffix}",
        std::process::id()
    ));
    let mut temporary_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("failed to create local state temporary file: {error}"),
            )
        })?;
    temporary_file.write_all(content).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to write local state temporary file: {error}"),
        )
    })?;
    temporary_file.sync_all().map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to sync local state temporary file: {error}"),
        )
    })?;

    let had_existing = path.exists();
    if had_existing {
        fs::rename(path, &backup).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("failed to stage local state '{}': {error}", path.display()),
            )
        })?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if had_existing {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err(RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!(
                "failed to activate local state '{}': {error}",
                path.display()
            ),
        ));
    }
    if had_existing {
        fs::remove_file(&backup).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("failed to remove local state backup: {error}"),
            )
        })?;
    }
    Ok(())
}

fn validation_error(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorCode::ValidationFailed, message)
}
