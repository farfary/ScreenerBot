//! TOTP two-factor authentication handlers

use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use axum::response::IntoResponse as _;
use axum::{response::Response, Json};

use crate::config;
use crate::secure_storage::verify_password;
use crate::webserver::utils::success_response;
use crate::webserver::{totp, Error};

use super::types::{
    SetPasswordResponse, TotpDisableRequest, TotpSetupRequest, TotpSetupResponse,
    TotpStatusResponse, TotpVerifySetupRequest,
};

/// GET /api/auth/totp/status - Check if TOTP is enabled
pub async fn totp_status() -> Response {
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
pub async fn totp_setup(Json(req): Json<TotpSetupRequest>) -> Response {
    // Verify password first
    let (salt, hash) = config::with_config(|cfg| {
        (
            cfg.webserver.auth_password_salt.clone(),
            cfg.webserver.auth_password_hash.clone(),
        )
    });

    if hash.is_empty() || salt.is_empty() {
        return ApiError::new(
            ApiErrorCode::PasswordNotSet,
            ids::ERRORS_AUTH_TOTP_PASSWORD_REQUIRED,
        )
        .into_response();
    }

    if !verify_password(&req.password, &salt, &hash) {
        return ApiError::new(
            ApiErrorCode::InvalidPassword,
            ids::ERRORS_AUTH_PASSWORD_INCORRECT,
        )
        .into_response();
    }

    // Generate new TOTP secret
    let secret = totp::generate_secret();
    let account = "Dashboard";

    // Generate URI and QR code
    let uri = match totp::get_totp_uri(&secret, account) {
        Ok(u) => u,
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_AUTH_TOTP_URI_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    let qr_code = match totp::generate_qr_data_url(&secret, account) {
        Ok(q) => q,
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_AUTH_TOTP_QR_FAILED)
                .details(e.to_string())
                .into_response();
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
pub async fn totp_verify_setup(Json(req): Json<TotpVerifySetupRequest>) -> Response {
    // Validate secret format (should be base32)
    if req.secret.is_empty() {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_AUTH_TOTP_SECRET_REQUIRED,
        )
        .into_response();
    }

    // Verify the TOTP code
    match totp::verify_totp(&req.secret, &req.code).and_then(|valid| {
        if valid {
            Ok(())
        } else {
            Err(Error::InvalidTotpCode)
        }
    }) {
        Ok(()) => {
            // Code verified, save secret and enable TOTP
            if let Err(e) = config::update_config_section(
                |cfg| {
                    cfg.webserver.auth_totp_secret = req.secret.clone();
                    cfg.webserver.auth_totp_enabled = true;
                },
                true,
            ) {
                return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_AUTH_TOTP_SAVE_FAILED)
                    .details(e.to_string())
                    .into_response();
            }

            success_response(SetPasswordResponse {
                success: true,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(Error::InvalidTotpCode) => ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_AUTH_TOTP_CODE_INVALID,
        )
        .into_response(),
        Err(e) => ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_AUTH_TOTP_CODE_VERIFY_FAILED,
        )
        .details(e.to_string())
        .into_response(),
    }
}

/// POST /api/auth/totp/disable - Disable TOTP 2FA
///
/// Requires password verification.
pub async fn totp_disable(Json(req): Json<TotpDisableRequest>) -> Response {
    // Verify password first
    let (salt, hash) = config::with_config(|cfg| {
        (
            cfg.webserver.auth_password_salt.clone(),
            cfg.webserver.auth_password_hash.clone(),
        )
    });

    if !verify_password(&req.password, &salt, &hash) {
        return ApiError::new(
            ApiErrorCode::InvalidPassword,
            ids::ERRORS_AUTH_PASSWORD_INCORRECT,
        )
        .into_response();
    }

    // Disable TOTP and clear secret
    if let Err(e) = config::update_config_section(
        |cfg| {
            cfg.webserver.auth_totp_enabled = false;
            cfg.webserver.auth_totp_secret = String::new();
        },
        true,
    ) {
        return ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_SAVE_FAILED)
            .details(e.to_string())
            .into_response();
    }

    success_response(SetPasswordResponse {
        success: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
