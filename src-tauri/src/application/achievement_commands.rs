use crate::domain::{
    split_scoped_id, AchievementDefinition, AchievementState, AchievementStatus, ArcanaRepository,
    ArcanaRepositoryReader, ArcanaRepositoryTransaction, RepositoryError, RepositoryErrorCode,
    RepositoryResult, Validate, ValidationErrors, ValidationIssue,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryAchievements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achievement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AchievementStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_record_definition_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAchievementState {
    pub achievement_id: String,
    pub status: AchievementStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achieved_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AchievementAvailability {
    Locked,
    Available,
    Tracked,
    Achieved,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AchievementEntry {
    pub achievement_id: String,
    pub pack_id: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<AchievementDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<AchievementState>,
    pub availability: AchievementAvailability,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unmet_prerequisite_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AchievementStateResult {
    pub achievement_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<AchievementState>,
    pub changed: bool,
}

pub(crate) enum AchievementMutation {
    Set(SetAchievementState),
    Revoke(String),
}

pub struct AchievementCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> AchievementCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list(&mut self, query: QueryAchievements) -> RepositoryResult<Vec<AchievementEntry>> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let states = snapshot
            .achievement_states
            .as_ref()
            .map(|file| &file.states);
        let achieved_ids: BTreeSet<String> = states
            .into_iter()
            .flat_map(|states| states.iter())
            .filter(|(_, state)| state.is_achieved())
            .map(|(id, _)| id.clone())
            .collect();
        let enabled_pack_ids: BTreeSet<&str> = snapshot
            .manifest
            .enabled_pack_ids
            .iter()
            .map(String::as_str)
            .collect();
        let mut definitions: BTreeMap<&str, (&str, &AchievementDefinition)> = BTreeMap::new();
        for (pack_id, pack) in &snapshot.packs {
            for definition in pack
                .achievements
                .iter()
                .flat_map(|file| file.achievements.iter())
            {
                definitions.insert(definition.id.as_str(), (pack_id.as_str(), definition));
            }
        }

        let mut entries = BTreeMap::new();
        for (achievement_id, (pack_id, definition)) in &definitions {
            if !enabled_pack_ids.contains(pack_id) {
                continue;
            }
            let state = states
                .and_then(|states| states.get(*achievement_id))
                .cloned();
            let (availability, unmet_prerequisite_ids) =
                achievement_availability(definition, state.as_ref(), &achieved_ids);
            entries.insert(
                (*achievement_id).to_string(),
                AchievementEntry {
                    achievement_id: (*achievement_id).to_string(),
                    pack_id: (*pack_id).to_string(),
                    enabled: true,
                    definition: Some((*definition).clone()),
                    state,
                    availability,
                    unmet_prerequisite_ids,
                },
            );
        }

        if let Some(states) = states {
            for (achievement_id, state) in states {
                if entries.contains_key(achievement_id) {
                    continue;
                }
                let (pack_id, definition) = definitions
                    .get(achievement_id.as_str())
                    .map(|(pack_id, definition)| {
                        ((*pack_id).to_string(), Some((*definition).clone()))
                    })
                    .unwrap_or_else(|| {
                        (
                            split_scoped_id(achievement_id)
                                .map(|(pack_id, _)| pack_id.to_string())
                                .unwrap_or_default(),
                            None,
                        )
                    });
                entries.insert(
                    achievement_id.clone(),
                    AchievementEntry {
                        achievement_id: achievement_id.clone(),
                        pack_id,
                        enabled: false,
                        definition,
                        state: Some(state.clone()),
                        availability: AchievementAvailability::Unresolved,
                        unmet_prerequisite_ids: Vec::new(),
                    },
                );
            }
        }

        let result = entries
            .into_values()
            .filter(|entry| matches_query(entry, &query))
            .collect();
        transaction.rollback()?;
        Ok(result)
    }

    pub fn set_state(
        &mut self,
        command: SetAchievementState,
    ) -> RepositoryResult<AchievementStateResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let result =
            apply_achievement_mutation(&mut transaction, AchievementMutation::Set(command))?;
        if !result.changed {
            transaction.rollback()?;
            return Ok(result);
        }
        transaction.commit()?;
        Ok(result)
    }

    /// Explicit revocation is always allowed, including for states whose Pack
    /// or AchievementDefinition is currently unavailable.
    pub fn revoke_state(
        &mut self,
        achievement_id: &str,
    ) -> RepositoryResult<AchievementStateResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let result = apply_achievement_mutation(
            &mut transaction,
            AchievementMutation::Revoke(achievement_id.to_string()),
        )?;
        if !result.changed {
            transaction.rollback()?;
            return Ok(result);
        }
        transaction.commit()?;
        Ok(result)
    }
}

pub(crate) fn apply_achievement_mutation<T>(
    transaction: &mut T,
    mutation: AchievementMutation,
) -> RepositoryResult<AchievementStateResult>
where
    T: ArcanaRepositoryTransaction,
{
    match mutation {
        AchievementMutation::Set(command) => {
            validate_achievement_id(&command.achievement_id)?;
            let state = AchievementState {
                status: command.status,
                achieved_at: command.achieved_at,
            };
            state.validate()?;
            let snapshot = transaction.load_synced_snapshot()?;
            let available = snapshot
                .manifest
                .enabled_pack_ids
                .iter()
                .filter_map(|pack_id| snapshot.packs.get(pack_id))
                .flat_map(|pack| pack.achievements.iter())
                .flat_map(|file| file.achievements.iter())
                .any(|definition| definition.id == command.achievement_id);
            if !available {
                return Err(RepositoryError::new(
                    RepositoryErrorCode::Unresolved,
                    format!(
                        "AchievementDefinition '{}' is not supplied by an enabled Pack",
                        command.achievement_id
                    ),
                ));
            }
            let current = snapshot
                .achievement_states
                .as_ref()
                .and_then(|file| file.states.get(&command.achievement_id));
            if current == Some(&state) {
                return Ok(AchievementStateResult {
                    achievement_id: command.achievement_id,
                    state: Some(state),
                    changed: false,
                });
            }
            transaction.set_achievement_state(&command.achievement_id, state.clone())?;
            Ok(AchievementStateResult {
                achievement_id: command.achievement_id,
                state: Some(state),
                changed: true,
            })
        }
        AchievementMutation::Revoke(achievement_id) => {
            validate_achievement_id(&achievement_id)?;
            let snapshot = transaction.load_synced_snapshot()?;
            let exists = snapshot
                .achievement_states
                .as_ref()
                .is_some_and(|file| file.states.contains_key(&achievement_id));
            if !exists {
                return Ok(AchievementStateResult {
                    achievement_id,
                    state: None,
                    changed: false,
                });
            }
            transaction.revoke_achievement_state(&achievement_id)?;
            Ok(AchievementStateResult {
                achievement_id,
                state: None,
                changed: true,
            })
        }
    }
}

pub(crate) fn achievement_availability(
    definition: &AchievementDefinition,
    state: Option<&AchievementState>,
    achieved_ids: &BTreeSet<String>,
) -> (AchievementAvailability, Vec<String>) {
    let unmet_prerequisite_ids = definition
        .prerequisites
        .iter()
        .filter(|id| !achieved_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    let availability = match state.map(|state| state.status) {
        Some(AchievementStatus::Tracked) => AchievementAvailability::Tracked,
        Some(AchievementStatus::Achieved) => AchievementAvailability::Achieved,
        None if unmet_prerequisite_ids.is_empty() => AchievementAvailability::Available,
        None => AchievementAvailability::Locked,
    };
    (availability, unmet_prerequisite_ids)
}

fn validate_achievement_id(achievement_id: &str) -> RepositoryResult<()> {
    if split_scoped_id(achievement_id).is_some() {
        return Ok(());
    }
    Err(RepositoryError::validation(ValidationErrors::new(vec![
        ValidationIssue {
            code: "invalid_achievement_id".to_string(),
            path: "achievement_id".to_string(),
            message: "must be <pack_id>::<local_id> using lowercase snake_case".to_string(),
        },
    ])))
}

fn matches_query(entry: &AchievementEntry, query: &QueryAchievements) -> bool {
    query
        .achievement_id
        .as_deref()
        .is_none_or(|id| entry.achievement_id == id)
        && query
            .pack_id
            .as_deref()
            .is_none_or(|pack_id| entry.pack_id == pack_id)
        && query
            .status
            .is_none_or(|status| entry.state.as_ref().map(|state| state.status) == Some(status))
        && query
            .related_record_definition_id
            .as_deref()
            .is_none_or(|definition_id| {
                entry.definition.as_ref().is_some_and(|definition| {
                    definition
                        .related_record_definition_ids
                        .binary_search_by(|id| id.as_str().cmp(definition_id))
                        .is_ok()
                })
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::basic_pack;
    use crate::domain::{
        AchievementDifficulty, AchievementFile, ArcanaRepositoryTransaction, Pack, PackManifest,
        RecordDefinition, RecordDefinitionFile, ScalarRecordDefinition, ValueType, SCHEMA_VERSION,
    };
    use crate::storage::DataRepository;
    use std::collections::BTreeMap;

    fn achievement_pack() -> Pack {
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
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![RecordDefinition::Scalar(ScalarRecordDefinition {
                    id: "cooking.dish_count".to_string(),
                    name: "Dish count".to_string(),
                    description: None,
                    value_type: ValueType::Integer,
                    unit: None,
                })],
            }),
            derived_values: None,
            dimensions: None,
            achievements: Some(AchievementFile {
                achievements: vec![
                    AchievementDefinition {
                        id: "cooking::first_dish".to_string(),
                        name: "First dish".to_string(),
                        description: "Cook one dish".to_string(),
                        difficulty: AchievementDifficulty::Beginner,
                        tags: Vec::new(),
                        prerequisites: Vec::new(),
                        related_record_definition_ids: vec!["cooking.dish_count".to_string()],
                        tip: None,
                    },
                    AchievementDefinition {
                        id: "cooking::host_dinner".to_string(),
                        name: "Host dinner".to_string(),
                        description: "Host a dinner".to_string(),
                        difficulty: AchievementDifficulty::Intermediate,
                        tags: Vec::new(),
                        prerequisites: vec!["cooking::first_dish".to_string()],
                        related_record_definition_ids: Vec::new(),
                        tip: None,
                    },
                ],
            }),
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn repository() -> DataRepository {
        let mut repository = DataRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.put_pack(basic_pack()).unwrap();
        transaction.set_pack_enabled("basic", true).unwrap();
        transaction.put_pack(achievement_pack()).unwrap();
        transaction.set_pack_enabled("cooking", true).unwrap();
        transaction.commit().unwrap();
        repository
    }

    #[test]
    fn list_derives_availability_and_supports_related_record_filter() {
        let mut repository = repository();
        let mut commands = AchievementCommands::new(&mut repository);
        let entries = commands.list(QueryAchievements::default()).unwrap();
        assert_eq!(entries[0].availability, AchievementAvailability::Available);
        assert_eq!(entries[1].availability, AchievementAvailability::Locked);
        assert_eq!(entries[1].unmet_prerequisite_ids, ["cooking::first_dish"]);

        let related = commands
            .list(QueryAchievements {
                related_record_definition_id: Some("cooking.dish_count".to_string()),
                ..QueryAchievements::default()
            })
            .unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].achievement_id, "cooking::first_dish");
    }

    #[test]
    fn tracked_does_not_unlock_prerequisite_but_achieved_does() {
        let mut repository = repository();
        let mut commands = AchievementCommands::new(&mut repository);
        commands
            .set_state(SetAchievementState {
                achievement_id: "cooking::first_dish".to_string(),
                status: AchievementStatus::Tracked,
                achieved_at: None,
            })
            .unwrap();
        let entries = commands.list(QueryAchievements::default()).unwrap();
        assert_eq!(entries[0].availability, AchievementAvailability::Tracked);
        assert_eq!(entries[1].availability, AchievementAvailability::Locked);

        commands
            .set_state(SetAchievementState {
                achievement_id: "cooking::first_dish".to_string(),
                status: AchievementStatus::Achieved,
                achieved_at: Some("2026-08".to_string()),
            })
            .unwrap();
        let entries = commands.list(QueryAchievements::default()).unwrap();
        assert_eq!(entries[0].availability, AchievementAvailability::Achieved);
        assert_eq!(entries[1].availability, AchievementAvailability::Available);
    }

    #[test]
    fn direct_achievement_ignores_prerequisite_and_revoke_survives_disable() {
        let mut repository = repository();
        let mut commands = AchievementCommands::new(&mut repository);
        let result = commands
            .set_state(SetAchievementState {
                achievement_id: "cooking::host_dinner".to_string(),
                status: AchievementStatus::Achieved,
                achieved_at: None,
            })
            .unwrap();
        assert!(result.changed);
        assert!(
            !commands
                .set_state(SetAchievementState {
                    achievement_id: "cooking::host_dinner".to_string(),
                    status: AchievementStatus::Achieved,
                    achieved_at: None,
                })
                .unwrap()
                .changed
        );

        let mut transaction = repository.begin_transaction().unwrap();
        transaction.set_pack_enabled("cooking", false).unwrap();
        transaction.commit().unwrap();
        let error = AchievementCommands::new(&mut repository)
            .set_state(SetAchievementState {
                achievement_id: "cooking::host_dinner".to_string(),
                status: AchievementStatus::Achieved,
                achieved_at: None,
            })
            .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Unresolved);
        let entries = AchievementCommands::new(&mut repository)
            .list(QueryAchievements::default())
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].availability, AchievementAvailability::Unresolved);

        let mut commands = AchievementCommands::new(&mut repository);
        assert!(
            commands
                .revoke_state("cooking::host_dinner")
                .unwrap()
                .changed
        );
        assert!(
            !commands
                .revoke_state("cooking::host_dinner")
                .unwrap()
                .changed
        );
    }
}
