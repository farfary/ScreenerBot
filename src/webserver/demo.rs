/// Demo mode for dashboard screenshots and marketing materials
///
/// This module provides hardcoded realistic data for showcasing the bot
/// in screenshots, videos, and social media posts.
///
/// Enable with: cargo run --bin screenerbot -- --gui --dashboard-demo
///
/// Affected endpoints:
/// - /api/dashboard/home (wallet balance, P&L, positions)
/// - /api/dashboard/overview
/// - /api/positions (open/closed positions)
/// - /api/wallet/current (SOL balance, tokens)
/// - /api/trader/status & /api/trader/stats
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{Duration, Utc};

use crate::webserver::routes::dashboard::{
    BlacklistInfo, DashboardOverview, HomeDashboardResponse, MonitoringInfo, OpenPositionDetail,
    PositionsSummary, PositionsSnapshot, RpcInfo, ServiceStatus, SystemInfo, SystemMetrics,
    TokenStatistics, TraderAnalytics, TradingPeriodStats, WalletAnalytics, WalletInfo,
};
use crate::webserver::routes::header::{
    FilteringHeaderInfo, HeaderMetricsResponse, PositionsHeaderInfo, RpcHeaderInfo,
    SystemHeaderInfo, TraderHeaderInfo, WalletHeaderInfo,
};
use crate::webserver::routes::positions::{PositionResponse, PositionsStatsResponse};
use crate::webserver::routes::trader::{ExitBreakdown, TraderStatsResponse};
use crate::webserver::routes::wallet::{TokenBalanceInfo, WalletCurrentResponse};

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

// =============================================================================
// DEMO CONSTANTS - Realistic showcase values
// =============================================================================

const DEMO_SOL_BALANCE: f64 = 9.847;
const DEMO_SOL_LAMPORTS: u64 = 9_847_000_000;
const DEMO_START_BALANCE: f64 = 8.5;
const DEMO_TOTAL_PNL: f64 = 4.127;
const DEMO_WIN_RATE: f64 = 71.2;
const DEMO_TOTAL_TRADES: usize = 118;
const DEMO_OPEN_POSITIONS: usize = 10;
const DEMO_INVESTED_SOL: f64 = 1.65;
const DEMO_UNREALIZED_PNL: f64 = 0.524;
const DEMO_MEMORY_MB: f64 = 384.5;
const DEMO_CPU_PERCENT: f64 = 12.3;
const DEMO_TOKENS_TRACKED: usize = 2847;
const DEMO_BLACKLISTED: usize = 1253;

/// Demo open positions - Top liquidity real tokens with logos
/// (symbol, name, mint, logo_url, entry_price_sol, current_price_sol, size_sol, hold_minutes)
const DEMO_OPEN_TOKENS: &[(&str, &str, &str, &str, f64, f64, f64, i64)] = &[
    (
        "TRUMP",
        "OFFICIAL TRUMP",
        "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN",
        "https://cdn.dexscreener.com/cms/images/85a2613c51c8ded8e51b1b3910487ab66691cb60fecec7d0905481a603bba899?width=64&height=64&quality=90",
        0.042,
        0.0489,
        0.25,
        145,
    ),
    (
        "MEW",
        "cat in a dogs world",
        "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5",
        "https://cdn.dexscreener.com/cms/images/33effe52dd5b1f6574ca5baaca9c02fecdecb557607a2a72889ceb0537eae9be?width=64&height=64&quality=90",
        0.0000085,
        0.00000992,
        0.18,
        87,
    ),
    (
        "Fartcoin",
        "Fartcoin",
        "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump",
        "https://cdn.dexscreener.com/cms/images/9af5672845c89585e9ff1e3b26a640090324aa4d92222052d1043e60ef8182de?width=64&height=64&quality=90",
        0.00158,
        0.001856,
        0.15,
        62,
    ),
    (
        "BOME",
        "BOOK OF MEME",
        "ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82",
        "https://cdn.dexscreener.com/cms/images/1fdb1c93b76e5aed7324c2c541558fd75fe7ffb3d0d0fb9ee8370cbac5890e4e?width=64&height=64&quality=90",
        0.0000048,
        0.000005762,
        0.12,
        38,
    ),
    (
        "MOODENG",
        "Moo Deng",
        "ED5nyyWEzpPPiWimP8vYm7sD7TD3LAt3Q3gRTWHzPJBY",
        "https://cdn.dexscreener.com/cms/images/8e02e1e7dec93a4ef4b2404976232940a1d80595e774cf3c5f2d6f23026867a9?width=64&height=64&quality=90",
        0.000498,
        0.000583,
        0.15,
        23,
    ),
    (
        "YZY",
        "YZY",
        "DrZ26cKJDksVRWib3DVVsjo9eeXccc7hKhDJviiYEEZY",
        "https://cdn.dexscreener.com/cms/images/b2292ab9842bdca5d57f4b6870273904432eebf7314a6b7532ab06d424b07d6d?width=64&height=64&quality=90",
        0.00195,
        0.00228,
        0.18,
        52,
    ),
    (
        "FWOG",
        "FWOG",
        "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump",
        "https://cdn.dexscreener.com/cms/images/ceccc0a74f91ec3a5f6162005af09d1f6e8bdbdd6a55f49e43ac8b217b6ec8bd?width=64&height=64&quality=90",
        0.0000923,
        0.0001096,
        0.12,
        41,
    ),
    (
        "GOAT",
        "Goatseus Maximus",
        "CzLSujWBLFsSjncfkh59rUFqvafWcY5tzedWJSuypump",
        "https://cdn.dexscreener.com/cms/images/e857505d98436d21d13451a83c93ff4db36d0b53829af8070ddae75845d9b459?width=64&height=64&quality=90",
        0.000258,
        0.000304,
        0.20,
        78,
    ),
    (
        "ZEREBRO",
        "zerebro",
        "8x5VqbHA8D7NkD52uNuS5nnt3PwA8pLD34ymskeSo2Wn",
        "https://cdn.dexscreener.com/cms/images/2eb3b0a304e9ed93ee44ff263a7cd1c5b376589644b70618aef104a037f391c2?width=64&height=64&quality=90",
        0.000175,
        0.000204,
        0.15,
        33,
    ),
    (
        "GIGA",
        "GIGACHAD",
        "63LfDmNb3MQ8mw9MtZ2To9bEA2M71kZUUGq5tiJxcqj9",
        "https://cdn.dexscreener.com/cms/images/102ba1dee6cf6239b293dbd86e0e11ddf78e74cdce00030c4511096bd2480e26?width=64&height=64&quality=90",
        0.0000285,
        0.000033,
        0.15,
        19,
    ),
];

/// Demo closed positions - Real tokens with profitable/loss trades
/// (symbol, name, mint, logo_url, entry_price_sol, exit_price_sol, size_sol, exit_reason)
const DEMO_CLOSED_TOKENS: &[(&str, &str, &str, &str, f64, f64, f64, &str)] = &[
    // Profitable trades - trailing stop
    (
        "Pnut",
        "Peanut the Squirrel",
        "2qEHjDLDLbuBgRYvsxhc5D6uDWAivNFZGan56P1tpump",
        "https://cdn.dexscreener.com/cms/images/778498984ea5b6eb7c9d74e1e81a2547c88a41ae80f6c840c94a7c3b7829bcd5?width=64&height=64&quality=90",
        0.000612,
        0.000765,
        0.18,
        "trailing_stop",
    ),
    (
        "PONKE",
        "PONKE",
        "5z3EqYQo9HiCEs3R84RCDMu2n7anpDMxRhdK8PSWmrRC",
        "https://cdn.dexscreener.com/cms/images/cfbe2eabb540e7ba8651832435e968e12c9df2efb452e78358e64d8f73ae5760?width=64&height=64&quality=90",
        0.000265,
        0.000347,
        0.15,
        "trailing_stop",
    ),
    (
        "ALCH",
        "Alchemist AI",
        "HNg5PYJmtqcmzXrv6S9zP1CDKk5BgDuyFBxbvNApump",
        "https://cdn.dexscreener.com/cms/images/2eb3cb7607a5331385421301279d0fbc35f6e1dac31a773c3261cba73306c390?width=64&height=64&quality=90",
        0.000645,
        0.000812,
        0.15,
        "trailing_stop",
    ),
    (
        "Ban",
        "Comedian",
        "9PR7nCP9DpcUotnDPVLUBUZKu5WAYkwrCUx9wDnSpump",
        "https://cdn.dexscreener.com/cms/images/ba76da857adf1e27735335b43698d95324888e3fd30132ebac0e42de59a6f140?width=64&height=64&quality=90",
        0.000287,
        0.000389,
        0.12,
        "trailing_stop",
    ),
    (
        "MANEKI",
        "MANEKI",
        "25hAyBQfoDhfWx9ay6rarbgvWGwDdNqcHsXS3jQ3mTDJ",
        "https://cdn.dexscreener.com/cms/images/2d1e97e69f64c1e77db437f9a93a756f645d100ac2c8d2ae7efa244ab5b75351?width=64&height=64&quality=90",
        0.00000312,
        0.00000423,
        0.2,
        "trailing_stop",
    ),
    (
        "arc",
        "AI Rig Complex",
        "61V8vBaqAGMpgDQi4JcAwo1dmBGHsyhzodcPqnEVpump",
        "https://cdn.dexscreener.com/cms/images/952429eb5f770cb20d90492e438aa92bf61d33fa6aaaeca0186d2862d68fc21c?width=64&height=64&quality=90",
        0.000142,
        0.000189,
        0.18,
        "trailing_stop",
    ),
    // Profitable trades - take profit
    (
        "TROLL",
        "TROLL",
        "5UUH9RTDiSpq6HKS6bp4NdU9PNJpXRXuiw6ShBTBhgH2",
        "https://cdn.dexscreener.com/cms/images/97b02493a3a6aa5c7433cfa8ccd4732e6d73b9ebe70cfe43f0c258c4de83593c?width=800&height=800&quality=90",
        0.000215,
        0.000312,
        0.15,
        "take_profit",
    ),
    (
        "CLOUD",
        "Cloud",
        "CLoUDKc4Ane7HeQcPpE3YHnznRxhMimJ4MyaUqyHFzAu",
        "https://cdn.dexscreener.com/cms/images/9061ac79a133ce4fd2a79c836e91b0ac09a0af5f0685bbde342bae54366b6f95?width=64&height=64&quality=90",
        0.000478,
        0.000654,
        0.20,
        "take_profit",
    ),
    (
        "pippin",
        "Pippin",
        "Dfh5DzRgSvvCFDoYc2ciTkMrbDfRKybA4SoFbPmApump",
        "https://cdn.dexscreener.com/cms/images/d237de55618e54fd7d66593ff2adf3ad8c092398f9049a31f1dcb1b23ad1dff8?width=64&height=64&quality=90",
        0.000168,
        0.000234,
        0.18,
        "take_profit",
    ),
    (
        "DBR",
        "deBridge",
        "DBRiDgJAMsM95moTzJs7M9LnkGErpbv9v6CUR1DXnUu5",
        "https://cdn.dexscreener.com/cms/images/f38cfcc8eb87637bc63840861ec2dfc9eb4b057aa77188fe935126b97b5dd6c8?width=64&height=64&quality=90",
        0.000125,
        0.000168,
        0.15,
        "take_profit",
    ),
    (
        "jellyjelly",
        "jelly-my-jelly",
        "FeR8VBqNRSUD5NtXAj2n3j1dAHkZHfyDktKuLXD4pump",
        "https://cdn.dexscreener.com/cms/images/4a290c337b983cfb1ab8e1caaef969051bd584ba52784db0cf1f74fe5307ae22?width=64&height=64&quality=90",
        0.000312,
        0.000456,
        0.12,
        "take_profit",
    ),
    (
        "PAIN",
        "PAIN",
        "1Qf8gESP4i6CFNWerUSDdLKJ9U1LpqTYvjJ2MM4pain",
        "https://cdn.dexscreener.com/cms/images/c2b438108725fd4d7f11523f122a1f3e1c8d698a22cdf7bb85938a415d59b263?width=64&height=64&quality=90",
        0.00478,
        0.00645,
        0.18,
        "take_profit",
    ),
    (
        "UMBRA",
        "Umbra",
        "PRVT6TB7uss3FrUd2D9xs2zqDBsa3GbMJMwCQsgmeta",
        "https://cdn.dexscreener.com/cms/images/ee2528aec5c886bb1e1884b73a701a13214170c57a6637934ea3cd88ed3f1273?width=64&height=64&quality=90",
        0.00512,
        0.00678,
        0.15,
        "take_profit",
    ),
    // Profitable trades - manual
    (
        "KMNO",
        "Kamino",
        "KMNo3nJsBXfcpJTVhZcXLW7RmTwTt4GVFE7suUBo9sS",
        "https://cdn.dexscreener.com/cms/images/b3b9a0026bec75db0e4ecb6e023901a812dad85d3ffa1d2ec8b3a53ca498da31?width=64&height=64&quality=90",
        0.000315,
        0.000425,
        0.20,
        "manual",
    ),
    (
        "META",
        "MetaDAO",
        "METAwkXcqyXKy1AtsSgJ8JiUHwGCafnZL38n3vYmeta",
        "https://cdn.dexscreener.com/cms/images/0a6627148da5491b5ea44a2e247a454812167702b609182b285ee346c20c7cc2?width=64&height=64&quality=90",
        0.0425,
        0.0578,
        0.15,
        "manual",
    ),
    (
        "USELESS",
        "USELESS COIN",
        "Dz9mQ9NzkBcCsuGPFJ3r1bS4wgqKMHBPiVuniW8Mbonk",
        "https://cdn.dexscreener.com/cms/images/4f8af59f26d45252fb4379d4b1a1e61d0b419fd34dab2ec9f3ba77585d1783cb?width=64&height=64&quality=90",
        0.000923,
        0.001245,
        0.12,
        "manual",
    ),
    (
        "aura",
        "aura",
        "DtR4D9FtVoTX2569gaL837ZgrB6wNjj6tkmnX9Rdk9B2",
        "https://cdn.dexscreener.com/cms/images/8f6c41e8155c0e0bac57d58f8415ab98bcc96380d4616758eb6ed468b623668d?width=64&height=64&quality=90",
        0.000278,
        0.000378,
        0.18,
        "manual",
    ),
    // Loss trades - stop loss
    (
        "ACT",
        "Act I : The AI Prophecy",
        "GJAFwWjJ3vnTsrQVabjBVK2TYB1YtRCQXRDfDgUnpump",
        "https://cdn.dexscreener.com/cms/images/14201087ebafd2b8ca5c3242ddfd4e6cb0824b539ad0736ebea5ec03edefd214?width=64&height=64&quality=90",
        0.000165,
        0.000132,
        0.18,
        "stop_loss",
    ),
    (
        "VINE",
        "Vine Coin",
        "6AJcP7wuLwmRYLBNbi825wgguaPsWzPBEHcHndpRpump",
        "https://cdn.dexscreener.com/cms/images/74a2255eb11fd430603a2ae1823456c98a5e241fa2e526c253c92ad16d1fa1ce?width=64&height=64&quality=90",
        0.000345,
        0.000218,
        0.15,
        "stop_loss",
    ),
    (
        "PYTHIA",
        "PYTHIA",
        "CreiuhfwdWCN5mJbMJtA9bBpYQrQF2tCBuZwSPWfpump",
        "https://cdn.dexscreener.com/cms/images/f8b6bdc1ff962f2935c1734e857b317b181f7c7449475e8e27da061133523f6c?width=64&height=64&quality=90",
        0.000385,
        0.000312,
        0.12,
        "stop_loss",
    ),
    (
        "GRIFFAIN",
        "test griffain.com",
        "KENJSUYLASHUMfHyy5o4Hp2FdNqZg1AsUPhfH2kYvEP",
        "https://cdn.dexscreener.com/cms/images/5e450081609e72dfaaf692052cddcd67be9cd6cf2f51f269439bba874c5a4f7f?width=64&height=64&quality=90",
        0.000145,
        0.000118,
        0.15,
        "stop_loss",
    ),
    (
        "CHILLGUY",
        "Just a chill guy",
        "Df6yfrKC8kZE3KNkrHERKzAetSxbrWeniQfyJY4Jpump",
        "https://cdn.dexscreener.com/cms/images/20ae19e21d577f3aead6ae8722a7a3a66c5376cbf11f10d278807bef32551b46?width=64&height=64&quality=90",
        0.000178,
        0.000142,
        0.18,
        "stop_loss",
    ),
];

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
    };

    let uptime_secs = 3 * 24 * 3600 + 7 * 3600 + 23 * 60 + 45; // 3d 7h 23m 45s
    let system = SystemMetrics {
        uptime_seconds: uptime_secs,
        uptime_formatted: "3d 7h 23m 45s".to_string(),
        memory_mb: DEMO_MEMORY_MB,
        memory_percent: 2.4,
        cpu_percent: DEMO_CPU_PERCENT,
        cpu_history: vec![11.2, 13.5, 12.8, 10.9, 14.2, 12.3, 11.8, 13.1, 12.5, 11.7, 
                         12.9, 13.4, 11.5, 12.1, 13.8, 12.3, 11.9, 12.7, 13.2, 12.3],
        memory_history: vec![2.3, 2.4, 2.4, 2.3, 2.5, 2.4, 2.3, 2.4, 2.4, 2.3,
                            2.4, 2.5, 2.4, 2.3, 2.4, 2.4, 2.5, 2.4, 2.3, 2.4],
    };

    let tokens = TokenStatistics {
        total_in_database: 12847,
        with_prices: 8923,
        passed_filters: 347,
        rejected_filters: 8576,
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
        .map(|(symbol, _name, mint, _logo, entry, current, _size, hold_min)| {
            let pnl_pct = ((current - entry) / entry) * 100.0;
            OpenPositionDetail {
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                entry_price: *entry,
                current_price: Some(*current),
                pnl_percent: Some(pnl_pct),
                hold_duration_minutes: *hold_min,
            }
        })
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
        uptime_formatted: "3d 7h 23m 45s".to_string(),
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
    by_reason.insert("Manual".to_string(), 47);
    by_reason.insert("MintAuthority".to_string(), 523);
    by_reason.insert("FreezeAuthority".to_string(), 412);
    by_reason.insert("NonAuthority::RugPull".to_string(), 271);

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
                position_type: "long".to_string(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: current * 1.05,
                price_lowest: entry * 0.95,
                entry_transaction_signature: Some(format!("demo_entry_sig_{}", id_counter)),
                exit_transaction_signature: None,
                token_amount: Some((size / entry * 1e9) as u64),
                effective_entry_price: Some(*entry),
                effective_exit_price: None,
                sol_received: None,
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_string()),
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
            });
            id_counter += 1;
        }
    }

    // Add closed positions
    if include_closed {
        for (i, (symbol, name, mint, logo, entry, exit, size, reason)) in DEMO_CLOSED_TOKENS.iter().enumerate() {
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
                position_type: "long".to_string(),
                entry_size_sol: *size,
                total_size_sol: *size,
                price_highest: exit * 1.02,
                price_lowest: entry * 0.97,
                entry_transaction_signature: Some(format!("demo_entry_sig_{}", id_counter)),
                exit_transaction_signature: Some(format!("demo_exit_sig_{}", id_counter)),
                token_amount: Some((size / entry * 1e9) as u64),
                effective_entry_price: Some(*entry),
                effective_exit_price: Some(*exit),
                sol_received: Some(size + pnl),
                profit_target_min: Some(15.0),
                profit_target_max: Some(50.0),
                liquidity_tier: Some("high".to_string()),
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
            mint: "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN".to_string(), // TRUMP
            balance: 125_340_000,
            balance_ui: 125.34,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "MEW1gQWJ3nEXg2qgERiKu7FAFj79PHvQVREQUzScPP5".to_string(), // MEW
            balance: 2_345_000_000,
            balance_ui: 2345.0,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_string(), // Fartcoin
            balance: 847_000_000,
            balance_ui: 847.0,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "ukHH6c7mMyiWCf1b9pnWe25TSpkDDt3H5pQZgZ74J82".to_string(), // BOME
            balance: 1_234_000_000,
            balance_ui: 1234.0,
            decimals: 6,
            is_token_2022: false,
        },
        TokenBalanceInfo {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
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
                exit_type: "trailing_stop".to_string(),
                count: 42,
                avg_profit_pct: 31.8,
            },
            ExitBreakdown {
                exit_type: "take_profit".to_string(),
                count: 38,
                avg_profit_pct: 45.2,
            },
            ExitBreakdown {
                exit_type: "stop_loss".to_string(),
                count: 18,
                avg_profit_pct: -18.5,
            },
            ExitBreakdown {
                exit_type: "manual".to_string(),
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
