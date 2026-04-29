use crate::storage::json_store::{read_json_file, resolve_data_dir};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Agent configuration.
///
/// Loaded by merging multiple sources (later overrides earlier):
///   1. Defaults
///   2. ~/.arcana/agent_config.json   (user-level: API keys, base_url)
///   3. data/agent_config.json            (project-level: model, iterations)
///   4. Environment variables             (temporary overrides)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Anthropic-compatible API base URL (no trailing slash).
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// API key or auth token.
    #[serde(default)]
    pub api_key: String,

    /// Model identifier.
    #[serde(default = "default_model")]
    pub model: String,

    /// Max tokens for LLM response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Max tool-calling iterations per request.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// Telegram channel configuration.
    #[serde(default)]
    pub telegram: TelegramConfig,

    /// Resolved data directory (not serialized in config file).
    #[serde(skip)]
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot token from @BotFather.
    #[serde(default)]
    pub token: String,

    /// Allowed sender IDs. Empty = deny all, ["*"] = allow all.
    /// Use Telegram user ID (numeric string) or "user_id|username" format.
    #[serde(default)]
    pub allow_from: Vec<String>,
}

fn default_base_url() -> String {
    "https://api.anthropic.com".to_string()
}
fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}
fn default_max_tokens() -> u32 {
    8192
}
fn default_max_iterations() -> usize {
    20
}
fn default_timeout_secs() -> u64 {
    120
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            api_key: String::new(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            max_iterations: default_max_iterations(),
            timeout_secs: default_timeout_secs(),
            telegram: TelegramConfig::default(),
            data_dir: PathBuf::new(),
        }
    }
}

/// Partial config for merging — all fields optional.
/// Only non-null, non-empty fields override the base config.
#[derive(Debug, Deserialize)]
struct PartialConfig {
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    max_tokens: Option<u32>,
    max_iterations: Option<usize>,
    timeout_secs: Option<u64>,
    telegram: Option<TelegramConfig>,
}

impl AgentConfig {
    /// Load config by merging: defaults → ~/.arcana/ → data/ → env vars.
    pub fn load() -> Result<Self, String> {
        let data_dir = resolve_data_dir()?;
        let mut config = AgentConfig::default();
        config.data_dir = data_dir.clone();

        // Layer 1: ~/.arcana/agent_config.json (user-level)
        if let Some(user_path) = user_config_path() {
            if user_path.exists() {
                let partial: PartialConfig = read_json_file(&user_path)?;
                config.merge(&partial);
            }
        }

        // Layer 2: data/agent_config.json (project-level)
        let project_path = data_dir.join("agent_config.json");
        if project_path.exists() {
            let partial: PartialConfig = read_json_file(&project_path)?;
            config.merge(&partial);
        }

        // Layer 3: environment variable overrides
        config.apply_env();

        // Validate
        if config.api_key.is_empty() {
            return Err(format!(
                "API key not configured. Set it in:\n\
                 \x20 1. ~/.arcana/agent_config.json  (recommended)\n\
                 \x20 2. data/agent_config.json\n\
                 \x20 3. ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN env var"
            ));
        }

        // Normalize: strip trailing slash from base_url
        config.base_url = config.base_url.trim_end_matches('/').to_string();

        Ok(config)
    }

    /// Merge non-empty fields from a partial config.
    fn merge(&mut self, partial: &PartialConfig) {
        if let Some(v) = &partial.base_url {
            if !v.is_empty() {
                self.base_url = v.clone();
            }
        }
        if let Some(v) = &partial.api_key {
            if !v.is_empty() {
                self.api_key = v.clone();
            }
        }
        if let Some(v) = &partial.model {
            if !v.is_empty() {
                self.model = v.clone();
            }
        }
        if let Some(v) = partial.max_tokens {
            self.max_tokens = v;
        }
        if let Some(v) = partial.max_iterations {
            self.max_iterations = v;
        }
        if let Some(v) = partial.timeout_secs {
            self.timeout_secs = v;
        }
        if let Some(v) = &partial.telegram {
            if !v.token.is_empty() {
                self.telegram.token = v.token.clone();
            }
            if !v.allow_from.is_empty() {
                self.telegram.allow_from = v.allow_from.clone();
            }
        }
    }

    /// Apply environment variable overrides.
    fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("ANTHROPIC_BASE_URL") {
            if !v.is_empty() {
                self.base_url = v;
            }
        }
        // ANTHROPIC_AUTH_TOKEN first, ANTHROPIC_API_KEY second (higher priority)
        if let Ok(v) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
            if !v.is_empty() {
                self.api_key = v;
            }
        }
        if let Ok(v) = std::env::var("ANTHROPIC_API_KEY") {
            if !v.is_empty() {
                self.api_key = v;
            }
        }
        if let Ok(v) = std::env::var("ARCANA_MODEL") {
            if !v.is_empty() {
                self.model = v;
            }
        }
        if let Ok(v) = std::env::var("ARCANA_MAX_TOKENS") {
            if let Ok(n) = v.parse() {
                self.max_tokens = n;
            }
        }
        if let Ok(v) = std::env::var("ARCANA_TIMEOUT_SECS") {
            if let Ok(n) = v.parse() {
                self.timeout_secs = n;
            }
        }
        // Telegram token: TELOXIDE_TOKEN (teloxide convention) or ARCANA_TELEGRAM_TOKEN
        if let Ok(v) = std::env::var("TELOXIDE_TOKEN") {
            if !v.is_empty() {
                self.telegram.token = v;
            }
        }
        if let Ok(v) = std::env::var("ARCANA_TELEGRAM_TOKEN") {
            if !v.is_empty() {
                self.telegram.token = v;
            }
        }
    }
}

/// Resolve ~/.arcana/agent_config.json path.
fn user_config_path() -> Option<PathBuf> {
    crate::storage::settings::home_dir().map(|h| h.join(".arcana").join("agent_config.json"))
}
