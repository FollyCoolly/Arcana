use crate::domain::{
    is_snake_case_id, ArcanaManifest, ArcanaRepository, ArcanaRepositoryReader,
    ArcanaRepositoryTransaction, FieldDefinition, Pack, Record, RecordDefinition,
    RecordDefinitionFile, RecordFile, RepositoryError, RepositoryErrorCode, RepositoryResult,
    SyncedRepositorySnapshot, Validate, ValueType,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const ROOT_MANIFEST: &str = "arcana.json";
const ACHIEVEMENT_STATES: &str = "achievement-states.json";
const ASSISTANT_MEMORY: &str = "assistant-memory.json";
const MISSIONS: &str = "missions.json";
const PACK_MANIFEST: &str = "manifest.json";
const RECORD_DEFINITIONS: &str = "record-definitions.json";
const DERIVED_VALUES: &str = "derived-values.json";
const DIMENSIONS: &str = "dimensions.json";
const ACHIEVEMENTS: &str = "achievements.json";
const SKILLS: &str = "skills.json";

/// Converts between the synced domain snapshot and its human-readable JSON
/// directory. This layer deliberately performs no Git operations.
pub struct JsonRepositoryCodec;

impl JsonRepositoryCodec {
    /// Export a consistent Repository snapshot into a brand-new directory.
    /// Refusing an existing target keeps this codec from silently overwriting
    /// manual edits; a later synchronization layer can apply rendered files
    /// with its digest and recovery protocol.
    pub fn export_to_new_directory<R: ArcanaRepository>(
        repository: &mut R,
        target: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        let transaction = repository.begin_transaction()?;
        let snapshot = transaction.load_synced_snapshot()?;
        transaction.rollback()?;
        Self::write_snapshot_to_new_directory(snapshot, target)
    }

    pub fn write_snapshot_to_new_directory(
        snapshot: SyncedRepositorySnapshot,
        target: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        let target = target.as_ref();
        if path_entry_exists(target)? {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Conflict,
                format!("JSON export target already exists: {}", target.display()),
            ));
        }
        let target_name = target.file_name().ok_or_else(|| {
            codec_validation_error(format!(
                "JSON export target must name a directory: {}",
                target.display()
            ))
        })?;
        let parent = non_empty_parent(target);
        fs::create_dir_all(parent)
            .map_err(|error| io_error("create JSON export parent directory", error))?;

        let normalized = normalize_snapshot(snapshot)?;
        let files = render_snapshot(&normalized)?;
        let staging = TemporaryExportDirectory::create(parent, target_name)?;
        write_rendered_files(staging.path(), &files)?;

        let round_tripped = Self::read_directory(staging.path())?;
        if round_tripped != normalized {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Storage,
                "JSON export failed its semantic round-trip check",
            ));
        }

        match fs::create_dir(target) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(RepositoryError::new(
                    RepositoryErrorCode::Conflict,
                    format!("JSON export target already exists: {}", target.display()),
                ));
            }
            Err(error) => return Err(io_error("create JSON export target directory", error)),
        }
        // The target directory was reserved with create_dir, so no concurrent
        // creator can be overwritten. A later sync layer will add journaled,
        // atomic replacement for an existing managed workspace.
        write_rendered_files(target, &files)?;
        Ok(normalized)
    }

    /// Parse and validate the complete managed JSON directory. Array order is
    /// normalized before validation, so harmless manual reordering does not
    /// alter semantics; duplicates and all other schema errors still fail.
    pub fn read_directory(source: impl AsRef<Path>) -> RepositoryResult<SyncedRepositorySnapshot> {
        let source = source.as_ref();
        require_directory(source, "JSON repository root")?;
        reject_unknown_root_json(source)?;

        let manifest: ArcanaManifest = read_json(&source.join(ROOT_MANIFEST), ROOT_MANIFEST)?;
        let packs = read_packs(&source.join("packs"))?;
        let records = read_records(&source.join("records"))?;
        let achievement_states =
            read_optional_json(&source.join(ACHIEVEMENT_STATES), ACHIEVEMENT_STATES)?;
        let missions = read_optional_json(&source.join(MISSIONS), MISSIONS)?;
        let assistant_memory =
            read_optional_json(&source.join(ASSISTANT_MEMORY), ASSISTANT_MEMORY)?;

        normalize_snapshot(SyncedRepositorySnapshot {
            manifest,
            packs,
            records,
            achievement_states,
            missions,
            assistant_memory,
        })
    }

    /// Read only the JSON-owned portion of a repository. Record JSON is a
    /// synchronization representation and is intentionally ignored during
    /// normal runtime reads; live Records come from SQLite.
    pub fn read_semantic_directory(
        source: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        let source = source.as_ref();
        require_directory(source, "JSON repository root")?;
        reject_unknown_root_json(source)?;

        let manifest: ArcanaManifest = read_json(&source.join(ROOT_MANIFEST), ROOT_MANIFEST)?;
        let packs = read_packs(&source.join("packs"))?;
        let achievement_states =
            read_optional_json(&source.join(ACHIEVEMENT_STATES), ACHIEVEMENT_STATES)?;
        let missions = read_optional_json(&source.join(MISSIONS), MISSIONS)?;
        let assistant_memory =
            read_optional_json(&source.join(ASSISTANT_MEMORY), ASSISTANT_MEMORY)?;

        normalize_snapshot(SyncedRepositorySnapshot {
            manifest,
            packs,
            records: BTreeMap::new(),
            achievement_states,
            missions,
            assistant_memory,
        })
    }

    /// Persist JSON-owned entities into an existing working directory without
    /// touching `records/`, `.git`, or unrelated non-managed files.
    pub fn update_semantic_directory(
        source: impl AsRef<Path>,
        snapshot: SyncedRepositorySnapshot,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        let source = source.as_ref();
        require_directory(source, "JSON repository root")?;
        let normalized = normalize_snapshot(SyncedRepositorySnapshot {
            records: BTreeMap::new(),
            ..snapshot
        })?;
        let current = Self::read_semantic_directory(source)?;
        let current_files = render_semantic_snapshot(&current)?;
        let next_files = render_semantic_snapshot(&normalized)?;

        for (relative, content) in &next_files {
            if current_files.get(relative) == Some(content) {
                continue;
            }
            let path = repository_path(source, relative);
            atomic_replace_file(&path, content)?;
        }
        for relative in current_files.keys() {
            if next_files.contains_key(relative) {
                continue;
            }
            let path = repository_path(source, relative);
            fs::remove_file(&path)
                .map_err(|error| io_error("remove stale JSON repository file", error))?;
            remove_empty_parents(path.parent(), source)?;
        }

        let stored = Self::read_semantic_directory(source)?;
        if stored != normalized {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Storage,
                "JSON semantic update failed its semantic round-trip check",
            ));
        }
        Ok(stored)
    }

    /// Replace all synced entities through an open Repository. The concrete
    /// adapter owns the per-store commit semantics and retains local-only data.
    pub fn import_from_directory<R: ArcanaRepository>(
        repository: &mut R,
        source: impl AsRef<Path>,
    ) -> RepositoryResult<SyncedRepositorySnapshot> {
        let snapshot = Self::read_directory(source)?;
        let mut transaction = repository.begin_transaction()?;
        transaction.replace_synced_snapshot(snapshot.clone())?;
        let stored = transaction.load_synced_snapshot()?;
        if stored != snapshot {
            return Err(RepositoryError::new(
                RepositoryErrorCode::Storage,
                "repository import failed its semantic round-trip check",
            ));
        }
        transaction.commit()?;
        Ok(snapshot)
    }
}

fn render_snapshot(
    snapshot: &SyncedRepositorySnapshot,
) -> RepositoryResult<BTreeMap<String, Vec<u8>>> {
    snapshot.validate().map_err(RepositoryError::validation)?;
    let mut files = BTreeMap::new();
    insert_json(&mut files, ROOT_MANIFEST, &snapshot.manifest)?;

    if let Some(states) = &snapshot.achievement_states {
        insert_json(&mut files, ACHIEVEMENT_STATES, states)?;
    }
    if let Some(memory) = &snapshot.assistant_memory {
        insert_json(&mut files, ASSISTANT_MEMORY, memory)?;
    }
    if let Some(missions) = &snapshot.missions {
        insert_json(&mut files, MISSIONS, missions)?;
    }
    for (namespace, records) in &snapshot.records {
        insert_json(&mut files, &format!("records/{namespace}.json"), records)?;
    }
    for (pack_id, pack) in &snapshot.packs {
        let base = format!("packs/{pack_id}");
        insert_json(
            &mut files,
            &format!("{base}/{PACK_MANIFEST}"),
            &pack.manifest,
        )?;
        if let Some(definitions) = &pack.record_definitions {
            let canonical = CanonicalRecordDefinitionFile::from(definitions);
            insert_json(
                &mut files,
                &format!("{base}/{RECORD_DEFINITIONS}"),
                &canonical,
            )?;
        }
        if let Some(derived_values) = &pack.derived_values {
            insert_json(
                &mut files,
                &format!("{base}/{DERIVED_VALUES}"),
                derived_values,
            )?;
        }
        if let Some(dimensions) = &pack.dimensions {
            insert_json(&mut files, &format!("{base}/{DIMENSIONS}"), dimensions)?;
        }
        if let Some(achievements) = &pack.achievements {
            insert_json(&mut files, &format!("{base}/{ACHIEVEMENTS}"), achievements)?;
        }
        if let Some(skills) = &pack.skills {
            insert_json(&mut files, &format!("{base}/{SKILLS}"), skills)?;
        }
        for (asset_path, content) in &pack.assets {
            insert_bytes(&mut files, &format!("{base}/{asset_path}"), content.clone())?;
        }
    }
    Ok(files)
}

fn render_semantic_snapshot(
    snapshot: &SyncedRepositorySnapshot,
) -> RepositoryResult<BTreeMap<String, Vec<u8>>> {
    let mut files = render_snapshot(snapshot)?;
    files.retain(|path, _| !path.starts_with("records/"));
    Ok(files)
}

fn repository_path(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

fn atomic_replace_file(path: &Path, content: &[u8]) -> RepositoryResult<()> {
    let parent = path.parent().ok_or_else(|| {
        codec_validation_error(format!("managed path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| io_error("create managed JSON directory", error))?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| codec_validation_error("managed JSON file name must be UTF-8"))?;
    let suffix = format!("{}-{}", std::process::id(), unique_suffix());
    let temporary = parent.join(format!(".{name}.arcana-temp-{suffix}"));
    let backup = parent.join(format!(".{name}.arcana-backup-{suffix}"));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options
        .open(&temporary)
        .map_err(|error| io_error("create managed JSON temporary file", error))?;
    io::Write::write_all(&mut file, content)
        .map_err(|error| io_error("write managed JSON temporary file", error))?;
    file.sync_all()
        .map_err(|error| io_error("sync managed JSON temporary file", error))?;

    let had_existing = path.exists();
    if had_existing {
        fs::rename(path, &backup)
            .map_err(|error| io_error("stage previous managed JSON file", error))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if had_existing {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err(io_error("activate managed JSON file", error));
    }
    if had_existing {
        fs::remove_file(&backup).map_err(|error| io_error("remove managed JSON backup", error))?;
    }
    Ok(())
}

fn remove_empty_parents(mut directory: Option<&Path>, root: &Path) -> RepositoryResult<()> {
    while let Some(path) = directory {
        if path == root {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => directory = path.parent(),
            Err(error) if error.kind() == io::ErrorKind::DirectoryNotEmpty => break,
            Err(error) => return Err(io_error("remove empty JSON repository directory", error)),
        }
    }
    Ok(())
}

fn normalize_snapshot(
    mut snapshot: SyncedRepositorySnapshot,
) -> RepositoryResult<SyncedRepositorySnapshot> {
    snapshot.manifest.enabled_pack_ids.sort();
    for pack in snapshot.packs.values_mut() {
        pack.manifest.tags.sort();
        if let Some(file) = &mut pack.record_definitions {
            file.definitions
                .sort_by(|left, right| left.id().cmp(right.id()));
        }
        if let Some(file) = &mut pack.dimensions {
            file.dimensions
                .sort_by(|left, right| left.id.cmp(&right.id));
            for dimension in &mut file.dimensions {
                dimension
                    .scores
                    .sort_by(|left, right| left.id.cmp(&right.id));
            }
        }
        if let Some(file) = &mut pack.achievements {
            file.achievements
                .sort_by(|left, right| left.id.cmp(&right.id));
            for achievement in &mut file.achievements {
                achievement.tags.sort();
                achievement.prerequisites.sort();
                achievement.related_record_definition_ids.sort();
            }
        }
        if let Some(file) = &mut pack.skills {
            file.skills.sort_by(|left, right| left.id.cmp(&right.id));
            for skill in &mut file.skills {
                skill
                    .nodes
                    .sort_by(|left, right| left.achievement_id.cmp(&right.achievement_id));
            }
        }
    }
    for file in snapshot.records.values_mut() {
        file.records
            .sort_by(|left, right| left.definition_id().cmp(right.definition_id()));
        for record in &mut file.records {
            match record {
                Record::Scalar(record) => normalize_json_value(&mut record.value),
                Record::Collection(record) => {
                    record.items.sort_by(|left, right| left.id.cmp(&right.id));
                    for item in &mut record.items {
                        for value in item.fields.values_mut() {
                            normalize_json_value(value);
                        }
                    }
                }
                Record::Event(record) => {
                    record.events.sort_by(|left, right| {
                        left.occurred_at
                            .cmp(&right.occurred_at)
                            .then_with(|| left.id.cmp(&right.id))
                    });
                    for event in &mut record.events {
                        for value in event.fields.values_mut() {
                            normalize_json_value(value);
                        }
                    }
                }
            }
        }
    }
    if let Some(file) = &mut snapshot.missions {
        file.missions.sort_by(|left, right| left.id.cmp(&right.id));
    }
    if let Some(file) = &mut snapshot.assistant_memory {
        file.memories.sort_by(|left, right| left.id.cmp(&right.id));
    }
    snapshot.validate().map_err(RepositoryError::validation)?;
    Ok(snapshot)
}

fn normalize_json_value(value: &mut Value) {
    match value {
        Value::Number(number)
            if number.as_f64().is_some_and(|value| value == 0.0)
                && number.to_string().starts_with('-') =>
        {
            *number = Number::from(0);
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_json_value),
        Value::Object(values) => values.values_mut().for_each(normalize_json_value),
        _ => {}
    }
}

fn insert_json<T: Serialize>(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &T,
) -> RepositoryResult<()> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        RepositoryError::new(
            RepositoryErrorCode::Storage,
            format!("failed to serialize '{path}': {error}"),
        )
    })?;
    bytes.push(b'\n');
    insert_bytes(files, path, bytes)
}

fn insert_bytes(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    bytes: Vec<u8>,
) -> RepositoryResult<()> {
    if files.insert(path.to_string(), bytes).is_some() {
        return Err(RepositoryError::new(
            RepositoryErrorCode::Conflict,
            format!("duplicate JSON repository path '{path}'"),
        ));
    }
    Ok(())
}

fn write_rendered_files(root: &Path, files: &BTreeMap<String, Vec<u8>>) -> RepositoryResult<()> {
    for (relative, content) in files {
        let path = relative
            .split('/')
            .fold(root.to_path_buf(), |path, segment| path.join(segment));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| io_error("create JSON export directory", error))?;
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = options
            .open(&path)
            .map_err(|error| io_error("create JSON export file", error))?;
        io::Write::write_all(&mut file, content)
            .map_err(|error| io_error("write JSON export file", error))?;
    }
    Ok(())
}

fn read_packs(root: &Path) -> RepositoryResult<BTreeMap<String, Pack>> {
    if !path_entry_exists(root)? {
        return Ok(BTreeMap::new());
    }
    require_directory(root, "packs directory")?;
    let mut packs = BTreeMap::new();
    for entry in sorted_entries(root)? {
        let pack_id = utf8_file_name(&entry)?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| io_error("inspect Pack directory", error))?;
        if !metadata.file_type().is_dir() || !is_snake_case_id(&pack_id) {
            return Err(codec_validation_error(format!(
                "invalid Pack directory 'packs/{pack_id}'"
            )));
        }
        reject_unknown_pack_json(&entry.path(), &pack_id)?;
        let manifest = read_json(
            &entry.path().join(PACK_MANIFEST),
            &format!("packs/{pack_id}/{PACK_MANIFEST}"),
        )?;
        let record_definitions = read_optional_json(
            &entry.path().join(RECORD_DEFINITIONS),
            &format!("packs/{pack_id}/{RECORD_DEFINITIONS}"),
        )?;
        let derived_values = read_optional_json(
            &entry.path().join(DERIVED_VALUES),
            &format!("packs/{pack_id}/{DERIVED_VALUES}"),
        )?;
        let dimensions = read_optional_json(
            &entry.path().join(DIMENSIONS),
            &format!("packs/{pack_id}/{DIMENSIONS}"),
        )?;
        let achievements = read_optional_json(
            &entry.path().join(ACHIEVEMENTS),
            &format!("packs/{pack_id}/{ACHIEVEMENTS}"),
        )?;
        let skills = read_optional_json(
            &entry.path().join(SKILLS),
            &format!("packs/{pack_id}/{SKILLS}"),
        )?;
        let assets = read_assets(&entry.path().join("assets"))?;
        packs.insert(
            pack_id,
            Pack {
                manifest,
                record_definitions,
                derived_values,
                dimensions,
                achievements,
                skills,
                assets,
            },
        );
    }
    Ok(packs)
}

fn read_records(root: &Path) -> RepositoryResult<BTreeMap<String, RecordFile>> {
    if !path_entry_exists(root)? {
        return Ok(BTreeMap::new());
    }
    require_directory(root, "records directory")?;
    let mut records = BTreeMap::new();
    for entry in sorted_entries(root)? {
        let name = utf8_file_name(&entry)?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| io_error("inspect Record file", error))?;
        let namespace = name.strip_suffix(".json").unwrap_or_default();
        if !metadata.file_type().is_file() || !is_snake_case_id(namespace) {
            return Err(codec_validation_error(format!(
                "invalid Record file 'records/{name}'"
            )));
        }
        let file = read_json(&entry.path(), &format!("records/{name}"))?;
        records.insert(namespace.to_string(), file);
    }
    Ok(records)
}

fn read_assets(root: &Path) -> RepositoryResult<BTreeMap<String, Vec<u8>>> {
    if !path_entry_exists(root)? {
        return Ok(BTreeMap::new());
    }
    require_directory(root, "Pack assets directory")?;
    let mut assets = BTreeMap::new();
    read_assets_recursive(root, root, &mut assets)?;
    Ok(assets)
}

fn read_assets_recursive(
    asset_root: &Path,
    directory: &Path,
    assets: &mut BTreeMap<String, Vec<u8>>,
) -> RepositoryResult<()> {
    for entry in sorted_entries(directory)? {
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)
            .map_err(|error| io_error("inspect Pack asset", error))?;
        if metadata.file_type().is_symlink() {
            return Err(codec_validation_error(format!(
                "Pack asset cannot be a symbolic link: {}",
                entry_path.display()
            )));
        }
        let relative = entry_path
            .strip_prefix(asset_root)
            .map_err(|_| codec_validation_error("Pack asset escaped its assets directory"))?;
        let relative = path_to_forward_slashes(relative)?;
        let portable_path = if metadata.file_type().is_dir() {
            format!("assets/{relative}/placeholder")
        } else {
            format!("assets/{relative}")
        };
        if !crate::domain::is_portable_asset_path(&portable_path) {
            return Err(codec_validation_error(format!(
                "invalid portable Pack asset path 'assets/{relative}'"
            )));
        }
        if metadata.file_type().is_dir() {
            read_assets_recursive(asset_root, &entry_path, assets)?;
            continue;
        }
        if !metadata.file_type().is_file() {
            return Err(codec_validation_error(format!(
                "Pack asset must be a regular file: {}",
                entry_path.display()
            )));
        }
        let asset_path = format!("assets/{relative}");
        let content = fs::read(&entry_path).map_err(|error| io_error("read Pack asset", error))?;
        if assets.insert(asset_path.clone(), content).is_some() {
            return Err(codec_validation_error(format!(
                "duplicate Pack asset path '{asset_path}'"
            )));
        }
    }
    Ok(())
}

fn reject_unknown_root_json(root: &Path) -> RepositoryResult<()> {
    let known = BTreeSet::from([
        ROOT_MANIFEST,
        ACHIEVEMENT_STATES,
        ASSISTANT_MEMORY,
        MISSIONS,
    ]);
    for entry in sorted_entries(root)? {
        let name = utf8_file_name(&entry)?;
        if Path::new(&name).extension() == Some(OsStr::new("json"))
            && !known.contains(name.as_str())
        {
            return Err(codec_validation_error(format!(
                "unknown root JSON file '{name}'"
            )));
        }
    }
    Ok(())
}

fn reject_unknown_pack_json(root: &Path, pack_id: &str) -> RepositoryResult<()> {
    let known = BTreeSet::from([
        PACK_MANIFEST,
        RECORD_DEFINITIONS,
        DERIVED_VALUES,
        DIMENSIONS,
        ACHIEVEMENTS,
        SKILLS,
    ]);
    for entry in sorted_entries(root)? {
        let name = utf8_file_name(&entry)?;
        if Path::new(&name).extension() == Some(OsStr::new("json"))
            && !known.contains(name.as_str())
        {
            return Err(codec_validation_error(format!(
                "unknown Pack JSON file 'packs/{pack_id}/{name}'"
            )));
        }
    }
    Ok(())
}

fn read_optional_json<T: DeserializeOwned>(
    path: &Path,
    display_path: &str,
) -> RepositoryResult<Option<T>> {
    if !path_entry_exists(path)? {
        return Ok(None);
    }
    read_json(path, display_path).map(Some)
}

fn read_json<T: DeserializeOwned>(path: &Path, display_path: &str) -> RepositoryResult<T> {
    require_regular_file(path, display_path)?;
    let bytes = fs::read(path).map_err(|error| io_error("read JSON file", error))?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        codec_validation_error(format!("invalid JSON in '{display_path}': {error}"))
    })?;
    if contains_null(&value) {
        return Err(codec_validation_error(format!(
            "JSON null is not allowed in '{display_path}'"
        )));
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        codec_validation_error(format!("invalid schema in '{display_path}': {error}"))
    })
}

fn contains_null(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(values) => values.iter().any(contains_null),
        Value::Object(values) => values.values().any(contains_null),
        _ => false,
    }
}

fn path_entry_exists(path: &Path) -> RepositoryResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("inspect JSON repository path", error)),
    }
}

fn require_regular_file(path: &Path, display_path: &str) -> RepositoryResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!("required file '{display_path}' does not exist"),
            )
        } else {
            io_error("inspect JSON repository file", error)
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(codec_validation_error(format!(
            "'{display_path}' must be a regular file"
        )));
    }
    Ok(())
}

fn require_directory(path: &Path, description: &str) -> RepositoryResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            RepositoryError::new(
                RepositoryErrorCode::NotFound,
                format!("{description} does not exist: {}", path.display()),
            )
        } else {
            io_error("inspect JSON repository directory", error)
        }
    })?;
    if !metadata.file_type().is_dir() {
        return Err(codec_validation_error(format!(
            "{description} must be a real directory: {}",
            path.display()
        )));
    }
    Ok(())
}

fn sorted_entries(directory: &Path) -> RepositoryResult<Vec<DirEntry>> {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .map_err(|error| io_error("read JSON repository directory", error))?
        .collect::<Result<_, _>>()
        .map_err(|error| io_error("read JSON repository entry", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn utf8_file_name(entry: &DirEntry) -> RepositoryResult<String> {
    entry.file_name().into_string().map_err(|_| {
        codec_validation_error(format!(
            "JSON repository path is not valid UTF-8: {}",
            entry.path().display()
        ))
    })
}

fn path_to_forward_slashes(path: &Path) -> RepositoryResult<String> {
    let mut result = String::new();
    for component in path.components() {
        let segment = component.as_os_str().to_str().ok_or_else(|| {
            codec_validation_error(format!("Pack asset path is not UTF-8: {}", path.display()))
        })?;
        if !result.is_empty() {
            result.push('/');
        }
        result.push_str(segment);
    }
    Ok(result)
}

fn non_empty_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn codec_validation_error(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorCode::ValidationFailed, message)
}

fn io_error(action: &str, error: io::Error) -> RepositoryError {
    RepositoryError::new(
        RepositoryErrorCode::Storage,
        format!("failed to {action}: {error}"),
    )
}

struct TemporaryExportDirectory {
    path: PathBuf,
}

impl TemporaryExportDirectory {
    fn create(parent: &Path, target_name: &OsStr) -> RepositoryResult<Self> {
        for attempt in 0..100_u32 {
            let path = parent.join(format!(
                ".{}.arcana-export-{}-{}-{attempt}",
                target_name.to_string_lossy(),
                std::process::id(),
                unique_suffix()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(io_error("create temporary JSON export directory", error));
                }
            }
        }
        Err(RepositoryError::new(
            RepositoryErrorCode::Storage,
            "could not allocate a temporary JSON export directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryExportDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[derive(Serialize)]
struct CanonicalRecordDefinitionFile<'a> {
    definitions: Vec<CanonicalRecordDefinition<'a>>,
}

impl<'a> From<&'a RecordDefinitionFile> for CanonicalRecordDefinitionFile<'a> {
    fn from(file: &'a RecordDefinitionFile) -> Self {
        Self {
            definitions: file
                .definitions
                .iter()
                .map(CanonicalRecordDefinition::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum CanonicalRecordDefinition<'a> {
    Scalar(CanonicalScalarDefinition<'a>),
    Structured(CanonicalStructuredDefinition<'a>),
}

impl<'a> From<&'a RecordDefinition> for CanonicalRecordDefinition<'a> {
    fn from(definition: &'a RecordDefinition) -> Self {
        match definition {
            RecordDefinition::Scalar(definition) => Self::Scalar(CanonicalScalarDefinition {
                id: &definition.id,
                name: &definition.name,
                description: definition.description.as_deref(),
                kind: "scalar",
                value_type: definition.value_type,
                unit: definition.unit.as_deref(),
            }),
            RecordDefinition::Collection(definition) => {
                Self::Structured(CanonicalStructuredDefinition {
                    id: &definition.id,
                    name: &definition.name,
                    description: definition.description.as_deref(),
                    kind: "collection",
                    fields: &definition.fields,
                })
            }
            RecordDefinition::Event(definition) => {
                Self::Structured(CanonicalStructuredDefinition {
                    id: &definition.id,
                    name: &definition.name,
                    description: definition.description.as_deref(),
                    kind: "event",
                    fields: &definition.fields,
                })
            }
        }
    }
}

#[derive(Serialize)]
struct CanonicalScalarDefinition<'a> {
    id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    kind: &'static str,
    value_type: ValueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<&'a str>,
}

#[derive(Serialize)]
struct CanonicalStructuredDefinition<'a> {
    id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    kind: &'static str,
    fields: &'a BTreeMap<String, FieldDefinition>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AchievementDefinition, AchievementDifficulty, AchievementFile, AchievementState,
        AchievementStateFile, AchievementStatus, AssistantMemory, AssistantMemoryFile,
        AssistantMemoryKind, CollectionRecord, DerivedValueDefinition, DerivedValueFile,
        DimensionDefinition, DimensionFile, EventEntry, EventRecord, FieldDefinition, Mission,
        MissionDifficulty, MissionFile, MissionStatus, MissionSuggestion, MissionSuggestionStatus,
        PackManifest, ScalarRecord, ScalarRecordDefinition, ScoreDefinition, SkillDefinition,
        SkillFile, SkillNode, StructuredRecordDefinition, ValueType, PACK_SCHEMA_VERSION,
        SCHEMA_VERSION,
    };
    use crate::storage::DataRepository;
    use serde_json::json;

    fn sample_snapshot() -> SyncedRepositorySnapshot {
        let pack = Pack {
            manifest: PackManifest {
                schema_version: PACK_SCHEMA_VERSION,
                id: "fitness".to_string(),
                name: "Fitness".to_string(),
                description: Some("Fitness records".to_string()),
                author: Some("Alice".to_string()),
                parent_pack_id: None,
                tags: vec!["wellness".to_string(), "activity".to_string()],
            },
            record_definitions: Some(RecordDefinitionFile {
                definitions: vec![
                    RecordDefinition::Scalar(ScalarRecordDefinition {
                        id: "fitness.total_runs".to_string(),
                        name: "Total runs".to_string(),
                        description: None,
                        value_type: ValueType::Integer,
                        unit: None,
                    }),
                    RecordDefinition::Collection(StructuredRecordDefinition {
                        id: "fitness.routes".to_string(),
                        name: "Routes".to_string(),
                        description: None,
                        fields: BTreeMap::from([(
                            "name".to_string(),
                            FieldDefinition {
                                value_type: ValueType::String,
                                required: true,
                                unit: None,
                            },
                        )]),
                    }),
                    RecordDefinition::Event(StructuredRecordDefinition {
                        id: "fitness.running".to_string(),
                        name: "Running".to_string(),
                        description: None,
                        fields: BTreeMap::from([(
                            "distance".to_string(),
                            FieldDefinition {
                                value_type: ValueType::Number,
                                required: true,
                                unit: Some("km".to_string()),
                            },
                        )]),
                    }),
                ],
            }),
            derived_values: Some(DerivedValueFile {
                values: vec![DerivedValueDefinition {
                    id: "fitness.run_score".to_string(),
                    name: "Run score".to_string(),
                    description: None,
                    unit: None,
                    expression: "record('fitness.total_runs') * 10".to_string(),
                }],
            }),
            dimensions: Some(DimensionFile {
                dimensions: vec![DimensionDefinition {
                    id: "fitness::overall".to_string(),
                    name: "Overall".to_string(),
                    level_titles: [
                        "Starter".to_string(),
                        "Mover".to_string(),
                        "Runner".to_string(),
                        "Athlete".to_string(),
                        "Master".to_string(),
                    ],
                    level_thresholds: [20.0, 40.0, 60.0, 80.0],
                    scores: vec![ScoreDefinition {
                        id: "frequency".to_string(),
                        name: "Frequency".to_string(),
                        weight: 1.0,
                        expression: "record('fitness.total_runs')".to_string(),
                    }],
                }],
            }),
            achievements: Some(AchievementFile {
                achievements: vec![AchievementDefinition {
                    id: "fitness::first_run".to_string(),
                    name: "First run".to_string(),
                    description: "Complete a run".to_string(),
                    difficulty: AchievementDifficulty::Beginner,
                    tags: vec!["milestone".to_string(), "fitness".to_string()],
                    prerequisites: vec![],
                    related_record_definition_ids: vec!["fitness.running".to_string()],
                    tip: None,
                }],
            }),
            skills: Some(SkillFile {
                skills: vec![SkillDefinition {
                    id: "fitness::general".to_string(),
                    name: "Fitness".to_string(),
                    description: None,
                    level_thresholds: [10, 20, 30, 40],
                    nodes: vec![SkillNode {
                        achievement_id: "fitness::first_run".to_string(),
                        points: 40,
                    }],
                    card_image: Some("assets/card.png".to_string()),
                }],
            }),
            assets: BTreeMap::from([(
                "assets/card.png".to_string(),
                b"\x89PNG\r\n\x1a\nfixture".to_vec(),
            )]),
        };

        SyncedRepositorySnapshot {
            manifest: ArcanaManifest {
                schema_version: SCHEMA_VERSION,
                enabled_pack_ids: vec!["fitness".to_string()],
            },
            packs: BTreeMap::from([("fitness".to_string(), pack)]),
            records: BTreeMap::from([(
                "fitness".to_string(),
                RecordFile {
                    namespace: "fitness".to_string(),
                    records: vec![
                        Record::Scalar(ScalarRecord {
                            definition_id: "fitness.total_runs".to_string(),
                            value: json!(2),
                            effective_at: None,
                            recorded_at: "2026-08-15T20:30:00+08:00".to_string(),
                        }),
                        Record::Collection(CollectionRecord {
                            definition_id: "fitness.routes".to_string(),
                            items: vec![],
                        }),
                        Record::Event(EventRecord {
                            definition_id: "fitness.running".to_string(),
                            events: vec![
                                EventEntry {
                                    id: "later".to_string(),
                                    occurred_at: "2026-08-15T19:00:00+08:00".to_string(),
                                    fields: BTreeMap::from([("distance".to_string(), json!(5.2))]),
                                    recorded_at: "2026-08-15T20:31:00+08:00".to_string(),
                                },
                                EventEntry {
                                    id: "earlier".to_string(),
                                    occurred_at: "2026-08-15T08:00:00+08:00".to_string(),
                                    fields: BTreeMap::from([("distance".to_string(), json!(-0.0))]),
                                    recorded_at: "2026-08-15T20:32:00+08:00".to_string(),
                                },
                            ],
                        }),
                    ],
                },
            )]),
            achievement_states: Some(AchievementStateFile {
                states: BTreeMap::from([(
                    "fitness::first_run".to_string(),
                    AchievementState {
                        status: AchievementStatus::Achieved,
                        achieved_at: Some("2026-08".to_string()),
                    },
                )]),
            }),
            missions: Some(MissionFile {
                missions: vec![
                    Mission {
                        id: "mission-b".to_string(),
                        title: "Child".to_string(),
                        description: None,
                        status: MissionStatus::Completed,
                        progress: Some(100),
                        difficulty: Some(MissionDifficulty::B),
                        deadline: None,
                        parent_id: Some("mission-a".to_string()),
                        created_at: "2026-08-15T20:30:00+08:00".to_string(),
                        completed_at: Some("2026-08-16T20:30:00+08:00".to_string()),
                    },
                    Mission {
                        id: "mission-a".to_string(),
                        title: "Parent".to_string(),
                        description: None,
                        status: MissionStatus::Active,
                        progress: None,
                        difficulty: None,
                        deadline: Some("2026-12-31".to_string()),
                        parent_id: None,
                        created_at: "2026-08-15T20:00:00+08:00".to_string(),
                        completed_at: None,
                    },
                ],
            }),
            assistant_memory: Some(AssistantMemoryFile {
                memories: vec![
                    AssistantMemory {
                        id: "memory-b".to_string(),
                        kind: AssistantMemoryKind::Reminder,
                        content: "Remember history".to_string(),
                        created_at: "2026-08-15T21:00:00+08:00".to_string(),
                        updated_at: "2026-08-15T21:00:00+08:00".to_string(),
                    },
                    AssistantMemory {
                        id: "memory-a".to_string(),
                        kind: AssistantMemoryKind::Preference,
                        content: "Short missions".to_string(),
                        created_at: "2026-08-15T20:00:00+08:00".to_string(),
                        updated_at: "2026-08-15T20:00:00+08:00".to_string(),
                    },
                ],
            }),
        }
    }

    fn collect_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(directory).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &entry.path(), files);
                } else {
                    let relative = entry.path().strip_prefix(root).unwrap().to_path_buf();
                    files.insert(
                        path_to_forward_slashes(&relative).unwrap(),
                        fs::read(entry.path()).unwrap(),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    #[test]
    fn export_is_canonical_and_round_trips_every_synced_entity() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first");
        let normalized =
            JsonRepositoryCodec::write_snapshot_to_new_directory(sample_snapshot(), &first)
                .unwrap();
        let parsed = JsonRepositoryCodec::read_directory(&first).unwrap();
        assert_eq!(parsed, normalized);

        let definitions =
            fs::read_to_string(first.join("packs/fitness/record-definitions.json")).unwrap();
        assert!(definitions.contains(
            "\"id\": \"fitness.running\",\n      \"name\": \"Running\",\n      \"kind\": \"event\""
        ));
        assert!(!definitions.contains("\"kind\": \"event\",\n      \"id\""));

        let derived_values =
            fs::read_to_string(first.join("packs/fitness/derived-values.json")).unwrap();
        assert!(derived_values.contains("\"id\": \"fitness.run_score\""));

        let records = fs::read_to_string(first.join("records/fitness.json")).unwrap();
        assert!(records.contains("\"items\": []"));
        assert!(!records.contains("-0.0"));
        assert!(!records.contains('\r'));
        assert!(records.ends_with('\n'));
        assert_eq!(
            fs::read(first.join("packs/fitness/assets/card.png")).unwrap(),
            b"\x89PNG\r\n\x1a\nfixture"
        );

        let second = directory.path().join("second");
        JsonRepositoryCodec::write_snapshot_to_new_directory(parsed, &second).unwrap();
        assert_eq!(collect_files(&first), collect_files(&second));
    }

    #[test]
    fn repository_import_replaces_synced_data_and_retains_local_only_data() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let expected =
            JsonRepositoryCodec::write_snapshot_to_new_directory(sample_snapshot(), &source)
                .unwrap();

        let mut repository = DataRepository::open_in_memory().unwrap();
        let suggestion = MissionSuggestion {
            id: "suggestion-a".to_string(),
            title: "Try running".to_string(),
            description: None,
            difficulty: None,
            deadline: None,
            parent_mission_id: None,
            reason: None,
            generated_at: "2026-08-15T20:00:00+08:00".to_string(),
            status: MissionSuggestionStatus::Pending,
        };
        let mut transaction = repository.begin_transaction().unwrap();
        transaction
            .put_mission_suggestion(suggestion.clone())
            .unwrap();
        transaction.commit().unwrap();

        let imported =
            JsonRepositoryCodec::import_from_directory(&mut repository, &source).unwrap();
        assert_eq!(imported, expected);
        assert_eq!(repository.load_synced_snapshot().unwrap(), expected);
        assert_eq!(repository.list_mission_suggestions().unwrap(), [suggestion]);

        fs::write(
            source.join("missions.json"),
            "{\n  \"missions\": [{\"id\": \"bad\", \"title\": null}]\n}\n",
        )
        .unwrap();
        let before = repository.load_synced_snapshot().unwrap();
        let error =
            JsonRepositoryCodec::import_from_directory(&mut repository, &source).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);
        assert_eq!(repository.load_synced_snapshot().unwrap(), before);
    }

    #[test]
    fn import_rejects_unknown_json_and_export_never_overwrites() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        JsonRepositoryCodec::write_snapshot_to_new_directory(sample_snapshot(), &source).unwrap();
        fs::write(source.join("mission.json"), "{}\n").unwrap();
        let error = JsonRepositoryCodec::read_directory(&source).unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::ValidationFailed);

        let error =
            JsonRepositoryCodec::write_snapshot_to_new_directory(sample_snapshot(), &source)
                .unwrap_err();
        assert_eq!(error.code, RepositoryErrorCode::Conflict);
        assert!(source.join("mission.json").exists());
    }

    #[test]
    fn empty_optional_entities_are_omitted() {
        let mut snapshot = sample_snapshot();
        snapshot.records.clear();
        snapshot.achievement_states = None;
        snapshot.missions = None;
        snapshot.assistant_memory = None;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("minimal");
        JsonRepositoryCodec::write_snapshot_to_new_directory(snapshot, &target).unwrap();
        assert!(!target.join("records").exists());
        assert!(!target.join(ACHIEVEMENT_STATES).exists());
        assert!(!target.join(MISSIONS).exists());
        assert!(!target.join(ASSISTANT_MEMORY).exists());
    }
}
