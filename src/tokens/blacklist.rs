use crate::arguments::is_debug_blacklist_enabled;
use crate::global::TOKENS_DATABASE;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Duration as ChronoDuration, Utc };
use once_cell::sync::Lazy;
/// Token blacklist system for managing problematic tokens
/// Uses database storage for persistence and performance
use rusqlite::{ Connection, Result as SqliteResult };
use serde::{ Deserialize, Serialize };
use std::collections::{ HashMap, HashSet };
use std::sync::Mutex;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Low liquidity threshold in USD
pub const LOW_LIQUIDITY_THRESHOLD: f64 = 100.0;

/// Minimum token age in hours before tracking for blacklist
pub const MIN_AGE_HOURS: i64 = 2;

/// Maximum low liquidity occurrences before blacklisting
pub const MAX_LOW_LIQUIDITY_COUNT: u32 = 5;

/// Maximum retry attempts before blacklisting permanently failed decimal tokens
pub const MAX_DECIMAL_RETRY_ATTEMPTS: i32 = 3;

/// Maximum no-route failures before blacklisting
pub const MAX_NO_ROUTE_FAILURES: u32 = 5;

/// System and stable tokens that should always be excluded from trading
pub const SYSTEM_STABLE_TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
    "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj", // stSOL
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
    "11111111111111111111111111111111", // System Program (invalid token)
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", // Token-2022 Program
];

// =============================================================================
// DATABASE INITIALIZATION
// =============================================================================

/// Initialize blacklist database tables
fn init_blacklist_database() -> SqliteResult<()> {
    let conn = Connection::open(TOKENS_DATABASE)?;

    // Create blacklist table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS blacklist (
            mint TEXT PRIMARY KEY,
            symbol TEXT NOT NULL,
            reason TEXT NOT NULL,
            first_occurrence TEXT NOT NULL,
            last_occurrence TEXT NOT NULL,
            occurrence_count INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        []
    )?;

    // Create liquidity tracking table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS liquidity_tracking (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mint TEXT NOT NULL,
            symbol TEXT NOT NULL,
            liquidity_usd REAL NOT NULL,
            token_age_hours INTEGER NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (mint) REFERENCES tokens(mint)
        )",
        []
    )?;

    // Create route failure tracking table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS route_failure_tracking (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mint TEXT NOT NULL,
            symbol TEXT NOT NULL,
            error_type TEXT NOT NULL,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (mint) REFERENCES tokens(mint)
        )",
        []
    )?;

    // Create indices for performance
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blacklist_reason ON blacklist(reason)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blacklist_updated ON blacklist(updated_at)", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_liquidity_tracking_mint ON liquidity_tracking(mint)",
        []
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_liquidity_tracking_timestamp ON liquidity_tracking(timestamp)",
        []
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_route_failure_tracking_mint ON route_failure_tracking(mint)",
        []
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_route_failure_tracking_timestamp ON route_failure_tracking(timestamp)",
        []
    )?;

    Ok(())
}

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Token blacklist entry with tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistEntry {
    pub mint: String,
    pub symbol: String,
    pub reason: BlacklistReason,
    pub first_occurrence: DateTime<Utc>,
    pub last_occurrence: DateTime<Utc>,
    pub occurrence_count: u32,
    pub liquidity_checks: Vec<LiquidityCheck>,
}

/// Reasons for blacklisting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlacklistReason {
    LowLiquidity,
    PoorPerformance,
    ManualBlacklist,
    SystemToken, // System/program tokens
    StableToken, // Stable coins and major tokens
    ApiError, // Tokens that return API errors (502, etc.)
    NoRoute, // Tokens that consistently fail due to no routing available
}

/// Individual liquidity check record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityCheck {
    pub timestamp: DateTime<Utc>,
    pub liquidity_usd: f64,
    pub token_age_hours: i64,
}

// =============================================================================
// DATABASE BLACKLIST FUNCTIONS
// =============================================================================

/// Check if token is blacklisted in database
pub fn is_token_blacklisted_db(mint: &str) -> bool {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return false;
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return false;
        }
    };

    let mut stmt = match conn.prepare("SELECT 1 FROM blacklist WHERE mint = ?1") {
        Ok(stmt) => stmt,
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to prepare blacklist check query: {}", e)
            );
            return false;
        }
    };

    match stmt.exists([mint]) {
        Ok(exists) => exists,
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to check blacklist for {}: {}", mint, e)
            );
            false
        }
    }
}

/// Add token to blacklist in database
pub fn add_to_blacklist_db(mint: &str, symbol: &str, reason: BlacklistReason) -> bool {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return false;
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return false;
        }
    };

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let reason_str = match reason {
        BlacklistReason::LowLiquidity => "LowLiquidity",
        BlacklistReason::PoorPerformance => "PoorPerformance",
        BlacklistReason::ManualBlacklist => "ManualBlacklist",
        BlacklistReason::SystemToken => "SystemToken",
        BlacklistReason::StableToken => "StableToken",
        BlacklistReason::ApiError => "ApiError",
        BlacklistReason::NoRoute => "NoRoute",
    };

    let result = conn.execute(
        "INSERT OR REPLACE INTO blacklist (mint, symbol, reason, first_occurrence, last_occurrence, occurrence_count, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?4, 1, ?4)
         ON CONFLICT(mint) DO UPDATE SET 
         last_occurrence = ?4, 
         occurrence_count = occurrence_count + 1,
         updated_at = ?4",
        [mint, symbol, reason_str, &now]
    );

    match result {
        Ok(_) => {
            log(
                LogTag::Blacklist,
                "ADDED",
                &format!("Blacklisted {} ({}) - {}", symbol, mint, reason_str)
            );
            // Refresh cache after adding to blacklist
            refresh_blacklist_cache();
            true
        }
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to add {} to blacklist: {}", mint, e));
            false
        }
    }
}

/// Track liquidity for a token in database
pub fn track_liquidity_db(
    mint: &str,
    symbol: &str,
    liquidity_usd: f64,
    token_age_hours: i64
) -> bool {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return true; // Allow processing if we can't track
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return true; // Allow processing if we can't track
        }
    };

    // Add liquidity tracking record
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    if
        let Err(e) = conn.execute(
            "INSERT INTO liquidity_tracking (mint, symbol, liquidity_usd, token_age_hours, timestamp) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
            [mint, symbol, &liquidity_usd.to_string(), &token_age_hours.to_string(), &now]
        )
    {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to track liquidity for {}: {}", mint, e));
        return true; // Allow processing if we can't track
    }

    // Check if we should blacklist due to low liquidity
    if liquidity_usd < LOW_LIQUIDITY_THRESHOLD && token_age_hours >= MIN_AGE_HOURS {
        // Count low liquidity occurrences
        let mut stmt = match
            conn.prepare(
                "SELECT COUNT(*) FROM liquidity_tracking 
             WHERE mint = ?1 AND liquidity_usd < ?2 AND datetime(timestamp) > datetime('now', '-7 days')"
            )
        {
            Ok(stmt) => stmt,
            Err(e) => {
                log(
                    LogTag::Blacklist,
                    "ERROR",
                    &format!("Failed to prepare liquidity count query: {}", e)
                );
                return true;
            }
        };

        let low_count: i64 = match
            stmt.query_row([mint, &LOW_LIQUIDITY_THRESHOLD.to_string()], |row| { row.get(0) })
        {
            Ok(count) => count,
            Err(e) => {
                log(
                    LogTag::Blacklist,
                    "ERROR",
                    &format!("Failed to count low liquidity for {}: {}", mint, e)
                );
                return true;
            }
        };

        log(
            LogTag::Blacklist,
            "TRACK",
            &format!(
                "Low liquidity for {} ({}): ${:.2} USD (count: {})",
                symbol,
                mint,
                liquidity_usd,
                low_count
            )
        );

        if low_count >= (MAX_LOW_LIQUIDITY_COUNT as i64) {
            add_to_blacklist_db(mint, symbol, BlacklistReason::LowLiquidity);
            return false; // Don't allow processing
        }
    }

    true
}

/// Track route failure for a token in database
pub fn track_route_failure_db(mint: &str, symbol: &str, error_type: &str) -> bool {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return true; // Allow processing if we can't track
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return true; // Allow processing if we can't track
        }
    };

    // Add route failure tracking record
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    if
        let Err(e) = conn.execute(
            "INSERT INTO route_failure_tracking (mint, symbol, error_type, timestamp) 
         VALUES (?1, ?2, ?3, ?4)",
            [mint, symbol, error_type, &now]
        )
    {
        log(
            LogTag::Blacklist,
            "ERROR",
            &format!("Failed to track route failure for {}: {}", mint, e)
        );
        return true; // Allow processing if we can't track
    }

    // Count no route failures in the last 7 days
    let mut stmt = match
        conn.prepare(
            "SELECT COUNT(*) FROM route_failure_tracking 
         WHERE mint = ?1 AND timestamp > datetime('now', '-7 days')"
        )
    {
        Ok(stmt) => stmt,
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to prepare route failure count query: {}", e)
            );
            return true;
        }
    };

    let failure_count: i64 = match stmt.query_row([mint], |row| row.get(0)) {
        Ok(count) => count,
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to count route failures for {}: {}", mint, e)
            );
            return true;
        }
    };

    log(
        LogTag::Blacklist,
        "TRACK",
        &format!(
            "Route failure for {} ({}): {} (count: {})",
            symbol,
            mint,
            error_type,
            failure_count
        )
    );

    if failure_count >= (MAX_NO_ROUTE_FAILURES as i64) {
        log(
            LogTag::Blacklist,
            "BLACKLIST",
            &format!("Blacklisting {} ({}) after {} route failures", symbol, mint, failure_count)
        );
        add_to_blacklist_db(mint, symbol, BlacklistReason::NoRoute);
        return false; // Don't allow processing
    }

    true
}

/// Get blacklist statistics from database
pub fn get_blacklist_stats_db() -> Option<BlacklistStats> {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return None;
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return None;
        }
    };

    // Get total blacklisted count
    let total_blacklisted: usize = match
        conn.query_row("SELECT COUNT(*) FROM blacklist", [], |row| { row.get::<_, i64>(0) })
    {
        Ok(count) => count as usize,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to get blacklist count: {}", e));
            return None;
        }
    };

    // Get tracked tokens count
    let total_tracked: usize = match
        conn.query_row(
            "SELECT COUNT(DISTINCT mint) FROM liquidity_tracking WHERE datetime(timestamp) > datetime('now', '-7 days')",
            [],
            |row| row.get::<_, i64>(0)
        )
    {
        Ok(count) => count as usize,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to get tracked count: {}", e));
            return None;
        }
    };

    // Get reason breakdown
    let mut reason_breakdown = HashMap::new();
    let mut stmt = match conn.prepare("SELECT reason, COUNT(*) FROM blacklist GROUP BY reason") {
        Ok(stmt) => stmt,
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to prepare reason breakdown query: {}", e)
            );
            return None;
        }
    };

    let rows = match
        stmt.query_map([], |row| { Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)) })
    {
        Ok(rows) => rows,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to get reason breakdown: {}", e));
            return None;
        }
    };

    for row in rows {
        if let Ok((reason, count)) = row {
            reason_breakdown.insert(reason, count as usize);
        }
    }

    Some(BlacklistStats {
        total_blacklisted,
        total_tracked,
        reason_breakdown,
    })
}

/// Remove old liquidity tracking data (older than 7 days)
pub fn cleanup_old_blacklist_data() -> bool {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return false;
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return false;
        }
    };

    match
        conn.execute(
            "DELETE FROM liquidity_tracking WHERE datetime(timestamp) < datetime('now', '-7 days')",
            []
        )
    {
        Ok(deleted) => {
            if deleted > 0 {
                log(
                    LogTag::Blacklist,
                    "CLEANUP",
                    &format!("Cleaned {} old liquidity tracking records", deleted)
                );
            }
        }
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to cleanup old liquidity tracking data: {}", e)
            );
        }
    }

    // Clean up old route failure tracking data
    match
        conn.execute(
            "DELETE FROM route_failure_tracking WHERE datetime(timestamp) < datetime('now', '-7 days')",
            []
        )
    {
        Ok(deleted) => {
            if deleted > 0 {
                log(
                    LogTag::Blacklist,
                    "CLEANUP",
                    &format!("Cleaned {} old route failure tracking records", deleted)
                );
            }
            true
        }
        Err(e) => {
            log(
                LogTag::Blacklist,
                "ERROR",
                &format!("Failed to cleanup old route failure data: {}", e)
            );
            false
        }
    }
}

/// Get all blacklisted token mints (for efficient filtering)
pub fn get_blacklisted_mints() -> Vec<String> {
    if let Err(e) = init_blacklist_database() {
        log(LogTag::Blacklist, "ERROR", &format!("Failed to init blacklist database: {}", e));
        return Vec::new();
    }

    let conn = match Connection::open(TOKENS_DATABASE) {
        Ok(conn) => conn,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to connect to database: {}", e));
            return Vec::new();
        }
    };

    let mut stmt = match conn.prepare("SELECT mint FROM blacklist") {
        Ok(stmt) => stmt,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to prepare blacklist query: {}", e));
            return Vec::new();
        }
    };

    let rows = match stmt.query_map([], |row| row.get::<_, String>(0)) {
        Ok(rows) => rows,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to get blacklisted mints: {}", e));
            return Vec::new();
        }
    };

    let mut mints = Vec::new();
    for row in rows {
        if let Ok(mint) = row {
            mints.push(mint);
        }
    }

    mints
}

// =============================================================================
// =============================================================================
// BLACKLIST STATISTICS
// =============================================================================

/// Blacklist statistics
#[derive(Debug, Clone)]
pub struct BlacklistStats {
    pub total_blacklisted: usize,
    pub total_tracked: usize,
    pub reason_breakdown: HashMap<String, usize>,
}

impl Default for BlacklistStats {
    fn default() -> Self {
        Self {
            total_blacklisted: 0,
            total_tracked: 0,
            reason_breakdown: HashMap::new(),
        }
    }
}

// =============================================================================
// =============================================================================
// GLOBAL STATE & CACHING
// =============================================================================

/// Simplified blacklist cache for performance
static TOKEN_BLACKLIST_CACHE: Lazy<Mutex<HashSet<String>>> = Lazy::new(||
    Mutex::new(HashSet::new())
);

/// Cache refresh timestamp
static BLACKLIST_CACHE_LAST_REFRESH: Lazy<Mutex<Option<DateTime<Utc>>>> = Lazy::new(||
    Mutex::new(None)
);

/// Cache refresh interval (5 minutes)
const CACHE_REFRESH_INTERVAL_MINUTES: i64 = 5;

/// Refresh blacklist cache from database
fn refresh_blacklist_cache() -> bool {
    let mints = get_blacklisted_mints();

    let mut cache = match TOKEN_BLACKLIST_CACHE.lock() {
        Ok(cache) => cache,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to lock blacklist cache: {}", e));
            return false;
        }
    };

    cache.clear();
    for mint in mints {
        cache.insert(mint);
    }

    let mut last_refresh = match BLACKLIST_CACHE_LAST_REFRESH.lock() {
        Ok(last) => last,
        Err(e) => {
            log(LogTag::Blacklist, "ERROR", &format!("Failed to lock cache timestamp: {}", e));
            return false;
        }
    };
    *last_refresh = Some(Utc::now());

    log(
        LogTag::Blacklist,
        "CACHE",
        &format!("Refreshed blacklist cache with {} tokens", cache.len())
    );
    true
}

/// Check if cache needs refresh
fn cache_needs_refresh() -> bool {
    let last_refresh = match BLACKLIST_CACHE_LAST_REFRESH.lock() {
        Ok(last) => last,
        Err(_) => {
            return true;
        }
    };

    match *last_refresh {
        Some(last) => {
            let minutes_since_refresh = Utc::now().signed_duration_since(last).num_minutes();
            minutes_since_refresh >= CACHE_REFRESH_INTERVAL_MINUTES
        }
        None => true,
    }
}

/// Fast cached blacklist check
pub fn is_token_blacklisted_cached(mint: &str) -> bool {
    // Refresh cache if needed
    if cache_needs_refresh() {
        refresh_blacklist_cache();
    }

    let cache = match TOKEN_BLACKLIST_CACHE.lock() {
        Ok(cache) => cache,
        Err(_) => {
            // Fall back to direct database check if cache fails
            return is_token_blacklisted_db(mint);
        }
    };

    cache.contains(mint)
}

// =============================================================================
// HELPER FUNCTIONS (Updated for Database Integration)
// =============================================================================

/// Check if token is blacklisted (high-performance with cache)
pub fn is_token_blacklisted(mint: &str) -> bool {
    is_token_blacklisted_cached(mint)
}

/// Check if token is excluded from trading (main filtering function)
pub fn is_token_excluded_from_trading(mint: &str) -> bool {
    // Check system/stable tokens first (fastest)
    if is_system_or_stable_token(mint) {
        return true;
    }

    // Use cached check for maximum performance
    if is_token_blacklisted_cached(mint) {
        if is_debug_blacklist_enabled() {
            log(LogTag::Blacklist, "DEBUG", &format!("Blocked trading for {}: blacklisted", mint));
        }
        return true;
    }
    false
}

/// Add decimal fetch failure to blacklist
pub fn add_decimal_failure_to_blacklist(mint: &str, symbol: &str, attempts: u32) -> bool {
    log(
        LogTag::Blacklist,
        "DECIMAL_FAIL",
        &format!(
            "Adding {} ({}) to blacklist after {} decimal fetch attempts",
            symbol,
            mint,
            attempts
        )
    );
    let result = add_to_blacklist_db(mint, symbol, BlacklistReason::ApiError);
    if result {
        // Force cache refresh on next check
        if let Ok(mut last_refresh) = BLACKLIST_CACHE_LAST_REFRESH.lock() {
            *last_refresh = None;
        }
    }
    result
}

/// Initialize blacklist system
pub fn initialize_blacklist_system() -> Result<(), Box<dyn std::error::Error>> {
    init_blacklist_database()?;
    refresh_blacklist_cache();
    Ok(())
}

/// Cleanup old data
pub fn cleanup_blacklist_data() -> bool {
    cleanup_old_blacklist_data()
}

// =============================================================================
// SYSTEM & STABLE TOKEN HANDLING
// =============================================================================

/// Check if token is a system or stable token that should be excluded from trading
pub fn is_system_or_stable_token(mint: &str) -> bool {
    SYSTEM_STABLE_TOKENS.contains(&mint)
}

/// Initialize system and stable tokens in blacklist database (run at startup)
pub fn initialize_system_stable_blacklist() {
    let mut tokens_added = 0;
    for &mint in SYSTEM_STABLE_TOKENS {
        if !is_token_blacklisted_cached(mint) {
            let (symbol, reason) = match mint {
                "So11111111111111111111111111111111111111112" => {
                    ("SOL", BlacklistReason::StableToken)
                }
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => {
                    ("USDC", BlacklistReason::StableToken)
                }
                "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => {
                    ("USDT", BlacklistReason::StableToken)
                }
                "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj" => {
                    ("stSOL", BlacklistReason::StableToken)
                }
                "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => {
                    ("mSOL", BlacklistReason::StableToken)
                }
                "11111111111111111111111111111111" => ("SYSTEM", BlacklistReason::SystemToken),
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                    ("TOKEN_PROGRAM", BlacklistReason::SystemToken)
                }
                "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => {
                    ("TOKEN_2022", BlacklistReason::SystemToken)
                }
                _ => ("UNKNOWN", BlacklistReason::SystemToken),
            };

            if add_to_blacklist_db(mint, symbol, reason) {
                tokens_added += 1;
            }
        }
    }

    // Force cache refresh if we added any tokens
    if tokens_added > 0 {
        if let Ok(mut last_refresh) = BLACKLIST_CACHE_LAST_REFRESH.lock() {
            *last_refresh = None;
        }
    }

    log(
        LogTag::Blacklist,
        "INIT",
        &format!("System and stable tokens initialized in blacklist database ({} added)", tokens_added)
    );
}

// =============================================================================
// BLACKLIST SUMMARY FOR DASHBOARD
// =============================================================================

/// Summary of blacklist statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistSummary {
    pub total_count: usize,
    pub low_liquidity_count: usize,
    pub no_route_count: usize,
    pub api_error_count: usize,
    pub system_token_count: usize,
    pub stable_token_count: usize,
    pub manual_count: usize,
    pub poor_performance_count: usize,
}

/// Get blacklist summary statistics
pub fn get_blacklist_summary() -> Result<BlacklistSummary, String> {
    if let Err(e) = init_blacklist_database() {
        return Err(format!("Failed to init blacklist database: {}", e));
    }

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Failed to connect to database: {}", e)
    )?;

    // Get total count
    let total_count: usize = conn
        .query_row("SELECT COUNT(*) FROM blacklist", [], |row| row.get(0))
        .map_err(|e| format!("Failed to get total count: {}", e))?;

    // Get counts by reason
    let mut stmt = conn
        .prepare("SELECT reason, COUNT(*) FROM blacklist GROUP BY reason")
        .map_err(|e| format!("Failed to prepare summary query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            let reason: String = row.get(0)?;
            let count: usize = row.get(1)?;
            Ok((reason, count))
        })
        .map_err(|e| format!("Failed to execute summary query: {}", e))?;

    let mut low_liquidity_count = 0;
    let mut no_route_count = 0;
    let mut api_error_count = 0;
    let mut system_token_count = 0;
    let mut stable_token_count = 0;
    let mut manual_count = 0;
    let mut poor_performance_count = 0;

    for row_result in rows {
        if let Ok((reason, count)) = row_result {
            match reason.as_str() {
                "LowLiquidity" => {
                    low_liquidity_count = count;
                }
                "NoRoute" => {
                    no_route_count = count;
                }
                "ApiError" => {
                    api_error_count = count;
                }
                "SystemToken" => {
                    system_token_count = count;
                }
                "StableToken" => {
                    stable_token_count = count;
                }
                "ManualBlacklist" => {
                    manual_count = count;
                }
                "PoorPerformance" => {
                    poor_performance_count = count;
                }
                _ => {}
            }
        }
    }

    Ok(BlacklistSummary {
        total_count,
        low_liquidity_count,
        no_route_count,
        api_error_count,
        system_token_count,
        stable_token_count,
        manual_count,
        poor_performance_count,
    })
}
