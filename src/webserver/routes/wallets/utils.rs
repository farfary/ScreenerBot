//! Utility functions for wallet route handlers

use super::types::{IMPORT_SESSIONS, MAX_IMPORT_SESSIONS, SESSION_EXPIRY_SECS};

/// Clean up expired sessions and enforce session limit
pub async fn cleanup_expired_sessions() {
    let mut sessions = IMPORT_SESSIONS.write().await;
    let now = std::time::Instant::now();

    // Remove expired sessions
    sessions.retain(|_, session| {
        now.duration_since(session.created_at).as_secs() < SESSION_EXPIRY_SECS
    });

    // Warn if approaching session limit
    if sessions.len() >= MAX_IMPORT_SESSIONS {
        crate::logger::warning(
            crate::logger::LogTag::Webserver,
            &format!(
                "Import session limit reached ({}/{}). Oldest sessions will be dropped.",
                sessions.len(),
                MAX_IMPORT_SESSIONS
            ),
        );

        // Remove oldest sessions to stay under limit
        while sessions.len() >= MAX_IMPORT_SESSIONS {
            // Find oldest session
            if let Some(oldest_id) = sessions
                .iter()
                .min_by_key(|(_, s)| s.created_at)
                .map(|(id, _)| id.clone())
            {
                sessions.remove(&oldest_id);
            } else {
                break;
            }
        }
    }
}

/// Escape a field for CSV output
pub fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}
