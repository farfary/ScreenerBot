//! Dashboard data computation and API

use chrono::{Duration as ChronoDuration, Utc};
use std::time::{Duration, Instant};

use crate::config::with_config;
use crate::logger::{self, LogTag};

use super::cache::*;
use super::database::GLOBAL_WALLET_DB;
use super::service::{
    get_recent_wallet_snapshots, get_snapshot_nft_balances, get_snapshot_token_balances,
    initialize_wallet_database,
};
use super::types::*;

const TOKEN_METADATA_CONCURRENCY: usize = 20;
const MAX_API_CACHE_ENTRIES: usize = 128;

mod flow_metrics;
mod token_metadata;

use flow_metrics::{compute_daily_flows, compute_flow_metrics};
use token_metadata::enrich_token_overview;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

pub(super) fn clamp_window_hours(window_hours: i64) -> i64 {
    // 0 = All Time (no filter)
    // Otherwise clamp to reasonable range (1 hour to 2 years)
    if window_hours == 0 {
        0
    } else {
        window_hours.clamp(1, 24 * 365 * 2)
    }
}

pub(super) fn clamp_snapshot_limit(limit: usize) -> usize {
    limit.clamp(16, 2880)
}

pub(super) fn clamp_token_limit(limit: usize) -> usize {
    limit.clamp(10, 1000)
}

fn calc_change_percent(current: f64, previous: f64) -> Option<f64> {
    if previous.abs() < f64::EPSILON {
        None
    } else {
        Some((current - previous) / previous * 100.0)
    }
}

fn short_mint_label(mint: &str) -> String {
    if mint.len() <= 4 {
        mint.to_string()
    } else {
        format!("{}…", &mint[..4])
    }
}

// =============================================================================
// DASHBOARD PAYLOAD COMPUTATION
// =============================================================================

pub(super) async fn compute_dashboard_payload_realtime(
    window_hours: i64,
    snapshot_limit: usize,
    max_tokens: usize,
) -> Result<WalletDashboardData, String> {
    let window_hours = clamp_window_hours(window_hours);
    let snapshot_limit = clamp_snapshot_limit(snapshot_limit);

    let mut snapshots = match get_recent_wallet_snapshots(snapshot_limit).await {
        Ok(snaps) => snaps,
        Err(err) => {
            if err.contains("not initialized") {
                if let Err(init_err) = initialize_wallet_database().await {
                    return Err(format!("Wallet database unavailable: {init_err}"));
                }

                match get_recent_wallet_snapshots(snapshot_limit).await {
                    Ok(snaps) => snaps,
                    Err(retry_err) => {
                        if retry_err.contains("not initialized") {
                            Vec::new()
                        } else {
                            return Err(retry_err);
                        }
                    }
                }
            } else {
                return Err(err);
            }
        }
    };
    if snapshots.is_empty() {
        let flows = compute_flow_metrics(window_hours).await?;
        let daily_flows = compute_daily_flows(window_hours).await.unwrap_or_default();
        return Ok(WalletDashboardData {
            summary: WalletSummarySnapshot {
                window_hours,
                current_sol_balance: 0.0,
                previous_sol_balance: None,
                sol_change: 0.0,
                sol_change_percent: None,
                token_count: 0,
                last_snapshot_time: None,
            },
            flows,
            balance_trend: Vec::new(),
            daily_flows,
            tokens: Vec::new(),
            nfts: Vec::new(),
            last_updated: None,
            cache_metadata: None,
        });
    }

    snapshots.sort_by(|a, b| a.snapshot_time.cmp(&b.snapshot_time));

    let latest_snapshot = snapshots
        .last()
        .cloned()
        .ok_or_else(|| "Latest snapshot unavailable".to_owned())?;
    // Determine window_start for trend; for all-time (0), include all loaded snapshots
    let window_start = if window_hours == 0 {
        snapshots
            .first()
            .map(|s| s.snapshot_time)
            .unwrap_or(latest_snapshot.snapshot_time)
    } else {
        Utc::now() - ChronoDuration::hours(window_hours)
    };

    let baseline_snapshot = snapshots
        .iter()
        .find(|snap| snap.snapshot_time >= window_start)
        .or_else(|| snapshots.first())
        .cloned();

    let previous_sol_balance = baseline_snapshot.as_ref().map(|snap| snap.sol_balance);
    let sol_change =
        latest_snapshot.sol_balance - previous_sol_balance.unwrap_or(latest_snapshot.sol_balance);
    let sol_change_percent = previous_sol_balance
        .and_then(|prev| calc_change_percent(latest_snapshot.sol_balance, prev));

    let mut trend: Vec<WalletBalancePoint> = snapshots
        .iter()
        .filter(|snap| snap.snapshot_time >= window_start)
        .map(|snap| WalletBalancePoint {
            timestamp: snap.snapshot_time.timestamp(),
            sol_balance: snap.sol_balance,
        })
        .collect();

    if trend.is_empty() {
        trend.push(WalletBalancePoint {
            timestamp: latest_snapshot.snapshot_time.timestamp(),
            sol_balance: latest_snapshot.sol_balance,
        });
    }

    let mut tokens = Vec::new();
    let mut nfts = Vec::new();
    if let Some(snapshot_id) = latest_snapshot.id {
        let balances = get_snapshot_token_balances(snapshot_id).await?;
        tokens = enrich_token_overview(balances, max_tokens).await;

        // Get NFT balances
        let nft_balances = get_snapshot_nft_balances(snapshot_id)
            .await
            .unwrap_or_default();
        nfts = nft_balances
            .into_iter()
            .map(|nft| WalletNftOverview {
                mint: nft.mint,
                account_address: nft.account_address,
                name: nft.name,
                symbol: nft.symbol,
                image_url: nft.image_url,
                is_token_2022: nft.is_token_2022,
            })
            .collect();
    }

    let flows = compute_flow_metrics(window_hours).await?;

    // Compute daily flows for chart
    let daily_flows = compute_daily_flows(window_hours).await.unwrap_or_else(|e| {
        logger::warning(
            LogTag::Wallet,
            &format!("Failed to compute daily flows: {e}"),
        );
        Vec::new()
    });

    let summary = WalletSummarySnapshot {
        window_hours,
        current_sol_balance: latest_snapshot.sol_balance,
        previous_sol_balance,
        sol_change,
        sol_change_percent,
        token_count: latest_snapshot.total_tokens_count,
        last_snapshot_time: Some(latest_snapshot.snapshot_time.to_rfc3339()),
    };

    Ok(WalletDashboardData {
        summary,
        flows,
        balance_trend: trend,
        daily_flows,
        tokens,
        nfts,
        last_updated: Some(latest_snapshot.snapshot_time.to_rfc3339()),
        cache_metadata: None,
    })
}

// =============================================================================
// PUBLIC API
// =============================================================================

pub async fn get_wallet_dashboard_data(
    window_hours: i64,
    snapshot_limit: usize,
    max_tokens: usize,
) -> Result<WalletDashboardData, String> {
    let clamped_window = clamp_window_hours(window_hours);
    let clamped_snapshot_limit = clamp_snapshot_limit(snapshot_limit);
    let clamped_token_limit = clamp_token_limit(max_tokens);

    let cache_ttl_secs = with_config(|cfg| cfg.wallet.api_response_cache_ttl_secs.max(5));
    let request_key = DashboardRequestKey {
        window_hours: clamped_window,
        snapshot_limit: clamped_snapshot_limit,
        max_tokens: clamped_token_limit,
    };

    let start = Instant::now();

    // Memory cache layer
    {
        if let Some(entry) = API_RESPONSE_CACHE.get(&request_key) {
            if entry.cached_at.elapsed().as_secs() < cache_ttl_secs {
                let payload = entry.data.clone();
                let stale = payload
                    .cache_metadata
                    .as_ref()
                    .map(|meta| matches!(meta.freshness, DashboardCacheFreshness::Stale))
                    .unwrap_or_default();
                record_cache_metrics(
                    DashboardDataSource::Memory,
                    start.elapsed().as_millis(),
                    stale,
                )
                .await;
                return Ok(payload);
            }
        }
    }

    // Database cache layer
    if let Some((window_key, _canonical_hours)) = canonical_window(clamped_window) {
        let metrics = {
            let db_guard = GLOBAL_WALLET_DB.lock().await;
            match db_guard.as_ref() {
                Some(db) => db.get_dashboard_metrics(window_key)?,
                None => None,
            }
        };

        if let Some(metrics) = metrics {
            let covers_snapshots = metrics.snapshot_limit >= clamped_snapshot_limit;
            let covers_tokens = metrics.token_limit >= clamped_token_limit;
            let ttl_secs = ttl_for_window(window_key).max(5);
            let now = Utc::now();
            let valid = metrics.valid_until >= now;

            if covers_snapshots && covers_tokens {
                match deserialize_dashboard_payload(&metrics.payload) {
                    Ok(mut payload) => {
                        if payload.balance_trend.len() > clamped_snapshot_limit {
                            let start_index = payload.balance_trend.len() - clamped_snapshot_limit;
                            payload.balance_trend = payload
                                .balance_trend
                                .into_iter()
                                .skip(start_index)
                                .collect();
                        }
                        if payload.tokens.len() > clamped_token_limit {
                            payload.tokens.truncate(clamped_token_limit);
                        }

                        let age_secs = now
                            .signed_duration_since(metrics.computed_at)
                            .num_seconds()
                            .max(0) as u64;
                        let next_update = if metrics.valid_until > now {
                            Some((metrics.valid_until - now).num_seconds() as u64)
                        } else {
                            Some(0)
                        };

                        let freshness = if !valid {
                            DashboardCacheFreshness::Stale
                        } else if age_secs <= ttl_secs / 2 {
                            DashboardCacheFreshness::Fresh
                        } else {
                            DashboardCacheFreshness::Aging
                        };

                        let metadata = DashboardCacheMetadata {
                            window_key: Some(metrics.window_key.clone()),
                            cached_at: Some(metrics.computed_at.to_rfc3339()),
                            valid_until: Some(metrics.valid_until.to_rfc3339()),
                            age_seconds: Some(age_secs),
                            next_update_in_seconds: next_update,
                            freshness: freshness.clone(),
                            source: DashboardDataSource::Database,
                            computation_duration_ms: metrics
                                .computation_duration_ms
                                .map(|value| value as u64),
                            snapshot_count: Some(metrics.snapshot_count),
                        };
                        payload.cache_metadata = Some(metadata.clone());

                        if valid {
                            {
                                API_RESPONSE_CACHE.insert(
                                    request_key.clone(),
                                    CachedDashboardResponse {
                                        data: payload.clone(),
                                        cached_at: Instant::now(),
                                    },
                                );
                                // Moka automatically handles eviction based on max_capacity and TTL
                            }

                            record_cache_metrics(
                                DashboardDataSource::Database,
                                start.elapsed().as_millis(),
                                matches!(freshness, DashboardCacheFreshness::Stale),
                            )
                            .await;
                            return Ok(payload);
                        } else {
                            logger::debug(
                                LogTag::Wallet,
                                &format!(
                                    "Discarding stale dashboard cache for {} (age={}s, ttl={}s)",
                                    window_key, age_secs, ttl_secs
                                ),
                            );
                        }
                    }
                    Err(err) => {
                        logger::warning(
                            LogTag::Wallet,
                            &format!(
                                "Failed to deserialize dashboard cache for {}: {}",
                                window_key, err
                            ),
                        );
                    }
                }
            } else {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "Cache entry {} does not cover requested limits (snapshots={} tokens={})",
                        window_key, metrics.snapshot_limit, metrics.token_limit
                    ),
                );
            }

            if !valid {
                let cached_window_key = metrics.window_key.clone();
                let cached_window_hours = metrics.window_hours;
                tokio::spawn(async move {
                    if let Some((canonical_key, canonical_hours)) =
                        canonical_window(cached_window_hours)
                    {
                        if canonical_key == cached_window_key {
                            compute_and_cache_metrics_internal(canonical_key, canonical_hours)
                                .await;
                        }
                    }
                });
            }
        }
    }

    // Real-time computation fallback
    let mut payload = compute_dashboard_payload_realtime(
        clamped_window,
        clamped_snapshot_limit,
        clamped_token_limit,
    )
    .await?;

    let latency = start.elapsed().as_millis();
    let now = Utc::now();
    payload.cache_metadata = Some(DashboardCacheMetadata {
        window_key: canonical_window(clamped_window).map(|(key, _)| key.to_string()),
        cached_at: Some(now.to_rfc3339()),
        valid_until: None,
        age_seconds: Some(0),
        next_update_in_seconds: None,
        freshness: DashboardCacheFreshness::Realtime,
        source: DashboardDataSource::Realtime,
        computation_duration_ms: Some(latency as u64),
        snapshot_count: Some(payload.balance_trend.len()),
    });

    {
        API_RESPONSE_CACHE.insert(
            request_key,
            CachedDashboardResponse {
                data: payload.clone(),
                cached_at: Instant::now(),
            },
        );
        // Moka automatically handles eviction based on max_capacity and TTL
    }

    record_cache_metrics(DashboardDataSource::Realtime, latency, false).await;

    Ok(payload)
}
