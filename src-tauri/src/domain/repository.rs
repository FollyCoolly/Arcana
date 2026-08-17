use super::{
    AchievementState, AchievementStateFile, ArcanaManifest, AssistantMemory, AssistantMemoryFile,
    DashboardMissionSelection, DashboardMissionSelections, DefinitionRegistry,
    DerivedValueRegistry, Mission, MissionFile, MissionSuggestion, Pack, Record, RecordFile,
    StatusDimensionSelection, Validate, ValidationErrors, ValidationIssue, ValidationResult,
    Validator,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct SyncedRepositorySnapshot {
    pub manifest: ArcanaManifest,
    pub packs: BTreeMap<String, Pack>,
    pub records: BTreeMap<String, RecordFile>,
    pub achievement_states: Option<AchievementStateFile>,
    pub missions: Option<MissionFile>,
    pub assistant_memory: Option<AssistantMemoryFile>,
}

impl SyncedRepositorySnapshot {
    pub fn definition_registry(&self) -> Result<DefinitionRegistry, ValidationErrors> {
        DefinitionRegistry::build(
            self.manifest
                .enabled_pack_ids
                .iter()
                .filter_map(|id| self.packs.get(id)),
        )
    }

    pub fn derived_value_registry(&self) -> Result<DerivedValueRegistry, ValidationErrors> {
        DerivedValueRegistry::build(
            self.manifest
                .enabled_pack_ids
                .iter()
                .filter_map(|id| self.packs.get(id)),
        )
    }

    pub fn unresolved_record_ids(&self) -> BTreeSet<&str> {
        let Ok(registry) = self.definition_registry() else {
            return self
                .records
                .values()
                .flat_map(|file| file.records.iter())
                .map(Record::definition_id)
                .collect();
        };
        self.records
            .values()
            .flat_map(|file| file.records.iter())
            .map(Record::definition_id)
            .filter(|id| registry.get(id).is_none())
            .collect()
    }

    pub fn unresolved_achievement_state_ids(&self) -> BTreeSet<&str> {
        let known: BTreeSet<&str> = self
            .packs
            .values()
            .flat_map(|pack| pack.achievements.iter())
            .flat_map(|file| file.achievements.iter())
            .map(|achievement| achievement.id.as_str())
            .collect();
        self.achievement_states
            .iter()
            .flat_map(|file| file.states.keys())
            .map(String::as_str)
            .filter(|id| !known.contains(id))
            .collect()
    }

    pub fn missing_parent_pack_ids(&self) -> BTreeSet<&str> {
        self.packs
            .values()
            .filter_map(|pack| pack.manifest.parent_pack_id.as_deref())
            .filter(|parent_id| !self.packs.contains_key(*parent_id))
            .collect()
    }
}

impl Validate for SyncedRepositorySnapshot {
    fn validate(&self) -> ValidationResult {
        let mut validator = Validator::default();
        validator.merge("arcana.json", self.manifest.validate());

        for (id, pack) in &self.packs {
            validator.require(
                id == &pack.manifest.id,
                "pack_directory_mismatch",
                &format!("packs.{id}.manifest.id"),
                "Pack map/directory id must equal manifest.id",
            );
            validator.merge(&format!("packs.{id}"), pack.validate());
        }
        for (index, id) in self.manifest.enabled_pack_ids.iter().enumerate() {
            validator.require(
                self.packs.contains_key(id),
                "enabled_pack_missing",
                &format!("arcana.json.enabled_pack_ids[{index}]"),
                "enabled Pack must exist in the repository",
            );
        }
        validate_pack_forest(&mut validator, &self.packs);

        let registry = match self.definition_registry() {
            Ok(registry) => Some(registry),
            Err(errors) => {
                validator.merge("definition_registry", Err(errors));
                None
            }
        };
        if let Err(errors) = self.derived_value_registry() {
            validator.merge("derived_value_registry", Err(errors));
        }
        for (namespace, file) in &self.records {
            validator.require(
                namespace == &file.namespace,
                "record_file_namespace_mismatch",
                &format!("records.{namespace}.namespace"),
                "Record map/file name must equal the file namespace",
            );
            validator.merge(&format!("records.{namespace}"), file.validate());
            if let Some(registry) = &registry {
                for (index, record) in file.records.iter().enumerate() {
                    if let Some(definition) = registry.get(record.definition_id()) {
                        validator.merge(
                            &format!("records.{namespace}.records[{index}]"),
                            record.validate_against(definition),
                        );
                    }
                }
            }
        }
        if let Some(states) = &self.achievement_states {
            validator.merge("achievement-states.json", states.validate());
        }
        if let Some(missions) = &self.missions {
            validator.merge("missions.json", missions.validate());
        }
        if let Some(memory) = &self.assistant_memory {
            validator.merge("assistant-memory.json", memory.validate());
        }
        validator.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryErrorCode {
    NotFound,
    Conflict,
    Unresolved,
    ValidationFailed,
    Busy,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError {
    pub code: RepositoryErrorCode,
    pub message: String,
    pub validation_issues: Vec<ValidationIssue>,
}

impl RepositoryError {
    pub fn new(code: RepositoryErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            validation_issues: Vec::new(),
        }
    }

    pub fn validation(errors: ValidationErrors) -> Self {
        Self {
            code: RepositoryErrorCode::ValidationFailed,
            message: "domain validation failed".to_string(),
            validation_issues: errors.into_issues(),
        }
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for RepositoryError {}

impl From<ValidationErrors> for RepositoryError {
    fn from(value: ValidationErrors) -> Self {
        Self::validation(value)
    }
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Read side shared by the UI, CLI, sync codec, and Agent Skills.
pub trait ArcanaRepositoryReader {
    fn load_synced_snapshot(&self) -> RepositoryResult<SyncedRepositorySnapshot>;
    fn get_record(&self, definition_id: &str) -> RepositoryResult<Option<Record>>;
    fn list_mission_suggestions(&self) -> RepositoryResult<Vec<MissionSuggestion>>;
    fn status_dimension_selection(&self) -> RepositoryResult<Vec<StatusDimensionSelection>>;
    fn dashboard_mission_selections(&self) -> RepositoryResult<DashboardMissionSelections>;
}

/// Read/write surface available only inside a repository transaction.
/// Read-modify-write commands must use these read methods after the
/// transaction begins, never read through the outer repository first.
pub trait ArcanaRepositoryTransaction: ArcanaRepositoryReader {
    fn put_pack(&mut self, pack: Pack) -> RepositoryResult<()>;
    fn delete_pack(&mut self, pack_id: &str) -> RepositoryResult<()>;
    fn set_pack_enabled(&mut self, pack_id: &str, enabled: bool) -> RepositoryResult<()>;

    fn put_record(&mut self, record: Record) -> RepositoryResult<()>;
    fn delete_record(&mut self, definition_id: &str) -> RepositoryResult<()>;

    fn set_achievement_state(
        &mut self,
        achievement_id: &str,
        state: AchievementState,
    ) -> RepositoryResult<()>;
    fn revoke_achievement_state(&mut self, achievement_id: &str) -> RepositoryResult<()>;

    fn put_mission(&mut self, mission: Mission) -> RepositoryResult<()>;
    fn delete_mission(&mut self, mission_id: &str) -> RepositoryResult<()>;
    fn put_mission_suggestion(&mut self, suggestion: MissionSuggestion) -> RepositoryResult<()>;
    fn delete_mission_suggestion(&mut self, suggestion_id: &str) -> RepositoryResult<()>;

    fn put_assistant_memory(&mut self, memory: AssistantMemory) -> RepositoryResult<()>;
    fn delete_assistant_memory(&mut self, memory_id: &str) -> RepositoryResult<()>;

    fn set_status_dimension_selection(
        &mut self,
        selection: StatusDimensionSelection,
    ) -> RepositoryResult<()>;
    fn clear_status_dimension_selection(&mut self, position: u8) -> RepositoryResult<()>;
    fn set_dashboard_mission_selection(
        &mut self,
        slot: super::DashboardMissionSlot,
        selection: DashboardMissionSelection,
    ) -> RepositoryResult<()>;
    fn clear_dashboard_mission_selection(
        &mut self,
        slot: super::DashboardMissionSlot,
    ) -> RepositoryResult<()>;

    fn replace_synced_snapshot(
        &mut self,
        snapshot: SyncedRepositorySnapshot,
    ) -> RepositoryResult<()>;
    fn commit(self) -> RepositoryResult<()>;
    fn rollback(self) -> RepositoryResult<()>;
}

/// Unit-of-work boundary. A concrete adapter defines its atomicity boundary.
/// The composite runtime guarantees atomic multi-operation commits only for
/// Record-only SQLite transactions and rejects ordinary cross-store batches.
pub trait ArcanaRepository: ArcanaRepositoryReader {
    type Transaction<'a>: ArcanaRepositoryTransaction
    where
        Self: 'a;

    fn begin_transaction(&mut self) -> RepositoryResult<Self::Transaction<'_>>;
}

fn validate_pack_forest(validator: &mut Validator, packs: &BTreeMap<String, Pack>) {
    for pack in packs.values() {
        let mut path = BTreeSet::new();
        let mut current = Some(pack.manifest.id.as_str());
        while let Some(id) = current {
            if !path.insert(id) {
                validator.error(
                    "pack_parent_cycle",
                    "packs",
                    format!("PackForest contains a cycle through '{id}'"),
                );
                break;
            }
            current = packs
                .get(id)
                .and_then(|pack| pack.manifest.parent_pack_id.as_deref())
                .filter(|parent_id| packs.contains_key(*parent_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PackManifest, SCHEMA_VERSION};

    fn empty_pack(id: &str, parent: Option<&str>) -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                author: None,
                parent_pack_id: parent.map(str::to_string),
                tags: vec![],
            },
            record_definitions: None,
            derived_values: None,
            dimensions: None,
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    #[test]
    fn missing_pack_parent_is_warning_not_validation_error() {
        let snapshot = SyncedRepositorySnapshot {
            manifest: ArcanaManifest {
                schema_version: SCHEMA_VERSION,
                enabled_pack_ids: vec!["child".to_string()],
            },
            packs: BTreeMap::from([("child".to_string(), empty_pack("child", Some("missing")))]),
            records: BTreeMap::new(),
            achievement_states: None,
            missions: None,
            assistant_memory: None,
        };
        assert!(snapshot.validate().is_ok());
        assert_eq!(
            snapshot.missing_parent_pack_ids(),
            BTreeSet::from(["missing"])
        );
    }

    #[test]
    fn pack_parent_cycle_is_rejected() {
        let snapshot = SyncedRepositorySnapshot {
            manifest: ArcanaManifest {
                schema_version: SCHEMA_VERSION,
                enabled_pack_ids: vec![],
            },
            packs: BTreeMap::from([
                ("a".to_string(), empty_pack("a", Some("b"))),
                ("b".to_string(), empty_pack("b", Some("a"))),
            ]),
            records: BTreeMap::new(),
            achievement_states: None,
            missions: None,
            assistant_memory: None,
        };
        assert!(snapshot.validate().is_err());
    }
}
