//! RealityMod Telegram Bot.
//!
//! Starts the agent service with a Telegram channel adapter.
//! Messages from allowed Telegram users are processed by the agent
//! and replies are sent back to the same chat.
//!
//! Configuration: ~/.realitymod/agent_config.json
//!
//!   {
//!     "base_url": "https://zenmux.ai/api/anthropic",
//!     "api_key": "sk-ss-v1-xxx",
//!     "model": "anthropic/claude-sonnet-4.5",
//!     "telegram": {
//!       "token": "123456:ABC-DEF...",
//!       "allow_from": ["*"]
//!     }
//!   }
//!
//! Run:
//!   cargo run --bin agent-telegram

use reality_mod_lib::agent::{
    bus::MessageBus,
    channels::telegram,
    config::AgentConfig,
    llm, prompt,
    runner::AgentRunner,
    session::SessionStore,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    let config = match AgentConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    if config.telegram.token.is_empty() {
        eprintln!("Telegram token not configured.");
        eprintln!("Add to ~/.realitymod/agent_config.json:");
        eprintln!(r#"  "telegram": {{ "token": "123456:ABC-DEF...", "allow_from": ["*"] }}"#);
        std::process::exit(1);
    }

    log::info!("RealityMod Telegram Agent");
    log::info!("  Endpoint: {}/v1/messages", config.base_url);
    log::info!("  Model:    {}", config.model);
    log::info!("  Data:     {}", config.data_dir.display());
    log::info!(
        "  ACL:      {:?}",
        config.telegram.allow_from
    );

    let (bus, mut inbound_rx) = MessageBus::new(64);
    let inbound_tx = bus.inbound_tx.clone();
    let outbound_rx = bus.outbound_rx;

    // Start Telegram channel (consumes inbound_tx, outbound_rx)
    let tg_config = config.telegram.clone();
    let tg_handle = tokio::spawn(async move {
        telegram::run(tg_config, inbound_tx, outbound_rx).await;
    });

    // Agent loop: consume inbound messages, run LLM, push replies to outbound
    let runner = AgentRunner::new(&config);
    let sessions = Arc::new(Mutex::new(
        SessionStore::new(config.data_dir.join("sessions")),
    ));

    let agent_handle = tokio::spawn(async move {
        while let Some(msg) = inbound_rx.recv().await {
            let session_key = msg.session_key.clone();

            let history = {
                let store = sessions.lock().await;
                store.load(&session_key)
            };

            let system_prompt = prompt::build_system_prompt(&config.data_dir);
            let mut messages = history;
            messages.push(llm::Message {
                role: "user".into(),
                content: llm::Content::Text(msg.content.clone()),
            });

            match runner.run(&system_prompt, &mut messages).await {
                Ok(reply) => {
                    {
                        let store = sessions.lock().await;
                        store.save(&session_key, &messages);
                    }
                    let _ = bus
                        .outbound_tx
                        .send(reality_mod_lib::agent::bus::OutboundMessage {
                            channel: msg.channel.clone(),
                            chat_id: msg.chat_id.clone(),
                            content: reply,
                        })
                        .await;
                }
                Err(e) => {
                    log::error!("[agent] Error: {e}");
                    let _ = bus
                        .outbound_tx
                        .send(reality_mod_lib::agent::bus::OutboundMessage {
                            channel: msg.channel.clone(),
                            chat_id: msg.chat_id.clone(),
                            content: format!("Agent error: {e}"),
                        })
                        .await;
                }
            }
        }
    });

    // Wait for either task to finish (normally they run forever)
    tokio::select! {
        _ = tg_handle => log::info!("Telegram channel stopped."),
        _ = agent_handle => log::info!("Agent loop stopped."),
    }
}
