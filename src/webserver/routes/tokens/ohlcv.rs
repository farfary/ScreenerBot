//! OHLCV data, focus management, and external data handlers

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};

use super::types::*;
use crate::{
    logger::{self, LogTag},
    positions,
};

/// GET /api/tokens/:mint/ohlcv
///
/// Get OHLCV chart data for a token
pub async fn get_token_ohlcv(
    Path(mint): Path<String>,
    Query(query): Query<OhlcvQuery>,
) -> Result<Json<Vec<OhlcvPoint>>, StatusCode> {
    let normalized_tf = query.timeframe.trim().to_ascii_lowercase();
    let timeframe = match crate::ohlcvs::Timeframe::from_str(normalized_tf.as_str()) {
        Some(tf) => tf,
        None => {
            logger::info(
                LogTag::Webserver,
                &format!("mint={} timeframe={} (fallback=1m)", mint, query.timeframe),
            );
            crate::ohlcvs::Timeframe::Minute1
        }
    };

    logger::debug(
        LogTag::Webserver,
        &format!(
            "mint={} limit={} timeframe={}",
            mint, query.limit, timeframe
        ),
    );

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures data collection starts when a user views the chart
    let is_open_position = positions::is_open_position(&mint).await;
    let priority = if is_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High // User is viewing chart, high interest
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to add {mint} to OHLCV monitoring: {e}"),
        );
    }

    // Record chart view activity (stronger signal than just viewing token)
    if let Err(e) =
        crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::ChartViewed).await
    {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to record chart view for {mint}: {e}"),
        );
    }

    // Fetch OHLCV data using new API - return empty array if no data available
    let data = match crate::ohlcvs::get_ohlcv_data(
        &mint,
        timeframe,
        None,
        query.limit as usize,
        None,
        None,
    )
    .await
    {
        Ok(data) => data,
        Err(e) => {
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} timeframe={timeframe} no_data error={e}"),
            );
            // Return empty array for tokens without OHLCV data yet
            Vec::new()
        }
    };

    let points: Vec<OhlcvPoint> = data
        .iter()
        .map(|d| OhlcvPoint {
            timestamp: d.timestamp,
            open: d.open,
            high: d.high,
            low: d.low,
            close: d.close,
            volume: d.volume,
        })
        .collect();

    Ok(Json(points))
}

/// POST /api/tokens/:mint/ohlcv/refresh
///
/// Force refresh OHLCV data (immediate fetch outside scheduled monitoring)
pub async fn refresh_token_ohlcv(
    Path(mint): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("OHLCV refresh requested for mint={mint}"),
    );

    // First, ensure token is being monitored (add if not already)
    let is_open_position = positions::is_open_position(&mint).await;
    let priority = if is_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High
    };

    // Add to monitoring (idempotent - no-op if already monitored)
    let _ = crate::ohlcvs::add_token_monitoring(&mint, priority).await;

    // Record activity
    let _ = crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::DataRequested).await;

    // Try to refresh - but don't fail if no pools available yet
    match crate::ohlcvs::request_refresh(&mint).await {
        Ok(_) => {
            logger::info(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_refresh_success"),
            );
            Ok(Json(serde_json::json!({
              "success": true,
              "mint": mint,
              "message": "OHLCV refresh triggered",
            })))
        }
        Err(e) => {
            // Log as debug, not warning - this is normal for new tokens without pools
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_refresh_deferred error={e}"),
            );
            // Return success anyway - monitoring is active, data will come when pools are available
            Ok(Json(serde_json::json!({
              "success": true,
              "mint": mint,
              "message": "OHLCV monitoring active, data pending pool availability",
            })))
        }
    }
}

/// POST /api/tokens/:mint/ohlcv/deprioritize
///
/// Deprioritize token OHLCV monitoring (when user closes token detail dialog)
pub async fn deprioritize_token_ohlcv(
    Path(mint): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("OHLCV deprioritize requested for mint={mint}"),
    );

    // Don't deprioritize if this is an open position
    let is_open_position = positions::is_open_position(&mint).await;
    if is_open_position {
        return Ok(Json(serde_json::json!({
          "success": true,
          "mint": mint,
          "message": "Token is open position, priority unchanged",
        })));
    }

    // Downgrade to Medium priority (normal monitoring level)
    match crate::ohlcvs::update_token_priority(&mint, crate::ohlcvs::Priority::Medium).await {
        Ok(_) => {
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_deprioritized"),
            );
            Ok(Json(serde_json::json!({
              "success": true,
              "mint": mint,
              "message": "OHLCV priority reduced",
            })))
        }
        Err(e) => {
            // Not an error if token wasn't being monitored
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_deprioritize_skipped error={e}"),
            );
            Ok(Json(serde_json::json!({
              "success": true,
              "mint": mint,
              "message": "Token not in OHLCV monitoring",
            })))
        }
    }
}

/// POST /api/tokens/:mint/focus
///
/// Called when user opens token details dialog.
/// Sets this token as the dashboard-active token for priority data fetching.
/// Also boosts OHLCV priority to Critical.
pub async fn focus_token(
    Path(mint): Path<String>,
) -> Result<Json<FocusResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::info(
        LogTag::Webserver,
        &format!("Token focus requested: mint={mint}"),
    );

    // Set as dashboard active token
    crate::global::set_dashboard_active_token(Some(mint.clone()));

    // Boost OHLCV priority to Critical
    let ohlcv_updated = match crate::ohlcvs::update_token_priority(
        &mint,
        crate::ohlcvs::Priority::Critical,
    )
    .await
    {
        Ok(_) => {
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_priority=Critical"),
            );
            true
        }
        Err(e) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to update OHLCV priority for {mint}: {e}"),
            );
            false
        }
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "Token focused: mint={} ohlcv_updated={}",
            mint, ohlcv_updated
        ),
    );

    Ok(Json(FocusResponse {
        success: true,
        mint,
        focused: true,
        ohlcv_priority_updated: ohlcv_updated,
        message: Some("Token focused for priority data fetching".to_owned()),
    }))
}

/// POST /api/tokens/:mint/unfocus
///
/// Called when user closes token details dialog.
/// Clears the dashboard-active token and reduces OHLCV priority.
pub async fn unfocus_token(
    Path(mint): Path<String>,
) -> Result<Json<FocusResponse>, (StatusCode, Json<serde_json::Value>)> {
    logger::info(
        LogTag::Webserver,
        &format!("Token unfocus requested: mint={mint}"),
    );

    // Clear dashboard active token
    crate::global::set_dashboard_active_token(None);

    // Reduce OHLCV priority (unless it's an open position)
    let is_open_position = positions::is_open_position(&mint).await;
    let ohlcv_updated = if is_open_position {
        logger::debug(
            LogTag::Webserver,
            &format!("mint={mint} is open position, keeping Critical priority"),
        );
        false
    } else {
        match crate::ohlcvs::update_token_priority(&mint, crate::ohlcvs::Priority::Medium).await {
            Ok(_) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("mint={mint} ohlcv_priority=Medium"),
                );
                true
            }
            Err(e) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("Failed to update OHLCV priority for {mint}: {e}"),
                );
                false
            }
        }
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "Token unfocused: mint={} ohlcv_updated={}",
            mint, ohlcv_updated
        ),
    );

    Ok(Json(FocusResponse {
        success: true,
        mint,
        focused: false,
        ohlcv_priority_updated: ohlcv_updated,
        message: Some("Token unfocused".to_owned()),
    }))
}

/// GET /api/tokens/:mint/dexscreener
///
/// Get DexScreener data for a token
pub async fn get_token_dexscreener(
    Path(mint): Path<String>,
) -> Result<Json<crate::tokens::DexScreenerData>, StatusCode> {
    logger::debug(LogTag::Webserver, &format!("mint={mint}"));

    // Get DexScreener data from token database
    let mint_clone = mint.clone();
    let data = tokio::task::spawn_blocking(move || {
        let db = crate::tokens::get_global_database()
            .ok_or_else(|| "Token database not initialized".to_owned())?;
        db.get_dexscreener_data(&mint_clone)
            .map_err(|e| format!("Database error: {e}"))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match data {
        Some(dexscreener_data) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "mint={} price_sol={:.9} liquidity_usd={} fetched={}",
                    mint,
                    dexscreener_data.price_sol,
                    dexscreener_data
                        .liquidity_usd
                        .map_or("N/A".to_owned(), |v| format!("{:.2}", v)),
                    dexscreener_data
                        .market_data_last_fetched_at
                        .format("%Y-%m-%d %H:%M:%S")
                ),
            );
            Ok(Json(dexscreener_data))
        }
        None => {
            logger::info(LogTag::Webserver, &format!("mint={mint} not found"));
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// GET /api/tokens/:mint/transactions
///
/// Get recent transactions for a specific token.
/// Filters the global transaction database for transactions involving this mint.
pub async fn get_token_transactions(
    Path(mint): Path<String>,
) -> Result<Json<Vec<crate::transactions::database::TransactionListRow>>, StatusCode> {
    logger::debug(
        LogTag::Webserver,
        &format!("Fetching transactions for token: {mint}"),
    );

    // Get connection to transaction database
    let db = match crate::transactions::database::get_transaction_database().await {
        Some(db) => db,
        None => {
            logger::error(
                LogTag::Webserver,
                "Transaction database not available when fetching token transactions",
            );
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    // Create filters: specific mint
    let mut filters = crate::transactions::database::TransactionListFilters::default();
    filters.mint = Some(mint.clone());

    // Fetch recent transactions (limit 50)
    match db.list_transactions(&filters, None, 50).await {
        Ok(result) => {
            logger::debug(
                LogTag::Webserver,
                &format!(
                    "Found {} transactions for token {}",
                    result.items.len(),
                    mint
                ),
            );
            Ok(Json(result.items))
        }
        Err(e) => {
            logger::error(
                LogTag::Webserver,
                &format!("Failed to fetch transactions for token {mint}: {e}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
