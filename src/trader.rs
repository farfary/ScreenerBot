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
pub const MAX_OPEN_POSITIONS: usize = 8;

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
pub const TIME_OVERRIDE_DURATION_HOURS: f64 = 72.0;

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

/// Per-token re-entry cooldown after closing a position (minutes) - prevents immediate re-buy of same token
/// This is applied in apply_cooldown_filter() and is separate from:
/// - Global position open cooldown (5s between any opens) - in positions.rs
/// - Frozen account cooldowns (account-specific) - in positions.rs
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 120; // 2 hours

// -----------------------------------------------------------------------------
// Trading Logic Configuration
// -----------------------------------------------------------------------------
// Monitoring & Display Configuration
// -----------------------------------------------------------------------------

/// Summary display refresh interval (seconds) - optimized for 5s priority checking
pub const SUMMARY_DISPLAY_INTERVAL_SECS: u64 = 5;

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
pub const TOKEN_CHECK_TASK_TIMEOUT_SECS: u64 = 60;

/// Price cache lock acquire timeout (milliseconds)
pub const PRICE_CACHE_LOCK_TIMEOUT_MS: u64 = 2000;

/// Task collection timeout for concurrent operations (seconds)
pub const TASK_COLLECTION_TIMEOUT_SECS: u64 = 120;

/// Token check result collection timeout (seconds)
pub const TOKEN_CHECK_COLLECTION_TIMEOUT_SECS: u64 = 120;

/// Individual token check handle timeout (seconds)
pub const TOKEN_CHECK_HANDLE_TIMEOUT_SECS: u64 = 120;

/// Sell operations collection timeout (seconds) - must accommodate multiple 3-min operations
pub const SELL_OPERATIONS_COLLECTION_TIMEOUT_SECS: u64 = 240;

/// Individual sell operation timeout (seconds) - removed for smart timeout handling
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
/// Higher values speed up scanning but increase load on price services
pub const ENTRY_CHECK_CONCURRENCY: usize = 24; // previously 10

// Capacity-aware Scheduling
// -------------------------
/// Max number of tokens to fully process per cycle (rotated across cycles)
/// Rule of thumb: ~3x concurrency to keep workers fed without overloading services
pub const MAX_TOKENS_PER_CYCLE: usize = ENTRY_CHECK_CONCURRENCY * 2;
/// Maximum number of tokens to keep after prioritization per cache refresh
/// This caps the working set early to reduce churn and focus checks
pub const PREPARED_TOKENS_CAP: usize = 200;

/// Limit tokens analyzed for watchlist seeding per cycle (keeps history refresh light)

/// Fraction of the cycle interval used as a soft time budget; beyond this we stop scheduling new tasks
/// Increased to allow more time for token processing after preparation phase
pub const TIME_BUDGET_FRACTION: f64 = 1.8;

use crate::global::is_debug_trader_enabled;
use crate::logger::{ log, LogTag };
use crate::positions_lib::calculate_position_pnl;
use crate::tokens::{
    get_all_tokens_by_liquidity,
    get_price,
    pool::{ add_watchlist_tokens, get_pool_service, request_price_warmup_batch },
    cache::TokenDatabase,
    PriceOptions,
    Token,
};
use crate::utils::check_shutdown_or_delay;

use crate::entry::get_profit_target;
use crate::filtering::log_filtering_summary;

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use chrono::{ Utc, Duration as ChronoDuration };
use futures;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Notify;
use tabled::{ Tabled, Table, settings::{ Style, object::Rows, Alignment, Modify } };
use std::collections::HashSet;
use std::sync::RwLock as StdRwLock;

use crate::positions_db;

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: tracks critical trading operations in progress to prevent force shutdown
pub static CRITICAL_OPERATIONS_IN_PROGRESS: Lazy<Arc<std::sync::atomic::AtomicUsize>> = Lazy::new(||
    Arc::new(std::sync::atomic::AtomicUsize::new(0))
);

/// Global tracker: number of buy operations currently in-flight (reserved but not yet reflected in open positions)
// removed legacy in-flight buy tracking; PositionsManager enforces capacity

/// Rotating scheduler offset for capacity-aware token batching across cycles
static SCHEDULER_OFFSET: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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

/// Display structure for tokens sorted by total checks
#[derive(Tabled)]
pub struct TokenCheckDisplay {
    #[tabled(rename = "#")]
    rank: String,
    #[tabled(rename = "🔑 Mint")]
    mint: String,
    #[tabled(rename = "🏷️ Symbol")]
    symbol: String,
    #[tabled(rename = "💧 Liq")]
    liquidity: String,
    #[tabled(rename = "📊 MCap")]
    market_cap: String,
    #[tabled(rename = "💲 Last Price")]
    last_price: String,
    #[tabled(rename = "🏊 Pool")]
    pool_info: String,
    #[tabled(rename = "🔍 Checks")]
    check_count: String,
    #[tabled(rename = "🎯 Entry")]
    entry_checks: String,
}

/// Display structure for tokens sorted by entry checks
#[derive(Tabled)]
pub struct TokenEntryDisplay {
    #[tabled(rename = "#")]
    rank: String,
    #[tabled(rename = "🔑 Mint")]
    mint: String,
    #[tabled(rename = "🏷️ Symbol")]
    symbol: String,
    #[tabled(rename = "💧 Liq")]
    liquidity: String,
    #[tabled(rename = "📊 MCap")]
    market_cap: String,
    #[tabled(rename = "💲 Last Price")]
    last_price: String,
    #[tabled(rename = "🏊 Pool")]
    pool_info: String,
    #[tabled(rename = "🎯 Entry")]
    entry_checks: String,
    #[tabled(rename = "🔍 Checks")]
    check_count: String,
}

/// Display structure for price summary statistics
#[derive(Tabled)]
pub struct PriceSummaryDisplay {
    #[tabled(rename = "📊 Metric")]
    metric: String,
    #[tabled(rename = "🔢 Value")]
    value: String,
}

/// Display structure for blacklist statistics
#[derive(Tabled)]
pub struct BlacklistSummaryDisplay {
    #[tabled(rename = "🚫 Status")]
    status: String,
    #[tabled(rename = "🔢 Count")]
    count: String,
}

/// Global token tracking state
pub static TOKEN_CHECK_TRACKER: Lazy<
    Arc<std::sync::RwLock<HashMap<String, TokenCheckInfo>>>
> = Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

/// Token confidence tracking for intelligent monitoring
#[derive(Clone, Debug)]
pub struct TokenConfidenceInfo {
    pub mint: String,
    pub symbol: String,
    pub confidence: f64,
    pub last_updated: Instant,
    pub last_price: Option<f64>,
    pub trend: String, // "rising", "falling", "stable"
    pub consecutive_updates: usize,
}

/// Global confidence-based token ranking system
pub static TOKEN_CONFIDENCE_TRACKER: Lazy<Arc<std::sync::RwLock<Vec<TokenConfidenceInfo>>>> =
    Lazy::new(|| Arc::new(std::sync::RwLock::new(Vec::new())));

// =============================================================================
// INVALID POOL TOKENS BLACKLIST SYSTEM
// =============================================================================

/// Information about a token that failed pool validation
#[derive(Clone, Debug)]
pub struct InvalidPoolTokenInfo {
    pub mint: String,
    pub symbol: String,
    pub failure_reason: String,
    pub failed_at: Instant,
    pub retry_after: Instant,
    pub failure_count: u32,
    pub last_pool_check_error: Option<String>,
}

impl InvalidPoolTokenInfo {
    /// Create new invalid pool token info
    pub fn new(mint: String, symbol: String, reason: String) -> Self {
        let now = Instant::now();
        Self {
            mint,
            symbol,
            failure_reason: reason,
            failed_at: now,
            retry_after: now + Duration::from_secs(3600), // Retry after 1 hour initially
            failure_count: 1,
            last_pool_check_error: None,
        }
    }

    /// Check if this token is ready for retry
    pub fn can_retry(&self) -> bool {
        Instant::now() > self.retry_after
    }

    /// Update retry time based on failure count (exponential backoff)
    pub fn update_retry_time(&mut self) {
        self.failure_count += 1;
        let backoff_hours = std::cmp::min((self.failure_count as u64) * 2, 24); // Max 24 hours
        self.retry_after = Instant::now() + Duration::from_secs(backoff_hours * 3600);
    }

    /// Check if this entry is stale (older than 7 days)
    pub fn is_stale(&self) -> bool {
        Instant::now().duration_since(self.failed_at).as_secs() > 7 * 24 * 3600 // 7 days
    }
}

/// Global blacklist for tokens with invalid/unsupported pools
pub static INVALID_POOL_TOKENS: Lazy<
    Arc<std::sync::RwLock<HashMap<String, InvalidPoolTokenInfo>>>
> = Lazy::new(|| Arc::new(std::sync::RwLock::new(HashMap::new())));

// =============================================================================
// LIGHTWEIGHT TOKEN META CACHE FOR SUMMARIES
// =============================================================================

#[derive(Clone, Debug)]
struct TokenMetaBrief {
    symbol: String,
    name: String,
    liquidity_usd: Option<f64>,
    market_cap: Option<f64>,
    cached_at: Instant,
}

static TOKEN_META_CACHE: Lazy<Arc<std::sync::RwLock<HashMap<String, TokenMetaBrief>>>> = Lazy::new(
    || Arc::new(std::sync::RwLock::new(HashMap::new()))
);

const TOKEN_META_TTL_SECS: u64 = 600; // 10 minutes

fn format_usd_short(v: f64) -> String {
    let abs_v = v.abs();
    if abs_v >= 1_000_000_000.0 {
        format!("${:.1}B", v / 1_000_000_000.0)
    } else if abs_v >= 1_000_000.0 {
        format!("${:.1}M", v / 1_000_000.0)
    } else if abs_v >= 1_000.0 {
        format!("${:.1}k", v / 1_000.0)
    } else {
        format!("${:.0}", v)
    }
}

fn get_token_meta_brief(mint: &str) -> Option<TokenMetaBrief> {
    // Try cache first
    {
        let cache = TOKEN_META_CACHE.read().ok()?;
        if let Some(meta) = cache.get(mint) {
            if meta.cached_at.elapsed().as_secs() < TOKEN_META_TTL_SECS {
                return Some(meta.clone());
            }
        }
    }

    // Load from DB (sync API) and cache it
    let db = match crate::tokens::cache::TokenDatabase::new() {
        Ok(db) => db,
        Err(_) => {
            return None;
        }
    };

    match db.get_token_by_mint(mint) {
        Ok(Some(api_token)) => {
            let meta = TokenMetaBrief {
                symbol: api_token.symbol.clone(),
                name: api_token.name.clone(),
                liquidity_usd: api_token.liquidity.as_ref().and_then(|l| l.usd),
                market_cap: api_token.market_cap,
                cached_at: Instant::now(),
            };
            if let Ok(mut cache) = TOKEN_META_CACHE.write() {
                cache.insert(mint.to_string(), meta.clone());
            }
            Some(meta)
        }
        _ => None,
    }
}

fn fmt_opt9(v: Option<f64>) -> String {
    v.map(|x| format!("{:.9}", x)).unwrap_or_else(|| "-".to_string())
}

fn short8(s: &str) -> String {
    if s.len() > 8 { s[..8].to_string() } else { s.to_string() }
}

// =============================================================================
// TOKEN CACHE FOR PERFORMANCE OPTIMIZATION
// =============================================================================

// (duplicate block removed)

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
    if let Some(cache) = RECENTLY_CLOSED_CACHE.read().unwrap().as_ref() {
        if cache.is_valid(RECENTLY_CLOSED_TTL_SECS) {
            return cache.mints.clone();
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
    {
        let mut cache = RECENTLY_CLOSED_CACHE.write().unwrap();
        *cache = Some(RecentlyClosedCache {
            mints: mints.clone(),
            cached_at: Instant::now(),
        });
    }

    mints
}

/// Cached token data structure for 10-minute caching
#[derive(Clone, Debug)]
pub struct CachedTokenData {
    pub tokens: Vec<Token>,
    pub cached_at: Instant,
    pub cache_duration_minutes: u64,
}

impl CachedTokenData {
    /// Check if cache is still valid
    pub fn is_valid(&self) -> bool {
        // Never treat an empty token set as a valid cache
        if self.tokens.is_empty() {
            return false;
        }
        let age_minutes = self.cached_at.elapsed().as_secs() / 60;
        age_minutes < self.cache_duration_minutes
    }

    /// Get age of cache in minutes
    pub fn age_minutes(&self) -> u64 {
        self.cached_at.elapsed().as_secs() / 60
    }
}

/// Global cache for filtered tokens (10-minute duration)
/// Reduces database load by caching filtered tokens across multiple monitoring cycles
pub static FILTERED_TOKENS_CACHE: Lazy<Arc<std::sync::RwLock<Option<CachedTokenData>>>> = Lazy::new(
    || Arc::new(std::sync::RwLock::new(None))
);

/// Token cache duration in minutes
pub const TOKEN_CACHE_DURATION_MINUTES: u64 = 10;

/// Clear the token cache (useful for debugging or forced refresh)
pub fn clear_token_cache() {
    let mut cache = FILTERED_TOKENS_CACHE.write().unwrap();
    *cache = None;
    log(LogTag::Trader, "CACHE_CLEARED", "🗑️ Token cache manually cleared");
}

/// Get token cache status for debugging
pub fn get_token_cache_status() -> String {
    let cache = FILTERED_TOKENS_CACHE.read().unwrap();
    match cache.as_ref() {
        Some(cached_data) => {
            let age_minutes = cached_data.age_minutes();
            let is_valid = cached_data.is_valid();
            format!(
                "Token Cache Status:\n  \
                 - Tokens: {}\n  \
                 - Age: {}min / {}min limit\n  \
                 - Valid: {}\n  \
                 - Remaining: {}min",
                cached_data.tokens.len(),
                age_minutes,
                TOKEN_CACHE_DURATION_MINUTES,
                is_valid,
                if is_valid {
                    TOKEN_CACHE_DURATION_MINUTES - age_minutes
                } else {
                    0
                }
            )
        }
        None => "Token Cache Status: No cache present".to_string(),
    }
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
                position.symbol,
                position_age_secs,
                DEBUG_FORCE_SELL_TIMEOUT_SECS
            )
        );
        return true;
    }

    false
}

// =============================================================================
// CONFIDENCE-BASED TOKEN TRACKING SYSTEM
// =============================================================================

/// Update token confidence in the global tracking system
pub fn update_token_confidence(
    mint: &str,
    symbol: &str,
    confidence: f64,
    current_price: Option<f64>
) {
    let mut tracker = TOKEN_CONFIDENCE_TRACKER.write().unwrap();
    let now = Instant::now();

    // Find existing entry or create new one
    let mut found_index = None;
    for (i, info) in tracker.iter().enumerate() {
        if info.mint == mint {
            found_index = Some(i);
            break;
        }
    }

    if let Some(index) = found_index {
        // Update existing entry
        let existing = &mut tracker[index];
        let prev_confidence = existing.confidence;

        // Determine trend
        let trend = if confidence > prev_confidence + 5.0 {
            "rising".to_string()
        } else if confidence < prev_confidence - 5.0 {
            "falling".to_string()
        } else {
            "stable".to_string()
        };

        existing.confidence = confidence;
        existing.last_updated = now;
        existing.last_price = current_price;
        existing.trend = trend;
        existing.consecutive_updates += 1;
    } else {
        // Create new entry
        tracker.push(TokenConfidenceInfo {
            mint: mint.to_string(),
            symbol: symbol.to_string(),
            confidence,
            last_updated: now,
            last_price: current_price,
            trend: "stable".to_string(),
            consecutive_updates: 1,
        });
    }

    // Sort by confidence (highest first)
    tracker.sort_by(|a, b| {
        b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Keep only top 50 entries to prevent memory bloat
    tracker.truncate(50);
}

/// Get top confidence tokens for priority monitoring
pub fn get_top_confidence_tokens(limit: usize) -> Vec<TokenConfidenceInfo> {
    let tracker = TOKEN_CONFIDENCE_TRACKER.read().unwrap();
    let now = Instant::now();

    // Filter out stale entries (older than 5 minutes) and return top entries
    tracker
        .iter()
        .filter(|info| now.duration_since(info.last_updated).as_secs() < 300) // 5 minutes
        .take(limit)
        .cloned()
        .collect()
}

/// Clean up stale confidence entries
pub fn cleanup_stale_confidence_entries() {
    let mut tracker = TOKEN_CONFIDENCE_TRACKER.write().unwrap();
    let now = Instant::now();

    // Remove entries older than 10 minutes
    tracker.retain(|info| now.duration_since(info.last_updated).as_secs() < 600);
}

/// Get detailed confidence tracking status for debugging
pub fn get_confidence_tracking_status() -> String {
    let tracker = TOKEN_CONFIDENCE_TRACKER.read().unwrap();
    let now = Instant::now();

    if tracker.is_empty() {
        return "No tokens in confidence tracking system".to_string();
    }

    let mut status = format!("Confidence Tracking Status ({} tokens):\n", tracker.len());

    for (i, info) in tracker.iter().enumerate() {
        let age_secs = now.duration_since(info.last_updated).as_secs();
        let price_str = info.last_price
            .map(|p| format!("{:.9} SOL", p))
            .unwrap_or_else(|| "No price".to_string());

        status.push_str(
            &format!(
                "  {}. {} ({}): {:.1}% confidence, trend: {}, price: {}, age: {}s, updates: {}\n",
                i + 1,
                info.symbol,
                &info.mint[..8],
                info.confidence,
                info.trend,
                price_str,
                age_secs,
                info.consecutive_updates
            )
        );

        // Limit display to top 20 for readability
        if i >= 19 {
            if tracker.len() > 20 {
                status.push_str(&format!("  ... and {} more tokens\n", tracker.len() - 20));
            }
            break;
        }
    }

    status
}

// =============================================================================
// INVALID POOL TOKENS BLACKLIST MANAGEMENT
// =============================================================================

/// Add a token to the invalid pool blacklist
pub fn add_to_invalid_pool_blacklist(mint: &str, symbol: &str, reason: &str) {
    // Use the pool service blacklist instead
    let mint = mint.to_string();
    let symbol = symbol.to_string();
    let reason = reason.to_string();

    tokio::spawn(async move {
        crate::tokens::pool::add_to_invalid_pool_blacklist(&mint, Some(&symbol), &reason).await;
    });
}

/// Check if a token is in the invalid pool blacklist and not ready for retry
pub fn is_token_in_invalid_pool_blacklist(mint: &str) -> bool {
    // Use async tokio block_on to call the pool service async function
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle
            ::current()
            .block_on(async { crate::tokens::pool::is_in_invalid_pool_blacklist(mint).await })
    })
}

/// Remove a token from the invalid pool blacklist (for manual override)
pub fn remove_from_invalid_pool_blacklist(mint: &str) -> bool {
    let mut blacklist = INVALID_POOL_TOKENS.write().unwrap();

    if let Some(info) = blacklist.remove(mint) {
        log(
            LogTag::Trader,
            "BLACKLIST_REMOVE",
            &format!(
                "⚪ Manually removed {} ({}) from invalid pool blacklist",
                info.symbol,
                &mint[..8]
            )
        );
        true
    } else {
        false
    }
}

/// Get tokens ready for retry (not actively blacklisted but tracked)
pub fn get_tokens_ready_for_pool_retry() -> Vec<String> {
    let blacklist = INVALID_POOL_TOKENS.read().unwrap();
    blacklist
        .values()
        .filter(|info| info.can_retry())
        .map(|info| info.mint.clone())
        .collect()
}

/// Clean up stale entries from invalid pool blacklist
pub fn cleanup_invalid_pool_blacklist() -> usize {
    let mut blacklist = INVALID_POOL_TOKENS.write().unwrap();
    let before_count = blacklist.len();

    blacklist.retain(|_, info| !info.is_stale());

    let removed_count = before_count - blacklist.len();
    if removed_count > 0 {
        log(
            LogTag::Trader,
            "BLACKLIST_CLEANUP",
            &format!(
                "🧹 Cleaned up {} stale invalid pool blacklist entries ({} remaining)",
                removed_count,
                blacklist.len()
            )
        );
    }

    removed_count
}

/// Get invalid pool blacklist status for debugging
pub fn get_invalid_pool_blacklist_status() -> String {
    let blacklist = INVALID_POOL_TOKENS.read().unwrap();
    let now = Instant::now();

    if blacklist.is_empty() {
        return "Invalid pool blacklist is empty".to_string();
    }

    let mut active_blacklist = Vec::new();
    let mut ready_for_retry = Vec::new();

    for (mint, info) in blacklist.iter() {
        if info.can_retry() {
            ready_for_retry.push((mint, info));
        } else {
            active_blacklist.push((mint, info));
        }
    }

    let mut status = format!(
        "Invalid Pool Blacklist Status: {} total ({} active, {} ready for retry)\n",
        blacklist.len(),
        active_blacklist.len(),
        ready_for_retry.len()
    );

    if !active_blacklist.is_empty() {
        status.push_str("Active Blacklist (blocked from checks):\n");
        for (i, (mint, info)) in active_blacklist.iter().enumerate() {
            let retry_in_hours = info.retry_after.duration_since(now).as_secs() / 3600;
            status.push_str(
                &format!(
                    "  {}. {} ({}) - {} | attempts: {} | retry in: {}h\n",
                    i + 1,
                    info.symbol,
                    &mint[..8],
                    info.failure_reason,
                    info.failure_count,
                    retry_in_hours
                )
            );
            if i >= 9 {
                status.push_str(
                    &format!("  ... and {} more active entries\n", active_blacklist.len() - 10)
                );
                break;
            }
        }
    }

    if !ready_for_retry.is_empty() {
        status.push_str("Ready for Retry (tracking only):\n");
        for (i, (mint, info)) in ready_for_retry.iter().enumerate() {
            let hours_since_fail = now.duration_since(info.failed_at).as_secs() / 3600;
            status.push_str(
                &format!(
                    "  {}. {} ({}) - {} | attempts: {} | failed {}h ago\n",
                    i + 1,
                    info.symbol,
                    &mint[..8],
                    info.failure_reason,
                    info.failure_count,
                    hours_since_fail
                )
            );
            if i >= 9 {
                status.push_str(
                    &format!("  ... and {} more retry entries\n", ready_for_retry.len() - 10)
                );
                break;
            }
        }
    }

    status
}

/// Update token tracking after checking a token
pub fn update_token_check_info(
    mint: &str,
    current_price: Option<f64>,
    had_drop: bool,
    entry_checked: bool,
    pool: Option<&crate::tokens::PriceResult>
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

    // Update pool fields if provided
    if let Some(p) = pool {
        // Only overwrite with Some values to preserve last known good
        if let Some(t) = &p.pool_type {
            info.pool_type = Some(t.clone());
        }
        if let Some(addr) = &p.pool_address {
            info.pool_address = Some(addr.clone());
        }
        if let Some(pp) = p.pool_price_sol.or(p.price_sol) {
            info.pool_price_sol = Some(pp);
        }
        if let Some(rs) = p.reserve_sol {
            info.reserve_sol = Some(rs);
        }
        if let Some(rt) = p.reserve_token {
            info.reserve_token = Some(rt);
        }
    }
}

/// Print a compact per-cycle summary of price availability and token check stats
fn log_cycle_price_summary(available: usize, unavailable: usize, zero_hist_warmed: usize) {
    let tracker = TOKEN_CHECK_TRACKER.read().unwrap();

    // Top tokens by total checks
    let mut top_by_checks: Vec<(&String, &TokenCheckInfo)> = tracker.iter().collect();
    top_by_checks.sort_by(|a, b| b.1.check_count.cmp(&a.1.check_count));
    let top_by_checks_displays: Vec<TokenCheckDisplay> = top_by_checks
        .iter()
        .take(10)
        .enumerate()
        .map(|(idx, (mint, info))| {
            let last_price = info.last_price
                .map(|p| format!("{:.9}", p))
                .unwrap_or_else(|| "-".to_string());
            let meta = get_token_meta_brief(mint);
            let sym_name = meta
                .as_ref()
                .map(|m| format!("{} ({})", m.symbol, m.name))
                .unwrap_or_else(|| "-".to_string());
            let liq = meta
                .as_ref()
                .and_then(|m| m.liquidity_usd)
                .map(|v| format_usd_short(v))
                .unwrap_or_else(|| "-".to_string());
            let mcap = meta
                .as_ref()
                .and_then(|m| m.market_cap)
                .map(|v| format_usd_short(v))
                .unwrap_or_else(|| "-".to_string());
            // Use last stored pool info from tracker
            let ptype = info.pool_type.clone().unwrap_or_default();
            let paddr = info.pool_address.clone().unwrap_or_default();
            let pprice = info.pool_price_sol;
            let rsol = info.reserve_sol;
            let rtok = info.reserve_token;
            let pool_str = if paddr.is_empty() {
                "pool:-".to_string()
            } else {
                format!(
                    "pool:{}/{} p:{} rs:{} tok:{}",
                    if ptype.is_empty() {
                        "-".to_string()
                    } else {
                        ptype
                    },
                    short8(&paddr),
                    fmt_opt9(pprice),
                    fmt_opt9(rsol),
                    fmt_opt9(rtok)
                )
            };
            TokenCheckDisplay {
                rank: format!("{}.", idx + 1),
                mint: format!("{}", &mint[..8]),
                symbol: sym_name,
                liquidity: liq,
                market_cap: mcap,
                last_price: format!("{} SOL", last_price),
                pool_info: pool_str,
                check_count: format!("{}", info.check_count),
                entry_checks: format!("{}", info.entry_check_count),
            }
        })
        .collect();

    // Top tokens by entry checks
    let mut top_by_entries: Vec<(&String, &TokenCheckInfo)> = tracker.iter().collect();
    top_by_entries.sort_by(|a, b| b.1.entry_check_count.cmp(&a.1.entry_check_count));
    let top_by_entries_displays: Vec<TokenEntryDisplay> = top_by_entries
        .iter()
        .take(10)
        .enumerate()
        .map(|(idx, (mint, info))| {
            let last_price = info.last_price
                .map(|p| format!("{:.9}", p))
                .unwrap_or_else(|| "-".to_string());
            let meta = get_token_meta_brief(mint);
            let sym_name = meta
                .as_ref()
                .map(|m| format!("{} ({})", m.symbol, m.name))
                .unwrap_or_else(|| "-".to_string());
            let liq = meta
                .as_ref()
                .and_then(|m| m.liquidity_usd)
                .map(|v| format_usd_short(v))
                .unwrap_or_else(|| "-".to_string());
            let mcap = meta
                .as_ref()
                .and_then(|m| m.market_cap)
                .map(|v| format_usd_short(v))
                .unwrap_or_else(|| "-".to_string());
            // Use last stored pool info from tracker
            let ptype = info.pool_type.clone().unwrap_or_default();
            let paddr = info.pool_address.clone().unwrap_or_default();
            let pprice = info.pool_price_sol;
            let rsol = info.reserve_sol;
            let rtok = info.reserve_token;
            let pool_str = if paddr.is_empty() {
                "pool:-".to_string()
            } else {
                format!(
                    "pool:{}/{} p:{} rs:{} tok:{}",
                    if ptype.is_empty() {
                        "-".to_string()
                    } else {
                        ptype
                    },
                    short8(&paddr),
                    fmt_opt9(pprice),
                    fmt_opt9(rsol),
                    fmt_opt9(rtok)
                )
            };
            TokenEntryDisplay {
                rank: format!("{}.", idx + 1),
                mint: format!("{}", &mint[..8]),
                symbol: sym_name,
                liquidity: liq,
                market_cap: mcap,
                last_price: format!("{} SOL", last_price),
                pool_info: pool_str,
                entry_checks: format!("{}", info.entry_check_count),
                check_count: format!("{}", info.check_count),
            }
        })
        .collect();

    // Overall tracker stats
    let tracked_tokens = tracker.len();
    let tokens_with_price = tracker
        .iter()
        .filter(|(_, info)| info.last_price.is_some())
        .count();
    let tokens_without_price = tracked_tokens.saturating_sub(tokens_with_price);

    let avg_checks = if tracked_tokens > 0 {
        (
            tracker
                .iter()
                .map(|(_, i)| i.check_count as u64)
                .sum::<u64>() as f64
        ) / (tracked_tokens as f64)
    } else {
        0.0
    };
    let avg_entry_checks = if tracked_tokens > 0 {
        (
            tracker
                .iter()
                .map(|(_, i)| i.entry_check_count as u64)
                .sum::<u64>() as f64
        ) / (tracked_tokens as f64)
    } else {
        0.0
    };

    // Blacklist stats
    let blacklist = INVALID_POOL_TOKENS.read().unwrap();
    let blacklist_total = blacklist.len();
    let blacklist_active = blacklist
        .values()
        .filter(|info| !info.can_retry())
        .count();
    let blacklist_ready_for_retry = blacklist_total - blacklist_active;
    drop(blacklist);

    // Create summary statistics displays
    let price_summary_displays = vec![
        PriceSummaryDisplay {
            metric: "🔍 Available".to_string(),
            value: format!("{}", available),
        },
        PriceSummaryDisplay {
            metric: "❌ Unavailable".to_string(),
            value: format!("{}", unavailable),
        },
        PriceSummaryDisplay {
            metric: "🔥 Zero Warmed".to_string(),
            value: format!("{}", zero_hist_warmed),
        },
        PriceSummaryDisplay {
            metric: "📊 Tracked Tokens".to_string(),
            value: format!("{}", tracked_tokens),
        },
        PriceSummaryDisplay {
            metric: "💲 With Price".to_string(),
            value: format!("{}", tokens_with_price),
        },
        PriceSummaryDisplay {
            metric: "❓ Without Price".to_string(),
            value: format!("{}", tokens_without_price),
        },
        PriceSummaryDisplay {
            metric: "📈 Avg Checks".to_string(),
            value: format!("{:.1}", avg_checks),
        },
        PriceSummaryDisplay {
            metric: "🎯 Avg Entry Checks".to_string(),
            value: format!("{:.1}", avg_entry_checks),
        }
    ];

    let blacklist_summary_displays = vec![
        BlacklistSummaryDisplay {
            status: "🚫 Total Blacklisted".to_string(),
            count: format!("{}", blacklist_total),
        },
        BlacklistSummaryDisplay {
            status: "❌ Active".to_string(),
            count: format!("{}", blacklist_active),
        },
        BlacklistSummaryDisplay {
            status: "🔄 Retry Ready".to_string(),
            count: format!("{}", blacklist_ready_for_retry),
        }
    ];

    // Build the complete summary using tables
    let mut summary = String::new();
    summary.push_str("===== ENTRY PRICE SUMMARY =====\n");

    // Price Statistics
    summary.push_str("\n📊 Price Statistics\n");
    let mut price_table = Table::new(price_summary_displays);
    price_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary.push_str(&format!("{}\n", price_table));

    // Blacklist Statistics
    summary.push_str("\n🚫 Blacklist Statistics\n");
    let mut blacklist_table = Table::new(blacklist_summary_displays);
    blacklist_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    summary.push_str(&format!("{}\n", blacklist_table));

    // Top by checks table
    if !top_by_checks_displays.is_empty() {
        summary.push_str("\n🔍 Top by Total Checks\n");
        let mut checks_table = Table::new(top_by_checks_displays);
        checks_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        summary.push_str(&format!("{}\n", checks_table));
    }

    // Top by entry checks table
    if !top_by_entries_displays.is_empty() {
        summary.push_str("\n🎯 Top by Entry Checks\n");
        let mut entries_table = Table::new(top_by_entries_displays);
        entries_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        summary.push_str(&format!("{}\n", entries_table));
    }

    // Print directly to stdout instead of using logger
    println!("{}", summary);
}

// Ensure token is in watchlist after a price failure, with a clear log
async fn ensure_watchlist_on_price_fail(mint: &str, symbol: &str, reason: &str) {
    let service = get_pool_service();
    let watchlist = service.get_watchlist_tokens().await;
    if !watchlist.iter().any(|m| m == mint) {
        add_watchlist_tokens(&[mint.to_string()]).await;
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "WATCHLIST_ADD",
                &format!("📝 Added {} to watchlist (reason: price_fail: {})", symbol, reason)
            );
        }
    }
}

/// Check if token had recent price drop (within 30 seconds)
pub async fn check_token_for_recent_drop(token: &Token) -> bool {
    let pool_service = get_pool_service();
    let history = pool_service.get_price_history(&token.mint).await;

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
    let current = recent_prices
        .last()
        .map(|(_, price)| *price)
        .unwrap_or(0.0);

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
    // Only do this cleanup every ~10 calls to reduce lock contention
    static mut CLEANUP_COUNTER: u32 = 0;
    let should_cleanup = unsafe {
        CLEANUP_COUNTER += 1;
        CLEANUP_COUNTER % 10 == 0
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
    let before_cooldown = tokens.len();
    let tokens_after_cooldown: Vec<Token> = tokens
        .into_iter()
        .filter(|t| !recently_closed_mints.contains(&t.mint))
        .collect();

    let removed_for_cooldown = before_cooldown.saturating_sub(tokens_after_cooldown.len());
    if removed_for_cooldown > 0 {
        log(
            LogTag::Trader,
            "COOLDOWN_FILTER",
            &format!(
                "⏳ Excluded {} tokens within {}m cooldown after close; {} remaining",
                removed_for_cooldown,
                POSITION_CLOSE_COOLDOWN_MINUTES,
                tokens_after_cooldown.len()
            )
        );
    }

    tokens_after_cooldown
}

// =============================================================================
// TOKEN PREPARATION AND TRADING FUNCTIONS
// =============================================================================

/// Prepare tokens for filtering and trading by fetching from database
/// Returns all available tokens ready for the filtering system to process
/// Now uses 10-minute caching to reduce database load and improve performance
pub async fn prepare_tokens(_cycle_start: std::time::Instant) -> Result<Vec<Token>, String> {
    // Check cache first - if valid, return cached tokens
    {
        let cache = FILTERED_TOKENS_CACHE.read().unwrap();
        if let Some(cached_data) = cache.as_ref() {
            if cached_data.is_valid() {
                let cache_age = cached_data.age_minutes();
                log(
                    LogTag::Trader,
                    "CACHE_HIT",
                    &format!(
                        "📋 Using cached tokens: {} tokens (age: {}min/{} min cache)",
                        cached_data.tokens.len(),
                        cache_age,
                        TOKEN_CACHE_DURATION_MINUTES
                    )
                );

                if is_debug_trader_enabled() {
                    log(
                        LogTag::Trader,
                        "DEBUG_CACHE_HIT",
                        &format!(
                            "🎯 Cache details: {} tokens cached at {:.1}min ago, {} min remaining",
                            cached_data.tokens.len(),
                            cache_age,
                            TOKEN_CACHE_DURATION_MINUTES - cache_age
                        )
                    );
                }

                return Ok(cached_data.tokens.clone());
            } else {
                let cache_age = cached_data.age_minutes();
                log(
                    LogTag::Trader,
                    "CACHE_EXPIRED",
                    &format!(
                        "⏰ Cache expired: age {}min > {}min limit, refreshing tokens",
                        cache_age,
                        TOKEN_CACHE_DURATION_MINUTES
                    )
                );
            }
        } else {
            log(
                LogTag::Trader,
                "CACHE_MISS",
                "🆕 No cached tokens found, performing fresh fetch and filter"
            );
        }
    }

    // Cache miss or expired - perform full token fetch and filtering
    log(
        LogTag::Trader,
        "TOKEN_REFRESH_START",
        "🔄 Starting fresh token fetch and filtering (cache refresh)"
    );

    use crate::filtering::{ filter_tokens_with_reasons, log_transaction_activity_stats };

    // Timeout for filtering operations - increased for larger token sets
    const FILTERING_TIMEOUT_SECS: u64 = 180;

    // 1. Fetch tokens from safe system
    log(LogTag::Trader, "TOKEN_FETCH_START", "🔄 Fetching tokens from database...");
    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "DEBUG_TOKEN_FETCH",
            "� Starting token fetch from get_all_tokens_by_liquidity()"
        );
    }

    let fetch_start = std::time::Instant::now();
    let tokens = {
        let tokens_from_module: Vec<Token> = match get_all_tokens_by_liquidity().await {
            Ok(api_tokens) => {
                let fetch_duration = fetch_start.elapsed();
                log(
                    LogTag::Trader,
                    "TOKEN_FETCH_SUCCESS",
                    &format!(
                        "✅ Fetched {} tokens from database in {:.3}s",
                        api_tokens.len(),
                        fetch_duration.as_secs_f32()
                    )
                );
                if is_debug_trader_enabled() {
                    log(
                        LogTag::Trader,
                        "DEBUG_TOKEN_FETCH_SUCCESS",
                        &format!(
                            "🔍 Token fetch details:\n  \
                             - Raw tokens from DB: {}\n  \
                             - First 5 tokens: [{}]\n  \
                             - Fetch duration: {:.3}s",
                            api_tokens.len(),
                            api_tokens
                                .iter()
                                .take(5)
                                .map(|t|
                                    format!(
                                        "{}({:.1}k)",
                                        t.symbol,
                                        t.liquidity
                                            .as_ref()
                                            .and_then(|l| l.usd)
                                            .unwrap_or(0.0) / 1000.0
                                    )
                                )
                                .collect::<Vec<_>>()
                                .join(", "),
                            fetch_duration.as_secs_f32()
                        )
                    );
                }
                // Convert ApiToken to Token for compatibility with existing code
                let mut converted_tokens: Vec<Token> = api_tokens
                    .into_iter()
                    .map(|api_token| api_token.into())
                    .collect();

                // Populate tokens with rugcheck_data and decimals from database
                let db = TokenDatabase::new().map_err(|e|
                    format!("Failed to create database: {}", e)
                );
                if let Ok(database) = db {
                    if
                        let Err(e) = database.populate_tokens_with_cached_data(
                            &mut converted_tokens
                        ).await
                    {
                        log(
                            LogTag::Trader,
                            "WARN",
                            &format!("Failed to populate tokens with cached data: {}", e)
                        );
                    }
                }

                converted_tokens
            }
            Err(e) => {
                let fetch_duration = fetch_start.elapsed();
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!(
                        "❌ Failed to get tokens from safe system after {:.3}s: {}",
                        fetch_duration.as_secs_f32(),
                        e
                    )
                );
                Vec::new()
            }
        };

        // Log total tokens available
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!("Total tokens from safe system: {}", tokens_from_module.len())
            );
        }

        // Count tokens with liquidity data
        let with_liquidity = tokens_from_module
            .iter()
            .filter(
                |token|
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
            )
            .count();

        if with_liquidity > 0 {
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "Processing {} tokens with liquidity (out of {} total)",
                    with_liquidity,
                    tokens_from_module.len()
                )
            );
        }

        // Keep tokens in liquidity order for consistent processing
        // Smart prioritization will happen after filtering
        tokens_from_module
    };

    // 2. Apply filtering with timeout protection
    log(
        LogTag::Trader,
        "FILTER_START",
        &format!(
            "🔍 Starting filtering of {} tokens (timeout: {}s)",
            tokens.len(),
            FILTERING_TIMEOUT_SECS
        )
    );
    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "DEBUG_FILTER_START",
            &format!(
                "� Filter details:\n  \
                 - Input tokens: {}\n  \
                 - With liquidity: {}\n  \
                 - Timeout: {}s\n  \
                 - Sample tokens: [{}]",
                tokens.len(),
                tokens
                    .iter()
                    .filter(
                        |t|
                            t.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0) > 0.0
                    )
                    .count(),
                FILTERING_TIMEOUT_SECS,
                tokens
                    .iter()
                    .take(3)
                    .map(|t|
                        format!(
                            "{}({:.1}k)",
                            t.symbol,
                            t.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0) / 1000.0
                        )
                    )
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        );
    }

    let filter_start = std::time::Instant::now();
    let filtering_result = tokio::time::timeout(
        std::time::Duration::from_secs(FILTERING_TIMEOUT_SECS),
        async {
            // Run filtering in spawn_blocking to avoid blocking the async runtime
            tokio::task::spawn_blocking({
                let tokens_copy = tokens.to_vec();
                move || filter_tokens_with_reasons(&tokens_copy)
            }).await
        }
    ).await;

    let filter_duration = filter_start.elapsed();
    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "FILTER_TIMING",
            &format!("⏱️ Filtering completed in {:.2}s", filter_duration.as_secs_f32())
        );
    }

    let (eligible_tokens, rejected_tokens) = match filtering_result {
        Ok(Ok(result)) => {
            log(
                LogTag::Trader,
                "FILTER_SUCCESS",
                &format!(
                    "✅ Filtering completed in {:.2}s: {}/{} tokens passed",
                    filter_duration.as_secs_f32(),
                    result.0.len(),
                    tokens.len()
                )
            );
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG_FILTER_SUCCESS",
                    &format!(
                        "📊 Filter results:\n  \
                         - Input tokens: {}\n  \
                         - Eligible tokens: {}\n  \
                         - Rejected tokens: {}\n  \
                         - Pass rate: {:.1}%\n  \
                         - Processing time: {:.2}s\n  \
                         - First 3 eligible: [{}]",
                        tokens.len(),
                        result.0.len(),
                        result.1.len(),
                        if tokens.len() > 0 {
                            ((result.0.len() as f64) / (tokens.len() as f64)) * 100.0
                        } else {
                            0.0
                        },
                        filter_duration.as_secs_f32(),
                        result.0
                            .iter()
                            .take(3)
                            .map(|t| t.symbol.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                );
            }
            result
        }
        Ok(Err(e)) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "❌ Token filtering task failed after {:.2}s: {}",
                    filter_duration.as_secs_f32(),
                    e
                )
            );
            return Err(format!("Token filtering task failed: {}", e));
        }
        Err(_) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "⏰ Token filtering TIMED OUT after {:.2}s (limit: {}s)",
                    filter_duration.as_secs_f32(),
                    FILTERING_TIMEOUT_SECS
                )
            );
            return Err(format!("Token filtering timed out after {}s", FILTERING_TIMEOUT_SECS));
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
            )
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
                &format!("🚫 {} filtered out: {:?}", token.symbol, reason)
            );
        }
        if rejected_tokens.len() > sample_size {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!("... and {} more tokens filtered out", rejected_tokens.len() - sample_size)
            );
        }
    }

    // 6. Quick blacklist filtering only (no bulk validation)
    let mut tokens_after_blacklist = Vec::new();
    let mut blacklisted_count = 0;

    // Clean up blacklist periodically
    cleanup_invalid_pool_blacklist();

    for token in eligible_tokens {
        // Only skip tokens that are already known to be blacklisted (no RPC calls)
        if is_token_in_invalid_pool_blacklist(&token.mint) {
            blacklisted_count += 1;
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "POOL_BLACKLISTED",
                    &format!(
                        "⚫ Skipping blacklisted token: {} ({})",
                        token.symbol,
                        &token.mint[..8]
                    )
                );
            }
            continue;
        }
        tokens_after_blacklist.push(token);
    }

    if blacklisted_count > 0 {
        log(
            LogTag::Trader,
            "BLACKLIST_FILTER",
            &format!(
                "⚫ Filtered out {} blacklisted tokens, {} remaining",
                blacklisted_count,
                tokens_after_blacklist.len()
            )
        );
    }

    // 6b. Note: Per-token re-entry cooldown now applied after token retrieval for cache-independence
    // This ensures recently closed positions are always filtered with fresh data
    let tokens_after_blacklist = tokens_after_blacklist; // Cooldown will be applied by caller

    // 7. Smart token prioritization with confidence-based top tokens
    // Partition tokens to ensure ~50% are from actively monitored/history-rich set
    let pool_service = get_pool_service();
    let recent_pool_tokens = pool_service.get_tokens_with_recent_pools_infos(600).await; // 10 minutes window for active monitoring
    let recent_set: std::collections::HashSet<String> = recent_pool_tokens.into_iter().collect();
    let mut with_monitoring: Vec<Token> = Vec::new();
    let mut without_monitoring: Vec<Token> = Vec::new();
    for t in tokens_after_blacklist {
        if recent_set.contains(&t.mint) {
            with_monitoring.push(t);
        } else {
            without_monitoring.push(t);
        }
    }

    // Load DB-backed history presence via pool service lightweight cache (kept at startup)
    // Prefer tokens that have any history entries in memory
    let mut with_history: Vec<Token> = Vec::new();
    let mut no_history: Vec<Token> = Vec::new();
    for t in with_monitoring.into_iter() {
        let hist = pool_service.get_price_history(&t.mint).await;
        if !hist.is_empty() {
            with_history.push(t);
        } else {
            no_history.push(t);
        }
    }

    // Prioritize monitored tokens (with history first, then without)
    let mut prioritized_monitored = prioritize_tokens_for_checking(with_history);
    let mut prioritized_monitored_nohist = prioritize_tokens_for_checking(no_history);
    prioritized_monitored.append(&mut prioritized_monitored_nohist);

    // Also prioritize tokens not currently in monitoring
    let prioritized_others = prioritize_tokens_for_checking(without_monitoring);

    // Interleave ~50/50 between monitored and others
    let mut prioritized_tokens: Vec<Token> = Vec::with_capacity(
        prioritized_monitored.len() + prioritized_others.len()
    );
    let mut i = 0usize;
    let mut j = 0usize;
    while i < prioritized_monitored.len() || j < prioritized_others.len() {
        // Take up to one from monitored
        if i < prioritized_monitored.len() {
            prioritized_tokens.push(prioritized_monitored[i].clone());
            i += 1;
        }
        // Take up to one from others
        if j < prioritized_others.len() {
            prioritized_tokens.push(prioritized_others[j].clone());
            j += 1;
        }
    }

    // Get top 10 confidence tokens for priority monitoring
    let top_confidence_tokens = get_top_confidence_tokens(10);
    let top_confidence_mints: std::collections::HashSet<String> = top_confidence_tokens
        .iter()
        .map(|info| info.mint.clone())
        .collect();

    // Add top confidence tokens to the beginning if not already present
    let mut confidence_tokens_to_add = Vec::new();
    for confidence_info in top_confidence_tokens {
        // Check if token is already in prioritized list
        let already_present = prioritized_tokens.iter().any(|t| t.mint == confidence_info.mint);

        if !already_present {
            // Try to find token in original tokens list
            if let Some(token) = tokens.iter().find(|t| t.mint == confidence_info.mint) {
                confidence_tokens_to_add.push(token.clone());
                if is_debug_trader_enabled() {
                    log(
                        LogTag::Trader,
                        "CONFIDENCE_ADD",
                        &format!(
                            "📈 Adding high-confidence token: {} (confidence: {:.1}%, trend: {})",
                            confidence_info.symbol,
                            confidence_info.confidence,
                            confidence_info.trend
                        )
                    );
                }
            }
        }
    }

    // Prepend confidence tokens to prioritized list
    confidence_tokens_to_add.extend(prioritized_tokens);
    let mut prioritized_tokens = confidence_tokens_to_add;

    // Clean up stale entries periodically
    cleanup_stale_confidence_entries();

    if is_debug_trader_enabled() && !prioritized_tokens.is_empty() {
        let confidence_count = prioritized_tokens
            .iter()
            .filter(|t| top_confidence_mints.contains(&t.mint))
            .count();

        log(
            LogTag::Trader,
            "PRIORITY_ORDER",
            &format!(
                "📊 Prioritized {} tokens: confidence_top={}, drops={}, fair_rotation={}, others={}",
                prioritized_tokens.len(),
                confidence_count,
                prioritized_tokens
                    .iter()
                    .take(20)
                    .filter(|t| {
                        TOKEN_CHECK_TRACKER.read()
                            .unwrap()
                            .get(&t.mint)
                            .map(|info| info.had_recent_drop)
                            .unwrap_or(false)
                    })
                    .count(),
                prioritized_tokens
                    .iter()
                    .filter(|t| {
                        TOKEN_CHECK_TRACKER.read()
                            .unwrap()
                            .get(&t.mint)
                            .map(|info| info.check_count == 0)
                            .unwrap_or(true)
                    })
                    .count(),
                prioritized_tokens.len() - 10
            )
        );
    }

    // Apply hard cap to prioritized tokens to keep working set bounded
    if prioritized_tokens.len() > PREPARED_TOKENS_CAP {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "PREPARED_CAP",
                &format!(
                    "🔧 Capping prepared tokens: {} → {} (configured cap)",
                    prioritized_tokens.len(),
                    PREPARED_TOKENS_CAP
                )
            );
        }
        prioritized_tokens.truncate(PREPARED_TOKENS_CAP);
    }

    // Cache the filtered and prioritized tokens for future cycles
    {
        if prioritized_tokens.is_empty() {
            log(
                LogTag::Trader,
                "CACHE_SKIP_EMPTY",
                "⚠️ Not caching empty filtered token list; will retry next cycle"
            );
            return Ok(prioritized_tokens);
        }
        let mut cache = FILTERED_TOKENS_CACHE.write().unwrap();
        *cache = Some(CachedTokenData {
            tokens: prioritized_tokens.clone(),
            cached_at: Instant::now(),
            cache_duration_minutes: TOKEN_CACHE_DURATION_MINUTES,
        });

        log(
            LogTag::Trader,
            "CACHE_UPDATED",
            &format!(
                "💾 Cached {} filtered tokens for {}min reuse",
                prioritized_tokens.len(),
                TOKEN_CACHE_DURATION_MINUTES
            )
        );

        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG_CACHE_UPDATED",
                &format!(
                    "📝 Cache details: {} tokens stored, next refresh in {}min",
                    prioritized_tokens.len(),
                    TOKEN_CACHE_DURATION_MINUTES
                )
            );
        }
    }

    Ok(prioritized_tokens)
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

        let tokens = match prepare_tokens(cycle_start).await {
            Ok(prepared_tokens) => {
                // Apply cooldown filter with fresh data (never cached)
                let tokens = apply_cooldown_filter(prepared_tokens).await;

                log(
                    LogTag::Trader,
                    "CYCLE_PREPARED",
                    &format!(
                        "✅ Token preparation completed: {} eligible tokens ready for entry checking in {:.3}s",
                        tokens.len(),
                        cycle_start.elapsed().as_secs_f32()
                    )
                );
                if is_debug_trader_enabled() {
                    log(
                        LogTag::Trader,
                        "DEBUG_TOKENS_PREPARED",
                        &format!(
                            "🔍 First 5 eligible tokens: [{}]",
                            tokens
                                .iter()
                                .take(5)
                                .map(|t|
                                    format!(
                                        "{}({:.9}k)",
                                        t.symbol,
                                        t.liquidity
                                            .as_ref()
                                            .and_then(|l| l.usd)
                                            .unwrap_or(0.0) / 1000.0
                                    )
                                )
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    );
                }
                tokens
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!(
                        "❌ Token preparation failed after {:.3}s: {}",
                        cycle_start.elapsed().as_secs_f32(),
                        e
                    )
                );
                continue; // Skip this cycle and try again
            }
        };

        // Early return if no tokens to process
        if tokens.is_empty() {
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

        // Log filtering summary
        log_filtering_summary(&tokens);

        // Debug: Show top confidence tokens every few cycles
        if is_debug_trader_enabled() {
            let top_confidence = get_top_confidence_tokens(5);
            if !top_confidence.is_empty() {
                let confidence_summary: Vec<String> = top_confidence
                    .iter()
                    .map(|info| format!("{}:{:.1}%", info.symbol, info.confidence))
                    .collect();
                log(
                    LogTag::Trader,
                    "CONFIDENCE_TOP",
                    &format!("🎯 Top confidence tokens: [{}]", confidence_summary.join(", "))
                );
            }
        }

        // Capacity-aware scheduling: rotate through eligible tokens with a fixed per-cycle cap
        let total_tokens = tokens.len();
        let batch_size = std::cmp::min(MAX_TOKENS_PER_CYCLE, total_tokens);
        let start_raw = SCHEDULER_OFFSET.fetch_add(batch_size, Ordering::Relaxed);
        let start_idx = start_raw % total_tokens;

        let base_slice: Vec<Token> = if start_idx + batch_size <= total_tokens {
            tokens[start_idx..start_idx + batch_size].to_vec()
        } else {
            let first = tokens[start_idx..].to_vec();
            let remaining = batch_size - first.len();
            let mut combined = first;
            combined.extend_from_slice(&tokens[..remaining]);
            combined
        };

        // Interleave: ensure ~50% of scheduled are tokens with history/active monitoring
        // Guard this call with a timeout to prevent rare stalls blocking the cycle
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "SCHEDULER_MONITORED_FETCH",
                "⏳ Fetching monitored-set (recent pools infos ≤600s) with 2s timeout"
            );
        }
        let pool_service = get_pool_service();
        let monitored_vec = match
            tokio::time::timeout(
                Duration::from_secs(2),
                pool_service.get_tokens_with_recent_pools_infos(600)
            ).await
        {
            Ok(v) => v,
            Err(_) => {
                log(
                    LogTag::Trader,
                    "SCHEDULER_WARN",
                    "⚠️ Monitored-set fetch timed out after 2s; proceeding without bias this cycle"
                );
                Vec::new()
            }
        };
        let monitored_set: std::collections::HashSet<String> = monitored_vec.into_iter().collect();
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "SCHEDULER_MONITORED_FETCH",
                &format!(
                    "✅ Monitored-set ready: {} tokens with recent pools infos",
                    monitored_set.len()
                )
            );
        }

        // Wrap the entire token scheduling logic in a timeout to prevent hangs
        if is_debug_trader_enabled() {
            log(LogTag::Trader, "SCHEDULER_START", "🔄 Starting token scheduling with 5s timeout");
        }

        let scheduled_tokens = match
            tokio::time::timeout(Duration::from_secs(5), async {
                if is_debug_trader_enabled() {
                    log(
                        LogTag::Trader,
                        "SCHEDULER_SORT_START",
                        &format!(
                            "🔄 Starting token sorting: {} base tokens, {} monitored, batch_size: {}",
                            base_slice.len(),
                            monitored_set.len(),
                            batch_size
                        )
                    );
                }

                let mut monitored: Vec<Token> = Vec::new();
                let mut others: Vec<Token> = Vec::new();
                for t in base_slice {
                    if monitored_set.contains(&t.mint) {
                        monitored.push(t);
                    } else {
                        others.push(t);
                    }
                }

                if is_debug_trader_enabled() {
                    log(
                        LogTag::Trader,
                        "SCHEDULER_SORT_COMPLETE",
                        &format!(
                            "📊 Token sorting complete: {} monitored, {} others",
                            monitored.len(),
                            others.len()
                        )
                    );
                }

                // Cap monitored portion to 50% of batch
                let target_monitored = (batch_size / 2).max(1);
                let mut scheduled_tokens: Vec<Token> = Vec::with_capacity(batch_size);
                let mut i = 0usize;
                let mut loop_iterations = 0u32;
                const MAX_LOOP_ITERATIONS: u32 = 10000; // Defensive limit to prevent infinite loops

                while
                    scheduled_tokens.len() < batch_size &&
                    (i < monitored.len() || !others.is_empty())
                {
                    loop_iterations += 1;
                    if loop_iterations > MAX_LOOP_ITERATIONS {
                        log(
                            LogTag::Trader,
                            "SCHEDULER_LOOP_LIMIT",
                            &format!(
                                "⚠️ Scheduling loop hit iteration limit ({}), breaking. Scheduled: {}, Target: {}",
                                MAX_LOOP_ITERATIONS,
                                scheduled_tokens.len(),
                                batch_size
                            )
                        );
                        break;
                    }

                    let before_len = scheduled_tokens.len();
                    if scheduled_tokens.len() < target_monitored && i < monitored.len() {
                        scheduled_tokens.push(monitored[i].clone());
                        i += 1;
                    }
                    if scheduled_tokens.len() < batch_size {
                        if let Some(t) = others.pop() {
                            scheduled_tokens.push(t);
                        }
                    }

                    // Defensive check: ensure we're making progress
                    if scheduled_tokens.len() == before_len {
                        log(
                            LogTag::Trader,
                            "SCHEDULER_NO_PROGRESS",
                            &format!("⚠️ Scheduling loop made no progress at iteration {}. Breaking to prevent hang.", loop_iterations)
                        );
                        break;
                    }
                }

                if is_debug_trader_enabled() && loop_iterations > 100 {
                    log(
                        LogTag::Trader,
                        "SCHEDULER_LOOP_INFO",
                        &format!(
                            "🔄 Scheduling loop completed after {} iterations. Final: {} scheduled",
                            loop_iterations,
                            scheduled_tokens.len()
                        )
                    );
                }

                scheduled_tokens
            }).await
        {
            Ok(tokens) => tokens,
            Err(_) => {
                log(
                    LogTag::Trader,
                    "SCHEDULER_TIMEOUT",
                    "⚠️ Token scheduling timed out after 5s, using empty schedule to prevent cycle hang"
                );
                Vec::new() // Return empty to prevent the entire cycle from hanging
            }
        };

        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "SCHEDULER_SUCCESS",
                &format!(
                    "✅ Token scheduling completed: {} tokens ready for processing",
                    scheduled_tokens.len()
                )
            );
        }

        // Proactively warm prices and seed watchlist for the scheduled set
        if !scheduled_tokens.is_empty() {
            let warms_all: Vec<String> = scheduled_tokens
                .iter()
                .map(|t| t.mint.clone())
                .collect();
            let warms_watchlist = warms_all.clone();
            let warms_for_adhoc = warms_all; // moved into spawn
            let warm_count = warms_watchlist.len();

            // Best-effort: ad-hoc warmup queue (batch processed every 1s)
            tokio::spawn(async move {
                request_price_warmup_batch(&warms_for_adhoc).await;
            });

            // Light-touch: add to watchlist so monitoring loop can rotate updates
            // Use small batches implicitly handled by add_watchlist_tokens
            add_watchlist_tokens(&warms_watchlist).await;

            // Immediate: request batch priority calculation to get prices right away
            // This uses the calculator to compute multiple tokens quickly and feed cache/history
            let svc = get_pool_service();
            let _ = svc.request_priority_price_updates(&warms_watchlist).await;

            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "SCHEDULER_WARM_SEED",
                    &format!("🔥 Seeded {} scheduled tokens for warm-up and watchlist rotation", warm_count)
                );
            }
        }

        // Process scheduled tokens in parallel; for valid entries, send OpenPosition via PositionsHandle
        // Per-cycle aggregation counters (atomic to update from tasks)
        let price_available_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let price_unavailable_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let zero_history_warmed = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
        let handles_initial_size = scheduled_tokens.len(); // Track for summary logging

        // Log detailed information about token processing
        log(
            LogTag::Trader,
            "TOKEN_PROCESSING_START",
            &format!(
                "🔄 Starting token processing: {} tokens scheduled (of {} eligible). Preparation took {:.3}s",
                scheduled_tokens.len(),
                total_tokens,
                cycle_start.elapsed().as_secs_f32()
            )
        );

        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG_TOKEN_PROCESSING",
                &format!(
                    "📋 Token processing details:\n  \
                     - Total eligible tokens: {}\n  \
                     - Semaphore limit: {} concurrent checks\n  \
                     - Task timeout: {}s per token\n  \
                     - Tokens being processed: [{}]",
                    scheduled_tokens.len(),
                    ENTRY_CHECK_CONCURRENCY, // semaphore limit
                    TOKEN_CHECK_TASK_TIMEOUT_SECS,
                    scheduled_tokens
                        .iter()
                        .take(10)
                        .map(|t| t.symbol.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            );
        }

        let mut processed_count = 0;
        // Separate time tracking: allow generous budget for actual token processing
        // Don't penalize processing for slow preparation phase
        let processing_start = std::time::Instant::now();
        let processing_budget = Duration::from_secs_f64(
            (ENTRY_MONITOR_INTERVAL_SECS as f64) * TIME_BUDGET_FRACTION
        );

        for token in scheduled_tokens.iter() {
            processed_count += 1;

            // Use processing time (not total cycle time) for budget check
            // This allows token processing even if preparation took a long time
            if processing_start.elapsed() >= processing_budget {
                log(
                    LogTag::Trader,
                    "TIME_BUDGET_REACHED",
                    &format!(
                        "⏱️ Processing time budget reached at {:.3}s (limit {:.3}s). Scheduled {}/{} tokens. Total cycle time: {:.3}s",
                        processing_start.elapsed().as_secs_f32(),
                        processing_budget.as_secs_f32(),
                        processed_count - 1,
                        scheduled_tokens.len(),
                        cycle_start.elapsed().as_secs_f32()
                    )
                );
                break;
            }

            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG_TOKEN_START",
                    &format!(
                        "🎯 Processing token {}/{}: {} ({})",
                        processed_count,
                        scheduled_tokens.len(),
                        token.symbol,
                        &token.mint[..8]
                    )
                );
            }
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
            let price_available_count = price_available_count.clone();
            let price_unavailable_count = price_unavailable_count.clone();
            let zero_history_warmed = zero_history_warmed.clone();
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
                        let pool_service = get_pool_service();

                        // ENHANCEMENT: Check if token has insufficient price history and force an update if needed
                        let history_before = pool_service.get_price_history(&token.mint).await;
                        let needs_history_boost = history_before.len() < 3;

                        if needs_history_boost {
                            // Force a price update by calling get_price which will cache the result
                            let warm_fut = get_price(
                                &token.mint,
                                Some(PriceOptions {
                                    warm_on_miss: true,
                                    ..PriceOptions::default()
                                }),
                                false
                            );
                            // Guard with short timeout to avoid per-task stalls
                            let _ = tokio::time::timeout(Duration::from_secs(3), warm_fut).await;
                            // Count zero-history warms for summary
                            zero_history_warmed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                            if is_debug_trader_enabled() {
                                log(
                                    LogTag::Trader,
                                    "HISTORY_BOOST",
                                    &format!(
                                        "🔄 Enqueued warm-up for {} (history entries before: {})",
                                        token.symbol,
                                        history_before.len()
                                    )
                                );
                            }
                        }

                        // Get current pool price (warm on miss)
                        let price_result = match
                            tokio::time::timeout(
                                Duration::from_secs(5),
                                get_price(
                                    &token.mint,
                                    Some(PriceOptions {
                                        warm_on_miss: true,
                                        ..PriceOptions::default()
                                    }),
                                    false
                                )
                            ).await
                        {
                            Ok(r) => r,
                            Err(_) => {
                                if is_debug_trader_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "PRICE_TIMEOUT",
                                        &format!(
                                            "⏱️ get_price timed out (5s) for {} ({})",
                                            token.symbol,
                                            token.mint
                                        )
                                    );
                                }
                                None
                            }
                        };

                        // Try to extract current price; if missing, do one short retry
                        let mut current_price_opt: Option<f64> = match &price_result {
                            Some(result) =>
                                match result.sol_price() {
                                    Some(p) if p > 0.0 && p.is_finite() => Some(p),
                                    _ => None,
                                }
                            None => None,
                        };

                        if current_price_opt.is_none() {
                            // Optional short retry after warm-up enqueue
                            tokio::time::sleep(Duration::from_millis(600)).await;
                            let retry = match
                                tokio::time::timeout(
                                    Duration::from_secs(3),
                                    get_price(
                                        &token.mint,
                                        Some(PriceOptions {
                                            warm_on_miss: false,
                                            ..PriceOptions::default()
                                        }),
                                        false
                                    )
                                ).await
                            {
                                Ok(r) => r,
                                Err(_) => None,
                            };
                            if let Some(r) = retry {
                                if let Some(p2) = r.sol_price() {
                                    if p2 > 0.0 && p2.is_finite() {
                                        if is_debug_trader_enabled() {
                                            log(
                                                LogTag::Trader,
                                                "PRICE_RETRY_HIT",
                                                &format!(
                                                    "✅ Retry hit for {} ({}) at {:.9} SOL",
                                                    token.symbol,
                                                    token.mint,
                                                    p2
                                                )
                                            );
                                        }
                                        current_price_opt = Some(p2);
                                    }
                                }
                            }
                        }

                        let current_price = match current_price_opt {
                            Some(p) => p,
                            None => {
                                // Update tracking even for failed price fetches
                                update_token_check_info(
                                    &token.mint,
                                    None,
                                    false,
                                    false,
                                    price_result.as_ref()
                                );
                                let error_detail = price_result
                                    .as_ref()
                                    .and_then(|r| r.error.as_ref())
                                    .cloned()
                                    .unwrap_or_else(|| "retry_failed".to_string());

                                // Check if this is a structural pool failure that should trigger blacklisting
                                // Only blacklist on permanent issues, NOT temporary price unavailability
                                if let Some(price_result) = price_result.as_ref() {
                                    if let Some(error) = &price_result.error {
                                        if
                                            error.contains("Unsupported pool program") ||
                                            error.contains("unknown pool program") ||
                                            error.contains("decode failed") ||
                                            error.contains("Failed to decode pool") ||
                                            error.contains("Invalid pool program")
                                        {
                                            add_to_invalid_pool_blacklist(
                                                &token.mint,
                                                &token.symbol,
                                                error
                                            );
                                        }
                                    }
                                }

                                if is_debug_trader_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "PRICE_FAIL",
                                        &format!(
                                            "❌ No valid price for {} ({}){} - skipping",
                                            token.symbol,
                                            token.mint,
                                            format!(": {}", error_detail)
                                        )
                                    );
                                }

                                // Trader-controlled: ensure it's on watchlist for background calculation
                                // But only if it's not a permanent pool issue
                                if !is_token_in_invalid_pool_blacklist(&token.mint) {
                                    ensure_watchlist_on_price_fail(
                                        &token.mint,
                                        &token.symbol,
                                        &error_detail
                                    ).await;
                                }

                                price_unavailable_count.fetch_add(
                                    1,
                                    std::sync::atomic::Ordering::Relaxed
                                );
                                return;
                            }
                        };

                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "PRICE_CHECK",
                                &format!("💰 {} price: {:.9} SOL", token.symbol, current_price)
                            );
                        }
                        price_available_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                        // Check for recent drops
                        let had_recent_drop = check_token_for_recent_drop(&token).await;

                        // Entry decision delegated to entry::should_buy
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "ENTRY_CHECK",
                                &format!(
                                    "🔍 Checking entry criteria for {} at {:.9} SOL",
                                    token.symbol,
                                    current_price
                                )
                            );
                        }

                        let entry_start = std::time::Instant::now();
                        let (approved, confidence, reason) = crate::entry::should_buy(&token).await;
                        let entry_duration = entry_start.elapsed();

                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "ENTRY_RESULT",
                                &format!(
                                    "📊 Entry check for {} completed in {:.3}s: {} (confidence: {:.1}%, reason: {})",
                                    token.symbol,
                                    entry_duration.as_secs_f32(),
                                    if approved {
                                        "APPROVED"
                                    } else {
                                        "REJECTED"
                                    },
                                    confidence,
                                    reason
                                )
                            );
                        }

                        // Update confidence tracking system
                        update_token_confidence(
                            &token.mint,
                            &token.symbol,
                            confidence,
                            Some(current_price)
                        );

                        // Update token tracking info
                        update_token_check_info(
                            &token.mint,
                            Some(current_price),
                            had_recent_drop,
                            true,
                            price_result.as_ref()
                        );

                        if !approved {
                            // Token passed filtering but doesn't meet entry criteria
                            // Add to watchlist for future monitoring
                            add_watchlist_tokens(&[token.mint.clone()]).await;

                            if is_debug_trader_enabled() {
                                log(
                                    LogTag::Trader,
                                    "WATCHLIST_ADD",
                                    &format!(
                                        "📝 Added {} to watchlist (confidence: {:.1}%, reason: {}) [Drop: {}]",
                                        &token.symbol,
                                        confidence,
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
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "ENTRY_APPROVED",
                                &format!(
                                    "🚀 ENTRY APPROVED: {} at {:.9} SOL (confidence: {:.1}%, drop: {})",
                                    &token.symbol,
                                    current_price,
                                    confidence,
                                    if had_recent_drop {
                                        "YES"
                                    } else {
                                        "NO"
                                    }
                                )
                            );
                        }

                        // Compute percent change from recent history if available
                        let change = {
                            let history = pool_service.get_price_history(&token.mint).await;
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
                            let Some(price_result) = (match
                                tokio::time::timeout(
                                    Duration::from_secs(3),
                                    get_price(&token.mint, Some(PriceOptions::default()), false)
                                ).await
                            {
                                Ok(r) => r,
                                Err(_) => None,
                            })
                        {
                            let reserve_sol = price_result.reserve_sol.unwrap_or(0.0);
                            if reserve_sol < 0.0 {
                                Some("INVALID".to_string())
                            } else {
                                let tier = match reserve_sol {
                                    x if x < 5.0 => "MICRO", // < 5 SOL (~$1K at $200/SOL)
                                    x if x < 50.0 => "SMALL", // 5-50 SOL (~$1K-$10K)
                                    x if x < 250.0 => "MEDIUM", // 50-250 SOL (~$10K-$50K)
                                    x if x < 1250.0 => "LARGE", // 250-1250 SOL (~$50K-$250K)
                                    x if x < 5000.0 => "XLARGE", // 1250-5000 SOL (~$250K-$1M)
                                    _ => "MEGA", // > 5000 SOL (~$1M+)
                                };
                                Some(tier.to_string())
                            }
                        } else {
                            Some("UNKNOWN".to_string())
                        };

                        // Check current position limits before attempting to open
                        if is_debug_trader_enabled() {
                            let current_positions =
                                crate::positions::get_open_positions_count().await;
                            log(
                                LogTag::Trader,
                                "POSITION_LIMITS",
                                &format!(
                                    "📊 Position limit check: {}/{} open positions before attempting buy for {}",
                                    current_positions,
                                    MAX_OPEN_POSITIONS,
                                    token.symbol
                                )
                            );
                        }

                        // Open position directly
                        if is_debug_trader_enabled() {
                            log(
                                LogTag::Trader,
                                "POSITION_OPENING",
                                &format!(
                                    "📈 Opening position for {} at {:.9} SOL (size: {} SOL)",
                                    token.symbol,
                                    current_price,
                                    TRADE_SIZE_SOL
                                )
                            );
                        }

                        let position_start = std::time::Instant::now();
                        let position_result = crate::positions::open_position_direct(
                            &token,
                            current_price,
                            change,
                            TRADE_SIZE_SOL,
                            liquidity_tier,
                            profit_min,
                            profit_max
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
                                        token.symbol,
                                        position_duration.as_secs_f32()
                                    )
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Trader,
                                    "POSITION_FAILED",
                                    &format!(
                                        "❌ Failed to open position for {} after {:.3}s: {}",
                                        token.symbol,
                                        position_duration.as_secs_f32(),
                                        e
                                    )
                                );
                            }
                        }

                        // Add to OHLCV watch list as open position for priority monitoring
                        if position_result.is_ok() {
                            if
                                let Ok(ohlcv_service) =
                                    crate::tokens::get_ohlcv_service_clone().await
                            {
                                ohlcv_service.add_to_watch_list(&token.mint, true).await; // true = open position
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
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "COLLECT_START",
                &format!(
                    "⏳ Collecting {} token tasks with {}s overall timeout",
                    handles.len(),
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

        // Print per-cycle summary for prices and checks
        log_cycle_price_summary(
            price_available_count.load(std::sync::atomic::Ordering::Relaxed),
            price_unavailable_count.load(std::sync::atomic::Ordering::Relaxed),
            zero_history_warmed.load(std::sync::atomic::Ordering::Relaxed)
        );

        // Add cycle summary logging
        if is_debug_trader_enabled() {
            let final_positions_count = crate::positions::get_open_positions_count().await;
            let actual_tasks = handles_initial_size; // scheduled count
            log(
                LogTag::Trader,
                "CYCLE_SUMMARY",
                &format!(
                    "🔄 Cycle summary: Scheduled {}/{} tokens → {} tasks spawned → Positions: {}/{}",
                    actual_tasks,
                    total_tokens,
                    actual_tasks,
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

            // Use the pool service to request priority updates
            let updated_count = crate::tokens::request_priority_updates_for_open_positions().await;

            if is_debug_trader_enabled() {
                debug_trader_log(
                    "PRIORITY_RESULT",
                    &format!(
                        "Priority updates completed: {}/{} successful",
                        updated_count,
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
                    // Get price data with warm-up on miss to reduce stale/no-cache reads
                    let price_result = get_price(
                        &mint,
                        Some(PriceOptions { warm_on_miss: true, ..PriceOptions::default() }),
                        false
                    ).await;

                    // Extract best available price and price info
                    if let Some(result) = price_result {
                        let best_price = result.sol_price();
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
        let price_map: std::collections::HashMap<
            String,
            (f64, crate::tokens::PriceResult)
        > = price_results
            .into_iter()
            .filter_map(|(mint, result_opt)| {
                result_opt.map(|(price, price_result)| (mint, (price, price_result)))
            })
            .collect();

        // Now process each position with async calls (mutex is released)
        for position in open_positions_data.into_iter() {
            let position = position; // local copy for calculations/logs

            // Get current price and price result from our parallel fetch results
            if let Some((current_price, price_result)) = price_map.get(&position.mint) {
                let current_price = *current_price;
                if current_price > 0.0 && current_price.is_finite() {
                    // Send price update to positions manager for tracking
                    let _tracking_result = crate::positions::update_position_tracking(
                        &position.mint,
                        current_price,
                        price_result
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

                    if should_exit {
                        // CRITICAL: Check pool availability before selling
                        let pool_service = get_pool_service();
                        let has_pool_availability = pool_service.check_token_availability(
                            &position.mint
                        ).await;

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

                        // Fetch full token from database
                        let Some(full_token) = crate::tokens::get_token_from_db(
                            &position.mint
                        ).await else {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Token not found in DB for mint {} — skipping sell",
                                    position.mint
                                )
                            );
                            continue;
                        };

                        log(
                            LogTag::Trader,
                            "SELL",
                            &format!(
                                "Sell signal for {} ({}) - P&L: {:.2}% ({:.6} SOL) - SHOULD EXIT",
                                position.symbol,
                                position.mint,
                                pnl_percent,
                                pnl_sol
                            )
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
            for (position, token, exit_price, _exit_time, _sell_urgency) in positions_to_close {
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
                // We already have the position from the analysis phase for logging only
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    // CRITICAL OPERATION PROTECTION - Prevent shutdown during sell
                    let _guard = CriticalOperationGuard::new(&format!("SELL_{}", token.symbol));

                    let position = position;
                    let token_symbol = token.symbol.clone();

                    // Check for shutdown before starting sell operation (non-blocking check)
                    let shutdown_check = tokio::time::timeout(
                        Duration::from_millis(SELL_OPERATION_SHUTDOWN_CHECK_MS),
                        shutdown_for_task.notified()
                    ).await;
                    if shutdown_check.is_ok() {
                        return false;
                    }

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(
                            Duration::from_secs(SELL_OPERATION_SMART_TIMEOUT_SECS),
                            async {
                                crate::positions
                                    ::close_position_direct(
                                        &position.mint,
                                        &token,
                                        exit_price,
                                        "Trading decision".to_string(),
                                        Utc::now()
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
                            true
                        }
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to close position for {}: {}", token_symbol, e)
                            );
                            false
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Sell operation for {} timed out", token_symbol)
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
                                    Ok(success) => {
                                        completed += 1;
                                        if success {
                                            successful += 1;
                                        }
                                    }
                                    Err(_) => {
                                        completed += 1;
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
