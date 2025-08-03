/// Centralized RPC Client for Solana
///
/// This module provides a centralized RPC client that can be used throughout the application
/// for consistent RPC configuration and connection management.

use crate::logger::{ log, LogTag };
use crate::global::read_configs;
use solana_client::rpc_client::RpcClient as SolanaRpcClient;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
    client::SyncClient,
};
use std::sync::Arc;
use std::str::FromStr;
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };

/// Statistics tracking for RPC usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcStats {
    /// Total calls per RPC URL
    pub calls_per_url: HashMap<String, u64>,
    /// Total calls per RPC method
    pub calls_per_method: HashMap<String, u64>,
    /// Statistics since startup
    pub startup_time: DateTime<Utc>,
    /// Last save time
    pub last_save_time: DateTime<Utc>,
}

impl Default for RpcStats {
    fn default() -> Self {
        Self {
            calls_per_url: HashMap::new(),
            calls_per_method: HashMap::new(),
            startup_time: Utc::now(),
            last_save_time: Utc::now(),
        }
    }
}

impl RpcStats {
    /// Record a call to an RPC method on a specific URL
    pub fn record_call(&mut self, url: &str, method: &str) {
        *self.calls_per_url.entry(url.to_string()).or_insert(0) += 1;
        *self.calls_per_method.entry(method.to_string()).or_insert(0) += 1;
    }

    /// Get total calls across all URLs
    pub fn total_calls(&self) -> u64 {
        self.calls_per_url.values().sum()
    }

    /// Get calls per second since startup
    pub fn calls_per_second(&self) -> f64 {
        let duration = Utc::now().signed_duration_since(self.startup_time);
        let seconds = duration.num_seconds() as f64;
        if seconds > 0.0 {
            (self.total_calls() as f64) / seconds
        } else {
            0.0
        }
    }

    /// Save stats to disk
    pub fn save_to_disk(&mut self) -> Result<(), String> {
        self.last_save_time = Utc::now();
        let json_data = serde_json
            ::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize RPC stats: {}", e))?;

        std::fs
            ::write("rpc_stats.json", json_data)
            .map_err(|e| format!("Failed to write RPC stats file: {}", e))?;

        Ok(())
    }

    /// Load stats from disk, merging with current stats
    pub fn load_from_disk(&mut self) -> Result<(), String> {
        match std::fs::read_to_string("rpc_stats.json") {
            Ok(data) => {
                match serde_json::from_str::<RpcStats>(&data) {
                    Ok(loaded_stats) => {
                        // Store counts before moving
                        let url_count = loaded_stats.calls_per_url.len();
                        let method_count = loaded_stats.calls_per_method.len();
                        let total_calls = loaded_stats.total_calls();

                        // Merge URL stats
                        for (url, count) in loaded_stats.calls_per_url {
                            *self.calls_per_url.entry(url).or_insert(0) += count;
                        }

                        // Merge method stats
                        for (method, count) in loaded_stats.calls_per_method {
                            *self.calls_per_method.entry(method).or_insert(0) += count;
                        }

                        log(
                            LogTag::Rpc,
                            "STATS",
                            &format!(
                                "Loaded RPC stats from disk: {} total calls, {} URLs, {} methods",
                                total_calls,
                                url_count,
                                method_count
                            )
                        );
                        Ok(())
                    }
                    Err(e) => {
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            &format!("Failed to parse RPC stats file, starting fresh: {}", e)
                        );
                        Ok(())
                    }
                }
            }
            Err(_) => {
                log(LogTag::Rpc, "INFO", "No existing RPC stats file found, starting fresh");
                Ok(())
            }
        }
    }
}

/// Rate limiter for RPC calls
pub struct RpcRateLimiter {
    main_rpc_interval: Duration,
    last_main_call: Option<Instant>,
}

impl RpcRateLimiter {
    pub fn new(calls_per_second: u64) -> Self {
        let interval_duration = Duration::from_millis(1000 / calls_per_second.max(1));
        Self {
            main_rpc_interval: interval_duration,
            last_main_call: None,
        }
    }

    /// Wait for rate limit before making a call to main RPC
    pub async fn wait_for_main_rpc(&mut self) {
        if let Some(last_call) = self.last_main_call {
            let elapsed = last_call.elapsed();
            if elapsed < self.main_rpc_interval {
                let wait_duration = self.main_rpc_interval - elapsed;
                tokio::time::sleep(wait_duration).await;
            }
        }
        self.last_main_call = Some(Instant::now());
    }

    /// Check if we need to wait for rate limit (without waiting)
    pub fn should_wait_for_main_rpc(&self, min_interval_ms: u64) -> bool {
        if let Some(last_call) = self.last_main_call {
            last_call.elapsed() < Duration::from_millis(min_interval_ms)
        } else {
            false
        }
    }
}

/// Centralized RPC client with connection pooling and error handling
pub struct RpcClient {
    client: Arc<SolanaRpcClient>,
    rpc_url: String,
    premium_url: Option<String>,
    fallback_urls: Vec<String>,
    current_url_index: usize,
    stats: Arc<std::sync::Mutex<RpcStats>>,
    rate_limiter: Arc<std::sync::Mutex<RpcRateLimiter>>,
}

impl RpcClient {
    /// Create new RPC client with configuration from configs.json
    pub fn new() -> Self {
        Self::from_config().unwrap_or_else(|e| {
            log(LogTag::Rpc, "ERROR", &format!("Failed to load config: {}", e));
            panic!(
                "Cannot initialize RPC client without valid configuration. Please check configs.json"
            );
        })
    }

    /// Create new RPC client from configs.json
    pub fn from_config() -> Result<Self, String> {
        let configs = read_configs("configs.json").map_err(|e|
            format!("Failed to read configs: {}", e)
        )?;

        let mut all_urls = vec![configs.rpc_url.clone()];
        all_urls.extend(configs.rpc_fallbacks.clone());

        log(
            LogTag::Rpc,
            "INIT",
            &format!(
                "Initializing RPC client with {} URLs (primary + {} fallbacks), premium: {}",
                all_urls.len(),
                configs.rpc_fallbacks.len(),
                configs.rpc_url_premium
            )
        );

        if !configs.rpc_fallbacks.is_empty() {
            log(
                LogTag::Rpc,
                "FALLBACKS",
                &format!("Available fallback URLs: {}", configs.rpc_fallbacks.join(", "))
            );
        }

        Self::new_with_urls(&configs.rpc_url, Some(configs.rpc_url_premium), configs.rpc_fallbacks)
    }

    /// Create new RPC client with primary URL and fallbacks
    pub fn new_with_urls(
        primary_url: &str,
        premium_url: Option<String>,
        fallback_urls: Vec<String>
    ) -> Result<Self, String> {
        log(LogTag::Rpc, "INIT", &format!("Initializing RPC client with primary: {}", primary_url));

        let client = SolanaRpcClient::new_with_commitment(
            primary_url.to_string(),
            CommitmentConfig::confirmed()
        );

        let mut stats = RpcStats::default();
        let _ = stats.load_from_disk(); // Load existing stats, ignore errors

        Ok(Self {
            client: Arc::new(client),
            rpc_url: primary_url.to_string(),
            premium_url,
            fallback_urls,
            current_url_index: 0,
            stats: Arc::new(std::sync::Mutex::new(stats)),
            rate_limiter: Arc::new(std::sync::Mutex::new(RpcRateLimiter::new(10))), // 10 calls per second default
        })
    }

    /// Create new RPC client with custom URL (legacy method)
    pub fn new_with_url(rpc_url: &str) -> Self {
        log(LogTag::Rpc, "INIT", &format!("Initializing RPC client with URL: {}", rpc_url));

        let client = SolanaRpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed()
        );

        let mut stats = RpcStats::default();
        let _ = stats.load_from_disk(); // Load existing stats, ignore errors

        Self {
            client: Arc::new(client),
            rpc_url: rpc_url.to_string(),
            premium_url: None,
            fallback_urls: Vec::new(),
            current_url_index: 0,
            stats: Arc::new(std::sync::Mutex::new(stats)),
            rate_limiter: Arc::new(std::sync::Mutex::new(RpcRateLimiter::new(10))), // 10 calls per second default
        }
    }

    /// Get the underlying RPC client
    pub fn client(&self) -> Arc<SolanaRpcClient> {
        self.client.clone()
    }

    /// Get RPC URL
    pub fn url(&self) -> &str {
        &self.rpc_url
    }

    /// Get premium RPC URL
    pub fn premium_url(&self) -> Option<&str> {
        self.premium_url.as_deref()
    }

    /// Get RPC statistics
    pub fn get_stats(&self) -> RpcStats {
        self.stats.lock().unwrap().clone()
    }

    /// Save RPC statistics to disk
    pub fn save_stats(&self) -> Result<(), String> {
        self.stats.lock().unwrap().save_to_disk()
    }

    /// Record an RPC call for statistics
    fn record_call(&self, method: &str) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, method);

            // Auto-save every 100 calls
            if stats.total_calls() % 100 == 0 {
                let _ = stats.save_to_disk();
            }
        }
    }

    /// Wait for rate limit if using main RPC
    async fn wait_for_rate_limit(&self) {
        // Only rate limit the main RPC URL, not fallbacks
        if self.current_url_index == 0 {
            // Check if we need to wait and get the wait duration
            let wait_duration = {
                if let Ok(rate_limiter) = self.rate_limiter.lock() {
                    if let Some(last_call) = rate_limiter.last_main_call {
                        let elapsed = last_call.elapsed();
                        if elapsed < rate_limiter.main_rpc_interval {
                            Some(rate_limiter.main_rpc_interval - elapsed)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }; // Lock is released here

            // Wait if needed
            if let Some(duration) = wait_duration {
                tokio::time::sleep(duration).await;
            }

            // Update last call time
            if let Ok(mut rate_limiter) = self.rate_limiter.lock() {
                rate_limiter.last_main_call = Some(Instant::now());
            }
        }
    }

    /// Create a new client using premium URL (for wallet operations)
    pub fn create_premium_client(&self) -> Option<Arc<SolanaRpcClient>> {
        if let Some(premium_url) = &self.premium_url {
            log(LogTag::Rpc, "PREMIUM", &format!("Using premium RPC: {}", premium_url));
            let client = SolanaRpcClient::new_with_commitment(
                premium_url.clone(),
                CommitmentConfig::confirmed()
            );
            Some(Arc::new(client))
        } else {
            None
        }
    }

    /// Check if error should trigger fallback (rate limits, timeouts) vs real errors (account not found)
    fn should_fallback_on_error(error: &str) -> bool {
        let error_lower = error.to_lowercase();

        // Rate limiting and temporary issues - should fallback
        if
            error_lower.contains("429") ||
            error_lower.contains("too many requests") ||
            error_lower.contains("rate limit") ||
            error_lower.contains("timeout") ||
            error_lower.contains("connection") ||
            error_lower.contains("network")
        {
            return true;
        }

        // Real blockchain state - don't fallback, cache as failed
        if
            error_lower.contains("account not found") ||
            error_lower.contains("invalid account") ||
            error_lower.contains("account does not exist")
        {
            return false;
        }

        // Default to fallback for unknown errors
        true
    }

    /// Get all available URLs (primary + fallbacks)
    pub fn get_all_urls(&self) -> Vec<String> {
        let mut urls = vec![self.rpc_url.clone()];
        urls.extend(self.fallback_urls.clone());
        urls
    }

    /// Switch to next fallback URL
    pub fn switch_to_fallback(&mut self) -> Result<(), String> {
        if self.fallback_urls.is_empty() {
            return Err("No fallback URLs available".to_string());
        }

        self.current_url_index = (self.current_url_index + 1) % (self.fallback_urls.len() + 1);

        let new_url = if self.current_url_index == 0 {
            self.rpc_url.clone()
        } else {
            self.fallback_urls[self.current_url_index - 1].clone()
        };

        log(LogTag::Rpc, "FALLBACK", &format!("Switching to URL: {}", new_url));

        let new_client = SolanaRpcClient::new_with_commitment(
            new_url.clone(),
            CommitmentConfig::confirmed()
        );

        self.client = Arc::new(new_client);

        // Update current URL in stats tracking
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&new_url, "switch_fallback");
        }

        Ok(())
    }

    /// Get single account data
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String> {
        self.wait_for_rate_limit().await;
        self.record_call("get_account");

        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                let pubkey = *pubkey;
                move || {
                    client
                        .get_account(&pubkey)
                        .map_err(|e| format!("Failed to get account {}: {}", pubkey, e))
                }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }

    /// Get multiple accounts data (batch request for efficiency)
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey]
    ) -> Result<Vec<Option<Account>>, String> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        self.wait_for_rate_limit().await;
        self.record_call("get_multiple_accounts");

        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                let pubkeys = pubkeys.to_vec();
                move || {
                    client
                        .get_multiple_accounts(&pubkeys)
                        .map_err(|e| format!("Failed to get multiple accounts: {}", e))
                }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }

    /// Get account data with automatic fallback support
    pub async fn get_account_with_fallback(&mut self, pubkey: &Pubkey) -> Result<Account, String> {
        let max_attempts = self.get_all_urls().len();
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            match self.get_account(pubkey).await {
                Ok(account) => {
                    return Ok(account);
                }
                Err(e) => {
                    last_error = e.clone();
                    log(LogTag::Rpc, "ERROR", &format!("RPC call failed on {}: {}", self.url(), e));

                    if attempt < max_attempts - 1 {
                        if let Err(switch_err) = self.switch_to_fallback() {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to switch fallback: {}", switch_err)
                            );
                            break;
                        }
                    }
                }
            }
        }

        Err(format!("Failed on all {} RPC endpoints: {}", max_attempts, last_error))
    }

    /// Get multiple accounts with automatic fallback support
    pub async fn get_multiple_accounts_with_fallback(
        &mut self,
        pubkeys: &[Pubkey]
    ) -> Result<Vec<Option<Account>>, String> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        let max_attempts = self.get_all_urls().len();
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            match self.get_multiple_accounts(pubkeys).await {
                Ok(accounts) => {
                    return Ok(accounts);
                }
                Err(e) => {
                    last_error = e.clone();
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC batch call failed on {}: {}", self.url(), e)
                    );

                    if attempt < max_attempts - 1 {
                        if let Err(switch_err) = self.switch_to_fallback() {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to switch fallback: {}", switch_err)
                            );
                            break;
                        }
                    }
                }
            }
        }

        Err(format!("Failed batch request on all {} RPC endpoints: {}", max_attempts, last_error))
    }

    /// Test connection with automatic fallback
    pub async fn test_connection_with_fallback(&mut self) -> Result<(), String> {
        let max_attempts = self.get_all_urls().len();
        let mut last_error = String::new();

        for attempt in 0..max_attempts {
            match self.test_connection().await {
                Ok(()) => {
                    log(
                        LogTag::Rpc,
                        "SUCCESS",
                        &format!("RPC connection test passed on {}", self.url())
                    );
                    return Ok(());
                }
                Err(e) => {
                    last_error = e.clone();
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC connection test failed on {}: {}", self.url(), e)
                    );

                    if attempt < max_attempts - 1 {
                        if let Err(switch_err) = self.switch_to_fallback() {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to switch fallback: {}", switch_err)
                            );
                            break;
                        }
                    }
                }
            }
        }

        Err(format!("Connection test failed on all {} RPC endpoints: {}", max_attempts, last_error))
    }

    /// Test RPC connection
    pub async fn test_connection(&self) -> Result<(), String> {
        self.wait_for_rate_limit().await;
        self.record_call("get_slot");

        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                move || {
                    client
                        .get_slot()
                        .map_err(|e| format!("RPC connection test failed: {}", e))
                        .map(|_| ())
                }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }

    /// Get current slot
    pub async fn get_slot(&self) -> Result<u64, String> {
        self.wait_for_rate_limit().await;
        self.record_call("get_slot");

        tokio::task
            ::spawn_blocking({
                let client = self.client.clone();
                move || { client.get_slot().map_err(|e| format!("Failed to get slot: {}", e)) }
            }).await
            .map_err(|e| format!("Task error: {}", e))?
    }
}

/// Global RPC client instance
static mut GLOBAL_RPC_CLIENT: Option<RpcClient> = None;
static RPC_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global RPC client from configuration
pub fn init_rpc_client() -> Result<&'static RpcClient, String> {
    unsafe {
        let mut init_error: Option<String> = None;

        RPC_INIT.call_once(|| {
            match RpcClient::from_config() {
                Ok(client) => {
                    log(LogTag::Rpc, "SUCCESS", "Global RPC client initialized from configuration");
                    GLOBAL_RPC_CLIENT = Some(client);
                }
                Err(e) => {
                    init_error = Some(e.clone());
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to init RPC client from config: {}", e)
                    );
                }
            }
        });

        if let Some(error) = init_error {
            Err(error)
        } else {
            Ok(GLOBAL_RPC_CLIENT.as_ref().unwrap())
        }
    }
}

/// Initialize global RPC client with custom URL (legacy method)
/// Note: This method requires a valid URL parameter as hardcoded fallbacks have been removed
pub fn init_rpc_client_with_url(rpc_url: Option<&str>) -> Result<&'static RpcClient, String> {
    unsafe {
        let mut init_error: Option<String> = None;

        RPC_INIT.call_once(|| {
            match rpc_url {
                Some(url) => {
                    log(
                        LogTag::Rpc,
                        "INIT",
                        &format!("Initializing global RPC client with custom URL: {}", url)
                    );
                    GLOBAL_RPC_CLIENT = Some(RpcClient::new_with_url(url));
                }
                None => {
                    init_error = Some(
                        "No RPC URL provided and no hardcoded fallback available".to_string()
                    );
                    log(LogTag::Rpc, "ERROR", "Cannot initialize RPC client without URL parameter");
                }
            }
        });

        if let Some(error) = init_error {
            Err(error)
        } else {
            Ok(GLOBAL_RPC_CLIENT.as_ref().unwrap())
        }
    }
}

/// Get global RPC client instance
pub fn get_rpc_client() -> &'static RpcClient {
    unsafe {
        if GLOBAL_RPC_CLIENT.is_none() {
            let _ = init_rpc_client(); // Initialize if not already done
        }
        GLOBAL_RPC_CLIENT.as_ref().unwrap()
    }
}

/// Get global RPC statistics
pub fn get_global_rpc_stats() -> Option<RpcStats> {
    unsafe { GLOBAL_RPC_CLIENT.as_ref().map(|client| client.get_stats()) }
}

/// Save global RPC statistics to disk
pub fn save_global_rpc_stats() -> Result<(), String> {
    unsafe {
        if let Some(client) = GLOBAL_RPC_CLIENT.as_ref() {
            client.save_stats()
        } else {
            Err("RPC client not initialized".to_string())
        }
    }
}

/// Parse string to Pubkey
pub fn parse_pubkey(address: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(address).map_err(|e| format!("Invalid pubkey '{}': {}", address, e))
}

/// Get premium RPC URL for wallet operations (high priority transactions)
pub fn get_premium_transaction_rpc(configs: &crate::global::Configs) -> String {
    configs.rpc_url_premium.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        // Use new_with_url since new() requires configs.json which may not exist in tests
        let test_url = "https://api.mainnet-beta.solana.com";
        let client = RpcClient::new_with_url(test_url);
        assert!(!client.url().is_empty());
        assert_eq!(client.url(), test_url);
    }

    #[tokio::test]
    async fn test_parse_pubkey() {
        let valid_pubkey = "So11111111111111111111111111111111111111112";
        assert!(parse_pubkey(valid_pubkey).is_ok());

        let invalid_pubkey = "invalid";
        assert!(parse_pubkey(invalid_pubkey).is_err());
    }
}
