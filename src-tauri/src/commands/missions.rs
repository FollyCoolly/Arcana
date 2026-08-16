use super::data_platform::DataCommandError;
use crate::application::{
    ArcanaRuntime, DashboardMissionSelectionResult, MissionCommands, MissionDashboardCommands,
    MissionResult, MissionSuggestionResult, QueryMissionSuggestions, QueryMissions,
};
use crate::domain::{
    ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction,
    DashboardMissionSelection, DashboardMissionSelections, DashboardMissionSlot, Mission,
    MissionStatus, MissionSuggestion, MissionSuggestionStatus, RepositoryError,
};
use crate::storage::date_utils::{days_from_civil, parse_date, today_epoch_days};
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::State;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionDashboardItem {
    #[serde(flatten)]
    pub mission: Mission,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionSuggestionDashboardItem {
    #[serde(flatten)]
    pub suggestion: MissionSuggestion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionDashboardData {
    pub missions: Vec<MissionDashboardItem>,
    pub suggestions: Vec<MissionSuggestionDashboardItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CountdownDisplay {
    pub label: String,
    pub short_desc: String,
    pub days_remaining: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HintDisplay {
    pub short_desc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProgressDisplay {
    pub label: String,
    pub progress: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MissionMenuDashboardData {
    pub countdown: Option<CountdownDisplay>,
    pub hints: Vec<HintDisplay>,
    pub progress: Option<ProgressDisplay>,
    pub selections: DashboardMissionSelections,
    pub unresolved_slots: Vec<DashboardMissionSlot>,
}

#[tauri::command]
pub fn load_mission_dashboard(
    runtime: State<'_, ArcanaRuntime>,
) -> Result<MissionDashboardData, DataCommandError> {
    runtime
        .with_repository(build_mission_dashboard)
        .map_err(Into::into)
}

#[tauri::command]
pub fn load_mission_menu_dashboard(
    runtime: State<'_, ArcanaRuntime>,
) -> Result<MissionMenuDashboardData, DataCommandError> {
    runtime
        .with_repository(build_mission_menu_dashboard)
        .map_err(Into::into)
}

#[tauri::command]
pub fn complete_mission(
    runtime: State<'_, ArcanaRuntime>,
    mission_id: String,
) -> Result<MissionResult, DataCommandError> {
    runtime
        .with_repository(|repository| MissionCommands::new(repository).complete(&mission_id))
        .map_err(Into::into)
}

#[tauri::command]
pub fn archive_mission(
    runtime: State<'_, ArcanaRuntime>,
    mission_id: String,
) -> Result<MissionResult, DataCommandError> {
    runtime
        .with_repository(|repository| MissionCommands::new(repository).archive(&mission_id))
        .map_err(Into::into)
}

#[tauri::command]
pub fn accept_mission_suggestion(
    runtime: State<'_, ArcanaRuntime>,
    suggestion_id: String,
) -> Result<MissionResult, DataCommandError> {
    runtime
        .with_repository(|repository| MissionCommands::new(repository).accept(&suggestion_id))
        .map_err(Into::into)
}

#[tauri::command]
pub fn reject_mission_suggestion(
    runtime: State<'_, ArcanaRuntime>,
    suggestion_id: String,
) -> Result<MissionSuggestionResult, DataCommandError> {
    runtime
        .with_repository(|repository| MissionCommands::new(repository).reject(&suggestion_id))
        .map_err(Into::into)
}

#[tauri::command]
pub fn select_mission_dashboard_slot(
    runtime: State<'_, ArcanaRuntime>,
    slot: DashboardMissionSlot,
    mission_id: String,
    label: Option<String>,
) -> Result<DashboardMissionSelectionResult, DataCommandError> {
    runtime
        .with_repository(|repository| {
            MissionDashboardCommands::new(repository).select(slot, mission_id, label)
        })
        .map_err(Into::into)
}

#[tauri::command]
pub fn clear_mission_dashboard_slot(
    runtime: State<'_, ArcanaRuntime>,
    slot: DashboardMissionSlot,
) -> Result<DashboardMissionSelectionResult, DataCommandError> {
    runtime
        .with_repository(|repository| MissionDashboardCommands::new(repository).clear(slot))
        .map_err(Into::into)
}

fn build_mission_dashboard<R>(repository: &mut R) -> Result<MissionDashboardData, RepositoryError>
where
    R: ArcanaRepository,
{
    let missions = MissionCommands::new(repository)
        .list(QueryMissions::default())?
        .into_iter()
        .map(|mission| MissionDashboardItem {
            days_remaining: deadline_days_remaining(mission.deadline.as_deref()),
            mission,
        })
        .collect();
    let suggestions = MissionCommands::new(repository)
        .list_suggestions(QueryMissionSuggestions {
            suggestion_id: None,
            status: Some(MissionSuggestionStatus::Pending),
        })?
        .into_iter()
        .map(|suggestion| MissionSuggestionDashboardItem {
            days_remaining: deadline_days_remaining(suggestion.deadline.as_deref()),
            suggestion,
        })
        .collect();
    Ok(MissionDashboardData {
        missions,
        suggestions,
    })
}

fn build_mission_menu_dashboard<R>(
    repository: &mut R,
) -> Result<MissionMenuDashboardData, RepositoryError>
where
    R: ArcanaRepository,
{
    let transaction = repository.begin_transaction()?;
    let snapshot = transaction.load_synced_snapshot()?;
    let selections = transaction.dashboard_mission_selections()?;
    transaction.rollback()?;
    let missions: BTreeMap<&str, &Mission> = snapshot
        .missions
        .iter()
        .flat_map(|file| file.missions.iter())
        .map(|mission| (mission.id.as_str(), mission))
        .collect();
    let mut unresolved_slots = Vec::new();

    let countdown =
        match selected_active_mission(DashboardMissionSlot::Countdown, &selections, &missions) {
            Ok(Some((selection, mission))) => {
                let days_remaining = deadline_days_remaining(mission.deadline.as_deref());
                match days_remaining.filter(|days| (0..=99).contains(days)) {
                    Some(days_remaining) => Some(CountdownDisplay {
                        label: selection
                            .label
                            .clone()
                            .unwrap_or_else(|| "任务期限".to_string()),
                        short_desc: mission.title.clone(),
                        days_remaining,
                    }),
                    None => {
                        unresolved_slots.push(DashboardMissionSlot::Countdown);
                        None
                    }
                }
            }
            Ok(None) => None,
            Err(()) => {
                unresolved_slots.push(DashboardMissionSlot::Countdown);
                None
            }
        };

    let progress =
        match selected_active_mission(DashboardMissionSlot::Progress, &selections, &missions) {
            Ok(Some((selection, mission))) => Some(ProgressDisplay {
                label: selection
                    .label
                    .clone()
                    .unwrap_or_else(|| mission.title.clone()),
                progress: mission.progress.unwrap_or(0),
            }),
            Ok(None) => None,
            Err(()) => {
                unresolved_slots.push(DashboardMissionSlot::Progress);
                None
            }
        };

    let mut hints = Vec::new();
    for slot in [DashboardMissionSlot::Hint1, DashboardMissionSlot::Hint2] {
        match selected_active_mission(slot, &selections, &missions) {
            Ok(Some((_selection, mission))) => hints.push(HintDisplay {
                short_desc: mission.title.clone(),
            }),
            Ok(None) => {}
            Err(()) => unresolved_slots.push(slot),
        }
    }

    Ok(MissionMenuDashboardData {
        countdown,
        hints,
        progress,
        selections,
        unresolved_slots,
    })
}

fn selected_active_mission<'a>(
    slot: DashboardMissionSlot,
    selections: &'a DashboardMissionSelections,
    missions: &'a BTreeMap<&str, &Mission>,
) -> Result<Option<(&'a DashboardMissionSelection, &'a Mission)>, ()> {
    let Some(selection) = selections.get(&slot) else {
        return Ok(None);
    };
    let mission = missions
        .get(selection.mission_id.as_str())
        .copied()
        .ok_or(())?;
    if mission.status != MissionStatus::Active {
        return Err(());
    }
    Ok(Some((selection, mission)))
}

fn deadline_days_remaining(deadline: Option<&str>) -> Option<i64> {
    let deadline = deadline?;
    let (year, month, day) = parse_date(deadline).ok()?;
    let epoch_base = days_from_civil(1970, 1, 1);
    let deadline_days = days_from_civil(year, month, day) - epoch_base;
    let today_days = today_epoch_days().ok()?;
    Some(deadline_days - today_days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{CreateMission, SuggestMission};
    use crate::domain::{DashboardMissionSelection, MissionDifficulty};

    #[test]
    fn empty_runtime_returns_empty_mission_dashboards() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();

        runtime
            .with_repository(|repository| {
                assert_eq!(
                    build_mission_dashboard(repository)?,
                    MissionDashboardData {
                        missions: Vec::new(),
                        suggestions: Vec::new(),
                    }
                );
                assert_eq!(
                    build_mission_menu_dashboard(repository)?,
                    MissionMenuDashboardData {
                        countdown: None,
                        hints: Vec::new(),
                        progress: None,
                        selections: DashboardMissionSelections::new(),
                        unresolved_slots: Vec::new(),
                    }
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn dashboard_separates_suggestions_and_resolves_local_slots() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut commands = MissionCommands::new(repository);
                commands.create_at(
                    CreateMission {
                        title: "Read Rust Book".to_string(),
                        description: None,
                        progress: Some(40),
                        difficulty: Some(MissionDifficulty::B),
                        deadline: None,
                        parent_id: None,
                    },
                    "019b1234-89ab-7def-8123-456789abcdef".to_string(),
                    "2026-08-16T10:00:00Z".to_string(),
                )?;
                commands.suggest_at(
                    SuggestMission {
                        title: "Try Rustlings".to_string(),
                        description: None,
                        difficulty: Some(MissionDifficulty::C),
                        deadline: None,
                        parent_mission_id: None,
                        reason: Some("Practice ownership".to_string()),
                    },
                    "019b1234-89ab-7def-8123-456789abcdf0".to_string(),
                    "2026-08-16T10:01:00Z".to_string(),
                )?;
                let mut transaction = repository.begin_transaction()?;
                for slot in [
                    DashboardMissionSlot::Progress,
                    DashboardMissionSlot::Hint1,
                    DashboardMissionSlot::Countdown,
                ] {
                    transaction.set_dashboard_mission_selection(
                        slot,
                        DashboardMissionSelection {
                            mission_id: "019b1234-89ab-7def-8123-456789abcdef".to_string(),
                            label: None,
                        },
                    )?;
                }
                transaction.commit()?;
                Ok(())
            })
            .unwrap();

        runtime
            .with_repository(|repository| {
                let dashboard = build_mission_dashboard(repository)?;
                assert_eq!(dashboard.missions.len(), 1);
                assert_eq!(dashboard.suggestions.len(), 1);
                assert_eq!(dashboard.suggestions[0].suggestion.title, "Try Rustlings");

                let menu = build_mission_menu_dashboard(repository)?;
                assert_eq!(menu.progress.as_ref().map(|item| item.progress), Some(40));
                assert_eq!(menu.hints[0].short_desc, "Read Rust Book");
                assert_eq!(menu.unresolved_slots, vec![DashboardMissionSlot::Countdown]);
                Ok(())
            })
            .unwrap();
    }
}
