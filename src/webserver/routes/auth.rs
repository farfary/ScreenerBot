//! Authentication API routes for headless mode
//!
//! Provides REST API endpoints for password-based authentication when running
//! in headless/VPS mode. These routes handle login, logout, session management,
//! and TOTP two-factor authentication.

use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config;
use crate::secure_storage::{generate_password_salt, hash_password, verify_password};
use crate::webserver::session;
use crate::webserver::state::AppState;
use crate::webserver::totp;
use crate::webserver::utils::{error_response, success_response};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Cookie name for session token
pub const SESSION_COOKIE_NAME: &str = "screenerbot_session";

// =============================================================================
// RESPONSE TYPES (inline per ScreenerBot convention)
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

// =============================================================================
// ROUTES
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/status", get(get_status))
        .route("/set-password", post(set_password))
        // TOTP 2FA routes
        .route("/totp/status", get(totp_status))
        .route("/totp/setup", post(totp_setup))
        .route("/totp/verify-setup", post(totp_verify_setup))
        .route("/totp/disable", post(totp_disable))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// GET /api/auth/status - Get authentication status and configuration
async fn get_status(headers: HeaderMap) -> Response {
    let (auth_enabled, has_password, totp_enabled, show_logo, show_name, custom_title) =
        config::with_config(|cfg| {
            (
                cfg.webserver.auth_enabled,
                !cfg.webserver.auth_password_hash.is_empty(),
                cfg.webserver.auth_totp_enabled && !cfg.webserver.auth_totp_secret.is_empty(),
                cfg.webserver.auth_show_logo,
                cfg.webserver.auth_show_name,
                cfg.webserver.auth_custom_title.clone(),
            )
        });

    // Check if current session is valid
    let authenticated = if let Some(token) = get_cookie_value(&headers, SESSION_COOKIE_NAME) {
        session::validate_session(&token)
    } else {
        false
    };

    success_response(AuthStatusResponse {
        auth_enabled,
        authenticated,
        has_password,
        totp_enabled,
        show_logo,
        show_name,
        custom_title,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/auth/login - Authenticate with password (and optional TOTP)
async fn login(Json(req): Json<LoginRequest>) -> Response {
    // Get auth config
    let (auth_enabled, salt, hash, timeout, totp_enabled, totp_secret) = config::with_config(|cfg| {
        (
            cfg.webserver.auth_enabled,
            cfg.webserver.auth_password_salt.clone(),
            cfg.webserver.auth_password_hash.clone(),
            cfg.webserver.auth_session_timeout_secs,
            cfg.webserver.auth_totp_enabled && !cfg.webserver.auth_totp_secret.is_empty(),
            cfg.webserver.auth_totp_secret.clone(),
        )
    });

    // Check if auth is enabled
    if !auth_enabled {
        return error_response(
            StatusCode::BAD_REQUEST,
            "AUTH_DISABLED",
            "Authentication is not enabled",
            None,
        );
    }

    // Check if password is set
    if hash.is_empty() || salt.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_PASSWORD",
            "No password has been configured",
            None,
        );
    }

    // Verify password
    if !verify_password(&req.password, &salt, &hash) {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "INVALID_PASSWORD",
            "Incorrect password",
            None,
        );
    }

    // If TOTP is enabled, verify the code
    if totp_enabled {
        match &req.totp_code {
            Some(code) => {
                // Verify TOTP code
                match totp::verify_totp(&totp_secret, code) {
                    Ok(true) => {
                        // TOTP verified, continue to create session
                    }
                    Ok(false) => {
                        return error_response(
                            StatusCode::UNAUTHORIZED,
                            "INVALID_TOTP",
                            "Invalid or expired 2FA code",
                            None,
                        );
                    }
                    Err(e) => {
                        return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "TOTP_ERROR",
                            "Failed to verify 2FA code",
                            Some(&e),
                        );
                    }
                }
            }
            None => {
                // Password verified but TOTP code not provided - request it
                return success_response(LoginResponse {
                    success: false,
                    requires_totp: Some(true),
                    token: None,
                    expires_at: 0,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
        }
    }

    // Generate session token
    let token = session::generate_session_token();
    let sess = session::create_session(token.clone());

    // Build Set-Cookie header
    let cookie_value = build_session_cookie(&token, timeout);

    // Return success with token
    let response_body = LoginResponse {
        success: true,
        requires_totp: None,
        token: Some(token),
        expires_at: sess.expires_at,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut response = success_response(response_body);
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_value.parse().unwrap_or_else(|_| {
            // Fallback if cookie value is invalid
            "screenerbot_session=; Max-Age=0".parse().unwrap()
        }),
    );

    response
}

/// POST /api/auth/logout - Revoke current session
async fn logout(headers: HeaderMap) -> Response {
    // Get current session token from cookie
    if let Some(token) = get_cookie_value(&headers, SESSION_COOKIE_NAME) {
        session::revoke_session(&token);
    }

    // Build cookie to clear the session
    let clear_cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        SESSION_COOKIE_NAME
    );

    let response_body = LogoutResponse {
        success: true,
        message: "Logged out successfully".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut response = success_response(response_body);
    response.headers_mut().insert(
        header::SET_COOKIE,
        clear_cookie.parse().unwrap(),
    );

    response
}

/// POST /api/auth/set-password - Set or change authentication password
async fn set_password(Json(req): Json<SetPasswordRequest>) -> Response {
    // Get current password info
    let (existing_salt, existing_hash) = config::with_config(|cfg| {
        (
            cfg.webserver.auth_password_salt.clone(),
            cfg.webserver.auth_password_hash.clone(),
        )
    });

    let has_existing = !existing_hash.is_empty() && !existing_salt.is_empty();

    // If password exists, verify current password first
    if has_existing {
        match &req.current_password {
            Some(current) => {
                if !verify_password(current, &existing_salt, &existing_hash) {
                    return error_response(
                        StatusCode::UNAUTHORIZED,
                        "INVALID_PASSWORD",
                        "Current password is incorrect",
                        None,
                    );
                }
            }
            None => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "CURRENT_REQUIRED",
                    "Current password is required to change password",
                    None,
                );
            }
        }
    }

    // Handle password clear (empty new password)
    if req.new_password.is_empty() {
        // Clear password and disable auth
        if let Err(e) = config::update_config_section(
            |cfg| {
                cfg.webserver.auth_password_hash = String::new();
                cfg.webserver.auth_password_salt = String::new();
                cfg.webserver.auth_enabled = false;
            },
            true,
        ) {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CONFIG_ERROR",
                "Failed to save configuration",
                Some(&e),
            );
        }

        return success_response(SetPasswordResponse {
            success: true,
            message: "Password cleared and authentication disabled".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    // Validate new password
    if req.new_password.len() < 4 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "PASSWORD_TOO_SHORT",
            "Password must be at least 4 characters",
            None,
        );
    }

    if req.new_password.len() > 128 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "PASSWORD_TOO_LONG",
            "Password must be at most 128 characters",
            None,
        );
    }

    // Generate new salt and hash
    let new_salt = generate_password_salt();
    let new_hash = match hash_password(&req.new_password, &new_salt) {
        Ok(h) => h,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "HASH_ERROR",
                "Failed to hash password",
                Some(&e),
            );
        }
    };

    // Update config
    if let Err(e) = config::update_config_section(
        |cfg| {
            cfg.webserver.auth_password_hash = new_hash;
            cfg.webserver.auth_password_salt = new_salt;
            // Enable auth when password is set
            cfg.webserver.auth_enabled = true;
        },
        true,
    ) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "Failed to save configuration",
            Some(&e),
        );
    }

    // Security: Invalidate all existing sessions when password is changed
    // This forces re-authentication with the new password
    session::clear_all_sessions();

    success_response(SetPasswordResponse {
        success: true,
        message: if has_existing {
            "Password changed successfully".to_string()
        } else {
            "Password set and authentication enabled".to_string()
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// =============================================================================
// TOTP HANDLERS
// =============================================================================

/// GET /api/auth/totp/status - Check if TOTP is enabled
async fn totp_status() -> Response {
    let enabled = config::with_config(|cfg| {
        cfg.webserver.auth_totp_enabled && !cfg.webserver.auth_totp_secret.is_empty()
    });

    success_response(TotpStatusResponse {
        enabled,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/auth/totp/setup - Generate new TOTP secret for setup
///
/// Requires password verification. Returns secret, URI, and QR code.
/// The secret is NOT saved until verify-setup is called.
async fn totp_setup(Json(req): Json<TotpSetupRequest>) -> Response {
    // Verify password first
    let (salt, hash) = config::with_config(|cfg| {
        (
            cfg.webserver.auth_password_salt.clone(),
            cfg.webserver.auth_password_hash.clone(),
        )
    });

    if hash.is_empty() || salt.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_PASSWORD",
            "Password must be set before enabling 2FA",
            None,
        );
    }

    if !verify_password(&req.password, &salt, &hash) {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "INVALID_PASSWORD",
            "Incorrect password",
            None,
        );
    }

    // Generate new TOTP secret
    let secret = totp::generate_secret();
    let account = "Dashboard";

    // Generate URI and QR code
    let uri = match totp::get_totp_uri(&secret, account) {
        Ok(u) => u,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "TOTP_ERROR",
                "Failed to generate TOTP URI",
                Some(&e),
            );
        }
    };

    let qr_code = match totp::generate_qr_data_url(&secret, account) {
        Ok(q) => q,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QR_ERROR",
                "Failed to generate QR code",
                Some(&e),
            );
        }
    };

    success_response(TotpSetupResponse {
        secret,
        uri,
        qr_code,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/auth/totp/verify-setup - Verify TOTP code and enable 2FA
///
/// Verifies the provided code against the secret and saves to config if valid.
async fn totp_verify_setup(Json(req): Json<TotpVerifySetupRequest>) -> Response {
    // Validate secret format (should be base32)
    if req.secret.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SECRET",
            "Secret is required",
            None,
        );
    }

    // Verify the TOTP code
    match totp::verify_totp(&req.secret, &req.code) {
        Ok(true) => {
            // Code verified, save secret and enable TOTP
            if let Err(e) = config::update_config_section(
                |cfg| {
                    cfg.webserver.auth_totp_secret = req.secret.clone();
                    cfg.webserver.auth_totp_enabled = true;
                },
                true,
            ) {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CONFIG_ERROR",
                    "Failed to save TOTP configuration",
                    Some(&e),
                );
            }

            success_response(SetPasswordResponse {
                success: true,
                message: "Two-factor authentication enabled successfully".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Ok(false) => {
            error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_CODE",
                "Invalid verification code. Please check the code and try again.",
                None,
            )
        }
        Err(e) => {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "TOTP_ERROR",
                "Failed to verify code",
                Some(&e),
            )
        }
    }
}

/// POST /api/auth/totp/disable - Disable TOTP 2FA
///
/// Requires password verification.
async fn totp_disable(Json(req): Json<TotpDisableRequest>) -> Response {
    // Verify password first
    let (salt, hash) = config::with_config(|cfg| {
        (
            cfg.webserver.auth_password_salt.clone(),
            cfg.webserver.auth_password_hash.clone(),
        )
    });

    if !verify_password(&req.password, &salt, &hash) {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "INVALID_PASSWORD",
            "Incorrect password",
            None,
        );
    }

    // Disable TOTP and clear secret
    if let Err(e) = config::update_config_section(
        |cfg| {
            cfg.webserver.auth_totp_enabled = false;
            cfg.webserver.auth_totp_secret = String::new();
        },
        true,
    ) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "Failed to save configuration",
            Some(&e),
        );
    }

    success_response(SetPasswordResponse {
        success: true,
        message: "Two-factor authentication disabled".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// =============================================================================
// HELPERS
// =============================================================================

/// Get a cookie value from headers by name
fn get_cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|cookies| cookies.to_str().ok())
        .and_then(|cookies| {
            for cookie in cookies.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == name {
                    return Some(parts[1].to_string());
                }
            }
            None
        })
}

/// Build session cookie string with proper attributes
fn build_session_cookie(token: &str, timeout_secs: u64) -> String {
    let max_age = if timeout_secs > 0 {
        format!("; Max-Age={}", timeout_secs)
    } else {
        // Session cookie (expires when browser closes) - no Max-Age
        String::new()
    };

    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict{}",
        SESSION_COOKIE_NAME, token, max_age
    )
}

/// Extract session token from request cookies (for use in middleware)
pub fn extract_session_token(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::COOKIE)
        .and_then(|cookies| cookies.to_str().ok())
        .and_then(|cookies| {
            for cookie in cookies.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == SESSION_COOKIE_NAME {
                    return Some(parts[1].to_string());
                }
            }
            None
        })
}
