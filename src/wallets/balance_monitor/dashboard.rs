//! Dashboard data computation and API

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::transactions::get_transaction_database;

use super::cache::*;
use super::database::GLOBAL_WALLET_DB;
use super::service::{
    get_recent_wallet_snapshots, get_snapshot_nft_balances, get_snapshot_token_balances,
    initialize_wallet_database,
};
use super::types::*;

const TOKEN_METADATA_CONCURRENCY: usize = 20;
const MAX_API_CACHE_ENTRIES: usize = 128;

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
// FLOW METRICS COMPUTATION
// =============================================================================

async fn compute_flow_metrics(window_hours: i64) -> Result<WalletFlowMetrics, String> {
    logger::debug(
        LogTag::Wallet,
        &format!("Computing flow metrics for window: {window_hours} hours"),
    );

    // All-time mode when window_hours <= 0
    if window_hours <= 0 {
        if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
            if let Ok(Some(min_ts)) = db.get_flow_cache_min_ts_sync() {
                if let Ok((inflow, outflow, tx_count)) =
                    db.aggregate_cached_flows_sync(min_ts, None)
                {
                    if tx_count > 0 {
                        logger::debug(
                            LogTag::Wallet,
                            &format!(
                                "All-time cached: inflow={:.6}, outflow={:.6}, txs={}",
                                inflow, outflow, tx_count
                            ),
                        );
                        return Ok(WalletFlowMetrics {
                            window_hours: 0,
                            inflow_sol: inflow,
                            outflow_sol: outflow,
                            net_sol: inflow - outflow,
                            transactions_analyzed: tx_count,
                        });
                    }
                }
            }
        }
        // Fallback to full aggregation from transactions DB (from epoch)
        let tx_db = get_transaction_database()
            .await
            .ok_or_else(|| "Transaction database not initialized".to_owned())?;
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let (inflow, outflow, tx_count) = tx_db
            .aggregate_sol_flows_since(epoch, None)
            .await
            .map_err(|e| format!("Failed to aggregate all-time SOL flows: {e}"))?;
        logger::debug(
            LogTag::Wallet,
            &format!(
                "All-time DB: inflow={:.6}, outflow={:.6}, txs={}",
                inflow, outflow, tx_count
            ),
        );
        return Ok(WalletFlowMetrics {
            window_hours: 0,
            inflow_sol: inflow,
            outflow_sol: outflow,
            net_sol: inflow - outflow,
            transactions_analyzed: tx_count,
        });
    }

    let window_hours = clamp_window_hours(window_hours);
    let window_start = Utc::now() - ChronoDuration::hours(window_hours);

    logger::debug(
        LogTag::Wallet,
        &format!("Window start: {}", window_start.to_rfc3339()),
    );

    // Try cached aggregation first
    if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
        match db.aggregate_cached_flows_sync(window_start, None) {
            Ok((inflow, outflow, tx_count)) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "Cached: inflow={:.6}, outflow={:.6}, txs={}",
                        inflow, outflow, tx_count
                    ),
                );
                if tx_count > 0 {
                    return Ok(WalletFlowMetrics {
                        window_hours,
                        inflow_sol: inflow,
                        outflow_sol: outflow,
                        net_sol: inflow - outflow,
                        transactions_analyzed: tx_count,
                    });
                }
            }
            Err(e) => {
                logger::debug(LogTag::Wallet, &format!("Cache aggregation failed: {e}"));
            }
        }
    }

    // Fallback to live aggregation from transactions DB
    logger::debug(
        LogTag::Wallet,
        "Using live aggregation from transactions DB",
    );

    let tx_db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_owned())?;
    let (inflow, outflow, tx_count) = tx_db
        .aggregate_sol_flows_since(window_start, None)
        .await
        .map_err(|e| format!("Failed to aggregate SOL flows: {e}"))?;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "DB aggregation: inflow={:.6}, outflow={:.6}, txs={}",
            inflow, outflow, tx_count
        ),
    );

    Ok(WalletFlowMetrics {
        window_hours,
        inflow_sol: inflow,
        outflow_sol: outflow,
        net_sol: inflow - outflow,
        transactions_analyzed: tx_count,
    })
}

async fn compute_daily_flows(window_hours: i64) -> Result<Vec<DailyFlowPoint>, String> {
    let window_hours = clamp_window_hours(window_hours);
    let (window_start, is_all_time) = if window_hours == 0 {
        (DateTime::<Utc>::from(std::time::UNIX_EPOCH), true)
    } else {
        (Utc::now() - ChronoDuration::hours(window_hours), false)
    };

    let tx_db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_owned())?;

    let daily_data = tx_db
        .aggregate_daily_flows(window_start, None)
        .await
        .map_err(|e| format!("Failed to aggregate daily flows: {e}"))?;

    // Convert to DailyFlowPoint with timestamps
    let mut result: Vec<DailyFlowPoint> = daily_data
        .into_iter()
        .filter_map(|(date_str, inflow, outflow, tx_count)| {
            // Parse date string and convert to timestamp
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|naive_dt| DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc))
                .map(|dt| DailyFlowPoint {
                    date: date_str,
                    timestamp: dt.timestamp(),
                    inflow,
                    outflow,
                    net: inflow - outflow,
                    tx_count,
                })
        })
        .collect();

    // Apply payload cap/decimation for very long ranges to avoid huge responses
    let (max_days, decimate_threshold_days) = with_config(|cfg| {
        (
            cfg.wallet.max_daily_flow_days,
            cfg.wallet.daily_flow_decimate_threshold_days,
        )
    });

    if result.len() > max_days {
        // Keep most recent max_days points
        result.sort_by_key(|p| p.timestamp);
        result = result.split_off(result.len() - max_days);
    }

    if result.len() > decimate_threshold_days {
        // Decimate older half to every Nth point while keeping recent quarter dense
        let len = result.len();
        let recent_keep = len / 4; // keep last quarter in full resolution
        let (older, recent) = result.split_at(len - recent_keep);
        // Choose stride to reduce older to about half of decimate_threshold_days
        let target_older = decimate_threshold_days - recent_keep.min(decimate_threshold_days / 2);
        let stride = ((older.len() as f64) / (target_older as f64))
            .ceil()
            .max(1.0) as usize;
        let decimated_older: Vec<DailyFlowPoint> = older
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if i % stride == 0 {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect();
        let mut merged = decimated_older;
        merged.extend_from_slice(recent);
        result = merged;
    }

    logger::debug(
        LogTag::Wallet,
        &format!("Computed {} daily flow points", result.len()),
    );

    Ok(result)
}

// =============================================================================
// TOKEN METADATA ENRICHMENT
// =============================================================================

async fn fetch_token_metadata_batch(
    mints: &[String],
) -> HashMap<String, crate::tokens::types::Token> {
    if mints.is_empty() {
        return HashMap::new();
    }

    stream::iter(mints.iter().cloned())
        .map(|mint| async move {
            match crate::tokens::get_full_token_async(&mint).await {
                Ok(Some(token)) => Some((mint, token)),
                Ok(None) => None,
                Err(err) => {
                    logger::debug(
                        LogTag::Wallet,
                        &format!("Failed to load token metadata for {mint}: {err}"),
                    );
                    None
                }
            }
        })
        .buffer_unordered(TOKEN_METADATA_CONCURRENCY)
        .filter_map(|entry| async move { entry })
        .collect()
        .await
}

async fn enrich_token_overview(
    balances: Vec<TokenBalance>,
    max_tokens: usize,
) -> Vec<WalletTokenOverview> {
    let mut rows = Vec::with_capacity(balances.len());

    let mut unique_mints: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for balance in &balances {
        if seen.insert(balance.mint.clone()) {
            unique_mints.push(balance.mint.clone());
        }
    }

    let metadata_map: HashMap<String, crate::tokens::types::Token> =
        fetch_token_metadata_batch(&unique_mints).await;

    for balance in balances {
        let token_meta = metadata_map.get(&balance.mint);

        let (
            symbol,
            name,
            image_url,
            price_sol,
            price_usd,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        ) = if let Some(meta) = token_meta {
            let price_sol = if meta.price_sol > 0.0 {
                Some(meta.price_sol)
            } else {
                None
            };
            let price_usd = if meta.price_usd > 0.0 {
                Some(meta.price_usd)
            } else {
                None
            };
            let liquidity_usd = meta.liquidity_usd;
            let volume_24h = meta.volume_h24;
            let last_updated = Some(meta.market_data_last_fetched_at.to_rfc3339());
            let dex_id = Some(meta.data_source.as_str().to_owned());

            let symbol = if meta.symbol.trim().is_empty() {
                short_mint_label(&balance.mint)
            } else {
                meta.symbol.clone()
            };

            (
                symbol,
                Some(meta.name.clone()),
                meta.image_url.clone(),
                price_sol,
                price_usd,
                liquidity_usd,
                volume_24h,
                last_updated,
                dex_id,
            )
        } else {
            (
                short_mint_label(&balance.mint),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };

        let value_sol = price_sol.map(|price| price * balance.balance_ui);

        rows.push(WalletTokenOverview {
            mint: balance.mint.clone(),
            symbol,
            name,
            image_url,
            balance_ui: balance.balance_ui,
            balance_raw: balance.balance,
            decimals: balance.decimals,
            is_token_2022: balance.is_token_2022,
            price_sol,
            price_usd,
            value_sol,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        });
    }

    rows.sort_by(|a, b| {
        let a_key = a.value_sol.unwrap_or(a.balance_ui);
        let b_key = b.value_sol.unwrap_or(b.balance_ui);
        b_key
            .partial_cmp(&a_key)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_tokens = clamp_token_limit(max_tokens);
    if rows.len() > max_tokens {
        rows.truncate(max_tokens);
    }

    rows
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
        let daily_flows = compute_daily_flows(window_hours)
            .await
            .unwrap_or_default();
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
