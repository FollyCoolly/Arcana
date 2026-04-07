//! Minimal CLI for testing the RealityMod agent without Tauri.
//!
//! Usage:
//!   ANTHROPIC_API_KEY=sk-... cargo run --bin agent-cli
//!
//! Reads user input from stdin, sends it through the agent loop,
//! prints the reply. Type "quit" or Ctrl-D to exit.

use reality_mod_lib::agent::{
    bus, llm, prompt, runner, session,
};
use reality_mod_lib::storage::json_store::resolve_data_dir;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() {
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            eprintln!("Error: ANTHROPIC_API_KEY environment variable not set.");
            eprintln!("Usage: ANTHROPIC_API_KEY=sk-... cargo run --bin agent-cli");
            std::process::exit(1);
        }
    };

    let model = std::env::var("REALITYMOD_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

    let data_dir = match resolve_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("Run from the project root or ensure ./data/ exists.");
            std::process::exit(1);
        }
    };

    eprintln!("RealityMod Agent CLI");
    eprintln!("  Model: {model}");
    eprintln!("  Data:  {}", data_dir.display());
    eprintln!("  Type your message, then press Enter. Type 'quit' to exit.\n");

    let agent = runner::AgentRunner::new(&api_key, &model, &data_dir);
    let session_store = session::SessionStore::new(data_dir.join("sessions"));
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

        let system = prompt::build_system_prompt(&data_dir);

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
