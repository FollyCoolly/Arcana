use crate::domain::{
    ArcanaRepository, DerivedValueDefinition, DerivedValueRegistry, FormulaValue, Record,
    RepositoryError, RepositoryErrorCode, RepositoryResult, SyncedRepositorySnapshot,
    ValidationIssue,
};
use chrono::{Local, NaiveDate};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DerivedValueEvaluation {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub expression: String,
    pub as_of_date: String,
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_record_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DerivedValueComputation {
    pub value: Option<f64>,
    pub missing_record_ids: BTreeSet<String>,
}

pub struct DerivedValueCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> DerivedValueCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list(&self) -> RepositoryResult<Vec<DerivedValueEvaluation>> {
        self.list_on(Local::now().date_naive())
    }

    pub fn list_on(&self, as_of_date: NaiveDate) -> RepositoryResult<Vec<DerivedValueEvaluation>> {
        let snapshot = self.repository.load_synced_snapshot()?;
        let mut evaluator = FormulaSnapshotEvaluator::new(&snapshot, as_of_date)?;
        let ids = evaluator
            .registry
            .iter()
            .map(|(id, _)| id.to_string())
            .collect::<Vec<_>>();
        ids.into_iter()
            .map(|id| evaluator.evaluation(&id))
            .collect()
    }

    pub fn evaluate(&self, id: &str) -> RepositoryResult<DerivedValueEvaluation> {
        self.evaluate_on(id, Local::now().date_naive())
    }

    pub fn evaluate_on(
        &self,
        id: &str,
        as_of_date: NaiveDate,
    ) -> RepositoryResult<DerivedValueEvaluation> {
        let snapshot = self.repository.load_synced_snapshot()?;
        let mut evaluator = FormulaSnapshotEvaluator::new(&snapshot, as_of_date)?;
        evaluator.evaluation(id)
    }
}

pub(crate) struct FormulaSnapshotEvaluator {
    registry: DerivedValueRegistry,
    all_definition_ids: BTreeSet<String>,
    records: BTreeMap<String, FormulaValue>,
    as_of_date: NaiveDate,
    cache: BTreeMap<String, DerivedValueComputation>,
    visiting: BTreeSet<String>,
}

impl FormulaSnapshotEvaluator {
    pub(crate) fn new(
        snapshot: &SyncedRepositorySnapshot,
        as_of_date: NaiveDate,
    ) -> RepositoryResult<Self> {
        let registry = snapshot.derived_value_registry()?;
        let all_definition_ids = snapshot
            .packs
            .values()
            .flat_map(|pack| pack.derived_values.iter())
            .flat_map(|file| file.values.iter())
            .map(|definition| definition.id.clone())
            .collect();
        let records = snapshot
            .records
            .values()
            .flat_map(|file| file.records.iter())
            .filter_map(|record| match record {
                Record::Scalar(record) => {
                    let value = record
                        .value
                        .as_f64()
                        .map(FormulaValue::Number)
                        .or_else(|| {
                            record
                                .value
                                .as_str()
                                .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                                .map(FormulaValue::Date)
                        })?;
                    Some((record.definition_id.clone(), value))
                }
                Record::Collection(_) | Record::Event(_) => None,
            })
            .collect();
        Ok(Self {
            registry,
            all_definition_ids,
            records,
            as_of_date,
            cache: BTreeMap::new(),
            visiting: BTreeSet::new(),
        })
    }

    pub(crate) fn record_value(&self, id: &str) -> Option<FormulaValue> {
        self.records.get(id).cloned()
    }

    pub(crate) fn as_of_date(&self) -> NaiveDate {
        self.as_of_date
    }

    pub(crate) fn evaluate_id(&mut self, id: &str) -> RepositoryResult<DerivedValueComputation> {
        if let Some(cached) = self.cache.get(id) {
            return Ok(cached.clone());
        }
        let definition = self.registry.get(id).cloned().ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!("DerivedValue '{id}' is not supplied by an enabled Pack"),
            )
        })?;
        if !self.visiting.insert(id.to_string()) {
            return Err(derived_evaluation_error(
                &definition,
                "derived_value_cycle",
                "DerivedValue dependency cycle detected".to_string(),
            ));
        }

        let result = self.evaluate_definition(&definition);
        self.visiting.remove(id);
        let result = result?;
        self.cache.insert(id.to_string(), result.clone());
        Ok(result)
    }

    fn evaluate_definition(
        &mut self,
        definition: &DerivedValueDefinition,
    ) -> RepositoryResult<DerivedValueComputation> {
        let expression = definition.parse_expression().map_err(|error| {
            derived_evaluation_error(
                definition,
                "derived_expression_parse_failed",
                error.to_string(),
            )
        })?;
        let mut missing_record_ids = expression
            .record_references()
            .filter(|id| !self.records.contains_key(*id))
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        let mut derived_values = BTreeMap::new();
        for id in expression.derived_value_references() {
            let computation = self.evaluate_id(id)?;
            missing_record_ids.extend(computation.missing_record_ids);
            derived_values.insert(id.to_string(), computation.value);
        }
        let value = expression
            .evaluate(
                |id| self.record_value(id),
                |id| derived_values.get(id).copied().flatten(),
                self.as_of_date,
            )
            .map_err(|error| {
                derived_evaluation_error(
                    definition,
                    "derived_expression_evaluation_failed",
                    error.to_string(),
                )
            })?;
        Ok(DerivedValueComputation {
            value,
            missing_record_ids,
        })
    }

    fn evaluation(&mut self, id: &str) -> RepositoryResult<DerivedValueEvaluation> {
        let definition = self.registry.get(id).cloned().ok_or_else(|| {
            let code = if self.all_definition_ids.contains(id) {
                RepositoryErrorCode::Unresolved
            } else {
                RepositoryErrorCode::NotFound
            };
            RepositoryError::new(
                code,
                if code == RepositoryErrorCode::Unresolved {
                    format!("DerivedValue '{id}' is not supplied by an enabled Pack")
                } else {
                    format!("DerivedValue '{id}' was not found")
                },
            )
        })?;
        let computation = self.evaluate_id(id)?;
        Ok(DerivedValueEvaluation {
            id: definition.id,
            name: definition.name,
            description: definition.description,
            unit: definition.unit,
            expression: definition.expression,
            as_of_date: self.as_of_date.to_string(),
            value: computation.value,
            missing_record_ids: computation.missing_record_ids.into_iter().collect(),
        })
    }
}

fn derived_evaluation_error(
    definition: &DerivedValueDefinition,
    code: &str,
    message: String,
) -> RepositoryError {
    let mut error = RepositoryError::new(
        RepositoryErrorCode::ValidationFailed,
        format!(
            "failed to evaluate DerivedValue '{}': {message}",
            definition.id
        ),
    );
    error.validation_issues.push(ValidationIssue {
        code: code.to_string(),
        path: format!("derived_values.{}.expression", definition.id),
        message,
    });
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{RecordCommands, SetScalarRecord};
    use crate::domain::{
        ArcanaRepositoryReader, ArcanaRepositoryTransaction, DerivedValueFile, Pack, PackManifest,
        RecordDefinition, RecordDefinitionFile, ScalarRecordDefinition, ValueType,
        PACK_SCHEMA_VERSION,
    };
    use crate::storage::DataRepository;
    use serde_json::json;

    fn health_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: PACK_SCHEMA_VERSION,
                id: "health".to_string(),
                name: "Health".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: vec![],
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "health.height_m".to_string(),
                        name: "Height".to_string(),
                        description: None,
                        value_type: ValueType::Number,
                        unit: Some("m".to_string()),
                    }),
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "health.weight_kg".to_string(),
                        name: "Weight".to_string(),
                        description: None,
                        value_type: ValueType::Number,
                        unit: Some("kg".to_string()),
                    }),
                ],
            }),
            derived_values: Some(DerivedValueFile {
                values: vec![
                    DerivedValueDefinition {
                        id: "health.bmi".to_string(),
                        name: "BMI".to_string(),
                        description: None,
                        unit: None,
                        expression: "record('health.weight_kg') / (record('health.height_m') * record('health.height_m'))".to_string(),
                    },
                    DerivedValueDefinition {
                        id: "health.bmi_percent".to_string(),
                        name: "BMI percent".to_string(),
                        description: None,
                        unit: None,
                        expression: "derived('health.bmi') * 4".to_string(),
                    },
                ],
            }),
            dimensions: None,
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn repository() -> DataRepository {
        let mut repository = DataRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(health_pack()).unwrap();
        transaction.set_pack_enabled("health", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    #[test]
    fn evaluates_reusable_chained_values_without_persisting_results() {
        let mut repository = repository();
        let mut records = RecordCommands::new(&mut repository);
        records
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "health.height_m".to_string(),
                    value: json!(1.8),
                    effective_at: None,
                },
                "2026-08-17T12:00:00+08:00".to_string(),
            )
            .unwrap();
        records
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "health.weight_kg".to_string(),
                    value: json!(81),
                    effective_at: None,
                },
                "2026-08-17T12:00:00+08:00".to_string(),
            )
            .unwrap();

        let evaluation = DerivedValueCommands::new(&mut repository)
            .evaluate_on(
                "health.bmi_percent",
                NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
            )
            .unwrap();
        assert_eq!(evaluation.value, Some(100.0));
        assert!(evaluation.missing_record_ids.is_empty());
        assert!(repository.get_record("health.bmi").unwrap().is_none());
    }

    #[test]
    fn reports_missing_records_through_a_derived_chain() {
        let mut repository = repository();
        RecordCommands::new(&mut repository)
            .set_scalar_at(
                SetScalarRecord {
                    definition_id: "health.height_m".to_string(),
                    value: json!(1.8),
                    effective_at: None,
                },
                "2026-08-17T12:00:00+08:00".to_string(),
            )
            .unwrap();

        let evaluation = DerivedValueCommands::new(&mut repository)
            .evaluate_on(
                "health.bmi_percent",
                NaiveDate::from_ymd_opt(2026, 8, 17).unwrap(),
            )
            .unwrap();
        assert_eq!(evaluation.value, None);
        assert_eq!(evaluation.missing_record_ids, ["health.weight_kg"]);
    }
}
