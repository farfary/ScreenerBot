/// Trading System Orchestrator
///
/// ======================================
/// TRADER MODULE RESPONSIBILITIES
/// ======================================
///
/// The trader module serves as the main orchestrator for automated trading operations:
///
/// **Core Functions:**
/// 1. **Entry Monitoring** - Continuously scans tokens for trading opportunities
/// 2. **Position Monitoring** - Tracks open positions and monitors exit conditions
/// 3. **System Coordination** - Integrates filtering, entry, profit, and position systems
/// 4. **Concurrency Management** - Handles parallel token processing with semaphores
/// 5. **Safety Controls** - Implements critical operation guards and shutdown handling
///
/// **Background Services:**
/// - `monitor_new_entries()` - Scans for new position opportunities every 2 seconds
/// - `monitor_open_positions()` - Monitors existing positions for exit signals every 2 seconds
///
/// **Integration Points:**
/// - Uses `filtering` module to determine token eligibility
/// - Delegates entry decisions to `entry` module
/// - Delegates exit decisions to `profit` module
/// - Executes trades through `positions` manager
/// - Coordinates with `tokens` system for price data
///
/// **Safety Features:**
/// - Critical operation guards prevent shutdown during trades
/// - Semaphore-based concurrency limiting (5 entry checks, 3 concurrent sells)
/// - Comprehensive timeout handling for all operations
/// - Graceful shutdown with proper cleanup

// =============================================================================
// TRADING SYSTEM CONFIGURATION CONSTANTS
// =============================================================================

// -----------------------------------------------------------------------------
// Core Trading Parameters
// -----------------------------------------------------------------------------

/// Maximum number of concurrent open positions
pub const MAX_OPEN_POSITIONS: usize = 2;

/// Trade size in SOL for each position
pub const TRADE_SIZE_SOL: f64 = 0.005;

/// Enable minimum profit threshold requirement before allowing sells
pub const MIN_PROFIT_THRESHOLD_ENABLED: bool = true;

/// Minimum profit threshold percentage (e.g., 5.0 for 5%, -5.0 for -5%)
/// Positions below this P&L will not be sold regardless of other exit conditions
pub const MIN_PROFIT_THRESHOLD_PERCENT: f64 = 0.0;

/// Time-based override: Allow sell decisions after this duration (hours)
/// Positions held longer than this can bypass profit threshold if in significant loss
/// This prevents positions from being held indefinitely when they're clearly failing
pub const TIME_OVERRIDE_DURATION_HOURS: f64 = 12.0;

/// Loss threshold for time-based override (negative percentage, e.g., -20.0 for -20%)
/// Positions with losses worse than this threshold can bypass profit requirements after time override
/// This allows cutting losses on positions that have been failing for extended periods
pub const TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT: f64 = -40.0;

pub const PROFIT_EXTRA_NEEDED_SOL: f64 = 0.00005;

// -----------------------------------------------------------------------------
// Debug Mode Configuration
// -----------------------------------------------------------------------------

/// Debug mode: Force sell all positions after a timeout (for testing)
pub const DEBUG_FORCE_SELL_MODE: bool = false;

/// Debug mode: Force sell timeout in seconds
pub const DEBUG_FORCE_SELL_TIMEOUT_SECS: f64 = 20.0;

// -----------------------------------------------------------------------------
// Position Timing Configuration - Improved for longer holding
// -----------------------------------------------------------------------------

/// Minimum hold time before considering sell (seconds) - reduced for flexibility
pub const MIN_POSITION_HOLD_TIME_SECS: f64 = 45.0;

/// Maximum hold time cap for open positions (reduced to 45 minutes for risk control)
pub const MAX_POSITION_HOLD_TIME_SECS: f64 = 45.0 * 60.0; // 45 minutes (2700 seconds)

/// Re-entry cooldown after closing a position (minutes) - prevents immediate re-buy of same token
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 6 * 60; // 6 hours

// -----------------------------------------------------------------------------
// Trading Logic Configuration
// -----------------------------------------------------------------------------
// Monitoring & Display Configuration
// -----------------------------------------------------------------------------

/// Summary display refresh interval (seconds) - optimized for 5s priority checking
pub const SUMMARY_DISPLAY_INTERVAL_SECS: u64 = 5;

/// New entry signals check interval (seconds) - optimized for fastest price checking
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 5;

/// Open positions monitoring interval (seconds) - maximum priority price checking every 5 seconds
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

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

use crate::global::is_debug_trader_enabled;
use crate::logger::{log, LogTag};
use crate::positions_lib::calculate_position_pnl;
use crate::tokens::{
    discover_tokens_once, get_all_tokens_by_liquidity, get_price,
    pool::{add_watchlist_tokens, get_pool_service},
    sync_watch_list_with_trader, PriceOptions, Token,
};
use crate::utils::check_shutdown_or_delay;
use crate::utils::*;

use crate::entry::get_profit_target;
use crate::entry::should_buy;
use crate::errors::{PositionError, ScreenerBotError};
use crate::filtering::log_filtering_summary;

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use chrono::Utc;
use colored::Colorize;
use futures;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Notify;

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: tracks critical trading operations in progress to prevent force shutdown
pub static CRITICAL_OPERATIONS_IN_PROGRESS: Lazy<Arc<std::sync::atomic::AtomicUsize>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicUsize::new(0)));

/// Global tracker: number of buy operations currently in-flight (reserved but not yet reflected in open positions)
// removed legacy in-flight buy tracking; PositionsManager enforces capacity

// =============================================================================
// TOKEN TRACKING FOR INTELLIGENT CHECKING
// =============================================================================

/// Tracks token checking information for intelligent prioritization
#[derive(Clone)]
pub struct TokenCheckInfo {
    pub last_check_time: Instant,
    pub last_price: Option<f64>,
    pub check_count: usize,
    pub had_recent_drop: bool,
}

/// Global token tracking state
pub static TOKEN_CHECK_TRACKER: Lazy<Arc<std::sync::RwLock<HashMap<String, TokenCheckInfo>>>> =
    Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

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
        let count =
            CRITICAL_OPERATIONS_IN_PROGRESS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        log(
            LogTag::Trader,
            "CRITICAL_OP_START",
            &format!(
                "🔒 PROTECTED: {} operation started (active operations: {})",
                operation_name,
                count + 1
            ),
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
        let count =
            CRITICAL_OPERATIONS_IN_PROGRESS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        log(
            LogTag::Trader,
            "CRITICAL_OP_END",
            &format!(
                "🔓 UNPROTECTED: Critical operation finished (remaining operations: {})",
                count - 1
            ),
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

/// Debug function: Check if a position should be force-sold due to debug timeout
pub fn should_debug_force_sell(position: &crate::positions_types::Position) -> bool {
    if !DEBUG_FORCE_SELL_MODE {
        return false;
    }

    let position_age_secs = Utc::now()
        .signed_duration_since(position.entry_time)
        .num_seconds() as f64;

    if position_age_secs >= DEBUG_FORCE_SELL_TIMEOUT_SECS {
        log(
            LogTag::Trader,
            "DEBUG_FORCE_SELL",
            &format!(
                "🚨 DEBUG MODE: Force selling {} after {:.1}s (timeout: {:.1}s)",
                position.symbol, position_age_secs, DEBUG_FORCE_SELL_TIMEOUT_SECS
            ),
        );
        return true;
    }

    false
}

// =============================================================================
// TOKEN TRACKING AND INTELLIGENT PRIORITIZATION
// =============================================================================

/// Update token tracking after checking a token
pub fn update_token_check_info(mint: &str, current_price: Option<f64>, had_drop: bool) {
    let mut tracker = TOKEN_CHECK_TRACKER.write().unwrap();
    let info = tracker.entry(mint.to_string()).or_insert(TokenCheckInfo {
        last_check_time: Instant::now(),
        last_price: None,
        check_count: 0,
        had_recent_drop: false,
    });

    info.last_check_time = Instant::now();
    if let Some(price) = current_price {
        info.last_price = Some(price);
    }
    info.check_count += 1;
    info.had_recent_drop = had_drop;
}

/// Check if token had recent price drop (within 30 seconds)
pub async fn check_token_for_recent_drop(token: &Token) -> bool {
    let pool_service = get_pool_service();
    let history = pool_service.get_recent_price_history(&token.mint).await;

    if history.len() < 2 {
        return false;
    }

    // Check for drops in last 30 seconds
    let now = chrono::Utc::now();
    let thirty_seconds_ago = now - chrono::Duration::seconds(30);

    let recent_prices: Vec<_> = history
        .into_iter()
        .filter(|(timestamp, _)| *timestamp > thirty_seconds_ago)
        .collect();

    if recent_prices.len() < 2 {
        return false;
    }

    // Check if there was a significant drop (>2%) in recent period
    let max_recent = recent_prices
        .iter()
        .map(|(_, price)| *price)
        .fold(0.0f64, f64::max);
    let current = recent_prices.last().map(|(_, price)| *price).unwrap_or(0.0);

    if max_recent > 0.0 && current > 0.0 {
        let drop_percent = ((max_recent - current) / max_recent) * 100.0;
        drop_percent > 2.0
    } else {
        false
    }
}

/// Prioritize tokens for checking based on drops, check history, and fairness
pub fn prioritize_tokens_for_checking(mut tokens: Vec<Token>) -> Vec<Token> {
    let now = Instant::now();
    let tracker = TOKEN_CHECK_TRACKER.read().unwrap();

    // Sort tokens by priority:
    // 1. Tokens that had recent drops (within 30s)
    // 2. Tokens that haven't been checked or haven't been checked in >1min with no price change
    // 3. Tokens with fewest check counts (fairness)
    // 4. Others

    tokens.sort_by(|a, b| {
        let info_a = tracker.get(&a.mint);
        let info_b = tracker.get(&b.mint);

        // Priority 1: Recent drops (highest priority)
        let drop_a = info_a.map(|i| i.had_recent_drop).unwrap_or(false);
        let drop_b = info_b.map(|i| i.had_recent_drop).unwrap_or(false);
        if drop_a != drop_b {
            return drop_b.cmp(&drop_a); // true first
        }

        // Priority 2: Never checked or stale checks (>60s)
        let stale_a = info_a
            .map(|i| now.duration_since(i.last_check_time).as_secs() > 60)
            .unwrap_or(true);
        let stale_b = info_b
            .map(|i| now.duration_since(i.last_check_time).as_secs() > 60)
            .unwrap_or(true);
        if stale_a != stale_b {
            return stale_b.cmp(&stale_a); // true first
        }

        // Priority 3: Fairness - fewer checks first
        let count_a = info_a.map(|i| i.check_count).unwrap_or(0);
        let count_b = info_b.map(|i| i.check_count).unwrap_or(0);
        count_a.cmp(&count_b)
    });

    // Clean up very old entries (>10 minutes) to prevent memory growth
    drop(tracker);
    let mut tracker_write = TOKEN_CHECK_TRACKER.write().unwrap();
    tracker_write.retain(|_, info| now.duration_since(info.last_check_time).as_secs() < 600);

    tokens
}

// =============================================================================
// TOKEN PREPARATION AND TRADING FUNCTIONS
// =============================================================================

/// Prepare tokens for filtering and trading by fetching from database
/// Returns all available tokens ready for the filtering system to process
pub async fn prepare_tokens(cycle_start: std::time::Instant) -> Result<Vec<Token>, String> {
    use crate::filtering::{
        filter_tokens_with_reasons, get_filtering_stats, log_transaction_activity_stats,
    };

    // Timeout for filtering operations
    const FILTERING_TIMEOUT_SECS: u64 = 120;

    // 1. Fetch tokens from safe system
    let mut tokens = {
        let tokens_from_module: Vec<Token> = match get_all_tokens_by_liquidity().await {
            Ok(api_tokens) => {
                // Convert ApiToken to Token for compatibility with existing code
                api_tokens
                    .into_iter()
                    .map(|api_token| api_token.into())
                    .collect()
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!("Failed to get tokens from safe system: {}", e),
                );
                Vec::new()
            }
        };

        // Log total tokens available
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "Total tokens from safe system: {}",
                    tokens_from_module.len()
                ),
            );
        }

        // Count tokens with liquidity data
        let with_liquidity = tokens_from_module
            .iter()
            .filter(|token| token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0) > 0.0)
            .count();

        if with_liquidity > 0 {
            log(
                LogTag::Trader,
                "INFO",
                &format!("Processing {} tokens with liquidity", with_liquidity),
            );
        }

        // Keep tokens in liquidity order for consistent processing
        // Smart prioritization will happen after filtering
        tokens_from_module
    };

    // 2. Apply filtering with timeout protection
    let filtering_result = tokio::time::timeout(
        std::time::Duration::from_secs(FILTERING_TIMEOUT_SECS),
        async {
            // Run filtering in spawn_blocking to avoid blocking the async runtime
            tokio::task::spawn_blocking({
                let tokens_copy = tokens.to_vec();
                move || filter_tokens_with_reasons(&tokens_copy)
            })
            .await
        },
    )
    .await;

    let (eligible_tokens, rejected_tokens) = match filtering_result {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => {
            return Err(format!("Token filtering task failed: {}", e));
        }
        Err(_) => {
            return Err(format!(
                "Token filtering timed out after {}s",
                FILTERING_TIMEOUT_SECS
            ));
        }
    };

    // 3. Log filtering statistics
    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "FILTER_STATS",
            &format!(
                "Token filtering: {}/{} passed ({:.1}% pass rate) - processed {} tokens",
                eligible_tokens.len(),
                tokens.len(),
                if tokens.len() > 0 {
                    ((eligible_tokens.len() as f64) / (tokens.len() as f64)) * 100.0
                } else {
                    0.0
                },
                tokens.len()
            ),
        );
    }

    // 4. Log transaction activity statistics (debug mode)
    if is_debug_trader_enabled() {
        log_transaction_activity_stats(&tokens);
    }

    // 5. Debug logging for rejected tokens
    if is_debug_trader_enabled() && !rejected_tokens.is_empty() {
        let sample_size = std::cmp::min(5, rejected_tokens.len());
        for (token, reason) in rejected_tokens.iter().take(sample_size) {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!("🚫 {} filtered out: {:?}", token.symbol, reason),
            );
        }
        if rejected_tokens.len() > sample_size {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "... and {} more tokens filtered out",
                    rejected_tokens.len() - sample_size
                ),
            );
        }
    }

    // 6. Smart token prioritization instead of random shuffling
    let prioritized_tokens = prioritize_tokens_for_checking(eligible_tokens);

    if is_debug_trader_enabled() && !prioritized_tokens.is_empty() {
        log(
            LogTag::Trader,
            "PRIORITY_ORDER",
            &format!(
                "📊 Prioritized {} tokens: drops={}, fair_rotation={}, others={}",
                prioritized_tokens.len(),
                prioritized_tokens
                    .iter()
                    .take(10)
                    .filter(|t| {
                        TOKEN_CHECK_TRACKER
                            .read()
                            .unwrap()
                            .get(&t.mint)
                            .map(|info| info.had_recent_drop)
                            .unwrap_or(false)
                    })
                    .count(),
                prioritized_tokens
                    .iter()
                    .filter(|t| {
                        TOKEN_CHECK_TRACKER
                            .read()
                            .unwrap()
                            .get(&t.mint)
                            .map(|info| info.check_count == 0)
                            .unwrap_or(true)
                    })
                    .count(),
                prioritized_tokens.len() - 10
            ),
        );
    }

    Ok(prioritized_tokens)
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    log(
        LogTag::Trader,
        "STARTUP",
        "🚀 Starting monitor_new_entries task",
    );

    'outer: loop {
        // Check for shutdown at the very beginning of each loop iteration
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
            log(
                LogTag::Trader,
                "INFO",
                "✅ New entries monitor shutdown requested at loop start",
            );
            break 'outer;
        }

        // CRITICAL: Wait for position recalculation to complete before starting any trading operations
        if !crate::global::POSITION_RECALCULATION_COMPLETE.load(std::sync::atomic::Ordering::SeqCst)
        {
            log(
                LogTag::Trader,
                "STARTUP",
                "⏳ Waiting for position recalculation to complete...",
            );

            // Use shutdown-aware sleep instead of fixed sleep
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(1)).await {
                log(
                    LogTag::Trader,
                    "INFO",
                    "✅ New entries monitor shutdown during position recalc wait",
                );
                break 'outer;
            }
            continue;
        }

        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        // Check for shutdown before starting main processing
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
            log(
                LogTag::Trader,
                "INFO",
                "✅ New entries monitor shutdown before token processing",
            );
            break 'outer;
        }

        // Prepare tokens for trading (fetch, sort, filter)
        let tokens = match prepare_tokens(cycle_start).await {
            Ok(tokens) => tokens,
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Token preparation failed: {}", e),
                );
                continue; // Skip this cycle and try again
            }
        };

        // Early return if no tokens to process
        if tokens.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                "No tokens to process, skipping token checking cycle",
            );

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
            } else {
                Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(
                    LogTag::Trader,
                    "INFO",
                    "new entries monitor shutting down...",
                );
                break;
            }
            continue;
        }

        // Limit concurrent token checks to avoid overwhelming services
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(5)); // Reduced to 5 concurrent checks to avoid overwhelming

        // Log filtering summary
        log_filtering_summary(&tokens);

        // Sync OHLCV watch list with trader tokens (run async to not block trading)
        let shutdown_clone_for_ohlcv = shutdown.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_watch_list_with_trader(Some(shutdown_clone_for_ohlcv)).await {
                log(LogTag::Trader, "WARN", &format!("OHLCV sync failed: {}", e));
            }
        });

        // Process tokens in parallel; for valid entries, send OpenPosition via PositionsHandle
        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        // Note: tokens are now prioritized by recent drops and fair rotation
        for token in tokens.iter() {
            // Check for shutdown before spawning tasks
            if check_shutdown_or_delay(
                &shutdown,
                Duration::from_millis(TOKEN_PROCESSING_SHUTDOWN_CHECK_MS),
            )
            .await
            {
                log(
                    LogTag::Trader,
                    "INFO",
                    "new entries monitor shutting down...",
                );
                break 'outer;
            }

            // Get permit from semaphore to limit concurrency with timeout
            let permit = match tokio::time::timeout(
                Duration::from_secs(SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                semaphore.clone().acquire_owned(),
            )
            .await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(e)) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to acquire semaphore permit: {}", e),
                    );
                    continue;
                }
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!(
                            "Semaphore acquire timed out after {} seconds",
                            SEMAPHORE_ACQUIRE_TIMEOUT_SECS
                        ),
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
                if check_shutdown_or_delay(
                    &shutdown_clone,
                    Duration::from_millis(TASK_SHUTDOWN_CHECK_MS),
                )
                .await
                {
                    return;
                }

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(TOKEN_CHECK_TASK_TIMEOUT_SECS), async {
                        let pool_service = get_pool_service();

                        // Get current pool price
                        let current_price = match
                            pool_service
                                .get_pool_price(&token.mint, None, &PriceOptions::default()).await
                                .and_then(|r| r.price_sol)
                        {
                            Some(p) if p > 0.0 && p.is_finite() => p,
                            _ => {
                                // Update tracking even for failed price fetches
                                update_token_check_info(&token.mint, None, false);
                                return;
                            }
                        };

                        // Check for recent drops
                        let had_recent_drop = check_token_for_recent_drop(&token).await;

                        // Entry decision delegated to entry::should_buy
                        let (approved, _confidence, reason) = should_buy(&token).await;

                        // Update token tracking info
                        update_token_check_info(&token.mint, Some(current_price), had_recent_drop);

                        if !approved {
                            // Token passed filtering but doesn't meet entry criteria
                            // Add to watchlist for future monitoring
                            add_watchlist_tokens(&[token.mint.clone()]).await;

                            if is_debug_trader_enabled() {
                                log(
                                    LogTag::Trader,
                                    "WATCHLIST_ADD",
                                    &format!(
                                        "Added {} to watchlist (passed filtering, waiting for entry signal: {}) [Drop: {}]",
                                        &token.symbol,
                                        reason,
                                        if had_recent_drop {
                                            "YES"
                                        } else {
                                            "NO"
                                        }
                                    )
                                );
                            }
                            return;
                        }

                        // Token approved for entry!

                        // Compute percent change from recent history if available
                        let change = {
                            let history = pool_service.get_recent_price_history(&token.mint).await;
                            if history.len() >= 2 {
                                let prev = history[history.len() - 2].1;
                                if prev > 0.0 {
                                    ((current_price - prev) / prev) * 100.0
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        };

                        // Get profit targets and liquidity tier
                        let (profit_min, profit_max) = get_profit_target(&token).await;

                        // Get liquidity tier from pool data
                        let liquidity_tier = if
                            let Some(price_result) = get_price(
                                &token.mint,
                                Some(PriceOptions::pool_only()),
                                false
                            ).await
                        {
                            let liquidity_usd = price_result.liquidity_usd.unwrap_or(0.0);
                            if liquidity_usd < 0.0 {
                                Some("INVALID".to_string())
                            } else {
                                let tier = match liquidity_usd {
                                    x if x < 1_000.0 => "MICRO", // < $1K
                                    x if x < 10_000.0 => "SMALL", // $1K - $10K
                                    x if x < 50_000.0 => "MEDIUM", // $10K - $50K
                                    x if x < 250_000.0 => "LARGE", // $50K - $250K
                                    x if x < 1_000_000.0 => "XLARGE", // $250K - $1M
                                    _ => "MEGA", // > $1M
                                };
                                Some(tier.to_string())
                            }
                        } else {
                            Some("UNKNOWN".to_string())
                        };

                        // Open position directly
                        let _ = crate::positions::open_position_direct(
                            &token,
                            current_price,
                            change,
                            TRADE_SIZE_SOL,
                            liquidity_tier,
                            profit_min,
                            profit_max
                        ).await;
                    }).await
                {
                    Ok(_) => {}
                    Err(_) => {}
                }
            });

            handles.push(handle);
        }

        // Wait for tasks to finish with overall timeout (best-effort)
        let collection_result = tokio::time::timeout(
            Duration::from_secs(TOKEN_CHECK_COLLECTION_TIMEOUT_SECS),
            async {
                for handle in handles {
                    if check_shutdown_or_delay(
                        &shutdown,
                        Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS),
                    )
                    .await
                    {
                        return;
                    }
                    let _ = tokio::time::timeout(
                        Duration::from_secs(TOKEN_CHECK_HANDLE_TIMEOUT_SECS),
                        handle,
                    )
                    .await;
                }
            },
        )
        .await;
        if collection_result.is_err() {
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "Token check collection timed out after {} seconds",
                    TOKEN_CHECK_COLLECTION_TIMEOUT_SECS
                ),
            );
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!(
                    "Token checking cycle took longer than interval: {:?}",
                    cycle_duration
                ),
            );
            Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
        } else {
            // Otherwise wait for the remaining interval time
            Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        if check_shutdown_or_delay(&shutdown, wait_time).await {
            log(
                LogTag::Trader,
                "INFO",
                "new entries monitor shutting down...",
            );
            break;
        }
    }
}

/// Background task to monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    loop {
        // CRITICAL: Wait for position recalculation to complete before starting any position monitoring
        if !crate::global::POSITION_RECALCULATION_COMPLETE.load(std::sync::atomic::Ordering::SeqCst)
        {
            log(
                LogTag::Trader,
                "STARTUP",
                "⏳ Position monitor waiting for recalculation to complete...",
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // First, collect all open position mints to fetch pool prices in parallel
        let open_position_mints: Vec<String> = crate::positions::get_open_mints().await;

        // Request priority price updates for all open positions at the start of each cycle
        // This ensures we have fresh prices before making any trading decisions
        if !open_position_mints.is_empty() {
            if is_debug_trader_enabled() {
                debug_trader_log(
                    "PRIORITY_UPDATE",
                    &format!(
                        "Requesting priority price updates for {} open positions",
                        open_position_mints.len()
                    ),
                );
            }

            // Use the pool service to request priority updates
            let updated_count = crate::tokens::request_priority_updates_for_open_positions().await;

            if is_debug_trader_enabled() {
                debug_trader_log(
                    "PRIORITY_RESULT",
                    &format!(
                        "Priority updates completed: {}/{} successful",
                        updated_count,
                        open_position_mints.len()
                    ),
                );
            }
        }

        let mut positions_to_close = Vec::new();

        // First, collect open positions data (without holding mutex across await)
        let open_positions_data_all: Vec<crate::positions_types::Position> =
            crate::positions::get_open_positions().await;

        // Filter to only verified-entry, not yet exited positions (preserve previous behavior)
        let mut unverified_count = 0usize;
        let open_positions_data: Vec<crate::positions_types::Position> = open_positions_data_all
            .into_iter()
            .filter(|p| {
                if p.transaction_entry_verified {
                    p.exit_price.is_none()
                } else {
                    unverified_count += 1;
                    false
                }
            })
            .collect();

        if unverified_count > 0 {
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "Skipping {} unverified open positions, processing {} verified positions",
                    unverified_count,
                    open_positions_data.len()
                ),
            );
        }

        // Use efficient parallel price fetching - background service keeps prices fresh
        let price_futures: Vec<_> = open_positions_data
            .iter()
            .map(|pos| {
                let mint = pos.mint.clone();
                async move {
                    // Get price data from cache (background service keeps it fresh every 5s)
                    let price_result = get_price(&mint, None, false).await;

                    // Extract best available price and price info
                    if let Some(result) = price_result {
                        let best_price = result.best_sol_price();
                        if let Some(price) = best_price {
                            (mint, Some((price, result)))
                        } else {
                            (mint, None)
                        }
                    } else {
                        (mint, None)
                    }
                }
            })
            .collect();

        // Execute all price fetches in parallel
        let price_results = futures::future::join_all(price_futures).await;

        // Create price lookup map for fast access
        let price_map: std::collections::HashMap<String, (f64, crate::tokens::PriceResult)> =
            price_results
                .into_iter()
                .filter_map(|(mint, result_opt)| {
                    result_opt.map(|(price, price_result)| (mint, (price, price_result)))
                })
                .collect();

        // Now process each position with async calls (mutex is released)
        for position in open_positions_data.into_iter() {
            let mut position = position; // local mutable copy for calculations/logs

            // Get current price and price result from our parallel fetch results
            if let Some((current_price, price_result)) = price_map.get(&position.mint) {
                let current_price = *current_price;
                if current_price > 0.0 && current_price.is_finite() {
                    // Send price update to positions manager for tracking
                    let tracking_result = crate::positions::update_position_tracking(
                        &position.mint,
                        current_price,
                        price_result,
                    )
                    .await;

                    let now = Utc::now();

                    // Calculate P&L for logging and decision making
                    let (pnl_sol, pnl_percent) =
                        calculate_position_pnl(&position, Some(current_price)).await;

                    // Check debug force sell first
                    let debug_force_sell = should_debug_force_sell(&position);

                    // Calculate sell decision using the unified profit system
                    let should_exit_base = debug_force_sell
                        || crate::profit::should_sell(&position, current_price).await;

                    // Apply minimum profit threshold check if enabled
                    let should_exit = if MIN_PROFIT_THRESHOLD_ENABLED && !debug_force_sell {
                        // Check if position qualifies for time-based override
                        let position_age_hours =
                            (now.signed_duration_since(position.entry_time).num_seconds() as f64)
                                / 3600.0;
                        let time_override_applies = position_age_hours
                            >= TIME_OVERRIDE_DURATION_HOURS
                            && pnl_percent <= TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT;

                        if time_override_applies {
                            // Time override: Allow should_sell to decide for old positions with significant losses
                            should_exit_base
                        } else if pnl_percent >= MIN_PROFIT_THRESHOLD_PERCENT {
                            // Normal case: Only allow exit if P&L meets minimum threshold
                            should_exit_base
                        } else {
                            false // Block exit due to insufficient profit
                        }
                    } else {
                        should_exit_base // Normal behavior when threshold disabled or debug force sell
                    };

                    if is_debug_trader_enabled() {
                        let position_age_hours =
                            (now.signed_duration_since(position.entry_time).num_seconds() as f64)
                                / 3600.0;
                        let time_override_applies = position_age_hours
                            >= TIME_OVERRIDE_DURATION_HOURS
                            && pnl_percent <= TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT;

                        debug_trader_log(
                            "SELL_ANALYSIS",
                            &format!(
                                "{} | Should Exit: {} (Base: {}) | P&L: {:.2}% ({:.6} SOL) | Age: {:.1}h | Min Threshold: {}% (Enabled: {}) | Time Override: {} | Debug Force: {}",
                                position.symbol,
                                should_exit,
                                should_exit_base,
                                pnl_percent,
                                pnl_sol,
                                position_age_hours,
                                MIN_PROFIT_THRESHOLD_PERCENT,
                                MIN_PROFIT_THRESHOLD_ENABLED,
                                if time_override_applies {
                                    "YES"
                                } else {
                                    "NO"
                                },
                                debug_force_sell
                            )
                        );
                    }

                    if should_exit {
                        // CRITICAL: Check pool availability before selling
                        let pool_service = get_pool_service();
                        let has_pool_availability =
                            pool_service.check_token_availability(&position.mint).await;

                        if !has_pool_availability {
                            if is_debug_trader_enabled() {
                                debug_trader_log(
                                    "SELL_POOL_UNAVAILABLE",
                                    &format!(
                                        "SKIPPING SELL for {} ({}): No pool available for trading",
                                        position.symbol, position.mint
                                    ),
                                );
                            }
                            continue;
                        }

                        // Fetch full token from database
                        let Some(full_token) =
                            crate::tokens::get_token_from_db(&position.mint).await
                        else {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Token not found in DB for mint {} — skipping sell",
                                    position.mint
                                ),
                            );
                            continue;
                        };

                        log(
                            LogTag::Trader,
                            "SELL",
                            &format!(
                                "Sell signal for {} ({}) - P&L: {:.2}% ({:.6} SOL) - SHOULD EXIT",
                                position.symbol, position.mint, pnl_percent, pnl_sol
                            ),
                        );

                        positions_to_close.push((
                            position.clone(), // keep for logging only
                            full_token,
                            current_price,
                            now,
                            1.0, // High urgency since we decided to exit
                        ));
                    } else {
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "HOLD",
                                &format!(
                                    "Holding {} ({}) - P&L: {:.2}% ({:.6} SOL), Price: {:.12}",
                                    position.symbol,
                                    position.mint,
                                    pnl_percent,
                                    pnl_sol,
                                    current_price
                                ),
                            );
                        }
                    }

                    // No direct mutation of positions here; actor has updated tracking.
                } else {
                    // Price found but invalid (0, negative, or NaN)
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!(
                            "Invalid price for position monitoring: {} ({}) - Price = {:.10}",
                            position.symbol, position.mint, current_price
                        ),
                    );
                }
            } else {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!(
                        "No price found for open position: {} ({})",
                        position.symbol, position.mint
                    ),
                );
            }
        }

        // Close positions that need to be closed concurrently
        if !positions_to_close.is_empty() {
            // Use a semaphore to limit concurrent sell transactions
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent sells

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (position, token, exit_price, exit_time, sell_urgency) in positions_to_close {
                // Check for shutdown before spawning tasks
                if check_shutdown_or_delay(
                    &shutdown,
                    Duration::from_millis(SELL_OPERATION_SHUTDOWN_CHECK_MS),
                )
                .await
                {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "open positions monitor shutting down during sell processing...",
                    );
                    break;
                }

                // Get permit from semaphore to limit concurrency with timeout
                let permit = match tokio::time::timeout(
                    Duration::from_secs(SELL_SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                    semaphore.clone().acquire_owned(),
                )
                .await
                {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(_)) | Err(_) => {
                        continue;
                    }
                };

                // Clone shutdown for use in the spawned sell task
                let shutdown_for_task = shutdown.clone();
                // We already have the position from the analysis phase for logging only
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    // CRITICAL OPERATION PROTECTION - Prevent shutdown during sell
                    let _guard = CriticalOperationGuard::new(&format!("SELL_{}", token.symbol));

                    let mut position = position;
                    let token_symbol = token.symbol.clone();

                    // Check for shutdown before starting sell operation (non-blocking check)
                    let shutdown_check = tokio::time::timeout(
                        Duration::from_millis(SELL_OPERATION_SHUTDOWN_CHECK_MS),
                        shutdown_for_task.notified(),
                    )
                    .await;
                    if shutdown_check.is_ok() {
                        return false;
                    }

                    // Wrap the sell operation in a timeout
                    match tokio::time::timeout(
                        Duration::from_secs(SELL_OPERATION_SMART_TIMEOUT_SECS),
                        async {
                            crate::positions::close_position_direct(
                                &position.mint,
                                &token,
                                exit_price,
                                "Trading decision".to_string(),
                                Utc::now(),
                            )
                            .await
                            .map(|_| ())
                        },
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!("Successfully closed position for {}", token_symbol),
                            );
                            true
                        }
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to close position for {}: {}", token_symbol, e),
                            );
                            false
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Sell operation for {} timed out", token_symbol),
                            );
                            false
                        }
                    }
                });

                handles.push(handle);
            }

            // Collect results from all concurrent sell operations
            let collection_result = tokio::time::timeout(
                Duration::from_secs(SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS),
                async {
                    let mut completed = 0usize;
                    let mut successful = 0usize;

                    for handle in handles {
                        // Skip if shutdown signal received
                        if check_shutdown_or_delay(
                            &shutdown,
                            Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS),
                        )
                        .await
                        {
                            break;
                        }

                        // Add timeout for each handle
                        match tokio::time::timeout(
                            Duration::from_secs(SELL_TASK_HANDLE_TIMEOUT_SECS),
                            handle,
                        )
                        .await
                        {
                            Ok(task_result) => match task_result {
                                Ok(success) => {
                                    completed += 1;
                                    if success {
                                        successful += 1;
                                    }
                                }
                                Err(_) => {
                                    completed += 1;
                                }
                            },
                            Err(_) => {
                                completed += 1;
                            }
                        }
                    }

                    (completed, successful)
                },
            )
            .await;

            if let Ok((completed, successful)) = collection_result {
                if completed > 0 {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!(
                            "Sell operations completed: {}/{} successful",
                            successful, completed
                        ),
                    );
                }
            }
        }

        if check_shutdown_or_delay(
            &shutdown,
            Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS),
        )
        .await
        {
            log(
                LogTag::Trader,
                "INFO",
                "open positions monitor shutting down...",
            );
            break;
        }
    }
}
