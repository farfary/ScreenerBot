/// Failed pool analysis cache system
///
/// This module tracks pools that have failed analysis to prevent repeated attempts
/// and reduce log spam. Similar to the token decimals failed cache system.

use crate::global::TOKENS_DATABASE;
use crate::logger::{log, LogTag};
use crate::arguments::is_debug_pool_analyzer_enabled;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// How long to wait before retrying a failed pool analysis (in hours)
pub const FAILED_POOL_RETRY_HOURS: i64 = 6;

/// Maximum number of failures before permanently blacklisting a pool
pub const MAX_POOL_FAILURES: u32 = 5;

/// How long to keep failed pool records in database (in days)
pub const FAILED_POOL_RETENTION_DAYS: i64 = 7;

// =============================================================================
// TYPES
// =============================================================================

/// Information about a failed pool analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedPoolInfo {
    pub pool_id: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub program_id: String,
    pub error_message: String,
    pub failure_count: u32,
    pub first_failed_at: DateTime<Utc>,
    pub last_failed_at: DateTime<Utc>,
    pub is_permanent: bool,
}

impl FailedPoolInfo {
    /// Create a new failed pool info entry
    pub fn new(
        pool_id: &str,
        base_mint: &str,
        quote_mint: &str,
        program_id: &str,
        error_message: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            pool_id: pool_id.to_string(),
            base_mint: base_mint.to_string(),
            quote_mint: quote_mint.to_string(),
            program_id: program_id.to_string(),
            error_message: error_message.to_string(),
            failure_count: 1,
            first_failed_at: now,
            last_failed_at: now,
            is_permanent: false,
        }
    }

    /// Check if this pool can be retried (not permanent and enough time has passed)
    pub fn can_retry(&self) -> bool {
        if self.is_permanent {
            return false;
        }

        let hours_since_last_failure = Utc::now()
            .signed_duration_since(self.last_failed_at)
            .num_hours();

        hours_since_last_failure >= FAILED_POOL_RETRY_HOURS
    }

    /// Update failure information
    pub fn update_failure(&mut self, error_message: &str) {
        self.failure_count += 1;
        self.last_failed_at = Utc::now();
        self.error_message = error_message.to_string();

        // Mark as permanent if too many failures
        if self.failure_count >= MAX_POOL_FAILURES {
            self.is_permanent = true;
        }
    }
}

// =============================================================================
// GLOBAL CACHE
// =============================================================================

/// In-memory cache for fast lookups
static FAILED_POOL_CACHE: Lazy<Mutex<HashMap<String, FailedPoolInfo>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// =============================================================================
// DATABASE FUNCTIONS
// =============================================================================

/// Initialize the failed pool database table
fn init_failed_pools_database() -> Result<(), String> {
    let conn = Connection::open(TOKENS_DATABASE)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS failed_pools (
            pool_id TEXT PRIMARY KEY,
            base_mint TEXT NOT NULL,
            quote_mint TEXT NOT NULL,
            program_id TEXT NOT NULL,
            error_message TEXT NOT NULL,
            failure_count INTEGER NOT NULL DEFAULT 1,
            first_failed_at TEXT NOT NULL,
            last_failed_at TEXT NOT NULL,
            is_permanent INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| format!("Failed to create failed_pools table: {}", e))?;

    // Create index for faster lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_failed_pools_last_failed 
         ON failed_pools(last_failed_at)",
        [],
    )
    .map_err(|e| format!("Failed to create index: {}", e))?;

    Ok(())
}

/// Save failed pool info to database
fn save_failed_pool_to_db(failed_info: &FailedPoolInfo) -> Result<(), String> {
    let conn = Connection::open(TOKENS_DATABASE)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO failed_pools 
         (pool_id, base_mint, quote_mint, program_id, error_message, failure_count, 
          first_failed_at, last_failed_at, is_permanent)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        [
            &failed_info.pool_id,
            &failed_info.base_mint,
            &failed_info.quote_mint,
            &failed_info.program_id,
            &failed_info.error_message,
            &failed_info.failure_count.to_string(),
            &failed_info.first_failed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            &failed_info.last_failed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            &(if failed_info.is_permanent { 1 } else { 0 }).to_string(),
        ],
    )
    .map_err(|e| format!("Failed to save failed pool: {}", e))?;

    Ok(())
}

/// Load failed pool info from database
fn load_failed_pool_from_db(pool_id: &str) -> Option<FailedPoolInfo> {
    let conn = Connection::open(TOKENS_DATABASE).ok()?;

    let mut stmt = conn
        .prepare(
            "SELECT pool_id, base_mint, quote_mint, program_id, error_message, 
                    failure_count, first_failed_at, last_failed_at, is_permanent
             FROM failed_pools WHERE pool_id = ?1"
        )
        .ok()?;

    let row = stmt.query_row([pool_id], |row| {
        let first_failed_str: String = row.get(6)?;
        let last_failed_str: String = row.get(7)?;
        
        let first_failed_at = DateTime::parse_from_str(&first_failed_str, "%Y-%m-%d %H:%M:%S UTC")
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
            
        let last_failed_at = DateTime::parse_from_str(&last_failed_str, "%Y-%m-%d %H:%M:%S UTC")
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(FailedPoolInfo {
            pool_id: row.get(0)?,
            base_mint: row.get(1)?,
            quote_mint: row.get(2)?,
            program_id: row.get(3)?,
            error_message: row.get(4)?,
            failure_count: row.get(5)?,
            first_failed_at,
            last_failed_at,
            is_permanent: row.get::<_, i32>(8)? != 0,
        })
    }).ok()?;

    Some(row)
}

/// Clean up old failed pool records
fn cleanup_old_failed_pools() -> Result<usize, String> {
    let conn = Connection::open(TOKENS_DATABASE)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let cutoff_date = Utc::now()
        .checked_sub_signed(chrono::Duration::days(FAILED_POOL_RETENTION_DAYS))
        .unwrap_or_else(Utc::now)
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    let deleted = conn
        .execute(
            "DELETE FROM failed_pools WHERE last_failed_at < ?1 AND is_permanent = 0",
            [&cutoff_date],
        )
        .map_err(|e| format!("Failed to cleanup old records: {}", e))?;

    Ok(deleted)
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Initialize the failed pool analysis cache system
pub fn initialize_failed_pool_cache() -> Result<(), String> {
    if let Err(e) = init_failed_pools_database() {
        log(
            LogTag::PoolAnalyzer,
            "ERROR",
            &format!("Failed to initialize failed pools database: {}", e)
        );
        return Err(e);
    }

    // Load existing failed pools into memory cache
    load_failed_pools_into_cache()?;

    if is_debug_pool_analyzer_enabled() {
        log(
            LogTag::PoolAnalyzer,
            "INFO",
            "Failed pool analysis cache initialized"
        );
    }

    Ok(())
}

/// Load all failed pools from database into memory cache
fn load_failed_pools_into_cache() -> Result<(), String> {
    let conn = Connection::open(TOKENS_DATABASE)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT pool_id, base_mint, quote_mint, program_id, error_message, 
                    failure_count, first_failed_at, last_failed_at, is_permanent
             FROM failed_pools"
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            let first_failed_str: String = row.get(6)?;
            let last_failed_str: String = row.get(7)?;
            
            let first_failed_at = DateTime::parse_from_str(&first_failed_str, "%Y-%m-%d %H:%M:%S UTC")
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
                
            let last_failed_at = DateTime::parse_from_str(&last_failed_str, "%Y-%m-%d %H:%M:%S UTC")
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(FailedPoolInfo {
                pool_id: row.get(0)?,
                base_mint: row.get(1)?,
                quote_mint: row.get(2)?,
                program_id: row.get(3)?,
                error_message: row.get(4)?,
                failure_count: row.get(5)?,
                first_failed_at,
                last_failed_at,
                is_permanent: row.get::<_, i32>(8)? != 0,
            })
        })
        .map_err(|e| format!("Failed to query failed pools: {}", e))?;

    let mut cache = FAILED_POOL_CACHE.lock().unwrap();
    cache.clear();

    let mut loaded_count = 0;
    for row in rows {
        match row {
            Ok(failed_info) => {
                cache.insert(failed_info.pool_id.clone(), failed_info);
                loaded_count += 1;
            }
            Err(e) => {
                log(
                    LogTag::PoolAnalyzer,
                    "WARN",
                    &format!("Failed to load failed pool record: {}", e)
                );
            }
        }
    }

    if is_debug_pool_analyzer_enabled() {
        log(
            LogTag::PoolAnalyzer,
            "DEBUG",
            &format!("Loaded {} failed pools into cache", loaded_count)
        );
    }

    Ok(())
}

/// Check if a pool has failed analysis and cannot be retried yet
pub fn is_pool_analysis_failed(pool_id: &Pubkey) -> bool {
    let pool_str = pool_id.to_string();

    // Check in-memory cache first
    if let Ok(cache) = FAILED_POOL_CACHE.lock() {
        if let Some(failed_info) = cache.get(&pool_str) {
            return !failed_info.can_retry();
        }
    }

    // If not in cache, check database
    if let Some(failed_info) = load_failed_pool_from_db(&pool_str) {
        // Add to cache for future lookups
        if let Ok(mut cache) = FAILED_POOL_CACHE.lock() {
            let can_retry = failed_info.can_retry();
            cache.insert(pool_str, failed_info);
            return !can_retry;
        }
    }

    false
}

/// Record a failed pool analysis attempt
pub fn record_failed_pool_analysis(
    pool_id: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    program_id: &Pubkey,
    error_message: &str,
) {
    let pool_str = pool_id.to_string();
    let base_str = base_mint.to_string();
    let quote_str = quote_mint.to_string();
    let program_str = program_id.to_string();

    let mut cache = match FAILED_POOL_CACHE.lock() {
        Ok(cache) => cache,
        Err(_) => {
            log(
                LogTag::PoolAnalyzer,
                "ERROR",
                "Failed to lock failed pool cache"
            );
            return;
        }
    };

    let failed_info = if let Some(existing) = cache.get_mut(&pool_str) {
        // Update existing failure
        existing.update_failure(error_message);
        existing.clone()
    } else {
        // Create new failure record
        FailedPoolInfo::new(&pool_str, &base_str, &quote_str, &program_str, error_message)
    };

    // Insert/update in cache
    cache.insert(pool_str.clone(), failed_info.clone());
    drop(cache); // Release lock before database operation

    // Save to database
    if let Err(e) = save_failed_pool_to_db(&failed_info) {
        log(
            LogTag::PoolAnalyzer,
            "ERROR",
            &format!("Failed to save failed pool to database: {}", e)
        );
    }

    if is_debug_pool_analyzer_enabled() {
        let token_str = if base_str.contains("So11111111111111111111111111111111111111112") {
            quote_str
        } else {
            base_str
        };

        log(
            LogTag::PoolAnalyzer,
            "CACHE_FAIL",
            &format!(
                "Cached failed analysis for pool {} (token {}, failure #{}, permanent: {}): {}",
                pool_str,
                token_str,
                failed_info.failure_count,
                failed_info.is_permanent,
                error_message
            )
        );
    }
}

/// Get statistics about failed pool analysis cache
pub fn get_failed_pool_stats() -> (usize, usize, usize) {
    let cache = match FAILED_POOL_CACHE.lock() {
        Ok(cache) => cache,
        Err(_) => return (0, 0, 0),
    };

    let total = cache.len();
    let permanent = cache.values().filter(|info| info.is_permanent).count();
    let retryable = cache.values().filter(|info| info.can_retry()).count();

    (total, permanent, retryable)
}

/// Clean up expired failed pool records
pub fn cleanup_failed_pool_cache() -> Result<(), String> {
    // Clean up database first
    let deleted = cleanup_old_failed_pools()?;

    // Reload cache from database to remove expired entries
    load_failed_pools_into_cache()?;

    if is_debug_pool_analyzer_enabled() {
        let (total, permanent, retryable) = get_failed_pool_stats();
        log(
            LogTag::PoolAnalyzer,
            "CLEANUP",
            &format!(
                "Failed pool cache cleanup: deleted {} old records, cache now has {} total ({} permanent, {} retryable)",
                deleted, total, permanent, retryable
            )
        );
    }

    Ok(())
}

/// Clear the entire failed pool cache (for debugging)
pub fn clear_failed_pool_cache() {
    if let Ok(mut cache) = FAILED_POOL_CACHE.lock() {
        let old_size = cache.len();
        cache.clear();

        if is_debug_pool_analyzer_enabled() {
            log(
                LogTag::PoolAnalyzer,
                "CACHE_CLEAR",
                &format!("Cleared failed pool cache ({} entries)", old_size)
            );
        }
    }
}
