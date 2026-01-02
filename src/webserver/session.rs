//! Session management for dashboard authentication
//!
//! Provides in-memory session token storage with expiration handling.
//! Sessions are ephemeral and cleared on server restart.

use once_cell::sync::Lazy;
use rand::Rng;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::config;

/// Session data with creation and expiration timestamps
#[derive(Debug, Clone)]
pub struct Session {
    /// Unix timestamp when session was created
    pub created_at: u64,
    /// Unix timestamp when session expires (0 = never)
    pub expires_at: u64,
}

/// Global session storage
static SESSIONS: Lazy<RwLock<HashMap<String, Session>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Generate a cryptographically random 64-character alphanumeric session token
pub fn generate_session_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();

    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Create a new session with the given token
///
/// Uses session timeout from config. If timeout is 0, session never expires.
pub fn create_session(token: String) -> Session {
    let timeout = config::with_config(|cfg| cfg.webserver.auth_session_timeout_secs);
    let now = current_timestamp();

    let session = Session {
        created_at: now,
        expires_at: if timeout > 0 { now + timeout } else { 0 },
    };

    if let Ok(mut sessions) = SESSIONS.write() {
        sessions.insert(token, session.clone());
    }

    session
}

/// Validate a session token
///
/// Returns true if the token exists and has not expired.
/// Automatically removes expired sessions.
pub fn validate_session(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }

    let now = current_timestamp();

    // Check if session exists and is valid
    let valid = if let Ok(sessions) = SESSIONS.read() {
        if let Some(session) = sessions.get(token) {
            // Session is valid if it never expires (expires_at = 0) or hasn't expired yet
            session.expires_at == 0 || session.expires_at > now
        } else {
            false
        }
    } else {
        false
    };

    // If session is expired, remove it
    if !valid {
        if let Ok(mut sessions) = SESSIONS.write() {
            sessions.remove(token);
        }
    }

    valid
}

/// Revoke (remove) a session token
pub fn revoke_session(token: &str) {
    if let Ok(mut sessions) = SESSIONS.write() {
        sessions.remove(token);
    }
}

/// Remove all expired sessions
///
/// Called periodically to clean up stale sessions.
pub fn cleanup_expired_sessions() {
    let now = current_timestamp();

    if let Ok(mut sessions) = SESSIONS.write() {
        sessions.retain(|_, session| {
            // Keep if never expires (expires_at = 0) or hasn't expired yet
            session.expires_at == 0 || session.expires_at > now
        });
    }
}

/// Get the number of active sessions (for debugging/monitoring)
pub fn active_session_count() -> usize {
    SESSIONS.read().map(|s| s.len()).unwrap_or(0)
}

/// Clear all sessions (for testing or security reset)
pub fn clear_all_sessions() {
    if let Ok(mut sessions) = SESSIONS.write() {
        sessions.clear();
    }
}
