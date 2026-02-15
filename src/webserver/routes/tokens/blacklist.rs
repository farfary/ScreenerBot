//! Blacklist management handlers

use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};

use super::types::*;
use crate::{
    logger::{self, LogTag},
    tokens::{cleanup, database::get_global_database},
};

/// POST /api/tokens/:mint/blacklist
///
/// Add a token to the blacklist
pub async fn add_to_blacklist(
    Path(mint): Path<String>,
    Json(request): Json<Option<AddBlacklistRequest>>,
) -> Result<Json<BlacklistResponse>, (StatusCode, Json<serde_json::Value>)> {
    let reason = request
        .map(|r| r.reason)
        .unwrap_or_else(|| "Manual blacklist via UI".to_string());

    logger::debug(
        LogTag::Webserver,
        &format!("Adding to blacklist: mint={}, reason={}", mint, reason),
    );

    let db = match get_global_database() {
        Some(db) => db,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Token database not available"
                })),
            ));
        }
    };

    // Use spawn_blocking for sync database operation
    let mint_clone = mint.clone();
    let reason_clone = reason.clone();
    match tokio::task::spawn_blocking(move || {
        cleanup::blacklist_token(&mint_clone, &reason_clone, &db)
    })
    .await
    {
        Ok(Ok(())) => {
            logger::info(
                LogTag::Webserver,
                &format!("Token blacklisted: mint={}, reason={}", mint, reason),
            );
            Ok(Json(BlacklistResponse {
                success: true,
                mint,
                is_blacklisted: true,
                message: Some(format!("Token blacklisted: {}", reason)),
            }))
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to blacklist token mint={}: {}", mint, e),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to blacklist token: {}", e)
                })),
            ))
        }
        Err(join_err) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Join error blacklisting token mint={}: {}", mint, join_err),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Internal error during blacklist operation"
                })),
            ))
        }
    }
}

/// DELETE /api/tokens/:mint/blacklist
///
/// Remove a token from the blacklist
pub async fn remove_from_blacklist(
    Path(mint): Path<String>,
) -> Result<Json<BlacklistResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Removing from blacklist: mint={}", mint),
    );

    let db = match get_global_database() {
        Some(db) => db,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Token database not available"
                })),
            ));
        }
    };

    // Use spawn_blocking for sync database operation
    let mint_clone = mint.clone();
    match tokio::task::spawn_blocking(move || cleanup::unblacklist_token(&mint_clone, &db)).await {
        Ok(Ok(())) => {
            logger::info(
                LogTag::Webserver,
                &format!("Token removed from blacklist: mint={}", mint),
            );
            Ok(Json(BlacklistResponse {
                success: true,
                mint,
                is_blacklisted: false,
                message: Some("Token removed from blacklist".to_string()),
            }))
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to remove from blacklist mint={}: {}", mint, e),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to remove from blacklist: {}", e)
                })),
            ))
        }
        Err(join_err) => {
            logger::warning(
                LogTag::Webserver,
                &format!(
                    "Join error removing from blacklist mint={}: {}",
                    mint, join_err
                ),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Internal error during unblacklist operation"
                })),
            ))
        }
    }
}

/// GET /api/tokens/:mint/blacklist
///
/// Get blacklist status for a token
pub async fn get_blacklist_status(
    Path(mint): Path<String>,
) -> Result<Json<BlacklistResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Checking blacklist status: mint={}", mint),
    );

    let db = match get_global_database() {
        Some(db) => db,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Token database not available"
                })),
            ));
        }
    };

    // Use spawn_blocking for sync database operation
    let mint_clone = mint.clone();
    match tokio::task::spawn_blocking(move || db.is_blacklisted(&mint_clone)).await {
        Ok(Ok(is_blacklisted)) => {
            logger::debug(
                LogTag::Webserver,
                &format!(
                    "Blacklist status checked: mint={} blacklisted={}",
                    mint, is_blacklisted
                ),
            );
            Ok(Json(BlacklistResponse {
                success: true,
                mint,
                is_blacklisted,
                message: None,
            }))
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Webserver,
                &format!("Failed to check blacklist status mint={}: {}", mint, e),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": format!("Failed to check blacklist status: {}", e)
                })),
            ))
        }
        Err(join_err) => {
            logger::warning(
                LogTag::Webserver,
                &format!(
                    "Join error checking blacklist status mint={}: {}",
                    mint, join_err
                ),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                  "success": false,
                  "error": "Internal error during blacklist status check"
                })),
            ))
        }
    }
}
