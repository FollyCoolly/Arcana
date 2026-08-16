use crate::domain::{
    ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction, AssistantMemory,
    AssistantMemoryKind, RepositoryError, RepositoryErrorCode, RepositoryResult,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryAssistantMemory {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<AssistantMemoryKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAssistantMemory {
    pub kind: AssistantMemoryKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAssistantMemory {
    pub memory_id: String,
    pub kind: AssistantMemoryKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssistantMemoryResult {
    pub memory: AssistantMemory,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssistantMemoryDeleteResult {
    pub memory_id: String,
    pub deleted: bool,
}

pub struct MemoryCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> MemoryCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list(&mut self, query: QueryAssistantMemory) -> RepositoryResult<Vec<AssistantMemory>> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let memories = snapshot
            .assistant_memory
            .into_iter()
            .flat_map(|file| file.memories)
            .filter(|memory| matches_query(memory, &query))
            .collect();
        transaction.rollback()?;
        Ok(memories)
    }

    pub fn create(
        &mut self,
        command: CreateAssistantMemory,
    ) -> RepositoryResult<AssistantMemoryResult> {
        self.create_at(command, Uuid::now_v7().to_string(), now_rfc3339())
    }

    pub fn update(
        &mut self,
        command: UpdateAssistantMemory,
    ) -> RepositoryResult<AssistantMemoryResult> {
        self.update_at(command, now_rfc3339())
    }

    pub fn delete(&mut self, memory_id: &str) -> RepositoryResult<AssistantMemoryDeleteResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        memory_from_snapshot(&snapshot, memory_id)?;
        transaction.delete_assistant_memory(memory_id)?;
        transaction.commit()?;
        Ok(AssistantMemoryDeleteResult {
            memory_id: memory_id.to_string(),
            deleted: true,
        })
    }

    pub(crate) fn create_at(
        &mut self,
        command: CreateAssistantMemory,
        memory_id: String,
        created_at: String,
    ) -> RepositoryResult<AssistantMemoryResult> {
        let memory = AssistantMemory {
            id: memory_id,
            kind: command.kind,
            content: command.content,
            created_at: created_at.clone(),
            updated_at: created_at,
        };
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        if snapshot.assistant_memory.as_ref().is_some_and(|file| {
            file.memories
                .iter()
                .any(|existing| existing.id == memory.id)
        }) {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("AssistantMemory '{}' already exists", memory.id),
            ));
        }
        transaction.put_assistant_memory(memory.clone())?;
        transaction.commit()?;
        Ok(AssistantMemoryResult {
            memory,
            changed: true,
        })
    }

    pub(crate) fn update_at(
        &mut self,
        command: UpdateAssistantMemory,
        updated_at: String,
    ) -> RepositoryResult<AssistantMemoryResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let current = memory_from_snapshot(&snapshot, &command.memory_id)?;
        if current.kind == command.kind && current.content == command.content {
            transaction.rollback()?;
            return Ok(AssistantMemoryResult {
                memory: current,
                changed: false,
            });
        }
        let memory = AssistantMemory {
            id: current.id,
            kind: command.kind,
            content: command.content,
            created_at: current.created_at,
            updated_at,
        };
        transaction.put_assistant_memory(memory.clone())?;
        transaction.commit()?;
        Ok(AssistantMemoryResult {
            memory,
            changed: true,
        })
    }
}

fn memory_from_snapshot(
    snapshot: &crate::domain::SyncedRepositorySnapshot,
    memory_id: &str,
) -> RepositoryResult<AssistantMemory> {
    snapshot
        .assistant_memory
        .iter()
        .flat_map(|file| file.memories.iter())
        .find(|memory| memory.id == memory_id)
        .cloned()
        .ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!("AssistantMemory '{memory_id}' was not found"),
            )
        })
}

fn matches_query(memory: &AssistantMemory, query: &QueryAssistantMemory) -> bool {
    query
        .memory_id
        .as_deref()
        .is_none_or(|memory_id| memory.id == memory_id)
        && query.kind.is_none_or(|kind| memory.kind == kind)
}

fn now_rfc3339() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ArcanaRuntime;

    #[test]
    fn create_query_update_and_delete_preserve_memory_identity() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut commands = MemoryCommands::new(repository);
                let created = commands.create_at(
                    CreateAssistantMemory {
                        kind: AssistantMemoryKind::Preference,
                        content: "Prefer short missions".to_string(),
                    },
                    "019b2234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-16T10:00:00Z".to_string(),
                )?;
                assert_eq!(created.memory.created_at, created.memory.updated_at);
                assert_eq!(
                    commands
                        .list(QueryAssistantMemory {
                            memory_id: None,
                            kind: Some(AssistantMemoryKind::Preference),
                        })?
                        .len(),
                    1
                );

                let updated = commands.update_at(
                    UpdateAssistantMemory {
                        memory_id: created.memory.id.clone(),
                        kind: AssistantMemoryKind::Constraint,
                        content: "Missions must fit within one week".to_string(),
                    },
                    "2026-08-17T10:00:00Z".to_string(),
                )?;
                assert!(updated.changed);
                assert_eq!(updated.memory.created_at, "2026-08-16T10:00:00Z");
                assert_eq!(updated.memory.updated_at, "2026-08-17T10:00:00Z");
                assert!(
                    !commands
                        .update(UpdateAssistantMemory {
                            memory_id: updated.memory.id.clone(),
                            kind: updated.memory.kind,
                            content: updated.memory.content.clone(),
                        })?
                        .changed
                );

                commands.delete(&updated.memory.id)?;
                assert!(commands.list(QueryAssistantMemory::default())?.is_empty());
                assert_eq!(
                    commands.delete(&updated.memory.id).unwrap_err().code,
                    RepositoryErrorCode::NotFound
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn update_rejects_a_timestamp_before_creation() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut commands = MemoryCommands::new(repository);
                let created = commands.create_at(
                    CreateAssistantMemory {
                        kind: AssistantMemoryKind::Reminder,
                        content: "Ask about cooking history later".to_string(),
                    },
                    "019b2234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-17T10:00:00Z".to_string(),
                )?;
                assert_eq!(
                    commands
                        .update_at(
                            UpdateAssistantMemory {
                                memory_id: created.memory.id,
                                kind: AssistantMemoryKind::Reminder,
                                content: "Ask about cooking history next time".to_string(),
                            },
                            "2026-08-16T10:00:00Z".to_string(),
                        )
                        .unwrap_err()
                        .code,
                    RepositoryErrorCode::ValidationFailed
                );
                Ok(())
            })
            .unwrap();
    }
}
