//! Demo mode for dashboard screenshots and marketing materials
//!
//! This module provides hardcoded realistic data for showcasing the bot
//! in screenshots, videos, and social media posts.
//!
//! Enable with: cargo run --bin screenerbot -- --gui --dashboard-demo
//!
//! Affected endpoints:
//! - /api/dashboard/home (wallet balance, P&L, positions)
//! - /api/dashboard/overview
//! - /api/positions (open/closed positions)
//! - /api/wallet/current (SOL balance, tokens)
//! - /api/trader/status & /api/trader/stats

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{Duration, Utc};

use crate::webserver::routes::dashboard::{
    BlacklistInfo, DashboardOverview, HomeDashboardResponse, MonitoringInfo, OpenPositionDetail,
    PositionPerformer, PositionsSnapshot, PositionsSummary, RpcInfo, ServiceStatus, SystemInfo,
    SystemMetrics, TokenStatistics, TraderAnalytics, TraderStatusInfo, TradingPeriodStats,
    WalletAnalytics, WalletInfo,
};
use crate::webserver::routes::header::{
    FilteringHeaderInfo, HeaderMetricsResponse, PositionsHeaderInfo, RpcHeaderInfo,
    SystemHeaderInfo, TraderHeaderInfo, WalletHeaderInfo,
};
use crate::webserver::routes::positions::types::{PositionResponse, PositionsStatsResponse};
use crate::webserver::routes::trader::types::{ExitBreakdown, TraderStatsResponse};
use crate::webserver::routes::wallet::{
    TokenBalanceInfo, WalletCurrentResponse, WalletTokenHolding, WalletTokensResponse,
};

/// Global flag for demo mode - set at startup based on --dashboard-demo argument
pub static DEMO_MODE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if demo mode is active
pub fn is_demo_mode() -> bool {
    DEMO_MODE_ENABLED.load(Ordering::Relaxed)
}

/// Enable demo mode (called at startup if --dashboard-demo flag is present)
pub fn enable_demo_mode() {
    DEMO_MODE_ENABLED.store(true, Ordering::SeqCst);
}

// Import demo data constants from sibling module
use super::demo_data::*;

// =============================================================================
// DEMO DATA GENERATORS
// =============================================================================

/// Generate demo home dashboard response
pub fn get_demo_home_dashboard() -> HomeDashboardResponse {
    let now = Utc::now();

    // Trading analytics with realistic profitable trading history
    let trader = TraderAnalytics {
        today: TradingPeriodStats {
            buys: 5,
            sells: 4,
            profit_sol: 0.347,
            loss_sol: 0.082,
            net_pnl_sol: 0.265,
            drawdown_percent: 2.1,
            win_rate: 75.0,
        },
        yesterday: TradingPeriodStats {
            buys: 8,
            sells: 7,
            profit_sol: 0.523,
            loss_sol: 0.145,
            net_pnl_sol: 0.378,
            drawdown_percent: 3.2,
            win_rate: 71.4,
        },
        this_week: TradingPeriodStats {
            buys: 32,
            sells: 29,
            profit_sol: 1.847,
            loss_sol: 0.423,
            net_pnl_sol: 1.424,
            drawdown_percent: 4.8,
            win_rate: 69.0,
        },
        this_month: TradingPeriodStats {
            buys: 89,
            sells: 82,
            profit_sol: 4.234,
            loss_sol: 1.387,
            net_pnl_sol: 2.847,
            drawdown_percent: 8.5,
            win_rate: DEMO_WIN_RATE,
        },
        all_time: TradingPeriodStats {
            buys: 156,
            sells: 143,
            profit_sol: 7.892,
            loss_sol: 2.145,
            net_pnl_sol: 5.747,
            drawdown_percent: 12.3,
            win_rate: 67.8,
        },
    };

    let wallet = WalletAnalytics {
        current_balance_sol: DEMO_SOL_BALANCE,
        token_count: 4,
        tokens_worth_sol: 0.45,
        start_of_day_balance_sol: DEMO_START_BALANCE,
        change_sol: DEMO_SOL_BALANCE - DEMO_START_BALANCE,
        change_percent: ((DEMO_SOL_BALANCE - DEMO_START_BALANCE) / DEMO_START_BALANCE) * 100.0,
    };

    let positions = PositionsSnapshot {
        open_count: DEMO_OPEN_POSITIONS as i64,
        total_invested_sol: DEMO_INVESTED_SOL,
        unrealized_pnl_sol: DEMO_UNREALIZED_PNL,
        unrealized_pnl_percent: (DEMO_UNREALIZED_PNL / DEMO_INVESTED_SOL) * 100.0,
        avg_position_size_sol: DEMO_INVESTED_SOL / DEMO_OPEN_POSITIONS as f64,
        avg_hold_duration_mins: 47,
        best_performer: Some(PositionPerformer {
            symbol: "BONK".to_owned(),
            pnl_percent: 23.5,
        }),
        worst_performer: Some(PositionPerformer {
            symbol: "MEME".to_owned(),
            pnl_percent: -8.2,
        }),
        dca_count: 1,
    };

    let uptime_secs = 3 * 24 * 3600 + 7 * 3600 + 23 * 60 + 45; // 3d 7h 23m 45s
    let system = SystemMetrics {
        uptime_seconds: uptime_secs,
        uptime_formatted: "3d 7h 23m 45s".to_owned(),
        memory_mb: DEMO_MEMORY_MB,
        memory_percent: 2.4,
        cpu_percent: DEMO_CPU_PERCENT,
        rpc_calls_per_min: 847.3,
        rpc_success_rate: 99.7,
        websocket_connected: true,
        services_healthy: 12,
        services_total: 12,
    };

    let tokens = TokenStatistics {
        total_in_database: 12847,
        with_prices: 8923,
        passed_filters: 347,
        rejected_filters: 8576,
        blacklisted: 1924,
        with_ohlcv: 2847,
        found_today: 234,
        found_this_week: 1523,
        found_this_month: 4892,
        found_all_time: 12847,
    };

    HomeDashboardResponse {
        trader,
        wallet,
        positions,
        system,
        tokens,
        trader_status: TraderStatusInfo { running: true }, // Demo mode: always running
        timestamp: now.to_rfc3339(),
    }
}

/// Generate demo dashboard overview response
pub fn get_demo_dashboard_overview() -> DashboardOverview {
    let now = Utc::now();

    let wallet = WalletInfo {
        sol_balance: DEMO_SOL_BALANCE,
        sol_balance_lamports: DEMO_SOL_LAMPORTS,
        total_tokens_count: 4,
        last_updated: Some(now.to_rfc3339()),
    };

    let open_position_details: Vec<OpenPositionDetail> = DEMO_OPEN_TOKENS
        .iter()
        .map(
            |(symbol, _name, mint, _logo, entry, current, _size, hold_min)| {
                let pnl_pct = ((current - entry) / entry) * 100.0;
                OpenPositionDetail {
                    mint: mint.to_string(),
                    symbol: symbol.to_string(),
                    entry_price: *entry,
                    current_price: Some(*current),
                    pnl_percent: Some(pnl_pct),
                    hold_duration_minutes: *hold_min,
                }
            },
        )
        .collect();

    let positions = PositionsSummary {
        total_positions: (DEMO_OPEN_POSITIONS + DEMO_CLOSED_TOKENS.len()) as i64,
        open_positions: DEMO_OPEN_POSITIONS as i64,
        closed_positions: DEMO_CLOSED_TOKENS.len() as i64,
        total_invested_sol: DEMO_INVESTED_SOL,
        total_pnl: DEMO_TOTAL_PNL,
        win_rate: DEMO_WIN_RATE,
        open_position_details,
    };

    let uptime_secs = 3 * 24 * 3600 + 7 * 3600 + 23 * 60 + 45;
    let system = SystemInfo {
        all_services_ready: true,
        services: ServiceStatus {
            tokens_system: true,
            positions_system: true,
            pool_service: true,
            transactions_system: true,
        },
        uptime_seconds: uptime_secs,
        uptime_formatted: "3d 7h 23m 45s".to_owned(),
        memory_mb: DEMO_MEMORY_MB,
        cpu_percent: DEMO_CPU_PERCENT,
        active_threads: 24,
    };

    let rpc = RpcInfo {
        total_calls: 847_234,
        calls_per_second: 4.7,
        uptime_seconds: uptime_secs,
    };

    let mut by_reason = HashMap::new();
    by_reason.insert("Manual".to_owned(), 47);
    by_reason.insert("MintAuthority".to_owned(), 523);
    by_reason.insert("FreezeAuthority".to_owned(), 412);
    by_reason.insert("NonAuthority::RugPull".to_owned(), 271);

    let blacklist = BlacklistInfo {
        total_blacklisted: DEMO_BLACKLISTED,
        by_reason,
    };

    let monitoring = MonitoringInfo {
        tokens_tracked: DEMO_TOKENS_TRACKED,
        entry_check_interval_secs: 10,
        position_monitor_interval_secs: 5,
    };

    DashboardOverview {
        wallet,
        positions,
        system,
        rpc,
        blacklist,
        monitoring,
        timestamp: now.to_rfc3339(),
    }
}

/// Generate demo positions list
pub fn get_demo_positions(status: Option<&str>) -> Vec<PositionResponse> {
    let now = Utc::now();
    let mut positions = Vec::new();
    let mut id_counter: i64 = 1;

    let include_open = status.is_none() || status == Some("open") || status == Some("all");
    let include_closed = status.is_none() || status == Some("closed") || status == Some("all");

    // Add open positions
    if include_open {
        for (symbol, name, mint, logo, entry, current, size, hold_min) in DEMO_OPEN_TOKENS.iter() {
            let entry_time = now - Duration::minutes(*hold_min);
            let pnl_pct = ((current - entry) / entry) * 100.0;
            let unrealized_pnl = (current - entry) * size / entry;

            positions.push(PositionResponse {
                id: Some(id_counter),
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                name: name.to_string(),
                logo_url: Some(logo.to_string()),
                entry_price: *entry,
                entry_time: entry_time.timestamp(),
                exit_price: None,
                exit_time: None,
                position_type: "long".to_owned(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: current * 1.05,
                price_lowest: entry * 0.95,
                entry_transaction_signature: Some(format!("demo_entry_sig_{id_counter}")),
                exit_transaction_signature: None,
                token_amount: Some((size / entry * 1e9) as u64),
                effective_entry_price: Some(*entry),
                effective_exit_price: None,
                sol_received: None,
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_owned()),
                transaction_entry_verified: true,
                transaction_exit_verified: false,
                entry_fee_lamports: Some(5000),
                exit_fee_lamports: None,
                current_price: Some(*current),
                current_price_updated: Some(now.timestamp()),
                phantom_confirmations: 0,
                synthetic_exit: false,
                closed_reason: None,
                pnl: None,
                pnl_percent: None,
                unrealized_pnl: Some(unrealized_pnl),
                unrealized_pnl_percent: Some(pnl_pct),
                dca_count: 0,
                average_entry_price: *entry,
                partial_exit_count: 0,
                average_exit_price: None,
                remaining_token_amount: Some((size / entry * 1e9) as u64),
                total_exited_amount: 0,
                archived: false,
                archived_at: None,
            });
            id_counter += 1;
        }
    }

    // Add closed positions
    if include_closed {
        for (i, (symbol, name, mint, logo, entry, exit, size, reason)) in
            DEMO_CLOSED_TOKENS.iter().enumerate()
        {
            let exit_time = now - Duration::hours((i as i64 + 1) * 6);
            let entry_time = exit_time - Duration::hours(2);
            let pnl = (exit - entry) * size / entry;
            let pnl_pct = ((exit - entry) / entry) * 100.0;

            positions.push(PositionResponse {
                id: Some(id_counter),
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                name: name.to_string(),
                logo_url: Some(logo.to_string()),
                entry_price: *entry,
                entry_time: entry_time.timestamp(),
                exit_price: Some(*exit),
                exit_time: Some(exit_time.timestamp()),
                position_type: "long".to_owned(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: exit * 1.02,
                price_lowest: entry * 0.97,
                entry_transaction_signature: Some(format!("demo_entry_sig_{id_counter}")),
                exit_transaction_signature: Some(format!("demo_exit_sig_{id_counter}")),
                token_amount: Some((size / entry * 1e9) as u64),
                effective_entry_price: Some(*entry),
                effective_exit_price: Some(*exit),
                sol_received: Some(size + pnl),
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_owned()),
                transaction_entry_verified: true,
                transaction_exit_verified: true,
                entry_fee_lamports: Some(5000),
                exit_fee_lamports: Some(5000),
                current_price: None,
                current_price_updated: None,
                phantom_confirmations: 0,
                synthetic_exit: false,
                closed_reason: Some(reason.to_string()),
                pnl: Some(pnl),
                pnl_percent: Some(pnl_pct),
                unrealized_pnl: None,
                unrealized_pnl_percent: None,
                dca_count: 0,
                average_entry_price: *entry,
                partial_exit_count: 0,
                average_exit_price: Some(*exit),
                remaining_token_amount: None,
                total_exited_amount: (size / entry * 1e9) as u64,
                archived: false,
                archived_at: None,
            });
            id_counter += 1;
        }
    }

    positions
}

/// Generate demo positions stats
pub fn get_demo_positions_stats() -> PositionsStatsResponse {
    PositionsStatsResponse {
        total: DEMO_OPEN_POSITIONS + DEMO_CLOSED_TOKENS.len(),
        open: DEMO_OPEN_POSITIONS,
        closed: DEMO_CLOSED_TOKENS.len(),
        total_invested_sol: DEMO_INVESTED_SOL,
        total_pnl: DEMO_TOTAL_PNL,
    }
}

/// Generate demo wallet current response
pub fn get_demo_wallet_current() -> WalletCurrentResponse {
    let now = Utc::now();

    // Include demo token balances with real token data
    let token_balances = vec![
        TokenBalanceInfo {
            mint: "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN".to_owned(), // TRUMP
            balance: 125_340_000,
            balance_ui: 125.34,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5".to_owned(), // MEW
            balance: 2_345_000_000,
            balance_ui: 2345.0,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_owned(), // Fartcoin
            balance: 847_000_000,
            balance_ui: 847.0,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82".to_owned(), // BOME
            balance: 1_234_000_000,
            balance_ui: 1234.0,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_owned(), // USDC
            balance: 125_340_000,
            balance_ui: 125.34,
            decimals: 6,
            is_token_2022: false,
        },
    ];

    WalletCurrentResponse {
        sol_balance: DEMO_SOL_BALANCE,
        sol_balance_lamports: DEMO_SOL_LAMPORTS,
        total_tokens_count: token_balances.len() as u32,
        token_balances,
        snapshot_time: now.to_rfc3339(),
    }
}

/// Generate demo trader stats response  
pub fn get_demo_trader_stats() -> TraderStatsResponse {
    TraderStatsResponse {
        open_positions_count: DEMO_OPEN_POSITIONS,
        locked_sol: DEMO_INVESTED_SOL,
        win_rate_pct: DEMO_WIN_RATE,
        total_trades: DEMO_TOTAL_TRADES,
        avg_hold_time_hours: 2.4,
        best_trade_pct: 92.5,
        exit_breakdown: vec![
            ExitBreakdown {
                exit_type: "trailing_stop".to_owned(),
                count: 42,
                avg_profit_pct: 31.8,
            },
            ExitBreakdown {
                exit_type: "take_profit".to_owned(),
                count: 38,
                avg_profit_pct: 45.2,
            },
            ExitBreakdown {
                exit_type: "stop_loss".to_owned(),
                count: 18,
                avg_profit_pct: -18.5,
            },
            ExitBreakdown {
                exit_type: "manual".to_owned(),
                count: 10,
                avg_profit_pct: 34.7,
            },
        ],
    }
}

/// Generate demo header metrics response
pub fn get_demo_header_metrics() -> HeaderMetricsResponse {
    let now = Utc::now();

    let trader = TraderHeaderInfo {
        running: true,
        enabled: true,
        today_pnl_sol: 0.265,
        today_pnl_percent: 3.12,
        uptime_seconds: 3 * 24 * 3600 + 7 * 3600 + 23 * 60 + 45, // 3d 7h 23m 45s
    };

    let wallet = WalletHeaderInfo {
        sol_balance: DEMO_SOL_BALANCE,
        change_24h_sol: 0.847,
        change_24h_percent: 9.4,
        token_count: 4,
        tokens_worth_sol: 0.45,
        last_updated: now.to_rfc3339(),
    };

    let positions = PositionsHeaderInfo {
        open_count: DEMO_OPEN_POSITIONS as i64,
        unrealized_pnl_sol: DEMO_UNREALIZED_PNL,
        unrealized_pnl_percent: (DEMO_UNREALIZED_PNL / DEMO_INVESTED_SOL) * 100.0,
        total_invested_sol: DEMO_INVESTED_SOL,
    };

    let rpc = RpcHeaderInfo {
        success_rate_percent: 99.7,
        avg_latency_ms: 142,
        calls_per_minute: 284.5,
        healthy: true,
    };

    let filtering = FilteringHeaderInfo {
        monitoring_count: DEMO_TOKENS_TRACKED,
        passed_count: 347,
        rejected_count: 2500,
        last_refresh: now.to_rfc3339(),
    };

    let system = SystemHeaderInfo {
        all_services_healthy: true,
        unhealthy_services: vec![],
        critical_degraded: false,
    };

    HeaderMetricsResponse {
        trader,
        wallet,
        positions,
        rpc,
        filtering,
        system,
        timestamp: now.to_rfc3339(),
    }
}

/// Generate demo wallet tokens response
pub fn get_demo_wallet_tokens() -> WalletTokensResponse {
    let tokens = vec![
        WalletTokenHolding {
            mint: "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN".to_owned(),
            symbol: Some("TRUMP".to_owned()),
            name: Some("Official Trump".to_owned()),
            balance: 125_340_000,
            ui_amount: 125.34,
            decimals: 6,
            is_token_2022: false,
        },
        WalletTokenHolding {
            mint: "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5".to_owned(),
            symbol: Some("MEW".to_owned()),
            name: Some("cat in a dogs world".to_owned()),
            balance: 2_345_000_000,
            ui_amount: 2345.0,
            decimals: 6,
            is_token_2022: false,
        },
        WalletTokenHolding {
            mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_owned(),
            symbol: Some("Fartcoin".to_owned()),
            name: Some("Fartcoin".to_owned()),
            balance: 847_000_000,
            ui_amount: 847.0,
            decimals: 6,
            is_token_2022: false,
        },
        WalletTokenHolding {
            mint: "ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82".to_owned(),
            symbol: Some("BOME".to_owned()),
            name: Some("BOOK OF MEME".to_owned()),
            balance: 1_234_000_000,
            ui_amount: 1234.0,
            decimals: 6,
            is_token_2022: false,
        },
    ];

    WalletTokensResponse { tokens }
}
