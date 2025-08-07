/// ScreenerBot Trading Engine
///
/// ======================================
/// STOP LOSS PROTECTION SYSTEM
/// ======================================
///
/// This trading bot implements a stop loss at -55% with multiple layers of protection:
///
/// 1. **Stop Loss Threshold**: Set at -55% for controlled risk management
/// 2. **Profit-First Exit Logic**: All sell decisions prioritize profit taking
/// 3. **Smart System Validation**: Advanced profit system enforces stop loss rule
/// 4. **Final Safeguard**: Last-minute validation before any sell execution
/// 5. **Controlled Loss Strategy**: Positions at loss below -55% are sold to limit damage
///
/// This ensures controlled risk management while allowing reasonable loss limits
/// for positions that don't recover within acceptable timeframes.
///
/// MULTI-STRATEGY DIP DETECTION SYSTEM
/// ===================================
///
/// This system implements 5 sophisticated strategies to detect profitable dip entry points:
///
/// 1. **Immediate Drop Detection** (-5% to -30%)
///    - Detects sudden price drops in the immediate timeframe
///    - Adapts thresholds based on token's historical volatility
///    - Higher urgency for larger drops within the range
///
/// 2. **Moving Average Deviation** (-8% to -25%)
///    - Analyzes deviation from 5, 10, and 20-period moving averages
///    - Longer period deviations get higher weight
///    - Identifies oversold conditions relative to recent trends
///
/// 3. **Support Level Bounce** (-10% to -30%)
///    - Detects when price approaches historical support levels
///    - Uses technical analysis to find key price levels
///    - Bonus urgency for proximity to strong support
///
/// 4. **Volume-Weighted Price Dip** (-7% to -20%)
///    - Combines price drops with volume analysis
///    - Higher volume during dips indicates buying interest
///    - Validates dips with market participation data
///
/// 5. **Multi-Timeframe Convergence** (-6% to -18%)
///    - Requires multiple timeframes showing dip signals
///    - Short, medium, and long-term trend analysis
///    - Higher confidence when timeframes converge
///
/// The system calculates urgency scores from 0.0 to 2.0, with multiple strategies
/// providing consensus-based scoring and increased confidence.

// =============================================================================
// TRADING SYSTEM CONFIGURATION CONSTANTS
// =============================================================================

// -----------------------------------------------------------------------------
// Core Trading Parameters
// -----------------------------------------------------------------------------

/// Maximum number of concurrent open positions
pub const MAX_OPEN_POSITIONS: usize = 1;

/// Trade size in SOL for each position
pub const TRADE_SIZE_SOL: f64 = 0.001;

/// Default transaction fee for buy/sell operations
pub const TRANSACTION_FEE_SOL: f64 = 0.000015;

/// Default swap fee (set to 0 for GMGN routing)
pub const SWAP_FEE_PERCENT: f64 = 0.0;

/// Default slippage tolerance for swaps
pub const SLIPPAGE_TOLERANCE_PERCENT: f64 = 5.0;

// -----------------------------------------------------------------------------
// Position Timing Configuration - Improved for longer holding
// -----------------------------------------------------------------------------

/// Minimum hold time before considering sell (seconds) - reduced for flexibility
pub const MIN_POSITION_HOLD_TIME_SECS: f64 = 30.0;

/// Maximum hold time extended for longer-term profit taking (1 hour)
pub const MAX_POSITION_HOLD_TIME_SECS: f64 = 1.0 * 60.0 * 60.0; // 1 hour (3600 seconds)

/// Time after which time decay pressure starts - now 2 hours for better patience
pub const TIME_DECAY_START_SECS: f64 = 7200.0; // 2 hours

// -----------------------------------------------------------------------------
// Trading Logic Configuration
// -----------------------------------------------------------------------------
// Monitoring & Display Configuration
// -----------------------------------------------------------------------------

/// Summary display refresh interval (seconds)
pub const SUMMARY_DISPLAY_INTERVAL_SECS: u64 = 5;

/// New entry signals check interval (seconds)
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 5;

/// Open positions monitoring interval (seconds)
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

/// Price history tracking duration (hours)
pub const PRICE_HISTORY_DURATION_HOURS: i64 = 2;

// -----------------------------------------------------------------------------
// Task Timeout Configuration
// -----------------------------------------------------------------------------

/// Semaphore acquire timeout for token processing tasks (seconds)
pub const SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 120;

/// Individual token check task timeout (seconds)
pub const TOKEN_CHECK_TASK_TIMEOUT_SECS: u64 = 60;

/// Price cache lock acquire timeout (milliseconds)
pub const PRICE_CACHE_LOCK_TIMEOUT_MS: u64 = 2000;

/// Task collection timeout for concurrent operations (seconds)
pub const TASK_COLLECTION_TIMEOUT_SECS: u64 = 120;

/// Token check result collection timeout (seconds)
pub const TOKEN_CHECK_COLLECTION_TIMEOUT_SECS: u64 = 120;

/// Individual token check handle timeout (seconds)
pub const TOKEN_CHECK_HANDLE_TIMEOUT_SECS: u64 = 120;

/// Buy operations collection timeout (seconds)
pub const BUY_OPERATIONS_COLLECTION_TIMEOUT_SECS: u64 = 120;

/// Individual buy operation timeout (seconds) - extended for smart timeout handling
pub const BUY_OPERATION_SMART_TIMEOUT_SECS: u64 = 600; // 10 minutes total allowance for complex operations

/// Sell operations collection timeout (seconds) - must accommodate multiple 3-min operations
pub const SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS: u64 = 240;

/// Individual sell operation timeout (seconds) - removed for smart timeout handling
/// Now using step-based timeout detection instead of total operation timeout
pub const SELL_OPERATION_SMART_TIMEOUT_SECS: u64 = 600; // 10 minutes total allowance for complex operations

/// Sell semaphore acquire timeout (seconds) - increased for safety
pub const SELL_SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 30;

/// Buy semaphore acquire timeout (seconds) - increased for safety
pub const BUY_SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 180;

/// Individual sell task handle timeout (seconds) - must be longer than operation timeout
pub const SELL_TASK_HANDLE_TIMEOUT_SECS: u64 = 200;

/// Entry monitor cycle timeout warning threshold (seconds)
pub const ENTRY_CYCLE_TIMEOUT_WARNING_SECS: u64 = 5;

/// Entry monitor cycle minimum wait time (milliseconds)
pub const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;

/// Token processing shutdown check delay (milliseconds)
pub const TOKEN_PROCESSING_SHUTDOWN_CHECK_MS: u64 = 10;

/// Task shutdown check delay (milliseconds)
pub const TASK_SHUTDOWN_CHECK_MS: u64 = 1;

/// Buy operation shutdown check delay (milliseconds)
pub const BUY_OPERATION_SHUTDOWN_CHECK_MS: u64 = 1;

/// Sell operation shutdown check delay (milliseconds)
pub const SELL_OPERATION_SHUTDOWN_CHECK_MS: u64 = 1;

/// Collection shutdown check delay (milliseconds)
pub const COLLECTION_SHUTDOWN_CHECK_MS: u64 = 1;

// -----------------------------------------------------------------------------
// Wallet Management Configuration
// -----------------------------------------------------------------------------

/// Automatically close Associated Token Accounts after selling tokens
pub const AUTO_CLOSE_ATA_AFTER_SELL: bool = true;

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_trader_enabled;
use crate::tokens::{
    Token,
    update_open_positions_safe,
    get_all_tokens_by_liquidity,
    discover_tokens_once,
    monitor_tokens_once,
    get_token_price_blocking_safe,
    sync_watch_list_with_trader,
    pool::{ get_pool_service, get_price_history_for_rl_learning },
};
use crate::positions::{
    Position,
    calculate_position_pnl,
    update_position_tracking,
    get_open_positions_count,
    open_position,
    close_position,
    SAVED_POSITIONS,
};
use crate::utils::*;

use crate::filtering::{ should_buy_token, log_filtering_summary };
use crate::tokens::get_token_rugcheck_data_safe;
use crate::tokens::rugcheck::{ is_token_safe_for_trading, get_high_risk_issues };
use crate::entry::{ should_buy };

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use once_cell::sync::Lazy;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use colored::Colorize;

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: tracks critical trading operations in progress to prevent force shutdown
pub static CRITICAL_OPERATIONS_IN_PROGRESS: Lazy<Arc<std::sync::atomic::AtomicUsize>> = Lazy::new(||
    Arc::new(std::sync::atomic::AtomicUsize::new(0))
);

// =============================================================================
// CRITICAL OPERATION PROTECTION
// =============================================================================

/// RAII guard that increments critical operations counter on creation and decrements on drop
/// Prevents shutdown while critical trading operations (buy/sell) are in progress
pub struct CriticalOperationGuard {
    _phantom: std::marker::PhantomData<()>,
}

impl CriticalOperationGuard {
    /// Create a new critical operation guard
    /// This should be created before any buy/sell operation
    pub fn new(operation_name: &str) -> Self {
        let count = CRITICAL_OPERATIONS_IN_PROGRESS.fetch_add(
            1,
            std::sync::atomic::Ordering::SeqCst
        );
        log(
            LogTag::Trader,
            "CRITICAL_OP_START",
            &format!(
                "🔒 PROTECTED: {} operation started (active operations: {})",
                operation_name,
                count + 1
            )
        );

        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the current number of critical operations in progress
    pub fn get_active_count() -> usize {
        CRITICAL_OPERATIONS_IN_PROGRESS.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for CriticalOperationGuard {
    fn drop(&mut self) {
        let count = CRITICAL_OPERATIONS_IN_PROGRESS.fetch_sub(
            1,
            std::sync::atomic::Ordering::SeqCst
        );
        log(
            LogTag::Trader,
            "CRITICAL_OP_END",
            &format!(
                "🔓 UNPROTECTED: Critical operation finished (remaining operations: {})",
                count - 1
            )
        );
    }
}

// =============================================================================
// MEMORY MANAGEMENT AND CLEANUP
// =============================================================================

// =============================================================================
// DEBUG LOGGING CONFIGURATION
// =============================================================================

/// Helper function for conditional debug trader logs
pub fn debug_trader_log(log_type: &str, message: &str) {
    if is_debug_trader_enabled() {
        log(LogTag::Trader, log_type, message);
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR TOKENS MODULE INTEGRATION
// =============================================================================

/// Get tokens using the safe tokens module system
pub async fn get_tokens_from_safe_system() -> Vec<Token> {
    match get_all_tokens_by_liquidity().await {
        Ok(api_tokens) => {
            // Convert ApiToken to Token for compatibility with existing code
            api_tokens
                .into_iter()
                .map(|api_token| api_token.into())
                .collect()
        }
        Err(e) => {
            log(LogTag::Trader, "WARN", &format!("Failed to get tokens from safe system: {}", e));
            Vec::new()
        }
    }
}

/// Update open positions tracking in price service
async fn update_position_tracking_in_service() {
    let open_mints = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .filter(|pos| pos.exit_price.is_none())
                .map(|pos| pos.mint.clone())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        }
    };

    if !open_mints.is_empty() {
        update_open_positions_safe(open_mints).await;
        log(LogTag::Trader, "TRACK", "Updated open positions in price service");
    }
}

/// Try to populate tokens database with discovery data if it's empty
async fn ensure_tokens_populated() {
    // Check if we have tokens in the database
    match get_all_tokens_by_liquidity().await {
        Ok(tokens) if tokens.is_empty() => {
            log(LogTag::Trader, "INFO", "Token database is empty, running discovery...");

            // Run manual discovery to populate the database
            if let Err(e) = discover_tokens_once().await {
                log(LogTag::Trader, "WARN", &format!("Failed to run token discovery: {}", e));
            }

            // Run manual monitoring to update prices
            if let Err(e) = monitor_tokens_once().await {
                log(LogTag::Trader, "WARN", &format!("Failed to run token monitoring: {}", e));
            }
        }
        Ok(tokens) => {
            log(LogTag::Trader, "INFO", &format!("Token database has {} tokens", tokens.len()));
        }
        Err(e) => {
            log(LogTag::Trader, "WARN", &format!("Failed to check token database: {}", e));
        }
    }
}

/// Simple Buy Signal Detection System
/// Returns urgency score from 0.0 (don't buy) to 2.0 (buy immediately)
pub async fn should_buy_enhanced(token: &Token, current_price: f64, prev_price: f64) -> f64 {
    use crate::global::is_debug_trader_enabled;

    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "BUY_ANALYSIS_START",
            &format!(
                "🔬 Buy Analysis for {}: Current={:.10} SOL | Previous={:.10} SOL",
                token.symbol.as_str(),
                current_price,
                prev_price
            )
        );
    }

    // Emergency buy logic - large price drops
    let price_change_percent = ((current_price - prev_price) / prev_price) * 100.0;
    let price_drop_percent = -price_change_percent;

    // Emergency thresholds for massive dips
    let emergency_dip_threshold = match
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    {
        liq if liq >= 100_000.0 => 5.0, // High liquidity: 5%+ = emergency
        liq if liq >= 50_000.0 => 8.0, // Medium liquidity: 8%+ = emergency
        liq if liq >= 10_000.0 => 12.0, // Low liquidity: 12%+ = emergency
        _ => 15.0, // Very low: 15%+ = emergency
    };

    // Emergency buy - skip all other checks for major dips
    if price_drop_percent >= emergency_dip_threshold {
        if current_price > 0.0 && current_price.is_finite() && prev_price > 0.0 {
            let emergency_urgency = (price_drop_percent / emergency_dip_threshold).min(2.0);

            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "EMERGENCY_BUY_SIGNAL",
                    &format!(
                        "🚨 EMERGENCY BUY for {}: {:.2}% drop >= {:.1}% threshold | Urgency={:.2}",
                        token.symbol.as_str(),
                        price_drop_percent,
                        emergency_dip_threshold,
                        emergency_urgency
                    )
                );
            }
            return emergency_urgency;
        }
    }

    // Use centralized filtering system
    if !should_buy_token(token).await {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "BUY_FILTER_REJECT",
                &format!("❌ {} rejected by filtering system", token.symbol.as_str())
            );
        }
        return 0.0;
    }

    // Simple entry analysis
    let is_safe_for_entry = should_buy(token).await;

    if !is_safe_for_entry {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected by entry analysis", token.symbol)
            );
        }
        return 0.0;
    }

    // Simple urgency calculation based on price drop
    let base_urgency = if price_drop_percent >= 10.0 {
        1.5 // High urgency for big drops
    } else if price_drop_percent >= 5.0 {
        1.2 // Medium urgency for moderate drops
    } else if price_drop_percent >= 2.0 {
        1.0 // Low urgency for small drops
    } else {
        0.8 // Very low urgency for tiny/no drops
    };

    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "BUY_DECISION",
            &format!(
                "✅ {} BUY APPROVED: Drop={:.2}% | Urgency={:.2}",
                token.symbol.as_str(),
                price_drop_percent,
                base_urgency
            )
        );
    }

    base_urgency
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    log(LogTag::Trader, "STARTUP", "🚀 Starting monitor_new_entries task");

    'outer: loop {
        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        // Update position tracking in price service
        let position_update_start = std::time::Instant::now();
        update_position_tracking_in_service().await;
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "✅ Position tracking updated in {:.1}ms",
                    position_update_start.elapsed().as_millis()
                )
            );
        }

        // Ensure we have tokens to work with
        if is_debug_trader_enabled() {
            log(LogTag::Trader, "DEBUG", "🪙 Ensuring tokens are populated...");
        }
        let token_populate_start = std::time::Instant::now();
        ensure_tokens_populated().await;
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "✅ Tokens populated in {:.1}ms",
                    token_populate_start.elapsed().as_millis()
                )
            );
        }

        let mut tokens: Vec<_> = {
            // Get tokens from safe system
            if is_debug_trader_enabled() {
                log(LogTag::Trader, "DEBUG", "📡 Getting tokens from safe system...");
            }
            let token_fetch_start = std::time::Instant::now();
            let tokens_from_module = get_tokens_from_safe_system().await;
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "✅ Got {} tokens from safe system in {:.1}ms",
                        tokens_from_module.len(),
                        token_fetch_start.elapsed().as_millis()
                    )
                );
            }

            // Log total tokens available
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Total tokens from safe system: {}", tokens_from_module.len())
                        .dimmed()
                        .to_string()
                );
            }

            // Add debug info about token prices and update times
            if is_debug_trader_enabled() {
                log(LogTag::Trader, "DEBUG", "🏷️ Checking price service for sample tokens...");
            }
            let price_check_start = std::time::Instant::now();
            debug_trader_log(
                "TOKEN_PRICE_DEBUG",
                &format!(
                    "DEBUG: First 3 tokens price info - {} tokens total",
                    tokens_from_module.len()
                )
            );

            for (i, token) in tokens_from_module.iter().take(3).enumerate() {
                let price_lookup_start = std::time::Instant::now();
                let price_from_service = get_token_price_blocking_safe(&token.mint).await;
                debug_trader_log(
                    "TOKEN_PRICE_SAMPLE",
                    &format!(
                        "Token {}: {} ({}) - price_service={:?} (lookup took {:.1}ms)",
                        i + 1,
                        token.symbol,
                        token.mint,
                        price_from_service,
                        price_lookup_start.elapsed().as_millis()
                    )
                );
            }
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "✅ Price service check completed in {:.1}ms",
                        price_check_start.elapsed().as_millis()
                    )
                );
            } // Include all tokens - we want to trade on existing tokens with updated info
            // The discovery system ensures tokens are updated with fresh data before trading
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "Using all {} tokens for trading (startup filter removed)",
                        tokens_from_module.len()
                    )
                        .dimmed()
                        .to_string()
                );
            }

            // Count tokens with liquidity data
            let with_liquidity = tokens_from_module
                .iter()
                .filter(|token| {
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
                })
                .count();

            if with_liquidity > 0 {
                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Processing {} tokens with liquidity", with_liquidity)
                );
            }

            tokens_from_module
        };

        // Sort tokens by liquidity in descending order (highest liquidity first)
        tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Safety check - if processing is taking too long, log it
        if cycle_start.elapsed() > Duration::from_secs(ENTRY_CYCLE_TIMEOUT_WARNING_SECS) {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token sorting took too long: {:?}", cycle_start.elapsed())
            );
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!(
                "Checking {} tokens for entry opportunities (sorted by liquidity)",
                tokens.len()
            )
                .dimmed()
                .to_string()
        );

        // Use centralized filtering system to get eligible tokens
        use crate::filtering::{ filter_tokens_with_reasons, get_filtering_stats };

        let (eligible_tokens, rejected_tokens) = filter_tokens_with_reasons(&tokens).await;

        // Log filtering statistics
        let (total, passed, pass_rate) = get_filtering_stats(&tokens).await;
        log(
            LogTag::Trader,
            "FILTER_STATS",
            &format!("Token filtering: {}/{} passed ({:.1}% pass rate)", passed, total, pass_rate)
        );

        // Use eligible tokens for trading
        tokens = eligible_tokens;

        // Early return if no tokens to process
        if tokens.is_empty() {
            log(LogTag::Trader, "INFO", "No tokens to process, skipping token checking cycle");

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
            } else {
                Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break;
            }
            continue;
        }

        log(
            LogTag::Trader,
            "DEBUG",
            &format!("🔍 Starting to process {} eligible tokens", tokens.len())
        );

        // Use a semaphore to limit the number of concurrent token checks
        // This balances between parallelism and not overwhelming external APIs
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(5)); // Reduced to 5 concurrent checks to avoid overwhelming

        // Log filtering summary
        log_filtering_summary(&tokens).await;

        // Sync OHLCV watch list with trader tokens (run async to not block trading)
        if is_debug_trader_enabled() {
            log(LogTag::Trader, "DEBUG", "📈 Syncing OHLCV watch list with filtered tokens...");
        }
        let ohlcv_sync_start = std::time::Instant::now();
        tokio::spawn(async move {
            if let Err(e) = sync_watch_list_with_trader().await {
                log(LogTag::Trader, "WARN", &format!("OHLCV sync failed: {}", e));
            }
        });
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "✅ OHLCV sync task spawned in {:.1}ms",
                    ohlcv_sync_start.elapsed().as_millis()
                )
            );
        }

        // Process all tokens in parallel with concurrent tasks
        let mut handles = Vec::new();

        // Get the total token count before starting the loop
        let total_tokens = tokens.len();
        let token_processing_start = std::time::Instant::now();
        // Note: tokens are still sorted by liquidity from highest to lowest
        for (index, token) in tokens.iter().enumerate() {
            // Check for shutdown before spawning tasks
            if
                check_shutdown_or_delay(
                    &shutdown,
                    Duration::from_millis(TOKEN_PROCESSING_SHUTDOWN_CHECK_MS)
                ).await
            {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break 'outer;
            }

            // Get permit from semaphore to limit concurrency with timeout
            let permit = match
                tokio::time::timeout(
                    Duration::from_secs(SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                    semaphore.clone().acquire_owned()
                ).await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(e)) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to acquire semaphore permit: {}", e)
                    );
                    continue;
                }
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!("Semaphore acquire timed out after {} seconds", SEMAPHORE_ACQUIRE_TIMEOUT_SECS)
                    );
                    continue;
                }
            };

            // Clone necessary variables for the task
            let token = token.clone();
            let shutdown_clone = shutdown.clone();

            // Spawn a new task for this token with overall timeout
            let handle = tokio::spawn(async move {
                // Keep the permit alive for the duration of this task
                let _permit = permit; // This will be automatically dropped when the task completes

                // Check for shutdown before starting task
                if
                    check_shutdown_or_delay(
                        &shutdown_clone,
                        Duration::from_millis(TASK_SHUTDOWN_CHECK_MS)
                    ).await
                {
                    return None;
                }

                // Clone the symbol for error logging before moving token into timeout
                let token_symbol = token.symbol.clone();

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(TOKEN_CHECK_TASK_TIMEOUT_SECS), async {
                        let task_start = std::time::Instant::now();

                        // Use centralized price service instead of direct token field access
                        debug_trader_log(
                            "TASK_TIMING",
                            &format!("Token {} starting price fetch", token.symbol)
                        );
                        let price_start = std::time::Instant::now();

                        if
                            let Some(current_price) = get_token_price_blocking_safe(
                                &token.mint
                            ).await
                        {
                            debug_trader_log(
                                "TASK_TIMING",
                                &format!(
                                    "Token {} price fetch took {:.2}ms",
                                    token.symbol,
                                    price_start.elapsed().as_millis()
                                )
                            );
                            // CRITICAL: Validate price is actually loaded and valid before any trading
                            if current_price <= 0.0 || !current_price.is_finite() {
                                debug_trader_log(
                                    "PRICE_INVALID",
                                    &format!(
                                        "Token {} ({}): INVALID PRICE DETECTED - not trading. Price = {:.10}",
                                        token.symbol,
                                        token.mint,
                                        current_price
                                    )
                                );
                                return None;
                            }

                            debug_trader_log(
                                "PRICE_SOURCE",
                                &format!(
                                    "Token {} ({}): current price from price service = {:.10}",
                                    token.symbol,
                                    token.mint,
                                    current_price
                                )
                            );

                            // Use centralized filtering system
                            debug_trader_log(
                                "TASK_TIMING",
                                &format!("Token {} starting filtering", token.symbol)
                            );
                            let filter_start = std::time::Instant::now();

                            if !should_buy_token(&token).await {
                                // Token was filtered out, skip processing
                                return None;
                            }

                            // Check for shutdown after filtering
                            if
                                check_shutdown_or_delay(
                                    &shutdown_clone,
                                    Duration::from_millis(TASK_SHUTDOWN_CHECK_MS)
                                ).await
                            {
                                return None;
                            }

                            debug_trader_log(
                                "TASK_TIMING",
                                &format!(
                                    "Token {} filtering took {:.2}ms",
                                    token.symbol,
                                    filter_start.elapsed().as_millis()
                                )
                            );

                            let liquidity_usd = token.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);

                            // Price history is now handled by the pool service automatically
                            // No manual tracking needed here

                            // Check for shutdown after price history update
                            if
                                check_shutdown_or_delay(
                                    &shutdown_clone,
                                    Duration::from_millis(TASK_SHUTDOWN_CHECK_MS)
                                ).await
                            {
                                return None;
                            }

                            // Check for entry opportunity using should_buy function
                            let mut buy_urgency = 0.0;

                            // Get previous price from pool service price history
                            let prev_price = {
                                let pool_service = get_pool_service();
                                let price_history = pool_service.get_recent_price_history(
                                    &token.mint
                                ).await;
                                if price_history.len() >= 2 {
                                    // Get the second-to-last price (previous price)
                                    Some(price_history[price_history.len() - 2].1)
                                } else {
                                    None
                                }
                            };

                            if let Some(prev_price) = prev_price {
                                if prev_price > 0.0 {
                                    debug_trader_log(
                                        "TOKEN_CHECK_PRICES",
                                        &format!(
                                            "Token {} ({}): checking buy signal with current={:.10}, prev={:.10}",
                                            token.symbol,
                                            token.mint,
                                            current_price,
                                            prev_price
                                        )
                                    );

                                    // Check if prices are identical (indicating no price update)
                                    if (current_price - prev_price).abs() < 0.000000000001 {
                                        debug_trader_log(
                                            "PRICE_UPDATE_ISSUE",
                                            &format!(
                                                "Token {} ({}): IDENTICAL PRICES detected! Current={:.10} == Prev={:.10} - No price movement detected, this indicates price cache not updating",
                                                token.symbol,
                                                token.mint,
                                                current_price,
                                                prev_price
                                            )
                                        );
                                    }

                                    // Rugcheck safety validation - block rugged or high-risk tokens
                                    debug_trader_log(
                                        "TASK_TIMING",
                                        &format!("Token {} starting rugcheck", token.symbol)
                                    );
                                    let rugcheck_start = std::time::Instant::now();

                                    match get_token_rugcheck_data_safe(&token.mint).await {
                                        Ok(Some(rugcheck_data)) => {
                                            // Block tokens that are detected as rugged
                                            if rugcheck_data.rugged.unwrap_or(false) {
                                                debug_trader_log(
                                                    "RUGCHECK_REJECT_RUGGED",
                                                    &format!(
                                                        "Token {} ({}) rejected - detected as rugged",
                                                        token.symbol,
                                                        token.mint
                                                    )
                                                );
                                                buy_urgency = 0.0;
                                            } else {
                                                // Use the proper rugcheck safety validation
                                                let rugcheck_score = rugcheck_data.score_normalised
                                                    .or(rugcheck_data.score)
                                                    .unwrap_or(50);

                                                if !is_token_safe_for_trading(&rugcheck_data) {
                                                    let high_risk_issues = get_high_risk_issues(
                                                        &rugcheck_data
                                                    );
                                                    debug_trader_log(
                                                        "RUGCHECK_REJECT_UNSAFE",
                                                        &format!(
                                                            "Token {} ({}) rejected - failed safety check (risk score: {}, issues: {:?})",
                                                            token.symbol,
                                                            token.mint,
                                                            rugcheck_score,
                                                            high_risk_issues
                                                        )
                                                    );
                                                    buy_urgency = 0.0;
                                                } else if
                                                    // Block tokens with critical risks (additional check)
                                                    let Some(risks) = &rugcheck_data.risks
                                                {
                                                    let critical_risks = risks
                                                        .iter()
                                                        .filter(|r| {
                                                            r.level
                                                                .as_ref()
                                                                .map(
                                                                    |l|
                                                                        l.to_lowercase() ==
                                                                        "critical"
                                                                )
                                                                .unwrap_or(false)
                                                        })
                                                        .count();

                                                    if critical_risks > 0 {
                                                        debug_trader_log(
                                                            "RUGCHECK_REJECT_CRITICAL",
                                                            &format!(
                                                                "Token {} ({}) rejected - {} critical risks detected",
                                                                token.symbol,
                                                                token.mint,
                                                                critical_risks
                                                            )
                                                        );
                                                        buy_urgency = 0.0;
                                                    } else {
                                                        // Use the enhanced OHLCV-based should_buy function
                                                        buy_urgency = should_buy_enhanced(
                                                            &token,
                                                            current_price,
                                                            prev_price
                                                        ).await;

                                                        debug_trader_log(
                                                            "RUGCHECK_OK",
                                                            &format!(
                                                                "Token {} ({}) passed rugcheck validation (risk score: {})",
                                                                token.symbol,
                                                                token.mint,
                                                                rugcheck_score
                                                            )
                                                        );
                                                    }
                                                } else {
                                                    // Use the enhanced OHLCV-based should_buy function
                                                    buy_urgency = should_buy_enhanced(
                                                        &token,
                                                        current_price,
                                                        prev_price
                                                    ).await;

                                                    debug_trader_log(
                                                        "RUGCHECK_OK",
                                                        &format!(
                                                            "Token {} ({}) passed rugcheck validation (risk score: {})",
                                                            token.symbol,
                                                            token.mint,
                                                            rugcheck_score
                                                        )
                                                    );
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            debug_trader_log(
                                                "RUGCHECK_MISSING",
                                                &format!(
                                                    "Token {} ({}) has no rugcheck data - proceeding with caution",
                                                    token.symbol,
                                                    token.mint
                                                )
                                            );

                                            // Use the enhanced OHLCV-based should_buy function
                                            buy_urgency = should_buy_enhanced(
                                                &token,
                                                current_price,
                                                prev_price
                                            ).await;
                                        }
                                        Err(e) => {
                                            debug_trader_log(
                                                "RUGCHECK_ERROR",
                                                &format!(
                                                    "Token {} ({}) rugcheck error: {} - proceeding with caution",
                                                    token.symbol,
                                                    token.mint,
                                                    e
                                                )
                                            );

                                            // Use the enhanced OHLCV-based should_buy function
                                            buy_urgency = should_buy_enhanced(
                                                &token,
                                                current_price,
                                                prev_price
                                            ).await;
                                        }
                                    }

                                    debug_trader_log(
                                        "TASK_TIMING",
                                        &format!(
                                            "Token {} rugcheck took {:.2}ms",
                                            token.symbol,
                                            rugcheck_start.elapsed().as_millis()
                                        )
                                    );

                                    // Check for shutdown after rugcheck processing
                                    if
                                        check_shutdown_or_delay(
                                            &shutdown_clone,
                                            Duration::from_millis(TASK_SHUTDOWN_CHECK_MS)
                                        ).await
                                    {
                                        return None;
                                    }

                                    debug_trader_log(
                                        "TOKEN_CHECK_RESULT",
                                        &format!(
                                            "Token {} ({}): buy urgency result: {:.3}",
                                            token.symbol,
                                            token.mint,
                                            buy_urgency
                                        )
                                    );
                                } else {
                                    debug_trader_log(
                                        "TOKEN_CHECK_INVALID_PREV",
                                        &format!(
                                            "Token {} ({}): invalid prev_price: {:.10}",
                                            token.symbol,
                                            token.mint,
                                            prev_price
                                        )
                                    );
                                }
                            } else {
                                debug_trader_log(
                                    "TOKEN_CHECK_NO_PREV",
                                    &format!(
                                        "Token {} ({}): no previous price available",
                                        token.symbol,
                                        token.mint
                                    )
                                );
                            }

                            // Price tracking is now handled by the pool service automatically

                            // Return the token and price if buy signal detected
                            if buy_urgency > 0.0 {
                                use crate::global::is_debug_entry_enabled;

                                if is_debug_entry_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "BUY_DECISION_POSITIVE",
                                        &format!(
                                            "🚀 BUY DECISION: {} has urgency {:.3} > 0.0 → Will attempt to buy!",
                                            token.symbol,
                                            buy_urgency
                                        )
                                    );
                                }

                                // Check for shutdown before attempting to buy
                                if
                                    check_shutdown_or_delay(
                                        &shutdown_clone,
                                        Duration::from_millis(TASK_SHUTDOWN_CHECK_MS)
                                    ).await
                                {
                                    if is_debug_entry_enabled() {
                                        log(
                                            LogTag::Trader,
                                            "BUY_DECISION_SHUTDOWN",
                                            &format!(
                                                "❌ {} buy cancelled due to shutdown",
                                                token.symbol
                                            )
                                        );
                                    }
                                    return None;
                                }

                                // Calculate price change using pool service price history
                                let change = {
                                    let pool_service = get_pool_service();
                                    let price_history = pool_service.get_recent_price_history(
                                        &token.mint
                                    ).await;
                                    if price_history.len() >= 2 {
                                        let prev_price = price_history[price_history.len() - 2].1;
                                        if prev_price > 0.0 {
                                            ((current_price - prev_price) / prev_price) * 100.0
                                        } else {
                                            0.0
                                        }
                                    } else {
                                        0.0
                                    }
                                };

                                if is_debug_entry_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "BUY_DECISION_FINAL",
                                        &format!(
                                            "✅ {} PASSED ALL CHECKS → Returning for buy execution (price: {:.10}, change: {:.2}%)",
                                            token.symbol,
                                            current_price,
                                            change
                                        )
                                    );
                                }

                                return Some((token, current_price, change));
                            } else {
                                use crate::global::is_debug_entry_enabled;

                                if is_debug_entry_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "BUY_DECISION_NEGATIVE",
                                        &format!(
                                            "❌ {} has urgency {:.3} ≤ 0.0 → No buy signal generated",
                                            token.symbol,
                                            buy_urgency
                                        )
                                    );
                                }
                            }
                        } else {
                            // Price is not loaded - do not attempt any trading
                            debug_trader_log(
                                "PRICE_NOT_LOADED",
                                &format!(
                                    "Token {} ({}): PRICE NOT LOADED - skipping trading. Price service returned None",
                                    token.symbol,
                                    token.mint
                                )
                            );
                        }

                        debug_trader_log(
                            "TASK_TIMING",
                            &format!(
                                "Token {} task completed in {:.2}ms",
                                token.symbol,
                                task_start.elapsed().as_millis()
                            )
                        );

                        None
                    }).await
                {
                    Ok(result) => result,
                    Err(_) => {
                        // Task timed out
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "Token check task for {} timed out after {} seconds",
                                token_symbol,
                                TOKEN_CHECK_TASK_TIMEOUT_SECS
                            )
                        );
                        None
                    }
                }
            });

            handles.push(handle);
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Successfully spawned {} token checking tasks", handles.len())
                .dimmed()
                .to_string()
        );

        // Process the results of all tasks with overall timeout
        let collection_result = tokio::time::timeout(
            Duration::from_secs(TOKEN_CHECK_COLLECTION_TIMEOUT_SECS),
            async {
                // This maintains the priority of processing high-liquidity tokens first
                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Waiting for {} token checks to complete", handles.len())
                        .dimmed()
                        .to_string()
                );

                let mut opportunities = Vec::new();

                for handle in handles {
                    // Skip any tasks that failed or if shutdown signal received
                    if
                        check_shutdown_or_delay(
                            &shutdown,
                            Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS)
                        ).await
                    {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "new entries monitor shutting down during result collection..."
                        );
                        return opportunities; // Return what we have so far
                    }

                    // Add timeout for each handle to prevent getting stuck on a single task
                    match
                        tokio::time::timeout(
                            Duration::from_secs(TOKEN_CHECK_HANDLE_TIMEOUT_SECS),
                            handle
                        ).await
                    {
                        Ok(task_result) => {
                            match task_result {
                                Ok(Some((token, price, percent_change))) => {
                                    opportunities.push((token, price, percent_change));
                                }
                                Ok(None) => {
                                    // No opportunity found for this token, continue
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Trader,
                                        "ERROR",
                                        &format!("Token check task failed: {}", e)
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            // Task timed out after the specified timeout
                            log(
                                LogTag::Trader,
                                "WARN",
                                &format!("Token check task timed out after {} seconds", TOKEN_CHECK_TASK_TIMEOUT_SECS)
                            );
                        }
                    }
                }

                opportunities
            }
        ).await;

        let mut opportunities = match collection_result {
            Ok(opportunities) => opportunities,
            Err(_) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Token check collection timed out after {} seconds", TOKEN_CHECK_COLLECTION_TIMEOUT_SECS)
                );
                Vec::new() // Return empty if timeout
            }
        };

        // Sort opportunities by liquidity again to ensure priority
        opportunities.sort_by(|(a, _, _), (b, _, _)| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Enhanced debug logging for opportunities
        use crate::global::is_debug_entry_enabled;

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "OPPORTUNITIES_SUMMARY",
                &format!(
                    "🎯 Entry Opportunities Found: {} tokens passed all checks",
                    opportunities.len()
                )
            );

            if opportunities.is_empty() {
                log(
                    LogTag::Trader,
                    "OPPORTUNITIES_NONE",
                    "❌ No tokens generated buy signals this cycle"
                );
            } else {
                log(
                    LogTag::Trader,
                    "OPPORTUNITIES_LIST",
                    &format!("📋 Tokens ready for purchase:")
                );

                for (i, (token, price, change)) in opportunities.iter().enumerate() {
                    let liquidity = token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0);

                    log(
                        LogTag::Trader,
                        "OPPORTUNITY_DETAIL",
                        &format!(
                            "   {}. {} - Price: {:.10} SOL | Change: {:.2}% | Liquidity: ${:.0}",
                            i + 1,
                            token.symbol,
                            price,
                            change,
                            liquidity
                        )
                    );
                }
            }
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Found {} potential entry opportunities", opportunities.len())
        );

        // Log the total time taken for the token checking cycle
        log(
            LogTag::Trader,
            "INFO",
            &format!("Token checking cycle completed in {:?}", cycle_start.elapsed())
                .dimmed()
                .to_string()
        );

        // Process opportunities concurrently while respecting position limits
        if !opportunities.is_empty() {
            let current_open_count = get_open_positions_count();
            let available_slots = MAX_OPEN_POSITIONS.saturating_sub(current_open_count);

            if is_debug_entry_enabled() {
                log(
                    LogTag::Trader,
                    "POSITION_CHECK",
                    &format!(
                        "💼 Position Limits: Current Open: {} | Max: {} | Available Slots: {}",
                        current_open_count,
                        MAX_OPEN_POSITIONS,
                        available_slots
                    )
                );
            }

            if available_slots == 0 {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Trader,
                        "POSITION_LIMIT_REACHED",
                        &format!(
                            "🚫 POSITION LIMIT REACHED: Cannot buy any tokens - {} opportunities skipped",
                            opportunities.len()
                        )
                    );
                }

                log(
                    LogTag::Trader,
                    "LIMIT",
                    &format!(
                        "Maximum open positions already reached ({}/{}). Skipping all opportunities.",
                        current_open_count,
                        MAX_OPEN_POSITIONS
                    )
                );
            } else {
                // Capture the original count before consuming the vector
                let total_opportunities_count = opportunities.len();

                // Limit opportunities to available slots
                let opportunities_to_process = opportunities
                    .into_iter()
                    .take(available_slots)
                    .collect::<Vec<_>>();

                if is_debug_entry_enabled() {
                    log(
                        LogTag::Trader,
                        "PROCESSING_OPPORTUNITIES",
                        &format!(
                            "🚀 PROCESSING {} opportunities for purchase (limited by available slots)",
                            opportunities_to_process.len()
                        )
                    );

                    if opportunities_to_process.len() < total_opportunities_count {
                        log(
                            LogTag::Trader,
                            "OPPORTUNITIES_LIMITED",
                            &format!(
                                "⚠️ Limited by position slots: Processing {} out of {} opportunities",
                                opportunities_to_process.len(),
                                total_opportunities_count
                            )
                        );
                    }
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Processing {} opportunities concurrently (available slots: {}, current open: {})",
                        opportunities_to_process.len(),
                        available_slots,
                        current_open_count
                    )
                );

                // Use a semaphore to limit concurrent buy transactions
                use tokio::sync::Semaphore;
                let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent buys

                let mut handles = Vec::new();

                // Process all buy orders concurrently
                for (token, price, percent_change) in opportunities_to_process {
                    // Check for shutdown before spawning tasks
                    if
                        check_shutdown_or_delay(
                            &shutdown,
                            Duration::from_millis(BUY_OPERATION_SHUTDOWN_CHECK_MS)
                        ).await
                    {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "new entries monitor shutting down during buy processing..."
                        );
                        break;
                    }

                    // Get permit from semaphore to limit concurrency with timeout
                    let permit = match
                        tokio::time::timeout(
                            Duration::from_secs(BUY_SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                            semaphore.clone().acquire_owned()
                        ).await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to acquire semaphore permit for buy: {}", e)
                            );
                            continue;
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                "Semaphore acquire timed out for buy operation"
                            );
                            continue;
                        }
                    };

                    // Clone shutdown for use in the spawned task
                    let shutdown_for_task = shutdown.clone();
                    let handle = tokio::spawn(async move {
                        let _permit = permit; // Keep permit alive for duration of task

                        // CRITICAL OPERATION PROTECTION - Prevent shutdown during buy
                        let _guard = CriticalOperationGuard::new(&format!("BUY_{}", token.symbol));

                        let token_symbol = token.symbol.clone();

                        // Check for shutdown before starting buy operation (non-blocking check)
                        let shutdown_check = tokio::time::timeout(
                            Duration::from_millis(BUY_OPERATION_SHUTDOWN_CHECK_MS),
                            shutdown_for_task.notified()
                        ).await;
                        if shutdown_check.is_ok() {
                            log(
                                LogTag::Trader,
                                "SHUTDOWN",
                                &format!("Skipping buy operation for {} - shutdown in progress", token_symbol)
                            );
                            return false;
                        }

                        // Wrap the buy operation in a timeout
                        match
                            tokio::time::timeout(
                                Duration::from_secs(BUY_OPERATION_SMART_TIMEOUT_SECS),
                                async {
                                    open_position(&token, price, percent_change).await
                                }
                            ).await
                        {
                            Ok(_) => {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Completed buy operation for {} in concurrent task", token_symbol)
                                );
                                true
                            }
                            Err(_) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!(
                                        "Buy operation for {} timed out after {} seconds",
                                        token_symbol,
                                        BUY_OPERATION_SMART_TIMEOUT_SECS
                                    )
                                );
                                false
                            }
                        }
                    });

                    handles.push(handle);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Spawned {} concurrent buy tasks", handles.len()).dimmed().to_string()
                );

                // Collect results from all concurrent buy operations with overall timeout
                let collection_result = tokio::time::timeout(
                    Duration::from_secs(BUY_OPERATIONS_COLLECTION_TIMEOUT_SECS),
                    async {
                        let mut completed = 0;
                        let mut successful = 0;
                        let total_handles = handles.len();

                        for handle in handles {
                            // Skip if shutdown signal received
                            if
                                check_shutdown_or_delay(
                                    &shutdown,
                                    Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS)
                                ).await
                            {
                                log(
                                    LogTag::Trader,
                                    "INFO",
                                    "new entries monitor shutting down during buy result collection..."
                                );
                                break;
                            }

                            // Add timeout for each handle to prevent getting stuck
                            match
                                tokio::time::timeout(
                                    Duration::from_secs(BUY_OPERATION_SMART_TIMEOUT_SECS),
                                    handle
                                ).await
                            {
                                Ok(task_result) => {
                                    match task_result {
                                        Ok(success) => {
                                            if success {
                                                successful += 1;
                                            }
                                        }
                                        Err(e) => {
                                            log(
                                                LogTag::Trader,
                                                "ERROR",
                                                &format!("Buy task failed: {}", e)
                                            );
                                        }
                                    }
                                }
                                Err(_) => {
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!("Buy task timed out after {} seconds", BUY_OPERATION_SMART_TIMEOUT_SECS)
                                    );
                                }
                            }

                            completed += 1;
                            if completed % 2 == 0 || completed == total_handles {
                                log(
                                    LogTag::Trader,
                                    "INFO",
                                    &format!(
                                        "Completed {}/{} buy operations",
                                        completed,
                                        total_handles
                                    )
                                        .dimmed()
                                        .to_string()
                                );
                            }
                        }

                        (completed, successful)
                    }
                ).await;

                match collection_result {
                    Ok((completed, successful)) => {
                        let new_open_count = get_open_positions_count();
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!(
                                "Concurrent buy operations completed: {}/{} successful, new open positions: {}",
                                successful,
                                completed,
                                new_open_count
                            )
                        );
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Buy operations collection timed out after {} seconds", BUY_OPERATIONS_COLLECTION_TIMEOUT_SECS)
                        );
                    }
                }
            }
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token checking cycle took longer than interval: {:?}", cycle_duration)
            );
            Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
        } else {
            // Otherwise wait for the remaining interval time
            Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        if check_shutdown_or_delay(&shutdown, wait_time).await {
            log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
            break;
        }
    }
}

/// Background task to monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    loop {
        // First, collect all open position mints to fetch pool prices in parallel
        let open_position_mints: Vec<String> = {
            if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions
                    .iter()
                    .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
                    .map(|p| p.mint.clone())
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Request immediate pool price checks for open positions (non-blocking)
        if !open_position_mints.is_empty() {
            for mint in &open_position_mints {
                let mint_clone = mint.clone();
                // Update prices for open positions using thread-safe price service
                tokio::spawn(async move {
                    // Use thread-safe price function for immediate price check
                    let _price = crate::tokens::price::get_token_price_safe(&mint_clone).await;
                });
            }
        }

        let mut positions_to_close = Vec::new();

        // First, collect open positions data (without holding mutex across await)
        let open_positions_data: Vec<(usize, Position)> = {
            if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, position)| {
                        if position.position_type == "buy" && position.exit_price.is_none() {
                            Some((index, position.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Now process each position with async calls (mutex is released)
        for (index, mut position) in open_positions_data {
            // Get current price from safe price service
            if let Some(current_price) = get_token_price_blocking_safe(&position.mint).await {
                if current_price > 0.0 && current_price.is_finite() {
                    // Update position tracking (extremes) on the local copy
                    update_position_tracking(&mut position, current_price);

                    // Calculate P&L using unified function with pool price
                    let (pnl_sol, pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(current_price)
                    );

                    let now = Utc::now();

                    // Calculate sell urgency using the unified profit system
                    let (sell_urgency, sell_reason) = crate::profit::should_sell(
                        &position,
                        current_price
                    ).await;

                    // Use profit system decision only - no duplicate emergency checks
                    let should_exit = sell_urgency > 0.7;

                    if is_debug_trader_enabled() {
                        debug_trader_log(
                            "SELL_ANALYSIS",
                            &format!(
                                "{} | Urgency: {:.2} | Reason: {} | Exit: {}",
                                position.symbol,
                                sell_urgency,
                                sell_reason,
                                should_exit
                            )
                        );
                    }

                    if should_exit {
                        let minimal_token = Token {
                            mint: position.mint.clone(),
                            symbol: position.symbol.clone(),
                            name: position.symbol.clone(),
                            chain: "solana".to_string(),
                            // All other fields as None/defaults
                            logo_url: None,
                            coingecko_id: None,
                            website: None,
                            description: None,
                            tags: vec![],
                            is_verified: false,
                            created_at: None,
                            price_dexscreener_sol: None,
                            price_dexscreener_usd: None,
                            price_pool_sol: None,
                            price_pool_usd: None,
                            dex_id: None,
                            pair_address: None,
                            pair_url: None,
                            labels: vec![],
                            fdv: None,
                            market_cap: None,
                            txns: None,
                            volume: None,
                            price_change: None,
                            liquidity: None,
                            info: None,
                            boosts: None,
                        };

                        log(
                            LogTag::Trader,
                            "SELL",
                            &format!(
                                "Sell signal for {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Emergency: {}",
                                position.symbol,
                                position.mint,
                                sell_urgency,
                                pnl_percent,
                                sell_urgency >= 1.0 // Emergency is when urgency is 1.0 (from profit.rs)
                            )
                        );

                        positions_to_close.push((
                            index,
                            position.clone(), // Include the full position data
                            minimal_token,
                            current_price,
                            now,
                            sell_urgency, // Include urgency score for safeguard logic
                        ));
                    } else {
                        // log(
                        //     LogTag::Trader,
                        //     "HOLD",
                        //     &format!(
                        //         "Holding {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Price: {:.12}",
                        //         position.symbol,
                        //         position.mint,
                        //         sell_urgency,
                        //         pnl_percent,
                        //         current_price
                        //     )
                        // );
                    }

                    // Update the position in the global list with tracking data
                    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                        if let Some(saved_position) = positions.get_mut(index) {
                            saved_position.price_highest = position.price_highest;
                            saved_position.price_lowest = position.price_lowest;
                        }
                    }
                } else {
                    // Price found but invalid (0, negative, or NaN)
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!(
                            "Invalid price for position monitoring: {} ({}) - Price = {:.10}",
                            position.symbol,
                            position.mint,
                            current_price
                        )
                    );
                }
            } else {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!(
                        "No price found for open position: {} ({})",
                        position.symbol,
                        position.mint
                    )
                );
            }
        }

        // Close positions that need to be closed concurrently (outside of lock to avoid deadlock)
        if !positions_to_close.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!("Processing {} positions for concurrent closing", positions_to_close.len())
            );

            // Use a semaphore to limit concurrent sell transactions to avoid overwhelming the network
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent sells

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (
                index,
                position,
                token,
                exit_price,
                exit_time,
                sell_urgency,
            ) in positions_to_close {
                // Check for shutdown before spawning tasks
                if
                    check_shutdown_or_delay(
                        &shutdown,
                        Duration::from_millis(SELL_OPERATION_SHUTDOWN_CHECK_MS)
                    ).await
                {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "open positions monitor shutting down during sell processing..."
                    );
                    break;
                }

                // Get permit from semaphore to limit concurrency with timeout
                let permit = match
                    tokio::time::timeout(
                        Duration::from_secs(SELL_SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                        semaphore.clone().acquire_owned()
                    ).await
                {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(e)) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to acquire semaphore permit for sell: {}", e)
                        );
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            "Semaphore acquire timed out for sell operation"
                        );
                        continue;
                    }
                };

                // Clone shutdown for use in the spawned sell task
                let shutdown_for_task = shutdown.clone();
                // We already have the position from the analysis phase, no need to look it up
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    // CRITICAL OPERATION PROTECTION - Prevent shutdown during sell
                    let _guard = CriticalOperationGuard::new(&format!("SELL_{}", token.symbol));

                    let mut position = position;
                    let token_symbol = token.symbol.clone();

                    // EXECUTION SAFEGUARD: Only execute sales recommended by profit system
                    // All sell decisions should come from profit.rs, trader only executes
                    let (_, final_pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(exit_price)
                    );

                    // Log the execution details for debugging
                    log(
                        LogTag::Trader,
                        "EXECUTION",
                        &format!(
                            "Executing sale of {} - P&L {:.2}% with urgency {:.2} (decision from profit system)",
                            token_symbol,
                            final_pnl_percent,
                            sell_urgency
                        )
                    );

                    // Check for shutdown before starting sell operation (non-blocking check)
                    let shutdown_check = tokio::time::timeout(
                        Duration::from_millis(SELL_OPERATION_SHUTDOWN_CHECK_MS),
                        shutdown_for_task.notified()
                    ).await;
                    if shutdown_check.is_ok() {
                        log(
                            LogTag::Trader,
                            "SHUTDOWN",
                            &format!("Skipping sell operation for {} - shutdown in progress", token_symbol)
                        );
                        return None;
                    }

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(
                            Duration::from_secs(SELL_OPERATION_SMART_TIMEOUT_SECS),
                            async {
                                close_position(&mut position, &token, exit_price, exit_time).await
                            }
                        ).await
                    {
                        Ok(success) => {
                            if success {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Successfully closed position for {} in concurrent task", token_symbol)
                                );
                                Some((index, position))
                            } else {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Failed to close position for {} in concurrent task", token_symbol)
                                );
                                None
                            }
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Sell operation for {} timed out after {} seconds",
                                    token_symbol,
                                    SELL_OPERATION_SMART_TIMEOUT_SECS
                                )
                            );
                            None
                        }
                    }
                });

                handles.push(handle);
            }

            log(
                LogTag::Trader,
                "INFO",
                &format!("Spawned {} concurrent sell tasks", handles.len()).dimmed().to_string()
            );

            // Collect results from all concurrent sell operations with overall timeout
            // Increased timeout to 60 seconds to accommodate multiple 15-second sell operations
            let collection_result = tokio::time::timeout(
                Duration::from_secs(SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS),
                async {
                    let mut completed_positions = Vec::new();

                    for handle in handles {
                        // Skip if shutdown signal received
                        if
                            check_shutdown_or_delay(
                                &shutdown,
                                Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS)
                            ).await
                        {
                            log(
                                LogTag::Trader,
                                "INFO",
                                "open positions monitor shutting down during sell result collection..."
                            );
                            break;
                        }

                        // Add timeout for each handle to prevent getting stuck
                        // Increased timeout to 15 seconds to allow for transaction verification and ATA closing
                        match
                            tokio::time::timeout(
                                Duration::from_secs(SELL_TASK_HANDLE_TIMEOUT_SECS),
                                handle
                            ).await
                        {
                            Ok(task_result) => {
                                match task_result {
                                    Ok(Some((index, updated_position))) => {
                                        completed_positions.push((index, updated_position));
                                    }
                                    Ok(None) => {
                                        // Position failed to close, continue
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Trader,
                                            "ERROR",
                                            &format!("Sell task failed: {}", e)
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                log(
                                    LogTag::Trader,
                                    "WARN",
                                    &format!("Sell task timed out after {} seconds", SELL_TASK_HANDLE_TIMEOUT_SECS)
                                );
                            }
                        }
                    }

                    completed_positions
                }
            ).await;

            let completed_positions = match collection_result {
                Ok(positions) => positions,
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Sell operations collection timed out after {} seconds", SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS)
                    );
                    Vec::new()
                }
            };

            // Update all successfully closed positions in the saved positions
            if !completed_positions.is_empty() {
                if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                    for (index, updated_position) in &completed_positions {
                        if let Some(saved_position) = positions.get_mut(*index) {
                            *saved_position = updated_position.clone();
                        }
                    }
                    save_positions_to_file(&positions);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Updated {} positions after concurrent sell operations",
                        completed_positions.len()
                    )
                );
            }
        }

        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "open positions monitor shutting down...");
            break;
        }
    }
}
