//! Blocking Telegram crash notification, used from panic-hook context.

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
            eprintln!("Failed to send crash notification: {e}");
            eprintln!("Crash message: {message}");
        }
    }
}
