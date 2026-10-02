// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Password management handlers.

use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use axum::response::IntoResponse as _;
use axum::{response::Response, Json};

use crate::config;
use crate::secure_storage::{generate_password_salt, hash_password, verify_password};
use crate::webserver::session;
use crate::webserver::utils::success_response;

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
            return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
                .details(e.to_string())
                .into_response();
        }

        return success_response(SetPasswordResponse {
            success: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    // Validate new password
    if req.new_password.len() < 4 {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_AUTH_PASSWORD_TOO_SHORT,
        )
        .into_response();
    }

    if req.new_password.len() > 128 {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_AUTH_PASSWORD_TOO_LONG,
        )
        .into_response();
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
            cfg.webserver.auth_password_hash = new_hash;
            cfg.webserver.auth_password_salt = new_salt;
            // Enable auth when password is set
            cfg.webserver.auth_enabled = true;
        },
        true,
    ) {
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    // Security: Invalidate all existing sessions when password is changed
    // This forces re-authentication with the new password
    session::clear_all_sessions();

    success_response(SetPasswordResponse {
        success: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
