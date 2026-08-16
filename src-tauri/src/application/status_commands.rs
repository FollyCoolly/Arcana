use crate::domain::{
    aggregate_dimension_score, dimension_level, ArcanaRepository, ArcanaRepositoryReader,
    ArcanaRepositoryTransaction, DimensionDefinition, Record, RepositoryError, RepositoryErrorCode,
    RepositoryResult, ScoreDefinition, StatusDimensionSelection, ValidationIssue,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AvailableStatusDimension {
    pub pack_id: String,
    pub definition: DimensionDefinition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusSelectionAvailability {
    pub position: u8,
    pub dimension_id: String,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusDimensionList {
    pub dimensions: Vec<AvailableStatusDimension>,
    pub selections: Vec<StatusSelectionAvailability>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusScoreEvaluation {
    pub id: String,
    pub name: String,
    pub weight: f64,
    pub expression: String,
    pub raw_value: Option<f64>,
    pub score: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusDimensionEvaluation {
    pub pack_id: String,
    pub dimension_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_position: Option<u8>,
    pub score: Option<f64>,
    pub level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_title: Option<String>,
    pub scores: Vec<StatusScoreEvaluation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusSelectionResult {
    pub position: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension_id: Option<String>,
    pub changed: bool,
}

pub struct StatusCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> StatusCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list_dimensions(&mut self) -> RepositoryResult<StatusDimensionList> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let selections = transaction.status_dimension_selection()?;
        let selected_positions: BTreeMap<&str, u8> = selections
            .iter()
            .map(|selection| (selection.dimension_id.as_str(), selection.position))
            .collect();
        let dimensions = enabled_dimensions(&snapshot)
            .map(|(pack_id, definition)| AvailableStatusDimension {
                pack_id: pack_id.to_string(),
                selected_position: selected_positions.get(definition.id.as_str()).copied(),
                definition: definition.clone(),
            })
            .collect::<Vec<_>>();
        let available_ids: BTreeSet<&str> = dimensions
            .iter()
            .map(|dimension| dimension.definition.id.as_str())
            .collect();
        let selections = selections
            .into_iter()
            .map(|selection| StatusSelectionAvailability {
                position: selection.position,
                available: available_ids.contains(selection.dimension_id.as_str()),
                dimension_id: selection.dimension_id,
            })
            .collect();
        let result = StatusDimensionList {
            dimensions,
            selections,
        };
        transaction.rollback()?;
        Ok(result)
    }

    /// Evaluate one enabled Dimension, or every enabled Dimension when no ID
    /// is supplied. Results are derived from one consistent repository snapshot
    /// and are never persisted.
    pub fn evaluate(
        &mut self,
        dimension_id: Option<&str>,
    ) -> RepositoryResult<Vec<StatusDimensionEvaluation>> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let selections = transaction.status_dimension_selection()?;
        let matches = evaluate_dimensions_from_snapshot(&snapshot, &selections, dimension_id)?;
        transaction.rollback()?;
        Ok(matches)
    }

    pub fn select(
        &mut self,
        position: u8,
        dimension_id: String,
    ) -> RepositoryResult<StatusSelectionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let selections = transaction.status_dimension_selection()?;
        if selections.iter().any(|selection| {
            selection.position == position && selection.dimension_id == dimension_id
        }) {
            transaction.rollback()?;
            return Ok(StatusSelectionResult {
                position,
                dimension_id: Some(dimension_id),
                changed: false,
            });
        }
        transaction.set_status_dimension_selection(StatusDimensionSelection {
            position,
            dimension_id: dimension_id.clone(),
        })?;
        transaction.commit()?;
        Ok(StatusSelectionResult {
            position,
            dimension_id: Some(dimension_id),
            changed: true,
        })
    }

    pub fn clear_selection(&mut self, position: u8) -> RepositoryResult<StatusSelectionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let selections = transaction.status_dimension_selection()?;
        if !selections
            .iter()
            .any(|selection| selection.position == position)
        {
            // Let the repository validate an out-of-range position even when
            // there is no existing row at that position.
            if position >= 5 {
                transaction.clear_status_dimension_selection(position)?;
            }
            transaction.rollback()?;
            return Ok(StatusSelectionResult {
                position,
                dimension_id: None,
                changed: false,
            });
        }
        transaction.clear_status_dimension_selection(position)?;
        transaction.commit()?;
        Ok(StatusSelectionResult {
            position,
            dimension_id: None,
            changed: true,
        })
    }
}

pub(crate) fn evaluate_dimensions_from_snapshot(
    snapshot: &crate::domain::SyncedRepositorySnapshot,
    selections: &[StatusDimensionSelection],
    dimension_id: Option<&str>,
) -> RepositoryResult<Vec<StatusDimensionEvaluation>> {
    let selected_positions: BTreeMap<&str, u8> = selections
        .iter()
        .map(|selection| (selection.dimension_id.as_str(), selection.position))
        .collect();
    let numeric_records: BTreeMap<&str, f64> = snapshot
        .records
        .values()
        .flat_map(|file| file.records.iter())
        .filter_map(|record| match record {
            Record::Scalar(record) => record
                .value
                .as_f64()
                .map(|value| (record.definition_id.as_str(), value)),
            Record::Collection(_) | Record::Event(_) => None,
        })
        .collect();

    let mut matches = enabled_dimensions(snapshot)
        .filter(|(_, definition)| {
            dimension_id.is_none_or(|dimension_id| definition.id == dimension_id)
        })
        .map(|(pack_id, definition)| {
            evaluate_dimension(
                pack_id,
                definition,
                selected_positions.get(definition.id.as_str()).copied(),
                &numeric_records,
            )
        })
        .collect::<RepositoryResult<Vec<_>>>()?;

    if let Some(dimension_id) = dimension_id {
        if matches.is_empty() {
            let exists_but_disabled = snapshot.packs.values().any(|pack| {
                pack.dimensions.as_ref().is_some_and(|file| {
                    file.dimensions
                        .iter()
                        .any(|dimension| dimension.id == dimension_id)
                })
            });
            let (code, message) = if exists_but_disabled {
                (
                    RepositoryErrorCode::Unresolved,
                    format!("Status Dimension '{dimension_id}' is not supplied by an enabled Pack"),
                )
            } else {
                (
                    RepositoryErrorCode::NotFound,
                    format!("Status Dimension '{dimension_id}' was not found"),
                )
            };
            return Err(RepositoryError::new(code, message));
        }
    }
    matches.sort_by(|left, right| left.dimension_id.cmp(&right.dimension_id));
    Ok(matches)
}

fn enabled_dimensions(
    snapshot: &crate::domain::SyncedRepositorySnapshot,
) -> impl Iterator<Item = (&str, &DimensionDefinition)> {
    snapshot
        .manifest
        .enabled_pack_ids
        .iter()
        .filter_map(|pack_id| {
            snapshot
                .packs
                .get(pack_id)
                .map(|pack| (pack_id.as_str(), pack))
        })
        .flat_map(|(pack_id, pack)| {
            pack.dimensions
                .iter()
                .flat_map(|file| file.dimensions.iter())
                .map(move |dimension| (pack_id, dimension))
        })
}

fn evaluate_dimension(
    pack_id: &str,
    definition: &DimensionDefinition,
    selected_position: Option<u8>,
    numeric_records: &BTreeMap<&str, f64>,
) -> RepositoryResult<StatusDimensionEvaluation> {
    let mut scores = Vec::with_capacity(definition.scores.len());
    for score in &definition.scores {
        let expression = score.parse_expression().map_err(|error| {
            status_evaluation_error(
                definition,
                score,
                "status_expression_parse_failed",
                error.to_string(),
            )
        })?;
        let missing_record_ids = expression
            .record_references()
            .filter(|id| !numeric_records.contains_key(*id))
            .map(str::to_string)
            .collect();
        let raw_value = expression
            .evaluate_raw(|id| numeric_records.get(id).copied())
            .map_err(|error| {
                status_evaluation_error(
                    definition,
                    score,
                    "status_expression_evaluation_failed",
                    error.to_string(),
                )
            })?;
        scores.push(StatusScoreEvaluation {
            id: score.id.clone(),
            name: score.name.clone(),
            weight: score.weight,
            expression: score.expression.clone(),
            raw_value,
            score: raw_value.map(|value| value.clamp(0.0, 100.0)),
            missing_record_ids,
        });
    }
    let score = aggregate_dimension_score(
        definition
            .scores
            .iter()
            .zip(scores.iter().map(|evaluation| evaluation.score)),
    );
    let level = dimension_level(score, &definition.level_thresholds);
    let level_title = (level > 0).then(|| definition.level_titles[usize::from(level - 1)].clone());
    Ok(StatusDimensionEvaluation {
        pack_id: pack_id.to_string(),
        dimension_id: definition.id.clone(),
        name: definition.name.clone(),
        selected_position,
        score,
        level,
        level_title,
        scores,
    })
}

fn status_evaluation_error(
    dimension: &DimensionDefinition,
    score: &ScoreDefinition,
    code: &str,
    message: String,
) -> RepositoryError {
    let mut error = RepositoryError::new(
        RepositoryErrorCode::ValidationFailed,
        format!(
            "failed to evaluate Status Dimension '{}' Score '{}': {message}",
            dimension.id, score.id
        ),
    );
    error.validation_issues.push(ValidationIssue {
        code: code.to_string(),
        path: format!("dimensions.{}.scores.{}.expression", dimension.id, score.id),
        message,
    });
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{basic_pack, RecordCommands, SetScalarRecord};
    use crate::domain::{
        ArcanaRepositoryTransaction, DimensionFile, Pack, PackManifest, RecordDefinition,
        RecordDefinitionFile, ScalarRecordDefinition, ScoreDefinition, ValueType, SCHEMA_VERSION,
    };
    use crate::storage::sqlite::SqliteRepository;
    use serde_json::json;

    fn status_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "fitness".to_string(),
                name: "Fitness".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: Vec::new(),
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "fitness.endurance".to_string(),
                        name: "Endurance".to_string(),
                        description: None,
                        value_type: ValueType::Number,
                        unit: None,
                    }),
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "fitness.strength".to_string(),
                        name: "Strength".to_string(),
                        description: None,
                        value_type: ValueType::Integer,
                        unit: None,
                    }),
                ],
            }),
            dimensions: Some(DimensionFile {
                dimensions: vec![DimensionDefinition {
                    id: "fitness::physical".to_string(),
                    name: "Physical".to_string(),
                    level_titles: [
                        "Awake".to_string(),
                        "Growing".to_string(),
                        "Skilled".to_string(),
                        "Excellent".to_string(),
                        "Peak".to_string(),
                    ],
                    level_thresholds: [25.0, 50.0, 75.0, 90.0],
                    scores: vec![
                        ScoreDefinition {
                            id: "endurance".to_string(),
                            name: "Endurance".to_string(),
                            weight: 1.0,
                            expression: "record('fitness.endurance') * 2".to_string(),
                        },
                        ScoreDefinition {
                            id: "strength".to_string(),
                            name: "Strength".to_string(),
                            weight: 3.0,
                            expression: "record('fitness.strength')".to_string(),
                        },
                    ],
                }],
            }),
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn repository() -> SqliteRepository {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(basic_pack()).unwrap();
        transaction.set_pack_enabled("basic", true).unwrap();
        transaction.put_pack(status_pack()).unwrap();
        transaction.set_pack_enabled("fitness", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    #[test]
    fn evaluates_scores_from_one_snapshot_and_ignores_missing_values() {
        let mut repository = repository();
        RecordCommands::new(&mut repository)
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "fitness.endurance".to_string(),
                    value: json!(60),
                    effective_at: None,
                },
                "2026-08-16T10:00:00+08:00".to_string(),
            )
            .unwrap();
        let evaluation = StatusCommands::new(&mut repository)
            .evaluate(Some("fitness::physical"))
            .unwrap()
            .remove(0);
        assert_eq!(evaluation.scores[0].raw_value, Some(120.0));
        assert_eq!(evaluation.scores[0].score, Some(100.0));
        assert_eq!(evaluation.scores[1].score, None);
        assert_eq!(
            evaluation.scores[1].missing_record_ids,
            ["fitness.strength"]
        );
        assert_eq!(evaluation.score, Some(100.0));
        assert_eq!(evaluation.level, 5);
        assert_eq!(evaluation.level_title.as_deref(), Some("Peak"));
    }

    #[test]
    fn selections_are_local_idempotent_and_expose_disabled_dimensions() {
        let mut repository = repository();
        let mut commands = StatusCommands::new(&mut repository);
        assert!(
            commands
                .select(2, "fitness::physical".to_string())
                .unwrap()
                .changed
        );
        assert!(
            !commands
                .select(2, "fitness::physical".to_string())
                .unwrap()
                .changed
        );
        assert_eq!(commands.list_dimensions().unwrap().dimensions.len(), 1);

        let mut transaction = repository.begin_transaction().unwrap();
        transaction.set_pack_enabled("fitness", false).unwrap();
        transaction.commit().unwrap();
        let list = StatusCommands::new(&mut repository)
            .list_dimensions()
            .unwrap();
        assert!(list.dimensions.is_empty());
        assert_eq!(list.selections.len(), 1);
        assert!(!list.selections[0].available);

        let error = StatusCommands::new(&mut repository)
            .evaluate(Some("fitness::physical"))
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Unresolved);
    }

    #[test]
    fn clear_selection_is_idempotent_and_validates_position() {
        let mut repository = repository();
        let mut commands = StatusCommands::new(&mut repository);
        assert!(!commands.clear_selection(0).unwrap().changed);
        commands.select(0, "fitness::physical".to_string()).unwrap();
        assert!(commands.clear_selection(0).unwrap().changed);
        assert!(!commands.clear_selection(0).unwrap().changed);
        assert_eq!(
            commands.clear_selection(5).unwrap_err().code,
            RepositoryErrorCode::ValidationFailed
        );
    }
}
