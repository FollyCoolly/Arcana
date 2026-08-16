use crate::domain::{
    ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction, Mission,
    MissionDifficulty, MissionStatus, MissionSuggestion, MissionSuggestionStatus, RepositoryError,
    RepositoryErrorCode, RepositoryResult,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryMissions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<MissionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateMission {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<MissionDifficulty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// Full replacement of a Mission's editable fields. Omitted optional fields
/// are cleared; lifecycle and timestamp fields remain command-owned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMission {
    pub mission_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<MissionDifficulty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionResult {
    pub mission: Mission,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionDeleteResult {
    pub mission_id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryMissionSuggestions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<MissionSuggestionStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuggestMission {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<MissionDifficulty>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_mission_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionSuggestionResult {
    pub suggestion: MissionSuggestion,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionSuggestionDeleteResult {
    pub suggestion_id: String,
    pub deleted: bool,
}

pub struct MissionCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> MissionCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list(&mut self, query: QueryMissions) -> RepositoryResult<Vec<Mission>> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let missions = snapshot
            .missions
            .into_iter()
            .flat_map(|file| file.missions)
            .filter(|mission| matches_mission_query(mission, &query))
            .collect();
        transaction.rollback()?;
        Ok(missions)
    }

    pub fn create(&mut self, command: CreateMission) -> RepositoryResult<MissionResult> {
        self.create_at(command, Uuid::now_v7().to_string(), now_rfc3339())
    }

    pub fn update(&mut self, command: UpdateMission) -> RepositoryResult<MissionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let current = mission_from_snapshot(&snapshot, &command.mission_id)?;
        let mission = Mission {
            id: current.id.clone(),
            title: command.title,
            description: command.description,
            status: current.status,
            progress: command.progress,
            difficulty: command.difficulty,
            deadline: command.deadline,
            parent_id: command.parent_id,
            created_at: current.created_at.clone(),
            completed_at: current.completed_at.clone(),
        };
        if mission == current {
            transaction.rollback()?;
            return Ok(MissionResult {
                mission,
                changed: false,
            });
        }
        transaction.put_mission(mission.clone())?;
        transaction.commit()?;
        Ok(MissionResult {
            mission,
            changed: true,
        })
    }

    pub fn complete(&mut self, mission_id: &str) -> RepositoryResult<MissionResult> {
        self.complete_at(mission_id, now_rfc3339())
    }

    pub fn archive(&mut self, mission_id: &str) -> RepositoryResult<MissionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let mut mission = mission_from_snapshot(&snapshot, mission_id)?;
        if mission.status == MissionStatus::Archived {
            transaction.rollback()?;
            return Ok(MissionResult {
                mission,
                changed: false,
            });
        }
        mission.status = MissionStatus::Archived;
        transaction.put_mission(mission.clone())?;
        transaction.commit()?;
        Ok(MissionResult {
            mission,
            changed: true,
        })
    }

    pub fn delete(&mut self, mission_id: &str) -> RepositoryResult<MissionDeleteResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        mission_from_snapshot(&snapshot, mission_id)?;
        if let Some(child) = snapshot
            .missions
            .iter()
            .flat_map(|file| file.missions.iter())
            .find(|mission| mission.parent_id.as_deref() == Some(mission_id))
        {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!(
                    "Mission '{mission_id}' is still the parent of Mission '{}'",
                    child.id
                ),
            ));
        }
        transaction.delete_mission(mission_id)?;
        transaction.commit()?;
        Ok(MissionDeleteResult {
            mission_id: mission_id.to_string(),
            deleted: true,
        })
    }

    pub fn list_suggestions(
        &mut self,
        query: QueryMissionSuggestions,
    ) -> RepositoryResult<Vec<MissionSuggestion>> {
        let transaction = self.repository.begin_transaction()?;
        let suggestions = transaction
            .list_mission_suggestions()?
            .into_iter()
            .filter(|suggestion| matches_suggestion_query(suggestion, &query))
            .collect();
        transaction.rollback()?;
        Ok(suggestions)
    }

    pub fn suggest(
        &mut self,
        command: SuggestMission,
    ) -> RepositoryResult<MissionSuggestionResult> {
        self.suggest_at(command, Uuid::now_v7().to_string(), now_rfc3339())
    }

    pub fn accept(&mut self, suggestion_id: &str) -> RepositoryResult<MissionResult> {
        let created_at = now_rfc3339();
        let mut transaction = self.repository.begin_transaction()?;
        let suggestion = suggestion_from_transaction(&transaction, suggestion_id)?;
        let snapshot = transaction.load_synced_snapshot()?;
        if snapshot.missions.as_ref().is_some_and(|file| {
            file.missions
                .iter()
                .any(|mission| mission.id == suggestion.id)
        }) {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("Mission '{}' already exists", suggestion.id),
            ));
        }
        let mission = Mission {
            id: suggestion.id.clone(),
            title: suggestion.title,
            description: suggestion.description,
            status: MissionStatus::Active,
            progress: None,
            difficulty: suggestion.difficulty,
            deadline: suggestion.deadline,
            parent_id: suggestion.parent_mission_id,
            created_at,
            completed_at: None,
        };
        transaction.put_mission(mission.clone())?;
        transaction.delete_mission_suggestion(suggestion_id)?;
        transaction.commit()?;
        Ok(MissionResult {
            mission,
            changed: true,
        })
    }

    pub fn reject(&mut self, suggestion_id: &str) -> RepositoryResult<MissionSuggestionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let mut suggestion = suggestion_from_transaction(&transaction, suggestion_id)?;
        if suggestion.status == MissionSuggestionStatus::Rejected {
            transaction.rollback()?;
            return Ok(MissionSuggestionResult {
                suggestion,
                changed: false,
            });
        }
        suggestion.status = MissionSuggestionStatus::Rejected;
        transaction.put_mission_suggestion(suggestion.clone())?;
        transaction.commit()?;
        Ok(MissionSuggestionResult {
            suggestion,
            changed: true,
        })
    }

    pub fn delete_suggestion(
        &mut self,
        suggestion_id: &str,
    ) -> RepositoryResult<MissionSuggestionDeleteResult> {
        let mut transaction = self.repository.begin_transaction()?;
        suggestion_from_transaction(&transaction, suggestion_id)?;
        transaction.delete_mission_suggestion(suggestion_id)?;
        transaction.commit()?;
        Ok(MissionSuggestionDeleteResult {
            suggestion_id: suggestion_id.to_string(),
            deleted: true,
        })
    }

    pub(crate) fn create_at(
        &mut self,
        command: CreateMission,
        mission_id: String,
        created_at: String,
    ) -> RepositoryResult<MissionResult> {
        let mission = Mission {
            id: mission_id,
            title: command.title,
            description: command.description,
            status: MissionStatus::Active,
            progress: command.progress,
            difficulty: command.difficulty,
            deadline: command.deadline,
            parent_id: command.parent_id,
            created_at,
            completed_at: None,
        };
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        if snapshot
            .missions
            .as_ref()
            .is_some_and(|file| file.missions.iter().any(|item| item.id == mission.id))
        {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("Mission '{}' already exists", mission.id),
            ));
        }
        transaction.put_mission(mission.clone())?;
        transaction.commit()?;
        Ok(MissionResult {
            mission,
            changed: true,
        })
    }

    pub(crate) fn complete_at(
        &mut self,
        mission_id: &str,
        completed_at: String,
    ) -> RepositoryResult<MissionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let mut mission = mission_from_snapshot(&snapshot, mission_id)?;
        match mission.status {
            MissionStatus::Completed => {
                transaction.rollback()?;
                return Ok(MissionResult {
                    mission,
                    changed: false,
                });
            }
            MissionStatus::Archived => {
                return Err(RepositoryError::new(
                    RepositoryErrorCode::Conflict,
                    format!("archived Mission '{mission_id}' cannot be completed"),
                ));
            }
            MissionStatus::Active => {}
        }
        mission.status = MissionStatus::Completed;
        if mission.progress.is_some() {
            mission.progress = Some(100);
        }
        mission.completed_at = Some(completed_at);
        transaction.put_mission(mission.clone())?;
        transaction.commit()?;
        Ok(MissionResult {
            mission,
            changed: true,
        })
    }

    pub(crate) fn suggest_at(
        &mut self,
        command: SuggestMission,
        suggestion_id: String,
        generated_at: String,
    ) -> RepositoryResult<MissionSuggestionResult> {
        let suggestion = MissionSuggestion {
            id: suggestion_id,
            title: command.title,
            description: command.description,
            difficulty: command.difficulty,
            deadline: command.deadline,
            parent_mission_id: command.parent_mission_id,
            reason: command.reason,
            generated_at,
            status: MissionSuggestionStatus::Pending,
        };
        let mut transaction = self.repository.begin_transaction()?;
        if transaction
            .list_mission_suggestions()?
            .iter()
            .any(|item| item.id == suggestion.id)
        {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("MissionSuggestion '{}' already exists", suggestion.id),
            ));
        }
        transaction.put_mission_suggestion(suggestion.clone())?;
        transaction.commit()?;
        Ok(MissionSuggestionResult {
            suggestion,
            changed: true,
        })
    }
}

fn mission_from_snapshot(
    snapshot: &crate::domain::SyncedRepositorySnapshot,
    mission_id: &str,
) -> RepositoryResult<Mission> {
    snapshot
        .missions
        .iter()
        .flat_map(|file| file.missions.iter())
        .find(|mission| mission.id == mission_id)
        .cloned()
        .ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!("Mission '{mission_id}' was not found"),
            )
        })
}

fn suggestion_from_transaction<T>(
    transaction: &T,
    suggestion_id: &str,
) -> RepositoryResult<MissionSuggestion>
where
    T: ArcanaRepositoryReader,
{
    transaction
        .list_mission_suggestions()?
        .into_iter()
        .find(|suggestion| suggestion.id == suggestion_id)
        .ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!("MissionSuggestion '{suggestion_id}' was not found"),
            )
        })
}

fn matches_mission_query(mission: &Mission, query: &QueryMissions) -> bool {
    query
        .mission_id
        .as_deref()
        .is_none_or(|mission_id| mission.id == mission_id)
        && query.status.is_none_or(|status| mission.status == status)
        && query
            .parent_id
            .as_deref()
            .is_none_or(|parent_id| mission.parent_id.as_deref() == Some(parent_id))
}

fn matches_suggestion_query(
    suggestion: &MissionSuggestion,
    query: &QueryMissionSuggestions,
) -> bool {
    query
        .suggestion_id
        .as_deref()
        .is_none_or(|suggestion_id| suggestion.id == suggestion_id)
        && query
            .status
            .is_none_or(|status| suggestion.status == status)
}

fn now_rfc3339() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ArcanaRuntime;

    fn create_command(parent_id: Option<String>) -> CreateMission {
        CreateMission {
            title: "Read Rust Book".to_string(),
            description: Some("Finish the ownership chapter".to_string()),
            progress: Some(25),
            difficulty: Some(MissionDifficulty::B),
            deadline: Some("2026-12-31".to_string()),
            parent_id,
        }
    }

    #[test]
    fn mission_lifecycle_preserves_identity_and_completion_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut commands = MissionCommands::new(repository);
                let created = commands.create_at(
                    create_command(None),
                    "019b1234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-16T10:00:00Z".to_string(),
                )?;
                assert_eq!(created.mission.status, MissionStatus::Active);

                let updated = commands.update(UpdateMission {
                    mission_id: created.mission.id.clone(),
                    title: "Read all of Rust Book".to_string(),
                    description: None,
                    progress: Some(60),
                    difficulty: Some(MissionDifficulty::A),
                    deadline: None,
                    parent_id: None,
                })?;
                assert!(updated.changed);
                assert_eq!(updated.mission.created_at, "2026-08-16T10:00:00Z");
                assert!(updated.mission.description.is_none());

                let completed = commands
                    .complete_at(&created.mission.id, "2026-08-20T12:00:00Z".to_string())?;
                assert_eq!(completed.mission.status, MissionStatus::Completed);
                assert_eq!(completed.mission.progress, Some(100));
                assert!(!commands.complete(&created.mission.id)?.changed);

                let archived = commands.archive(&created.mission.id)?;
                assert_eq!(archived.mission.status, MissionStatus::Archived);
                assert_eq!(
                    archived.mission.completed_at.as_deref(),
                    Some("2026-08-20T12:00:00Z")
                );
                assert!(!commands.archive(&created.mission.id)?.changed);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn parent_prevents_delete_until_child_is_removed() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut commands = MissionCommands::new(repository);
                let parent = commands.create_at(
                    create_command(None),
                    "019b1234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-16T10:00:00Z".to_string(),
                )?;
                let child = commands.create_at(
                    create_command(Some(parent.mission.id.clone())),
                    "019b1234-89ab-7def-8123-456789abcdf0".to_string(),
                    "2026-08-16T10:01:00Z".to_string(),
                )?;
                assert_eq!(
                    commands.delete(&parent.mission.id).unwrap_err().code,
                    RepositoryErrorCode::Conflict
                );
                commands.delete(&child.mission.id)?;
                commands.delete(&parent.mission.id)?;
                assert!(commands.list(QueryMissions::default())?.is_empty());
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn suggestion_reject_accept_and_delete_are_local_lifecycle_operations() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut commands = MissionCommands::new(repository);
                let suggestion = commands.suggest_at(
                    SuggestMission {
                        title: "Try Rustlings".to_string(),
                        description: None,
                        difficulty: Some(MissionDifficulty::C),
                        deadline: None,
                        parent_mission_id: None,
                        reason: Some("Practice ownership".to_string()),
                    },
                    "019b1234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-16T10:00:00Z".to_string(),
                )?;
                assert_eq!(
                    suggestion.suggestion.status,
                    MissionSuggestionStatus::Pending
                );
                assert!(commands.reject(&suggestion.suggestion.id)?.changed);
                assert!(!commands.reject(&suggestion.suggestion.id)?.changed);

                let accepted = commands.accept(&suggestion.suggestion.id)?;
                assert_eq!(accepted.mission.id, suggestion.suggestion.id);
                assert_eq!(accepted.mission.status, MissionStatus::Active);
                assert!(commands
                    .list_suggestions(QueryMissionSuggestions::default())?
                    .is_empty());

                let disposable = commands.suggest_at(
                    SuggestMission {
                        title: "Disposable".to_string(),
                        description: None,
                        difficulty: None,
                        deadline: None,
                        parent_mission_id: None,
                        reason: None,
                    },
                    "019b1234-89ab-7def-8123-456789abcdf0".to_string(),
                    "2026-08-16T10:01:00Z".to_string(),
                )?;
                commands.delete_suggestion(&disposable.suggestion.id)?;
                assert!(commands
                    .list_suggestions(QueryMissionSuggestions::default())?
                    .is_empty());
                Ok(())
            })
            .unwrap();
    }
}
