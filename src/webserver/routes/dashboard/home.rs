// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard home route — renders the main dashboard page with summary statistics.

use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::filtering::SnapshotState;
use crate::global::{
    POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
};
use crate::positions;
use crate::rpc::get_global_rpc_stats;
use crate::trader::is_trader_running;
use crate::wallet::WalletWorth;
use crate::webserver::promo;
use crate::webserver::snapshot::get_cached_system_metrics;
use crate::webserver::state::AppState;

use super::types::*;
use super::utils::format_uptime;

/// The hero's wallet block from the live worth. Without a snapshot (no wallet, or the
/// first balance read has not landed) the worth is unknown, so its default zeroes never
/// reach the hero as a measured balance. No baseline means the change is unknown, not
/// zero: falling back to the current worth reported a flat day the header shows as
/// unknown.
fn wallet_analytics(
    wallet_address: String,
    worth: &WalletWorth,
    start_of_day_balance_native: Option<f64>,
    native_price_usd: f64,
    balance_history: Vec<f64>,
) -> WalletAnalytics {
    let known = |value| worth.has_snapshot.then_some(value);
    let equity = known(worth.total_equity_native);
    WalletAnalytics {
        wallet_address,
        current_balance_native: known(worth.native_balance),
        token_count: worth.has_snapshot.then_some(worth.token_count),
        tokens_worth_native: known(worth.tokens_worth_native),
        total_equity_native: equity,
        unpriced_token_count: worth.has_snapshot.then_some(worth.unpriced_token_count),
        start_of_day_balance_native,
        change_native: equity
            .zip(start_of_day_balance_native)
            .map(|(equity, start)| equity - start),
        change_percent: equity
            .zip(start_of_day_balance_native.filter(|start| *start > 0.0))
            .map(|(equity, start)| (equity - start) / start * 100.0),
        native_price_usd,
        balance_history,
    }
}

/// GET /api/dashboard/home
/// Comprehensive home dashboard with all analytics
pub async fn get_home_dashboard(State(state): State<Arc<AppState>>) -> Json<HomeDashboardResponse> {
    // Return promotional fixtures only for owner-initiated media capture.
    if promo::are_promo_fixtures_enabled() {
        return Json(promo::get_promo_home_dashboard());
    }

    use chrono::{Duration, TimeZone};

    let now = chrono::Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| chrono::NaiveDateTime::default());
    let today_start = chrono::Utc.from_utc_datetime(&today_start);
    let yesterday_start = today_start - Duration::days(1);
    let week_start = today_start - Duration::days(7);
    let month_start = today_start - Duration::days(30);
    let epoch_start = chrono::Utc
        .timestamp_opt(0, 0)
        .earliest()
        .unwrap_or(today_start);

    // FULLY PARALLELIZED: Fetch ALL independent data sources concurrently
    // This reduces dashboard load time from ~2s to ~500ms (limited by slowest query)
    let (
        // Trader analytics (5 parallel SQL queries)
        (
            today_stats_result,
            yesterday_stats_result,
            week_stats_result,
            month_stats_result,
            alltime_stats_result,
        ),
        // Wallet data
        main_wallet_address_result,
        recent_snapshots_result,
        start_of_day_balance_result,
        // Positions data
        open_positions_result,
        // System metrics (cached, fast)
        cached_metrics,
        // Filtering stats
        filtering_stats_result,
    ) = tokio::join!(
        // Trader stats - 5 parallel SQL queries
        async {
            tokio::join!(
                positions::get_period_trading_stats(today_start, Some(now)),
                positions::get_period_trading_stats(yesterday_start, Some(today_start)),
                positions::get_period_trading_stats(week_start, Some(now)),
                positions::get_period_trading_stats(month_start, Some(now)),
                positions::get_period_trading_stats(epoch_start, Some(now)),
            )
        },
        // Main wallet public address
        crate::wallets::get_main_address(),
        // Wallet snapshots (newest first) — [0] is the current status, the rest
        // form the balance-trend sparkline.
        crate::wallet::get_recent_wallet_snapshots(30),
        // Start of day balance
        crate::wallet::get_balance_at_time(today_start),
        // Open positions
        positions::get_db_open_positions(),
        // System metrics (cached)
        get_cached_system_metrics(),
        // Filtering stats — the NON-BLOCKING read on purpose. This one fetch is what
        // clears the dashboard's first-paint skeleton, and the blocking variant waits up
        // to 30 seconds for the first snapshot to be built, so a freshly-launched app sat
        // in its loading state for that entire timeout on every launch. Counts that are
        // briefly absent cost nothing; a dashboard that will not paint costs everything.
        crate::filtering::try_fetch_stats(crate::chains::ChainScope::All),
    );

    // Convert from database PeriodTradingStats to dashboard TradingPeriodStats
    let convert_stats =
        |result: positions::Result<positions::PeriodTradingStats>| -> TradingPeriodStats {
            match result {
                Ok(stats) => TradingPeriodStats {
                    buys: stats.buys,
                    sells: stats.sells,
                    profit_native: stats.profit_native,
                    loss_native: stats.loss_native,
                    net_pnl_native: stats.net_pnl_native,
                    drawdown_percent: stats.drawdown_percent,
                    win_rate: stats.win_rate,
                },
                Err(_) => TradingPeriodStats {
                    buys: 0,
                    sells: 0,
                    profit_native: 0.0,
                    loss_native: 0.0,
                    net_pnl_native: 0.0,
                    drawdown_percent: 0.0,
                    win_rate: 0.0,
                },
            }
        };

    let trader = TraderAnalytics {
        today: convert_stats(today_stats_result),
        yesterday: convert_stats(yesterday_stats_result),
        this_week: convert_stats(week_stats_result),
        this_month: convert_stats(month_stats_result),
        all_time: convert_stats(alltime_stats_result),
    };

    // Wallet worth comes from the live in-memory snapshot, priced now — the SAME call
    // the header makes, so the two surfaces can never disagree. The snapshot list is only
    // used for the trend sparkline.
    //
    // (Summing `snapshot.token_balances` here used to be the bug: `get_recent_snapshots`
    // leaves that vector EMPTY, so the holdings value was permanently 0 and the hero's
    // "Portfolio Value" was really just cash.)
    let worth = crate::wallet::get_wallet_worth();
    let recent_snapshots = recent_snapshots_result.unwrap_or_default();

    // Oldest-first worth trend for the sparkline (reverse of newest-first). It plots the
    // same quantity as the headline above it — it used to plot cash while the headline
    // showed equity.
    let balance_history: Vec<f64> = recent_snapshots
        .iter()
        .rev()
        .map(|s| s.total_equity_native)
        .collect();

    let wallet = wallet_analytics(
        main_wallet_address_result.unwrap_or_default(),
        &worth,
        start_of_day_balance_result.ok().flatten(),
        crate::native_price::get_native_price(),
        balance_history,
    );

    // Process positions snapshot from parallel results
    let open_positions = open_positions_result.unwrap_or_default();
    let open_count = open_positions.len() as i64;
    // See `dashboard/overview.rs`: the basis is cumulative, and a round without one
    // contributes nothing.
    let total_invested_native: f64 = open_positions
        .iter()
        .filter(|p| p.has_trustworthy_pnl())
        .map(|p| p.total_size_native)
        .sum();

    // Calculate position P&L with performers
    let mut best_performer: Option<PositionPerformer> = None;
    let mut worst_performer: Option<PositionPerformer> = None;
    let mut total_hold_duration_mins: i64 = 0;
    let mut dca_count: i64 = 0;

    let unrealized_pnl_native: f64 = open_positions
        .iter()
        .filter_map(|p| {
            // Track DCA positions
            if p.dca_count > 0 {
                dca_count += 1;
            }

            // Calculate hold duration
            let hold_mins = (now - p.entry_time).num_minutes();
            total_hold_duration_mins += hold_mins;

            // A wallet-derived round with no cost basis has no honest P&L at all, and
            // including it as 0% would drag the best/worst performers toward zero.
            if !p.has_trustworthy_pnl() {
                return None;
            }

            // The WEIGHTED average entry is the position's real cost per token;
            // `entry_price` is only the first buy and misprices anything DCA'd into.
            let entry = if p.average_entry_price > 0.0 {
                p.average_entry_price
            } else {
                p.entry_price
            };

            if let Some(current) = p.current_price {
                // Rank on the same fee-aware P&L the Open P&L figure sums, so a
                // position cannot read as a gain here and a loss beside it. The
                // raw price move stands in only until the first price tick.
                let pnl_pct = p.unrealized_pnl_percent.unwrap_or(if entry > 0.0 {
                    ((current - entry) / entry) * 100.0
                } else {
                    0.0
                });

                // Track best/worst performers
                match &best_performer {
                    None => {
                        best_performer = Some(PositionPerformer {
                            symbol: p.symbol.clone(),
                            pnl_percent: pnl_pct,
                        });
                    }
                    Some(best) if pnl_pct > best.pnl_percent => {
                        best_performer = Some(PositionPerformer {
                            symbol: p.symbol.clone(),
                            pnl_percent: pnl_pct,
                        });
                    }
                    _ => {}
                }

                match &worst_performer {
                    None => {
                        worst_performer = Some(PositionPerformer {
                            symbol: p.symbol.clone(),
                            pnl_percent: pnl_pct,
                        });
                    }
                    Some(worst) if pnl_pct < worst.pnl_percent => {
                        worst_performer = Some(PositionPerformer {
                            symbol: p.symbol.clone(),
                            pnl_percent: pnl_pct,
                        });
                    }
                    _ => {}
                }

                // Prefer the P&L the position already booked (fee-aware, computed by
                // the price updater); fall back to the basis-scaled estimate only until
                // the first price tick lands.
                Some(p.unrealized_pnl.unwrap_or_else(|| {
                    if entry > 0.0 {
                        (current - entry) * p.total_size_native / entry
                    } else {
                        0.0
                    }
                }))
            } else {
                None
            }
        })
        .sum();

    let unrealized_pnl_percent = (total_invested_native > 0.0)
        .then(|| (unrealized_pnl_native / total_invested_native) * 100.0);
    let avg_position_size_native =
        (open_count > 0).then(|| total_invested_native / open_count as f64);
    let avg_hold_duration_mins = (open_count > 0).then(|| total_hold_duration_mins / open_count);

    let positions_snapshot = PositionsSnapshot {
        open_count,
        total_invested_native,
        unrealized_pnl_native,
        unrealized_pnl_percent,
        avg_position_size_native,
        avg_hold_duration_mins,
        best_performer,
        worst_performer,
        dca_count,
    };

    // Process system metrics (already fetched in parallel)
    let uptime_seconds = state.uptime_seconds();
    let uptime_formatted = format_uptime(uptime_seconds);

    let memory_mb = cached_metrics.process_memory_mb as f64;
    let memory_total_mb = cached_metrics.system_memory_total_mb as f64;
    let memory_percent = if memory_total_mb > 0.0 {
        (memory_mb / memory_total_mb) * 100.0
    } else {
        0.0
    };
    let cpu_percent = cached_metrics.cpu_process_percent as f64;

    // Get RPC stats
    let rpc_stats = get_global_rpc_stats();
    let (rpc_calls_per_min, rpc_success_rate) = match rpc_stats {
        Some(stats) => {
            let calls_per_min = stats.calls_per_second() * 60.0;
            let total = stats.total_calls();
            let errors = stats.total_errors();
            let success_rate = if total > 0 {
                ((total - errors) as f64 / total as f64) * 100.0
            } else {
                100.0
            };
            (calls_per_min, success_rate)
        }
        None => (0.0, 100.0),
    };

    // Get service health - use global flags for simplicity
    let services_healthy = [
        TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
        POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
        POOL_SERVICE_READY.load(std::sync::atomic::Ordering::Relaxed),
        TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
    ]
    .iter()
    .filter(|&&x| x)
    .count();
    let services_total = 4;

    // Check WebSocket status (transactions system ready implies WebSocket connected)
    let websocket_connected = TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed);

    let system = SystemMetrics {
        uptime_seconds,
        uptime_formatted,
        memory_mb,
        memory_percent,
        cpu_percent,
        rpc_calls_per_min,
        rpc_success_rate,
        websocket_connected,
        services_healthy,
        services_total,
    };

    // Process token statistics (filtering already fetched in parallel).
    //
    // Via the async wrapper, which runs the query on a blocking thread. Called directly,
    // `count_tokens()` takes the token database's connection lock ON THE ASYNC WORKER
    // THREAD, so while a filtering snapshot held that connection this one line parked a
    // whole tokio worker for the duration — measured at 8.9s against the owner's database.
    // With few workers, a couple of concurrent dashboard polls doing this starved the
    // runtime, which is why unrelated panels (wallet, positions) stalled together.
    let total_in_database = crate::tokens::count_tokens_async(crate::chains::ChainScope::All)
        .await
        .unwrap_or_default();

    // Get filtering stats from the already fetched result (absent until the first snapshot
    // finishes building in the background). Absent stays absent all the way to the hero:
    // zeroing these would have the panel state that nothing passed and nothing is priced,
    // which is a different claim from "not counted yet".
    let filtering_stats = filtering_stats_result;

    let passed_filters = filtering_stats.as_ref().map(|s| s.passed_filtering);
    let with_prices = filtering_stats.as_ref().map(|s| s.with_pool_price);
    let blacklisted = filtering_stats.as_ref().map(|s| s.blacklisted);
    let with_ohlcv = filtering_stats.as_ref().map(|s| s.with_ohlcv);

    // Calculate rejected as total - passed - blacklisted
    let rejected_filters = passed_filters
        .zip(blacklisted)
        .map(|(passed, blacklisted)| {
            total_in_database
                .saturating_sub(passed)
                .saturating_sub(blacklisted)
        });

    let tokens = TokenStatistics {
        snapshot_state: SnapshotState::of(&filtering_stats),
        total_in_database,
        with_prices,
        passed_filters,
        rejected_filters,
        blacklisted,
        with_ohlcv,
        found_today: 0,      // Would need timestamp tracking in DB
        found_this_week: 0,  // Would need timestamp tracking in DB
        found_this_month: 0, // Would need timestamp tracking in DB
        found_all_time: total_in_database,
    };

    // Get trader status (always off in Explore Mode — trading is disabled)
    let trader_status = TraderStatusInfo {
        running: !crate::global::is_explore_mode() && is_trader_running(),
    };

    Json(HomeDashboardResponse {
        trader,
        wallet,
        positions: positions_snapshot,
        system,
        tokens,
        trader_status,
        timestamp: now.to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worth(has_snapshot: bool) -> WalletWorth {
        WalletWorth {
            native_balance: 1.5,
            tokens_worth_native: 0.5,
            total_equity_native: 2.0,
            token_count: 3,
            unpriced_token_count: 1,
            has_snapshot,
            ..WalletWorth::default()
        }
    }

    #[test]
    fn wallet_without_snapshot_reports_no_worth() {
        let wallet = wallet_analytics(String::new(), &worth(false), Some(1.0), 150.0, vec![]);
        assert_eq!(wallet.current_balance_native, None);
        assert_eq!(wallet.tokens_worth_native, None);
        assert_eq!(wallet.total_equity_native, None);
        assert_eq!(wallet.token_count, None);
        assert_eq!(wallet.unpriced_token_count, None);
        assert_eq!(wallet.change_native, None);
        assert_eq!(wallet.change_percent, None);
    }

    #[test]
    fn wallet_with_snapshot_reports_worth_and_change() {
        let wallet = wallet_analytics("addr".to_owned(), &worth(true), Some(1.0), 150.0, vec![]);
        assert_eq!(wallet.current_balance_native, Some(1.5));
        assert_eq!(wallet.total_equity_native, Some(2.0));
        assert_eq!(wallet.token_count, Some(3));
        assert_eq!(wallet.change_native, Some(1.0));
        assert_eq!(wallet.change_percent, Some(100.0));
    }

    #[test]
    fn wallet_without_baseline_reports_no_change() {
        let wallet = wallet_analytics("addr".to_owned(), &worth(true), None, 150.0, vec![]);
        assert_eq!(wallet.total_equity_native, Some(2.0));
        assert_eq!(wallet.change_native, None);
        assert_eq!(wallet.change_percent, None);
    }
}
