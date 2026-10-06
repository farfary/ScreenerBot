// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Config API Operations
//!
//! Config reload, reset, and diff endpoints.

use axum::response::{IntoResponse as _, Response};
use serde::Serialize;

use crate::config;
use crate::i18n::{ids, UiText};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

use super::types::UpdateResponse;

// ============================================================================
// UTILITY ENDPOINTS
// ============================================================================

/// POST /api/config/reload - Reload configuration from disk
pub async fn reload_config_from_disk() -> Response {
    match config::reload_config() {
        Ok(_) => success_response(UpdateResponse {
            saved_to_disk: false,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(e) => ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_RELOAD_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}

/// POST /api/config/reset - Reset configuration to defaults
pub async fn reset_config_to_defaults() -> Response {
    let (wallet_encrypted, wallet_nonce, coingecko_api_key) = config::with_config(|cfg| {
        (
            cfg.wallet_encrypted.clone(),
            cfg.wallet_nonce.clone(),
            cfg.tokens.discovery.coingecko.api_key.clone(),
        )
    });

    let result = config::update_config_section(
        |cfg| {
            // Keep secrets that are not recoverable via the UI while resetting everything else.
            let mut fresh = config::Config::default();
            if !wallet_encrypted.is_empty() && !wallet_nonce.is_empty() {
                fresh.wallet_encrypted = wallet_encrypted.clone();
                fresh.wallet_nonce = wallet_nonce.clone();
            }
            fresh.tokens.discovery.coingecko.api_key = coingecko_api_key.clone();
            *cfg = fresh;
        },
        true, // Save to disk
    );

    match result {
        Ok(_) => success_response(UpdateResponse {
            saved_to_disk: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(e) => ApiError::new(ApiErrorCode::ConfigError, ids::ERRORS_CONFIG_RESET_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}

/// GET /api/config/diff - Compare in-memory config with disk version
pub async fn get_config_diff() -> Response {
    // Load current in-memory config
    let memory_config = config::get_config_clone();

    // Try to load disk config
    let config_path = crate::paths::get_config_path();
    let disk_result = std::fs::read_to_string(&config_path);

    match disk_result {
        Ok(contents) => {
            match config::utils::parse_config_document(&contents) {
                Ok(config::utils::ParsedConfig {
                    config: disk_config,
                    ..
                }) => {
                    fn sanitize_config_json(value: &mut serde_json::Value) {
                        if let Some(obj) = value.as_object_mut() {
                            // Remove encrypted wallet fields from comparison output
                            obj.remove("wallet_encrypted");
                            obj.remove("wallet_nonce");
                        }
                    }

                    // Compare using JSON serialization for accurate comparison
                    let mut memory_json = serde_json::to_value(&memory_config)
                        .unwrap_or_else(|_| serde_json::Value::Null);
                    let mut disk_json = serde_json::to_value(&disk_config)
                        .unwrap_or_else(|_| serde_json::Value::Null);

                    sanitize_config_json(&mut memory_json);
                    sanitize_config_json(&mut disk_json);

                    let has_changes = memory_json != disk_json;

                    #[derive(Serialize)]
                    struct DiffResponse {
                        has_changes: bool,
                        memory: serde_json::Value,
                        disk: serde_json::Value,
                        memory_timestamp: String,
                        disk_file: String,
                        text: UiText,
                    }

                    success_response(DiffResponse {
                        has_changes,
                        memory: memory_json,
                        disk: disk_json,
                        memory_timestamp: chrono::Utc::now().to_rfc3339(),
                        disk_file: config_path.to_string_lossy().to_string(),
                        text: UiText::new(if has_changes {
                            ids::SYSTEM_RESULT_CONFIG_DIFFERS
                        } else {
                            ids::SYSTEM_RESULT_CONFIG_MATCHES
                        }),
                    })
                }
                Err(e) => ApiError::new(
                    ApiErrorCode::ConfigError,
                    ids::ERRORS_CONFIG_DISK_PARSE_FAILED,
                )
                .details(e.to_string())
                .into_response(),
            }
        }
        Err(e) => ApiError::new(
            ApiErrorCode::ConfigError,
            ids::ERRORS_CONFIG_DISK_READ_FAILED,
        )
        .details(e.to_string())
        .into_response(),
    }
}
