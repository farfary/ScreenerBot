//! TOTP two-factor authentication handlers

use axum::{http::StatusCode, response::Response, Json};

use crate::config;
use crate::secure_storage::verify_password;
use crate::webserver::totp;
use crate::webserver::utils::{error_response, success_response};

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
pub async fn totp_verify_setup(Json(req): Json<TotpVerifySetupRequest>) -> Response {
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
                    Some(&e.to_string()),
                );
            }

            success_response(SetPasswordResponse {
                success: true,
                message: "Two-factor authentication enabled successfully".to_owned(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Ok(false) => error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_CODE",
            "Invalid verification code. Please check the code and try again.",
            None,
        ),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "TOTP_ERROR",
            "Failed to verify code",
            Some(&e.to_string()),
        ),
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
            Some(&e.to_string()),
        );
    }

    success_response(SetPasswordResponse {
        success: true,
        message: "Two-factor authentication disabled".to_owned(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
