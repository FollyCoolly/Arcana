use super::config::AgentConfig;
use super::llm::{Content, ContentBlock, LlmClient, Message};
use super::tools::ToolRegistry;

/// Agent runner: the core LLM ↔ tool-calling loop.
/// Borrowed from Nanobot's runner.py — iterate until end_turn or max iterations.
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

        for iteration in 0..self.max_iterations {
            let response = self.client.chat(system, messages, &tool_defs).await?;

            if response.stop_reason == "end_turn" || response.tool_calls().is_empty() {
                // Final response — append assistant message and return text
                let text = response.text().unwrap_or("").to_string();
                messages.push(Message {
                    role: "assistant".into(),
                    content: Content::Text(text.clone()),
                });
                return Ok(text);
            }

            // Tool use — append assistant message with tool_use blocks,
            // then execute each tool and append tool_result blocks.
            let tool_calls = response.tool_calls();
            let assistant_blocks: Vec<ContentBlock> = response.content.clone();

            messages.push(Message {
                role: "assistant".into(),
                content: Content::Blocks(assistant_blocks),
            });

            // Execute tools (sequentially to avoid concurrent JSON file writes)
            let mut result_blocks = Vec::new();
            for (id, name, input) in &tool_calls {
                let result = match self.tools.execute(name, input) {
                    Ok(output) => ContentBlock::ToolResult {
                        tool_use_id: id.to_string(),
                        content: truncate(&output, 16000),
                        is_error: None,
                    },
                    Err(e) => ContentBlock::ToolResult {
                        tool_use_id: id.to_string(),
                        content: format!("Error: {e}"),
                        is_error: Some(true),
                    },
                };
                result_blocks.push(result);
            }

            messages.push(Message {
                role: "user".into(),
                content: Content::Blocks(result_blocks),
            });

            eprintln!(
                "[agent] Iteration {}: {} tool call(s)",
                iteration + 1,
                tool_calls.len()
            );
        }

        Err(format!(
            "Agent reached maximum iterations ({}) without completing.",
            self.max_iterations
        ))
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}... [truncated, {} chars total]", &s[..max_len], s.len())
    }
}
