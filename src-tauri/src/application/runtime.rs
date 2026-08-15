use super::{basic_pack, BASIC_PACK_ID};
use crate::domain::{
    ArcanaRepository, ArcanaRepositoryTransaction, RepositoryError, RepositoryErrorCode,
    RepositoryResult, SyncedRepositorySnapshot,
};
use crate::storage::json_repository::JsonRepositoryCodec;
use crate::storage::settings::{default_runtime_dir, expand_tilde, ArcanaSettings};
use crate::storage::sqlite::SqliteRepository;
use fs2::FileExt;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DATABASE_FILENAME: &str = "arcana.sqlite3";
pub const LOCK_FILENAME: &str = "arcana.lock";
const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone)]
pub struct ArcanaRuntime {
    runtime_dir: PathBuf,
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
        Self::new(runtime_dir)
    }

    pub fn new(runtime_dir: impl Into<PathBuf>) -> RepositoryResult<Self> {
        let runtime_dir = runtime_dir.into();
        if !runtime_dir.is_absolute() {
            return Err(RepositoryError::new(
                RepositoryErrorCode::ValidationFailed,
                format!(
                    "runtime directory must be absolute: {}",
                    runtime_dir.display()
                ),
            ));
        }
        Ok(Self {
            runtime_dir,
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
        let mut repository = SqliteRepository::open(&temporary_path)?;
        let mut transaction = repository.begin_transaction()?;
        transaction.put_pack(basic_pack())?;
        transaction.set_pack_enabled(BASIC_PACK_ID, true)?;
        transaction.commit()?;
        repository.checkpoint_and_close()?;
        temporary.activate_without_overwrite(&database_path)
    }

    /// Execute a normal local command while holding the shared runtime lock.
    /// A short exclusive preflight applies/validates compiled SQLite migrations
    /// before the command opens its shared-lock connection.
    pub fn with_repository<T>(
        &self,
        action: impl FnOnce(&mut SqliteRepository) -> RepositoryResult<T>,
    ) -> RepositoryResult<T> {
        if !self.database_path().exists() {
            return Err(RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!(
                    "Arcana runtime database does not exist: {}",
                    self.database_path().display()
                ),
            ));
        }

        {
            let _migration_lock = self.acquire_lock(LockMode::Exclusive)?;
            SqliteRepository::open(self.database_path())?.checkpoint_and_close()?;
        }

        let _command_lock = self.acquire_lock(LockMode::Shared)?;
        let mut repository = SqliteRepository::open(self.database_path())?;
        action(&mut repository)
    }

    /// Export the current SQLite state to a brand-new canonical JSON
    /// directory. This performs no Git operation and never overwrites an
    /// existing directory.
    pub fn export_json_to_new_directory(
        &self,
        target: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        self.require_initialized_database()?;
        let _lock = self.acquire_lock(LockMode::Exclusive)?;
        let mut repository = SqliteRepository::open(self.database_path())?;
        JsonRepositoryCodec::export_to_new_directory(&mut repository, target)
    }

    /// Create a missing SQLite runtime or replace its synced entities from a
    /// complete JSON directory. Existing local-only tables are retained.
    /// Parsing, validation and activation happen under the exclusive runtime
    /// lock; Git state is intentionally ignored.
    pub fn import_json_from_directory(
        &self,
        source: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        std::fs::create_dir_all(&self.runtime_dir)
            .map_err(|error| io_error("create runtime directory", error))?;
        let _lock = self.acquire_lock(LockMode::Exclusive)?;
        let database_path = self.database_path();
        if database_path.exists() {
            let mut repository = SqliteRepository::open(database_path)?;
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
        let mut repository = SqliteRepository::open(&temporary_path)?;
        let snapshot = JsonRepositoryCodec::import_from_directory(&mut repository, source)?;
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
                LockMode::Shared => FileExt::try_lock_shared(&file),
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

#[derive(Debug, Clone, Copy)]
enum LockMode {
    Shared,
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
        ArcanaRepositoryReader, Pack, PackManifest, Record, RecordDefinition, RecordDefinitionFile,
        ScalarRecordDefinition, ValueType, SCHEMA_VERSION,
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
