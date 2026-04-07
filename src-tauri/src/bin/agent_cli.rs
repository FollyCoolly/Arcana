//! Minimal CLI for testing the RealityMod agent without Tauri.
//!
//! Usage:
//!   cargo run --bin agent-cli
//!
//! Configuration: data/agent_config.json (see AgentConfig).
//! Env overrides: ANTHROPIC_BASE_URL, ANTHROPIC_API_KEY, REALITYMOD_MODEL, etc.
//!
//! Reads user input from stdin, sends it through the agent loop,
//! prints the reply. Type "quit" or Ctrl-D to exit.

use reality_mod_lib::agent::{config::AgentConfig, llm, prompt, runner, session};
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() {
    let config = match AgentConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            eprintln!();
            eprintln!("Create ~/.realitymod/agent_config.json:");
            eprintln!(r#"  {{"#);
            eprintln!(r#"    "base_url": "https://api.anthropic.com","#);
            eprintln!(r#"    "api_key": "sk-ant-...""#);
            eprintln!(r#"    "model": "claude-sonnet-4-20250514""#);
            eprintln!(r#"  }}"#);
            eprintln!();
            eprintln!("Or use data/agent_config.json, or env vars (ANTHROPIC_API_KEY, etc.)");
            std::process::exit(1);
        }
    };

    eprintln!("RealityMod Agent CLI");
    eprintln!("  Endpoint: {}/v1/messages", config.base_url);
    eprintln!("  Model:    {}", config.model);
    eprintln!("  Data:     {}", config.data_dir.display());
    eprintln!("  Timeout:  {}s", config.timeout_secs);
    eprintln!("  Type your message, then press Enter. Type 'quit' to exit.\n");

    let agent = runner::AgentRunner::new(&config);
    let session_store = session::SessionStore::new(config.data_dir.join("sessions"));
    let session_key = "cli:local";

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("you> ");
        stdout.flush().ok();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break; // EOF
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            break;
        }

        // Load history, append user message
        let mut messages = session_store.load(session_key);
        messages.push(llm::Message {
            role: "user".into(),
            content: llm::Content::Text(input.to_string()),
        });

        let system = prompt::build_system_prompt(&config.data_dir);

        eprint!("[thinking...] ");
        match agent.run(&system, &mut messages).await {
            Ok(reply) => {
                eprintln!("done.");
                println!("\nagent> {reply}\n");
                session_store.save(session_key, &messages);
            }
            Err(e) => {
                eprintln!("error.");
                eprintln!("Agent error: {e}\n");
            }
        }
    }

    eprintln!("\nGoodbye.");
}
