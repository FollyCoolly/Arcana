use crate::application::{
    ArcanaRuntime, StatusCommands, StatusDimensionEvaluation, StatusScoreEvaluation,
    StatusSelectionResult,
};
use crate::domain::{
    ArcanaRepository, ArcanaRepositoryReader, Record, RepositoryError, RepositoryErrorCode,
    ValidationIssue,
};
use crate::storage::date_utils::calculate_days_since;
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::State;

const DEFAULT_USERNAME: &str = "Trickster";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DataCommandError {
    pub code: RepositoryErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub validation_issues: Vec<ValidationIssue>,
}

impl From<RepositoryError> for DataCommandError {
    fn from(error: RepositoryError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            validation_issues: error.validation_issues,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusDashboardDimension {
    pub pack_id: String,
    pub id: String,
    pub name: String,
    pub level_titles: [String; 5],
    pub level_thresholds: [f64; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<u8>,
    pub score: Option<f64>,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_title: Option<String>,
    pub scores: Vec<StatusScoreEvaluation>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusDashboardData {
    pub username: String,
    pub game_days: Option<u64>,
    pub dimensions: Vec<StatusDashboardDimension>,
}

#[tauri::command]
pub fn load_status_dashboard(
    runtime: State<'_, ArcanaRuntime>,
) -> Result<StatusDashboardData, DataCommandError> {
    runtime
        .with_repository(build_status_dashboard)
        .map_err(Into::into)
}

#[tauri::command]
pub fn select_status_dimension(
    runtime: State<'_, ArcanaRuntime>,
    position: u8,
    dimension_id: String,
) -> Result<StatusSelectionResult, DataCommandError> {
    runtime
        .with_repository(|repository| {
            StatusCommands::new(repository).select(position, dimension_id)
        })
        .map_err(Into::into)
}

#[tauri::command]
pub fn clear_status_dimension(
    runtime: State<'_, ArcanaRuntime>,
    position: u8,
) -> Result<StatusSelectionResult, DataCommandError> {
    runtime
        .with_repository(|repository| StatusCommands::new(repository).clear_selection(position))
        .map_err(Into::into)
}

fn build_status_dashboard<R>(repository: &mut R) -> Result<StatusDashboardData, RepositoryError>
where
    R: ArcanaRepository,
{
    let dimension_list = StatusCommands::new(repository).list_dimensions()?;
    let evaluations = StatusCommands::new(repository).evaluate(None)?;
    let mut evaluations_by_id: BTreeMap<String, StatusDimensionEvaluation> = evaluations
        .into_iter()
        .map(|evaluation| (evaluation.dimension_id.clone(), evaluation))
        .collect();

    let dimensions = dimension_list
        .dimensions
        .into_iter()
        .map(|available| {
            let evaluation = evaluations_by_id
                .remove(&available.definition.id)
                .ok_or_else(|| {
                    RepositoryError::new(
                        RepositoryErrorCode::Unresolved,
                        format!(
                            "Status Dimension '{}' was listed but could not be evaluated",
                            available.definition.id
                        ),
                    )
                })?;
            Ok(StatusDashboardDimension {
                pack_id: available.pack_id,
                id: available.definition.id,
                name: available.definition.name,
                level_titles: available.definition.level_titles,
                level_thresholds: available.definition.level_thresholds,
                selected_position: available.selected_position,
                score: evaluation.score,
                level: evaluation.level,
                level_title: evaluation.level_title,
                scores: evaluation.scores,
            })
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;

    let username = scalar_string(repository, "identity.nickname")?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_USERNAME.to_string());
    let game_days = scalar_string(repository, "identity.birth_date")?
        .and_then(|date| calculate_days_since(&date).ok());

    Ok(StatusDashboardData {
        username,
        game_days,
        dimensions,
    })
}

fn scalar_string<R>(repository: &R, definition_id: &str) -> Result<Option<String>, RepositoryError>
where
    R: ArcanaRepositoryReader,
{
    Ok(match repository.get_record(definition_id)? {
        Some(Record::Scalar(record)) => record.value.as_str().map(str::to_string),
        Some(Record::Collection(_) | Record::Event(_)) | None => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{RecordCommands, SetScalarRecord};
    use serde_json::json;

    #[test]
    fn empty_runtime_returns_basic_identity_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();

        let dashboard = runtime.with_repository(build_status_dashboard).unwrap();

        assert_eq!(dashboard.username, DEFAULT_USERNAME);
        assert_eq!(dashboard.game_days, None);
        assert!(dashboard.dimensions.is_empty());
    }

    #[test]
    fn dashboard_reads_identity_from_basic_records() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                RecordCommands::new(repository).set_scalar_at(
                    SetScalarRecord {
                        definition_id: "identity.nickname".to_string(),
                        value: json!("Alice"),
                        effective_at: None,
                    },
                    "2026-08-16T12:00:00+08:00".to_string(),
                )?;
                Ok(())
            })
            .unwrap();

        let dashboard = runtime.with_repository(build_status_dashboard).unwrap();

        assert_eq!(dashboard.username, "Alice");
    }
}
