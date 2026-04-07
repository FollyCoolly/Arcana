use super::config::AgentConfig;
use super::llm::{Content, ContentBlock, LlmClient, Message};
use super::tools::ToolRegistry;
use std::time::Instant;

/// Agent runner: the core LLM ↔ tool-calling loop.
pub struct AgentRunner {
    client: LlmClient,
    tools: ToolRegistry,
    max_iterations: usize,
}

impl AgentRunner {
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            client: LlmClient::new(config),
            tools: ToolRegistry::new(&config.data_dir),
            max_iterations: config.max_iterations,
        }
    }

    /// Run the agent loop: send messages to LLM, execute tool calls, repeat.
    /// Returns the final text response.
    pub async fn run(
        &self,
        system: &str,
        messages: &mut Vec<Message>,
    ) -> Result<String, String> {
        let tool_defs = self.tools.definitions();
        let run_start = Instant::now();

        for iteration in 0..self.max_iterations {
            let llm_start = Instant::now();
            let response = self.client.chat(system, messages, &tool_defs).await?;
            let llm_ms = llm_start.elapsed().as_millis();

            if response.stop_reason == "end_turn" || response.tool_calls().is_empty() {
                let text = response.text().unwrap_or("").to_string();
                messages.push(Message {
                    role: "assistant".into(),
                    content: Content::Text(text.clone()),
                });

                let total_ms = run_start.elapsed().as_millis();
                eprintln!(
                    "[agent] Done in {} iteration(s), {:.1}s total (LLM {:.1}s)",
                    iteration + 1,
                    total_ms as f64 / 1000.0,
                    llm_ms as f64 / 1000.0,
                );
                return Ok(text);
            }

            // Tool use
            let tool_calls = response.tool_calls();
            let assistant_blocks: Vec<ContentBlock> = response.content.clone();

            messages.push(Message {
                role: "assistant".into(),
                content: Content::Blocks(assistant_blocks),
            });

            let tool_names: Vec<&str> = tool_calls.iter().map(|(_, name, _)| *name).collect();
            eprintln!(
                "[agent] LLM ({:.1}s) → {}",
                llm_ms as f64 / 1000.0,
                tool_names.join(", "),
            );

            // Execute tools sequentially
            let mut result_blocks = Vec::new();
            for (id, name, input) in &tool_calls {
                let tool_start = Instant::now();
                let exec_result = self.tools.execute(name, input);
                let tool_ms = tool_start.elapsed().as_millis();

                let (result, ok) = match exec_result {
                    Ok(output) => {
                        let block = ContentBlock::ToolResult {
                            tool_use_id: id.to_string(),
                            content: truncate(&output, 16000),
                            is_error: None,
                        };
                        (block, true)
                    }
                    Err(e) => {
                        let block = ContentBlock::ToolResult {
                            tool_use_id: id.to_string(),
                            content: format!("Error: {e}"),
                            is_error: Some(true),
                        };
                        (block, false)
                    }
                };

                let summary = summarize_tool_call(name, input);
                if ok {
                    eprintln!("  ✓ {name}({summary}) [{tool_ms}ms]");
                } else {
                    eprintln!("  ✗ {name}({summary}) [{tool_ms}ms]");
                }

                result_blocks.push(result);
            }

            messages.push(Message {
                role: "user".into(),
                content: Content::Blocks(result_blocks),
            });
        }

        Err(format!(
            "Agent reached maximum iterations ({}) without completing.",
            self.max_iterations
        ))
    }
}

/// Extract a short summary of what the tool call is doing from its input.
fn summarize_tool_call(name: &str, input: &serde_json::Value) -> String {
    match name {
        "get_context" => String::new(),
        "read_file" => input["path"]
            .as_str()
            .unwrap_or("?")
            .to_string(),
        "update_mission" => {
            let id = input["mission_id"].as_str().unwrap_or("?");
            let fields: Vec<&str> = input["updates"]
                .as_object()
                .map(|o| o.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();
            if fields.is_empty() {
                id.to_string()
            } else {
                format!("{id}: {}", fields.join(", "))
            }
        }
        "update_status" => {
            input["metrics"]
                .as_object()
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "?".into())
        }
        "update_achievement" => {
            let id = input["achievement_id"].as_str().unwrap_or("?");
            let status = input["status"].as_str().unwrap_or("?");
            format!("{id} → {status}")
        }
        "write_changelog" => input["summary"]
            .as_str()
            .map(|s| truncate(s, 60))
            .unwrap_or_else(|| "?".into()),
        _ => String::new(),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}…")
    }
}
