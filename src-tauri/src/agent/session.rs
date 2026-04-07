use super::llm::Message;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// JSONL-based session storage, one file per session_key.
/// Borrowed from Nanobot's session/manager.py pattern.
pub struct SessionStore {
    dir: PathBuf,
    cache: HashMap<String, Vec<Message>>,
}

impl SessionStore {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self {
            dir,
            cache: HashMap::new(),
        }
    }

    fn session_path(&self, key: &str) -> PathBuf {
        // Sanitize key for filesystem: replace colons with underscores
        let filename = key.replace(':', "_").replace('/', "_");
        self.dir.join(format!("{filename}.jsonl"))
    }

    pub fn load(&self, key: &str) -> Vec<Message> {
        if let Some(cached) = self.cache.get(key) {
            return cached.clone();
        }
        self.load_from_disk(key)
    }

    fn load_from_disk(&self, key: &str) -> Vec<Message> {
        let path = self.session_path(key);
        if !path.exists() {
            return Vec::new();
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut messages = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<Message>(line) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    eprintln!("[session] Skipping malformed line: {e}");
                }
            }
        }

        // Keep only last N messages to prevent context overflow
        trim_history(&mut messages, MAX_HISTORY_MESSAGES);
        messages
    }

    pub fn save(&self, key: &str, messages: &[Message]) {
        let path = self.session_path(key);

        // Only save the last N messages
        let start = messages.len().saturating_sub(MAX_HISTORY_MESSAGES);
        let to_save = &messages[start..];

        let mut lines = Vec::with_capacity(to_save.len());
        for msg in to_save {
            if let Ok(json) = serde_json::to_string(msg) {
                lines.push(json);
            }
        }

        let content = lines.join("\n") + "\n";
        if let Err(e) = fs::write(&path, content) {
            eprintln!("[session] Failed to save {}: {e}", path.display());
        }
    }

    pub fn clear(&mut self, key: &str) {
        self.cache.remove(key);
        let path = self.session_path(key);
        let _ = fs::remove_file(path);
    }
}

const MAX_HISTORY_MESSAGES: usize = 40;

fn trim_history(messages: &mut Vec<Message>, max: usize) {
    if messages.len() <= max {
        return;
    }

    // Find a safe cut point: don't split in the middle of a tool_use/tool_result pair.
    // Walk from the desired start to find a "user" role that isn't a tool_result block.
    let desired_start = messages.len() - max;
    let mut start = desired_start;

    for i in desired_start..messages.len() {
        if messages[i].role == "user" {
            // Check it's a real user message, not a tool_result continuation
            if let super::llm::Content::Text(_) = &messages[i].content {
                start = i;
                break;
            }
        }
    }

    messages.drain(..start);
}
