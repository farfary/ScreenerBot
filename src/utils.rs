use chrono::{ DateTime, Utc };
use tokio::sync::Notify;
use std::time::Duration;
use std::fs;
use serde_json;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use crate::global::POSITIONS_FILE;

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
        if let Err(e) = fs::write(POSITIONS_FILE, json) {
            log(LogTag::Trader, "ERROR", &format!("Failed to write {}: {}", POSITIONS_FILE, e));
        }
    }
}

pub fn load_positions_from_file() -> Vec<Position> {
    match fs::read_to_string(POSITIONS_FILE) {
        Ok(content) =>
            match serde_json::from_str::<Vec<Position>>(&content) {
                Ok(positions) => positions,
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to parse {}: {}", POSITIONS_FILE, e)
                    );
                    Vec::new()
                }
            }
        Err(_) => Vec::new(), // Return empty vector if file doesn't exist
    }
}

/// Helper function to format duration in a compact way
pub fn format_duration_compact(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let total_seconds = duration.num_seconds();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m", total_seconds / 60)
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours > 0 {
            format!("{}d{}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}

/// Utility function for hex dump debugging - prints data in hex format with ASCII representation
pub fn hex_dump_data(
    data: &[u8],
    start_offset: usize,
    length: usize,
    log_callback: impl Fn(&str, &str)
) {
    let end = std::cmp::min(start_offset + length, data.len());

    for chunk_start in (start_offset..end).step_by(16) {
        let chunk_end = std::cmp::min(chunk_start + 16, end);
        let chunk = &data[chunk_start..chunk_end];

        // Format offset
        let offset_str = format!("{:08X}", chunk_start);

        // Format hex bytes
        let hex_str = chunk
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        // Pad hex string to consistent width (48 chars for 16 bytes)
        let hex_padded = format!("{:<48}", hex_str);

        // Format ASCII representation
        let ascii_str: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
            .collect();

        log_callback("DEBUG", &format!("{}: {} |{}|", offset_str, hex_padded, ascii_str));
    }
}
