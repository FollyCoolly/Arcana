use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Types used throughout the agent system ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(flatten)]
    pub content: Content,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Serialize Content as the Anthropic API expects:
/// - Text -> "content": "string"
/// - Blocks -> "content": [...]
impl Content {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Content::Text(s) => Some(s),
            Content::Blocks(blocks) => {
                for block in blocks {
                    if let ContentBlock::Text { text } = block {
                        return Some(text);
                    }
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// --- Anthropic API request/response ---

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ApiMessage>,
    tools: Vec<ToolDef>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: Value,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ApiContentBlock>,
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ApiContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

pub struct LlmResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: String,
}

impl LlmResponse {
    pub fn text(&self) -> Option<&str> {
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                return Some(text);
            }
        }
        None
    }

    pub fn tool_calls(&self) -> Vec<(&str, &str, &Value)> {
        self.content
            .iter()
            .filter_map(|block| {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    Some((id.as_str(), name.as_str(), input))
                } else {
                    None
                }
            })
            .collect()
    }
}

// --- Client ---

pub struct LlmClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
}

impl LlmClient {
    pub fn new(config: &super::config::AgentConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            base_url: config.base_url.clone(),
            max_tokens: config.max_tokens,
        }
    }

    /// Call the Anthropic-compatible Messages API with tool definitions.
    pub async fn chat(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<LlmResponse, String> {
        let api_messages = self.convert_messages(messages)?;

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: system.to_string(),
            messages: api_messages,
            tools: tools.to_vec(),
        };

        let url = format!("{}/v1/messages", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {status}: {body}"));
        }

        let api_resp: ApiResponse = resp.json().await.map_err(|e| format!("Parse error: {e}"))?;

        let content = api_resp
            .content
            .into_iter()
            .map(|block| match block {
                ApiContentBlock::Text { text } => ContentBlock::Text { text },
                ApiContentBlock::ToolUse { id, name, input } => {
                    ContentBlock::ToolUse { id, name, input }
                }
            })
            .collect();

        Ok(LlmResponse {
            content,
            stop_reason: api_resp.stop_reason.unwrap_or_else(|| "end_turn".into()),
        })
    }

    fn convert_messages(&self, messages: &[Message]) -> Result<Vec<ApiMessage>, String> {
        messages
            .iter()
            .map(|msg| {
                let content = match &msg.content {
                    Content::Text(s) => Value::String(s.clone()),
                    Content::Blocks(blocks) => serde_json::to_value(blocks)
                        .map_err(|e| format!("Serialize error: {e}"))?,
                };
                Ok(ApiMessage {
                    role: msg.role.clone(),
                    content,
                })
            })
            .collect()
    }
}
