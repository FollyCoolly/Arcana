use crate::agent::bus::{InboundMessage, InboundTx, OutboundRx};
use crate::agent::config::TelegramConfig;
use log::{info, warn};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ChatAction;

/// Start the Telegram bot.
///
/// This function blocks forever (long-polling loop).
/// Call it from a dedicated tokio task or thread.
///
/// Architecture (borrowed from Nanobot's telegram.py):
///   Telegram → teloxide handler → InboundMessage → agent loop
///   agent loop → OutboundMessage → outbound consumer → bot.send_message
pub async fn run(
    tg_config: TelegramConfig,
    inbound_tx: InboundTx,
    mut outbound_rx: OutboundRx,
) {
    let bot = Bot::new(&tg_config.token);

    info!("[telegram] Bot starting...");

    // Verify bot identity
    match bot.get_me().await {
        Ok(me) => {
            info!(
                "[telegram] Connected as @{} (id: {})",
                me.username(),
                me.id
            );
        }
        Err(e) => {
            warn!("[telegram] Failed to get bot info: {e}");
        }
    }

    let allow_from = Arc::new(tg_config.allow_from.clone());
    let bot_for_outbound = bot.clone();

    // Outbound consumer: send agent replies back to Telegram
    tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            let chat_id_num: i64 = match msg.chat_id.parse() {
                Ok(id) => id,
                Err(_) => {
                    warn!("[telegram] Invalid chat_id: {}", msg.chat_id);
                    continue;
                }
            };
            let chat_id = ChatId(chat_id_num);

            // Split long messages (Telegram limit: 4096 chars)
            for chunk in split_message(&msg.content, 4000) {
                if let Err(e) = bot_for_outbound.send_message(chat_id, &chunk).await {
                    warn!("[telegram] Failed to send message: {e}");
                }
            }
        }
    });

    // Inbound: long-poll for messages and forward to agent
    let handler = Update::filter_message().endpoint(
        move |bot: Bot, msg: Message| {
            let inbound_tx = inbound_tx.clone();
            let allow_from = allow_from.clone();
            async move {
                handle_message(bot, msg, inbound_tx, &allow_from).await
            }
        },
    );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    inbound_tx: InboundTx,
    allow_from: &[String],
) -> ResponseResult<()> {
    // Extract sender info
    let user = match msg.from {
        Some(ref u) => u,
        None => return Ok(()), // no sender (channel posts, etc.)
    };

    let sender_id = user.id.to_string();
    let username = user.username.clone().unwrap_or_default();

    // ACL check (borrowed from Nanobot's BaseChannel.is_allowed)
    if !is_allowed(&sender_id, &username, allow_from) {
        warn!(
            "[telegram] Denied message from {} (@{})",
            sender_id, username
        );
        return Ok(());
    }

    // Only handle text messages for now
    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => return Ok(()),
    };

    if text.is_empty() {
        return Ok(());
    }

    let chat_id = msg.chat.id.to_string();

    info!(
        "[telegram] Message from @{} in chat {}: {}",
        username,
        chat_id,
        truncate_log(&text, 80)
    );

    // Send typing indicator
    let _ = bot.send_chat_action(msg.chat.id, ChatAction::Typing).await;

    // Push to agent
    let session_key = format!("telegram:{chat_id}");
    if let Err(e) = inbound_tx
        .send(InboundMessage {
            channel: "telegram".into(),
            chat_id,
            sender_id,
            content: text,
            session_key,
        })
        .await
    {
        warn!("[telegram] Failed to enqueue message: {e}");
    }

    Ok(())
}

/// Check if a sender is allowed.
/// Empty allow_from = deny all. ["*"] = allow all.
/// Matches against user ID or username.
fn is_allowed(sender_id: &str, username: &str, allow_from: &[String]) -> bool {
    if allow_from.is_empty() {
        return false;
    }
    for entry in allow_from {
        if entry == "*" {
            return true;
        }
        if entry == sender_id {
            return true;
        }
        // Match username (with or without @)
        let entry_clean = entry.trim_start_matches('@');
        if !username.is_empty() && entry_clean.eq_ignore_ascii_case(username) {
            return true;
        }
    }
    false
}

/// Split a message into chunks that fit Telegram's 4096-char limit.
fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to split at last newline within limit
        let split_at = remaining[..max_len]
            .rfind('\n')
            .unwrap_or(max_len);

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..].trim_start_matches('\n');
    }

    chunks
}

fn truncate_log(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
