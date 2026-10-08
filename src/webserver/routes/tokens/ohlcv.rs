// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV data, focus management, and external data handlers.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

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
    let chain = crate::chains::active_chain();
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

    // WSOL/SOL is special: its own "SOL-denominated" price is trivially 1.0, so the
    // regular OHLCV path is meaningless. Serve the SOL/USD reference chart (SOL's
    // price in USD) mirrored from the data server, so the token-details dialog shows
    // a real SOL chart. No monitoring/activity — this series is maintained globally.
    if crate::chains::adapter_for(chain).is_native_asset(&mint) {
        let series = crate::ohlcvs::native_usd_chart::series(chain, timeframe);
        // `limit == 0` means "all" here (the chart sends CHART_CANDLE_LIMIT = 0 to
        // fetch the full series); otherwise keep the newest `limit` candles.
        let take = if query.limit == 0 {
            series.len()
        } else {
            (query.limit as usize).min(series.len())
        };
        let start = series.len() - take;
        let points: Vec<OhlcvPoint> = series[start..]
            .iter()
            .map(|c| OhlcvPoint {
                timestamp: c.timestamp,
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
                volume: c.volume,
            })
            .collect();
        return Ok(Json(points));
    }

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

    if let Err(e) = crate::ohlcvs::add_token_monitoring(chain, &mint, priority).await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to add {mint} to OHLCV monitoring: {e}"),
        );
    }

    // Record chart view activity (stronger signal than just viewing token)
    if let Err(e) =
        crate::ohlcvs::record_activity(chain, &mint, crate::ohlcvs::ActivityType::ChartViewed).await
    {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to record chart view for {mint}: {e}"),
        );
    }

    // Fetch OHLCV data using new API - return empty array if no data available
    let data = match crate::ohlcvs::get_ohlcv_data(
        chain,
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

/// GET /api/tokens/:mint/ohlcv/status
///
/// Per-timeframe OHLCV process status for a token: which timeframes have data,
/// candle counts, backfill completion, and whether the token is being monitored.
/// Feeds the chart status indicator so the dialog can show data state without
/// firing a probe request per timeframe.
pub async fn get_token_ohlcv_status(
    Path(mint): Path<String>,
    Query(query): Query<OhlcvStatusQuery>,
) -> Result<Json<crate::ohlcvs::OhlcvStatus>, StatusCode> {
    let chain = crate::chains::active_chain();
    // A span only counts when both ends are given and ordered; anything else is a plain status.
    let range = match (query.from, query.to) {
        (Some(from), Some(to)) if from <= to => Some((from, to)),
        _ => None,
    };

    // WSOL/SOL uses the globally-maintained SOL/USD reference chart, so synthesize
    // its status from that in-memory series (it isn't in the per-token monitor).
    if crate::chains::adapter_for(chain).is_native_asset(&mint) {
        use crate::ohlcvs::{native_usd_chart, Timeframe};
        let tfs = [
            Timeframe::Minute1,
            Timeframe::Minute5,
            Timeframe::Minute15,
            Timeframe::Hour1,
            Timeframe::Hour4,
            Timeframe::Hour12,
            Timeframe::Day1,
        ];
        let mut timeframes = Vec::new();
        let mut total = 0i64;
        let mut best: Option<String> = None;
        for tf in tfs {
            let s = native_usd_chart::series(chain, tf);
            let count = s.len() as i64;
            total += count;
            let latest = s.last().map(|c| c.timestamp);
            if count > 0 && best.is_none() {
                best = Some(tf.to_string());
            }
            let range_candles = range.map(|(from, to)| {
                let seconds = tf.to_seconds();
                s.iter()
                    .filter(|c| c.timestamp > from - seconds && c.timestamp <= to)
                    .count() as i64
            });
            timeframes.push(crate::ohlcvs::OhlcvTimeframeStatus {
                timeframe: tf.to_string(),
                candles: count,
                backfill_complete: count > 0,
                earliest_timestamp: s.first().map(|c| c.timestamp),
                latest_timestamp: latest,
                range_candles,
                last_new_data_at: latest,
            });
        }
        return Ok(Json(crate::ohlcvs::OhlcvStatus {
            mint,
            monitored: true,
            has_data: total > 0,
            total_candles: total,
            best_timeframe: best,
            backfill_complete: total > 0,
            last_checked_at: native_usd_chart::last_updated(chain),
            last_new_data_at: native_usd_chart::last_updated(chain),
            timeframes,
        }));
    }

    match crate::ohlcvs::get_status(chain, &mint, range).await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_status_error error={e}"),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Mints whose `POST /ohlcv/refresh` task is still running. A repeated request for the same
/// mint joins the running refresh instead of queuing another provider fetch.
static REFRESH_IN_FLIGHT: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Holds a mint's slot in `REFRESH_IN_FLIGHT` for the lifetime of its refresh task and frees it
/// on completion, error or panic.
struct RefreshSlot(String);

impl RefreshSlot {
    /// Claims the slot for `mint`; `None` when a refresh for it is already running.
    fn claim(mint: &str) -> Option<Self> {
        let mut in_flight = REFRESH_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        in_flight
            .insert(mint.to_string())
            .then(|| Self(mint.to_string()))
    }
}

impl Drop for RefreshSlot {
    fn drop(&mut self) {
        REFRESH_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.0);
    }
}

/// POST /api/tokens/:mint/ohlcv/refresh
///
/// Starts an immediate fetch outside scheduled monitoring and returns 202 Accepted without
/// waiting for it: a refresh walks every timeframe against rate-limited providers and can run
/// far longer than a dashboard request timeout. The chart picks the new candles up on its own
/// poll. `already_running` is true when a refresh for this mint was in flight and none was
/// started.
pub async fn refresh_token_ohlcv(
    Path(mint): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let Some(slot) = RefreshSlot::claim(&mint) else {
        logger::debug(
            LogTag::Webserver,
            &format!("mint={mint} ohlcv_refresh_already_running"),
        );
        return (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
              "success": true,
              "mint": mint,
              "already_running": true,
            })),
        );
    };

    logger::debug(
        LogTag::Webserver,
        &format!("mint={mint} ohlcv_refresh_accepted"),
    );

    let task_mint = mint.clone();
    tokio::spawn(async move {
        let _slot = slot;
        run_ohlcv_refresh(&task_mint).await;
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
          "success": true,
          "mint": mint,
          "already_running": false,
        })),
    )
}

/// Ensures the token is monitored at chart-view priority, then fetches it now.
async fn run_ohlcv_refresh(mint: &str) {
    let chain = crate::chains::active_chain();
    let priority = if positions::is_open_position(mint).await {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High
    };

    // Idempotent: a token that is already monitored keeps its state.
    if let Err(e) = crate::ohlcvs::add_token_monitoring(chain, mint, priority).await {
        logger::warning(
            LogTag::Webserver,
            &format!("mint={mint} priority={priority:?} ohlcv_refresh_monitoring_failed error={e}"),
        );
        return;
    }

    // `request_refresh` records the data-requested activity itself.
    match crate::ohlcvs::request_refresh(chain, mint).await {
        Ok(()) => {
            logger::info(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_refresh_success"),
            );
        }
        Err(e) => {
            // A token without a discovered pool cannot fetch yet; monitoring stays active and
            // collects it once a pool is known.
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_refresh_deferred error={e}"),
            );
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
    let chain = crate::chains::active_chain();

    // Don't deprioritize if this is an open position
    let is_open_position = positions::is_open_position(&mint).await;
    if is_open_position {
        return Ok(Json(serde_json::json!({
          "success": true,
          "mint": mint,
        })));
    }

    // Downgrade to Medium priority (normal monitoring level)
    match crate::ohlcvs::update_token_priority(chain, &mint, crate::ohlcvs::Priority::Medium).await
    {
        Ok(_) => {
            logger::debug(
                LogTag::Webserver,
                &format!("mint={mint} ohlcv_deprioritized"),
            );
            Ok(Json(serde_json::json!({
              "success": true,
              "mint": mint,
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
    let chain = crate::chains::active_chain();

    // Set as dashboard active token
    crate::global::set_dashboard_active_token(Some(&mint));

    // Boost OHLCV priority to Critical
    let ohlcv_updated =
        match crate::ohlcvs::update_token_priority(chain, &mint, crate::ohlcvs::Priority::Critical)
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
    let chain = crate::chains::active_chain();

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
        match crate::ohlcvs::update_token_priority(chain, &mint, crate::ohlcvs::Priority::Medium)
            .await
        {
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
        let db = crate::tokens::database::database(crate::chains::active_chain())
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

    // Explore Mode intentionally has no wallet transaction service or database.
    // This is a valid empty history, not a backend outage.
    if crate::global::is_explore_mode() {
        return Ok(Json(Vec::new()));
    }

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
