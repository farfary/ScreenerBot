//! Telegram session manager for tracking connected users
//!
//! Provides session tracking, authentication, and chat discovery for Telegram bot users.

use crate::config::with_config;
use crate::telegram::types::{DiscoveredChat, SessionState, TelegramSession};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

// ============================================================================
// GLOBAL SESSION MANAGER
// ============================================================================

/// Thread-safe session manager for Telegram users
pub struct TelegramSessionManager {
    sessions: Arc<RwLock<HashMap<i64, TelegramSession>>>,
    discovered_chats: Arc<RwLock<Vec<DiscoveredChat>>>,
    discovery_active: Arc<AtomicBool>,
}

impl TelegramSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            discovered_chats: Arc::new(RwLock::new(Vec::new())),
            discovery_active: Arc::new(AtomicBool::new(false)),
        }
    }

    // ========================================================================
    // AUTHENTICATION
    // ========================================================================

    /// Start the login flow for a session (transition to AwaitingTotp)
    pub async fn start_login(&self, user_id: i64) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&user_id).ok_or("Session not found")?;

        // Check if locked
        if let SessionState::Locked { until } = session.state {
            if Instant::now() < until {
                let remaining = until.saturating_duration_since(Instant::now()).as_secs();
                return Err(format!(
                    "Account locked. Try again in {} seconds.",
                    remaining
                ));
            }
        }

        session.state = SessionState::AwaitingTotp;
        session.touch();
        Ok(())
    }

    /// Verify TOTP code for a user session (uses lockscreen TOTP secret)
    /// Returns Ok(true) if code matches, Ok(false) if wrong, Err if locked or no TOTP configured
    pub async fn verify_totp(&self, user_id: i64, code: &str) -> Result<bool, String> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&user_id).ok_or("Session not found")?;

        // Check if locked
        if let SessionState::Locked { until } = session.state {
            if Instant::now() < until {
                let remaining = until.saturating_duration_since(Instant::now()).as_secs();
                return Err(format!(
                    "Account locked. Try again in {} seconds.",
                    remaining
                ));
            }
            // Lock expired, allow retry
            session.state = SessionState::AwaitingTotp;
            session.failed_attempts = 0;
        }

        // Must be in AwaitingTotp state
        if session.state != SessionState::AwaitingTotp {
            return Err("Not awaiting TOTP verification".to_string());
        }

        // Get TOTP secret from WEBSERVER config (shared with lockscreen)
        let totp_secret = with_config(|c| c.webserver.auth_totp_secret.clone());
        if totp_secret.is_empty() {
            return Err("2FA not configured. Enable 2FA in Security settings first.".to_string());
        }

        // Verify TOTP code
        if crate::webserver::totp::verify_totp(&totp_secret, code)? {
            // Success - activate session
            session.state = SessionState::Active;
            session.failed_attempts = 0;
            session.touch();
            Ok(true)
        } else {
            // Wrong code
            session.failed_attempts += 1;
            let max_attempts = with_config(|c| c.telegram.max_failed_attempts) as u32;
            if session.failed_attempts >= max_attempts {
                let lockout_mins = with_config(|c| c.telegram.lockout_minutes) as u64;
                session.state = SessionState::Locked {
                    until: Instant::now() + std::time::Duration::from_secs(lockout_mins * 60),
                };
            }
            session.touch();
            Ok(false)
        }
    }

    // ========================================================================
    // SESSION MANAGEMENT
    // ========================================================================

    /// Get or create a session for a user
    pub async fn get_or_create_session(
        &self,
        user_id: i64,
        chat_id: i64,
        username: Option<String>,
        first_name: Option<String>,
    ) -> TelegramSession {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.get_mut(&user_id) {
            session.touch();
            return session.clone();
        }

        let session = TelegramSession::new(user_id, chat_id, username, first_name);
        sessions.insert(user_id, session.clone());
        session
    }

    /// Get chat IDs of all sessions (for broadcasting notifications)
    pub async fn get_authenticated_chat_ids(&self) -> Vec<i64> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.is_authenticated())
            .map(|s| s.chat_id)
            .collect()
    }

    /// Get session by user ID
    pub async fn get_session(&self, user_id: i64) -> Option<TelegramSession> {
        let sessions = self.sessions.read().await;
        sessions.get(&user_id).cloned()
    }

    /// Update session activity
    pub async fn touch_session(&self, user_id: i64) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&user_id) {
            session.touch();
        }
    }

    /// Revoke a session completely
    pub async fn revoke_session(&self, user_id: i64) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&user_id);
    }

    /// Get all active sessions
    pub async fn get_all_sessions(&self) -> Vec<TelegramSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    // ========================================================================
    // SESSION STATE MANAGEMENT
    // ========================================================================

    /// Set the state of a session
    pub async fn set_session_state(&self, user_id: i64, state: SessionState) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&user_id) {
            session.state = state;
            session.touch();
        }
    }

    /// Mark a session as active (authenticated)
    pub async fn authenticate_session(&self, user_id: i64) {
        self.set_session_state(user_id, SessionState::Active).await;
    }

    /// Invalidate a session (mark as expired, require re-login)
    pub async fn invalidate_session(&self, user_id: i64) {
        self.set_session_state(user_id, SessionState::Expired).await;
    }

    /// Check if a session has expired and update state accordingly
    pub async fn check_session_expired(&self, user_id: i64) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&user_id) {
            if session.state == SessionState::Active {
                let timeout_mins = with_config(|c| c.telegram.session_timeout_minutes) as u64;
                if session.last_activity.elapsed()
                    > std::time::Duration::from_secs(timeout_mins * 60)
                {
                    session.state = SessionState::Expired;
                    return true;
                }
            }
        }
        false
    }

    /// Check if session is active (not expired)
    pub async fn is_session_active(&self, user_id: i64) -> bool {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(&user_id) {
            matches!(session.state, SessionState::Active)
        } else {
            false
        }
    }

    // ========================================================================
    // CHAT DISCOVERY
    // ========================================================================

    /// Add a discovered chat during discovery mode
    /// Returns true if this is a new chat, false if already discovered
    pub async fn add_discovered_chat(
        &self,
        chat_id: i64,
        user_id: i64,
        username: Option<String>,
        first_name: Option<String>,
        chat_type: String,
        message_preview: Option<String>,
    ) -> bool {
        let mut chats = self.discovered_chats.write().await;
        // Check if already exists
        if chats.iter().any(|c| c.chat_id == chat_id) {
            return false;
        }
        chats.push(DiscoveredChat {
            chat_id,
            user_id,
            username,
            first_name,
            chat_type,
            message_preview,
            discovered_at: Instant::now(),
        });
        true
    }

    /// Get all discovered chats
    pub async fn get_discovered_chats(&self) -> Vec<DiscoveredChat> {
        self.discovered_chats.read().await.clone()
    }

    /// Clear all discovered chats
    pub async fn clear_discovered_chats(&self) {
        self.discovered_chats.write().await.clear();
    }

    /// Select a discovered chat by ID
    pub async fn select_discovered_chat(&self, chat_id: i64) -> Option<DiscoveredChat> {
        let chats = self.discovered_chats.read().await;
        chats.iter().find(|c| c.chat_id == chat_id).cloned()
    }

    /// Check if discovery mode is active
    pub fn is_discovery_active(&self) -> bool {
        self.discovery_active.load(Ordering::SeqCst)
    }

    /// Set discovery mode active state
    pub fn set_discovery_active(&self, active: bool) {
        self.discovery_active.store(active, Ordering::SeqCst);
    }
}

impl Default for TelegramSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GLOBAL SINGLETON
// ============================================================================

static SESSION_MANAGER: Lazy<TelegramSessionManager> = Lazy::new(TelegramSessionManager::new);

/// Get the global session manager instance
pub fn get_session_manager() -> &'static TelegramSessionManager {
    &SESSION_MANAGER
}
