use crate::application::{
    AchievementAvailability, AchievementCommands, AchievementEntry, AchievementStateResult,
    ArcanaRuntime, PackAssetContent, PackCommands, PackDeleteResult, PackEnabledState, PackSummary,
    QueryAchievements, QuerySkills, SetAchievementState, SkillCommands, SkillEvaluation,
    StatusCommands, StatusDimensionEvaluation, StatusScoreEvaluation, StatusSelectionResult,
};
use crate::domain::{
    AchievementDefinition, AchievementDifficulty, AchievementState, AchievementStatus,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AchievementDashboardItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: AchievementDifficulty,
    pub tags: Vec<String>,
    pub prerequisites: Vec<String>,
    pub related_record_definition_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<String>,
    pub enabled: bool,
    pub availability: AchievementAvailability,
    pub unmet_prerequisite_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AchievementDashboardPack {
    pub pack_id: String,
    pub pack_name: String,
    pub achievements: Vec<AchievementDashboardItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AchievementDashboardData {
    pub packs: Vec<AchievementDashboardPack>,
    pub progress: BTreeMap<String, AchievementState>,
    pub unresolved_achievement_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillDashboardEntry {
    pub pack_name: String,
    #[serde(flatten)]
    pub evaluation: SkillEvaluation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillDashboardData {
    pub skills: Vec<SkillDashboardEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackDashboardData {
    pub packs: Vec<PackSummary>,
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

#[tauri::command]
pub fn load_achievement_dashboard(
    runtime: State<'_, ArcanaRuntime>,
) -> Result<AchievementDashboardData, DataCommandError> {
    runtime
        .with_repository(build_achievement_dashboard)
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_achievement_achieved(
    runtime: State<'_, ArcanaRuntime>,
    achievement_id: String,
) -> Result<AchievementStateResult, DataCommandError> {
    runtime
        .with_repository(|repository| {
            AchievementCommands::new(repository).set_state(SetAchievementState {
                achievement_id,
                status: AchievementStatus::Achieved,
                achieved_at: None,
            })
        })
        .map_err(Into::into)
}

#[tauri::command]
pub fn revoke_achievement_state(
    runtime: State<'_, ArcanaRuntime>,
    achievement_id: String,
) -> Result<AchievementStateResult, DataCommandError> {
    runtime
        .with_repository(|repository| {
            AchievementCommands::new(repository).revoke_state(&achievement_id)
        })
        .map_err(Into::into)
}

#[tauri::command]
pub fn load_skill_dashboard(
    runtime: State<'_, ArcanaRuntime>,
) -> Result<SkillDashboardData, DataCommandError> {
    runtime
        .with_repository(build_skill_dashboard)
        .map_err(Into::into)
}

#[tauri::command]
pub fn load_pack_asset(
    runtime: State<'_, ArcanaRuntime>,
    pack_id: String,
    asset_path: String,
) -> Result<PackAssetContent, DataCommandError> {
    runtime
        .with_repository(|repository| {
            PackCommands::new(repository).read_asset(&pack_id, &asset_path)
        })
        .map_err(Into::into)
}

#[tauri::command]
pub fn load_pack_dashboard(
    runtime: State<'_, ArcanaRuntime>,
) -> Result<PackDashboardData, DataCommandError> {
    runtime
        .with_repository(|repository| {
            Ok(PackDashboardData {
                packs: PackCommands::new(repository).list()?,
            })
        })
        .map_err(Into::into)
}

#[tauri::command]
pub fn set_pack_enabled(
    runtime: State<'_, ArcanaRuntime>,
    pack_id: String,
    enabled: bool,
) -> Result<PackEnabledState, DataCommandError> {
    runtime
        .with_repository(|repository| PackCommands::new(repository).set_enabled(&pack_id, enabled))
        .map_err(Into::into)
}

#[tauri::command]
pub fn delete_pack(
    runtime: State<'_, ArcanaRuntime>,
    pack_id: String,
) -> Result<PackDeleteResult, DataCommandError> {
    runtime
        .with_repository(|repository| PackCommands::new(repository).delete(&pack_id))
        .map_err(Into::into)
}

#[tauri::command]
pub fn preview_pack_deletion(
    runtime: State<'_, ArcanaRuntime>,
    pack_id: String,
) -> Result<PackDeleteResult, DataCommandError> {
    runtime
        .with_repository(|repository| PackCommands::new(repository).preview_delete(&pack_id))
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

fn build_achievement_dashboard<R>(
    repository: &mut R,
) -> Result<AchievementDashboardData, RepositoryError>
where
    R: ArcanaRepository,
{
    let pack_names: BTreeMap<String, String> = PackCommands::new(repository)
        .list()?
        .into_iter()
        .map(|pack| (pack.id, pack.name))
        .collect();
    let entries = AchievementCommands::new(repository).list(QueryAchievements::default())?;
    let mut packs: BTreeMap<String, AchievementDashboardPack> = BTreeMap::new();
    let mut progress = BTreeMap::new();
    let mut unresolved_achievement_ids = Vec::new();

    for entry in entries {
        if let Some(state) = entry.state.clone() {
            progress.insert(entry.achievement_id.clone(), state);
        }
        let Some(definition) = entry.definition.clone() else {
            unresolved_achievement_ids.push(entry.achievement_id);
            continue;
        };
        packs
            .entry(entry.pack_id.clone())
            .or_insert_with(|| AchievementDashboardPack {
                pack_name: pack_names
                    .get(&entry.pack_id)
                    .cloned()
                    .unwrap_or_else(|| entry.pack_id.clone()),
                pack_id: entry.pack_id.clone(),
                achievements: Vec::new(),
            })
            .achievements
            .push(achievement_dashboard_item(entry, definition));
    }

    Ok(AchievementDashboardData {
        packs: packs.into_values().collect(),
        progress,
        unresolved_achievement_ids,
    })
}

fn achievement_dashboard_item(
    entry: AchievementEntry,
    definition: AchievementDefinition,
) -> AchievementDashboardItem {
    AchievementDashboardItem {
        id: definition.id,
        name: definition.name,
        description: definition.description,
        difficulty: definition.difficulty,
        tags: definition.tags,
        prerequisites: definition.prerequisites,
        related_record_definition_ids: definition.related_record_definition_ids,
        tip: definition.tip,
        enabled: entry.enabled,
        availability: entry.availability,
        unmet_prerequisite_ids: entry.unmet_prerequisite_ids,
    }
}

fn build_skill_dashboard<R>(repository: &mut R) -> Result<SkillDashboardData, RepositoryError>
where
    R: ArcanaRepository,
{
    let pack_names: BTreeMap<String, String> = PackCommands::new(repository)
        .list()?
        .into_iter()
        .map(|pack| (pack.id, pack.name))
        .collect();
    let skills = SkillCommands::new(repository)
        .list(QuerySkills::default())?
        .into_iter()
        .map(|evaluation| SkillDashboardEntry {
            pack_name: pack_names
                .get(&evaluation.pack_id)
                .cloned()
                .unwrap_or_else(|| evaluation.pack_id.clone()),
            evaluation,
        })
        .collect();
    Ok(SkillDashboardData { skills })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{RecordCommands, SetScalarRecord};
    use crate::domain::{
        AchievementFile, ArcanaRepositoryTransaction, Pack, PackManifest, SkillDefinition,
        SkillFile, SkillNode, SCHEMA_VERSION,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn achievement_skill_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "cooking".to_string(),
                name: "Cooking".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: Vec::new(),
            },
            record_definitions: None,
            dimensions: None,
            achievements: Some(AchievementFile {
                achievements: vec![AchievementDefinition {
                    id: "cooking::first_dish".to_string(),
                    name: "First dish".to_string(),
                    description: "Cook one dish".to_string(),
                    difficulty: AchievementDifficulty::Beginner,
                    tags: Vec::new(),
                    prerequisites: Vec::new(),
                    related_record_definition_ids: Vec::new(),
                    tip: None,
                }],
            }),
            skills: Some(SkillFile {
                skills: vec![SkillDefinition {
                    id: "cooking::general".to_string(),
                    name: "Cooking".to_string(),
                    description: None,
                    level_thresholds: [1, 2, 3, 4],
                    nodes: vec![SkillNode {
                        achievement_id: "cooking::first_dish".to_string(),
                        points: 4,
                    }],
                    card_image: None,
                }],
            }),
            assets: BTreeMap::new(),
        }
    }

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

    #[test]
    fn empty_runtime_returns_empty_achievement_and_skill_dashboards() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();

        runtime
            .with_repository(|repository| {
                assert!(build_achievement_dashboard(repository)?.packs.is_empty());
                assert!(build_skill_dashboard(repository)?.skills.is_empty());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn achievement_and_skill_dashboards_share_derived_state() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut transaction = repository.begin_transaction()?;
                transaction.put_pack(achievement_skill_pack())?;
                transaction.set_pack_enabled("cooking", true)?;
                transaction.commit()?;
                Ok(())
            })
            .unwrap();

        runtime
            .with_repository(|repository| {
                let achievements = build_achievement_dashboard(repository)?;
                assert_eq!(achievements.packs[0].pack_name, "Cooking");
                assert_eq!(
                    achievements.packs[0].achievements[0].availability,
                    AchievementAvailability::Available
                );
                let skills = build_skill_dashboard(repository)?;
                assert_eq!(skills.skills[0].evaluation.level, 0);

                AchievementCommands::new(repository).set_state(SetAchievementState {
                    achievement_id: "cooking::first_dish".to_string(),
                    status: AchievementStatus::Achieved,
                    achieved_at: None,
                })?;
                let skills = build_skill_dashboard(repository)?;
                assert_eq!(skills.skills[0].evaluation.level, 5);
                assert_eq!(skills.skills[0].evaluation.achieved_node_count, 1);
                Ok(())
            })
            .unwrap();
    }
}
