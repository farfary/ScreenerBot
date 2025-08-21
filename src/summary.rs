use crate::trader::*;
use crate::positions::*;
use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::utils::*;
use crate::global::{STARTUP_TIME, is_debug_summary_enabled};
use crate::arguments::{is_summary_enabled, is_dashboard_enabled};
use crate::ata_cleanup::{ get_ata_cleanup_statistics, get_failed_ata_count };
use crate::rpc::get_global_rpc_stats;
use crate::tokens::pool::get_pool_service;
use crate::trader::PROFIT_EXTRA_NEEDED_SOL;
use crate::transactions::{TransactionsManager, SwapPnLInfo};
use crate::utils::get_wallet_address;
// New pool price system is now integrated via background services

use chrono::{ Utc };
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::Notify;
use std::time::{Duration, Instant};
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };
use crate::tokens::discovery::get_discovery_stats;

/// Display structure for closed positions with specific "Exit" column
#[derive(Tabled)]
pub struct ClosedPositionDisplay {
    #[tabled(rename = "🏷️ Symbol")]
    symbol: String,
    #[tabled(rename = "🔑 Mint")]
    mint: String,
    #[tabled(rename = "📈 Entry")]
    entry_price: String,
    #[tabled(rename = "🚪 Exit")]
    exit_price: String,
    #[tabled(rename = "💰 Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "💸 P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "📊 P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "💳 Fees (SOL)")]
    pub fees_sol: String,
    #[tabled(rename = "⏱️ Duration")]
    duration: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}

/// Display structure for open positions with specific "Price" column
#[derive(Tabled)]
pub struct OpenPositionDisplay {
    #[tabled(rename = "🏷️ Symbol")]
    symbol: String,
    #[tabled(rename = "🔑 Mint")]
    mint: String,
    #[tabled(rename = "📈 Entry")]
    entry_price: String,
    #[tabled(rename = "💲 Price")]
    current_price: String,
    #[tabled(rename = "💰 Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "💸 P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "📊 P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "💳 Fees (SOL)")]
    pub fees_sol: String,
    #[tabled(rename = "⏱️ Duration")]
    duration: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}

/// Display structure for bot summary overview
#[derive(Tabled)]
pub struct BotOverviewDisplay {
    #[tabled(rename = "💼 Wallet Balance")]
    wallet_balance: String,
    #[tabled(rename = "🔄 Open Positions")]
    open_positions: String,
    #[tabled(rename = "📊 Total Trades")]
    total_trades: usize,
    #[tabled(rename = "⏰ Bot Uptime")]
    bot_uptime: String,
    #[tabled(rename = "💸 Total P&L")]
    total_pnl: String,
}

/// Display structure for detailed trading statistics
#[derive(Tabled)]
pub struct TradingStatsDisplay {
    #[tabled(rename = "🎯 Win Rate")]
    win_rate: String,
    #[tabled(rename = "🏆 Winners")]
    winners: usize,
    #[tabled(rename = "❌ Losers")]
    losers: usize,
    #[tabled(rename = "⚖️ Break-even")]
    break_even: usize,
    #[tabled(rename = "📊 Avg P&L/Trade")]
    avg_pnl: String,
    #[tabled(rename = "💰 Trade Volume")]
    total_volume: String,
}

/// Display structure for performance metrics
#[derive(Tabled)]
pub struct PerformanceDisplay {
    #[tabled(rename = "🚀 Best Trade")]
    best_trade: String,
    #[tabled(rename = "💀 Worst Trade")]
    worst_trade: String,
    #[tabled(rename = "⚡ Profit Factor")]
    profit_factor: String,
    #[tabled(rename = "📉 Max Drawdown")]
    max_drawdown: String,
    #[tabled(rename = "🔥 Best Streak")]
    best_streak: String,
    #[tabled(rename = "🧊 Worst Streak")]
    worst_streak: String,
}

/// Display structure for ATA cleanup statistics
#[derive(Tabled)]
pub struct AtaCleanupDisplay {
    #[tabled(rename = "🧹 ATAs Closed")]
    atas_closed: String,
    #[tabled(rename = "💰 Rent Reclaimed")]
    rent_reclaimed: String,
    #[tabled(rename = "❌ Failed Cache")]
    failed_cache: String,
    #[tabled(rename = "⏰ Last Cleanup")]
    last_cleanup: String,
}

/// Display structure for pool service statistics
#[derive(Tabled)]
pub struct PoolServiceDisplay {
    #[tabled(rename = "🏊 Memory Cache")]
    memory_cache: String,
    #[tabled(rename = "💰 Price Cache")]
    price_cache: String,
    #[tabled(rename = "⏳ Cycles")]
    cycles: String,
    #[tabled(rename = "📦 Avg/Chunk")]
    avg_per_chunk: String,
    #[tabled(rename = "⏱️ Last/Avg (ms)")]
    last_avg_ms: String,
    #[tabled(rename = "📞 Total Requests")]
    total_requests: String,
    #[tabled(rename = "✅ Success Rate")]
    success_rate: String,
    #[tabled(rename = "🔄 Cache Hits")]
    cache_hit_rate: String,
    #[tabled(rename = "⛓️ Blockchain")]
    blockchain_calcs: String,
    #[tabled(rename = "📈 Memory History")]
    memory_history: String,
    #[tabled(rename = "📡 Watch (tot/exp/never)")]
    watch_snapshot: String,
}

/// Display structure for detailed disk cache statistics
#[derive(Tabled)]
pub struct PoolDiskCacheDisplay {
    #[tabled(rename = "💾 Disk Tokens")]
    disk_tokens: String,
    #[tabled(rename = "🏊 Disk Pools")]
    disk_pools: String,
    #[tabled(rename = "📁 Cache Files")]
    cache_files: String,
    #[tabled(rename = "📊 Total Entries")]
    total_entries: String,
    #[tabled(rename = "💿 Cache Size")]
    cache_size: String,
    #[tabled(rename = "📅 Data Range")]
    data_range: String,
    #[tabled(rename = "📈 Avg/Token")]
    avg_per_token: String,
}

/// Display structure for Discovery statistics (printed first, compact)
#[derive(Tabled)]
pub struct DiscoveryDisplay {
    #[tabled(rename = "� Cycles")]
    cycles: String,
    #[tabled(rename = "📦 Proc/Add")]
    proc_add: String,
    #[tabled(rename = "🧹 Dedup/BL")]
    filters: String,
    #[tabled(rename = "📚 Sources (prof/boost/top | new/view/trend/verify)")]
    sources: String,
    #[tabled(rename = "⚠️ Error")]
    error: String,
}

/// Display structure for RPC URL usage statistics
#[derive(Tabled)]
pub struct RpcUrlStatsDisplay {
    #[tabled(rename = "🌐 RPC URL")]
    rpc_url: String,
    #[tabled(rename = "📞 Total Calls")]
    total_calls: String,
    #[tabled(rename = "📊 Percentage")]
    percentage: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}

/// Display structure for RPC method usage statistics
#[derive(Tabled)]
pub struct RpcMethodStatsDisplay {
    #[tabled(rename = "⚙️ RPC Method")]
    method_name: String,
    #[tabled(rename = "📞 Total Calls")]
    total_calls: String,
    #[tabled(rename = "📊 Percentage")]
    percentage: String,
    #[tabled(rename = "⚡ Avg/Sec")]
    calls_per_second: String,
}

/// Display structure for RPC overview statistics
#[derive(Tabled)]
pub struct RpcOverviewDisplay {
    #[tabled(rename = "📞 Total Calls")]
    total_calls: String,
    #[tabled(rename = "🌐 Active URLs")]
    active_urls: String,
    #[tabled(rename = "⚙️ Methods Used")]
    methods_used: String,
    #[tabled(rename = "⚡ Calls/Sec")]
    calls_per_second: String,
    #[tabled(rename = "⏰ Since Startup")]
    uptime: String,
}

/// Display structure for recent swaps table
#[derive(Tabled)]
pub struct RecentSwapDisplay {
    #[tabled(rename = "📅 Date")]
    date: String,
    #[tabled(rename = "⏰ Time")]
    time: String,
    #[tabled(rename = "⏳ Ago")]
    ago: String,
    #[tabled(rename = "🔑 Signature")]
    signature: String,
    #[tabled(rename = "🔄 Type")]
    swap_type: String,
    #[tabled(rename = "🏷️ Token")]
    token: String,
    #[tabled(rename = "💰 SOL")]
    sol_amount: String,
    #[tabled(rename = "🪙 Tokens")]
    token_amount: String,
    #[tabled(rename = "💲 Price")]
    price: String,
    #[tabled(rename = "🌐 Router")]
    router: String,
    #[tabled(rename = "💳 Fee")]
    fee: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}

/// Display structure for recent transactions table (last 20)
#[derive(Tabled)]
pub struct RecentTransactionDisplay {
    #[tabled(rename = "📅 Date")]
    date: String,
    #[tabled(rename = "⏰ Time")]
    time: String,
    #[tabled(rename = "⏳ Ago")]
    ago: String,
    #[tabled(rename = "🔑 Signature")]
    signature: String,
    #[tabled(rename = "🔢 Slot")]
    slot: String,
    #[tabled(rename = "🔄 Type")]
    tx_type: String,
    #[tabled(rename = "🏷️ Token")]
    token: String,
    #[tabled(rename = "💱 SOL Δ")]
    sol_delta: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}


/// Display structure for wallet transaction statistics
#[derive(Tabled)]
pub struct WalletTransactionDisplay {
    #[tabled(rename = "💾 Cached Transactions")]
    cached_transactions: String,
    #[tabled(rename = "📈 Total Fetched")]
    total_fetched: String,
    #[tabled(rename = "⏰ Last Sync")]
    last_sync: String,
    #[tabled(rename = "� Periodic Sync")]
    periodic_sync_status: String,
    #[tabled(rename = "📅 Oldest Signature")]
    oldest_signature: String,
    #[tabled(rename = "🆕 Newest Signature")]
    newest_signature: String,
}

/// Display structure for transaction finalization statistics
#[derive(Tabled)]
pub struct TransactionFinalizationDisplay {
    #[tabled(rename = "🔒 Total Finalized")]
    total_finalized: String,
    #[tabled(rename = "⏳ Pending Finalization")]
    pending_finalization: String,
    #[tabled(rename = "⏱️ Avg Finalization Time")]
    average_finalization_time: String,
    #[tabled(rename = "📦 Last Batch Size")]
    last_batch_size: String,
    #[tabled(rename = "🔄 Next Check")]
    next_check_status: String,
}



/// Background task to display positions table every 10 seconds
pub async fn monitor_positions_display(shutdown: Arc<Notify>) {
    if is_debug_summary_enabled() && !is_dashboard_enabled() {
        log(LogTag::Summary, "DEBUG", "Starting positions display monitor");
    }

    let mut tick: u64 = 0;
    loop {
        tick += 1;
        let tick_start = Instant::now();
        if is_debug_summary_enabled() {
            log(
                LogTag::Summary,
                "DEBUG",
                &format!("Summary tick #{} start - generating positions table", tick)
            );
        }

        // Display the positions table
        display_positions_table().await;

        if is_debug_summary_enabled() {
            let elapsed = tick_start.elapsed();
            log(
                LogTag::Summary,
                "DEBUG",
                &format!(
                    "Summary tick #{} display complete in {} ms",
                    tick,
                    elapsed.as_millis()
                )
            );
            log(
                LogTag::Summary,
                "DEBUG",
                &format!(
                    "Summary tick #{} sleeping for {}s (or until shutdown)",
                    tick,
                    SUMMARY_DISPLAY_INTERVAL_SECS
                )
            );
        }

        // Wait 10 seconds or until shutdown
        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(SUMMARY_DISPLAY_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "positions display monitor shutting down...");
            if is_debug_summary_enabled() {
                log(LogTag::Summary, "DEBUG", "Positions display monitor shutdown complete");
            }
            break;
        }
    }
}

pub async fn display_positions_table() {
    let fn_start = Instant::now();
    if is_debug_summary_enabled() && !is_dashboard_enabled() {
        log(LogTag::Summary, "DEBUG", "Starting positions table display generation");
    }

    // The new pool price system runs in background and continuously updates prices
    // for open positions, so we don't need to refresh them here

    // Use existing safe functions instead of locking SAVED_POSITIONS directly
    let collect_start = Instant::now();
    let open_positions = get_open_positions().await;
    let closed_positions = get_closed_positions().await;
    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Collected positions in {} ms",
                collect_start.elapsed().as_millis()
            )
        );
    }

    let open_count = open_positions.len();
    let closed_count = closed_positions.len();
    let total_invested: f64 = open_positions
        .iter()
        .map(|p| p.entry_size_sol)
        .sum();
    let total_pnl: f64 = closed_positions
        .iter()
        .map(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol
        })
        .sum();

    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Positions data collected - Open: {}, Closed: {}, Invested: {:.6} SOL, P&L: {:.6} SOL",
                open_count,
                closed_count,
                total_invested,
                total_pnl
            )
        );
    }

    // Build all positions output in one shot
    let mut positions_output = String::new();

    // Discovery section FIRST (less important but requested first)
    {
        // Add timeout protection for discovery stats
        let ds = match tokio::time::timeout(Duration::from_secs(2), get_discovery_stats()).await {
            Ok(stats) => stats,
            Err(_) => {
                log(LogTag::Summary, "WARN", "Discovery stats timeout - using default");
                crate::tokens::discovery::DiscoveryStats::default()
            }
        };
        let cycles = format!("{}", ds.total_cycles);
        let proc_add = format!("{}/{}", ds.last_processed, ds.last_added);
        let filters = format!("{}/{}", ds.last_deduplicated_removed, ds.last_blacklist_removed);
        let sources = format!(
            "{}/{}/{} | {}/{}/{}/{}",
            ds.per_source.profiles,
            ds.per_source.boosted,
            ds.per_source.top_boosts,
            ds.per_source.rug_new,
            ds.per_source.rug_viewed,
            ds.per_source.rug_trending,
            ds.per_source.rug_verified,
        );
        let error = ds.last_error.unwrap_or_default();

        let discovery_display = DiscoveryDisplay {
            cycles,
            proc_add,
            filters,
            sources,
            error,
        };

        positions_output.push_str("\n🧭 Discovery\n");
        let mut discovery_table = Table::new(vec![discovery_display]);
        discovery_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        positions_output.push_str(&format!("{}\n", discovery_table));
    }

    // Display bot summary section (now with owned data)
    let closed_refs: Vec<&Position> = closed_positions.iter().collect();
    let summary_start = Instant::now();
    let bot_summary = build_bot_summary(&closed_refs).await;
    positions_output.push_str(&bot_summary);
    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Bot summary built in {} ms",
                summary_start.elapsed().as_millis()
            )
        );
    }

    // Build closed positions first (last 10, sorted by close time)
    if !closed_positions.is_empty() {
        if is_debug_summary_enabled() {
            log(
                LogTag::Summary,
                "DEBUG",
                &format!("Processing {} closed positions for display", closed_positions.len())
            );
        }

        let mut sorted_closed = closed_positions.clone();
        sorted_closed.sort_by_key(|p| p.exit_time.unwrap_or(Utc::now()));

        let closed_build_start = Instant::now();
        let recent_closed: Vec<_> = sorted_closed
            .iter()
            .rev() // Most recent first
            .take(10) // Take last 10
            .rev() // Reverse back so oldest of the 10 is first
            .map(|position| ClosedPositionDisplay::from_position(position))
            .collect();
        if is_debug_summary_enabled() {
            log(
                LogTag::Summary,
                "DEBUG",
                &format!(
                    "Built recent closed positions (n={}) in {} ms",
                    recent_closed.len(),
                    closed_build_start.elapsed().as_millis()
                )
            );
        }

        if !recent_closed.is_empty() {
            if is_debug_summary_enabled() {
                log(
                    LogTag::Summary,
                    "DEBUG",
                    &format!("Building {} recent closed positions table", recent_closed.len())
                );
            }

            positions_output.push_str(&format!("\n📋 Recently Closed Positions (Last 10):\n"));
            let mut closed_table = Table::new(recent_closed);
            closed_table
                .with(Style::rounded())
                .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
            positions_output.push_str(&format!("{}\n\n", closed_table));
        }
    }

    // Build open positions (sorted by entry time, latest at bottom)
    if !open_positions.is_empty() {
        if is_debug_summary_enabled() {
            log(
                LogTag::Summary,
                "DEBUG",
                &format!("Processing {} open positions for display", open_positions.len())
            );
        }

        let mut sorted_open = open_positions.clone();
        sorted_open.sort_by_key(|p| p.entry_time);

        let open_position_displays: Vec<_> = {
            // Collect all mints that need prices
            let mints: Vec<String> = sorted_open
                .iter()
                .map(|position| position.mint.clone())
                .collect();

            if is_debug_summary_enabled() {
                log(
                    LogTag::Summary,
                    "DEBUG",
                    &format!("Fetching current prices for {} tokens", mints.len())
                );
            }

            // Fetch all prices in one batch call (much faster!) with timeout protection
            let price_fetch_start = Instant::now();
            let price_map = match tokio::time::timeout(
                Duration::from_secs(4), 
                crate::tokens::get_current_token_prices_batch(&mints)
            ).await {
                Ok(prices) => prices,
                Err(_) => {
                    log(LogTag::Summary, "WARN", "Token price batch fetch timeout - using empty map");
                    std::collections::HashMap::new()
                }
            };

            if is_debug_summary_enabled() {
                let prices_found = price_map.values().filter(|p| p.is_some()).count();
                log(
                    LogTag::Summary,
                    "DEBUG",
                    &format!(
                        "Price fetching complete - Found prices for {}/{} tokens in {} ms",
                        prices_found,
                        mints.len(),
                        price_fetch_start.elapsed().as_millis()
                    )
                );
            }

            // Build displays with fetched prices
            let mut displays = Vec::new();
            let build_start = Instant::now();
            for position in &sorted_open {
                let current_price = price_map.get(&position.mint).copied().flatten();
                displays.push(OpenPositionDisplay::from_position(position, current_price));
            }
            if is_debug_summary_enabled() {
                log(
                    LogTag::Summary,
                    "DEBUG",
                    &format!(
                        "Built open positions display (n={}) in {} ms",
                        displays.len(),
                        build_start.elapsed().as_millis()
                    )
                );
            }
            displays
        };

        positions_output.push_str(&format!("\n🔄 Open Positions ({}):\n", open_positions.len()));
        let mut open_table = Table::new(open_position_displays);
        open_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        positions_output.push_str(&format!("{}\n\n", open_table));

        if is_debug_summary_enabled() && !is_dashboard_enabled() {
            log(LogTag::Summary, "DEBUG", "Open positions table built");
        }
    }

    // Display everything in one shot
    if is_summary_enabled() && !is_dashboard_enabled() {
        print!("{}", positions_output);
    }

    if is_debug_summary_enabled() && !is_dashboard_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Positions table display generation complete in {} ms",
                fn_start.elapsed().as_millis()
            )
        );
    }
}

/// Convenience function to build bot summary using current positions and return as string
pub async fn build_current_bot_summary() -> String {
    let closed_positions = get_closed_positions().await;
    let refs: Vec<&_> = closed_positions.iter().collect();
    build_bot_summary(&refs).await
}

/// Builds comprehensive bot summary with detailed statistics and performance metrics and returns as string
pub async fn build_bot_summary(closed_positions: &[&Position]) -> String {
    let fn_start = Instant::now();
    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!("Starting bot summary generation with {} closed positions", closed_positions.len())
        );
    }

    // Get open positions count using existing function
    let open_count = get_open_positions_count().await;

    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!("Found {} open positions for summary", open_count)
        );
    }

    // Calculate comprehensive trading statistics
    let stats_start = Instant::now();
    let total_trades = closed_positions.len();
    let profitable_trades = closed_positions
        .iter()
        .filter(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol > 0.0
        })
        .count();
    let losing_trades = closed_positions
        .iter()
        .filter(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol < 0.0
        })
        .count();
    let break_even_trades = total_trades - profitable_trades - losing_trades;

    let win_rate = if total_trades > 0 {
        ((profitable_trades as f64) / (total_trades as f64)) * 100.0
    } else {
        0.0
    };

    // Calculate P&L metrics
    let pnl_values: Vec<f64> = closed_positions
        .iter()
        .map(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol
        })
        .collect();

    let total_pnl: f64 = pnl_values.iter().sum();
    let avg_pnl_per_trade = if total_trades > 0 { total_pnl / (total_trades as f64) } else { 0.0 };

    let best_trade = pnl_values
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0.0);

    let worst_trade = pnl_values
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0.0);

    // Calculate advanced metrics
    let total_volume = closed_positions
        .iter()
        .map(|p| p.entry_size_sol)
        .sum::<f64>();

    let total_gains: f64 = pnl_values
        .iter()
        .filter(|&&x| x > 0.0)
        .sum();
    let total_losses: f64 = pnl_values
        .iter()
        .filter(|&&x| x < 0.0)
        .sum::<f64>()
        .abs();
    let profit_factor = if total_losses > 0.0 { total_gains / total_losses } else { 0.0 };

    // Calculate streaks
    let (best_streak, worst_streak) = calculate_win_loss_streaks(&pnl_values);

    // Calculate maximum drawdown
    let max_drawdown = calculate_max_drawdown(&pnl_values);
    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!("Computed P&L stats in {} ms", stats_start.elapsed().as_millis())
        );
    }

    // Get wallet balance from wallet tracker
    let wallet_start = Instant::now();
    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Fetched wallet summary in {} ms",
                wallet_start.elapsed().as_millis()
            )
        );
    }

    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Statistics calculated - Win rate: {:.1}%, Total P&L: {:.6} SOL, Best: {:.6}, Worst: {:.6}",
                win_rate,
                total_pnl,
                best_trade,
                worst_trade
            )
        );
    }

    // Calculate wallet balance
    let wallet_balance = if let Ok(wallet_pubkey) = crate::utils::get_wallet_address() {
        match crate::utils::get_sol_balance(&wallet_pubkey).await {
            Ok(balance) => format!("{:.6} SOL", balance),
            Err(_) => "Error".to_string(),
        }
    } else {
        "N/A".to_string()
    };

    // Calculate bot uptime
    let uptime = format_duration_compact(*STARTUP_TIME, Utc::now());

    // Create display structures
    let overview = BotOverviewDisplay {
        wallet_balance,
        open_positions: format!("{}", open_count),
        total_trades,
        bot_uptime: uptime,
        total_pnl: format!("{:+.6} SOL", total_pnl),
    };

    let trading_stats = TradingStatsDisplay {
        win_rate: format!("{:.1}%", win_rate),
        winners: profitable_trades,
        losers: losing_trades,
        break_even: break_even_trades,
        avg_pnl: format!("{:+.6} SOL", avg_pnl_per_trade),
        total_volume: format!("{:.3} SOL", total_volume),
    };

    let performance = PerformanceDisplay {
        best_trade: format!("{:+.6} SOL", best_trade),
        worst_trade: format!("{:+.6} SOL", worst_trade),
        profit_factor: format!("{:.2}", profit_factor),
        max_drawdown: format!("{:.2}%", max_drawdown),
        best_streak: format!("{} wins", best_streak),
        worst_streak: format!("{} losses", worst_streak),
    };

    // Get ATA cleanup statistics
    let ata_stats = get_ata_cleanup_statistics();
    let failed_ata_count = get_failed_ata_count();

    let ata_cleanup = AtaCleanupDisplay {
        atas_closed: format!("{}", ata_stats.total_closed),
        rent_reclaimed: format!("{:.6} SOL", ata_stats.total_rent_reclaimed),
        failed_cache: format!("{} ATAs", failed_ata_count),
        last_cleanup: ata_stats.last_cleanup_time.unwrap_or_else(|| "Never".to_string()),
    };

    // Get pool service statistics with timeout protection
    let pool_service = get_pool_service();
    let pool_stats_start = Instant::now();
    
    // Add timeout protection for pool service calls
    let (pool_cache_count, price_cache_count, _availability_cache_count) = 
        match tokio::time::timeout(Duration::from_secs(3), pool_service.get_cache_stats()).await {
            Ok(stats) => stats,
            Err(_) => {
                log(LogTag::Summary, "WARN", "Pool cache stats timeout - using default");
                (0, 0, 0)
            }
        };
    
    let enhanced_stats = match tokio::time::timeout(Duration::from_secs(3), pool_service.get_enhanced_stats()).await {
        Ok(stats) => stats,
        Err(_) => {
            log(LogTag::Summary, "WARN", "Enhanced pool stats timeout - using default");
            crate::tokens::pool::PoolServiceStats::default()
        }
    };
    
    // Get detailed disk cache statistics with timeout
    let disk_cache_stats = match tokio::time::timeout(Duration::from_secs(2), pool_service.get_disk_cache_stats()).await {
        Ok(Ok(stats)) => stats,
        _ => {
            log(LogTag::Summary, "WARN", "Disk cache stats timeout - using default");
            crate::tokens::pool::PoolDiskCacheStats::default()
        }
    };
    
    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Fetched pool service stats in {} ms",
                pool_stats_start.elapsed().as_millis()
            )
        );
    }

    let pool_service_stats = PoolServiceDisplay {
        memory_cache: format!("{} pools", pool_cache_count),
        price_cache: format!("{} prices", price_cache_count),
        cycles: format!("{}", enhanced_stats.monitoring_cycles),
        avg_per_chunk: if enhanced_stats.monitoring_cycles > 0 {
            format!("{:.1}", enhanced_stats.avg_tokens_per_cycle)
        } else { "N/A".to_string() },
        last_avg_ms: if enhanced_stats.monitoring_cycles > 0 {
            format!("{:.0}/{:.0}", enhanced_stats.last_cycle_duration_ms, enhanced_stats.avg_cycle_duration_ms)
        } else { "-".to_string() },
        total_requests: format!("{}", enhanced_stats.total_price_requests),
        success_rate: if enhanced_stats.total_price_requests > 0 {
            format!("{:.1}%", enhanced_stats.get_success_rate())
        } else {
            "N/A".to_string()
        },
        cache_hit_rate: if enhanced_stats.total_price_requests > 0 {
            format!("{:.1}%", enhanced_stats.get_cache_hit_rate())
        } else {
            "0.0%".to_string()
        },
        blockchain_calcs: format!("{}", enhanced_stats.blockchain_calculations),
        memory_history: format!("{} tokens", enhanced_stats.tokens_with_price_history),
        watch_snapshot: format!("{}/{}/{}", enhanced_stats.watch_total, enhanced_stats.watch_expired, enhanced_stats.watch_never_checked),
    };

    // Build disk cache display
    let disk_cache_display = PoolDiskCacheDisplay {
        disk_tokens: format!("{}", disk_cache_stats.total_tokens),
        disk_pools: format!("{}", disk_cache_stats.total_pools),
        cache_files: format!("{}", disk_cache_stats.total_files),
        total_entries: format!("{}", disk_cache_stats.total_entries),
        cache_size: if disk_cache_stats.total_size_bytes > 0 {
            format!("{:.2} MB", disk_cache_stats.get_cache_size_mb())
        } else {
            "0 MB".to_string()
        },
        data_range: if let (Some(oldest), Some(newest)) = (disk_cache_stats.oldest_entry, disk_cache_stats.newest_entry) {
            let duration = newest.signed_duration_since(oldest);
            if duration.num_hours() > 0 {
                format!("{}h ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{}m ago", duration.num_minutes())
            } else {
                format!("{}s ago", duration.num_seconds())
            }
        } else {
            "No data".to_string()
        },
        avg_per_token: if disk_cache_stats.total_tokens > 0 {
            format!("{:.1} pools, {:.0} entries", 
                disk_cache_stats.get_avg_pools_per_token(),
                disk_cache_stats.get_avg_entries_per_token())
        } else {
            "N/A".to_string()
        },
    };

    // Build all table strings first, then display in one shot
    if is_debug_summary_enabled() {
        log(LogTag::Summary, "DEBUG", "Building bot overview tables");
    }

    let mut summary_output = String::new();

    // Build Bot Overview table
    summary_output.push_str("\n📊 Bot Overview\n");
    let mut overview_table = Table::new(vec![overview]);
    overview_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary_output.push_str(&format!("{}\n", overview_table));

    // Build Trading Statistics table
    summary_output.push_str("\n📈 Trading Statistics\n");
    let mut stats_table = Table::new(vec![trading_stats]);
    stats_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary_output.push_str(&format!("{}\n", stats_table));

    // Build Performance Metrics table
    summary_output.push_str("\n🎯 Performance Metrics\n");
    let mut performance_table = Table::new(vec![performance]);
    performance_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary_output.push_str(&format!("{}\n", performance_table));

    // Build ATA Cleanup Statistics table
    summary_output.push_str("\n🧹 ATA Cleanup Statistics\n");
    let mut ata_table = Table::new(vec![ata_cleanup]);
    ata_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary_output.push_str(&format!("{}\n", ata_table));

    // Build Pool Service Statistics table
    summary_output.push_str("\n🏊 Pool Service Statistics\n");
    let mut pool_table = Table::new(vec![pool_service_stats]);
    pool_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary_output.push_str(&format!("{}\n", pool_table));

    // Build Pool Disk Cache Statistics table
    summary_output.push_str("\n💾 Pool Disk Cache Statistics\n");
    let mut disk_cache_table = Table::new(vec![disk_cache_display]);
    disk_cache_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary_output.push_str(&format!("{}\n", disk_cache_table));

    // Build Recent Swaps table (last 20)
    if is_debug_summary_enabled() {
        log(LogTag::Summary, "DEBUG", "Fetching recent swaps for summary");
    }
    let swaps_start = Instant::now();
    match build_recent_swaps_table().await {
        Ok(swaps_table) => {
            summary_output.push_str(&swaps_table);
            if is_debug_summary_enabled() {
                log(
                    LogTag::Summary,
                    "DEBUG",
                    &format!(
                        "Recent swaps table built in {} ms",
                        swaps_start.elapsed().as_millis()
                    )
                );
            }
        }
        Err(e) => {
            if is_debug_summary_enabled() {
                log(LogTag::Summary, "DEBUG", &format!("Failed to build recent swaps table: {}", e));
            }
        }
    }

    // Build Recent Transactions table (last 20)
    match build_recent_transactions_table().await {
        Ok(tx_table) => {
            summary_output.push_str(&tx_table);
        }
        Err(e) => {
            log(LogTag::Summary, "WARN", &format!("Failed to build recent transactions table: {}", e));
        }
    }

    

    // Build RPC statistics tables if available
    if let Some(rpc_stats) = get_global_rpc_stats() {
        if is_debug_summary_enabled() {
            log(LogTag::Summary, "DEBUG", "Building RPC statistics tables");
        }
        let rpc_start = Instant::now();
        let rpc_tables = build_rpc_statistics_tables(&rpc_stats);
        summary_output.push_str(&rpc_tables);
        if is_debug_summary_enabled() {
            log(
                LogTag::Summary,
                "DEBUG",
                &format!(
                    "RPC statistics tables built in {} ms",
                    rpc_start.elapsed().as_millis()
                )
            );
        }
    }


    // Build frozen account cooldowns if any exist
    let active_cooldowns = crate::positions::get_active_frozen_cooldowns().await;
    if !active_cooldowns.is_empty() {
        summary_output.push_str("\n❄️ Frozen Account Cooldowns\n");
        for (mint, remaining_minutes) in active_cooldowns {
            let short_mint = format!("{}...", &mint[..8]);
            summary_output.push_str(&format!("  {} - {} minutes remaining\n", short_mint, remaining_minutes));
        }
    }

    summary_output.push_str("\n");

    if is_debug_summary_enabled() {
        log(
            LogTag::Summary,
            "DEBUG",
            &format!(
                "Bot summary build generation complete in {} ms",
                fn_start.elapsed().as_millis()
            )
        );
    }

    summary_output
}

/// Display comprehensive bot summary with detailed statistics and performance metrics (backwards compatibility)
pub async fn display_bot_summary(closed_positions: &[&Position]) {
    let summary = build_bot_summary(closed_positions).await;
    if is_summary_enabled() && !is_dashboard_enabled() {
        print!("{}", summary);
    }
}

/// Convenience function to display bot summary using current positions (backwards compatibility)
pub async fn display_current_bot_summary() {
    let summary = build_current_bot_summary().await;
    if is_summary_enabled() && !is_dashboard_enabled() {
        print!("{}", summary);
    }
}


/// Display RPC usage statistics (backwards compatibility)
pub fn display_rpc_statistics(rpc_stats: &crate::rpc::RpcStats) {
    let rpc_tables = build_rpc_statistics_tables(rpc_stats);
    if is_summary_enabled() && !is_dashboard_enabled() {
        print!("{}", rpc_tables);
    }
}

/// Calculate consecutive win/loss streaks
fn calculate_win_loss_streaks(pnl_values: &[f64]) -> (usize, usize) {
    if pnl_values.is_empty() {
        return (0, 0);
    }

    let mut best_win_streak = 0;
    let mut worst_loss_streak = 0;
    let mut current_win_streak = 0;
    let mut current_loss_streak = 0;

    for &pnl in pnl_values {
        if pnl > 0.0 {
            current_win_streak += 1;
            current_loss_streak = 0;
            best_win_streak = best_win_streak.max(current_win_streak);
        } else if pnl < 0.0 {
            current_loss_streak += 1;
            current_win_streak = 0;
            worst_loss_streak = worst_loss_streak.max(current_loss_streak);
        } else {
            // Break even trades reset both streaks
            current_win_streak = 0;
            current_loss_streak = 0;
        }
    }

    (best_win_streak, worst_loss_streak)
}

/// Build recent swaps table and return as string
async fn build_recent_swaps_table() -> Result<String, String> {
    let wallet_address_str = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    let wallet_pubkey = solana_sdk::pubkey::Pubkey::from_str(&wallet_address_str)
        .map_err(|e| format!("Invalid wallet address: {}", e))?;
    let mut manager = TransactionsManager::new(wallet_pubkey).await?;
    
    // Get last 20 swaps (already sorted newest-first by manager)
    let swaps = manager.get_all_swap_transactions_limited(Some(20)).await?;
    
    if swaps.is_empty() {
        return Ok("\n📈 Recent Swaps (Last 20)\nNo swaps found\n\n".to_string());
    }

    let recent_swaps: Vec<RecentSwapDisplay> = swaps
        .into_iter()
        .map(|swap| RecentSwapDisplay::from_swap_pnl_info(&swap))
        .collect();

    let mut output = String::new();
    output.push_str("\n📈 Recent Swaps (Last 20)\n");
    let mut swaps_table = Table::new(recent_swaps);
    swaps_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    output.push_str(&format!("{}\n", swaps_table));
    
    Ok(output)
}

/// Build recent transactions table (last 20 by time) and return as string
async fn build_recent_transactions_table() -> Result<String, String> {
    let wallet_address_str = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    let wallet_pubkey = solana_sdk::pubkey::Pubkey::from_str(&wallet_address_str)
        .map_err(|e| format!("Invalid wallet address: {}", e))?;
    let mut manager = TransactionsManager::new(wallet_pubkey).await?;

    // Pull a smaller window to reduce processing - get 25 and take best 20
    let mut txs = manager
        .get_recent_transactions(25)
        .await
        .map_err(|e| format!("Failed to get recent transactions: {}", e))?;

    // Since get_recent_transactions already does hydration, only recalc if really needed
    for tx in &mut txs {
        // Only recalc if hydration failed AND transaction is finalized (worth the cost)
        if matches!(tx.transaction_type, crate::transactions::TransactionType::Unknown) && 
           matches!(tx.status, crate::transactions::TransactionStatus::Finalized) {
            let _ = manager.recalculate_transaction_analysis(tx).await;
        }
    }

    // Sort by timestamp desc and take last 20
    txs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    txs.truncate(20);

    if txs.is_empty() {
        return Ok("\n🧾 Recent Transactions (Last 20)\nNo transactions found\n\n".to_string());
    }

    let rows: Vec<RecentTransactionDisplay> = txs
        .into_iter()
        .map(RecentTransactionDisplay::from_transaction)
        .collect();

    let mut output = String::new();
    output.push_str("\n🧾 Recent Transactions (Last 20)\n");
    let mut table = Table::new(rows);
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    output.push_str(&format!("{}\n", table));

    Ok(output)
}

/// Build RPC usage statistics tables and return as string
fn build_rpc_statistics_tables(rpc_stats: &crate::rpc::RpcStats) -> String {
    let mut output = String::new();
    let total_calls = rpc_stats.total_calls();
    if total_calls == 0 {
        return output; // No calls to display
    }

    // RPC Overview
    let uptime = format_duration_compact(rpc_stats.startup_time, Utc::now());
    let rpc_overview = RpcOverviewDisplay {
        total_calls: format!("{}", total_calls),
        active_urls: format!("{}", rpc_stats.calls_per_url.len()),
        methods_used: format!("{}", rpc_stats.calls_per_method.len()),
        calls_per_second: format!("{:.2}", rpc_stats.calls_per_second()),
        uptime,
    };

    output.push_str("\n📡 RPC Overview\n");
    let mut overview_table = Table::new(vec![rpc_overview]);
    overview_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    output.push_str(&format!("{}\n", overview_table));

    // RPC URL Statistics (top 5)
    let mut url_stats: Vec<_> = rpc_stats.calls_per_url.iter().collect();
    url_stats.sort_by(|a, b| b.1.cmp(a.1)); // Sort by call count descending

    if !url_stats.is_empty() {
        let url_displays: Vec<_> = url_stats
            .iter()
            .take(5) // Show top 5 URLs
            .map(|(url, calls)| {
                let percentage = ((**calls as f64) / (total_calls as f64)) * 100.0;
                let status = if url.contains("mainnet-beta.solana.com") {
                    "🔴 FREE"
                } else if url.contains("premium") || url.contains("paid") {
                    "💎 PREMIUM"
                } else {
                    "🟡 CUSTOM"
                };

                // Truncate long URLs for display
                let display_url = if url.len() > 40 {
                    format!("{}...", &url[..37])
                } else {
                    url.to_string()
                };

                RpcUrlStatsDisplay {
                    rpc_url: display_url,
                    total_calls: format!("{}", calls),
                    percentage: format!("{:.1}%", percentage),
                    status: status.to_string(),
                }
            })
            .collect();

        output.push_str("\n🌐 RPC URL Usage (Top 5)\n");
        let mut url_table = Table::new(url_displays);
        url_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        output.push_str(&format!("{}\n", url_table));
    }

    // RPC Method Statistics (top 10)
    let mut method_stats: Vec<_> = rpc_stats.calls_per_method.iter().collect();
    method_stats.sort_by(|a, b| b.1.cmp(a.1)); // Sort by call count descending

    if !method_stats.is_empty() {
        let method_displays: Vec<_> = method_stats
            .iter()
            .take(10) // Show top 10 methods
            .map(|(method, calls)| {
                let percentage = ((**calls as f64) / (total_calls as f64)) * 100.0;
                let duration = Utc::now().signed_duration_since(rpc_stats.startup_time);
                let seconds = duration.num_seconds() as f64;
                let calls_per_second = if seconds > 0.0 { (**calls as f64) / seconds } else { 0.0 };

                RpcMethodStatsDisplay {
                    method_name: method.to_string(),
                    total_calls: format!("{}", calls),
                    percentage: format!("{:.1}%", percentage),
                    calls_per_second: format!("{:.3}", calls_per_second),
                }
            })
            .collect();

        output.push_str("\n⚙️ RPC Method Usage (Top 10)\n");
        let mut method_table = Table::new(method_displays);
        method_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        output.push_str(&format!("{}\n", method_table));
    }

    output
}


/// Calculate maximum drawdown percentage
fn calculate_max_drawdown(pnl_values: &[f64]) -> f64 {
    if pnl_values.is_empty() {
        return 0.0;
    }

    let mut peak = pnl_values[0];
    let mut max_drawdown = 0.0;

    for &value in pnl_values.iter().skip(1) {
        if value > peak {
            peak = value;
        }
        let drawdown = (peak - value) / peak.abs().max(1.0) * 100.0;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    max_drawdown
}

impl ClosedPositionDisplay {
    pub fn from_position(position: &Position) -> Self {
        // Check if position is fully verified (both entry and exit must be verified for closed positions)
        let is_verified = position.transaction_entry_verified && position.transaction_exit_verified;
        
        // Calculate total fees for the position including profit buffer for display
        let total_fees = calculate_position_total_fees(position) + PROFIT_EXTRA_NEEDED_SOL;
        
        if !is_verified {
            // For unverified positions, hide sensitive data
            let duration = if let Some(exit_time) = position.exit_time {
                format_duration_compact(position.entry_time, exit_time)
            } else {
                format_duration_compact(position.entry_time, Utc::now())
            };

            return Self {
                symbol: position.symbol.clone(),
                mint: position.mint.clone(),
                entry_price: "UNVERIFIED".to_string(),
                exit_price: "UNVERIFIED".to_string(),
                size_sol: format!("{:.6}", position.entry_size_sol),
                pnl_sol: "UNVERIFIED".to_string(),
                pnl_percent: "UNVERIFIED".to_string(),
                fees_sol: format!("{:.6}", total_fees),
                duration,
                status: "🔍 UNVERIFIED".to_string(),
            };
        }

        // For verified positions, show full details
        let exit_price = position.effective_exit_price.unwrap_or(
            position.exit_price.unwrap_or(0.0)
        );

        let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);

        let pnl_sol_str = if pnl_sol >= 0.0 {
            format!("+{:.6}", pnl_sol)
        } else {
            format!("{:.6}", pnl_sol)
        };

        let pnl_percent_str = if pnl_percent >= 0.0 {
            format!("+{:.2}%", pnl_percent)
        } else {
            format!("{:.2}%", pnl_percent)
        };

        let duration = if let Some(exit_time) = position.exit_time {
            format_duration_compact(position.entry_time, exit_time)
        } else {
            format_duration_compact(position.entry_time, Utc::now())
        };

        let status = get_profit_status_emoji(pnl_sol, pnl_percent, true);

        Self {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.11}", effective_price)
            } else {
                format!("{:.11}", position.entry_price)
            },
            exit_price: format!("{:.11}", exit_price),
            size_sol: format!("{:.6}", position.entry_size_sol),
            pnl_sol: pnl_sol_str,
            pnl_percent: pnl_percent_str,
            fees_sol: format!("{:.6}", total_fees),
            duration,
            status,
        }
    }
}

impl OpenPositionDisplay {
    pub fn from_position(position: &Position, current_price: Option<f64>) -> Self {
        // Check if position entry is verified (for open positions, only entry needs to be verified)
        let is_verified = position.transaction_entry_verified;
        
        // Calculate total fees for the position including profit buffer for display (for open positions, only entry fees + manual adjustment)
        let total_fees = calculate_position_total_fees(position) + PROFIT_EXTRA_NEEDED_SOL;
        
        let duration = format_duration_compact(position.entry_time, Utc::now());

        if !is_verified {
            // For unverified positions, hide sensitive data
            let current_price_str = if current_price.is_some() {
                "UNVERIFIED".to_string()
            } else {
                "N/A".to_string()
            };

            return Self {
                symbol: position.symbol.clone(),
                mint: position.mint.clone(),
                entry_price: "UNVERIFIED".to_string(),
                current_price: current_price_str,
                size_sol: format!("{:.6}", position.entry_size_sol),
                pnl_sol: "UNVERIFIED".to_string(),
                pnl_percent: "UNVERIFIED".to_string(),
                fees_sol: format!("{:.6}", total_fees),
                duration,
                status: "🔍 UNVERIFIED".to_string(),
            };
        }

        // For verified positions, show full details
        let current_price_str = if let Some(price) = current_price {
            format!("{:.11}", price)
        } else {
            "N/A".to_string()
        };

        let (pnl_sol_str, pnl_percent_str) = if let Some(price) = current_price {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(price));
            let sol_str = if pnl_sol >= 0.0 {
                format!("+{:.6}", pnl_sol)
            } else {
                format!("{:.6}", pnl_sol)
            };
            let percent_str = if pnl_percent >= 0.0 {
                format!("+{:.2}%", pnl_percent)
            } else {
                format!("{:.2}%", pnl_percent)
            };
            (sol_str, percent_str)
        } else {
            ("N/A".to_string(), "N/A".to_string())
        };

        let status = if let Some(price) = current_price {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(price));
            get_profit_status_emoji(pnl_sol, pnl_percent, false)
        } else {
            "OPEN".to_string()
        };

        Self {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.11}", effective_price)
            } else {
                format!("{:.11}", position.entry_price)
            },
            current_price: current_price_str,
            size_sol: format!("{:.6}", position.entry_size_sol),
            pnl_sol: pnl_sol_str,
            pnl_percent: pnl_percent_str,
            fees_sol: format!("{:.6}", total_fees),
            duration,
            status,
        }
    }
}

impl RecentSwapDisplay {
    pub fn from_swap_pnl_info(swap: &SwapPnLInfo) -> Self {
        // Helper function to shorten signatures like in the transactions module
        let shorten_signature = |signature: &str| -> String {
            if signature.len() <= 16 {
                signature.to_string()
            } else {
                format!("{}...{}", &signature[..8], &signature[signature.len()-4..])
            }
        };

        let shortened_signature = shorten_signature(&swap.signature);

        // Apply intuitive sign conventions for display
        let (display_sol_amount, display_token_amount) = if swap.swap_type == "Buy" {
            // Buy: SOL spent (negative), tokens received (positive)
            (-swap.sol_amount, swap.token_amount.abs())
        } else {
            // Sell: SOL received (positive), tokens sold (negative)  
            (swap.sol_amount, -swap.token_amount.abs())
        };

        // Color coding for better readability
        let type_display = if swap.swap_type == "Buy" {
            "🟢 Buy".to_string()  // Green for buy
        } else {
            "🔴 Sell".to_string() // Red for sell
        };

        // Format SOL amount with sign
        let sol_formatted = if display_sol_amount >= 0.0 {
            format!("+{:.6}", display_sol_amount)
        } else {
            format!("{:.6}", display_sol_amount)
        };

        // Format token amount with sign
        let token_formatted = if display_token_amount >= 0.0 {
            format!("+{:.2}", display_token_amount)
        } else {
            format!("{:.2}", display_token_amount)
        };

        Self {
            date: swap.timestamp.format("%m-%d").to_string(),
            time: swap.timestamp.format("%H:%M").to_string(),
            ago: format!("{} ago", format_duration_compact(swap.timestamp, Utc::now())),
            signature: shortened_signature,
            swap_type: type_display,
            token: swap.token_symbol[..15.min(swap.token_symbol.len())].to_string(),
            sol_amount: sol_formatted,
            token_amount: token_formatted,
            price: format!("{:.9}", swap.calculated_price_sol),
            router: swap.router[..12.min(swap.router.len())].to_string(),
            fee: format!("{:.6}", swap.fee_sol),
            status: swap.status.clone(),
        }
    }
}

impl RecentTransactionDisplay {
    pub fn from_transaction(tx: crate::transactions::Transaction) -> Self {
        // Shorten signature
        let signature = if tx.signature.len() <= 16 {
            tx.signature.clone()
        } else {
            format!("{}...{}", &tx.signature[..8], &tx.signature[tx.signature.len()-4..])
        };

        // Type string
        let tx_type = match &tx.transaction_type {
            crate::transactions::TransactionType::SwapSolToToken { .. } => "Buy".to_string(),
            crate::transactions::TransactionType::SwapTokenToSol { .. } => "Sell".to_string(),
            crate::transactions::TransactionType::SwapTokenToToken { .. } => "Token→Token".to_string(),
            crate::transactions::TransactionType::SolTransfer { .. } => "SOL Transfer".to_string(),
            crate::transactions::TransactionType::TokenTransfer { .. } => "Token Transfer".to_string(),
            crate::transactions::TransactionType::AtaClose { .. } => "ATA Close".to_string(),
            crate::transactions::TransactionType::Other { .. } => "Other".to_string(),
            crate::transactions::TransactionType::Unknown => "Unknown".to_string(),
        };

        // Token symbol if present
        let token = if let Some(info) = &tx.token_info {
            info.symbol.clone()
        } else {
            "-".to_string()
        };

        // SOL delta with sign
        let sol_delta = if tx.sol_balance_change >= 0.0 {
            format!("+{:.6}", tx.sol_balance_change)
        } else {
            format!("{:.6}", tx.sol_balance_change)
        };

        // Status
        let status = if tx.success { "✅ Success".to_string() } else { "❌ Failed".to_string() };

        // Ago
        let ago = format!("{} ago", format_duration_compact(tx.timestamp, Utc::now()));

        Self {
            date: tx.timestamp.format("%m-%d").to_string(),
            time: tx.timestamp.format("%H:%M").to_string(),
            ago,
            signature,
            slot: tx.slot.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string()),
            tx_type,
            token,
            sol_delta,
            status,
        }
    }
}

/// Generate profit-based status for positions
fn get_profit_status_emoji(_pnl_sol: f64, pnl_percent: f64, is_closed: bool) -> String {
    let base_status = if is_closed { "CLOSED" } else { "OPEN" };

    if pnl_percent >= 15.0 {
        format!("🚀 {}", base_status) // Rocket gains (15%+)
    } else if pnl_percent >= 0.0 {
        format!("✅ {}", base_status) // Positive gains (0-15%)
    } else if pnl_percent >= -10.0 {
        format!("⚠️ {}", base_status) // Small loss (0 to -10%)
    } else if pnl_percent >= -50.0 {
        format!("❌ {}", base_status) // Negative loss (-10 to -50%)
    } else {
        format!("💀 {}", base_status) // Very loss (-50%+)
    }
}
