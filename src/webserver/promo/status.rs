//! Promo overlay for `GET /api/status`, the source of the dashboard status bar.
//!
//! The live snapshot is still gathered, so the version and every service block
//! stay real. Each field the status bar renders is then replaced with the value
//! the promo header and home dashboard already state; without this the footer of
//! every capture reported the empty live session ("Trading Inactive", no
//! positions, no RPC traffic) under a header showing a running trader.

use chrono::{Duration, Utc};

use crate::webserver::snapshot::{
    RpcStatsSnapshot, StatusSnapshot, WalletStatusSnapshot, WalletTokenBalanceSnapshot,
};

use super::aggregates;
use super::data::*;
use super::wallet::get_promo_wallet_current;

/// Replace the live values of a status snapshot with the promo session's.
pub fn apply_promo_status(snapshot: &mut StatusSnapshot) {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let trades = aggregates::closed_trades(now);
    let today = aggregates::within_hours(&trades, now, 24);

    snapshot.uptime_seconds = PROMO_UPTIME_SECS;
    snapshot.uptime_formatted = PROMO_UPTIME_STR.to_owned();
    snapshot.trading_enabled = true;
    snapshot.trader_running = true;
    snapshot.open_positions = open.count;
    snapshot.closed_positions_today = today.sells.max(0) as usize;
    snapshot.sol_balance = PROMO_SOL_BALANCE;
    snapshot.usdc_balance = 0.0;

    let memory_mb = PROMO_MEMORY_MB.round() as u64;
    let metrics = &mut snapshot.metrics;
    metrics.memory_usage_mb = memory_mb;
    metrics.process_memory_mb = memory_mb;
    metrics.cpu_usage_percent = PROMO_CPU_PERCENT as f32;
    metrics.cpu_process_percent = PROMO_CPU_PERCENT as f32;
    metrics.rpc_calls_total = PROMO_RPC_TOTAL_CALLS;
    metrics.rpc_calls_failed = promo_rpc_errors();
    metrics.rpc_success_rate = PROMO_RPC_SUCCESS_PERCENT as f32;
    metrics.rpc_calls_per_minute_recent = PROMO_RPC_CALLS_PER_MINUTE;

    snapshot.rpc_stats = Some(RpcStatsSnapshot {
        total_calls: PROMO_RPC_TOTAL_CALLS,
        total_errors: promo_rpc_errors(),
        success_rate: PROMO_RPC_SUCCESS_PERCENT as f32,
        calls_per_second: PROMO_RPC_CALLS_PER_MINUTE / 60.0,
        average_response_time_ms: PROMO_RPC_LATENCY_MS as f64,
        calls_per_url: Default::default(),
        errors_per_url: Default::default(),
        calls_per_method: Default::default(),
        errors_per_method: Default::default(),
        uptime_seconds: PROMO_UPTIME_SECS as i64,
        session_id: "promo".to_owned(),
        session_started_at: now - Duration::seconds(PROMO_UPTIME_SECS as i64),
        recent_calls_per_minute: PROMO_RPC_CALLS_PER_MINUTE,
        minute_buckets: Vec::new(),
        last_session: None,
    });

    // The live wallet block would publish the operator's own holdings.
    let wallet = get_promo_wallet_current();
    snapshot.wallet = Some(WalletStatusSnapshot {
        sol_balance: wallet.sol_balance,
        sol_balance_lamports: wallet.sol_balance_lamports,
        usdc_balance: 0.0,
        total_tokens_count: wallet.total_tokens_count,
        snapshot_time: Some(now),
        token_balances: wallet
            .token_balances
            .into_iter()
            .map(|t| WalletTokenBalanceSnapshot {
                mint: t.mint,
                balance: t.balance,
                balance_ui: t.balance_ui,
                decimals: t.decimals,
                is_token_2022: t.is_token_2022,
            })
            .collect(),
    });
}

fn promo_rpc_errors() -> u64 {
    (PROMO_RPC_TOTAL_CALLS as f64 * (100.0 - PROMO_RPC_SUCCESS_PERCENT) / 100.0).round() as u64
}
