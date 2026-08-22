//! Trade watcher handlers

use axum::{extract::Path, http::StatusCode, response::Response, Json};

use crate::logger::{self, LogTag};
use crate::tools::database::{
    add_watched_token, delete_watched_token, get_watched_tokens, WatchedTokenConfig,
};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// Trade Watcher Handlers
// =============================================================================

/// Search pools for a token
pub async fn search_pools_handler(Path(mint): Path<String>) -> Response {
    use crate::tools::trade_watcher::search_pools;

    logger::debug(
        LogTag::Tools,
        &format!("[TRADE_WATCHER] API: Searching pools for mint={mint}"),
    );

    match search_pools(&mint).await {
        Ok(pools) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Found {} pools for mint={}",
                    pools.len(),
                    mint
                ),
            );
            success_response(serde_json::json!({ "pools": pools }))
        }
        Err(e) => {
            logger::warning(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Pool search failed for mint={}: {}",
                    mint, e
                ),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "POOL_SEARCH_ERROR",
                &e,
                Some(&mint),
            )
        }
    }
}

/// Get all watched tokens
pub async fn get_watched_tokens_handler() -> Response {
    match get_watched_tokens() {
        Ok(tokens) => success_response(serde_json::json!({ "tokens": tokens })),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            &e,
            None,
        ),
    }
}

/// Add a watched token
pub async fn add_watched_token_handler(Json(req): Json<AddWatchedTokenRequest>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!(
            "[TRADE_WATCHER] Adding watched token: mint={}, pool={}, watch_type={}",
            req.mint, req.pool_address, req.watch_type
        ),
    );

    let config = WatchedTokenConfig {
        mint: req.mint.clone(),
        symbol: req.symbol,
        pool_address: req.pool_address,
        pool_source: req.pool_source,
        pool_dex: req.pool_dex,
        pool_pair: None,
        pool_liquidity: None,
        watch_type: req.watch_type,
        trigger_amount_sol: req.trigger_amount_sol,
        action_amount_sol: req.action_amount_sol,
        slippage_bps: Some(500),
    };

    match add_watched_token(&config) {
        Ok(id) => {
            logger::info(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Added watched token: id={}, mint={}",
                    id, req.mint
                ),
            );
            success_response(serde_json::json!({ "id": id, "success": true }))
        }
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!("[TRADE_WATCHER] Failed to add watched token: {e}"),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                &e,
                None,
            )
        }
    }
}

/// Delete a watched token
pub async fn delete_watched_token_handler(Path(id): Path<i64>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!("[TRADE_WATCHER] Deleting watched token: id={id}"),
    );

    match delete_watched_token(id) {
        Ok(()) => {
            logger::info(
                LogTag::Tools,
                &format!("[TRADE_WATCHER] Deleted watched token: id={id}"),
            );
            success_response(serde_json::json!({ "success": true }))
        }
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Failed to delete watched token id={}: {}",
                    id, e
                ),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                &e,
                None,
            )
        }
    }
}

/// Start the trade watcher monitor
pub async fn start_trade_watcher_handler() -> Response {
    use crate::tools::trade_watcher::start_trade_monitor;

    logger::info(
        LogTag::Tools,
        "[TRADE_WATCHER] Starting trade monitor via API",
    );
    start_trade_monitor().await;

    success_response(serde_json::json!({
        "success": true,
        "message": "Trade watcher started"
    }))
}

/// Stop the trade watcher monitor
pub async fn stop_trade_watcher_handler() -> Response {
    use crate::tools::trade_watcher::stop_trade_monitor;

    logger::info(
        LogTag::Tools,
        "[TRADE_WATCHER] Stopping trade monitor via API",
    );
    stop_trade_monitor().await;

    success_response(serde_json::json!({
        "success": true,
        "message": "Trade watcher stopped"
    }))
}

/// Get trade watcher status
pub async fn get_trade_watcher_status_handler() -> Response {
    use crate::tools::trade_watcher::get_trade_monitor_status;

    let status = get_trade_monitor_status().await;
    success_response(serde_json::json!({
        "is_running": status.is_running,
        "watched_count": status.watched_count,
        "active_count": status.active_count,
        "total_trades_detected": status.total_trades_detected,
        "total_actions_triggered": status.total_actions_triggered
    }))
}
