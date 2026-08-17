use super::{basic_pack, BASIC_PACK_ID};
use crate::domain::{
    RepositoryError, RepositoryErrorCode, RepositoryResult, SyncedRepositorySnapshot,
};
use crate::storage::json_repository::JsonRepositoryCodec;
use crate::storage::settings::{
    default_repository_dir, default_runtime_dir, expand_tilde, ArcanaSettings,
};
use crate::storage::sqlite::RecordRepository;
use crate::storage::DataRepository;
use fs2::FileExt;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DATABASE_FILENAME: &str = "arcana.sqlite3";
pub const LOCK_FILENAME: &str = "arcana.lock";
pub const LOCAL_STATE_FILENAME: &str = "local-state.json";
const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone)]
pub struct ArcanaRuntime {
    runtime_dir: PathBuf,
    repository_dir: PathBuf,
    lock_timeout: Duration,
}

impl ArcanaRuntime {
    pub fn from_settings(settings: &ArcanaSettings) -> RepositoryResult<Self> {
        let runtime_dir = match settings.runtime_dir.as_deref() {
            Some(path) => expand_tilde(path),
            None => default_runtime_dir().ok_or_else(|| {
                RepositoryError::new(
                    RepositoryErrorCode::Storage,
                    "cannot determine the default Arcana runtime directory",
                )
            })?,
        };
        let repository_dir = match settings.repository_dir.as_deref() {
            Some(path) => expand_tilde(path),
            None => default_repository_dir().ok_or_else(|| {
                RepositoryError::new(
                    RepositoryErrorCode::Storage,
                    "cannot determine the default Arcana JSON repository directory",
                )
            })?,
        };
        Self::new_with_repository(runtime_dir, repository_dir)
    }

    pub fn new(runtime_dir: impl Into<PathBuf>) -> RepositoryResult<Self> {
        let runtime_dir = runtime_dir.into();
        let repository_dir = runtime_dir.join("repository");
        Self::new_with_repository(runtime_dir, repository_dir)
    }

    pub fn new_with_repository(
        runtime_dir: impl Into<PathBuf>,
        repository_dir: impl Into<PathBuf>,
    ) -> RepositoryResult<Self> {
        let runtime_dir = runtime_dir.into();
        let repository_dir = repository_dir.into();
        if !runtime_dir.is_absolute() {
            return Err(RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                format!(
                    "runtime directory must be absolute: {}",
                    runtime_dir.display()
                ),
            ));
        }
        if !repository_dir.is_absolute() {
            return Err(RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                format!(
                    "JSON repository directory must be absolute: {}",
                    repository_dir.display()
                ),
            ));
        }
        Ok(Self {
            runtime_dir,
            repository_dir,
            lock_timeout: DEFAULT_LOCK_TIMEOUT,
        })
    }

    #[cfg(test)]
    fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = timeout;
        self
    }

    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    pub fn database_path(&self) -> PathBuf {
        self.runtime_dir.join(DATABASE_FILENAME)
    }

    pub fn repository_dir(&self) -> &Path {
        &self.repository_dir
    }

    pub fn local_state_path(&self) -> PathBuf {
        self.runtime_dir.join(LOCAL_STATE_FILENAME)
    }

    pub fn lock_path(&self) -> PathBuf {
        self.runtime_dir.join(LOCK_FILENAME)
    }

    /// Create a brand-new runtime database and install the standard basic Pack.
    /// Existing databases and SQLite sidecars are never overwritten.
    pub fn initialize(&self) -> RepositoryResult<()> {
        std::fs::create_dir_all(&self.runtime_dir)
            .map_err(|error| io_error("create runtime directory", error))?;
        let _lock = self.acquire_lock(LockMode::Exclusive)?;
        let database_path = self.database_path();
        let occupied_paths: Vec<_> = sqlite_paths(&database_path)
            .into_iter()
            .filter(|path| path.exists())
            .collect();
        if !occupied_paths.is_empty() {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!(
                    "Arcana runtime is already initialized: {}",
                    occupied_paths[0].display()
                ),
            ));
        }

        let temporary_path = self.runtime_dir.join(format!(
            ".arcana.sqlite3.init-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let mut temporary = TemporaryDatabase::new(temporary_path.clone());
        let snapshot = if self.repository_dir.exists() {
            JsonRepositoryCodec::read_directory(&self.repository_dir)?
        } else {
            let snapshot = initial_snapshot();
            JsonRepositoryCodec::write_snapshot_to_new_directory(snapshot, &self.repository_dir)?
        };
        let mut repository = RecordRepository::open(&temporary_path)?;
        let mut transaction = repository.begin_transaction()?;
        transaction.replace_records(&snapshot.records)?;
        transaction.commit()?;
        repository.checkpoint_and_close()?;
        temporary.activate_without_overwrite(&database_path)
    }

    /// Execute a normal local command while holding the exclusive runtime lock.
    /// JSON-backed writes require process-wide serialization; Record commands
    /// use the same boundary so a composed snapshot cannot change mid-command.
    pub fn with_repository<T>(
        &self,
        action: impl FnOnce(&mut DataRepository) -> RepositoryResult<T>,
    ) -> RepositoryResult<T> {
        self.with_repository_result(action)
    }

    /// Execute a command under the normal runtime locks while allowing an
    /// application-specific error to retain structured context around an
    /// underlying RepositoryError.
    pub fn with_repository_result<T, E>(
        &self,
        action: impl FnOnce(&mut DataRepository) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<RepositoryError>,
    {
        if !self.database_path().exists() {
            return Err(E::from(RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!(
                    "Arcana runtime database does not exist: {}",
                    self.database_path().display()
                ),
            )));
        }

        let _command_lock = self.acquire_lock(LockMode::Exclusive).map_err(E::from)?;
        self.ensure_semantic_repository().map_err(E::from)?;
        RecordRepository::open(self.database_path())
            .and_then(RecordRepository::checkpoint_and_close)
            .map_err(E::from)?;
        let mut repository = DataRepository::open(
            self.database_path(),
            &self.repository_dir,
            self.local_state_path(),
        )
        .map_err(E::from)?;
        action(&mut repository)
    }

    /// Export the combined live JSON and SQLite Record state to a brand-new
    /// canonical JSON directory. This performs no Git operation and never
    /// overwrites an existing directory.
    pub fn export_json_to_new_directory(
        &self,
        target: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        self.require_initialized_database()?;
        let _lock = self.acquire_lock(LockMode::Exclusive)?;
        self.ensure_semantic_repository()?;
        let mut repository = DataRepository::open(
            self.database_path(),
            &self.repository_dir,
            self.local_state_path(),
        )?;
        JsonRepositoryCodec::export_to_new_directory(&mut repository, target)
    }

    /// Create a missing SQLite runtime or replace live semantic JSON and
    /// SQLite Records from a complete JSON directory. Runtime-local JSON is
    /// retained. Parsing, validation and activation happen under the
    /// exclusive runtime lock; Git state is intentionally ignored.
    pub fn import_json_from_directory(
        &self,
        source: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        std::fs::create_dir_all(&self.runtime_dir)
            .map_err(|error| io_error("create runtime directory", error))?;
        let _lock = self.acquire_lock(LockMode::Exclusive)?;
        let database_path = self.database_path();
        if database_path.exists() {
            if !self.repository_dir.exists() {
                let snapshot = JsonRepositoryCodec::read_directory(&source)?;
                JsonRepositoryCodec::write_snapshot_to_new_directory(
                    snapshot,
                    &self.repository_dir,
                )?;
            }
            let mut repository =
                DataRepository::open(database_path, &self.repository_dir, self.local_state_path())?;
            return JsonRepositoryCodec::import_from_directory(&mut repository, source);
        }

        if let Some(sidecar) = sqlite_sidecars(&database_path)
            .into_iter()
            .find(|path| path.exists())
        {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!(
                    "cannot create Arcana runtime while a SQLite sidecar exists: {}",
                    sidecar.display()
                ),
            ));
        }

        let temporary_path = self.runtime_dir.join(format!(
            ".arcana.sqlite3.import-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let mut temporary = TemporaryDatabase::new(temporary_path.clone());
        let snapshot = JsonRepositoryCodec::read_directory(source)?;
        if self.repository_dir.exists() {
            JsonRepositoryCodec::update_semantic_directory(&self.repository_dir, snapshot.clone())?;
        } else {
            JsonRepositoryCodec::write_snapshot_to_new_directory(
                snapshot.clone(),
                &self.repository_dir,
            )?;
        }
        let mut repository = RecordRepository::open(&temporary_path)?;
        let mut transaction = repository.begin_transaction()?;
        transaction.replace_records(&snapshot.records)?;
        transaction.commit()?;
        repository.checkpoint_and_close()?;
        temporary.activate_without_overwrite(&database_path)?;
        Ok(snapshot)
    }

    fn require_initialized_database(&self) -> RepositoryResult<()> {
        if self.database_path().exists() {
            return Ok(());
        }
        Err(RepositoryError::new(
            RepositoryErrorCode::NotFound,
            format!(
                "Arcana runtime database does not exist: {}",
                self.database_path().display()
            ),
        ))
    }

    fn ensure_semantic_repository(&self) -> RepositoryResult<()> {
        if self.repository_dir.exists() {
            let mut snapshot = JsonRepositoryCodec::read_semantic_directory(&self.repository_dir)?;
            if snapshot
                .packs
                .get(BASIC_PACK_ID)
                .is_some_and(is_unmodified_legacy_basic_pack)
            {
                snapshot
                    .packs
                    .insert(BASIC_PACK_ID.to_string(), basic_pack());
                JsonRepositoryCodec::update_semantic_directory(&self.repository_dir, snapshot)?;
            }
            return Ok(());
        }

        #[cfg(not(test))]
        if let Some((snapshot, local_state)) =
            crate::storage::sqlite::read_legacy_v1_data(self.database_path())?
        {
            JsonRepositoryCodec::write_snapshot_to_new_directory(snapshot, &self.repository_dir)?;
            crate::storage::local_state::write_local_state(&self.local_state_path(), &local_state)?;
            return Ok(());
        }

        Err(RepositoryError::new(
            RepositoryErrorCode::NotFound,
            format!(
                "Arcana JSON repository does not exist: {}",
                self.repository_dir.display()
            ),
        ))
    }

    fn acquire_lock(&self, mode: LockMode) -> RepositoryResult<RuntimeLock> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.lock_path())
            .map_err(|error| io_error("open runtime lock", error))?;
        let started = Instant::now();
        loop {
            let result = match mode {
                LockMode::Exclusive => FileExt::try_lock_exclusive(&file),
            };
            match result {
                Ok(()) => return Ok(RuntimeLock { file }),
                Err(error) if is_lock_contention(&error) => {
                    if started.elapsed() >= self.lock_timeout {
                        return Err(RepositoryError::new(
                            RepositoryErrorCode::Busy,
                            format!(
                                "Arcana runtime remained busy for {} ms",
                                self.lock_timeout.as_millis()
                            ),
                        ));
                    }
                    let remaining = self.lock_timeout.saturating_sub(started.elapsed());
                    thread::sleep(LOCK_RETRY_INTERVAL.min(remaining));
                }
                Err(error) => return Err(io_error("acquire runtime lock", error)),
            }
        }
    }
}

fn is_unmodified_legacy_basic_pack(pack: &crate::domain::Pack) -> bool {
    let mut legacy = basic_pack();
    legacy.manifest.schema_version = crate::domain::LEGACY_PACK_SCHEMA_VERSION;
    legacy.derived_values = None;
    pack == &legacy
}

fn initial_snapshot() -> SyncedRepositorySnapshot {
    SyncedRepositorySnapshot {
        manifest: crate::domain::ArcanaManifest {
            schema_version: crate::domain::SCHEMA_VERSION,
            enabled_pack_ids: vec![BASIC_PACK_ID.to_string()],
        },
        packs: std::collections::BTreeMap::from([(BASIC_PACK_ID.to_string(), basic_pack())]),
        records: std::collections::BTreeMap::new(),
        achievement_states: None,
        missions: None,
        assistant_memory: None,
    }
}

#[derive(Debug, Clone, Copy)]
enum LockMode {
    Exclusive,
}

#[derive(Debug)]
struct RuntimeLock {
    file: File,
}

impl Drop for RuntimeLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

struct TemporaryDatabase {
    path: PathBuf,
    armed: bool,
}

impl TemporaryDatabase {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn remove_sidecars(&self) -> RepositoryResult<()> {
        for path in sqlite_sidecars(&self.path) {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|error| io_error("remove temporary SQLite sidecar", error))?;
            }
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn activate_without_overwrite(&mut self, target: &Path) -> RepositoryResult<()> {
        self.remove_sidecars()?;
        std::fs::hard_link(&self.path, target)
            .map_err(|error| io_error("activate Arcana database without overwrite", error))?;
        if let Err(error) = std::fs::remove_file(&self.path) {
            let rollback_error = std::fs::remove_file(target).err();
            let message = match rollback_error {
                Some(rollback_error) => format!(
                    "failed to remove temporary Arcana database: {error}; \
                     also failed to roll back activated database: {rollback_error}"
                ),
                None => format!("failed to remove temporary Arcana database: {error}"),
            };
            return Err(RepositoryError::new(RepositoryErrorCode::Storage, message));
        }
        self.disarm();
        Ok(())
    }
}

impl Drop for TemporaryDatabase {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for path in sqlite_paths(&self.path) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn sqlite_paths(database_path: &Path) -> [PathBuf; 3] {
    [
        database_path.to_path_buf(),
        sqlite_sidecar_path(database_path, "-wal"),
        sqlite_sidecar_path(database_path, "-shm"),
    ]
}

fn sqlite_sidecars(database_path: &Path) -> [PathBuf; 2] {
    [
        sqlite_sidecar_path(database_path, "-wal"),
        sqlite_sidecar_path(database_path, "-shm"),
    ]
}

fn sqlite_sidecar_path(database_path: &Path, suffix: &str) -> PathBuf {
    let mut path = OsString::from(database_path.as_os_str());
    path.push(suffix);
    PathBuf::from(path)
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn is_lock_contention(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::WouldBlock || matches!(error.raw_os_error(), Some(11 | 33 | 36))
}

fn io_error(action: &str, error: io::Error) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::Storage,
        format!("failed to {action}: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{IncrementScalarRecord, RecordCommands, SetScalarRecord};
    use crate::domain::{
        ArcanaRepository, ArcanaRepositoryReader, ArcanaRepositoryTransaction, Pack, PackManifest,
        Record, RecordDefinition, RecordDefinitionFile, ScalarRecordDefinition, ValueType,
        SCHEMA_VERSION,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Barrier};

    fn counter_pack() -> Pack {
        Pack {
            manifest: PackManifest {
                schema_version: SCHEMA_VERSION,
                id: "counter".to_string(),
                name: "Counter".to_string(),
                description: None,
                author: None,
                parent_pack_id: None,
                tags: vec![],
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![RecordDefinition::Scalar(ScalarRecordDefinition {
                    id: "counter.value".to_string(),
                    name: "Value".to_string(),
                    description: None,
                    value_type: ValueType::Integer,
                    unit: None,
                })],
            }),
            derived_values: None,
            dimensions: None,
            achievements: None,
            skills: None,
            assets: BTreeMap::new(),
        }
    }

    #[test]
    fn initializes_basic_pack_and_supports_local_commands() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        assert!(std::fs::read_dir(runtime.runtime_dir())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".arcana.sqlite3.init-")));

        runtime
            .with_repository(|repository| {
                let snapshot = repository.load_synced_snapshot()?;
                assert_eq!(snapshot.manifest.enabled_pack_ids, [BASIC_PACK_ID]);
                assert!(snapshot.packs.contains_key(BASIC_PACK_ID));

                let mut commands = RecordCommands::new(repository);
                commands.set_scalar_at(
                    SetScalarRecord {
                        definition_id: "identity.nickname".to_string(),
                        value: json!("Alice"),
                        effective_at: None,
                    },
                    "2026-08-15T20:30:00+08:00".to_string(),
                )?;
                Ok(())
            })
            .unwrap();

        runtime
            .with_repository(|repository| {
                assert_eq!(
                    repository.get_record("identity.nickname")?,
                    Some(crate::domain::Record::Scalar(crate::domain::ScalarRecord {
                        definition_id: "identity.nickname".to_string(),
                        value: json!("Alice"),
                        effective_at: None,
                        recorded_at: "2026-08-15T20:30:00+08:00".to_string(),
                    }))
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn initialization_refuses_to_overwrite_existing_database() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        let error = runtime.initialize().unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Conflict);
    }

    #[test]
    fn upgrades_only_the_unmodified_legacy_basic_pack() {
        let directory = tempfile::tempdir().unwrap();
        let runtime_dir = directory.path().join("runtime");
        let repository_dir = directory.path().join("repository");
        let mut snapshot = initial_snapshot();
        let legacy = snapshot.packs.get_mut(BASIC_PACK_ID).unwrap();
        legacy.manifest.schema_version = crate::domain::LEGACY_PACK_SCHEMA_VERSION;
        legacy.derived_values = None;
        JsonRepositoryCodec::write_snapshot_to_new_directory(snapshot, &repository_dir).unwrap();

        let runtime = ArcanaRuntime::new_with_repository(&runtime_dir, &repository_dir).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let snapshot = repository.load_synced_snapshot()?;
                let basic = &snapshot.packs[BASIC_PACK_ID];
                assert_eq!(
                    basic.manifest.schema_version,
                    crate::domain::PACK_SCHEMA_VERSION
                );
                assert_eq!(
                    basic.derived_values.as_ref().unwrap().values[0].id,
                    "identity.game_days"
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn runtime_reads_record_definitions_from_live_json_repository() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();

        let definitions_path = runtime
            .repository_dir()
            .join("packs/basic/record-definitions.json");
        let mut definitions: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&definitions_path).unwrap()).unwrap();
        let nickname = definitions["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|definition| definition["id"] == "identity.nickname")
            .unwrap();
        nickname["name"] = json!("Preferred name");
        std::fs::write(
            &definitions_path,
            serde_json::to_vec_pretty(&definitions).unwrap(),
        )
        .unwrap();

        runtime
            .with_repository(|repository| {
                let snapshot = repository.load_synced_snapshot()?;
                let nickname = snapshot.packs[BASIC_PACK_ID]
                    .record_definitions
                    .as_ref()
                    .unwrap()
                    .definitions
                    .iter()
                    .find(|definition| definition.id() == "identity.nickname")
                    .unwrap();
                assert_eq!(nickname.name(), "Preferred name");
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn normal_open_requires_initialized_database() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        let error = runtime.with_repository(|_| Ok(())).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::NotFound);
    }

    #[test]
    fn lock_timeout_reports_busy() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime"))
            .unwrap()
            .with_lock_timeout(Duration::ZERO);
        std::fs::create_dir_all(runtime.runtime_dir()).unwrap();
        let lock = runtime.acquire_lock(LockMode::Exclusive).unwrap();
        let error = runtime.acquire_lock(LockMode::Exclusive).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Busy);
        drop(lock);
    }

    #[test]
    fn concurrent_increments_do_not_lose_updates() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                let mut transaction = repository.begin_transaction()?;
                transaction.put_pack(counter_pack())?;
                transaction.set_pack_enabled("counter", true)?;
                transaction.commit()?;
                let mut commands = RecordCommands::new(repository);
                commands.set_scalar_at(
                    SetScalarRecord {
                        definition_id: "counter.value".to_string(),
                        value: json!(0),
                        effective_at: None,
                    },
                    "2026-08-15T20:30:00+08:00".to_string(),
                )?;
                Ok(())
            })
            .unwrap();

        let barrier = Arc::new(Barrier::new(4));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let runtime = runtime.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    runtime.with_repository(|repository| {
                        RecordCommands::new(repository).increment_scalar(
                            IncrementScalarRecord {
                                definition_id: "counter.value".to_string(),
                                delta: json!(1),
                                effective_at: None,
                            },
                        )?;
                        Ok(())
                    })
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        runtime
            .with_repository(|repository| {
                let record = repository.get_record("counter.value")?.unwrap();
                let crate::domain::Record::Scalar(record) = record else {
                    panic!("counter.value must be scalar");
                };
                assert_eq!(record.value, json!(4));
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn runtime_exports_and_imports_json_without_git() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = ArcanaRuntime::new(directory.path().join("runtime")).unwrap();
        runtime.initialize().unwrap();
        runtime
            .with_repository(|repository| {
                RecordCommands::new(repository).set_scalar_at(
                    SetScalarRecord {
                        definition_id: "identity.nickname".to_string(),
                        value: json!("Alice"),
                        effective_at: None,
                    },
                    "2026-08-15T20:30:00+08:00".to_string(),
                )?;
                Ok(())
            })
            .unwrap();

        let json_directory = directory.path().join("json");
        runtime
            .export_json_to_new_directory(&json_directory)
            .unwrap();
        assert!(json_directory.join("arcana.json").is_file());
        assert!(json_directory.join("records/identity.json").is_file());

        runtime
            .with_repository(|repository| {
                RecordCommands::new(repository).set_scalar_at(
                    SetScalarRecord {
                        definition_id: "identity.nickname".to_string(),
                        value: json!("Bob"),
                        effective_at: None,
                    },
                    "2026-08-15T20:31:00+08:00".to_string(),
                )?;
                Ok(())
            })
            .unwrap();
        runtime.import_json_from_directory(&json_directory).unwrap();

        runtime
            .with_repository(|repository| {
                let Record::Scalar(nickname) = repository
                    .get_record("identity.nickname")?
                    .expect("nickname Record must exist")
                else {
                    panic!("nickname must be scalar");
                };
                assert_eq!(nickname.value, json!("Alice"));
                Ok(())
            })
            .unwrap();

        let imported_runtime =
            ArcanaRuntime::new(directory.path().join("imported-runtime")).unwrap();
        imported_runtime
            .import_json_from_directory(&json_directory)
            .unwrap();
        imported_runtime
            .with_repository(|repository| {
                let Record::Scalar(nickname) = repository
                    .get_record("identity.nickname")?
                    .expect("imported nickname Record must exist")
                else {
                    panic!("imported nickname must be scalar");
                };
                assert_eq!(nickname.value, json!("Alice"));
                Ok(())
            })
            .unwrap();
    }
}
