// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Lockscreen API handlers covering status, password set/verify/clear and settings updates, with PIN and text-password format validation.

use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use axum::response::IntoResponse as _;
use axum::response::Response;
use axum::Json;

use crate::config;
use crate::secure_storage::{generate_password_salt, hash_password, verify_password};
use crate::webserver::{utils::success_response, Error, Result};

use super::types::*;

// =============================================================================
// HANDLERS
// =============================================================================

/// GET /api/lockscreen/status - Get lockscreen configuration status
pub(super) async fn get_status() -> Response {
    let status = config::with_config(|cfg| {
        let lockscreen = &cfg.gui.dashboard.lockscreen;
        LockscreenStatusResponse {
            enabled: lockscreen.enabled,
            password_type: lockscreen.password_type.clone(),
            has_password: !lockscreen.password_hash.is_empty(),
            auto_lock_timeout_secs: lockscreen.auto_lock_timeout_secs,
            lock_on_blur: lockscreen.lock_on_blur,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    });

    success_response(status)
}

/// POST /api/lockscreen/verify - Verify password attempt
pub(super) async fn verify_password_handler(Json(req): Json<VerifyPasswordRequest>) -> Response {
    let (salt, hash) = config::with_config(|cfg| {
        let lockscreen = &cfg.gui.dashboard.lockscreen;
        (
            lockscreen.password_salt.clone(),
            lockscreen.password_hash.clone(),
        )
    });

    // Check if password is set
    if hash.is_empty() || salt.is_empty() {
        return ApiError::new(
            ApiErrorCode::PasswordNotSet,
            ids::ERRORS_LOCKSCREEN_NO_PASSWORD_SET,
        )
        .into_response();
    }

    let valid = verify_password(&req.password, &salt, &hash);

    success_response(VerifyPasswordResponse {
        valid,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/lockscreen/set-password - Set or change password
pub(super) async fn set_password(Json(req): Json<SetPasswordRequest>) -> Response {
    // Validate password type
    if !["pin4", "pin6", "text"].contains(&req.password_type.as_str()) {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_LOCKSCREEN_INVALID_TYPE,
        )
        .into_response();
    }

    // Validate password format based on type
    if let Err(e) = validate_password_format(&req.new_password, &req.password_type) {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_LOCKSCREEN_INVALID_FORMAT,
        )
        .details(e.to_string())
        .into_response();
    }

    // Check if password already exists
    let (existing_salt, existing_hash) = config::with_config(|cfg| {
        let lockscreen = &cfg.gui.dashboard.lockscreen;
        (
            lockscreen.password_salt.clone(),
            lockscreen.password_hash.clone(),
        )
    });

    let has_existing = !existing_hash.is_empty() && !existing_salt.is_empty();

    // If password exists, verify current password
    if has_existing {
        match &req.current_password {
            Some(current) => {
                if !verify_password(current, &existing_salt, &existing_hash) {
                    return ApiError::new(
                        ApiErrorCode::InvalidPassword,
                        ids::ERRORS_AUTH_CURRENT_PASSWORD_INCORRECT,
                    )
                    .into_response();
                }
            }
            None => {
                return ApiError::new(
                    ApiErrorCode::CurrentPasswordRequired,
                    ids::ERRORS_AUTH_CURRENT_PASSWORD_REQUIRED,
                )
                .into_response();
            }
        }
    }

    // Generate new salt and hash
    let new_salt = generate_password_salt();
    let new_hash = match hash_password(&req.new_password, &new_salt) {
        Ok(h) => h,
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_AUTH_HASH_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    // Update config
    if let Err(e) = config::update_config_section(
        |cfg| {
            cfg.gui.dashboard.lockscreen.password_hash = new_hash;
            cfg.gui.dashboard.lockscreen.password_salt = new_salt;
            cfg.gui.dashboard.lockscreen.password_type = req.password_type.clone();
            // Enable lockscreen when password is set
            cfg.gui.dashboard.lockscreen.enabled = true;
        },
        true,
    ) {
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    success_response(SuccessResponse {
        success: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/lockscreen/clear-password - Remove password and disable lockscreen
pub(super) async fn clear_password(Json(req): Json<ClearPasswordRequest>) -> Response {
    // Get current password info
    let (salt, hash) = config::with_config(|cfg| {
        let lockscreen = &cfg.gui.dashboard.lockscreen;
        (
            lockscreen.password_salt.clone(),
            lockscreen.password_hash.clone(),
        )
    });

    // Check if password exists
    if hash.is_empty() || salt.is_empty() {
        return ApiError::new(
            ApiErrorCode::PasswordNotSet,
            ids::ERRORS_LOCKSCREEN_NO_CURRENT_PASSWORD,
        )
        .into_response();
    }

    // Verify current password
    if !verify_password(&req.current_password, &salt, &hash) {
        return ApiError::new(
            ApiErrorCode::InvalidPassword,
            ids::ERRORS_LOCKSCREEN_PASSWORD_INCORRECT,
        )
        .into_response();
    }

    // Clear password and disable lockscreen
    if let Err(e) = config::update_config_section(
        |cfg| {
            cfg.gui.dashboard.lockscreen.password_hash = String::new();
            cfg.gui.dashboard.lockscreen.password_salt = String::new();
            cfg.gui.dashboard.lockscreen.enabled = false;
        },
        true,
    ) {
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    success_response(SuccessResponse {
        success: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// POST /api/lockscreen/settings - Update lockscreen settings
pub(super) async fn update_settings(Json(req): Json<UpdateSettingsRequest>) -> Response {
    // Check if password is set before allowing enable
    if let Some(true) = req.enabled {
        let has_password =
            config::with_config(|cfg| !cfg.gui.dashboard.lockscreen.password_hash.is_empty());

        if !has_password {
            return ApiError::new(
                ApiErrorCode::PasswordNotSet,
                ids::ERRORS_LOCKSCREEN_ENABLE_NEEDS_PASSWORD,
            )
            .into_response();
        }
    }

    // Update settings
    if let Err(e) = config::update_config_section(
        |cfg| {
            if let Some(enabled) = req.enabled {
                cfg.gui.dashboard.lockscreen.enabled = enabled;
            }
            if let Some(timeout) = req.auto_lock_timeout_secs {
                cfg.gui.dashboard.lockscreen.auto_lock_timeout_secs = timeout;
            }
            if let Some(lock_on_blur) = req.lock_on_blur {
                cfg.gui.dashboard.lockscreen.lock_on_blur = lock_on_blur;
            }
        },
        true,
    ) {
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    success_response(SuccessResponse {
        success: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// =============================================================================
// HELPERS
// =============================================================================

/// Validate password format based on type
fn validate_password_format(password: &str, password_type: &str) -> Result<()> {
    match password_type {
        "pin4" => {
            if password.len() != 4 {
                return Err(Error::InvalidLockscreenPassword {
                    detail: "PIN must be exactly 4 digits".to_owned(),
                });
            }
            if !password.chars().all(|c| c.is_ascii_digit()) {
                return Err(Error::InvalidLockscreenPassword {
                    detail: "PIN must contain only digits".to_owned(),
                });
            }
        }
        "pin6" => {
            if password.len() != 6 {
                return Err(Error::InvalidLockscreenPassword {
                    detail: "PIN must be exactly 6 digits".to_owned(),
                });
            }
            if !password.chars().all(|c| c.is_ascii_digit()) {
                return Err(Error::InvalidLockscreenPassword {
                    detail: "PIN must contain only digits".to_owned(),
                });
            }
        }
        "text" => {
            if password.len() < 4 {
                return Err(Error::InvalidLockscreenPassword {
                    detail: "Password must be at least 4 characters".to_owned(),
                });
            }
            if password.len() > 128 {
                return Err(Error::InvalidLockscreenPassword {
                    detail: "Password must be at most 128 characters".to_owned(),
                });
            }
        }
        _ => {
            return Err(Error::InvalidLockscreenPassword {
                detail: "Invalid password type".to_owned(),
            });
        }
    }
    Ok(())
}
