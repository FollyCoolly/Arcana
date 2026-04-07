pub mod bus;
pub mod channels;
pub mod config;
pub mod llm;
pub mod prompt;
pub mod runner;
pub mod session;
pub mod tools;

use bus::MessageBus;
use config::AgentConfig;
use runner::AgentRunner;
use session::SessionStore;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared handle for the running agent service, stored in Tauri managed state.
pub struct AgentHandle {
    pub inbound_tx: bus::InboundTx,
}

/// Start the agent service as a background tokio task.
/// Returns an AgentHandle for sending messages into the agent.
pub fn start_agent_service(config: AgentConfig) -> AgentHandle {
    let (bus, inbound_rx) = MessageBus::new(64);
    let handle = AgentHandle {
        inbound_tx: bus.inbound_tx.clone(),
    };

    let data_dir = config.data_dir.clone();
    let sessions = Arc::new(Mutex::new(SessionStore::new(data_dir.join("sessions"))));

    tokio::spawn(async move {
        agent_loop(config, bus, inbound_rx, sessions).await;
    });

    handle
}

async fn agent_loop(
    config: AgentConfig,
    bus: MessageBus,
    mut inbound_rx: bus::InboundRx,
    sessions: Arc<Mutex<SessionStore>>,
) {
    let runner = AgentRunner::new(&config);

    while let Some(msg) = inbound_rx.recv().await {
        let session_key = msg.session_key.clone();

        // Load session history
        let history = {
            let store = sessions.lock().await;
            store.load(&session_key)
        };

        // Build messages: system + history + new user message
        let system_prompt = prompt::build_system_prompt(&config.data_dir);
        let mut messages = history;
        messages.push(llm::Message {
            role: "user".into(),
            content: llm::Content::Text(msg.content.clone()),
        });

        // Run agent loop
        match runner.run(&system_prompt, &mut messages).await {
            Ok(reply) => {
                // Save conversation
                {
                    let store = sessions.lock().await;
                    store.save(&session_key, &messages);
                }
                // Send reply back
                let _ = bus.outbound_tx.send(bus::OutboundMessage {
                    channel: msg.channel.clone(),
                    chat_id: msg.chat_id.clone(),
                    content: reply,
                }).await;
            }
            Err(e) => {
                eprintln!("[agent] Error processing message: {e}");
                let _ = bus.outbound_tx.send(bus::OutboundMessage {
                    channel: msg.channel.clone(),
                    chat_id: msg.chat_id.clone(),
                    content: format!("Agent error: {e}"),
                }).await;
            }
        }
    }
}
