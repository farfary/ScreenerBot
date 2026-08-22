//! Flow metrics computation helpers for wallet balance monitor dashboard.

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::transactions::get_transaction_database;

use super::super::database::GLOBAL_WALLET_DB;
use super::super::types::{DailyFlowPoint, WalletFlowMetrics};
use super::clamp_window_hours;

pub(super) async fn compute_flow_metrics(window_hours: i64) -> Result<WalletFlowMetrics, String> {
    logger::debug(
        LogTag::Wallet,
        &format!("Computing flow metrics for window: {window_hours} hours"),
    );

    // All-time mode when window_hours <= 0
    if window_hours <= 0 {
        if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
            if let Ok(Some(min_ts)) = db.get_flow_cache_min_ts() {
                if let Ok((inflow, outflow, tx_count)) = db.aggregate_cached_flows(min_ts, None) {
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
        match db.aggregate_cached_flows(window_start, None) {
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

pub(super) async fn compute_daily_flows(window_hours: i64) -> Result<Vec<DailyFlowPoint>, String> {
    let window_hours = clamp_window_hours(window_hours);
    let (window_start, _is_all_time) = if window_hours == 0 {
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
