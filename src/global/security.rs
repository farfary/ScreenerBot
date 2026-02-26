//! Security token generation and validation for GUI mode.

use std::sync::RwLock;

/// Security token required for all API requests in GUI mode.
/// Generated at startup, must be passed in X-ScreenerBot-Token header.
static SECURITY_TOKEN: RwLock<Option<String>> = RwLock::new(None);

/// Generate and store a new security token (called at webserver startup in GUI mode).
pub fn generate_security_token() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let mut guard = SECURITY_TOKEN.write().unwrap();
    *guard = Some(token.clone());
    token
}

/// Get the current security token (None if not generated).
pub fn get_security_token() -> Option<String> {
    SECURITY_TOKEN.read().unwrap().clone()
}

/// Validate a token against the stored security token.
/// Returns true if tokens match, or if not in GUI mode (no validation needed).
pub fn validate_security_token(token: &str) -> bool {
    if !super::is_gui_mode() {
        return true;
    }

    match SECURITY_TOKEN.read().unwrap().as_ref() {
        Some(stored) => stored == token,
        None => false,
    }
}
