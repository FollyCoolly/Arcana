use crate::domain::{
    split_record_definition_id, AchievementState, AchievementStateFile, ArcanaManifest,
    ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction, AssistantMemory,
    AssistantMemoryFile, DashboardMissionSelection, DashboardMissionSelections,
    DashboardMissionSlot, Mission, MissionFile, MissionSuggestion, Pack, Record, RecordFile,
    RepositoryError, RepositoryErrorCode, RepositoryResult, StatusDimensionSelection,
    SyncedRepositorySnapshot, Validate, SCHEMA_VERSION,
};
use crate::storage::json_repository::JsonRepositoryCodec;
use crate::storage::local_state::{read_local_state, write_local_state, LocalState};
use crate::storage::sqlite::{RecordRepository, RecordRepositoryTransaction};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct DataRepository {
    records: RecordRepository,
    semantic: SemanticStore,
    local: LocalStore,
}

enum SemanticStore {
    Directory(PathBuf),
    Memory(SyncedRepositorySnapshot),
}

enum LocalStore {
    File(PathBuf),
    Memory(LocalState),
}

impl DataRepository {
    pub fn open(
        database_path: impl AsRef<Path>,
        repository_dir: impl Into<PathBuf>,
        local_state_path: impl Into<PathBuf>,
    ) -> RepositoryResult<Self> {
        let repository_dir = repository_dir.into();
        JsonRepositoryCodec::read_semantic_directory(&repository_dir)?;
        let local_state_path = local_state_path.into();
        read_local_state(&local_state_path)?;
        Ok(Self {
            records: RecordRepository::open(database_path)?,
            semantic: SemanticStore::Directory(repository_dir),
            local: LocalStore::File(local_state_path),
        })
    }

    pub fn open_in_memory() -> RepositoryResult<Self> {
        Ok(Self {
            records: RecordRepository::open_in_memory()?,
            semantic: SemanticStore::Memory(empty_semantic_snapshot()),
            local: LocalStore::Memory(LocalState::default()),
        })
    }

    pub fn checkpoint_and_close(self) -> RepositoryResult<()> {
        self.records.checkpoint_and_close()
    }

    fn load_snapshot(&self) -> RepositoryResult<SyncedRepositorySnapshot> {
        let mut snapshot = self.semantic.load()?;
        snapshot.records = self.records.load_records()?;
        snapshot.validate().map_err(RepositoryError::validation)?;
        Ok(snapshot)
    }
}

impl SemanticStore {
    fn load(&self) -> RepositoryResult<SyncedRepositorySnapshot> {
        match self {
            Self::Directory(path) => JsonRepositoryCodec::read_semantic_directory(path),
            Self::Memory(snapshot) => Ok(snapshot.clone()),
        }
    }

    fn save(&mut self, snapshot: SyncedRepositorySnapshot) -> RepositoryResult<()> {
        let snapshot = semantic_only(snapshot);
        match self {
            Self::Directory(path) => {
                JsonRepositoryCodec::update_semantic_directory(path, snapshot)?;
            }
            Self::Memory(stored) => *stored = snapshot,
        }
        Ok(())
    }
}

impl LocalStore {
    fn load(&self) -> RepositoryResult<LocalState> {
        match self {
            Self::File(path) => read_local_state(path),
            Self::Memory(state) => Ok(state.clone()),
        }
    }

    fn save(&mut self, state: &LocalState) -> RepositoryResult<()> {
        match self {
            Self::File(path) => write_local_state(path, state)?,
            Self::Memory(stored) => *stored = state.clone(),
        }
        Ok(())
    }
}

impl ArcanaRepositoryReader for DataRepository {
    fn load_synced_snapshot(&self) -> RepositoryResult<SyncedRepositorySnapshot> {
        self.load_snapshot()
    }

    fn get_record(&self, definition_id: &str) -> RepositoryResult<Option<Record>> {
        self.records.get_record(definition_id)
    }

    fn list_mission_suggestions(&self) -> RepositoryResult<Vec<MissionSuggestion>> {
        Ok(self.local.load()?.mission_suggestions)
    }

    fn status_dimension_selection(&self) -> RepositoryResult<Vec<StatusDimensionSelection>> {
        Ok(self.local.load()?.status_dimension_selection)
    }

    fn dashboard_mission_selections(&self) -> RepositoryResult<DashboardMissionSelections> {
        Ok(self.local.load()?.dashboard_mission_selections)
    }
}

impl ArcanaRepository for DataRepository {
    type Transaction<'a>
        = DataRepositoryTransaction<'a>
    where
        Self: 'a;

    fn begin_transaction(&mut self) -> RepositoryResult<Self::Transaction<'_>> {
        let snapshot = self.load_snapshot()?;
        let local_state = self.local.load()?;
        let original_semantic = semantic_only(snapshot.clone());
        let original_local = local_state.clone();
        let record_transaction = self.records.begin_transaction()?;
        Ok(DataRepositoryTransaction {
            record_transaction: Some(record_transaction),
            semantic_store: &mut self.semantic,
            local_store: &mut self.local,
            snapshot,
            original_semantic,
            local_state,
            original_local,
            allow_mixed_commit: false,
        })
    }
}

pub struct DataRepositoryTransaction<'repository> {
    record_transaction: Option<RecordRepositoryTransaction<'repository>>,
    semantic_store: &'repository mut SemanticStore,
    local_store: &'repository mut LocalStore,
    snapshot: SyncedRepositorySnapshot,
    original_semantic: SyncedRepositorySnapshot,
    local_state: LocalState,
    original_local: LocalState,
    allow_mixed_commit: bool,
}

impl<'repository> DataRepositoryTransaction<'repository> {
    fn records(&self) -> &RecordRepositoryTransaction<'repository> {
        self.record_transaction
            .as_ref()
            .expect("Data Repository transaction is active")
    }

    fn records_mut(&mut self) -> &mut RecordRepositoryTransaction<'repository> {
        self.record_transaction
            .as_mut()
            .expect("Data Repository transaction is active")
    }
}

impl ArcanaRepositoryReader for DataRepositoryTransaction<'_> {
    fn load_synced_snapshot(&self) -> RepositoryResult<SyncedRepositorySnapshot> {
        Ok(self.snapshot.clone())
    }

    fn get_record(&self, definition_id: &str) -> RepositoryResult<Option<Record>> {
        self.records().get_record(definition_id)
    }

    fn list_mission_suggestions(&self) -> RepositoryResult<Vec<MissionSuggestion>> {
        Ok(self.local_state.mission_suggestions.clone())
    }

    fn status_dimension_selection(&self) -> RepositoryResult<Vec<StatusDimensionSelection>> {
        Ok(self.local_state.status_dimension_selection.clone())
    }

    fn dashboard_mission_selections(&self) -> RepositoryResult<DashboardMissionSelections> {
        Ok(self.local_state.dashboard_mission_selections.clone())
    }
}

impl ArcanaRepositoryTransaction for DataRepositoryTransaction<'_> {
    fn put_pack(&mut self, pack: Pack) -> RepositoryResult<()> {
        pack.validate().map_err(RepositoryError::validation)?;
        self.snapshot.packs.insert(pack.manifest.id.clone(), pack);
        Ok(())
    }

    fn delete_pack(&mut self, pack_id: &str) -> RepositoryResult<()> {
        if self.snapshot.packs.remove(pack_id).is_none() {
            return Err(not_found("Pack", pack_id));
        }
        if let Ok(index) = self
            .snapshot
            .manifest
            .enabled_pack_ids
            .binary_search_by(|id| id.as_str().cmp(pack_id))
        {
            self.snapshot.manifest.enabled_pack_ids.remove(index);
        }
        Ok(())
    }

    fn set_pack_enabled(&mut self, pack_id: &str, enabled: bool) -> RepositoryResult<()> {
        if !self.snapshot.packs.contains_key(pack_id) {
            return Err(not_found("Pack", pack_id));
        }
        let position = self
            .snapshot
            .manifest
            .enabled_pack_ids
            .binary_search_by(|id| id.as_str().cmp(pack_id));
        match (enabled, position) {
            (true, Err(index)) => self
                .snapshot
                .manifest
                .enabled_pack_ids
                .insert(index, pack_id.to_string()),
            (false, Ok(index)) => {
                self.snapshot.manifest.enabled_pack_ids.remove(index);
            }
            _ => {}
        }
        Ok(())
    }

    fn put_record(&mut self, record: Record) -> RepositoryResult<()> {
        record.validate().map_err(RepositoryError::validation)?;
        let registry = self
            .snapshot
            .definition_registry()
            .map_err(RepositoryError::validation)?;
        let definition = registry.get(record.definition_id()).ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!(
                    "RecordDefinition '{}' is not supplied by an enabled Pack",
                    record.definition_id()
                ),
            )
        })?;
        record
            .validate_against(definition)
            .map_err(RepositoryError::validation)?;
        self.records_mut().put_record(&record)?;
        upsert_record(&mut self.snapshot.records, record)?;
        Ok(())
    }

    fn delete_record(&mut self, definition_id: &str) -> RepositoryResult<()> {
        self.records_mut().delete_record(definition_id)?;
        remove_record(&mut self.snapshot.records, definition_id);
        Ok(())
    }

    fn set_achievement_state(
        &mut self,
        achievement_id: &str,
        state: AchievementState,
    ) -> RepositoryResult<()> {
        state.validate().map_err(RepositoryError::validation)?;
        let available = self
            .snapshot
            .manifest
            .enabled_pack_ids
            .iter()
            .filter_map(|id| self.snapshot.packs.get(id))
            .flat_map(|pack| pack.achievements.iter())
            .flat_map(|file| file.achievements.iter())
            .any(|achievement| achievement.id == achievement_id);
        if !available {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!(
                    "AchievementDefinition '{achievement_id}' is not supplied by an enabled Pack"
                ),
            ));
        }
        self.snapshot
            .achievement_states
            .get_or_insert_with(|| AchievementStateFile {
                states: BTreeMap::new(),
            })
            .states
            .insert(achievement_id.to_string(), state);
        Ok(())
    }

    fn revoke_achievement_state(&mut self, achievement_id: &str) -> RepositoryResult<()> {
        let Some(states) = &mut self.snapshot.achievement_states else {
            return Err(not_found("Achievement state", achievement_id));
        };
        if states.states.remove(achievement_id).is_none() {
            return Err(not_found("Achievement state", achievement_id));
        }
        if states.states.is_empty() {
            self.snapshot.achievement_states = None;
        }
        Ok(())
    }

    fn put_mission(&mut self, mission: Mission) -> RepositoryResult<()> {
        mission.validate().map_err(RepositoryError::validation)?;
        let missions = &mut self
            .snapshot
            .missions
            .get_or_insert_with(|| MissionFile {
                missions: Vec::new(),
            })
            .missions;
        match missions.binary_search_by(|candidate| candidate.id.cmp(&mission.id)) {
            Ok(index) => missions[index] = mission,
            Err(index) => missions.insert(index, mission),
        }
        Ok(())
    }

    fn delete_mission(&mut self, mission_id: &str) -> RepositoryResult<()> {
        let Some(file) = &mut self.snapshot.missions else {
            return Err(not_found("Mission", mission_id));
        };
        if file
            .missions
            .iter()
            .any(|mission| mission.parent_id.as_deref() == Some(mission_id))
        {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("Mission '{mission_id}' is still referenced by a child Mission"),
            ));
        }
        let index = file
            .missions
            .binary_search_by(|mission| mission.id.as_str().cmp(mission_id))
            .map_err(|_| not_found("Mission", mission_id))?;
        file.missions.remove(index);
        if file.missions.is_empty() {
            self.snapshot.missions = None;
        }
        Ok(())
    }

    fn put_mission_suggestion(&mut self, suggestion: MissionSuggestion) -> RepositoryResult<()> {
        suggestion.validate().map_err(RepositoryError::validation)?;
        if mission_exists(&self.snapshot, &suggestion.id) {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("Mission '{}' already exists", suggestion.id),
            ));
        }
        if let Some(parent_id) = &suggestion.parent_mission_id {
            if !mission_exists(&self.snapshot, parent_id) {
                return Err(RepositoryError::new(
                    RepositoryErrorCode::Unresolved,
                    format!("parent Mission '{parent_id}' does not exist"),
                ));
            }
        }
        match self
            .local_state
            .mission_suggestions
            .binary_search_by(|candidate| candidate.id.cmp(&suggestion.id))
        {
            Ok(index) => self.local_state.mission_suggestions[index] = suggestion,
            Err(index) => self
                .local_state
                .mission_suggestions
                .insert(index, suggestion),
        }
        Ok(())
    }

    fn delete_mission_suggestion(&mut self, suggestion_id: &str) -> RepositoryResult<()> {
        let index = self
            .local_state
            .mission_suggestions
            .binary_search_by(|suggestion| suggestion.id.as_str().cmp(suggestion_id))
            .map_err(|_| not_found("MissionSuggestion", suggestion_id))?;
        self.local_state.mission_suggestions.remove(index);
        Ok(())
    }

    fn put_assistant_memory(&mut self, memory: AssistantMemory) -> RepositoryResult<()> {
        memory.validate().map_err(RepositoryError::validation)?;
        let memories = &mut self
            .snapshot
            .assistant_memory
            .get_or_insert_with(|| AssistantMemoryFile {
                memories: Vec::new(),
            })
            .memories;
        match memories.binary_search_by(|candidate| candidate.id.cmp(&memory.id)) {
            Ok(index) => memories[index] = memory,
            Err(index) => memories.insert(index, memory),
        }
        Ok(())
    }

    fn delete_assistant_memory(&mut self, memory_id: &str) -> RepositoryResult<()> {
        let Some(file) = &mut self.snapshot.assistant_memory else {
            return Err(not_found("AssistantMemory", memory_id));
        };
        let index = file
            .memories
            .binary_search_by(|memory| memory.id.as_str().cmp(memory_id))
            .map_err(|_| not_found("AssistantMemory", memory_id))?;
        file.memories.remove(index);
        if file.memories.is_empty() {
            self.snapshot.assistant_memory = None;
        }
        Ok(())
    }

    fn set_status_dimension_selection(
        &mut self,
        selection: StatusDimensionSelection,
    ) -> RepositoryResult<()> {
        selection.validate().map_err(RepositoryError::validation)?;
        let available = self
            .snapshot
            .manifest
            .enabled_pack_ids
            .iter()
            .filter_map(|id| self.snapshot.packs.get(id))
            .flat_map(|pack| pack.dimensions.iter())
            .flat_map(|file| file.dimensions.iter())
            .any(|dimension| dimension.id == selection.dimension_id);
        if !available {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!(
                    "Status Dimension '{}' is not supplied by an enabled Pack",
                    selection.dimension_id
                ),
            ));
        }
        self.local_state
            .status_dimension_selection
            .retain(|current| {
                current.position != selection.position
                    && current.dimension_id != selection.dimension_id
            });
        self.local_state.status_dimension_selection.push(selection);
        self.local_state
            .status_dimension_selection
            .sort_by_key(|current| current.position);
        Ok(())
    }

    fn clear_status_dimension_selection(&mut self, position: u8) -> RepositoryResult<()> {
        if position >= 5 {
            return Err(RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                "Status Dimension position must be between 0 and 4",
            ));
        }
        self.local_state
            .status_dimension_selection
            .retain(|selection| selection.position != position);
        Ok(())
    }

    fn set_dashboard_mission_selection(
        &mut self,
        slot: DashboardMissionSlot,
        selection: DashboardMissionSelection,
    ) -> RepositoryResult<()> {
        selection.validate().map_err(RepositoryError::validation)?;
        if !mission_exists(&self.snapshot, &selection.mission_id) {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!("Mission '{}' does not exist", selection.mission_id),
            ));
        }
        self.local_state
            .dashboard_mission_selections
            .insert(slot, selection);
        Ok(())
    }

    fn clear_dashboard_mission_selection(
        &mut self,
        slot: DashboardMissionSlot,
    ) -> RepositoryResult<()> {
        self.local_state.dashboard_mission_selections.remove(&slot);
        Ok(())
    }

    fn replace_synced_snapshot(
        &mut self,
        snapshot: SyncedRepositorySnapshot,
    ) -> RepositoryResult<()> {
        snapshot.validate().map_err(RepositoryError::validation)?;
        self.records_mut().replace_records(&snapshot.records)?;
        self.local_state.mission_suggestions.retain(|suggestion| {
            !snapshot
                .missions
                .iter()
                .flat_map(|file| file.missions.iter())
                .any(|mission| mission.id == suggestion.id)
        });
        self.snapshot = snapshot;
        self.allow_mixed_commit = true;
        Ok(())
    }

    fn commit(mut self) -> RepositoryResult<()> {
        self.snapshot
            .validate()
            .map_err(RepositoryError::validation)?;
        self.local_state.normalize();
        self.local_state.validate()?;

        let semantic = semantic_only(self.snapshot.clone());
        let semantic_changed = semantic != self.original_semantic;
        let local_changed = self.local_state != self.original_local;
        let record_changed = self.records().has_changes();
        if record_changed && (semantic_changed || local_changed) && !self.allow_mixed_commit {
            self.record_transaction
                .take()
                .expect("Data Repository transaction is active")
                .rollback()?;
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                "a batch cannot mix Record SQLite mutations with JSON or local-state mutations",
            ));
        }

        if semantic_changed {
            self.semantic_store.save(semantic)?;
        }
        if local_changed {
            self.local_store.save(&self.local_state)?;
        }
        self.record_transaction
            .take()
            .expect("Data Repository transaction is active")
            .commit()
    }

    fn rollback(mut self) -> RepositoryResult<()> {
        self.record_transaction
            .take()
            .expect("Data Repository transaction is active")
            .rollback()
    }
}

fn empty_semantic_snapshot() -> SyncedRepositorySnapshot {
    SyncedRepositorySnapshot {
        manifest: ArcanaManifest {
            schema_version: SCHEMA_VERSION,
            enabled_pack_ids: Vec::new(),
        },
        packs: BTreeMap::new(),
        records: BTreeMap::new(),
        achievement_states: None,
        missions: None,
        assistant_memory: None,
    }
}

fn semantic_only(mut snapshot: SyncedRepositorySnapshot) -> SyncedRepositorySnapshot {
    snapshot.records.clear();
    snapshot
}

fn upsert_record(files: &mut BTreeMap<String, RecordFile>, record: Record) -> RepositoryResult<()> {
    let definition_id = record.definition_id().to_string();
    let namespace = split_record_definition_id(&definition_id)
        .map(|(namespace, _)| namespace.to_string())
        .ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                format!("invalid Record definition_id '{definition_id}'"),
            )
        })?;
    let records = &mut files
        .entry(namespace.clone())
        .or_insert_with(|| RecordFile {
            namespace,
            records: Vec::new(),
        })
        .records;
    match records.binary_search_by(|candidate| candidate.definition_id().cmp(&definition_id)) {
        Ok(index) => records[index] = record,
        Err(index) => records.insert(index, record),
    }
    Ok(())
}

fn remove_record(files: &mut BTreeMap<String, RecordFile>, definition_id: &str) {
    let Some((namespace, _)) = split_record_definition_id(definition_id) else {
        return;
    };
    let Some(file) = files.get_mut(namespace) else {
        return;
    };
    if let Ok(index) = file
        .records
        .binary_search_by(|record| record.definition_id().cmp(definition_id))
    {
        file.records.remove(index);
    }
    if file.records.is_empty() {
        files.remove(namespace);
    }
}

fn mission_exists(snapshot: &SyncedRepositorySnapshot, mission_id: &str) -> bool {
    snapshot
        .missions
        .iter()
        .flat_map(|file| file.missions.iter())
        .any(|mission| mission.id == mission_id)
}

fn not_found(entity: &str, id: &str) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::NotFound,
        format!("{entity} '{id}' was not found"),
    )
}
