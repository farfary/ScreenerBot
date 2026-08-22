//! Panic hook — sends a Telegram crash notification when the bot panics.

use crate::config::with_config;
use std::panic;

/// Set up panic hook to send Telegram notification when bot crashes
pub fn install() {
    // Get Telegram config before setting hook (config is already loaded at this point)
    let (enabled, bot_token, chat_id) = with_config(|cfg| {
        (
            cfg.telegram.enabled,
            cfg.telegram.bot_token.clone(),
            cfg.telegram.chat_id.clone(),
        )
    });

    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Log the panic to stderr
        eprintln!("\nPANIC: {:?}\n", panic_info);

        // Try to send Telegram notification if configured
        if enabled && !bot_token.is_empty() && !chat_id.is_empty() {
            let location = panic_info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "unknown".to_owned());

            let payload = panic_info.payload();
            let panic_message = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_owned()
            };

            // Truncate message if too long
            let panic_message = if panic_message.len() > 200 {
                let mut end = 200;
                while end > 0 && !panic_message.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &panic_message[..end])
            } else {
                panic_message
            };

            let message = format!(
                "🚨 <b>Bot Crashed!</b>\n\n\
                 <b>Location:</b> <code>{}</code>\n\
                 <b>Error:</b> {}\n\n\
                 ⚠️ Please restart the bot.",
                location, panic_message
            );

            let bot_token_clone = bot_token.clone();
            let chat_id_clone = chat_id.clone();

            // Spawn a thread for blocking HTTP call (tokio runtime may be unavailable in panic)
            let handle = std::thread::spawn(move || {
                crate::telegram::crash::send_crash_notification(
                    &bot_token_clone,
                    &chat_id_clone,
                    &message,
                );
            });

            // Wait up to 5 seconds for notification to send
            let _ = handle.join();
        }

        // Call default panic handler
        default_panic(panic_info);
    }));
}
