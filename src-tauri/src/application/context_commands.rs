use super::status_commands::evaluate_dimensions_from_snapshot;
use super::StatusDimensionEvaluation;
use crate::domain::{
    split_scoped_id, AchievementState, ArcanaRepository, ArcanaRepositoryReader,
    ArcanaRepositoryTransaction, AssistantMemory, Mission, MissionStatus, RepositoryResult,
};
use chrono::{Local, NaiveDate};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContextStatusSelection {
    pub position: u8,
    pub dimension_id: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<StatusDimensionEvaluation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextMission {
    #[serde(flatten)]
    pub mission: Mission,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContextAchievementState {
    pub achievement_id: String,
    pub pack_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub definition_available: bool,
    pub state: AchievementState,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContextSummary {
    pub as_of_date: String,
    pub status_selections: Vec<ContextStatusSelection>,
    pub active_missions: Vec<ContextMission>,
    pub achievement_states: Vec<ContextAchievementState>,
    pub memories: Vec<AssistantMemory>,
}

pub struct ContextCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> ContextCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    /// Build the compact Agent bootstrap context from one repository snapshot.
    /// Large Record values and complete Pack definitions remain available via
    /// their targeted queries and are intentionally omitted here.
    pub fn summary(&mut self) -> RepositoryResult<ContextSummary> {
        self.summary_on(Local::now().date_naive())
    }

    fn summary_on(&mut self, as_of_date: NaiveDate) -> RepositoryResult<ContextSummary> {
        let transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let mut selections = transaction.status_dimension_selection()?;
        selections.sort_by_key(|selection| selection.position);

        let evaluations = evaluate_dimensions_from_snapshot(&snapshot, &selections, None)?;
        let mut evaluations_by_id: BTreeMap<String, StatusDimensionEvaluation> = evaluations
            .into_iter()
            .map(|evaluation| (evaluation.dimension_id.clone(), evaluation))
            .collect();
        let status_selections = selections
            .into_iter()
            .map(|selection| {
                let evaluation = evaluations_by_id.remove(&selection.dimension_id);
                ContextStatusSelection {
                    position: selection.position,
                    dimension_id: selection.dimension_id,
                    available: evaluation.is_some(),
                    evaluation,
                }
            })
            .collect();

        let mut active_missions = snapshot
            .missions
            .iter()
            .flat_map(|file| file.missions.iter())
            .filter(|mission| mission.status == MissionStatus::Active)
            .cloned()
            .map(|mission| ContextMission {
                days_remaining: mission
                    .deadline
                    .as_deref()
                    .and_then(|deadline| NaiveDate::parse_from_str(deadline, "%Y-%m-%d").ok())
                    .map(|deadline| (deadline - as_of_date).num_days()),
                mission,
            })
            .collect::<Vec<_>>();
        active_missions.sort_by(|left, right| {
            left.mission
                .deadline
                .is_none()
                .cmp(&right.mission.deadline.is_none())
                .then_with(|| left.mission.deadline.cmp(&right.mission.deadline))
                .then_with(|| left.mission.id.cmp(&right.mission.id))
        });

        let enabled_pack_ids: BTreeSet<&str> = snapshot
            .manifest
            .enabled_pack_ids
            .iter()
            .map(String::as_str)
            .collect();
        let definitions = snapshot
            .packs
            .iter()
            .flat_map(|(pack_id, pack)| {
                pack.achievements
                    .iter()
                    .flat_map(|file| file.achievements.iter())
                    .map(move |definition| {
                        (
                            definition.id.as_str(),
                            (pack_id.as_str(), definition.name.as_str()),
                        )
                    })
            })
            .collect::<BTreeMap<_, _>>();
        let achievement_states = snapshot
            .achievement_states
            .iter()
            .flat_map(|file| file.states.iter())
            .map(|(achievement_id, state)| {
                let definition = definitions.get(achievement_id.as_str()).copied();
                let pack_id = definition
                    .map(|(pack_id, _)| pack_id)
                    .or_else(|| split_scoped_id(achievement_id).map(|(pack_id, _)| pack_id))
                    .unwrap_or_default();
                ContextAchievementState {
                    achievement_id: achievement_id.clone(),
                    pack_id: pack_id.to_string(),
                    name: definition.map(|(_, name)| name.to_string()),
                    definition_available: definition.is_some()
                        && enabled_pack_ids.contains(pack_id),
                    state: state.clone(),
                }
            })
            .collect();

        let memories = snapshot
            .assistant_memory
            .iter()
            .flat_map(|file| file.memories.iter().cloned())
            .collect();
        let summary = ContextSummary {
            as_of_date: as_of_date.format("%Y-%m-%d").to_string(),
            status_selections,
            active_missions,
            achievement_states,
            memories,
        };
        transaction.rollback()?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ArcanaRuntime;

    #[test]
    fn empty_runtime_has_an_empty_context_on_the_requested_date() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();

        let summary = runtime
            .with_repository(|repository| {
                ContextCommands::new(repository)
                    .summary_on(NaiveDate::from_ymd_opt(2026, 8, 16).unwrap())
            })
            .unwrap();

        assert_eq!(summary.as_of_date, "2026-08-16");
        assert!(summary.status_selections.is_empty());
        assert!(summary.active_missions.is_empty());
        assert!(summary.achievement_states.is_empty());
        assert!(summary.memories.is_empty());
    }
}
