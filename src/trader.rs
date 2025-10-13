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
/// - `debug_force_sell_mode` - Automatically sell positions after timeout
/// - `debug_force_buy_mode` - Automatically buy tokens on price drops (≥3% by default)
/// - Both debug modes can be independently enabled/disabled for testing
///
/// **Configuration:**
/// All trading parameters are now loaded from the centralized config system.
/// See `src/config/schemas.rs` for TraderConfig structure and defaults.
// NOTE: All trading configuration parameters are now in src/config/schemas.rs
// Access via: with_config(|cfg| cfg.trader.parameter_name)
// =============================================================================
use crate::config::{update_config_section, with_config};
use crate::global::is_debug_trader_enabled;
use crate::logger::{log, LogTag};
use crate::pools::{get_pool_price, PriceResult};
use crate::positions::calculate_position_pnl;
use crate::positions::is_open_position;
use crate::tokens::{
    database::TokenDatabase, get_all_tokens_by_liquidity, store::get_global_token_store, Token,
};
use crate::utils::{check_shutdown_or_delay, debug_trader_log, safe_read_lock, safe_write_lock};

use crate::entry::get_profit_target;

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use chrono::{Duration as ChronoDuration, Utc};
use futures;
use futures::FutureExt; // for now_or_never on shutdown future
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Notify;

use crate::positions::db;

// =============================================================================
// ERROR HANDLING UTILITIES
// =============================================================================

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: tracks critical trading operations in progress to prevent force shutdown
pub static CRITICAL_OPERATIONS_IN_PROGRESS: Lazy<Arc<std::sync::atomic::AtomicUsize>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicUsize::new(0)));

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
pub static TOKEN_CHECK_TRACKER: Lazy<Arc<std::sync::RwLock<HashMap<String, TokenCheckInfo>>>> =
    Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

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
        is_emergency: bool,
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
            max_retries: if is_emergency { 20 } else { 15 }, // Many more retries with smart timing
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
            self.next_retry_time
                .duration_since(Instant::now())
                .as_secs()
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
        output.push(format!(
            "🚀 NEW DYNAMIC RETRY SCHEDULE (Emergency: {})",
            is_emergency
        ));
        output.push("".to_string());

        // Fast phase (10 attempts)
        output.push("📈 FAST PHASE - First 10 attempts: 5-10 seconds each".to_string());
        for i in 1..=10 {
            output.push(format!("  Attempt {}: 5-10 seconds", i));
        }

        output.push("".to_string());

        // Dynamic backoff phase
        let (min_delay, max_delay) = if is_emergency { (15, 120) } else { (30, 300) };
        output.push(format!(
            "⚖️  DYNAMIC BACKOFF - Attempts 11+: {}-{} seconds",
            min_delay, max_delay
        ));

        // Show progression
        for backoff_attempt in 0..8 {
            let attempt_num = 11 + backoff_attempt;
            let progression = ((backoff_attempt as f64) / 8.0).min(1.0);
            let target_delay = (min_delay as f64) + progression * ((max_delay - min_delay) as f64);
            output.push(format!(
                "  Attempt {}: ~{:.0} seconds (±20%)",
                attempt_num, target_delay
            ));
        }

        let max_retries = if is_emergency { 20 } else { 15 };
        output.push(format!(
            "  Attempts 19-{}: ~{} seconds (±20%)",
            max_retries, max_delay
        ));

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
                    if is_emergency {
                        2
                    } else {
                        3
                    }
                }
                2 => {
                    if is_emergency {
                        5
                    } else {
                        8
                    }
                }
                3 => {
                    if is_emergency {
                        15
                    } else {
                        20
                    }
                }
                4 => {
                    if is_emergency {
                        30
                    } else {
                        45
                    }
                }
                5 => {
                    if is_emergency {
                        60
                    } else {
                        90
                    }
                }
                6 => {
                    if is_emergency {
                        120
                    } else {
                        180
                    }
                }
                7 => {
                    if is_emergency {
                        180
                    } else {
                        300
                    }
                }
                _ => {
                    if is_emergency {
                        300
                    } else {
                        600
                    }
                }
            };
            schedule.push(format!("Attempt {}: {}s", attempt, delay_secs));
        }
        format!(
            "Retry schedule for {} sells:\n{}",
            if is_emergency { "EMERGENCY" } else { "NORMAL" },
            schedule.join(", ")
        )
    }
}

/// Global cache for sell decisions awaiting execution/retry
pub static SELL_DECISION_CACHE: Lazy<Arc<std::sync::RwLock<HashMap<String, SellDecisionInfo>>>> =
    Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

/// Add a position to the sell decision cache
pub async fn cache_sell_decision(
    position_id: &str,
    mint: &str,
    symbol: &str,
    reason: &str,
    is_emergency: bool,
) {
    // RACE CONDITION PREVENTION: Check if position already has pending exit transaction
    if let Some(existing_position) = crate::positions::get_position_by_mint(mint).await {
        if existing_position.exit_transaction_signature.is_some() {
            let pending_sig = existing_position.exit_transaction_signature.unwrap();
            log(
                LogTag::Trader,
                "SELL_CACHE_BLOCKED",
                &format!(
                    "🚫 Blocked sell decision caching for {} ({}): Position already has pending exit: {}",
                    symbol,
                    mint,
                    &pending_sig[..8]
                )
            );
            return;
        }
    }

    if let Some(mut cache) = safe_write_lock(&SELL_DECISION_CACHE, "cache_sell_decision") {
        let decision = SellDecisionInfo::new(
            position_id.to_string(),
            mint.to_string(),
            symbol.to_string(),
            reason.to_string(),
            is_emergency,
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
                if is_emergency { "EMERGENCY" } else { "NORMAL" },
                symbol,
                reason,
                retry_info
            ),
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
                    position_id, decision.decision_reason, decision.attempt_count
                ),
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
                ),
            );
        }
    }
}

/// Get positions ready for sell retry
pub fn get_positions_ready_for_sell_retry() -> Vec<SellDecisionInfo> {
    if let Some(cache) = safe_read_lock(&SELL_DECISION_CACHE, "get_positions_ready_for_sell_retry")
    {
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
                ),
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
        let emergency_count = cache.values().filter(|d| d.is_emergency_sell).count();
        let ready_for_retry = cache
            .values()
            .filter(|d| d.can_retry() && !d.is_stale())
            .count();
        let exhausted_retries = cache
            .values()
            .filter(|d| d.attempt_count >= d.max_retries)
            .count();
        let stale_count = cache.values().filter(|d| d.is_stale()).count();

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

            status.push_str(&format!(
                "  {}: {} | {}\n",
                pos_id,
                decision.mint.get(..8).unwrap_or(&decision.mint),
                decision.status_string()
            ));
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

static RECENTLY_CLOSED_CACHE: Lazy<Arc<StdRwLock<Option<RecentlyClosedCache>>>> =
    Lazy::new(|| Arc::new(StdRwLock::new(None)));

const RECENTLY_CLOSED_TTL_SECS: u64 = 60; // refresh every minute

async fn get_recently_closed_mints_set() -> HashSet<String> {
    // Try cache first
    if let Some(cache_guard) = safe_read_lock(&RECENTLY_CLOSED_CACHE, "recently_closed_cache_read")
    {
        if let Some(cache) = cache_guard.as_ref() {
            if cache.is_valid(RECENTLY_CLOSED_TTL_SECS) {
                return cache.mints.clone();
            }
        }
    }

    // Load from DB: fetch closed positions and keep those within cooldown
    let now = Utc::now();
    let cooldown_minutes = with_config(|cfg| cfg.trader.position_close_cooldown_minutes);
    let cutoff = now - ChronoDuration::minutes(cooldown_minutes);
    let mut mints: HashSet<String> = HashSet::new();

    match db::get_closed_positions().await {
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
                &format!(
                    "Failed to load recently closed positions for cooldown filter: {}",
                    e
                ),
            );
        }
    }

    // Update cache
    if let Some(mut cache_guard) =
        safe_write_lock(&RECENTLY_CLOSED_CACHE, "recently_closed_cache_write")
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

/// Debug function: Check if a position should be force-sold due to debug timeout
pub fn should_debug_force_sell(position: &crate::positions::Position) -> bool {
    let debug_force_sell_mode = with_config(|cfg| cfg.trader.debug_force_sell_mode);
    if !debug_force_sell_mode {
        return false;
    }

    let position_age_secs = Utc::now()
        .signed_duration_since(position.entry_time)
        .num_seconds() as f64;

    let timeout_secs = with_config(|cfg| cfg.trader.debug_force_sell_timeout_secs);
    if position_age_secs >= timeout_secs {
        log(
            LogTag::Trader,
            "DEBUG_FORCE_SELL",
            &format!(
                "🚨 DEBUG MODE: Force selling {} after {:.1}s (timeout: {:.1}s)",
                position.symbol, position_age_secs, timeout_secs
            ),
        );
        return true;
    }

    false
}

/// Debug function: Check if a token should be force-bought due to simple price drop
pub fn should_debug_force_buy(
    current_price: f64,
    previous_price: Option<f64>,
    symbol: &str,
) -> bool {
    let debug_force_buy_mode = with_config(|cfg| cfg.trader.debug_force_buy_mode);
    if !debug_force_buy_mode {
        return false;
    }

    if let Some(prev_price) = previous_price {
        if prev_price > 0.0 && current_price > 0.0 {
            let drop_percent = ((prev_price - current_price) / prev_price) * 100.0;

            let threshold_percent =
                with_config(|cfg| cfg.trader.debug_force_buy_drop_threshold_percent);
            if drop_percent >= threshold_percent {
                log(
                    LogTag::Trader,
                    "DEBUG_FORCE_BUY",
                    &format!(
                        "🚨 DEBUG MODE: Force buying {} - {:.2}% drop detected (threshold: {:.1}%)",
                        symbol, drop_percent, threshold_percent
                    ),
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
    _pool: Option<f64>, // Changed from PriceResult to simple f64
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
                ),
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
        let cooldown_minutes = with_config(|cfg| cfg.trader.position_close_cooldown_minutes);
        log(
            LogTag::Trader,
            "COOLDOWN_FILTER",
            &format!(
                "⏳ Excluded {} tokens (sample: [{}]) within {}m cooldown; {} remain",
                removed_for_cooldown,
                removed.join(","),
                cooldown_minutes,
                tokens_after_cooldown.len()
            ),
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
    let cooldown_minutes = with_config(|cfg| cfg.trader.position_close_cooldown_minutes);
    format!(
        "Cooldown: {} mints (showing {}): [{}] (window={}m)",
        total,
        sample_list.split(',').filter(|s| !s.is_empty()).count(),
        sample_list,
        cooldown_minutes
    )
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();
    // Create a sticky shutdown future once to avoid missing previously-fired notifications
    let mut shutdown_fut = Box::pin(shutdown.notified());

    log(
        LogTag::Trader,
        "STARTUP",
        "🚀 Starting monitor_new_entries task",
    );

    'outer: loop {
        // Check for shutdown at the very beginning of each loop iteration (sticky)
        if let Some(_) = shutdown_fut.as_mut().now_or_never() {
            break 'outer;
        }

        // CRITICAL: Wait for all core services to be ready before starting any trading operations
        if !crate::global::are_core_services_ready() {
            let pending_services = crate::global::get_pending_services();
            log(
                LogTag::Trader,
                "STARTUP",
                &format!(
                    "⏳ Waiting for core services to be ready... Pending: [{}]",
                    pending_services.join(", ")
                ),
            );

            // Use shutdown-aware sleep instead of fixed sleep
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(1)).await {
                log(
                    LogTag::Trader,
                    "INFO",
                    "✅ New entries monitor shutdown during service startup wait",
                );
                break 'outer;
            }
            continue;
        }

        // Check if trader is enabled via config
        let trader_enabled = with_config(|cfg| cfg.trader.enabled);
        if !trader_enabled {
            // Trader is disabled, sleep and check again
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(2)).await {
                log(
                    LogTag::Trader,
                    "INFO",
                    "✅ New entries monitor shutdown while trader disabled",
                );
                break 'outer;
            }
            continue;
        }

        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        // Proceed with main processing; per-step waits below are shutdown-aware

        // Prepare tokens for trading (fetch, sort, filter)
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "CYCLE_START",
                &format!(
                    "🔄 Starting token preparation cycle at {:.3}s",
                    cycle_start.elapsed().as_secs_f32()
                ),
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
            ),
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
                ),
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
                ),
            );

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let entry_monitor_interval = with_config(|cfg| cfg.trader.entry_monitor_interval_secs);
            let entry_cycle_min_wait = with_config(|cfg| cfg.trader.entry_cycle_min_wait_ms);
            let wait_time = if cycle_duration >= Duration::from_secs(entry_monitor_interval) {
                Duration::from_millis(entry_cycle_min_wait)
            } else {
                Duration::from_secs(entry_monitor_interval) - cycle_duration
            };

            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "CYCLE_WAIT",
                    &format!(
                        "⏸️ Waiting {:.1}s before next cycle (cycle took {:.3}s)",
                        wait_time.as_secs_f32(),
                        cycle_duration.as_secs_f32()
                    ),
                );
            }

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
        let entry_check_concurrency = with_config(|cfg| cfg.trader.entry_check_concurrency);
        let semaphore = Arc::new(Semaphore::new(entry_check_concurrency));

        // Process all available tokens in parallel
        let total_tokens = price_infos.len();

        // Per-cycle aggregation counters (atomic to update from tasks)
        let price_available_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let price_unavailable_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

        let token_check_task_timeout = with_config(|cfg| cfg.trader.token_check_task_timeout_secs);
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
                    entry_check_concurrency,
                    token_check_task_timeout,
                    price_infos
                        .iter()
                        .take(10)
                        .map(|p| p.mint.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }

        let token_processing_shutdown_check =
            with_config(|cfg| cfg.trader.token_processing_shutdown_check_ms);
        let semaphore_acquire_timeout =
            with_config(|cfg| cfg.trader.semaphore_acquire_timeout_secs);

        for price_info in price_infos.iter() {
            // Check for shutdown before spawning tasks (short, responsive wait)
            if check_shutdown_or_delay(
                &shutdown,
                Duration::from_millis(token_processing_shutdown_check),
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
                Duration::from_secs(semaphore_acquire_timeout),
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
                            semaphore_acquire_timeout
                        ),
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

                let task_shutdown_check = with_config(|cfg| cfg.trader.task_shutdown_check_ms);
                let token_check_task_timeout =
                    with_config(|cfg| cfg.trader.token_check_task_timeout_secs);

                // Check for shutdown before starting task
                if check_shutdown_or_delay(
                    &shutdown_clone,
                    Duration::from_millis(task_shutdown_check),
                )
                .await
                {
                    return;
                }

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(token_check_task_timeout), async {
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

                        // Check cooldown period for recently closed positions
                        if crate::positions::is_token_in_cooldown(&price_info.mint).await {
                            return; // Skip this token due to cooldown
                        }

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
                        let debug_force_buy_mode = with_config(
                            |cfg| cfg.trader.debug_force_buy_mode
                        );
                        if debug_force_buy_mode {
                            // Get previous price from token tracker
                            let previous_price = {
                                if let Ok(tracker) = TOKEN_CHECK_TRACKER.read() {
                                    tracker.get(&price_info.mint).and_then(|info| info.last_price)
                                } else {
                                    None
                                }
                            };

                            // Note: Position availability check now handled atomically by global semaphore in open_position_direct
                            if
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
                                        "🚨 DEBUG FORCE BUY: Overriding normal entry logic for {} (limit enforced by semaphore)",
                                        price_info.mint
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
                        let trade_size_sol = with_config(|cfg| cfg.trader.trade_size_sol);
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "POSITION_OPENING",
                                &format!(
                                    "📈 Opening position for {} at {:.9} SOL (size: {} SOL)",
                                    price_info.mint,
                                    current_price,
                                    trade_size_sol
                                )
                            );
                        }

                        // Check if position already exists (open or pending unverified) before attempting to buy
                        if is_open_position(&price_info.mint).await {
                            log(
                                LogTag::Trader,
                                "POSITION_BLOCKED",
                                &format!(
                                    "🚫 POSITION BLOCKED: {} already has open or pending unverified position - skipping buy attempt",
                                    price_info.mint
                                )
                            );

                            // Update token tracking for blocked entry
                            update_token_check_info(
                                &price_info.mint,
                                Some(current_price),
                                false,
                                true,
                                Some(current_price)
                            );
                            return;
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

                        // Add to OHLCV monitoring with Critical priority for open position
                        if position_result.is_ok() {
                            use crate::ohlcvs::{ Priority, ActivityType };

                            if
                                let Err(e) = crate::ohlcvs::add_token_monitoring(
                                    &price_info.mint,
                                    Priority::Critical
                                ).await
                            {
                                log(
                                    LogTag::Trader,
                                    "WARN",
                                    &format!(
                                        "Failed to add {} to OHLCV monitoring: {}",
                                        price_info.mint,
                                        e
                                    )
                                );
                            }

                            if
                                let Err(e) = crate::ohlcvs::record_activity(
                                    &price_info.mint,
                                    ActivityType::PositionOpened
                                ).await
                            {
                                log(
                                    LogTag::Trader,
                                    "WARN",
                                    &format!(
                                        "Failed to record position opened activity for {}: {}",
                                        price_info.mint,
                                        e
                                    )
                                );
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
        let token_check_collection_timeout =
            with_config(|cfg| cfg.trader.token_check_collection_timeout_secs);
        let collection_shutdown_check = with_config(|cfg| cfg.trader.collection_shutdown_check_ms);
        let token_check_handle_timeout =
            with_config(|cfg| cfg.trader.token_check_handle_timeout_secs);

        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "COLLECT_START",
                &format!(
                    "⏳ Collecting {} token tasks with {}s overall timeout",
                    handles_count, token_check_collection_timeout
                ),
            );
        }
        let collection_result =
            tokio::time::timeout(Duration::from_secs(token_check_collection_timeout), async {
                for handle in handles {
                    if check_shutdown_or_delay(
                        &shutdown,
                        Duration::from_millis(collection_shutdown_check),
                    )
                    .await
                    {
                        return;
                    }
                    let _ = tokio::time::timeout(
                        Duration::from_secs(token_check_handle_timeout),
                        handle,
                    )
                    .await;
                }
            })
            .await;
        if collection_result.is_err() {
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "Token check collection timed out after {} seconds",
                    token_check_collection_timeout
                ),
            );
        }

        // Add cycle summary logging
        let max_open_positions = with_config(|cfg| cfg.trader.max_open_positions);
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
                    max_open_positions
                )
            );
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let entry_monitor_interval = with_config(|cfg| cfg.trader.entry_monitor_interval_secs);
        let entry_cycle_min_wait = with_config(|cfg| cfg.trader.entry_cycle_min_wait_ms);
        let wait_time = if cycle_duration >= Duration::from_secs(entry_monitor_interval) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!(
                    "⚠️ Token checking cycle took longer than interval: {:.3}s > {}s",
                    cycle_duration.as_secs_f32(),
                    entry_monitor_interval
                ),
            );
            Duration::from_millis(entry_cycle_min_wait)
        } else {
            // Otherwise wait for the remaining interval time
            let remaining = Duration::from_secs(entry_monitor_interval) - cycle_duration;
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "CYCLE_COMPLETE",
                    &format!(
                        "✅ Cycle completed in {:.3}s, waiting {:.1}s before next cycle",
                        cycle_duration.as_secs_f32(),
                        remaining.as_secs_f32()
                    ),
                );
            }
            remaining
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
    // Sticky shutdown future for open positions monitor
    let mut shutdown_fut = Box::pin(shutdown.notified());

    loop {
        // Sticky check at the top of the loop
        if let Some(_) = shutdown_fut.as_mut().now_or_never() {
            break;
        }
        // CRITICAL: Wait for all core services to be ready before starting any position monitoring
        if !crate::global::are_core_services_ready() {
            let pending_services = crate::global::get_pending_services();
            log(
                LogTag::Trader,
                "STARTUP",
                &format!(
                    "⏳ Position monitor waiting for core services... Pending: [{}]",
                    pending_services.join(", ")
                ),
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // Check if trader is enabled via config
        let trader_enabled = with_config(|cfg| cfg.trader.enabled);
        if !trader_enabled {
            // Trader is disabled, sleep and check again
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(2)).await {
                log(
                    LogTag::Trader,
                    "INFO",
                    "✅ Position monitor shutdown while trader disabled",
                );
                break;
            }
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

            if is_debug_trader_enabled() {
                debug_trader_log(
                    "PRIORITY_RESULT",
                    &format!(
                        "Pool service automatically handles price updates for {} open positions",
                        open_position_mints.len()
                    ),
                );
            }
        }

        let mut positions_to_close = Vec::new();

        // First, collect open positions data (without holding mutex across await)
        let open_positions_data_all: Vec<crate::positions::Position> =
            crate::positions::get_open_positions().await;

        // Filter to only verified-entry, not yet exited positions (preserve previous behavior)
        let mut unverified_count = 0usize;
        let open_positions_data: Vec<crate::positions::Position> = open_positions_data_all
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
                &format!(
                    "Processing {} cached sell decisions for retry",
                    retry_decisions.len()
                ),
            );

            for decision in retry_decisions {
                // Verify position still exists and is open
                if let Some(position) = open_positions_data.iter().find(|p| p.mint == decision.mint)
                {
                    // Get current price for this position
                    if let Some(current_price) = price_map.get(&position.mint) {
                        // Fetch token from cache
                        if let Some(snapshot) = get_global_token_store().get(&position.mint) {
                            let full_token: Token = snapshot.data.clone().into();

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
                                ),
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
                        ),
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
                        &crate::pools::PriceResult::default(), // Use default price result
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
                    let min_profit_threshold_enabled =
                        with_config(|cfg| cfg.trader.min_profit_threshold_enabled);
                    let time_override_duration_hours =
                        with_config(|cfg| cfg.trader.time_override_duration_hours);
                    let time_override_loss_threshold_percent =
                        with_config(|cfg| cfg.trader.time_override_loss_threshold_percent);
                    let min_profit_threshold_percent =
                        with_config(|cfg| cfg.trader.min_profit_threshold_percent);

                    let should_exit = if min_profit_threshold_enabled && !debug_force_sell {
                        // Check if position qualifies for time-based override
                        let position_age_hours =
                            (now.signed_duration_since(position.entry_time).num_seconds() as f64)
                                / 3600.0;
                        let time_override_applies = position_age_hours
                            >= time_override_duration_hours
                            && pnl_percent <= time_override_loss_threshold_percent;

                        if time_override_applies {
                            // Time override: Allow should_sell to decide for old positions with significant losses
                            should_exit_base
                        } else if pnl_percent >= min_profit_threshold_percent {
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
                            >= time_override_duration_hours
                            && pnl_percent <= time_override_loss_threshold_percent;

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
                                min_profit_threshold_percent,
                                min_profit_threshold_enabled,
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
                    let position_id =
                        format!("{}_{}", position.mint, position.entry_time.timestamp());

                    if let Some(cache_guard) =
                        safe_read_lock(&SELL_DECISION_CACHE, "check_cached_decision")
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
                                        ),
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
                                pnl_percent, pnl_sol
                            )
                        };

                        let is_emergency = debug_force_sell ||
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
                                        position.symbol, position.mint
                                    ),
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
                            is_emergency,
                        )
                        .await;

                        // Fetch token from cache for immediate processing
                        let Some(snapshot) = get_global_token_store().get(&position.mint) else {
                            // If token not found in cache, remove the cached decision since we can't trade it
                            remove_sell_decision(&position_id);
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Token not found in cache for mint {} — removing cached sell decision",
                                    position.mint
                                )
                            );
                            continue;
                        };
                        let full_token: Token = snapshot.data.clone().into();

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
                            "Invalid price for position monitoring: {} ({}) - Price = {:.9}",
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
            let semaphore = Arc::new(Semaphore::new(5)); // Allow up to 5 concurrent sells for better performance

            let mut handles = Vec::new();

            let sell_operation_shutdown_check =
                with_config(|cfg| cfg.trader.sell_operation_shutdown_check_ms);
            let sell_semaphore_acquire_timeout =
                with_config(|cfg| cfg.trader.sell_semaphore_acquire_timeout_secs);

            // Process all sell orders concurrently
            for (position, token, exit_price, sell_reason, cached_decision_opt) in
                positions_to_close
            {
                // Check for shutdown before spawning tasks
                if check_shutdown_or_delay(
                    &shutdown,
                    Duration::from_millis(sell_operation_shutdown_check),
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
                    Duration::from_secs(sell_semaphore_acquire_timeout),
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
                let cached_decision_for_task = cached_decision_opt.clone();

                // We already have the position from the analysis phase for logging only
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    // CRITICAL OPERATION PROTECTION - Prevent shutdown during sell
                    let _guard = CriticalOperationGuard::new(&format!("SELL_{}", token.symbol));

                    let position = position;
                    let token_symbol = token.symbol.clone();
                    let position_id =
                        format!("{}_{}", position.mint, position.entry_time.timestamp());

                    let sell_operation_shutdown_check =
                        with_config(|cfg| cfg.trader.sell_operation_shutdown_check_ms);
                    let sell_operation_smart_timeout =
                        with_config(|cfg| cfg.trader.sell_operation_smart_timeout_secs);

                    // Check for shutdown before starting sell operation (non-blocking check)
                    let shutdown_check = tokio::time::timeout(
                        Duration::from_millis(sell_operation_shutdown_check),
                        shutdown_for_task.notified(),
                    )
                    .await;
                    if shutdown_check.is_ok() {
                        return (false, position_id, "Shutdown requested".to_string());
                    }

                    // Wrap the sell operation in a timeout
                    match tokio::time::timeout(
                        Duration::from_secs(sell_operation_smart_timeout),
                        async {
                            crate::positions::close_position_direct(
                                &position.mint,
                                "Trading decision".to_string(),
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

                            // Remove successful sell decision from cache
                            remove_sell_decision(&position_id);

                            (true, position_id, "Success".to_string())
                        }
                        Ok(Err(e)) => {
                            let error_msg =
                                format!("Failed to close position for {}: {}", token_symbol, e);
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

            let sell_operations_collection_timeout =
                with_config(|cfg| cfg.trader.sell_operations_collection_timeout_secs);
            let collection_shutdown_check =
                with_config(|cfg| cfg.trader.collection_shutdown_check_ms);
            let sell_task_handle_timeout =
                with_config(|cfg| cfg.trader.sell_task_handle_timeout_secs);

            // Collect results from all concurrent sell operations
            let collection_result = tokio::time::timeout(
                Duration::from_secs(sell_operations_collection_timeout),
                async {
                    let mut completed = 0usize;
                    let mut successful = 0usize;

                    for handle in handles {
                        // Skip if shutdown signal received
                        if check_shutdown_or_delay(
                            &shutdown,
                            Duration::from_millis(collection_shutdown_check),
                        )
                        .await
                        {
                            break;
                        }

                        // Add timeout for each handle
                        match tokio::time::timeout(
                            Duration::from_secs(sell_task_handle_timeout),
                            handle,
                        )
                        .await
                        {
                            Ok(task_result) => match task_result {
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
                                                position_id, message
                                            ),
                                        );
                                    }
                                }
                                Err(e) => {
                                    completed += 1;
                                    log(
                                        LogTag::Trader,
                                        "ERROR",
                                        &format!("Sell task panicked: {}", e),
                                    );
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

        let position_monitor_interval =
            with_config(|cfg| cfg.trader.position_monitor_interval_secs);
        if check_shutdown_or_delay(&shutdown, Duration::from_secs(position_monitor_interval)).await
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

// =============================================================================
// TRADER RUNTIME CONTROL FUNCTIONS
// =============================================================================

#[derive(Debug)]
pub enum TraderControlError {
    AlreadyRunning,
    AlreadyStopped,
    ConfigUpdate(String),
}

impl std::fmt::Display for TraderControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraderControlError::AlreadyRunning => write!(f, "Trader is already running"),
            TraderControlError::AlreadyStopped => write!(f, "Trader is already stopped"),
            TraderControlError::ConfigUpdate(err) => write!(f, "Config update failed: {}", err),
        }
    }
}

impl std::error::Error for TraderControlError {}

/// Check if the trader is currently running
pub fn is_trader_running() -> bool {
    with_config(|cfg| cfg.trader.enabled)
}

/// Stop the trader gracefully by signaling shutdown and waiting for tasks to complete
pub async fn stop_trader_gracefully() -> Result<(), TraderControlError> {
    if !with_config(|cfg| cfg.trader.enabled) {
        return Err(TraderControlError::AlreadyStopped);
    }

    log(LogTag::Trader, "INFO", "Disabling trader operations...");

    update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    )
    .map_err(TraderControlError::ConfigUpdate)?;

    // Wait for critical operations to complete (max 30 seconds)
    let timeout = Instant::now() + Duration::from_secs(30);
    while CRITICAL_OPERATIONS_IN_PROGRESS.load(Ordering::SeqCst) > 0 {
        if Instant::now() > timeout {
            log(
                LogTag::Trader,
                "WARN",
                "Timeout waiting for critical operations to complete during trader stop",
            );
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    log(LogTag::Trader, "INFO", "Trader operations disabled");

    Ok(())
}

/// Start the trader by spawning monitoring tasks
pub async fn start_trader() -> Result<(), TraderControlError> {
    if with_config(|cfg| cfg.trader.enabled) {
        return Err(TraderControlError::AlreadyRunning);
    }

    log(LogTag::Trader, "INFO", "Enabling trader operations...");

    update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
        },
        true,
    )
    .map_err(TraderControlError::ConfigUpdate)?;

    log(LogTag::Trader, "INFO", "Trader operations enabled");

    Ok(())
}
