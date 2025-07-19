use chrono::{ DateTime, Utc };
use tokio::sync::Notify;
use std::time::Duration;
use std::fs;
use serde_json;
use crate::trader::Position;
use crate::logger::{ log, LogTag };

/// Format a duration (from Option<DateTime<Utc>>) as a human-readable age string (y d h m s)
pub fn format_age_string(created_at: Option<DateTime<Utc>>) -> String {
    if let Some(dt) = created_at {
        let now = Utc::now();
        let mut seconds = if now > dt { (now - dt).num_seconds() } else { 0 };
        let years = seconds / 31_536_000; // 365*24*60*60
        seconds %= 31_536_000;
        let days = seconds / 86_400;
        seconds %= 86_400;
        let hours = seconds / 3_600;
        seconds %= 3_600;
        let minutes = seconds / 60;
        seconds %= 60;
        let mut parts = Vec::new();
        if years > 0 {
            parts.push(format!("{}y", years));
        }
        if days > 0 {
            parts.push(format!("{}d", days));
        }
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
        if seconds > 0 || parts.is_empty() {
            parts.push(format!("{}s", seconds));
        }
        parts.join(" ")
    } else {
        "unknown".to_string()
    }
}

/// Waits for either shutdown signal or delay. Returns true if shutdown was triggered.
pub async fn check_shutdown_or_delay(shutdown: &Notify, duration: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        _ = shutdown.notified() => true,
    }
}

/// Waits for a delay or shutdown signal, whichever comes first.
pub async fn delay_with_shutdown(shutdown: &Notify, duration: Duration) {
    tokio::select! {
        _ = tokio::time::sleep(duration) => {},
        _ = shutdown.notified() => {},
    }
}

pub fn save_positions_to_file(positions: &Vec<Position>) {
    if let Ok(json) = serde_json::to_string_pretty(positions) {
        if let Err(e) = fs::write("positions.json", json) {
            log(LogTag::Trader, "ERROR", &format!("Failed to write positions.json: {}", e));
        }
    }
}

pub fn load_positions_from_file() -> Vec<Position> {
    match fs::read_to_string("positions.json") {
        Ok(content) => {
            serde_json::from_str(&content).unwrap_or_else(|e| {
                log(LogTag::Trader, "ERROR", &format!("Failed to parse positions.json: {}", e));
                Vec::new()
            })
        }
        Err(_) => Vec::new(), // File doesn't exist yet
    }
}
