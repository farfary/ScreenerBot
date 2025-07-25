use anyhow::Result;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration };
use chrono::{ Duration as ChronoDuration, Utc, DateTime };

use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_prices_enabled;
const CACHE_DURATION_MINUTES: i64 = 5;

// =============================================================================
// CACHE CONFIGURATION
// =============================================================================

/// Cache file path for persistent storage
const POOL_CACHE_FILE: &str = "pool_cache.json";

/// Cache validity duration (5 minutes as requested)
pub const POOL_CACHE_DURATION: Duration = Duration::from_secs(300); // 5 minutes

/// Failed token skip file path
const FAILED_TOKENS_FILE: &str = "failed_pool_tokens.json";

// =============================================================================
// CACHE TYPES
// =============================================================================

/// Cache entry for a token's biggest pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAddressCacheEntry {
    pub pool_address: String,
    pub dex_id: String,
    pub pool_type: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    // For pools with separate reserve accounts (like PumpFun)
    pub reserve_accounts: Option<ReserveAccountAddresses>,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveAccountAddresses {
    pub base_reserve_account: String,
    pub quote_reserve_account: String,
}

impl PoolAddressCacheEntry {
    pub fn new(
        pool_address: String,
        dex_id: String,
        pool_type: String,
        token_a_mint: String,
        token_b_mint: String,
        reserve_accounts: Option<ReserveAccountAddresses>
    ) -> Self {
        let now = Utc::now();
        Self {
            pool_address,
            dex_id,
            pool_type,
            token_a_mint,
            token_b_mint,
            reserve_accounts,
            cached_at: now,
            expires_at: now + ChronoDuration::minutes(CACHE_DURATION_MINUTES),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Failed token tracking entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTokenEntry {
    pub mint: String,
    pub failed_at: DateTime<Utc>,
    pub failure_count: u32,
    pub last_error: String,
}

impl FailedTokenEntry {
    pub fn new(mint: String, error: String) -> Self {
        Self {
            mint,
            failed_at: Utc::now(),
            failure_count: 1,
            last_error: error,
        }
    }

    pub fn increment_failure(&mut self, error: String) {
        self.failure_count += 1;
        self.failed_at = Utc::now();
        self.last_error = error;
    }
}

/// Persistent cache data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAddressCacheData {
    pub pool_addresses: HashMap<String, PoolAddressCacheEntry>,
    pub failed_tokens: HashMap<String, FailedTokenEntry>,
    pub last_updated: DateTime<Utc>,
}

impl Default for PoolAddressCacheData {
    fn default() -> Self {
        Self {
            pool_addresses: HashMap::new(),
            failed_tokens: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

// =============================================================================
// POOL CACHE MANAGER
// =============================================================================

/// Thread-safe pool cache manager with disk persistence
pub struct PoolAddressCacheManager {
    data: Arc<Mutex<PoolAddressCacheData>>,
    cache_file: String,
}

impl PoolAddressCacheManager {
    /// Create new cache manager instance
    pub fn new() -> Self {
        let mut manager = Self {
            data: Arc::new(Mutex::new(PoolAddressCacheData::default())),
            cache_file: "pool_address_cache.json".to_string(),
        };

        // Load existing cache from disk
        if let Err(e) = manager.load_from_disk() {
            if is_debug_pool_prices_enabled() {
                log(LogTag::Pool, "WARN", &format!("Failed to load pool address cache: {}", e));
            }
        }

        manager
    }

    /// Get cached pool address for a token (if valid and not expired)
    pub fn get_cached_pool_address(&self, token_mint: &str) -> Option<PoolAddressCacheEntry> {
        if let Ok(data) = self.data.lock() {
            if let Some(entry) = data.pool_addresses.get(token_mint) {
                if !entry.is_expired() {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "DEBUG",
                            &format!("Address Cache HIT: {} -> {}", token_mint, entry.pool_address)
                        );
                    }
                    return Some(entry.clone());
                } else {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "DEBUG",
                            &format!(
                                "Address Cache EXPIRED: {} (age: {}s)",
                                token_mint,
                                Utc::now().signed_duration_since(entry.cached_at).num_seconds()
                            )
                        );
                    }
                }
            } else {
                if is_debug_pool_prices_enabled() {
                    log(LogTag::Pool, "DEBUG", &format!("Address Cache MISS: {}", token_mint));
                }
            }
        }
        None
    }

    /// Cache the biggest pool address for a token (addresses only, no price/balance data)
    pub fn cache_pool_address(
        &self,
        token_mint: &str,
        pool_address: &str,
        dex_id: &str,
        pool_type: &str,
        token_a_mint: &str,
        token_b_mint: &str,
        reserve_accounts: Option<ReserveAccountAddresses>
    ) -> Result<()> {
        let entry = PoolAddressCacheEntry::new(
            pool_address.to_string(),
            dex_id.to_string(),
            pool_type.to_string(),
            token_a_mint.to_string(),
            token_b_mint.to_string(),
            reserve_accounts
        );

        if let Ok(mut data) = self.data.lock() {
            data.pool_addresses.insert(token_mint.to_string(), entry.clone());
            data.last_updated = Utc::now();

            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!("Cached pool address: {} -> {} ({})", token_mint, pool_address, dex_id)
                );
            }
        }

        // Save to disk
        self.save_to_disk()
    }

    /// Check if a token should be permanently skipped due to previous failures
    pub fn should_skip_token(&self, token_mint: &str) -> bool {
        if let Ok(data) = self.data.lock() {
            if let Some(failed_entry) = data.failed_tokens.get(token_mint) {
                // Skip tokens that have failed 3 or more times
                if failed_entry.failure_count >= 3 {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "DEBUG",
                            &format!(
                                "SKIP: {} (failed {} times, last: {})",
                                token_mint,
                                failed_entry.failure_count,
                                failed_entry.last_error
                            )
                        );
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Record a failed token for future skipping
    pub fn record_failed_token(&self, token_mint: &str, error: &str) -> Result<()> {
        if let Ok(mut data) = self.data.lock() {
            if let Some(existing) = data.failed_tokens.get_mut(token_mint) {
                existing.increment_failure(error.to_string());
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!(
                            "Failed token {} (count: {}): {}",
                            token_mint,
                            existing.failure_count,
                            error
                        )
                    );
                }
            } else {
                let failed_entry = FailedTokenEntry::new(token_mint.to_string(), error.to_string());
                data.failed_tokens.insert(token_mint.to_string(), failed_entry);
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!("New failed token {}: {}", token_mint, error)
                    );
                }
            }
            data.last_updated = Utc::now();
        }

        // Save to disk
        self.save_to_disk()
    }

    /// Get cache statistics for debugging
    pub fn get_stats(&self) -> (usize, usize, usize, usize) {
        if let Ok(data) = self.data.lock() {
            let total_cached = data.pool_addresses.len();
            let valid_cached = data.pool_addresses
                .values()
                .filter(|entry| !entry.is_expired())
                .count();
            let expired_cached = total_cached - valid_cached;
            let failed_tokens = data.failed_tokens.len();

            (total_cached, valid_cached, expired_cached, failed_tokens)
        } else {
            (0, 0, 0, 0)
        }
    }

    /// Clean up expired cache entries
    pub fn cleanup_expired(&self) -> Result<()> {
        if let Ok(mut data) = self.data.lock() {
            let before_count = data.pool_addresses.len();

            // Remove expired entries
            data.pool_addresses.retain(|_, entry| !entry.is_expired());

            let after_count = data.pool_addresses.len();
            let removed_count = before_count - after_count;

            if removed_count > 0 {
                data.last_updated = Utc::now();
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!("Cleaned up {} expired address cache entries", removed_count)
                    );
                }
                return self.save_to_disk();
            }
        }
        Ok(())
    }

    /// Load cache data from disk
    fn load_from_disk(&self) -> Result<()> {
        if !Path::new(&self.cache_file).exists() {
            if is_debug_pool_prices_enabled() {
                log(LogTag::Pool, "DEBUG", "Pool address cache file not found, starting fresh");
            }
            return Ok(());
        }

        let content = fs::read_to_string(&self.cache_file)?;
        let loaded_data: PoolAddressCacheData = serde_json::from_str(&content)?;

        if let Ok(mut data) = self.data.lock() {
            *data = loaded_data;

            let (total, valid, expired, failed) = self.get_stats();
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "INFO",
                    &format!(
                        "Loaded pool address cache: {} total, {} valid, {} expired, {} failed tokens",
                        total,
                        valid,
                        expired,
                        failed
                    )
                );
            }
        }

        Ok(())
    }

    /// Save cache data to disk
    fn save_to_disk(&self) -> Result<()> {
        if let Ok(data) = self.data.lock() {
            let content = serde_json::to_string_pretty(&*data)?;
            fs::write(&self.cache_file, content)?;

            if is_debug_pool_prices_enabled() {
                let (total, valid, _, failed) = self.get_stats();
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "Saved pool address cache: {} addresses, {} failed tokens",
                        valid,
                        failed
                    )
                );
            }
        }
        Ok(())
    }
}

// =============================================================================
// GLOBAL CACHE INSTANCE
// =============================================================================

use once_cell::sync::Lazy;

/// Global pool address cache manager instance
pub static POOL_ADDRESS_CACHE: Lazy<PoolAddressCacheManager> = Lazy::new(||
    PoolAddressCacheManager::new()
);

/// Helper function to get cached pool address
pub fn get_cached_pool_address(token_mint: &str) -> Option<PoolAddressCacheEntry> {
    POOL_ADDRESS_CACHE.get_cached_pool_address(token_mint)
}

/// Helper function to cache a pool address
pub fn cache_pool_address(
    token_mint: &str,
    pool_address: &str,
    dex_id: &str,
    pool_type: &str,
    token_a_mint: &str,
    token_b_mint: &str,
    reserve_accounts: Option<ReserveAccountAddresses>
) -> Result<()> {
    POOL_ADDRESS_CACHE.cache_pool_address(
        token_mint,
        pool_address,
        dex_id,
        pool_type,
        token_a_mint,
        token_b_mint,
        reserve_accounts
    )
}

/// Helper function to check if token should be skipped
pub fn should_skip_token(token_mint: &str) -> bool {
    POOL_ADDRESS_CACHE.should_skip_token(token_mint)
}

/// Helper function to record failed token
pub fn record_failed_token(token_mint: &str, error: &str) -> Result<()> {
    POOL_ADDRESS_CACHE.record_failed_token(token_mint, error)
}

/// Helper function to get cache stats
pub fn get_pool_cache_stats() -> (usize, usize, usize, usize) {
    POOL_ADDRESS_CACHE.get_stats()
}

/// Helper function to cleanup expired entries
pub fn cleanup_expired_pools() -> Result<()> {
    POOL_ADDRESS_CACHE.cleanup_expired()
}
