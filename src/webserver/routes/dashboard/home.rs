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
use crate::wallet::{worth_change, WalletWorth, WorthChange, WorthSample};
use crate::webserver::promo;
use crate::webserver::snapshot::get_cached_system_metrics;
use crate::webserver::state::AppState;

use super::status_cards::{auto_trader_card, read_copy_trading_card};
use super::types::*;
use super::utils::format_uptime;

/// Width of one sparkline sample: the newest snapshot of each 15-minute window, so a
/// full day is at most 96 points however often the monitor snapshots.
const TREND_BUCKET_SECS: i64 = 15 * 60;

/// The hero's wallet block from the live worth. Without a snapshot (no wallet, or the
/// first balance read has not landed) the worth is unknown, so its default zeroes never
/// reach the hero as a measured balance. The day change is measured like for like
/// (`wallet::worth_change`): without a baseline, or when the start of the day priced
/// other holdings than now, it is absent and `change_status` says why.
fn wallet_analytics(
    wallet_address: String,
    worth: &WalletWorth,
    start_of_day: Option<&WorthSample>,
    native_price_usd: f64,
    comparable_history: Vec<f64>,
) -> WalletAnalytics {
    let known = |value| worth.has_snapshot.then_some(value);
    let change = worth_change(worth, start_of_day);
    let (change_status, change_native, change_percent) = match change {
        WorthChange::Known { native, percent } => (WorthChangeStatus::Known, Some(native), percent),
        WorthChange::Unknown => (WorthChangeStatus::NoBaseline, None, None),
        WorthChange::Incomparable => (WorthChangeStatus::Incomparable, None, None),
    };
    let baseline = matches!(change, WorthChange::Known { .. })
        .then(|| start_of_day.map(|sample| sample.equity_native))
        .flatten();
    WalletAnalytics {
        wallet_address,
        current_balance_native: known(worth.native_balance),
        token_count: worth.has_snapshot.then_some(worth.token_count),
        tokens_worth_native: known(worth.tokens_worth_native),
        total_equity_native: known(worth.total_equity_native),
        unpriced_token_count: worth.has_snapshot.then(|| worth.unpriced_token_count()),
        start_of_day_balance_native: start_of_day.map(|sample| sample.equity_native),
        change_status,
        change_native,
        change_percent,
        native_price_usd,
        balance_history: day_trend(
            baseline,
            comparable_history,
            known(worth.total_equity_native),
        ),
    }
}

/// The day's worth trend: the start-of-day worth, the comparable samples since, and the
/// live worth last, so the line starts and ends on the two figures the day change
/// compares and its direction is the change's sign. Without a comparable baseline there
/// is no day change, and no line: a trend of the samples alone would read as the day's
/// movement while the change beside it is unknown.
fn day_trend(baseline: Option<f64>, samples: Vec<f64>, live: Option<f64>) -> Vec<f64> {
    let Some(baseline) = baseline else {
        return Vec::new();
    };
    std::iter::once(baseline)
        .chain(samples)
        .chain(live)
        .filter(|value| value.is_finite())
        .collect()
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
        start_of_day_result,
        // Positions data
        open_positions_result,
        // System metrics (cached, fast)
        cached_metrics,
        // Filtering stats
        filtering_stats_result,
        // Status cards
        position_capacity,
        copy_trading,
        auto_activity_result,
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
        // Start-of-day worth and the holdings it left unpriced
        crate::wallet::get_worth_at_time(today_start),
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
        crate::trader::safety::position_capacity(),
        read_copy_trading_card(),
        positions::get_auto_trade_activity(today_start),
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
    // the header makes, so the two surfaces can never disagree.
    let worth = crate::wallet::get_wallet_worth();
    let start_of_day = start_of_day_result.ok().flatten();

    // The day's trend plots the same quantity as the headline, and only samples that
    // priced the same holdings: a sample taken before a held token had a price would
    // draw a step that is no change in value. It depends on the live coverage, so it
    // is read after the worth.
    let comparable_history = if worth.has_snapshot {
        crate::wallet::get_comparable_worth_history(
            today_start,
            &worth.unpriced_mints,
            TREND_BUCKET_SECS,
        )
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let wallet = wallet_analytics(
        main_wallet_address_result.unwrap_or_default(),
        &worth,
        start_of_day.as_ref(),
        crate::native_price::get_native_price(),
        comparable_history,
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
    // A position with a cost basis but no price tick yet has an unknown value.
    let mut unpriced_with_basis = false;

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

            let Some(current) = p.current_price else {
                unpriced_with_basis = true;
                return None;
            };

            // Rank on the same fee-aware P&L the Open P&L figure sums, so a position
            // cannot read as a gain here and a loss beside it. The raw price move
            // stands in only until the first price tick.
            let pnl_pct = p.unrealized_pnl_percent.unwrap_or(if entry > 0.0 {
                ((current - entry) / entry) * 100.0
            } else {
                0.0
            });
            let performer = || PositionPerformer {
                id: p.id,
                mint: p.mint.clone(),
                symbol: p.symbol.clone(),
                pnl_percent: pnl_pct,
            };
            if best_performer
                .as_ref()
                .is_none_or(|best| pnl_pct > best.pnl_percent)
            {
                best_performer = Some(performer());
            }
            if worst_performer
                .as_ref()
                .is_none_or(|worst| pnl_pct < worst.pnl_percent)
            {
                worst_performer = Some(performer());
            }

            // Prefer the P&L the position already booked (fee-aware, computed by the
            // price updater); fall back to the basis-scaled estimate only until the
            // first price tick lands.
            Some(p.unrealized_pnl.unwrap_or_else(|| {
                if entry > 0.0 {
                    (current - entry) * p.total_size_native / entry
                } else {
                    0.0
                }
            }))
        })
        .sum();

    let unrealized_pnl_percent = (total_invested_native > 0.0)
        .then(|| (unrealized_pnl_native / total_invested_native) * 100.0);
    let current_value_native =
        (!unpriced_with_basis).then_some(total_invested_native + unrealized_pnl_native);
    let avg_position_size_native =
        (open_count > 0).then(|| total_invested_native / open_count as f64);
    let avg_hold_duration_mins = (open_count > 0).then(|| total_hold_duration_mins / open_count);

    let positions_snapshot = PositionsSnapshot {
        open_count,
        total_invested_native,
        current_value_native,
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

    // Process token statistics (filtering already fetched in parallel). Absent until the
    // first snapshot finishes building in the background, and absent stays absent all the
    // way to the hero: zeroing these would have the panel state that nothing passed and
    // nothing is priced, which is a different claim from "not counted yet".
    let filtering_stats = filtering_stats_result;

    let monitored = filtering_stats.as_ref().map(|s| s.total_tokens);
    let passed_filters = filtering_stats.as_ref().map(|s| s.passed_filtering);
    // Read live from the pool price cache, the source of every trading price. The
    // filtering snapshot's count is the cache intersected with its tokens at the moment
    // it was built, so it lagged the pool service by a whole snapshot cycle (0 right
    // after start-up while positions already showed a pool price).
    let with_prices = crate::pools::is_pool_service_running()
        .then(|| crate::pools::get_cache_stats(crate::chains::ChainScope::All).fresh_prices);
    let blacklisted = filtering_stats.as_ref().map(|s| s.blacklisted);
    let with_ohlcv = filtering_stats.as_ref().map(|s| s.with_ohlcv);

    // The funnel narrows in order: tracked (every token in the database) ⊇ monitored
    // (those with market data, which the filters evaluate) ⊇ passed. Rejected is
    // monitored minus passed, the header's count, all read from ONE filtering snapshot.
    // Only before the first snapshot is the tracked total counted live, through the
    // async wrapper: called directly, `count_tokens()` takes the token database's
    // connection lock on the async worker thread and parks it for the duration
    // (measured at 8.9s), starving unrelated dashboard panels.
    let total_in_database = match filtering_stats.as_ref() {
        Some(stats) => stats.total_tokens_in_database,
        None => crate::tokens::count_tokens_async(crate::chains::ChainScope::All)
            .await
            .unwrap_or_default(),
    };
    let rejected_filters = filtering_stats.as_ref().map(|s| s.rejected());

    let tokens = TokenStatistics {
        snapshot_state: SnapshotState::of(&filtering_stats),
        total_in_database,
        monitored,
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

    // The trader owner's run state, with Explore Mode and incomplete setup unavailable.
    let auto_trader = auto_trader_card(
        crate::trader::run_state(),
        crate::trader::live_block() == Some(crate::trader::LiveBlock::SetupIncomplete),
        position_capacity,
        auto_activity_result.ok(),
    );

    Json(HomeDashboardResponse {
        trader,
        wallet,
        positions: positions_snapshot,
        system,
        tokens,
        auto_trader,
        copy_trading,
        timestamp: now.to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worth(has_snapshot: bool, unpriced: &[&str]) -> WalletWorth {
        WalletWorth {
            native_balance: 1.5,
            tokens_worth_native: 0.5,
            total_equity_native: 2.0,
            token_count: 3,
            unpriced_mints: unpriced.iter().map(|mint| (*mint).to_owned()).collect(),
            has_snapshot,
            ..WalletWorth::default()
        }
    }

    fn start_of_day(equity_native: f64, unpriced: Option<&[&str]>) -> WorthSample {
        WorthSample {
            equity_native,
            unpriced_mints: unpriced.map(|mints| mints.iter().map(|m| (*m).to_owned()).collect()),
        }
    }

    #[test]
    fn wallet_without_snapshot_reports_no_worth() {
        let baseline = start_of_day(1.0, Some(&[]));
        let wallet = wallet_analytics(
            String::new(),
            &worth(false, &[]),
            Some(&baseline),
            150.0,
            vec![],
        );
        assert_eq!(wallet.current_balance_native, None);
        assert_eq!(wallet.tokens_worth_native, None);
        assert_eq!(wallet.total_equity_native, None);
        assert_eq!(wallet.token_count, None);
        assert_eq!(wallet.unpriced_token_count, None);
        assert_eq!(wallet.change_status, WorthChangeStatus::NoBaseline);
        assert_eq!(wallet.change_native, None);
        assert_eq!(wallet.change_percent, None);
        assert!(wallet.balance_history.is_empty());
    }

    #[test]
    fn wallet_with_snapshot_reports_worth_and_change() {
        let baseline = start_of_day(1.0, Some(&["dust"]));
        let wallet = wallet_analytics(
            "addr".to_owned(),
            &worth(true, &["dust"]),
            Some(&baseline),
            150.0,
            vec![1.2, 1.8],
        );
        assert_eq!(wallet.current_balance_native, Some(1.5));
        assert_eq!(wallet.total_equity_native, Some(2.0));
        assert_eq!(wallet.token_count, Some(3));
        assert_eq!(wallet.unpriced_token_count, Some(1));
        assert_eq!(wallet.change_status, WorthChangeStatus::Known);
        assert_eq!(wallet.change_native, Some(1.0));
        assert_eq!(wallet.change_percent, Some(100.0));
        // The trend starts at the baseline and ends at the live worth.
        assert_eq!(wallet.balance_history, vec![1.0, 1.2, 1.8, 2.0]);
    }

    /// A token priced now but not at the start of the day is not a gain: the change is
    /// withheld, and no trend is drawn beside the unknown change.
    #[test]
    fn a_token_entering_pricing_is_not_a_day_change() {
        let baseline = start_of_day(1.0, Some(&["late"]));
        let wallet = wallet_analytics(
            "addr".to_owned(),
            &worth(true, &[]),
            Some(&baseline),
            150.0,
            vec![1.9],
        );
        assert_eq!(wallet.total_equity_native, Some(2.0));
        assert_eq!(wallet.change_status, WorthChangeStatus::Incomparable);
        assert_eq!(wallet.change_native, None);
        assert_eq!(wallet.change_percent, None);
        assert!(wallet.balance_history.is_empty());

        let legacy = start_of_day(1.0, None);
        let wallet = wallet_analytics(
            "addr".to_owned(),
            &worth(true, &[]),
            Some(&legacy),
            150.0,
            vec![],
        );
        assert_eq!(wallet.change_status, WorthChangeStatus::Incomparable);
    }

    /// The live price count reads the pool price cache, never the filtering snapshot's
    /// copy of it, which is only as fresh as its last build.
    #[test]
    fn live_price_count_reads_the_pool_cache() {
        let source = include_str!("home.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(production.contains("get_cache_stats"));
        assert!(!production.contains("with_pool_price"));
    }

    #[test]
    fn wallet_without_baseline_reports_no_change() {
        let wallet = wallet_analytics("addr".to_owned(), &worth(true, &[]), None, 150.0, vec![]);
        assert_eq!(wallet.total_equity_native, Some(2.0));
        assert_eq!(wallet.change_status, WorthChangeStatus::NoBaseline);
        assert_eq!(wallet.change_native, None);
        assert_eq!(wallet.change_percent, None);
    }
}
