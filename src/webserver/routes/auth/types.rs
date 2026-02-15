//! Authentication request/response types

use serde::{Deserialize, Serialize};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Cookie name for session token
pub const SESSION_COOKIE_NAME: &str = "screenerbot_session";

// =============================================================================
// RESPONSE TYPES
// =============================================================================

/// Auth status response
#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    /// Whether authentication is enabled
    pub auth_enabled: bool,
    /// Whether the current request is authenticated
    pub authenticated: bool,
    /// Whether a password has been set
    pub has_password: bool,
    /// Whether TOTP 2FA is enabled
    pub totp_enabled: bool,
    /// Login page customization
    pub show_logo: bool,
    pub show_name: bool,
    pub custom_title: String,
    /// Timestamp of response
    pub timestamp: String,
}

/// Login request (supports both password-only and password+TOTP)
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// The password to verify
    pub password: String,
    /// TOTP code (optional, required if TOTP is enabled)
    pub totp_code: Option<String>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// Whether login was successful
    pub success: bool,
    /// Whether TOTP code is required (password verified, awaiting TOTP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_totp: Option<bool>,
    /// Session token (also set as cookie)
    pub token: Option<String>,
    /// Session expiry timestamp (0 = never)
    pub expires_at: u64,
    /// Timestamp of response
    pub timestamp: String,
}

/// Logout response
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    /// Whether logout was successful
    pub success: bool,
    pub message: String,
    pub timestamp: String,
}

/// Set password request
#[derive(Debug, Deserialize)]
pub struct SetPasswordRequest {
    /// Current password (required if password already set)
    pub current_password: Option<String>,
    /// New password to set (empty to clear)
    pub new_password: String,
}

/// Set password response
#[derive(Debug, Serialize)]
pub struct SetPasswordResponse {
    pub success: bool,
    pub message: String,
    pub timestamp: String,
}

/// TOTP status response
#[derive(Debug, Serialize)]
pub struct TotpStatusResponse {
    /// Whether TOTP is enabled
    pub enabled: bool,
    /// Timestamp of response
    pub timestamp: String,
}

/// TOTP setup request
#[derive(Debug, Deserialize)]
pub struct TotpSetupRequest {
    /// Password required to initiate setup
    pub password: String,
}

/// TOTP setup response (contains secret and QR for initial setup)
#[derive(Debug, Serialize)]
pub struct TotpSetupResponse {
    /// Base32-encoded secret (for manual entry)
    pub secret: String,
    /// otpauth:// URI
    pub uri: String,
    /// QR code as data URL (data:image/svg+xml;base64,...)
    pub qr_code: String,
    /// Timestamp of response
    pub timestamp: String,
}

/// TOTP verify setup request
#[derive(Debug, Deserialize)]
pub struct TotpVerifySetupRequest {
    /// The secret being set up
    pub secret: String,
    /// TOTP code to verify setup
    pub code: String,
}

/// TOTP disable request
#[derive(Debug, Deserialize)]
pub struct TotpDisableRequest {
    /// Password required to disable TOTP
    pub password: String,
}
