//! Webserver utility functions
//!
//! Helper functions for common webserver operations

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Format error response with consistent structure
pub fn error_response(
    status: StatusCode,
    code: &str,
    message: &str,
    details: Option<&str>,
) -> Response {
    let error = json!({
        "error": {
            "code": code,
            "message": message,
            "details": details,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }
    });

    (status, Json(error)).into_response()
}

/// Format success response with data
pub fn success_response<T: serde::Serialize>(data: T) -> Response {
    Json(data).into_response()
}

/// Truncate string to max length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(3);
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

/// Format duration in human-readable format
pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m {secs}s")
    } else if hours > 0 {
        format!("{hours}h {minutes}m {secs}s")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}
