//! Dashboard home route — renders the main dashboard page with summary statistics.

use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::global::{
    POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
};
use crate::positions;
use crate::rpc::get_global_rpc_stats;
use crate::trader::is_trader_running;
use crate::webserver::demo;
use crate::webserver::snapshot::get_cached_system_metrics;
use crate::webserver::state::AppState;

use super::types::*;
use super::utils::format_uptime;

/// GET /api/dashboard/home
/// Comprehensive home dashboard with all analytics
pub async fn get_home_dashboard(State(state): State<Arc<AppState>>) -> Json<HomeDashboardResponse> {
    // Return demo data if demo mode is enabled
    if demo::is_demo_mode() {
        return Json(demo::get_demo_home_dashboard());
    }

    use chrono::{DateTime, Duration, TimeZone};

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
        crate::filtering::try_fetch_stats(),
    );

    // Convert from database PeriodTradingStats to dashboard TradingPeriodStats
    let convert_stats =
        |result: Result<positions::PeriodTradingStats, String>| -> TradingPeriodStats {
            match result {
                Ok(stats) => TradingPeriodStats {
                    buys: stats.buys,
                    sells: stats.sells,
                    profit_sol: stats.profit_sol,
                    loss_sol: stats.loss_sol,
                    net_pnl_sol: stats.net_pnl_sol,
                    drawdown_percent: stats.drawdown_percent,
                    win_rate: stats.win_rate,
                },
                Err(_) => TradingPeriodStats {
                    buys: 0,
                    sells: 0,
                    profit_sol: 0.0,
                    loss_sol: 0.0,
                    net_pnl_sol: 0.0,
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

    let start_of_day_balance_sol = start_of_day_balance_result
        .ok()
        .flatten()
        .unwrap_or(worth.total_equity_sol);

    let change_sol = worth.total_equity_sol - start_of_day_balance_sol;
    let change_percent = if start_of_day_balance_sol > 0.0 {
        (change_sol / start_of_day_balance_sol) * 100.0
    } else {
        0.0
    };

    // Oldest-first worth trend for the sparkline (reverse of newest-first). It plots the
    // same quantity as the headline above it — it used to plot cash while the headline
    // showed equity.
    let balance_history: Vec<f64> = recent_snapshots
        .iter()
        .rev()
        .map(|s| s.total_equity_sol)
        .collect();

    let wallet = WalletAnalytics {
        wallet_address: main_wallet_address_result.unwrap_or_default(),
        current_balance_sol: worth.sol_balance,
        token_count: worth.token_count,
        tokens_worth_sol: worth.tokens_worth_sol,
        total_equity_sol: worth.total_equity_sol,
        unpriced_token_count: worth.unpriced_token_count,
        start_of_day_balance_sol,
        change_sol,
        change_percent,
        sol_price_usd: crate::sol_price::get_sol_price(),
        balance_history,
    };

    // Process positions snapshot from parallel results
    let open_positions = open_positions_result.unwrap_or_default();
    let open_count = open_positions.len() as i64;
    let total_invested_sol: f64 = open_positions.iter().map(|p| p.entry_size_sol).sum();

    // Calculate position P&L with performers
    let mut best_performer: Option<PositionPerformer> = None;
    let mut worst_performer: Option<PositionPerformer> = None;
    let mut total_hold_duration_mins: i64 = 0;
    let mut dca_count: i64 = 0;

    let unrealized_pnl_sol: f64 = open_positions
        .iter()
        .filter_map(|p| {
            // Track DCA positions
            if p.dca_count > 0 {
                dca_count += 1;
            }

            // Calculate hold duration
            let hold_mins = (now - p.entry_time).num_minutes();
            total_hold_duration_mins += hold_mins;

            if let (Some(current), entry) = (p.current_price, p.entry_price) {
                let pnl_pct = if entry > 0.0 {
                    ((current - entry) / entry) * 100.0
                } else {
                    0.0
                };

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

                Some((current - entry) * p.entry_size_sol / entry)
            } else {
                None
            }
        })
        .sum();

    let unrealized_pnl_percent = if total_invested_sol > 0.0 {
        (unrealized_pnl_sol / total_invested_sol) * 100.0
    } else {
        0.0
    };

    let avg_position_size_sol = if open_count > 0 {
        total_invested_sol / open_count as f64
    } else {
        0.0
    };

    let avg_hold_duration_mins = if open_count > 0 {
        total_hold_duration_mins / open_count
    } else {
        0
    };

    let positions_snapshot = PositionsSnapshot {
        open_count,
        total_invested_sol,
        unrealized_pnl_sol,
        unrealized_pnl_percent,
        avg_position_size_sol,
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
    let total_in_database = crate::tokens::count_tokens_async()
        .await
        .unwrap_or_default();

    // Get filtering stats from the already fetched result (absent until the first
    // snapshot finishes building in the background)
    let filtering_stats = filtering_stats_result;

    let passed_filters = filtering_stats
        .as_ref()
        .map(|s| s.passed_filtering)
        .unwrap_or_default();
    let with_prices = filtering_stats
        .as_ref()
        .map(|s| s.with_pool_price)
        .unwrap_or_default();
    let blacklisted = filtering_stats
        .as_ref()
        .map(|s| s.blacklisted)
        .unwrap_or_default();
    let with_ohlcv = filtering_stats
        .as_ref()
        .map(|s| s.with_ohlcv)
        .unwrap_or_default();

    // Calculate rejected as total - passed - blacklisted
    let rejected_filters = if total_in_database > passed_filters + blacklisted {
        total_in_database - passed_filters - blacklisted
    } else {
        0
    };

    let tokens = TokenStatistics {
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

    // Get trader status (always off in preview mode — trading is disabled)
    let trader_status = TraderStatusInfo {
        running: !crate::global::is_preview_mode() && is_trader_running(),
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
