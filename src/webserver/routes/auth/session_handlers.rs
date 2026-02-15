//! Session management handlers (login, logout, status)

use axum::{http::header, http::HeaderMap, http::StatusCode, response::Response, Json};

use crate::config;
use crate::secure_storage::verify_password;
use crate::webserver::session;
use crate::webserver::totp;
use crate::webserver::utils::{error_response, success_response};

use super::helpers::{build_session_cookie, get_cookie_value};
use super::types::{
    AuthStatusResponse, LoginRequest, LoginResponse, LogoutResponse, SESSION_COOKIE_NAME,
};

/// GET /api/auth/status - Get authentication status and configuration
pub async fn get_status(headers: HeaderMap) -> Response {
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
pub async fn login(Json(req): Json<LoginRequest>) -> Response {
    // Get auth config
    let (auth_enabled, salt, hash, timeout, totp_enabled, totp_secret) =
        config::with_config(|cfg| {
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
pub async fn logout(headers: HeaderMap) -> Response {
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
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_cookie.parse().unwrap());

    response
}
