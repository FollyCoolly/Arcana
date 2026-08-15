use super::migrations::initialize_connection;
use crate::domain::{
    split_record_definition_id, AchievementFile, AchievementState, AchievementStateFile,
    AchievementStatus, ArcanaManifest, ArcanaRepository, ArcanaRepositoryReader,
    ArcanaRepositoryTransaction, AssistantMemory, AssistantMemoryFile, AssistantMemoryKind,
    CollectionItem, CollectionRecord, DashboardMissionSelection, DashboardMissionSelections,
    DashboardMissionSlot, DefinitionRegistry, DimensionDefinition, DimensionFile, EventEntry,
    EventRecord, Mission, MissionDifficulty, MissionFile, MissionStatus, MissionSuggestion,
    MissionSuggestionStatus, Pack, PackManifest, Record, RecordDefinition, RecordDefinitionFile,
    RecordFile, RecordKind, RepositoryError, RepositoryErrorCode, RepositoryResult, ScalarRecord,
    SkillDefinition, SkillFile, StatusDimensionSelection, SyncedRepositorySnapshot, Validate,
    SCHEMA_VERSION,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

pub struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    pub fn open(path: impl AsRef<Path>) -> RepositoryResult<Self> {
        let connection = Connection::open(path).map_err(map_sqlite_error)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> RepositoryResult<Self> {
        let connection = Connection::open_in_memory().map_err(map_sqlite_error)?;
        Self::from_connection(connection)
    }

    fn from_connection(mut connection: Connection) -> RepositoryResult<Self> {
        initialize_connection(&mut connection).map_err(|error| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("failed to initialize Arcana database: {error}"),
            )
        })?;
        Ok(Self { connection })
    }
}

impl ArcanaRepositoryReader for SqliteRepository {
    fn load_synced_snapshot(&self) -> RepositoryResult<SyncedRepositorySnapshot> {
        load_synced_snapshot(&self.connection)
    }

    fn get_record(&self, definition_id: &str) -> RepositoryResult<Option<Record>> {
        load_record(&self.connection, definition_id)
    }

    fn list_mission_suggestions(&self) -> RepositoryResult<Vec<MissionSuggestion>> {
        load_mission_suggestions(&self.connection)
    }

    fn status_dimension_selection(&self) -> RepositoryResult<Vec<StatusDimensionSelection>> {
        load_status_dimension_selection(&self.connection)
    }

    fn dashboard_mission_selections(&self) -> RepositoryResult<DashboardMissionSelections> {
        load_dashboard_mission_selections(&self.connection)
    }
}

impl ArcanaRepository for SqliteRepository {
    type Transaction<'a>
        = SqliteRepositoryTransaction<'a>
    where
        Self: 'a;

    fn begin_transaction(&mut self) -> RepositoryResult<Self::Transaction<'_>> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error)?;
        Ok(SqliteRepositoryTransaction {
            transaction: Some(transaction),
            synced_data_changed: false,
        })
    }
}

pub struct SqliteRepositoryTransaction<'connection> {
    transaction: Option<Transaction<'connection>>,
    synced_data_changed: bool,
}

impl SqliteRepositoryTransaction<'_> {
    fn connection(&self) -> &Connection {
        self.transaction
            .as_ref()
            .expect("repository transaction is active")
    }

    fn transaction(&mut self) -> &Transaction<'_> {
        self.transaction
            .as_ref()
            .expect("repository transaction is active")
    }

    fn mark_synced_change(&mut self) {
        self.synced_data_changed = true;
    }
}

impl ArcanaRepositoryTransaction for SqliteRepositoryTransaction<'_> {
    fn put_pack(&mut self, pack: Pack) -> RepositoryResult<()> {
        validate(&pack)?;
        write_pack(self.transaction(), &pack)?;
        self.mark_synced_change();
        Ok(())
    }

    fn delete_pack(&mut self, pack_id: &str) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute("DELETE FROM packs WHERE id = ?1", [pack_id])
            .map_err(map_sqlite_error)?;
        require_changed(changed, "Pack", pack_id)?;
        self.mark_synced_change();
        Ok(())
    }

    fn set_pack_enabled(&mut self, pack_id: &str, enabled: bool) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute(
                "UPDATE packs SET enabled = ?2 WHERE id = ?1",
                params![pack_id, bool_to_i64(enabled)],
            )
            .map_err(map_sqlite_error)?;
        require_changed(changed, "Pack", pack_id)?;
        self.mark_synced_change();
        Ok(())
    }

    fn put_record(&mut self, record: Record) -> RepositoryResult<()> {
        validate(&record)?;
        let registry = load_definition_registry(self.connection())?;
        let definition = registry.get(record.definition_id()).ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!(
                    "RecordDefinition '{}' is not supplied by an enabled Pack",
                    record.definition_id()
                ),
            )
        })?;
        record.validate_against(definition)?;
        write_record(self.transaction(), &record)?;
        self.mark_synced_change();
        Ok(())
    }

    fn delete_record(&mut self, definition_id: &str) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute(
                "DELETE FROM records WHERE definition_id = ?1",
                [definition_id],
            )
            .map_err(map_sqlite_error)?;
        require_changed(changed, "Record", definition_id)?;
        self.mark_synced_change();
        Ok(())
    }

    fn set_achievement_state(
        &mut self,
        achievement_id: &str,
        state: AchievementState,
    ) -> RepositoryResult<()> {
        validate(&state)?;
        let available = self
            .connection()
            .query_row(
                "SELECT 1
                 FROM pack_achievements a
                 JOIN packs p ON p.id = a.pack_id
                 WHERE a.achievement_id = ?1 AND p.enabled = 1",
                [achievement_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(map_sqlite_error)?
            .is_some();
        if !available {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!(
                    "AchievementDefinition '{achievement_id}' is not supplied by an enabled Pack"
                ),
            ));
        }
        self.transaction()
            .execute(
                "INSERT INTO achievement_states(achievement_id, status, achieved_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(achievement_id) DO UPDATE SET
                    status = excluded.status,
                    achieved_at = excluded.achieved_at",
                params![
                    achievement_id,
                    achievement_status_to_str(state.status),
                    state.achieved_at
                ],
            )
            .map_err(map_sqlite_error)?;
        self.mark_synced_change();
        Ok(())
    }

    fn revoke_achievement_state(&mut self, achievement_id: &str) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute(
                "DELETE FROM achievement_states WHERE achievement_id = ?1",
                [achievement_id],
            )
            .map_err(map_sqlite_error)?;
        require_changed(changed, "Achievement state", achievement_id)?;
        self.mark_synced_change();
        Ok(())
    }

    fn put_mission(&mut self, mission: Mission) -> RepositoryResult<()> {
        validate(&mission)?;
        write_mission(self.transaction(), &mission)?;
        self.mark_synced_change();
        Ok(())
    }

    fn delete_mission(&mut self, mission_id: &str) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute("DELETE FROM missions WHERE id = ?1", [mission_id])
            .map_err(map_sqlite_error)?;
        require_changed(changed, "Mission", mission_id)?;
        self.mark_synced_change();
        Ok(())
    }

    fn put_mission_suggestion(&mut self, suggestion: MissionSuggestion) -> RepositoryResult<()> {
        validate(&suggestion)?;
        if row_exists(self.connection(), "missions", "id", &suggestion.id)? {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("Mission '{}' already exists", suggestion.id),
            ));
        }
        if let Some(parent_id) = &suggestion.parent_mission_id {
            if !row_exists(self.connection(), "missions", "id", parent_id)? {
                return Err(RepositoryError::new(
                    RepositoryErrorCode::Unresolved,
                    format!("parent Mission '{parent_id}' does not exist"),
                ));
            }
        }
        write_mission_suggestion(self.transaction(), &suggestion)
    }

    fn delete_mission_suggestion(&mut self, suggestion_id: &str) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute(
                "DELETE FROM mission_suggestions WHERE id = ?1",
                [suggestion_id],
            )
            .map_err(map_sqlite_error)?;
        require_changed(changed, "MissionSuggestion", suggestion_id)
    }

    fn put_assistant_memory(&mut self, memory: AssistantMemory) -> RepositoryResult<()> {
        validate(&memory)?;
        write_assistant_memory(self.transaction(), &memory)?;
        self.mark_synced_change();
        Ok(())
    }

    fn delete_assistant_memory(&mut self, memory_id: &str) -> RepositoryResult<()> {
        let changed = self
            .transaction()
            .execute("DELETE FROM assistant_memories WHERE id = ?1", [memory_id])
            .map_err(map_sqlite_error)?;
        require_changed(changed, "AssistantMemory", memory_id)?;
        self.mark_synced_change();
        Ok(())
    }

    fn set_status_dimension_selection(
        &mut self,
        selection: StatusDimensionSelection,
    ) -> RepositoryResult<()> {
        validate(&selection)?;
        let available = self
            .connection()
            .query_row(
                "SELECT 1
                 FROM pack_dimensions d
                 JOIN packs p ON p.id = d.pack_id
                 WHERE d.dimension_id = ?1 AND p.enabled = 1",
                [&selection.dimension_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(map_sqlite_error)?
            .is_some();
        if !available {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!(
                    "Status Dimension '{}' is not supplied by an enabled Pack",
                    selection.dimension_id
                ),
            ));
        }
        self.transaction()
            .execute(
                "DELETE FROM status_dimension_selection
                 WHERE dimension_id = ?1 AND position <> ?2",
                params![selection.dimension_id, selection.position],
            )
            .map_err(map_sqlite_error)?;
        self.transaction()
            .execute(
                "INSERT INTO status_dimension_selection(position, dimension_id)
                 VALUES (?1, ?2)
                 ON CONFLICT(position) DO UPDATE SET dimension_id = excluded.dimension_id",
                params![selection.position, selection.dimension_id],
            )
            .map_err(map_sqlite_error)?;
        Ok(())
    }

    fn clear_status_dimension_selection(&mut self, position: u8) -> RepositoryResult<()> {
        if position >= 5 {
            return Err(RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                "Status Dimension position must be between 0 and 4",
            ));
        }
        self.transaction()
            .execute(
                "DELETE FROM status_dimension_selection WHERE position = ?1",
                [position],
            )
            .map_err(map_sqlite_error)?;
        Ok(())
    }

    fn set_dashboard_mission_selection(
        &mut self,
        slot: DashboardMissionSlot,
        selection: DashboardMissionSelection,
    ) -> RepositoryResult<()> {
        validate(&selection)?;
        if !row_exists(self.connection(), "missions", "id", &selection.mission_id)? {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Unresolved,
                format!("Mission '{}' does not exist", selection.mission_id),
            ));
        }
        self.transaction()
            .execute(
                "INSERT INTO dashboard_mission_slots(slot, mission_id, label)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(slot) DO UPDATE SET
                    mission_id = excluded.mission_id,
                    label = excluded.label",
                params![
                    dashboard_slot_to_str(slot),
                    selection.mission_id,
                    selection.label
                ],
            )
            .map_err(map_sqlite_error)?;
        Ok(())
    }

    fn clear_dashboard_mission_selection(
        &mut self,
        slot: DashboardMissionSlot,
    ) -> RepositoryResult<()> {
        self.transaction()
            .execute(
                "DELETE FROM dashboard_mission_slots WHERE slot = ?1",
                [dashboard_slot_to_str(slot)],
            )
            .map_err(map_sqlite_error)?;
        Ok(())
    }

    fn replace_synced_snapshot(
        &mut self,
        snapshot: SyncedRepositorySnapshot,
    ) -> RepositoryResult<()> {
        validate(&snapshot)?;
        let transaction = self.transaction();
        transaction
            .execute_batch(
                "DELETE FROM achievement_states;
                 DELETE FROM assistant_memories;
                 DELETE FROM records;
                 UPDATE missions SET parent_id = NULL;
                 DELETE FROM missions;
                 DELETE FROM packs;",
            )
            .map_err(map_sqlite_error)?;

        for pack in snapshot.packs.values() {
            write_pack(transaction, pack)?;
        }
        for pack_id in &snapshot.manifest.enabled_pack_ids {
            transaction
                .execute("UPDATE packs SET enabled = 1 WHERE id = ?1", [pack_id])
                .map_err(map_sqlite_error)?;
        }
        for record in snapshot
            .records
            .values()
            .flat_map(|file| file.records.iter())
        {
            write_record(transaction, record)?;
        }
        if let Some(states) = &snapshot.achievement_states {
            for (achievement_id, state) in &states.states {
                write_achievement_state(transaction, achievement_id, state)?;
            }
        }
        if let Some(missions) = &snapshot.missions {
            for mission in &missions.missions {
                write_mission(transaction, mission)?;
            }
        }
        if let Some(memories) = &snapshot.assistant_memory {
            for memory in &memories.memories {
                write_assistant_memory(transaction, memory)?;
            }
        }
        transaction
            .execute(
                "DELETE FROM mission_suggestions
                 WHERE id IN (SELECT id FROM missions)",
                [],
            )
            .map_err(map_sqlite_error)?;
        self.mark_synced_change();
        Ok(())
    }

    fn commit(mut self) -> RepositoryResult<()> {
        let snapshot = load_synced_snapshot(self.connection())?;
        validate(&snapshot)?;
        if self.synced_data_changed {
            self.transaction()
                .execute(
                    "UPDATE sync_state SET data_revision = data_revision + 1 WHERE singleton = 1",
                    [],
                )
                .map_err(map_sqlite_error)?;
        }
        self.transaction
            .take()
            .expect("repository transaction is active")
            .commit()
            .map_err(map_sqlite_error)
    }

    fn rollback(mut self) -> RepositoryResult<()> {
        self.transaction
            .take()
            .expect("repository transaction is active")
            .rollback()
            .map_err(map_sqlite_error)
    }
}

fn load_synced_snapshot(connection: &Connection) -> RepositoryResult<SyncedRepositorySnapshot> {
    let (packs, enabled_pack_ids) = load_packs(connection)?;
    let snapshot = SyncedRepositorySnapshot {
        manifest: ArcanaManifest {
            schema_version: SCHEMA_VERSION,
            enabled_pack_ids,
        },
        packs,
        records: load_records(connection)?,
        achievement_states: load_achievement_states(connection)?,
        missions: load_missions(connection)?,
        assistant_memory: load_assistant_memories(connection)?,
    };
    validate(&snapshot)?;
    Ok(snapshot)
}

fn load_packs(connection: &Connection) -> RepositoryResult<(BTreeMap<String, Pack>, Vec<String>)> {
    let mut statement = connection
        .prepare("SELECT id, enabled, schema_version, manifest_json FROM packs ORDER BY id")
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(map_sqlite_error)?;
    let mut packs = BTreeMap::new();
    let mut enabled_pack_ids = Vec::new();
    for row in rows {
        let (id, enabled, schema_version, manifest_json) = row.map_err(map_sqlite_error)?;
        let manifest: PackManifest = deserialize_json(&manifest_json, "packs.manifest_json")?;
        require_projection(
            manifest.id == id && i64::from(manifest.schema_version) == schema_version,
            format!("Pack '{id}' projection does not match manifest_json"),
        )?;
        if enabled == 1 {
            enabled_pack_ids.push(id.clone());
        }
        let pack = Pack {
            manifest,
            record_definitions: load_record_definitions(connection, &id)?,
            dimensions: load_dimensions(connection, &id)?,
            achievements: load_achievement_definitions(connection, &id)?,
            skills: load_skill_definitions(connection, &id)?,
            assets: load_pack_assets(connection, &id)?,
        };
        packs.insert(id, pack);
    }
    Ok((packs, enabled_pack_ids))
}

fn load_record_definitions(
    connection: &Connection,
    pack_id: &str,
) -> RepositoryResult<Option<RecordDefinitionFile>> {
    let values = load_json_definitions::<RecordDefinition>(
        connection,
        "SELECT definition_id, definition_json
         FROM pack_record_definitions WHERE pack_id = ?1 ORDER BY definition_id",
        pack_id,
        |definition| definition.id(),
    )?;
    Ok((!values.is_empty()).then_some(RecordDefinitionFile {
        definitions: values,
    }))
}

fn load_dimensions(
    connection: &Connection,
    pack_id: &str,
) -> RepositoryResult<Option<DimensionFile>> {
    let values = load_json_definitions::<DimensionDefinition>(
        connection,
        "SELECT dimension_id, definition_json
         FROM pack_dimensions WHERE pack_id = ?1 ORDER BY dimension_id",
        pack_id,
        |dimension| &dimension.id,
    )?;
    Ok((!values.is_empty()).then_some(DimensionFile { dimensions: values }))
}

fn load_achievement_definitions(
    connection: &Connection,
    pack_id: &str,
) -> RepositoryResult<Option<AchievementFile>> {
    let values = load_json_definitions(
        connection,
        "SELECT achievement_id, definition_json
         FROM pack_achievements WHERE pack_id = ?1 ORDER BY achievement_id",
        pack_id,
        |achievement: &crate::domain::AchievementDefinition| &achievement.id,
    )?;
    Ok((!values.is_empty()).then_some(AchievementFile {
        achievements: values,
    }))
}

fn load_skill_definitions(
    connection: &Connection,
    pack_id: &str,
) -> RepositoryResult<Option<SkillFile>> {
    let values = load_json_definitions::<SkillDefinition>(
        connection,
        "SELECT skill_id, definition_json
         FROM pack_skills WHERE pack_id = ?1 ORDER BY skill_id",
        pack_id,
        |skill| &skill.id,
    )?;
    Ok((!values.is_empty()).then_some(SkillFile { skills: values }))
}

fn load_json_definitions<T>(
    connection: &Connection,
    sql: &str,
    pack_id: &str,
    id: impl Fn(&T) -> &str,
) -> RepositoryResult<Vec<T>>
where
    T: DeserializeOwned,
{
    let mut statement = connection.prepare(sql).map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([pack_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sqlite_error)?;
    let mut values = Vec::new();
    for row in rows {
        let (projected_id, json) = row.map_err(map_sqlite_error)?;
        let value: T = deserialize_json(&json, "Pack definition JSON")?;
        require_projection(
            id(&value) == projected_id,
            format!("Definition '{projected_id}' projection does not match JSON"),
        )?;
        values.push(value);
    }
    Ok(values)
}

fn load_pack_assets(
    connection: &Connection,
    pack_id: &str,
) -> RepositoryResult<BTreeMap<String, Vec<u8>>> {
    let mut statement = connection
        .prepare("SELECT path, content FROM pack_assets WHERE pack_id = ?1 ORDER BY path")
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([pack_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(map_sqlite_error)?;
    let mut assets = BTreeMap::new();
    for row in rows {
        let (path, content) = row.map_err(map_sqlite_error)?;
        assets.insert(path, content);
    }
    Ok(assets)
}

fn load_definition_registry(connection: &Connection) -> RepositoryResult<DefinitionRegistry> {
    let (packs, enabled_ids) = load_packs(connection)?;
    DefinitionRegistry::build(enabled_ids.iter().filter_map(|id| packs.get(id))).map_err(Into::into)
}

fn load_records(connection: &Connection) -> RepositoryResult<BTreeMap<String, RecordFile>> {
    let mut statement = connection
        .prepare("SELECT definition_id FROM records ORDER BY definition_id")
        .map_err(map_sqlite_error)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(map_sqlite_error)?;
    let mut files: BTreeMap<String, RecordFile> = BTreeMap::new();
    for id in ids {
        let id = id.map_err(map_sqlite_error)?;
        let record = load_record(connection, &id)?.ok_or_else(|| {
            RepositoryError::new(
                RepositoryErrorCode::Storage,
                format!("Record header '{id}' disappeared while reading"),
            )
        })?;
        let namespace = split_record_definition_id(&id)
            .map(|(namespace, _)| namespace.to_string())
            .ok_or_else(|| {
                RepositoryError::new(
                    RepositoryErrorCode::Storage,
                    format!("stored Record has invalid definition_id '{id}'"),
                )
            })?;
        files
            .entry(namespace.clone())
            .or_insert_with(|| RecordFile {
                namespace,
                records: Vec::new(),
            })
            .records
            .push(record);
    }
    Ok(files)
}

fn load_record(connection: &Connection, definition_id: &str) -> RepositoryResult<Option<Record>> {
    let kind = connection
        .query_row(
            "SELECT kind FROM records WHERE definition_id = ?1",
            [definition_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_sqlite_error)?;
    let Some(kind) = kind else {
        return Ok(None);
    };
    match kind.as_str() {
        "scalar" => {
            let row = connection
                .query_row(
                    "SELECT value_json, effective_at, recorded_at
                     FROM scalar_records WHERE definition_id = ?1",
                    [definition_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(map_sqlite_error)?
                .ok_or_else(|| {
                    RepositoryError::new(
                        RepositoryErrorCode::Storage,
                        format!("scalar Record '{definition_id}' has no payload"),
                    )
                })?;
            Ok(Some(Record::Scalar(ScalarRecord {
                definition_id: definition_id.to_string(),
                value: deserialize_json(&row.0, "scalar_records.value_json")?,
                effective_at: row.1,
                recorded_at: row.2,
            })))
        }
        "collection" => {
            let mut statement = connection
                .prepare(
                    "SELECT item_id, payload_json, recorded_at
                     FROM collection_items WHERE definition_id = ?1 ORDER BY item_id",
                )
                .map_err(map_sqlite_error)?;
            let rows = statement
                .query_map([definition_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(map_sqlite_error)?;
            let mut items = Vec::new();
            for row in rows {
                let (id, payload, recorded_at) = row.map_err(map_sqlite_error)?;
                items.push(CollectionItem {
                    id,
                    fields: deserialize_json(&payload, "collection_items.payload_json")?,
                    recorded_at,
                });
            }
            Ok(Some(Record::Collection(CollectionRecord {
                definition_id: definition_id.to_string(),
                items,
            })))
        }
        "event" => {
            let mut statement = connection
                .prepare(
                    "SELECT event_id, occurred_at, payload_json, recorded_at
                     FROM event_entries
                     WHERE definition_id = ?1 ORDER BY occurred_at, event_id",
                )
                .map_err(map_sqlite_error)?;
            let rows = statement
                .query_map([definition_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(map_sqlite_error)?;
            let mut events = Vec::new();
            for row in rows {
                let (id, occurred_at, payload, recorded_at) = row.map_err(map_sqlite_error)?;
                events.push(EventEntry {
                    id,
                    occurred_at,
                    fields: deserialize_json(&payload, "event_entries.payload_json")?,
                    recorded_at,
                });
            }
            Ok(Some(Record::Event(EventRecord {
                definition_id: definition_id.to_string(),
                events,
            })))
        }
        value => Err(RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("Record '{definition_id}' has unknown kind '{value}'"),
        )),
    }
}

fn load_achievement_states(
    connection: &Connection,
) -> RepositoryResult<Option<AchievementStateFile>> {
    let mut statement = connection
        .prepare(
            "SELECT achievement_id, status, achieved_at
             FROM achievement_states ORDER BY achievement_id",
        )
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(map_sqlite_error)?;
    let mut states = BTreeMap::new();
    for row in rows {
        let (id, status, achieved_at) = row.map_err(map_sqlite_error)?;
        states.insert(
            id,
            AchievementState {
                status: achievement_status_from_str(&status)?,
                achieved_at,
            },
        );
    }
    Ok((!states.is_empty()).then_some(AchievementStateFile { states }))
}

fn load_missions(connection: &Connection) -> RepositoryResult<Option<MissionFile>> {
    let mut statement = connection
        .prepare(
            "SELECT id, title, description, status, progress, difficulty, deadline,
                    parent_id, created_at, completed_at
             FROM missions ORDER BY id",
        )
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<u8>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(map_sqlite_error)?;
    let mut missions = Vec::new();
    for row in rows {
        let row = row.map_err(map_sqlite_error)?;
        missions.push(Mission {
            id: row.0,
            title: row.1,
            description: row.2,
            status: mission_status_from_str(&row.3)?,
            progress: row.4,
            difficulty: row
                .5
                .as_deref()
                .map(mission_difficulty_from_str)
                .transpose()?,
            deadline: row.6,
            parent_id: row.7,
            created_at: row.8,
            completed_at: row.9,
        });
    }
    Ok((!missions.is_empty()).then_some(MissionFile { missions }))
}

fn load_assistant_memories(
    connection: &Connection,
) -> RepositoryResult<Option<AssistantMemoryFile>> {
    let mut statement = connection
        .prepare(
            "SELECT id, kind, content, created_at, updated_at
             FROM assistant_memories ORDER BY id",
        )
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(map_sqlite_error)?;
    let mut memories = Vec::new();
    for row in rows {
        let (id, kind, content, created_at, updated_at) = row.map_err(map_sqlite_error)?;
        memories.push(AssistantMemory {
            id,
            kind: memory_kind_from_str(&kind)?,
            content,
            created_at,
            updated_at,
        });
    }
    Ok((!memories.is_empty()).then_some(AssistantMemoryFile { memories }))
}

fn load_mission_suggestions(connection: &Connection) -> RepositoryResult<Vec<MissionSuggestion>> {
    let mut statement = connection
        .prepare(
            "SELECT id, title, description, difficulty, deadline, parent_mission_id,
                    reason, generated_at, status
             FROM mission_suggestions ORDER BY generated_at, id",
        )
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(map_sqlite_error)?;
    let mut suggestions = Vec::new();
    for row in rows {
        let row = row.map_err(map_sqlite_error)?;
        suggestions.push(MissionSuggestion {
            id: row.0,
            title: row.1,
            description: row.2,
            difficulty: row
                .3
                .as_deref()
                .map(mission_difficulty_from_str)
                .transpose()?,
            deadline: row.4,
            parent_mission_id: row.5,
            reason: row.6,
            generated_at: row.7,
            status: suggestion_status_from_str(&row.8)?,
        });
    }
    Ok(suggestions)
}

fn load_status_dimension_selection(
    connection: &Connection,
) -> RepositoryResult<Vec<StatusDimensionSelection>> {
    let mut statement = connection
        .prepare(
            "SELECT position, dimension_id
             FROM status_dimension_selection ORDER BY position",
        )
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(StatusDimensionSelection {
                position: row.get(0)?,
                dimension_id: row.get(1)?,
            })
        })
        .map_err(map_sqlite_error)?;
    rows.map(|row| row.map_err(map_sqlite_error)).collect()
}

fn load_dashboard_mission_selections(
    connection: &Connection,
) -> RepositoryResult<DashboardMissionSelections> {
    let mut statement = connection
        .prepare("SELECT slot, mission_id, label FROM dashboard_mission_slots ORDER BY slot")
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(map_sqlite_error)?;
    let mut selections = BTreeMap::new();
    for row in rows {
        let (slot, mission_id, label) = row.map_err(map_sqlite_error)?;
        selections.insert(
            dashboard_slot_from_str(&slot)?,
            DashboardMissionSelection { mission_id, label },
        );
    }
    Ok(selections)
}

fn write_pack(connection: &Connection, pack: &Pack) -> RepositoryResult<()> {
    let manifest_json = serialize_json(&pack.manifest, "Pack manifest")?;
    connection
        .execute(
            "INSERT INTO packs(id, enabled, schema_version, manifest_json)
             VALUES (?1, 0, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
                schema_version = excluded.schema_version,
                manifest_json = excluded.manifest_json",
            params![
                pack.manifest.id,
                i64::from(pack.manifest.schema_version),
                manifest_json
            ],
        )
        .map_err(map_sqlite_error)?;
    for table in [
        "pack_record_definitions",
        "pack_dimensions",
        "pack_achievements",
        "pack_skills",
        "pack_assets",
    ] {
        connection
            .execute(
                &format!("DELETE FROM {table} WHERE pack_id = ?1"),
                [&pack.manifest.id],
            )
            .map_err(map_sqlite_error)?;
    }
    if let Some(file) = &pack.record_definitions {
        for definition in &file.definitions {
            connection
                .execute(
                    "INSERT INTO pack_record_definitions(pack_id, definition_id, definition_json)
                     VALUES (?1, ?2, ?3)",
                    params![
                        pack.manifest.id,
                        definition.id(),
                        serialize_json(definition, "RecordDefinition")?
                    ],
                )
                .map_err(map_sqlite_error)?;
        }
    }
    if let Some(file) = &pack.dimensions {
        for dimension in &file.dimensions {
            connection
                .execute(
                    "INSERT INTO pack_dimensions(pack_id, dimension_id, definition_json)
                     VALUES (?1, ?2, ?3)",
                    params![
                        pack.manifest.id,
                        dimension.id,
                        serialize_json(dimension, "DimensionDefinition")?
                    ],
                )
                .map_err(map_sqlite_error)?;
        }
    }
    if let Some(file) = &pack.achievements {
        for achievement in &file.achievements {
            connection
                .execute(
                    "INSERT INTO pack_achievements(pack_id, achievement_id, definition_json)
                     VALUES (?1, ?2, ?3)",
                    params![
                        pack.manifest.id,
                        achievement.id,
                        serialize_json(achievement, "AchievementDefinition")?
                    ],
                )
                .map_err(map_sqlite_error)?;
        }
    }
    if let Some(file) = &pack.skills {
        for skill in &file.skills {
            connection
                .execute(
                    "INSERT INTO pack_skills(pack_id, skill_id, definition_json)
                     VALUES (?1, ?2, ?3)",
                    params![
                        pack.manifest.id,
                        skill.id,
                        serialize_json(skill, "SkillDefinition")?
                    ],
                )
                .map_err(map_sqlite_error)?;
        }
    }
    for (path, content) in &pack.assets {
        connection
            .execute(
                "INSERT INTO pack_assets(pack_id, path, content) VALUES (?1, ?2, ?3)",
                params![pack.manifest.id, path, content],
            )
            .map_err(map_sqlite_error)?;
    }
    Ok(())
}

fn write_record(connection: &Connection, record: &Record) -> RepositoryResult<()> {
    let definition_id = record.definition_id();
    let kind = record_kind_to_str(record.kind());
    let existing_kind = connection
        .query_row(
            "SELECT kind FROM records WHERE definition_id = ?1",
            [definition_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_sqlite_error)?;
    if existing_kind
        .as_deref()
        .is_some_and(|existing| existing != kind)
    {
        return Err(RepositoryError::new(
            RepositoryErrorCode::Conflict,
            format!("Record '{definition_id}' kind cannot be changed"),
        ));
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO records(definition_id, kind) VALUES (?1, ?2)",
            params![definition_id, kind],
        )
        .map_err(map_sqlite_error)?;
    connection
        .execute(
            "DELETE FROM scalar_records WHERE definition_id = ?1",
            [definition_id],
        )
        .map_err(map_sqlite_error)?;
    connection
        .execute(
            "DELETE FROM collection_items WHERE definition_id = ?1",
            [definition_id],
        )
        .map_err(map_sqlite_error)?;
    connection
        .execute(
            "DELETE FROM event_entries WHERE definition_id = ?1",
            [definition_id],
        )
        .map_err(map_sqlite_error)?;

    match record {
        Record::Scalar(record) => {
            connection
                .execute(
                    "INSERT INTO scalar_records(definition_id, value_json, effective_at, recorded_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        record.definition_id,
                        serialize_json(&record.value, "scalar Record value")?,
                        record.effective_at,
                        record.recorded_at
                    ],
                )
                .map_err(map_sqlite_error)?;
        }
        Record::Collection(record) => {
            for item in &record.items {
                connection
                    .execute(
                        "INSERT INTO collection_items(definition_id, item_id, payload_json, recorded_at)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![
                            record.definition_id,
                            item.id,
                            serialize_json(&item.fields, "collection item payload")?,
                            item.recorded_at
                        ],
                    )
                    .map_err(map_sqlite_error)?;
            }
        }
        Record::Event(record) => {
            for event in &record.events {
                connection
                    .execute(
                        "INSERT INTO event_entries(
                            definition_id, event_id, occurred_at, payload_json, recorded_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            record.definition_id,
                            event.id,
                            event.occurred_at,
                            serialize_json(&event.fields, "event payload")?,
                            event.recorded_at
                        ],
                    )
                    .map_err(map_sqlite_error)?;
            }
        }
    }
    Ok(())
}

fn write_achievement_state(
    connection: &Connection,
    achievement_id: &str,
    state: &AchievementState,
) -> RepositoryResult<()> {
    connection
        .execute(
            "INSERT INTO achievement_states(achievement_id, status, achieved_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(achievement_id) DO UPDATE SET
                status = excluded.status,
                achieved_at = excluded.achieved_at",
            params![
                achievement_id,
                achievement_status_to_str(state.status),
                state.achieved_at
            ],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

fn write_mission(connection: &Connection, mission: &Mission) -> RepositoryResult<()> {
    connection
        .execute(
            "INSERT INTO missions(
                id, title, description, status, progress, difficulty, deadline,
                parent_id, created_at, completed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                status = excluded.status,
                progress = excluded.progress,
                difficulty = excluded.difficulty,
                deadline = excluded.deadline,
                parent_id = excluded.parent_id,
                created_at = excluded.created_at,
                completed_at = excluded.completed_at",
            params![
                mission.id,
                mission.title,
                mission.description,
                mission_status_to_str(mission.status),
                mission.progress,
                mission.difficulty.map(mission_difficulty_to_str),
                mission.deadline,
                mission.parent_id,
                mission.created_at,
                mission.completed_at
            ],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

fn write_mission_suggestion(
    connection: &Connection,
    suggestion: &MissionSuggestion,
) -> RepositoryResult<()> {
    connection
        .execute(
            "INSERT INTO mission_suggestions(
                id, title, description, difficulty, deadline, parent_mission_id,
                reason, generated_at, status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                difficulty = excluded.difficulty,
                deadline = excluded.deadline,
                parent_mission_id = excluded.parent_mission_id,
                reason = excluded.reason,
                generated_at = excluded.generated_at,
                status = excluded.status",
            params![
                suggestion.id,
                suggestion.title,
                suggestion.description,
                suggestion.difficulty.map(mission_difficulty_to_str),
                suggestion.deadline,
                suggestion.parent_mission_id,
                suggestion.reason,
                suggestion.generated_at,
                suggestion_status_to_str(suggestion.status)
            ],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

fn write_assistant_memory(
    connection: &Connection,
    memory: &AssistantMemory,
) -> RepositoryResult<()> {
    connection
        .execute(
            "INSERT INTO assistant_memories(id, kind, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                content = excluded.content,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at",
            params![
                memory.id,
                memory_kind_to_str(memory.kind),
                memory.content,
                memory.created_at,
                memory.updated_at
            ],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

fn validate<T: Validate>(value: &T) -> RepositoryResult<()> {
    value.validate().map_err(Into::into)
}

fn serialize_json(value: &impl Serialize, context: &str) -> RepositoryResult<String> {
    serde_json::to_string(value).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to serialize {context}: {error}"),
        )
    })
}

fn deserialize_json<T: DeserializeOwned>(json: &str, context: &str) -> RepositoryResult<T> {
    serde_json::from_str(json).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to deserialize {context}: {error}"),
        )
    })
}

fn require_projection(condition: bool, message: String) -> RepositoryResult<()> {
    if condition {
        Ok(())
    } else {
        Err(RepositoryError::new(RepositoryErrorCode::Storage, message))
    }
}

fn require_changed(changed: usize, entity: &str, id: &str) -> RepositoryResult<()> {
    if changed == 0 {
        Err(RepositoryError::new(
            RepositoryErrorCode::NotFound,
            format!("{entity} '{id}' was not found"),
        ))
    } else {
        Ok(())
    }
}

fn row_exists(
    connection: &Connection,
    table: &str,
    column: &str,
    value: &str,
) -> RepositoryResult<bool> {
    let allowed = matches!((table, column), ("missions", "id"));
    debug_assert!(allowed, "row_exists table and column must be hard-coded");
    if !allowed {
        return Err(RepositoryError::new(
            RepositoryErrorCode::Storage,
            "invalid internal row_exists query",
        ));
    }
    connection
        .query_row(
            &format!("SELECT 1 FROM {table} WHERE {column} = ?1"),
            [value],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(map_sqlite_error)
}

fn map_sqlite_error(error: rusqlite::Error) -> RepositoryError {
    use rusqlite::ffi::ErrorCode;
    let code = match &error {
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
            ) =>
        {
            RepositoryErrorCode::Busy
        }
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::ConstraintViolation =>
        {
            RepositoryErrorCode::Conflict
        }
        _ => RepositoryErrorCode::Storage,
    };
    RepositoryError::new(code, format!("SQLite error: {error}"))
}

fn bool_to_i64(value: bool) -> i64 {
    i64::from(value)
}

fn record_kind_to_str(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Scalar => "scalar",
        RecordKind::Collection => "collection",
        RecordKind::Event => "event",
    }
}

fn achievement_status_to_str(status: AchievementStatus) -> &'static str {
    match status {
        AchievementStatus::Tracked => "tracked",
        AchievementStatus::Achieved => "achieved",
    }
}

fn achievement_status_from_str(value: &str) -> RepositoryResult<AchievementStatus> {
    match value {
        "tracked" => Ok(AchievementStatus::Tracked),
        "achieved" => Ok(AchievementStatus::Achieved),
        _ => invalid_enum("Achievement status", value),
    }
}

fn mission_status_to_str(status: MissionStatus) -> &'static str {
    match status {
        MissionStatus::Active => "active",
        MissionStatus::Completed => "completed",
        MissionStatus::Archived => "archived",
    }
}

fn mission_status_from_str(value: &str) -> RepositoryResult<MissionStatus> {
    match value {
        "active" => Ok(MissionStatus::Active),
        "completed" => Ok(MissionStatus::Completed),
        "archived" => Ok(MissionStatus::Archived),
        _ => invalid_enum("Mission status", value),
    }
}

fn mission_difficulty_to_str(difficulty: MissionDifficulty) -> &'static str {
    match difficulty {
        MissionDifficulty::S => "S",
        MissionDifficulty::A => "A",
        MissionDifficulty::B => "B",
        MissionDifficulty::C => "C",
        MissionDifficulty::D => "D",
    }
}

fn mission_difficulty_from_str(value: &str) -> RepositoryResult<MissionDifficulty> {
    match value {
        "S" => Ok(MissionDifficulty::S),
        "A" => Ok(MissionDifficulty::A),
        "B" => Ok(MissionDifficulty::B),
        "C" => Ok(MissionDifficulty::C),
        "D" => Ok(MissionDifficulty::D),
        _ => invalid_enum("Mission difficulty", value),
    }
}

fn suggestion_status_to_str(status: MissionSuggestionStatus) -> &'static str {
    match status {
        MissionSuggestionStatus::Pending => "pending",
        MissionSuggestionStatus::Rejected => "rejected",
    }
}

fn suggestion_status_from_str(value: &str) -> RepositoryResult<MissionSuggestionStatus> {
    match value {
        "pending" => Ok(MissionSuggestionStatus::Pending),
        "rejected" => Ok(MissionSuggestionStatus::Rejected),
        _ => invalid_enum("MissionSuggestion status", value),
    }
}

fn memory_kind_to_str(kind: AssistantMemoryKind) -> &'static str {
    match kind {
        AssistantMemoryKind::Focus => "focus",
        AssistantMemoryKind::Preference => "preference",
        AssistantMemoryKind::Constraint => "constraint",
        AssistantMemoryKind::Habit => "habit",
        AssistantMemoryKind::Summary => "summary",
        AssistantMemoryKind::Reminder => "reminder",
        AssistantMemoryKind::Observation => "observation",
    }
}

fn memory_kind_from_str(value: &str) -> RepositoryResult<AssistantMemoryKind> {
    match value {
        "focus" => Ok(AssistantMemoryKind::Focus),
        "preference" => Ok(AssistantMemoryKind::Preference),
        "constraint" => Ok(AssistantMemoryKind::Constraint),
        "habit" => Ok(AssistantMemoryKind::Habit),
        "summary" => Ok(AssistantMemoryKind::Summary),
        "reminder" => Ok(AssistantMemoryKind::Reminder),
        "observation" => Ok(AssistantMemoryKind::Observation),
        _ => invalid_enum("AssistantMemory kind", value),
    }
}

fn dashboard_slot_to_str(slot: DashboardMissionSlot) -> &'static str {
    match slot {
        DashboardMissionSlot::Countdown => "countdown",
        DashboardMissionSlot::Progress => "progress",
        DashboardMissionSlot::Hint1 => "hint_1",
        DashboardMissionSlot::Hint2 => "hint_2",
    }
}

fn dashboard_slot_from_str(value: &str) -> RepositoryResult<DashboardMissionSlot> {
    match value {
        "countdown" => Ok(DashboardMissionSlot::Countdown),
        "progress" => Ok(DashboardMissionSlot::Progress),
        "hint_1" => Ok(DashboardMissionSlot::Hint1),
        "hint_2" => Ok(DashboardMissionSlot::Hint2),
        _ => invalid_enum("Dashboard Mission slot", value),
    }
}

fn invalid_enum<T>(name: &str, value: &str) -> RepositoryResult<T> {
    Err(RepositoryError::new(
        RepositoryErrorCode::Storage,
        format!("stored {name} has unknown value '{value}'"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AchievementDefinition, AchievementDifficulty, ScalarRecordDefinition, ValueType,
    };
    use serde_json::json;
    use std::collections::BTreeSet;

    fn pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "health".to_string(),
                name: "Health".to_string(),
                description: None,
                author: Some("Arcana".to_string()),
                parent_pack_id: None,
                tags: vec![],
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![RecordDefinition::Scalar(ScalarRecordDefinition {
                    id: "health.weight".to_string(),
                    name: "Weight".to_string(),
                    description: None,
                    value_type: ValueType::Number,
                    unit: Some("kg".to_string()),
                })],
            }),
            dimensions: Some(DimensionFile {
                dimensions: vec![DimensionDefinition {
                    id: "health::vitality".to_string(),
                    name: "Vitality".to_string(),
                    level_titles: [
                        "Starting".to_string(),
                        "Steady".to_string(),
                        "Growing".to_string(),
                        "Strong".to_string(),
                        "Excellent".to_string(),
                    ],
                    level_thresholds: [20.0, 40.0, 60.0, 80.0],
                    scores: vec![crate::domain::ScoreDefinition {
                        id: "weight".to_string(),
                        name: "Weight".to_string(),
                        weight: 1.0,
                        expression: "record('health.weight')".to_string(),
                    }],
                }],
            }),
            achievements: Some(AchievementFile {
                achievements: vec![AchievementDefinition {
                    id: "health::first_checkin".to_string(),
                    name: "First check-in".to_string(),
                    description: "Record a health check-in".to_string(),
                    difficulty: AchievementDifficulty::Beginner,
                    tags: vec![],
                    prerequisites: vec![],
                    related_record_definition_ids: vec!["health.weight".to_string()],
                    tip: None,
                }],
            }),
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    fn snapshot() -> SyncedRepositorySnapshot {
        SyncedRepositorySnapshot {
            manifest: ArcanaManifest {
                schema_version: SCHEMA_VERSION,
                enabled_pack_ids: vec!["health".to_string()],
            },
            packs: BTreeMap::from([("health".to_string(), pack())]),
            records: BTreeMap::from([(
                "health".to_string(),
                RecordFile {
                    namespace: "health".to_string(),
                    records: vec![Record::Scalar(ScalarRecord {
                        definition_id: "health.weight".to_string(),
                        value: json!(72.5),
                        effective_at: Some("2026-08-15".to_string()),
                        recorded_at: "2026-08-15T20:30:00+08:00".to_string(),
                    })],
                },
            )]),
            achievement_states: Some(AchievementStateFile {
                states: BTreeMap::from([(
                    "health::first_checkin".to_string(),
                    AchievementState {
                        status: AchievementStatus::Achieved,
                        achieved_at: Some("2026-08-15".to_string()),
                    },
                )]),
            }),
            missions: Some(MissionFile {
                missions: vec![Mission {
                    id: "mission-1".to_string(),
                    title: "Walk".to_string(),
                    description: None,
                    status: MissionStatus::Active,
                    progress: Some(20),
                    difficulty: Some(MissionDifficulty::D),
                    deadline: Some("2026-08-20".to_string()),
                    parent_id: None,
                    created_at: "2026-08-15T20:30:00+08:00".to_string(),
                    completed_at: None,
                }],
            }),
            assistant_memory: Some(AssistantMemoryFile {
                memories: vec![AssistantMemory {
                    id: "memory-1".to_string(),
                    kind: AssistantMemoryKind::Preference,
                    content: "Short missions".to_string(),
                    created_at: "2026-08-15T20:30:00+08:00".to_string(),
                    updated_at: "2026-08-15T20:30:00+08:00".to_string(),
                }],
            }),
        }
    }

    #[test]
    fn snapshot_round_trips_through_sqlite() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let expected = snapshot();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .replace_synced_snapshot(expected.clone())
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(repository.load_synced_snapshot().unwrap(), expected);
        assert_eq!(
            repository.get_record("health.weight").unwrap(),
            expected.records["health"].records.first().cloned()
        );
        let revision: i64 = repository
            .connection
            .query_row(
                "SELECT data_revision FROM sync_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision, 1);
    }

    #[test]
    fn ordinary_record_write_requires_enabled_definition() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let record = snapshot().records["health"].records[0].clone();
        let mut transaction = repository.begin_transaction().unwrap();
        let error = transaction.put_record(record).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Unresolved);
        transaction.rollback().unwrap();
    }

    #[test]
    fn local_only_changes_do_not_increment_sync_revision() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .put_mission_suggestion(MissionSuggestion {
                id: "suggestion-1".to_string(),
                title: "Try walking".to_string(),
                description: None,
                difficulty: None,
                deadline: None,
                parent_mission_id: None,
                reason: None,
                generated_at: "2026-08-15T20:30:00+08:00".to_string(),
                status: MissionSuggestionStatus::Pending,
            })
            .unwrap();
        transaction.commit().unwrap();

        let revision: i64 = repository
            .connection
            .query_row(
                "SELECT data_revision FROM sync_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision, 0);
        assert_eq!(repository.list_mission_suggestions().unwrap().len(), 1);
    }

    #[test]
    fn ddl_guard_rejects_payload_for_wrong_record_kind() {
        let repository = SqliteRepository::open_in_memory().unwrap();
        repository
            .connection
            .execute(
                "INSERT INTO records(definition_id, kind) VALUES ('health.weight', 'event')",
                [],
            )
            .unwrap();
        let error = repository
            .connection
            .execute(
                "INSERT INTO scalar_records(definition_id, value_json, recorded_at)
                 VALUES ('health.weight', '72', '2026-08-15T20:30:00+08:00')",
                [],
            )
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("scalar payload requires scalar record"));
    }

    #[test]
    fn unresolved_record_is_preserved_by_snapshot_import() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut expected = snapshot();
        expected.packs.clear();
        expected.manifest.enabled_pack_ids.clear();
        expected.achievement_states = None;

        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .replace_synced_snapshot(expected.clone())
            .unwrap();
        transaction.commit().unwrap();

        let actual = repository.load_synced_snapshot().unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            actual.unresolved_record_ids(),
            BTreeSet::from(["health.weight"])
        );
    }

    #[test]
    fn local_state_survives_snapshot_replace() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let expected = snapshot();
        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .replace_synced_snapshot(expected.clone())
            .unwrap();
        transaction.commit().unwrap();

        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .set_status_dimension_selection(StatusDimensionSelection {
                position: 0,
                dimension_id: "health::vitality".to_string(),
            })
            .unwrap();
        transaction
            .set_dashboard_mission_selection(
                DashboardMissionSlot::Progress,
                DashboardMissionSelection {
                    mission_id: "mission-1".to_string(),
                    label: Some("Walk progress".to_string()),
                },
            )
            .unwrap();
        transaction
            .put_mission_suggestion(MissionSuggestion {
                id: "suggestion-1".to_string(),
                title: "Try stretching".to_string(),
                description: None,
                difficulty: None,
                deadline: None,
                parent_mission_id: None,
                reason: None,
                generated_at: "2026-08-15T20:30:00+08:00".to_string(),
                status: MissionSuggestionStatus::Pending,
            })
            .unwrap();
        transaction.commit().unwrap();

        let mut transaction = repository.begin_transaction().unwrap();
        transaction.replace_synced_snapshot(expected).unwrap();
        transaction.commit().unwrap();

        assert_eq!(repository.status_dimension_selection().unwrap().len(), 1);
        assert_eq!(repository.dashboard_mission_selections().unwrap().len(), 1);
        assert_eq!(repository.list_mission_suggestions().unwrap().len(), 1);
    }

    #[test]
    fn replacing_snapshot_can_remove_parented_missions() {
        let mut repository = SqliteRepository::open_in_memory().unwrap();
        let mut with_child = snapshot();
        with_child
            .missions
            .as_mut()
            .unwrap()
            .missions
            .push(Mission {
                id: "mission-2".to_string(),
                title: "Longer walk".to_string(),
                description: None,
                status: MissionStatus::Active,
                progress: None,
                difficulty: None,
                deadline: None,
                parent_id: Some("mission-1".to_string()),
                created_at: "2026-08-15T21:30:00+08:00".to_string(),
                completed_at: None,
            });
        let mut transaction = repository.begin_transaction().unwrap();
        transaction.replace_synced_snapshot(with_child).unwrap();
        transaction.commit().unwrap();

        let mut without_missions = snapshot();
        without_missions.missions = None;
        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .replace_synced_snapshot(without_missions.clone())
            .unwrap();
        transaction.commit().unwrap();

        assert_eq!(repository.load_synced_snapshot().unwrap(), without_missions);
    }
}
