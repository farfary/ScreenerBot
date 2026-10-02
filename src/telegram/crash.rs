// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Blocking Telegram crash notification, used from panic-hook context.

use crate::i18n::{ids, UiText};
use crate::telegram::text::{locale_nonblocking, tg_with, with_icon};

/// Crash message in the Telegram language. Resolves the language without
/// waiting on the configuration lock, so it is safe to call from a panic hook.
/// The panic text is escaped by the renderer.
pub(crate) fn crash_message(location: &str, panic_message: &str) -> String {
    let locale = locale_nonblocking();
    let body = UiText::new(ids::TELEGRAM_NOTIFY_CRASH)
        .arg("location", crate::i18n::UiArg::Text(location.to_owned()))
        .arg("error", crate::i18n::UiArg::Text(panic_message.to_owned()));
    format!(
        "{}\n\n{}",
        with_icon("🚨", &tg_with(&locale, &body)),
        with_icon(
            "⚠️",
            &tg_with(&locale, &UiText::new(ids::TELEGRAM_NOTIFY_CRASH_RESTART))
        )
    )
}

/// Send crash notification directly via Telegram API (blocking, for panic context)
pub(crate) fn send_crash_notification(bot_token: &str, chat_id: &str, message: &str) {
    use std::collections::HashMap;
    use std::time::Duration;

    // Use reqwest blocking client (panic-safe, no async runtime needed)
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Could not create HTTP client for crash notification: {}", e);
            return;
        }
    };

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

    let mut params = HashMap::new();
    params.insert("chat_id", chat_id);
    params.insert("text", message);
    params.insert("parse_mode", "HTML");

    match client.post(&url).form(&params).send() {
        Ok(response) => {
            if response.status().is_success() {
                eprintln!("Crash notification sent to Telegram");
            } else {
                eprintln!(
                    "Telegram API returned error: {} - {}",
                    response.status(),
                    response.text().unwrap_or_default()
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to send crash notification: {}", e.without_url());
            eprintln!("Crash message: {message}");
        }
    }
}
