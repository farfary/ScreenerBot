// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Main update polling for Telegram bot
//!
//! Handles the polling loop for receiving and processing Telegram updates
//! when a chat_id is configured and the bot is fully connected.

use crate::config::with_config;
use crate::i18n::{ids, MessageId, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::telegram::commands::{handle_auth_attempt, handle_callback_query, handle_command};
use crate::telegram::session::get_session_manager;
use crate::telegram::text::{tg, tg_id, tg_plain_id, with_icon};
use crate::telegram::types::SessionState;
use crate::telegram::{Error, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::watch;
use tokio::task::JoinHandle;

/// Where the next `getUpdates` call resumes. Update ids are per bot, so each polling
/// task owns one that starts empty: a restart with a new bot token never confirms (and
/// so discards) the new bot's updates with the old bot's offset.
#[derive(Debug, Default)]
struct UpdateCursor {
    next: Option<i32>,
}

impl UpdateCursor {
    /// The offset to request, if any update has been handled yet.
    fn offset(&self) -> Option<i32> {
        self.next
    }

    /// Record that `update_id` was received; later requests start after it.
    fn advance(&mut self, update_id: i32) {
        self.next = Some(update_id.saturating_add(1));
    }
}

/// Start the main polling loop
///
/// This starts the command handler that listens for and processes
/// Telegram updates (messages and callback queries).
pub async fn start_polling(
    mut stop: watch::Receiver<bool>,
    running: Arc<AtomicBool>,
) -> Result<JoinHandle<()>> {
    let bot_token = with_config(|c| c.telegram.bot_token.clone());
    let chat_id = with_config(|c| c.telegram.chat_id.clone());

    if bot_token.is_empty() {
        return Err(Error::NotConfigured);
    }

    // If no chat_id, we should be in discovery mode instead
    if chat_id.is_empty() {
        return Err(Error::NotConfigured);
    }

    let bot = Bot::new(bot_token);

    running.store(true, Ordering::SeqCst);

    let handle = tokio::spawn(async move {
        logger::info(
            LogTag::Telegram,
            "Telegram command handler started (accepting all users)",
        );

        let mut cursor = UpdateCursor::default();
        loop {
            tokio::select! {
                _ = stop.wait_for(|stopped| *stopped) => {
                    logger::info(LogTag::Telegram, "Telegram command handler received its stop signal");
                    break;
                }
                _ = poll_updates(&bot, &mut cursor) => {
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
async fn poll_updates(bot: &Bot, cursor: &mut UpdateCursor) {
    // Use getUpdates with offset and timeout for proper long polling
    let mut request = bot.get_updates().timeout(30);
    if let Some(off) = cursor.offset() {
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
                cursor.advance(update.id.0 as i32);

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
                                    &format!("Error handling command '{text}': {e}"),
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
                                &format!("Error handling callback: {e}"),
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
                &format!("Error fetching Telegram updates: {e}"),
            );
        }
    }
}

/// Catalog label for a chat kind reported by the discovery poller.
fn chat_type_label(chat_type: &str) -> MessageId {
    match chat_type {
        "group" => ids::TELEGRAM_CHAT_TYPE_GROUP,
        "supergroup" => ids::TELEGRAM_CHAT_TYPE_SUPERGROUP,
        "channel" => ids::TELEGRAM_CHAT_TYPE_CHANNEL,
        _ => ids::TELEGRAM_CHAT_TYPE_PRIVATE,
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
            teloxide::types::PublicChatKind::Group => "group",
            teloxide::types::PublicChatKind::Supergroup(_) => "supergroup",
            teloxide::types::PublicChatKind::Channel(_) => "channel",
        },
    };

    // Get message preview (first 50 chars)
    let message_preview = message.text().map(|t| {
        if t.len() > 50 {
            let mut end = 47;
            while end > 0 && !t.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &t[..end])
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
        let chat_name = first_name
            .clone()
            .unwrap_or_else(|| tg_plain_id(ids::TELEGRAM_DISCOVERY_DEFAULT_NAME));
        let ack_message = format!(
            "{}\n\n{}\n\n{}",
            with_icon(
                "👋",
                &tg(&UiText::new(ids::TELEGRAM_DISCOVERY_HELLO)
                    .arg("name", UiArg::Text(chat_name.clone()))),
            ),
            with_icon("✅", &tg_id(ids::TELEGRAM_DISCOVERY_DETECTED)),
            tg(&UiText::new(ids::TELEGRAM_DISCOVERY_DETAILS)
                .arg("chat_id", UiArg::Text(chat_id.0.to_string()))
                .arg(
                    "chat_type",
                    UiArg::Nested(Box::new(UiText::new(chat_type_label(chat_type)))),
                )),
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

#[cfg(test)]
mod tests {
    use super::UpdateCursor;

    #[test]
    fn a_new_cursor_requests_no_offset() {
        assert_eq!(UpdateCursor::default().offset(), None);
    }

    #[test]
    fn the_cursor_resumes_after_the_last_update() {
        let mut cursor = UpdateCursor::default();
        cursor.advance(41);
        assert_eq!(cursor.offset(), Some(42));
        cursor.advance(57);
        assert_eq!(cursor.offset(), Some(58));
    }

    #[test]
    fn a_restarted_poller_does_not_inherit_the_previous_offset() {
        let mut previous = UpdateCursor::default();
        previous.advance(900_000_000);
        let restarted = UpdateCursor::default();
        assert_eq!(restarted.offset(), None);
    }
}
