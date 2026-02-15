//! Password management handlers

use axum::{http::StatusCode, response::Response, Json};

use crate::config;
use crate::secure_storage::{generate_password_salt, hash_password, verify_password};
use crate::webserver::session;
use crate::webserver::utils::{error_response, success_response};

use super::types::{SetPasswordRequest, SetPasswordResponse};

/// POST /api/auth/set-password - Set or change authentication password
pub async fn set_password(Json(req): Json<SetPasswordRequest>) -> Response {
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
