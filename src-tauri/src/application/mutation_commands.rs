use super::achievement_commands::{apply_achievement_mutation, AchievementMutation};
use super::memory_commands::{apply_memory_mutation, MemoryMutation, MemoryMutationResult};
use super::mission_commands::{apply_mission_mutation, MissionMutation, MissionMutationResult};
use super::pack_commands::{apply_pack_mutation, PackMutation, PackMutationResult};
use super::record_commands::{apply_record_mutation, RecordMutation, RecordMutationResult};
use super::status_commands::{apply_status_mutation, StatusMutation};
use super::{
    AddCollectionItem, AppendEvent, CorrectCollectionItem, CorrectEvent, CreateAssistantMemory,
    CreateEmptyRecord, CreateMission, DeleteEvent, IncrementScalarRecord, PackContent,
    RemoveCollectionItem, SetAchievementState, SetScalarRecord, SuggestMission,
    UpdateAssistantMemory, UpdateMission,
};
use crate::domain::{
    ArcanaRepository, ArcanaRepositoryTransaction, RepositoryError, RepositoryErrorCode,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTarget {
    pub definition_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackTarget {
    pub pack_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusSelectionInput {
    pub position: u8,
    pub dimension_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusPositionInput {
    pub position: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AchievementTarget {
    pub achievement_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionTarget {
    pub mission_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionSuggestionTarget {
    pub suggestion_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistantMemoryTarget {
    pub memory_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "input", deny_unknown_fields)]
pub enum MutationOperation {
    #[serde(rename = "record.set")]
    RecordSet(SetScalarRecord),
    #[serde(rename = "record.increment")]
    RecordIncrement(IncrementScalarRecord),
    #[serde(rename = "record.correct")]
    RecordCorrect(SetScalarRecord),
    #[serde(rename = "record.create-empty-collection")]
    RecordCreateEmptyCollection(CreateEmptyRecord),
    #[serde(rename = "record.create-empty-event")]
    RecordCreateEmptyEvent(CreateEmptyRecord),
    #[serde(rename = "record.add-item")]
    RecordAddItem(AddCollectionItem),
    #[serde(rename = "record.correct-item")]
    RecordCorrectItem(CorrectCollectionItem),
    #[serde(rename = "record.remove-item")]
    RecordRemoveItem(RemoveCollectionItem),
    #[serde(rename = "record.append-event")]
    RecordAppendEvent(AppendEvent),
    #[serde(rename = "record.correct-event")]
    RecordCorrectEvent(CorrectEvent),
    #[serde(rename = "record.delete-event")]
    RecordDeleteEvent(DeleteEvent),
    #[serde(rename = "record.delete")]
    RecordDelete(RecordTarget),
    #[serde(rename = "pack.write")]
    PackWrite(PackContent),
    #[serde(rename = "pack.enable")]
    PackEnable(PackTarget),
    #[serde(rename = "pack.disable")]
    PackDisable(PackTarget),
    #[serde(rename = "pack.delete")]
    PackDelete(PackTarget),
    #[serde(rename = "status.select")]
    StatusSelect(StatusSelectionInput),
    #[serde(rename = "status.clear")]
    StatusClear(StatusPositionInput),
    #[serde(rename = "achievement.state-set")]
    AchievementStateSet(SetAchievementState),
    #[serde(rename = "achievement.state-revoke")]
    AchievementStateRevoke(AchievementTarget),
    #[serde(rename = "mission.create")]
    MissionCreate(CreateMission),
    #[serde(rename = "mission.update")]
    MissionUpdate(UpdateMission),
    #[serde(rename = "mission.complete")]
    MissionComplete(MissionTarget),
    #[serde(rename = "mission.archive")]
    MissionArchive(MissionTarget),
    #[serde(rename = "mission.delete")]
    MissionDelete(MissionTarget),
    #[serde(rename = "mission.suggest")]
    MissionSuggest(SuggestMission),
    #[serde(rename = "mission.accept")]
    MissionAccept(MissionSuggestionTarget),
    #[serde(rename = "mission.reject")]
    MissionReject(MissionSuggestionTarget),
    #[serde(rename = "mission.suggestion-delete")]
    MissionSuggestionDelete(MissionSuggestionTarget),
    #[serde(rename = "memory.create")]
    MemoryCreate(CreateAssistantMemory),
    #[serde(rename = "memory.update")]
    MemoryUpdate(UpdateAssistantMemory),
    #[serde(rename = "memory.delete")]
    MemoryDelete(AssistantMemoryTarget),
}

impl MutationOperation {
    pub fn operation_name(&self) -> &'static str {
        match self {
            Self::RecordSet(_) => "record.set",
            Self::RecordIncrement(_) => "record.increment",
            Self::RecordCorrect(_) => "record.correct",
            Self::RecordCreateEmptyCollection(_) => "record.create-empty-collection",
            Self::RecordCreateEmptyEvent(_) => "record.create-empty-event",
            Self::RecordAddItem(_) => "record.add-item",
            Self::RecordCorrectItem(_) => "record.correct-item",
            Self::RecordRemoveItem(_) => "record.remove-item",
            Self::RecordAppendEvent(_) => "record.append-event",
            Self::RecordCorrectEvent(_) => "record.correct-event",
            Self::RecordDeleteEvent(_) => "record.delete-event",
            Self::RecordDelete(_) => "record.delete",
            Self::PackWrite(_) => "pack.write",
            Self::PackEnable(_) => "pack.enable",
            Self::PackDisable(_) => "pack.disable",
            Self::PackDelete(_) => "pack.delete",
            Self::StatusSelect(_) => "status.select",
            Self::StatusClear(_) => "status.clear",
            Self::AchievementStateSet(_) => "achievement.state-set",
            Self::AchievementStateRevoke(_) => "achievement.state-revoke",
            Self::MissionCreate(_) => "mission.create",
            Self::MissionUpdate(_) => "mission.update",
            Self::MissionComplete(_) => "mission.complete",
            Self::MissionArchive(_) => "mission.archive",
            Self::MissionDelete(_) => "mission.delete",
            Self::MissionSuggest(_) => "mission.suggest",
            Self::MissionAccept(_) => "mission.accept",
            Self::MissionReject(_) => "mission.reject",
            Self::MissionSuggestionDelete(_) => "mission.suggestion-delete",
            Self::MemoryCreate(_) => "memory.create",
            Self::MemoryUpdate(_) => "memory.update",
            Self::MemoryDelete(_) => "memory.delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyMutationBatch {
    pub operations: Vec<MutationOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MutationOperationResult {
    pub index: usize,
    pub operation: String,
    pub result: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MutationBatchResult {
    pub dry_run: bool,
    pub operations: Vec<MutationOperationResult>,
}

#[derive(Debug)]
pub struct MutationBatchError {
    pub operation_index: Option<usize>,
    pub operation: Option<String>,
    pub source: RepositoryError,
}

impl From<RepositoryError> for MutationBatchError {
    fn from(source: RepositoryError) -> Self {
        batch_error(source)
    }
}

pub struct MutationCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> MutationCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn apply_one(
        &mut self,
        operation: MutationOperation,
        dry_run: bool,
    ) -> Result<Value, RepositoryError> {
        let batch = self
            .apply_batch(
                ApplyMutationBatch {
                    operations: vec![operation],
                },
                dry_run,
            )
            .map_err(|error| error.source)?;
        Ok(batch
            .operations
            .into_iter()
            .next()
            .expect("single-operation batch must return one result")
            .result)
    }

    pub fn apply_batch(
        &mut self,
        batch: ApplyMutationBatch,
        dry_run: bool,
    ) -> Result<MutationBatchResult, MutationBatchError> {
        if batch.operations.is_empty() {
            return Err(batch_error(RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                "batch must contain at least one operation",
            )));
        }

        let mut transaction = self.repository.begin_transaction().map_err(batch_error)?;
        let executed_at = now_rfc3339();
        let mut results = Vec::with_capacity(batch.operations.len());
        for (index, operation) in batch.operations.into_iter().enumerate() {
            let operation_name = operation.operation_name().to_string();
            match apply_operation(&mut transaction, operation, &executed_at) {
                Ok(result) => results.push(MutationOperationResult {
                    index,
                    operation: operation_name,
                    result,
                }),
                Err(source) => {
                    transaction.rollback().map_err(batch_error)?;
                    return Err(MutationBatchError {
                        operation_index: Some(index),
                        operation: Some(operation_name),
                        source,
                    });
                }
            }
        }

        if dry_run {
            transaction.rollback().map_err(batch_error)?;
        } else {
            transaction.commit().map_err(batch_error)?;
        }
        Ok(MutationBatchResult {
            dry_run,
            operations: results,
        })
    }
}

fn apply_operation<T>(
    transaction: &mut T,
    operation: MutationOperation,
    executed_at: &str,
) -> Result<Value, RepositoryError>
where
    T: ArcanaRepositoryTransaction,
{
    match operation {
        MutationOperation::RecordSet(command) | MutationOperation::RecordCorrect(command) => {
            record_result(apply_record_mutation(
                transaction,
                RecordMutation::SetScalar(command),
                executed_at,
            )?)
        }
        MutationOperation::RecordIncrement(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::IncrementScalar(command),
            executed_at,
        )?),
        MutationOperation::RecordCreateEmptyCollection(command) => {
            record_result(apply_record_mutation(
                transaction,
                RecordMutation::CreateEmptyCollection(command),
                executed_at,
            )?)
        }
        MutationOperation::RecordCreateEmptyEvent(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::CreateEmptyEvent(command),
            executed_at,
        )?),
        MutationOperation::RecordAddItem(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::AddCollectionItem(command),
            executed_at,
        )?),
        MutationOperation::RecordCorrectItem(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::CorrectCollectionItem(command),
            executed_at,
        )?),
        MutationOperation::RecordRemoveItem(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::RemoveCollectionItem(command),
            executed_at,
        )?),
        MutationOperation::RecordAppendEvent(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::AppendEvent(command),
            executed_at,
        )?),
        MutationOperation::RecordCorrectEvent(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::CorrectEvent(command),
            executed_at,
        )?),
        MutationOperation::RecordDeleteEvent(command) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::DeleteEvent(command),
            executed_at,
        )?),
        MutationOperation::RecordDelete(target) => record_result(apply_record_mutation(
            transaction,
            RecordMutation::Delete(target.definition_id),
            executed_at,
        )?),
        MutationOperation::PackWrite(content) => pack_result(apply_pack_mutation(
            transaction,
            PackMutation::Write(Box::new(content)),
        )?),
        MutationOperation::PackEnable(target) => pack_result(apply_pack_mutation(
            transaction,
            PackMutation::SetEnabled {
                pack_id: target.pack_id,
                enabled: true,
            },
        )?),
        MutationOperation::PackDisable(target) => pack_result(apply_pack_mutation(
            transaction,
            PackMutation::SetEnabled {
                pack_id: target.pack_id,
                enabled: false,
            },
        )?),
        MutationOperation::PackDelete(target) => pack_result(apply_pack_mutation(
            transaction,
            PackMutation::Delete(target.pack_id),
        )?),
        MutationOperation::StatusSelect(input) => Ok(json!({
            "selection": apply_status_mutation(
                transaction,
                StatusMutation::Select {
                    position: input.position,
                    dimension_id: input.dimension_id,
                },
            )?
        })),
        MutationOperation::StatusClear(input) => Ok(json!({
            "selection": apply_status_mutation(
                transaction,
                StatusMutation::Clear {
                    position: input.position,
                },
            )?
        })),
        MutationOperation::AchievementStateSet(command) => Ok(json!({
            "achievement_state": apply_achievement_mutation(
                transaction,
                AchievementMutation::Set(command),
            )?
        })),
        MutationOperation::AchievementStateRevoke(target) => Ok(json!({
            "achievement_state": apply_achievement_mutation(
                transaction,
                AchievementMutation::Revoke(target.achievement_id),
            )?
        })),
        MutationOperation::MissionCreate(command) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Create {
                command,
                mission_id: Uuid::now_v7().to_string(),
                created_at: executed_at.to_string(),
            },
        )?),
        MutationOperation::MissionUpdate(command) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Update(command),
        )?),
        MutationOperation::MissionComplete(target) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Complete {
                mission_id: target.mission_id,
                completed_at: executed_at.to_string(),
            },
        )?),
        MutationOperation::MissionArchive(target) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Archive(target.mission_id),
        )?),
        MutationOperation::MissionDelete(target) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Delete(target.mission_id),
        )?),
        MutationOperation::MissionSuggest(command) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Suggest {
                command,
                suggestion_id: Uuid::now_v7().to_string(),
                generated_at: executed_at.to_string(),
            },
        )?),
        MutationOperation::MissionAccept(target) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Accept {
                suggestion_id: target.suggestion_id,
                created_at: executed_at.to_string(),
            },
        )?),
        MutationOperation::MissionReject(target) => mission_result(apply_mission_mutation(
            transaction,
            MissionMutation::Reject(target.suggestion_id),
        )?),
        MutationOperation::MissionSuggestionDelete(target) => {
            mission_result(apply_mission_mutation(
                transaction,
                MissionMutation::SuggestionDelete(target.suggestion_id),
            )?)
        }
        MutationOperation::MemoryCreate(command) => memory_result(apply_memory_mutation(
            transaction,
            MemoryMutation::Create {
                command,
                memory_id: Uuid::now_v7().to_string(),
                created_at: executed_at.to_string(),
            },
        )?),
        MutationOperation::MemoryUpdate(command) => memory_result(apply_memory_mutation(
            transaction,
            MemoryMutation::Update {
                command,
                updated_at: executed_at.to_string(),
            },
        )?),
        MutationOperation::MemoryDelete(target) => memory_result(apply_memory_mutation(
            transaction,
            MemoryMutation::Delete(target.memory_id),
        )?),
    }
}

fn record_result(result: RecordMutationResult) -> Result<Value, RepositoryError> {
    Ok(match result {
        RecordMutationResult::Record(record) => json!({ "record": record }),
        RecordMutationResult::Deleted { definition_id } => {
            json!({ "deleted_definition_id": definition_id })
        }
    })
}

fn pack_result(result: PackMutationResult) -> Result<Value, RepositoryError> {
    Ok(match result {
        PackMutationResult::Pack(result) => json!(result),
        PackMutationResult::Enabled(result) => json!(result),
        PackMutationResult::Deleted(result) => json!(result),
    })
}

fn mission_result(result: MissionMutationResult) -> Result<Value, RepositoryError> {
    Ok(match result {
        MissionMutationResult::Mission(result) => json!(result),
        MissionMutationResult::Deleted(result) => json!(result),
        MissionMutationResult::Suggestion(result) => json!(result),
        MissionMutationResult::SuggestionDeleted(result) => json!(result),
    })
}

fn memory_result(result: MemoryMutationResult) -> Result<Value, RepositoryError> {
    Ok(match result {
        MemoryMutationResult::Memory(result) => json!(result),
        MemoryMutationResult::Deleted(result) => json!(result),
    })
}

fn batch_error(source: RepositoryError) -> MutationBatchError {
    MutationBatchError {
        operation_index: None,
        operation: None,
        source,
    }
}

fn now_rfc3339() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ArcanaRuntime;
    use crate::domain::ArcanaRepositoryReader;

    fn nickname(value: &str) -> MutationOperation {
        MutationOperation::RecordSet(SetScalarRecord {
            definition_id: "identity.nickname".to_string(),
            value: json!(value),
            effective_at: None,
        })
    }

    #[test]
    fn dry_run_returns_results_and_rolls_back() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let result = MutationCommands::new(repository).apply_batch(
                    ApplyMutationBatch {
                        operations: vec![nickname("Preview")],
                    },
                    true,
                );
                let result = result.unwrap();
                assert!(result.dry_run);
                assert_eq!(result.operations[0].result["record"]["value"], "Preview");
                assert!(repository.get_record("identity.nickname")?.is_none());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn failed_batch_rolls_back_earlier_operations() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let error = MutationCommands::new(repository)
                    .apply_batch(
                        ApplyMutationBatch {
                            operations: vec![
                                nickname("Should roll back"),
                                MutationOperation::RecordIncrement(IncrementScalarRecord {
                                    definition_id: "identity.nickname".to_string(),
                                    delta: json!(1),
                                    effective_at: None,
                                }),
                            ],
                        },
                        false,
                    )
                    .unwrap_err();
                assert_eq!(error.operation_index, Some(1));
                assert_eq!(error.operation.as_deref(), Some("record.increment"));
                assert!(repository.get_record("identity.nickname")?.is_none());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn pack_write_enable_and_delete_share_the_batch_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let content = PackContent::scaffold("cooking".to_string(), "Cooking".to_string())?;
                let created = MutationCommands::new(repository)
                    .apply_batch(
                        ApplyMutationBatch {
                            operations: vec![
                                MutationOperation::PackWrite(content),
                                MutationOperation::PackEnable(PackTarget {
                                    pack_id: "cooking".to_string(),
                                }),
                            ],
                        },
                        false,
                    )
                    .map_err(|error| error.source)?;
                assert_eq!(created.operations[1].result["enabled"], true);
                assert!(repository
                    .load_synced_snapshot()?
                    .manifest
                    .enabled_pack_ids
                    .contains(&"cooking".to_string()));

                let preview = MutationCommands::new(repository)
                    .apply_batch(
                        ApplyMutationBatch {
                            operations: vec![MutationOperation::PackDelete(PackTarget {
                                pack_id: "cooking".to_string(),
                            })],
                        },
                        true,
                    )
                    .map_err(|error| error.source)?;
                assert!(preview.dry_run);
                assert_eq!(preview.operations[0].result["was_enabled"], true);
                assert!(repository
                    .load_synced_snapshot()?
                    .packs
                    .contains_key("cooking"));
                Ok(())
            })
            .unwrap();
    }
}
