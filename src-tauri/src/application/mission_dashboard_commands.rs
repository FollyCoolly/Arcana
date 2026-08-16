use crate::domain::{
    ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction,
    DashboardMissionSelection, DashboardMissionSelections, DashboardMissionSlot, MissionStatus,
    RepositoryError, RepositoryErrorCode, RepositoryResult,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DashboardMissionSelectionResult {
    pub slot: DashboardMissionSlot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection: Option<DashboardMissionSelection>,
    pub changed: bool,
}

pub struct MissionDashboardCommands<'repository, R> {
    repository: &'repository mut R,
}

impl<'repository, R> MissionDashboardCommands<'repository, R>
where
    R: ArcanaRepository,
{
    pub fn new(repository: &'repository mut R) -> Self {
        Self { repository }
    }

    pub fn list(&self) -> RepositoryResult<DashboardMissionSelections> {
        self.repository.dashboard_mission_selections()
    }

    pub fn select(
        &mut self,
        slot: DashboardMissionSlot,
        mission_id: String,
        label: Option<String>,
    ) -> RepositoryResult<DashboardMissionSelectionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        let mission = snapshot
            .missions
            .iter()
            .flat_map(|file| file.missions.iter())
            .find(|mission| mission.id == mission_id)
            .ok_or_else(|| {
                RepositoryError::new(
                    RepositoryErrorCode::NotFound,
                    format!("Mission '{mission_id}' was not found"),
                )
            })?;
        if mission.status != MissionStatus::Active {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("only an active Mission can be selected for Dashboard slot {slot:?}"),
            ));
        }
        let selection = DashboardMissionSelection { mission_id, label };
        let current = transaction.dashboard_mission_selections()?;
        if current.get(&slot) == Some(&selection) {
            transaction.rollback()?;
            return Ok(DashboardMissionSelectionResult {
                slot,
                selection: Some(selection),
                changed: false,
            });
        }
        transaction.set_dashboard_mission_selection(slot, selection.clone())?;
        transaction.commit()?;
        Ok(DashboardMissionSelectionResult {
            slot,
            selection: Some(selection),
            changed: true,
        })
    }

    pub fn clear(
        &mut self,
        slot: DashboardMissionSlot,
    ) -> RepositoryResult<DashboardMissionSelectionResult> {
        let mut transaction = self.repository.begin_transaction()?;
        if !transaction
            .dashboard_mission_selections()?
            .contains_key(&slot)
        {
            transaction.rollback()?;
            return Ok(DashboardMissionSelectionResult {
                slot,
                selection: None,
                changed: false,
            });
        }
        transaction.clear_dashboard_mission_selection(slot)?;
        transaction.commit()?;
        Ok(DashboardMissionSelectionResult {
            slot,
            selection: None,
            changed: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{ArcanaRuntime, CreateMission, MissionCommands};

    #[test]
    fn selection_requires_active_mission_and_clear_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path()).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let created = MissionCommands::new(repository).create_at(
                    CreateMission {
                        title: "Ship Arcana".to_string(),
                        description: None,
                        progress: Some(50),
                        difficulty: None,
                        deadline: None,
                        parent_id: None,
                    },
                    "019b1234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-16T10:00:00Z".to_string(),
                )?;
                {
                    let mut commands = MissionDashboardCommands::new(repository);
                    assert!(
                        commands
                            .select(
                                DashboardMissionSlot::Progress,
                                created.mission.id.clone(),
                                Some("Arcana".to_string()),
                            )?
                            .changed
                    );
                    assert!(
                        !commands
                            .select(
                                DashboardMissionSlot::Progress,
                                created.mission.id.clone(),
                                Some("Arcana".to_string()),
                            )?
                            .changed
                    );
                    assert_eq!(commands.list()?.len(), 1);

                    assert!(commands.clear(DashboardMissionSlot::Progress)?.changed);
                    assert!(!commands.clear(DashboardMissionSlot::Progress)?.changed);
                }

                MissionCommands::new(repository)
                    .complete_at(&created.mission.id, "2026-08-16T11:00:00Z".to_string())?;
                let error = MissionDashboardCommands::new(repository)
                    .select(DashboardMissionSlot::Hint1, created.mission.id, None)
                    .unwrap_err();
                assert_eq!(error.code, RepositoryErrorCode::Conflict);
                Ok(())
            })
            .unwrap();
    }
}
