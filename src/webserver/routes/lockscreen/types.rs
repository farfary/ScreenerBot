use serde::{Deserialize, Serialize};

// =============================================================================
// RESPONSE TYPES (inline per ScreenerBot convention)
// =============================================================================

/// Lockscreen status response
#[derive(Debug, Serialize)]
pub struct LockscreenStatusResponse {
    /// Whether lockscreen is enabled
    pub enabled: bool,
    /// Password type: "pin4", "pin6", "text"
    pub password_type: String,
    /// Whether a password has been set
    pub has_password: bool,
    /// Auto-lock timeout in seconds (0 = never)
    pub auto_lock_timeout_secs: u64,
    /// Lock on app blur/minimize
    pub lock_on_blur: bool,
    /// Timestamp of response
    pub timestamp: String,
}

/// Password verification request
#[derive(Debug, Deserialize)]
pub struct VerifyPasswordRequest {
    /// The password attempt
    pub password: String,
}

/// Password verification response
#[derive(Debug, Serialize)]
pub struct VerifyPasswordResponse {
    /// Whether verification succeeded
    pub valid: bool,
    /// Timestamp of response
    pub timestamp: String,
}

/// Set password request
#[derive(Debug, Deserialize)]
pub struct SetPasswordRequest {
    /// Current password (required if password already exists)
    pub current_password: Option<String>,
    /// New password to set
    pub new_password: String,
    /// Password type: "pin4", "pin6", "text"
    pub password_type: String,
}

/// Clear password request
#[derive(Debug, Deserialize)]
pub struct ClearPasswordRequest {
    /// Current password (required to clear)
    pub current_password: String,
}

/// Update settings request
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    /// Enable or disable lockscreen
    pub enabled: Option<bool>,
    /// Auto-lock timeout in seconds (0 = never)
    pub auto_lock_timeout_secs: Option<u64>,
    /// Lock on app blur/minimize
    pub lock_on_blur: Option<bool>,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
    pub timestamp: String,
}
