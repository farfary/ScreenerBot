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
///
/// **Debug Features:**
/// - `DEBUG_FORCE_SELL_MODE` - Automatically sell positions after timeout
/// - `DEBUG_FORCE_BUY_MODE` - Automatically buy tokens on price drops (≥3% by default)
/// - Both debug modes can be independently enabled/disabled for testing

// =============================================================================
// TRADING SYSTEM CONFIGURATION CONSTANTS
// =============================================================================

// -----------------------------------------------------------------------------
// Core Trading Parameters
// -----------------------------------------------------------------------------

/// Maximum number of concurrent open positions
pub const MAX_OPEN_POSITIONS: usize = 1;

/// Trade size in SOL for each position
pub const TRADE_SIZE_SOL: f64 = 0.005;

/// Enable minimum profit threshold requirement before allowing sells
pub const MIN_PROFIT_THRESHOLD_ENABLED: bool = true;

/// Minimum profit threshold percentage (e.g., 5.0 for 5%, -5.0 for -5%)
/// Positions below this P&L will not be sold regardless of other exit conditions
pub const MIN_PROFIT_THRESHOLD_PERCENT: f64 = 5.0;

/// Time-based override: Allow sell decisions after this duration (hours)
/// Positions held longer than this can bypass profit threshold if in significant loss
/// This prevents positions from being held indefinitely when they're clearly failing
pub const TIME_OVERRIDE_DURATION_HOURS: f64 = 7.0 * 24.0;

/// Loss threshold for time-based override (negative percentage, e.g., -20.0 for -20%)
/// Positions with losses worse than this threshold can bypass profit requirements after time override
/// This allows cutting losses on positions that have been failing for extended periods
pub const TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT: f64 = -40.0;

pub const PROFIT_EXTRA_NEEDED_SOL: f64 = 0.00005;

/// ================= Slippage Configuration (Exit) =================
/// Unified slippage & proceeds policy
/// Naming pattern:
///   SLIPPAGE_QUOTE_DEFAULT_PCT        -> default quote slippage tolerance for routers
///   SLIPPAGE_EXIT_PROFIT_SHORTFALL_PCT -> max allowed shortfall vs required proceeds (profit exits)
///   SLIPPAGE_EXIT_LOSS_SHORTFALL_PCT   -> max allowed shortfall (loss / emergency exits)
///   SLIPPAGE_EXIT_RETRY_STEPS_PCT      -> progressive retry slippage attempts (filtered by shortfall caps)
///
/// ONLY these constants should be referenced by other modules; duplicates elsewhere should be removed.
pub const SLIPPAGE_QUOTE_DEFAULT_PCT: f64 = 5.0;
pub const SLIPPAGE_EXIT_PROFIT_SHORTFALL_PCT: f64 = 8.0; // Increased from 3.0% - profit exits need more flexibility
pub const SLIPPAGE_EXIT_LOSS_SHORTFALL_PCT: f64 = 15.0; // Increased from 12.0% - stop losses need maximum flexibility
pub const SLIPPAGE_EXIT_RETRY_STEPS_PCT: &[f64] = &[3.0, 5.0, 8.0, 12.0, 15.0]; // Added 15.0% for maximum flexibility

pub const MAX_PROFIT_EXIT_SLIPPAGE_PCT: f64 = SLIPPAGE_EXIT_PROFIT_SHORTFALL_PCT;
pub const MAX_LOSS_EXIT_SLIPPAGE_PCT: f64 = SLIPPAGE_EXIT_LOSS_SHORTFALL_PCT;

// -----------------------------------------------------------------------------
// Debug Mode Configuration
// -----------------------------------------------------------------------------

/// Debug mode: Force sell all positions after a timeout (for testing)
pub const DEBUG_FORCE_SELL_MODE: bool = true;

/// Debug mode: Force sell timeout in seconds
pub const DEBUG_FORCE_SELL_TIMEOUT_SECS: f64 = 45.0;

/// Debug mode: Force buy tokens when they have a simple price drop (for testing)
pub const DEBUG_FORCE_BUY_MODE: bool = true;

/// Debug mode: Price drop threshold percentage to trigger force buy (e.g., 3.0 for 3% drop)
pub const DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT: f64 = 0.5;

/// Enable position-aware DexScreener API caching
/// When enabled: Tokens with open positions never make API calls, always use cached data
/// When disabled: Normal API behavior for all tokens
pub const ENABLE_POSITION_AWARE_DEXSCREENER_CACHE: bool = true;

// -----------------------------------------------------------------------------
// Position Timing Configuration - Improved for longer holding
// -----------------------------------------------------------------------------

/// Per-token re-entry cooldown after closing a position (minutes) - prevents immediate re-buy of same token
/// This is applied in apply_cooldown_filter() and is separate from:
/// - Global position open cooldown (5s between any opens) - in positions.rs
/// - Frozen account cooldowns (account-specific) - in positions.rs
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 6 * 60; // 24 hours

// -----------------------------------------------------------------------------
// Trading Logic Configuration
// -----------------------------------------------------------------------------
// Monitoring & Display Configuration
// -----------------------------------------------------------------------------

/// New entry signals check interval (seconds) - optimized for fastest price checking
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;

/// Open positions monitoring interval (seconds) - maximum priority price checking every 2 seconds for faster profit capture
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 2;

// -----------------------------------------------------------------------------
// Task Timeout Configuration
// -----------------------------------------------------------------------------

/// Semaphore acquire timeout for token processing tasks (seconds) - reduced for faster failure detection
pub const SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 60;

/// Individual token check task timeout (seconds)
/// Reduced to prevent individual tasks from blocking the cycle
pub const TOKEN_CHECK_TASK_TIMEOUT_SECS: u64 = 20;

/// Price cache lock acquire timeout (milliseconds)
pub const PRICE_CACHE_LOCK_TIMEOUT_MS: u64 = 2000;

/// Task collection timeout for concurrent operations (seconds)
/// Reduced to prevent cycles from taking longer than the 3s interval
pub const TASK_COLLECTION_TIMEOUT_SECS: u64 = 30;

/// Token check result collection timeout (seconds)
/// Reduced to prevent the 116s cycle timeout issue
pub const TOKEN_CHECK_COLLECTION_TIMEOUT_SECS: u64 = 30;

/// Individual token check handle timeout (seconds)
/// Reduced to match shorter collection timeout
pub const TOKEN_CHECK_HANDLE_TIMEOUT_SECS: u64 = 25;

/// Sell operations collection timeout (seconds) - must accommodate multiple 3-min operations
pub const SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS: u64 = 240;

/// Individual sell operation timeout (seconds)
/// Now using step-based timeout detection instead of total operation timeout
pub const SELL_OPERATION_SMART_TIMEOUT_SECS: u64 = 600; // 10 minutes total allowance for complex operations

/// Sell semaphore acquire timeout (seconds) - increased for safety
pub const SELL_SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 30;

/// Individual sell task handle timeout (seconds) - must be longer than operation timeout
pub const SELL_TASK_HANDLE_TIMEOUT_SECS: u64 = 200;

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
// Concurrency Configuration
// -----------------------------------------------------------------------------

/// Number of concurrent token checks during entry scanning
/// Reduced from 24 to prevent overwhelming pool services and API endpoints
/// This prevents the 116s cycle timeout issue by reducing service contention
pub const ENTRY_CHECK_CONCURRENCY: usize = 4; // Reduced from 24 to fix performance

use crate::global::is_debug_trader_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::{ get_pool_price, PriceResult };
use crate::positions_lib::calculate_position_pnl;
use crate::tokens::{ cache::TokenDatabase, get_all_tokens_by_liquidity, Token };
use crate::utils::{ check_shutdown_or_delay, safe_read_lock, safe_write_lock, debug_trader_log };

use crate::entry::get_profit_target;

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use chrono::{ Duration as ChronoDuration, Utc };
use futures;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Notify;

use crate::positions_db;

// =============================================================================
// ERROR HANDLING UTILITIES
// =============================================================================

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: tracks critical trading operations in progress to prevent force shutdown
pub static CRITICAL_OPERATIONS_IN_PROGRESS: Lazy<Arc<std::sync::atomic::AtomicUsize>> = Lazy::new(||
    Arc::new(std::sync::atomic::AtomicUsize::new(0))
);

/// Global tracker: number of buy operations currently in-flight (reserved but not yet reflected in open positions)

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
    pub entry_check_count: usize,
    pub pool_type: Option<String>,
    pub pool_address: Option<String>,
    pub pool_price_sol: Option<f64>,
    pub reserve_sol: Option<f64>,
    pub reserve_token: Option<f64>,
}

/// Global token tracking state
pub static TOKEN_CHECK_TRACKER: Lazy<
    Arc<std::sync::RwLock<HashMap<String, TokenCheckInfo>>>
> = Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

// =============================================================================
// POSITION SELL DECISION CACHE AND RETRY SYSTEM
// =============================================================================

/// Represents a cached decision to sell a position with retry tracking
#[derive(Clone, Debug)]
pub struct SellDecisionInfo {
    pub position_id: String,
    pub mint: String,
    pub symbol: String,
    pub decision_reason: String,
    pub decision_time: Instant,
    pub first_attempt_time: Option<Instant>,
    pub last_attempt_time: Option<Instant>,
    pub attempt_count: u32,
    pub next_retry_time: Instant,
    pub last_error: Option<String>,
    pub max_retries: u32,
    pub is_emergency_sell: bool, // High priority sells (stop loss, etc.)
}

impl SellDecisionInfo {
    /// Create new sell decision
    pub fn new(
        position_id: String,
        mint: String,
        symbol: String,
        reason: String,
        is_emergency: bool
    ) -> Self {
        let now = Instant::now();
        Self {
            position_id,
            mint,
            symbol,
            decision_reason: reason,
            decision_time: now,
            first_attempt_time: None,
            last_attempt_time: None,
            attempt_count: 0,
            next_retry_time: now, // Can attempt immediately
            last_error: None,
            max_retries: if is_emergency {
                20
            } else {
                15
            }, // Many more retries with smart timing
            is_emergency_sell: is_emergency,
        }
    }

    /// Check if this decision is ready for retry
    pub fn can_retry(&self) -> bool {
        if self.attempt_count >= self.max_retries {
            return false;
        }
        Instant::now() >= self.next_retry_time
    }

    /// Update retry timing after failed attempt with DYNAMIC strategy for trading
    pub fn mark_attempt_failed(&mut self, error: String) {
        let now = Instant::now();

        if self.first_attempt_time.is_none() {
            self.first_attempt_time = Some(now);
        }

        self.last_attempt_time = Some(now);
        self.attempt_count += 1;
        self.last_error = Some(error);

        // DYNAMIC RETRY STRATEGY: 10 fast attempts, then intelligent backoff
        use rand::Rng;
        let delay_secs = if self.attempt_count <= 10 {
            // First 10 attempts: 5-10 seconds (randomized to avoid timing patterns)
            rand::thread_rng().gen_range(5..=10)
        } else {
            // After 10 fast attempts: Dynamic backoff with min/max bounds
            let (min_delay, max_delay) = if self.is_emergency_sell {
                (15, 120) // Emergency: 15 seconds to 2 minutes
            } else {
                (30, 300) // Normal: 30 seconds to 5 minutes
            };

            // Progressive backoff: starts at min, grows to max
            let backoff_attempts = self.attempt_count - 10; // Attempts beyond the fast phase
            let max_progression = 8.0; // After 8 backoff attempts, stay at max
            let progression = ((backoff_attempts as f64) / max_progression).min(1.0); // 0.0 to 1.0

            // Interpolate between min and max with randomization (±20%)
            let target_delay = (min_delay as f64) + progression * ((max_delay - min_delay) as f64);
            let randomized_delay = target_delay * rand::thread_rng().gen_range(0.8..=1.2);

            randomized_delay.round() as u64
        };

        self.next_retry_time = now + Duration::from_secs(delay_secs);
    }

    /// Check if this decision is stale (older than configured time)
    pub fn is_stale(&self) -> bool {
        let max_age_secs = if self.is_emergency_sell {
            3600 // Emergency sells valid for 1 hour
        } else {
            1800 // Normal sells valid for 30 minutes
        };

        Instant::now().duration_since(self.decision_time).as_secs() > max_age_secs
    }

    /// Get human-readable status for logging
    pub fn status_string(&self) -> String {
        let age_secs = Instant::now().duration_since(self.decision_time).as_secs();
        let next_retry_in = if self.next_retry_time > Instant::now() {
            self.next_retry_time.duration_since(Instant::now()).as_secs()
        } else {
            0
        };

        format!(
            "Decision: {} | Age: {}s | Attempts: {}/{} | Next retry: {}s | Emergency: {}",
            self.decision_reason,
            age_secs,
            self.attempt_count,
            self.max_retries,
            next_retry_in,
            self.is_emergency_sell
        )
    }

    /// Demo the new retry schedule (for testing/debugging)
    pub fn demo_retry_schedule(is_emergency: bool) -> String {
        let mut output = Vec::new();
        output.push(format!("🚀 NEW DYNAMIC RETRY SCHEDULE (Emergency: {})", is_emergency));
        output.push("".to_string());

        // Fast phase (10 attempts)
        output.push("📈 FAST PHASE - First 10 attempts: 5-10 seconds each".to_string());
        for i in 1..=10 {
            output.push(format!("  Attempt {}: 5-10 seconds", i));
        }

        output.push("".to_string());

        // Dynamic backoff phase
        let (min_delay, max_delay) = if is_emergency { (15, 120) } else { (30, 300) };
        output.push(
            format!("⚖️  DYNAMIC BACKOFF - Attempts 11+: {}-{} seconds", min_delay, max_delay)
        );

        // Show progression
        for backoff_attempt in 0..8 {
            let attempt_num = 11 + backoff_attempt;
            let progression = ((backoff_attempt as f64) / 8.0).min(1.0);
            let target_delay = (min_delay as f64) + progression * ((max_delay - min_delay) as f64);
            output.push(format!("  Attempt {}: ~{:.0} seconds (±20%)", attempt_num, target_delay));
        }

        let max_retries = if is_emergency { 20 } else { 15 };
        output.push(format!("  Attempts 19-{}: ~{} seconds (±20%)", max_retries, max_delay));

        output.push("".to_string());
        output.push(format!("🎯 Total max attempts: {}", max_retries));

        output.join("\n")
    }

    /// Preview the retry schedule for debugging (static method)
    pub fn preview_retry_schedule(is_emergency: bool) -> String {
        let mut schedule = Vec::new();
        for attempt in 1..=8 {
            let delay_secs = match attempt {
                1 => {
                    if is_emergency { 2 } else { 3 }
                }
                2 => {
                    if is_emergency { 5 } else { 8 }
                }
                3 => {
                    if is_emergency { 15 } else { 20 }
                }
                4 => {
                    if is_emergency { 30 } else { 45 }
                }
                5 => {
                    if is_emergency { 60 } else { 90 }
                }
                6 => {
                    if is_emergency { 120 } else { 180 }
                }
                7 => {
                    if is_emergency { 180 } else { 300 }
                }
                _ => {
                    if is_emergency { 300 } else { 600 }
                }
            };
            schedule.push(format!("Attempt {}: {}s", attempt, delay_secs));
        }
        format!(
            "Retry schedule for {} sells:\n{}",
            if is_emergency {
                "EMERGENCY"
            } else {
                "NORMAL"
            },
            schedule.join(", ")
        )
    }
}

/// Global cache for sell decisions awaiting execution/retry
pub static SELL_DECISION_CACHE: Lazy<
    Arc<std::sync::RwLock<HashMap<String, SellDecisionInfo>>>
> = Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

/// Add a position to the sell decision cache
pub fn cache_sell_decision(
    position_id: &str,
    mint: &str,
    symbol: &str,
    reason: &str,
    is_emergency: bool
) {
    if let Some(mut cache) = safe_write_lock(&SELL_DECISION_CACHE, "cache_sell_decision") {
        let decision = SellDecisionInfo::new(
            position_id.to_string(),
            mint.to_string(),
            symbol.to_string(),
            reason.to_string(),
            is_emergency
        );

        cache.insert(position_id.to_string(), decision);

        // Show the new dynamic retry schedule
        let retry_info = if is_emergency {
            "10x fast (5-10s), then 15s→120s dynamic backoff, max 20 attempts"
        } else {
            "10x fast (5-10s), then 30s→300s dynamic backoff, max 15 attempts"
        };

        log(
            LogTag::Trader,
            "SELL_DECISION_CACHED",
            &format!(
                "🎯 Cached {} sell for {}: {} | Strategy: {}",
                if is_emergency {
                    "EMERGENCY"
                } else {
                    "NORMAL"
                },
                symbol,
                reason,
                retry_info
            )
        );
    }
}

/// Remove a position from sell decision cache (after successful sale)
pub fn remove_sell_decision(position_id: &str) -> bool {
    if let Some(mut cache) = safe_write_lock(&SELL_DECISION_CACHE, "remove_sell_decision") {
        if let Some(decision) = cache.remove(position_id) {
            log(
                LogTag::Trader,
                "SELL_DECISION_COMPLETED",
                &format!(
                    "✅ Completed sell decision for position {}: {} after {} attempts",
                    position_id,
                    decision.decision_reason,
                    decision.attempt_count
                )
            );
            return true;
        }
    }
    false
}

/// Mark a sell attempt as failed and update retry timing
pub fn mark_sell_attempt_failed(position_id: &str, error: &str) {
    if let Some(mut cache) = safe_write_lock(&SELL_DECISION_CACHE, "mark_sell_attempt_failed") {
        if let Some(decision) = cache.get_mut(position_id) {
            decision.mark_attempt_failed(error.to_string());

            log(
                LogTag::Trader,
                "SELL_ATTEMPT_FAILED",
                &format!(
                    "❌ Sell attempt failed for position {}: {} | {}",
                    position_id,
                    error,
                    decision.status_string()
                )
            );
        }
    }
}

/// Get positions ready for sell retry
pub fn get_positions_ready_for_sell_retry() -> Vec<SellDecisionInfo> {
    if let Some(cache) = safe_read_lock(&SELL_DECISION_CACHE, "get_positions_ready_for_sell_retry") {
        cache
            .values()
            .filter(|decision| decision.can_retry() && !decision.is_stale())
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

/// Clean up stale sell decisions
pub fn cleanup_stale_sell_decisions() -> usize {
    if let Some(mut cache) = safe_write_lock(&SELL_DECISION_CACHE, "cleanup_stale_sell_decisions") {
        let before_count = cache.len();
        cache.retain(|_, decision| !decision.is_stale());
        let removed_count = before_count - cache.len();

        if removed_count > 0 {
            log(
                LogTag::Trader,
                "SELL_DECISIONS_CLEANUP",
                &format!(
                    "🧹 Cleaned up {} stale sell decisions ({} -> {})",
                    removed_count,
                    before_count,
                    cache.len()
                )
            );
        }

        removed_count
    } else {
        0
    }
}

/// Get sell decision cache status for debugging
pub fn get_sell_decision_cache_status() -> String {
    if let Some(cache) = safe_read_lock(&SELL_DECISION_CACHE, "get_sell_decision_cache_status") {
        if cache.is_empty() {
            return "Sell Decision Cache: Empty".to_string();
        }

        let total = cache.len();
        let emergency_count = cache
            .values()
            .filter(|d| d.is_emergency_sell)
            .count();
        let ready_for_retry = cache
            .values()
            .filter(|d| d.can_retry() && !d.is_stale())
            .count();
        let exhausted_retries = cache
            .values()
            .filter(|d| d.attempt_count >= d.max_retries)
            .count();
        let stale_count = cache
            .values()
            .filter(|d| d.is_stale())
            .count();

        let mut status = format!(
            "Sell Decision Cache: {} total ({} emergency, {} ready for retry, {} exhausted, {} stale)\n",
            total,
            emergency_count,
            ready_for_retry,
            exhausted_retries,
            stale_count
        );

        // Show details for first few entries
        for (i, (pos_id, decision)) in cache.iter().enumerate() {
            if i >= 5 {
                // Limit to first 5 for readability
                if total > 5 {
                    status.push_str(&format!("... and {} more\n", total - 5));
                }
                break;
            }

            status.push_str(
                &format!(
                    "  {}: {} | {}\n",
                    pos_id,
                    decision.mint.get(..8).unwrap_or(&decision.mint),
                    decision.status_string()
                )
            );
        }

        status
    } else {
        "Sell Decision Cache: Lock error".to_string()
    }
}

// =============================================================================
// PER-TOKEN RE-ENTRY COOLDOWN CACHE
// =============================================================================
// Caches recently closed position mints to prevent immediate re-entry
// This is separate from global position cooldowns and frozen account cooldowns

#[derive(Clone)]
struct RecentlyClosedCache {
    mints: HashSet<String>,
    cached_at: Instant,
}

impl RecentlyClosedCache {
    fn is_valid(&self, ttl_secs: u64) -> bool {
        self.cached_at.elapsed().as_secs() < ttl_secs
    }
}

static RECENTLY_CLOSED_CACHE: Lazy<Arc<StdRwLock<Option<RecentlyClosedCache>>>> = Lazy::new(||
    Arc::new(StdRwLock::new(None))
);

const RECENTLY_CLOSED_TTL_SECS: u64 = 60; // refresh every minute

async fn get_recently_closed_mints_set() -> HashSet<String> {
    // Try cache first
    if let Some(cache_guard) = safe_read_lock(&RECENTLY_CLOSED_CACHE, "recently_closed_cache_read") {
        if let Some(cache) = cache_guard.as_ref() {
            if cache.is_valid(RECENTLY_CLOSED_TTL_SECS) {
                return cache.mints.clone();
            }
        }
    }

    // Load from DB: fetch closed positions and keep those within cooldown
    let now = Utc::now();
    let cutoff = now - ChronoDuration::minutes(POSITION_CLOSE_COOLDOWN_MINUTES);
    let mut mints: HashSet<String> = HashSet::new();

    match positions_db::get_closed_positions().await {
        Ok(positions) => {
            for p in positions.into_iter() {
                if let Some(exit_time) = p.exit_time {
                    if exit_time > cutoff {
                        mints.insert(p.mint);
                    }
                }
            }
        }
        Err(e) => {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Failed to load recently closed positions for cooldown filter: {}", e)
            );
        }
    }

    // Update cache
    if
        let Some(mut cache_guard) = safe_write_lock(
            &RECENTLY_CLOSED_CACHE,
            "recently_closed_cache_write"
        )
    {
        *cache_guard = Some(RecentlyClosedCache {
            mints: mints.clone(),
            cached_at: Instant::now(),
        });
    }

    mints
}

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
                position.symbol,
                position_age_secs,
                DEBUG_FORCE_SELL_TIMEOUT_SECS
            )
        );
        return true;
    }

    false
}

/// Debug function: Check if a token should be force-bought due to simple price drop
pub fn should_debug_force_buy(
    current_price: f64,
    previous_price: Option<f64>,
    symbol: &str
) -> bool {
    if !DEBUG_FORCE_BUY_MODE {
        return false;
    }

    if let Some(prev_price) = previous_price {
        if prev_price > 0.0 && current_price > 0.0 {
            let drop_percent = ((prev_price - current_price) / prev_price) * 100.0;

            if drop_percent >= DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT {
                log(
                    LogTag::Trader,
                    "DEBUG_FORCE_BUY",
                    &format!(
                        "🚨 DEBUG MODE: Force buying {} - {:.2}% drop detected (threshold: {:.1}%)",
                        symbol,
                        drop_percent,
                        DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT
                    )
                );
                return true;
            }
        }
    }

    false
}

/// Update token tracking after checking a token
pub fn update_token_check_info(
    mint: &str,
    current_price: Option<f64>,
    had_drop: bool,
    entry_checked: bool,
    _pool: Option<f64> // Changed from PriceResult to simple f64
) {
    let mut tracker = TOKEN_CHECK_TRACKER.write().unwrap();
    let info = tracker.entry(mint.to_string()).or_insert(TokenCheckInfo {
        last_check_time: Instant::now(),
        last_price: None,
        check_count: 0,
        entry_check_count: 0,
        had_recent_drop: false,
        pool_type: None,
        pool_address: None,
        pool_price_sol: None,
        reserve_sol: None,
        reserve_token: None,
    });

    info.last_check_time = Instant::now();
    if let Some(price) = current_price {
        info.last_price = Some(price);
    }
    info.check_count += 1;
    info.had_recent_drop = had_drop;
    if entry_checked {
        info.entry_check_count += 1;
    }
}

async fn ensure_watchlist_on_price_fail(_mint: &str, _symbol: &str, _reason: &str) {
    // Watchlist functionality removed
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
    // Only do this cleanup every ~10 calls to reduce lock contention
    static CLEANUP_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let should_cleanup = {
        let count = CLEANUP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        count % 10 == 0
    };

    if should_cleanup {
        drop(tracker);
        let mut tracker_write = TOKEN_CHECK_TRACKER.write().unwrap();
        let before_count = tracker_write.len();
        tracker_write.retain(|_, info| now.duration_since(info.last_check_time).as_secs() < 600);
        let after_count = tracker_write.len();

        if is_debug_trader_enabled() && before_count != after_count {
            log(
                LogTag::Trader,
                "TRACKER_CLEANUP",
                &format!(
                    "🧹 Cleaned up {} stale token tracking entries ({} -> {})",
                    before_count - after_count,
                    before_count,
                    after_count
                )
            );
        }
    } else {
        drop(tracker);
    }

    tokens
}

/// Apply per-token re-entry cooldown filter to exclude recently closed positions
/// This is separate from:
/// - Global position open cooldown (5s between any position opens) - handled in positions.rs
/// - Frozen account cooldowns (account-specific freezes) - handled in positions.rs
/// This must be called on every cycle with fresh data, never cached
async fn apply_cooldown_filter(tokens: Vec<Token>) -> Vec<Token> {
    let recently_closed_mints = get_recently_closed_mints_set().await;
    if recently_closed_mints.is_empty() {
        return tokens;
    }

    let before_cooldown = tokens.len();
    let mut removed: Vec<String> = Vec::new();
    let tokens_after_cooldown: Vec<Token> = tokens
        .into_iter()
        .filter(|t| {
            let exclude = recently_closed_mints.contains(&t.mint);
            if exclude && removed.len() < 10 {
                removed.push(t.mint.clone());
            }
            !exclude
        })
        .collect();

    let removed_for_cooldown = before_cooldown.saturating_sub(tokens_after_cooldown.len());
    if removed_for_cooldown > 0 {
        log(
            LogTag::Trader,
            "COOLDOWN_FILTER",
            &format!(
                "⏳ Excluded {} tokens (sample: [{}]) within {}m cooldown; {} remain",
                removed_for_cooldown,
                removed.join(","),
                POSITION_CLOSE_COOLDOWN_MINUTES,
                tokens_after_cooldown.len()
            )
        );
    }
    tokens_after_cooldown
}

/// Return human-readable status of current cooldown set for diagnostics
pub async fn get_cooldown_status(sample: usize) -> String {
    let recently_closed_mints = get_recently_closed_mints_set().await;
    if recently_closed_mints.is_empty() {
        return "Cooldown: none".to_string();
    }
    let mut mints: Vec<String> = recently_closed_mints.into_iter().collect();
    mints.sort();
    let total = mints.len();
    let sample_list = mints.into_iter().take(sample).collect::<Vec<_>>().join(",");
    format!(
        "Cooldown: {} mints (showing {}): [{}] (window={}m)",
        total,
        sample_list
            .split(',')
            .filter(|s| !s.is_empty())
            .count(),
        sample_list,
        POSITION_CLOSE_COOLDOWN_MINUTES
    )
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    log(LogTag::Trader, "STARTUP", "🚀 Starting monitor_new_entries task");

    'outer: loop {
        // Check for shutdown at the very beginning of each loop iteration
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
            log(LogTag::Trader, "INFO", "✅ New entries monitor shutdown requested at loop start");
            break 'outer;
        }

        // CRITICAL: Wait for position recalculation to complete before starting any trading operations
        if
            !crate::global::POSITION_RECALCULATION_COMPLETE.load(
                std::sync::atomic::Ordering::SeqCst
            )
        {
            log(LogTag::Trader, "STARTUP", "⏳ Waiting for position recalculation to complete...");

            // Use shutdown-aware sleep instead of fixed sleep
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(1)).await {
                log(
                    LogTag::Trader,
                    "INFO",
                    "✅ New entries monitor shutdown during position recalc wait"
                );
                break 'outer;
            }
            continue;
        }

        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        // Check for shutdown before starting main processing
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
            log(LogTag::Trader, "INFO", "✅ New entries monitor shutdown before token processing");
            break 'outer;
        }

        // Prepare tokens for trading (fetch, sort, filter)
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "CYCLE_START",
                &format!(
                    "🔄 Starting token preparation cycle at {:.3}s",
                    cycle_start.elapsed().as_secs_f32()
                )
            );
        }

        // Get available tokens directly from pool interface
        let available_mints = crate::pools::get_available_tokens();

        log(
            LogTag::Trader,
            "CYCLE_PREPARED",
            &format!(
                "✅ Got {} available tokens from pool interface in {:.3}s",
                available_mints.len(),
                cycle_start.elapsed().as_secs_f32()
            )
        );

        // Get price info for all available tokens
        let mut price_infos = Vec::new();
        let mut mint_price_pairs = Vec::new();
        for mint in available_mints {
            if let Some(price_info) = get_pool_price(&mint) {
                price_infos.push(price_info.clone());
                mint_price_pairs.push((mint, price_info));
            }
        }

        if is_debug_trader_enabled() && !mint_price_pairs.is_empty() {
            log(
                LogTag::Trader,
                "DEBUG_TOKENS_PREPARED",
                &format!(
                    "🔍 First 5 available tokens: [{}]",
                    mint_price_pairs
                        .iter()
                        .take(5)
                        .map(|(mint, p)| format!("{}({:.4}SOL)", &mint[..8], p.price_sol))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            );
        }

        // Early return if no tokens to process
        if price_infos.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "No tokens to process after {:.3}s, skipping token checking cycle",
                    cycle_start.elapsed().as_secs_f32()
                )
            );

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
            } else {
                Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "CYCLE_WAIT",
                    &format!(
                        "⏸️ Waiting {:.1}s before next cycle (cycle took {:.3}s)",
                        wait_time.as_secs_f32(),
                        cycle_duration.as_secs_f32()
                    )
                );
            }

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break;
            }
            continue;
        }

        // Limit concurrent token checks to avoid overwhelming services
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(ENTRY_CHECK_CONCURRENCY));

        // Process all available tokens in parallel
        let total_tokens = price_infos.len();

        // Per-cycle aggregation counters (atomic to update from tasks)
        let price_available_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let price_unavailable_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG_TOKEN_PROCESSING",
                &format!(
                    "📋 Processing all {} tokens:\n  \
                     - Semaphore limit: {} concurrent checks\n  \
                     - Task timeout: {}s per token\n  \
                     - First 10 tokens: [{}]",
                    total_tokens,
                    ENTRY_CHECK_CONCURRENCY,
                    TOKEN_CHECK_TASK_TIMEOUT_SECS,
                    price_infos
                        .iter()
                        .take(10)
                        .map(|p| p.mint.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            );
        }

        for price_info in price_infos.iter() {
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
            let price_info = price_info.clone();
            let shutdown_clone = shutdown.clone();
            let price_available_count = price_available_count.clone();
            let price_unavailable_count = price_unavailable_count.clone();
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
                    return;
                }

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(TOKEN_CHECK_TASK_TIMEOUT_SECS), async {
                        // Get current price from PriceResult
                        let current_price = if
                            price_info.price_sol > 0.0 &&
                            price_info.price_sol.is_finite()
                        {
                            price_info.price_sol
                        } else {
                            if is_debug_trader_enabled() {
                                log(
                                    LogTag::Trader,
                                    "PRICE_UNAVAILABLE",
                                    &format!("❌ No price available for {}", price_info.mint)
                                );
                            }
                            price_unavailable_count.fetch_add(
                                1,
                                std::sync::atomic::Ordering::Relaxed
                            );
                            return;
                        };

                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "PRICE_CHECK",
                                &format!("💰 {} price: {:.9} SOL", price_info.mint, current_price)
                            );
                        }
                        price_available_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        // Call should_buy with PriceResult (price history fetched internally)
                        let (approved, confidence, reason) = crate::entry::should_buy(
                            &price_info
                        ).await;

                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "ENTRY_CHECK",
                                &format!(
                                    "🔍 Checking entry criteria for {} at {:.9} SOL",
                                    price_info.mint,
                                    current_price
                                )
                            );
                        }

                        // Check for debug force buy (overrides normal entry logic)
                        let mut force_buy_triggered = false;
                        if DEBUG_FORCE_BUY_MODE {
                            // Get previous price from token tracker
                            let previous_price = {
                                if let Ok(tracker) = TOKEN_CHECK_TRACKER.read() {
                                    tracker.get(&price_info.mint).and_then(|info| info.last_price)
                                } else {
                                    None
                                }
                            };

                            // Check if we have available position slots
                            let open_positions_count =
                                crate::positions::get_open_positions_count().await;
                            let has_position_space = open_positions_count < MAX_OPEN_POSITIONS;

                            if
                                has_position_space &&
                                should_debug_force_buy(
                                    current_price,
                                    previous_price,
                                    &price_info.mint
                                )
                            {
                                force_buy_triggered = true;
                                log(
                                    LogTag::Trader,
                                    "DEBUG_FORCE_BUY_TRIGGERED",
                                    &format!(
                                        "🚨 DEBUG FORCE BUY: Overriding normal entry logic for {} (positions: {}/{})",
                                        price_info.mint,
                                        open_positions_count,
                                        MAX_OPEN_POSITIONS
                                    )
                                );
                            }
                        }

                        // Use force buy or normal entry approval
                        if !approved && !force_buy_triggered {
                            // Update token tracking for unsuccessful check
                            update_token_check_info(
                                &price_info.mint,
                                Some(current_price),
                                false,
                                true,
                                Some(current_price)
                            );
                            return;
                        }

                        // Token approved for entry!
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "ENTRY_APPROVED",
                                &format!(
                                    "🚀 ENTRY APPROVED: {} at {:.9} SOL",
                                    &price_info.mint,
                                    current_price
                                )
                            );
                        }

                        // Open position directly with PriceResult
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "POSITION_OPENING",
                                &format!(
                                    "📈 Opening position for {} at {:.9} SOL (size: {} SOL)",
                                    price_info.mint,
                                    current_price,
                                    TRADE_SIZE_SOL
                                )
                            );
                        }

                        let position_start = std::time::Instant::now();
                        let position_result = crate::positions::open_position_direct(
                            &price_info.mint
                        ).await;

                        let position_duration = position_start.elapsed();

                        match &position_result {
                            Ok(position_id) => {
                                log(
                                    LogTag::Trader,
                                    "POSITION_OPENED",
                                    &format!(
                                        "✅ Successfully opened position {} for {} in {:.3}s",
                                        position_id,
                                        price_info.mint,
                                        position_duration.as_secs_f32()
                                    )
                                );

                                // Update token tracking for successful entry
                                update_token_check_info(
                                    &price_info.mint,
                                    Some(current_price),
                                    force_buy_triggered, // Mark as drop if force buy was triggered
                                    true,
                                    Some(current_price)
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Trader,
                                    "POSITION_FAILED",
                                    &format!(
                                        "❌ Failed to open position for {} after {:.3}s: {}",
                                        price_info.mint,
                                        position_duration.as_secs_f32(),
                                        e
                                    )
                                );

                                // Update token tracking for failed entry
                                update_token_check_info(
                                    &price_info.mint,
                                    Some(current_price),
                                    force_buy_triggered, // Mark as drop if force buy was triggered
                                    true,
                                    Some(current_price)
                                );
                            }
                        }

                        // Add to OHLCV watch list as open position for priority monitoring
                        if position_result.is_ok() {
                            if
                                let Ok(ohlcv_service) =
                                    crate::tokens::get_ohlcv_service_clone().await
                            {
                                ohlcv_service.add_to_watch_list(&price_info.mint, true).await; // true = open position
                            }
                        }
                    }).await
                {
                    Ok(_) => {}
                    Err(_) => {}
                }
            });

            handles.push(handle);
        }

        // Wait for tasks to finish with overall timeout (best-effort)
        let handles_count = handles.len();
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "COLLECT_START",
                &format!(
                    "⏳ Collecting {} token tasks with {}s overall timeout",
                    handles_count,
                    TOKEN_CHECK_COLLECTION_TIMEOUT_SECS
                )
            );
        }
        let collection_result = tokio::time::timeout(
            Duration::from_secs(TOKEN_CHECK_COLLECTION_TIMEOUT_SECS),
            async {
                for handle in handles {
                    if
                        check_shutdown_or_delay(
                            &shutdown,
                            Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS)
                        ).await
                    {
                        return;
                    }
                    let _ = tokio::time::timeout(
                        Duration::from_secs(TOKEN_CHECK_HANDLE_TIMEOUT_SECS),
                        handle
                    ).await;
                }
            }
        ).await;
        if collection_result.is_err() {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Token check collection timed out after {} seconds", TOKEN_CHECK_COLLECTION_TIMEOUT_SECS)
            );
        }

        // Add cycle summary logging
        if is_debug_trader_enabled() {
            let final_positions_count = crate::positions::get_open_positions_count().await;
            log(
                LogTag::Trader,
                "CYCLE_SUMMARY",
                &format!(
                    "🔄 Cycle summary: Processed {}/{} tokens → {} tasks spawned → Positions: {}/{}",
                    handles_count,
                    total_tokens,
                    handles_count,
                    final_positions_count,
                    MAX_OPEN_POSITIONS
                )
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
                    "⚠️ Token checking cycle took longer than interval: {:.3}s > {}s",
                    cycle_duration.as_secs_f32(),
                    ENTRY_MONITOR_INTERVAL_SECS
                )
            );
            Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
        } else {
            // Otherwise wait for the remaining interval time
            let remaining = Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration;
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "CYCLE_COMPLETE",
                    &format!(
                        "✅ Cycle completed in {:.3}s, waiting {:.1}s before next cycle",
                        cycle_duration.as_secs_f32(),
                        remaining.as_secs_f32()
                    )
                );
            }
            remaining
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
        // CRITICAL: Wait for position recalculation to complete before starting any position monitoring
        if
            !crate::global::POSITION_RECALCULATION_COMPLETE.load(
                std::sync::atomic::Ordering::SeqCst
            )
        {
            log(
                LogTag::Trader,
                "STARTUP",
                "⏳ Position monitor waiting for recalculation to complete..."
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
                    )
                );
            }

            if is_debug_trader_enabled() {
                debug_trader_log(
                    "PRIORITY_RESULT",
                    &format!(
                        "Pool service automatically handles price updates for {} open positions",
                        open_position_mints.len()
                    )
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
                )
            );
        }

        // Use efficient parallel price fetching - background service keeps prices fresh
        let price_futures: Vec<_> = open_positions_data
            .iter()
            .map(|pos| {
                let mint = pos.mint.clone();
                async move {
                    let price_result = if let Some(price_info) = get_pool_price(&mint) {
                        Some(price_info.price_sol)
                    } else {
                        None
                    };

                    // Extract price if available
                    if let Some(price) = price_result {
                        if price > 0.0 && price.is_finite() {
                            (mint, Some(price))
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
        let price_map: std::collections::HashMap<String, f64> = price_results
            .into_iter()
            .filter_map(|(mint, result_opt)| result_opt.map(|price| (mint, price)))
            .collect();

        // First, process any cached sell decisions that are ready for retry
        let retry_decisions = get_positions_ready_for_sell_retry();
        if !retry_decisions.is_empty() {
            log(
                LogTag::Trader,
                "SELL_RETRY_PROCESSING",
                &format!("Processing {} cached sell decisions for retry", retry_decisions.len())
            );

            for decision in retry_decisions {
                // Verify position still exists and is open
                if
                    let Some(position) = open_positions_data
                        .iter()
                        .find(|p| p.mint == decision.mint)
                {
                    // Get current price for this position
                    if let Some(current_price) = price_map.get(&position.mint) {
                        // Fetch full token from database
                        if
                            let Some(full_token) = crate::tokens::get_token_from_db(
                                &position.mint
                            ).await
                        {
                            log(
                                LogTag::Trader,
                                "SELL_RETRY",
                                &format!(
                                    "🔄 Retrying sell for {} ({}): {} - Attempt {}/{}",
                                    position.symbol,
                                    decision.position_id,
                                    decision.decision_reason,
                                    decision.attempt_count + 1,
                                    decision.max_retries
                                )
                            );

                            positions_to_close.push((
                                position.clone(),
                                full_token,
                                *current_price,
                                decision.decision_reason.clone(),
                                Some(decision.clone()), // Include decision info for tracking
                            ));
                        }
                    }
                } else {
                    remove_sell_decision(&decision.position_id);
                    log(
                        LogTag::Trader,
                        "SELL_DECISION_STALE",
                        &format!(
                            "Position {} no longer exists, removing cached sell decision",
                            decision.position_id
                        )
                    );
                }
            }
        }

        // Clean up stale sell decisions periodically
        cleanup_stale_sell_decisions();

        // Now process each position with async calls (mutex is released)
        for position in open_positions_data.into_iter() {
            let position = position; // local copy for calculations/logs

            // Get current price from our parallel fetch results
            if let Some(current_price) = price_map.get(&position.mint) {
                let current_price = *current_price;
                if current_price > 0.0 && current_price.is_finite() {
                    // Update position with current price
                    let _tracking_result = crate::positions::update_position_tracking(
                        &position.mint,
                        current_price,
                        &crate::pools::PriceResult::default() // Use default price result
                    ).await;

                    let now = Utc::now();

                    // Calculate P&L for logging and decision making
                    let (pnl_sol, pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(current_price)
                    ).await;

                    // Check debug force sell first
                    let debug_force_sell = should_debug_force_sell(&position);

                    // Calculate sell decision using the unified profit system
                    let should_exit_base =
                        debug_force_sell ||
                        crate::profit::should_sell(&position, current_price).await;

                    // Apply minimum profit threshold check if enabled
                    let should_exit = if MIN_PROFIT_THRESHOLD_ENABLED && !debug_force_sell {
                        // Check if position qualifies for time-based override
                        let position_age_hours =
                            (now.signed_duration_since(position.entry_time).num_seconds() as f64) /
                            3600.0;
                        let time_override_applies =
                            position_age_hours >= TIME_OVERRIDE_DURATION_HOURS &&
                            pnl_percent <= TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT;

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
                            (now.signed_duration_since(position.entry_time).num_seconds() as f64) /
                            3600.0;
                        let time_override_applies =
                            position_age_hours >= TIME_OVERRIDE_DURATION_HOURS &&
                            pnl_percent <= TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT;

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

                    // Check if we already have a cached sell decision for this position
                    let position_id = format!(
                        "{}_{}",
                        position.mint,
                        position.entry_time.timestamp()
                    );

                    if
                        let Some(cache_guard) = safe_read_lock(
                            &SELL_DECISION_CACHE,
                            "check_cached_decision"
                        )
                    {
                        if let Some(cached_decision) = cache_guard.get(&position_id) {
                            if !cached_decision.is_stale() {
                                // We already have a cached decision, it will be processed in the retry section
                                if is_debug_trader_enabled() {
                                    debug_trader_log(
                                        "SELL_CACHED",
                                        &format!(
                                            "{} | Cached sell decision exists: {} | {}",
                                            position.symbol,
                                            cached_decision.decision_reason,
                                            cached_decision.status_string()
                                        )
                                    );
                                }
                                continue;
                            }
                        }
                    }

                    if should_exit {
                        // Determine sell reason and urgency
                        let sell_reason = if debug_force_sell {
                            "Debug force sell".to_string()
                        } else {
                            format!(
                                "Trading decision: P&L {:.2}% ({:.6} SOL)",
                                pnl_percent,
                                pnl_sol
                            )
                        };

                        let is_emergency =
                            debug_force_sell ||
                            pnl_percent <= -20.0 || // Stop loss situations
                            pnl_percent >= 50.0; // High profit situations

                        // CRITICAL: Check pool availability before caching sell decision
                        let has_pool_availability = get_pool_price(&position.mint).is_some();

                        if !has_pool_availability {
                            if is_debug_trader_enabled() {
                                debug_trader_log(
                                    "SELL_POOL_UNAVAILABLE",
                                    &format!(
                                        "SKIPPING SELL for {} ({}): No pool available for trading",
                                        position.symbol,
                                        position.mint
                                    )
                                );
                            }
                            continue;
                        }

                        // Cache the sell decision instead of immediately processing
                        cache_sell_decision(
                            &position_id,
                            &position.mint,
                            &position.symbol,
                            &sell_reason,
                            is_emergency
                        );

                        // Fetch full token from database for immediate processing
                        let Some(full_token) = crate::tokens::get_token_from_db(
                            &position.mint
                        ).await else {
                            // If token not found in DB, remove the cached decision since we can't trade it
                            remove_sell_decision(&position_id);
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Token not found in DB for mint {} — removing cached sell decision",
                                    position.mint
                                )
                            );
                            continue;
                        };

                        log(
                            LogTag::Trader,
                            "SELL_DECISION",
                            &format!(
                                "Cached sell decision for {} ({}) - P&L: {:.2}% ({:.6} SOL) - {} (Emergency: {})",
                                position.symbol,
                                position.mint,
                                pnl_percent,
                                pnl_sol,
                                sell_reason,
                                is_emergency
                            )
                        );

                        // Add to immediate processing (first attempt)
                        positions_to_close.push((
                            position.clone(),
                            full_token,
                            current_price,
                            sell_reason.clone(),
                            None, // No cached decision yet (this is the first attempt)
                        ));
                    } else {
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "HOLD",
                                &format!(
                                    "Holding {} ({}) - P&L: {:.2}% ({:.6} SOL), Price: {:.9}",
                                    position.symbol,
                                    position.mint,
                                    pnl_percent,
                                    pnl_sol,
                                    current_price
                                )
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
                            "Invalid price for position monitoring: {} ({}) - Price = {:.9}",
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

        // Close positions that need to be closed concurrently
        if !positions_to_close.is_empty() {
            // Use a semaphore to limit concurrent sell transactions
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(5)); // Allow up to 5 concurrent sells for better performance

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (
                position,
                token,
                exit_price,
                sell_reason,
                cached_decision_opt,
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
                    Ok(Err(_)) | Err(_) => {
                        continue;
                    }
                };

                // Clone shutdown for use in the spawned sell task
                let shutdown_for_task = shutdown.clone();
                let cached_decision_for_task = cached_decision_opt.clone();

                // We already have the position from the analysis phase for logging only
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    // CRITICAL OPERATION PROTECTION - Prevent shutdown during sell
                    let _guard = CriticalOperationGuard::new(&format!("SELL_{}", token.symbol));

                    let position = position;
                    let token_symbol = token.symbol.clone();
                    let position_id = format!(
                        "{}_{}",
                        position.mint,
                        position.entry_time.timestamp()
                    );

                    // Check for shutdown before starting sell operation (non-blocking check)
                    let shutdown_check = tokio::time::timeout(
                        Duration::from_millis(SELL_OPERATION_SHUTDOWN_CHECK_MS),
                        shutdown_for_task.notified()
                    ).await;
                    if shutdown_check.is_ok() {
                        return (false, position_id, "Shutdown requested".to_string());
                    }

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(
                            Duration::from_secs(SELL_OPERATION_SMART_TIMEOUT_SECS),
                            async {
                                crate::positions
                                    ::close_position_direct(
                                        &position.mint,
                                        "Trading decision".to_string()
                                    ).await
                                    .map(|_| ())
                            }
                        ).await
                    {
                        Ok(Ok(())) => {
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!("Successfully closed position for {}", token_symbol)
                            );

                            // Remove successful sell decision from cache
                            remove_sell_decision(&position_id);

                            (true, position_id, "Success".to_string())
                        }
                        Ok(Err(e)) => {
                            let error_msg = format!(
                                "Failed to close position for {}: {}",
                                token_symbol,
                                e
                            );
                            log(LogTag::Trader, "ERROR", &error_msg);

                            (false, position_id, error_msg)
                        }
                        Err(_) => {
                            let error_msg =
                                format!("Sell operation for {} timed out", token_symbol);
                            log(LogTag::Trader, "ERROR", &error_msg);

                            (false, position_id, error_msg)
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
                        if
                            check_shutdown_or_delay(
                                &shutdown,
                                Duration::from_millis(COLLECTION_SHUTDOWN_CHECK_MS)
                            ).await
                        {
                            break;
                        }

                        // Add timeout for each handle
                        match
                            tokio::time::timeout(
                                Duration::from_secs(SELL_TASK_HANDLE_TIMEOUT_SECS),
                                handle
                            ).await
                        {
                            Ok(task_result) =>
                                match task_result {
                                    Ok((success, position_id, message)) => {
                                        completed += 1;
                                        if success {
                                            successful += 1;
                                        } else {
                                            // Mark failed sell attempt for retry
                                            mark_sell_attempt_failed(&position_id, &message);
                                            log(
                                                LogTag::Trader,
                                                "WARN",
                                                &format!(
                                                    "Sell attempt failed for position {}: {}",
                                                    position_id,
                                                    message
                                                )
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        completed += 1;
                                        log(
                                            LogTag::Trader,
                                            "ERROR",
                                            &format!("Sell task panicked: {}", e)
                                        );
                                    }
                                }
                            Err(_) => {
                                completed += 1;
                            }
                        }
                    }

                    (completed, successful)
                }
            ).await;

            if let Ok((completed, successful)) = collection_result {
                if completed > 0 {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!(
                            "Sell operations completed: {}/{} successful",
                            successful,
                            completed
                        )
                    );
                }
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
