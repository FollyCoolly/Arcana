use std::path::{Path, PathBuf};

/// Validate that a relative path resolves to a location within the sandbox (data_dir).
/// Prevents path traversal, absolute paths, and symlink escapes.
pub fn sandbox_path(data_dir: &Path, rel_path: &str) -> Result<PathBuf, String> {
    if Path::new(rel_path).is_absolute() {
        return Err("Absolute paths not allowed".into());
    }
    if rel_path.contains("..") {
        return Err("Path traversal not allowed".into());
    }

    let joined = data_dir.join(rel_path);
    let canon_dir = data_dir
        .canonicalize()
        .map_err(|e| format!("Cannot resolve data dir: {e}"))?;
    let canon_path = joined
        .canonicalize()
        .map_err(|_| format!("File not found: {rel_path}"))?;

    if !canon_path.starts_with(&canon_dir) {
        return Err("Access denied: path escapes data directory".into());
    }

    Ok(canon_path)
}

pub fn read_sandboxed_file(data_dir: &Path, rel_path: &str) -> Result<String, String> {
    let safe_path = sandbox_path(data_dir, rel_path)?;
    std::fs::read_to_string(&safe_path).map_err(|e| format!("Failed to read {rel_path}: {e}"))
}
