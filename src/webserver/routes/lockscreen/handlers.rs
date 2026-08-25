use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;

use crate::config;
use crate::secure_storage::{generate_password_salt, hash_password, verify_password};
use crate::webserver::{
    utils::{error_response, success_response},
    Error, Result,
};

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
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_PASSWORD",
            "No password has been set",
            None,
        );
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
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_TYPE",
            "Invalid password type. Must be 'pin4', 'pin6', or 'text'",
            None,
        );
    }

    // Validate password format based on type
    if let Err(e) = validate_password_format(&req.new_password, &req.password_type) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_FORMAT",
            &e.to_string(),
            None,
        );
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
            cfg.gui.dashboard.lockscreen.password_hash = new_hash;
            cfg.gui.dashboard.lockscreen.password_salt = new_salt;
            cfg.gui.dashboard.lockscreen.password_type = req.password_type.clone();
            // Enable lockscreen when password is set
            cfg.gui.dashboard.lockscreen.enabled = true;
        },
        true,
    ) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "Failed to save configuration",
            Some(&e.to_string()),
        );
    }

    success_response(SuccessResponse {
        success: true,
        message: "Password set successfully".to_owned(),
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
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_PASSWORD",
            "No password is currently set",
            None,
        );
    }

    // Verify current password
    if !verify_password(&req.current_password, &salt, &hash) {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "INVALID_PASSWORD",
            "Password is incorrect",
            None,
        );
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
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "Failed to save configuration",
            Some(&e.to_string()),
        );
    }

    success_response(SuccessResponse {
        success: true,
        message: "Password cleared and lockscreen disabled".to_owned(),
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
            return error_response(
                StatusCode::BAD_REQUEST,
                "NO_PASSWORD",
                "Cannot enable lockscreen without setting a password first",
                None,
            );
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
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            "Failed to save configuration",
            Some(&e.to_string()),
        );
    }

    success_response(SuccessResponse {
        success: true,
        message: "Settings updated successfully".to_owned(),
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
