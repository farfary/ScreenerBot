//! Main update polling for Telegram bot
//!
//! Handles the polling loop for receiving and processing Telegram updates
//! when a chat_id is configured and the bot is fully connected.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::commands::{handle_auth_attempt, handle_callback_query, handle_command};
use crate::telegram::discovery;
use crate::telegram::session::get_session_manager;
use crate::telegram::types::SessionState;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Track last processed update ID for proper offset handling
static LAST_UPDATE_ID: AtomicI32 = AtomicI32::new(0);

/// Start the main polling loop
///
/// This starts the command handler that listens for and processes
/// Telegram updates (messages and callback queries).
pub async fn start_polling(
    shutdown: Arc<Notify>,
    running: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, String> {
    let bot_token = with_config(|c| c.telegram.bot_token.clone());
    let chat_id = with_config(|c| c.telegram.chat_id.clone());

    if bot_token.is_empty() {
        return Err("Bot token is empty".to_string());
    }

    // If no chat_id, we should be in discovery mode instead
    if chat_id.is_empty() {
        return Err("Chat ID is empty - use discovery mode first".to_string());
    }

    let bot = Bot::new(bot_token);

    running.store(true, Ordering::SeqCst);

    let handle = tokio::spawn(async move {
        logger::info(
            LogTag::Telegram,
            "Telegram command handler started (accepting all users)",
        );

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::Telegram, "Telegram command handler received shutdown signal");
                    break;
                }
                _ = poll_updates(&bot) => {
                    // Continue polling
                }
            }
        }

        running.store(false, Ordering::SeqCst);
        logger::info(LogTag::Telegram, "Telegram command handler stopped");
    });

    Ok(handle)
}

/// Poll and process Telegram updates
async fn poll_updates(bot: &Bot) {
    // Get offset - start from last processed + 1 to get new updates only
    let offset = LAST_UPDATE_ID.load(Ordering::SeqCst);
    let offset_param = if offset > 0 { Some(offset) } else { None };

    // Use getUpdates with offset and timeout for proper long polling
    let mut request = bot.get_updates().timeout(30);
    if let Some(off) = offset_param {
        request = request.offset(off);
    }

    match request.await {
        Ok(updates) => {
            if !updates.is_empty() {
                logger::debug(
                    LogTag::Telegram,
                    &format!("Received {} updates from Telegram", updates.len()),
                );
            }
            for update in updates {
                // Track the update ID for offset
                let update_id = update.id.0 as i32;
                LAST_UPDATE_ID.store(update_id + 1, Ordering::SeqCst);

                match update.kind {
                    teloxide::types::UpdateKind::Message(message) => {
                        // Extract user info from message.from (the sender)
                        let (user_id, username, first_name) = match &message.from {
                            Some(from) => (
                                from.id.0 as i64,
                                from.username.clone(),
                                Some(from.first_name.clone()),
                            ),
                            None => continue, // Skip messages without sender
                        };

                        let chat_id = message.chat.id;

                        // Log received message
                        logger::debug(
                            LogTag::Telegram,
                            &format!(
                                "Message from user {} in chat {}: {:?}",
                                username.as_deref().unwrap_or("unknown"),
                                chat_id.0,
                                message.text().unwrap_or("<no text>")
                            ),
                        );

                        let manager = get_session_manager();

                        // Check if in discovery mode - capture chats before normal processing
                        if manager.is_discovery_active() {
                            handle_discovery_message(bot, &message, user_id, username, first_name)
                                .await;
                            continue;
                        }

                        // Get or create session with user info
                        let session = manager
                            .get_or_create_session(user_id, chat_id.0, username, first_name)
                            .await;

                        // Handle text messages
                        if let Some(text) = message.text() {
                            let trimmed = text.trim();

                            // Check if in auth flow (awaiting TOTP)
                            if matches!(session.state, SessionState::AwaitingTotp) {
                                handle_auth_attempt(bot, chat_id, user_id, trimmed).await;
                                continue; // Skip normal command processing
                            }

                            // Normal command processing
                            if let Err(e) = handle_command(bot, chat_id, user_id, trimmed).await {
                                logger::error(
                                    LogTag::Telegram,
                                    &format!("Error handling command '{}': {}", text, e),
                                );
                            }
                        }
                    }
                    teloxide::types::UpdateKind::CallbackQuery(query) => {
                        // Extract user_id from callback query
                        let user_id = query.from.id.0 as i64;
                        let chat_id = query
                            .message
                            .as_ref()
                            .map(|m| m.chat().id)
                            .unwrap_or(teloxide::types::ChatId(user_id));

                        // Handle callback queries (button clicks)
                        if let Err(e) = handle_callback_query(bot, chat_id, user_id, query).await {
                            logger::error(
                                LogTag::Telegram,
                                &format!("Error handling callback: {}", e),
                            );
                        }
                    }
                    _ => {
                        // Ignore other update types
                    }
                }
            }
        }
        Err(e) => {
            // Log error but don't spam - connection issues are normal
            logger::debug(
                LogTag::Telegram,
                &format!("Error fetching Telegram updates: {}", e),
            );
        }
    }
}

/// Handle a message received during discovery mode
async fn handle_discovery_message(
    bot: &Bot,
    message: &teloxide::types::Message,
    user_id: i64,
    username: Option<String>,
    first_name: Option<String>,
) {
    let chat_id = message.chat.id;
    let manager = get_session_manager();

    // Get chat type
    let chat_type = match message.chat.kind {
        teloxide::types::ChatKind::Private(_) => "private",
        teloxide::types::ChatKind::Public(ref p) => match p.kind {
            teloxide::types::PublicChatKind::Group(_) => "group",
            teloxide::types::PublicChatKind::Supergroup(_) => "supergroup",
            teloxide::types::PublicChatKind::Channel(_) => "channel",
        },
    };

    // Get message preview (first 50 chars)
    let message_preview = message.text().map(|t| {
        if t.len() > 50 {
            format!("{}...", &t[..47])
        } else {
            t.to_string()
        }
    });

    // Add to discovered chats
    let is_new = manager
        .add_discovered_chat(
            chat_id.0,
            user_id,
            username.clone(),
            first_name.clone(),
            chat_type.to_string(),
            message_preview,
        )
        .await;

    // Send acknowledgment if this is a new chat
    if is_new {
        let chat_name = first_name.as_deref().unwrap_or("User");
        let ack_message = format!(
            "👋 Hello {}!\n\n\
            ✅ <b>Chat detected!</b>\n\n\
            Chat ID: <code>{}</code>\n\
            Type: {}\n\n\
            Please go to the ScreenerBot dashboard and click on this chat to select it.",
            chat_name, chat_id.0, chat_type
        );

        let _ = bot
            .send_message(chat_id, ack_message)
            .parse_mode(ParseMode::Html)
            .await;

        logger::info(
            LogTag::Telegram,
            &format!(
                "Discovered chat: {} ({}) - type: {}",
                chat_name, chat_id.0, chat_type
            ),
        );
    }
}
