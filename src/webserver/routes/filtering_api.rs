use axum::{
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    filtering,
    logger::{self, LogTag},
    tokens::get_rejection_stats_async,
    webserver::state::AppState,
    webserver::utils::{error_response, success_response},
};

/// Filtering management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/filtering/refresh", post(trigger_refresh))
        .route("/filtering/stats", get(get_stats))
        .route("/filtering/rejection-stats", get(get_rejection_stats))
}

#[derive(Debug, Serialize)]
struct RefreshResponse {
    message: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct FilteringStatsResponse {
    total_tokens: usize,
    with_pool_price: usize,
    open_positions: usize,
    blacklisted: usize,
    with_ohlcv: usize,
    passed_filtering: usize,
    updated_at: String,
    timestamp: String,
}

/// GET /api/filtering/stats
/// Retrieve current filtering statistics including token counts and metrics
async fn get_stats() -> Response {
    match filtering::fetch_stats().await {
        Ok(stats) => success_response(FilteringStatsResponse {
            total_tokens: stats.total_tokens,
            with_pool_price: stats.with_pool_price,
            open_positions: stats.open_positions,
            blacklisted: stats.blacklisted,
            with_ohlcv: stats.with_ohlcv,
            passed_filtering: stats.passed_filtering,
            updated_at: stats.updated_at.to_rfc3339(),
            timestamp: Utc::now().to_rfc3339(),
        }),
        Err(err) => {
            logger::info(
                LogTag::Filtering,
                &format!("Failed to fetch filtering stats: {}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STATS_FETCH_FAILED",
                &format!("Failed to fetch filtering statistics: {}", err),
                None,
            )
        }
    }
}

/// POST /api/filtering/refresh
/// Force a synchronous rebuild of the filtering snapshot so downstream
/// consumers see the newly-saved configuration immediately.
async fn trigger_refresh() -> Response {
    match filtering::refresh().await {
        Ok(()) => {
            logger::info(
                LogTag::Filtering,
                "Filtering snapshot rebuilt via API request",
            );

            success_response(RefreshResponse {
                message: "Filtering snapshot rebuilt".to_string(),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            logger::info(
                LogTag::Filtering,
                &format!("Filtering refresh failed: {}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FILTERING_REFRESH_FAILED",
                &format!("Failed to rebuild filtering snapshot: {}", err),
                None,
            )
        }
    }
}

/// Reason label to human-readable display mapping
fn get_rejection_display_label(reason: &str) -> String {
    match reason {
        "no_decimals" => "No decimals in database",
        "token_too_new" => "Token too new",
        "cooldown_filtered" => "Cooldown filtered",
        "dex_data_missing" => "DexScreener data missing",
        "gecko_data_missing" => "GeckoTerminal data missing",
        "rug_data_missing" => "Rugcheck data missing",
        "dex_empty_name" => "Empty name",
        "dex_empty_symbol" => "Empty symbol",
        "dex_empty_logo" => "Empty logo URL",
        "dex_empty_website" => "Empty website URL",
        "dex_txn_5m" => "Low 5m transactions",
        "dex_txn_1h" => "Low 1h transactions",
        "dex_zero_liq" => "Zero liquidity",
        "dex_liq_low" => "Liquidity too low",
        "dex_liq_high" => "Liquidity too high",
        "dex_mcap_low" => "Market cap too low",
        "dex_mcap_high" => "Market cap too high",
        "dex_vol_low" => "Volume too low",
        "dex_vol_missing" => "Volume missing",
        "dex_fdv_low" => "FDV too low",
        "dex_fdv_high" => "FDV too high",
        "dex_fdv_missing" => "FDV missing",
        "dex_vol5m_low" => "5m volume too low",
        "dex_vol5m_missing" => "5m volume missing",
        "dex_vol1h_low" => "1h volume too low",
        "dex_vol1h_missing" => "1h volume missing",
        "dex_vol6h_low" => "6h volume too low",
        "dex_vol6h_missing" => "6h volume missing",
        "dex_price_change_5m_low" => "5m price change too low",
        "dex_price_change_5m_high" => "5m price change too high",
        "dex_price_change_5m_missing" => "5m price change missing",
        "dex_price_change_low" => "Price change too low",
        "dex_price_change_high" => "Price change too high",
        "dex_price_change_missing" => "Price change missing",
        "dex_price_change_6h_low" => "6h price change too low",
        "dex_price_change_6h_high" => "6h price change too high",
        "dex_price_change_6h_missing" => "6h price change missing",
        "dex_price_change_24h_low" => "24h price change too low",
        "dex_price_change_24h_high" => "24h price change too high",
        "dex_price_change_24h_missing" => "24h price change missing",
        "gecko_liq_missing" => "Liquidity missing",
        "gecko_liq_low" => "Liquidity too low",
        "gecko_liq_high" => "Liquidity too high",
        "gecko_mcap_missing" => "Market cap missing",
        "gecko_mcap_low" => "Market cap too low",
        "gecko_mcap_high" => "Market cap too high",
        "gecko_vol5m_low" => "5m volume too low",
        "gecko_vol5m_missing" => "5m volume missing",
        "gecko_vol1h_low" => "1h volume too low",
        "gecko_vol1h_missing" => "1h volume missing",
        "gecko_vol24h_low" => "24h volume too low",
        "gecko_vol24h_missing" => "24h volume missing",
        "gecko_price_change_5m_low" => "5m price change too low",
        "gecko_price_change_5m_high" => "5m price change too high",
        "gecko_price_change_5m_missing" => "5m price change missing",
        "gecko_price_change_1h_low" => "1h price change too low",
        "gecko_price_change_1h_high" => "1h price change too high",
        "gecko_price_change_1h_missing" => "1h price change missing",
        "gecko_price_change_24h_low" => "24h price change too low",
        "gecko_price_change_24h_high" => "24h price change too high",
        "gecko_price_change_24h_missing" => "24h price change missing",
        "gecko_pool_count_low" => "Pool count too low",
        "gecko_pool_count_high" => "Pool count too high",
        "gecko_pool_count_missing" => "Pool count missing",
        "gecko_reserve_low" => "Reserve too low",
        "gecko_reserve_missing" => "Reserve missing",
        "rug_rugged" => "Rugged token",
        "rug_score" => "Risk score too high",
        "rug_level_danger" => "Danger risk level",
        "rug_mint_authority" => "Mint authority present",
        "rug_freeze_authority" => "Freeze authority present",
        "rug_top_holder" => "Top holder % too high",
        "rug_top3_holders" => "Top 3 holders % too high",
        "rug_min_holders" => "Not enough holders",
        "rug_insider_count" => "Too many insider holders",
        "rug_insider_pct" => "Insider % too high",
        "rug_creator_pct" => "Creator balance too high",
        "rug_transfer_fee_present" => "Transfer fee present",
        "rug_transfer_fee_high" => "Transfer fee too high",
        "rug_transfer_fee_missing" => "Transfer fee data missing",
        "rug_graph_insiders" => "Graph insiders too high",
        "rug_lp_providers_low" => "LP providers too low",
        "rug_lp_providers_missing" => "LP providers missing",
        "rug_lp_lock_low" => "LP lock too low",
        "rug_lp_lock_missing" => "LP lock missing",
        _ => reason, // Return original if not mapped
    }
    .to_string()
}

#[derive(Debug, Serialize)]
struct RejectionStatEntry {
    reason: String,
    display_label: String,
    source: String,
    count: i64,
}

#[derive(Debug, Serialize)]
struct RejectionStatsResponse {
    stats: Vec<RejectionStatEntry>,
    by_source: HashMap<String, i64>,
    total_rejected: i64,
    timestamp: String,
}

/// GET /api/filtering/rejection-stats
/// Get counts of rejected tokens grouped by rejection reason
async fn get_rejection_stats() -> Response {
    match get_rejection_stats_async().await {
        Ok(raw_stats) => {
            let mut by_source: HashMap<String, i64> = HashMap::new();
            let mut total_rejected: i64 = 0;

            let stats: Vec<RejectionStatEntry> = raw_stats
                .into_iter()
                .map(|(reason, source, count)| {
                    total_rejected += count;
                    *by_source.entry(source.clone()).or_insert(0) += count;
                    RejectionStatEntry {
                        display_label: get_rejection_display_label(&reason).to_string(),
                        reason,
                        source,
                        count,
                    }
                })
                .collect();

            success_response(RejectionStatsResponse {
                stats,
                by_source,
                total_rejected,
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejection stats: {:?}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "REJECTION_STATS_FAILED",
                &format!("Failed to fetch rejection statistics: {:?}", err),
                None,
            )
        }
    }
}
