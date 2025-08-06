/// Centralized RPC Client for Solana
///
/// This module provides a centralized RPC client that can be used throughout the application
/// for consistent RPC configuration and connection management.

use crate::logger::{ log, LogTag };
use crate::global::{ read_configs, is_debug_wallet_enabled, RPC_STATS };
use solana_client::rpc_client::RpcClient as SolanaRpcClient;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
    client::SyncClient,
    transaction::VersionedTransaction,
    signer::Signer,
    signature::Keypair,
    transaction::Transaction,
    hash::Hash,
};
use std::sync::Arc;
use std::str::FromStr;
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };
use base64::{ engine::general_purpose, Engine as _ };
use reqwest;
use serde_json;
use bincode;
use bs58;

/// Structure to hold token account information
#[derive(Debug)]
pub struct TokenAccountInfo {
    pub account: String,
    pub mint: String,
    pub balance: u64,
    pub is_token_2022: bool,
}

/// Transaction details from RPC
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetails {
    pub slot: u64,
    pub transaction: TransactionData,
    pub meta: Option<TransactionMeta>,
}

/// Transaction data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionData {
    pub message: serde_json::Value,
    pub signatures: Vec<String>,
}

/// Transaction metadata with balance changes
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub err: Option<serde_json::Value>,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances")]
    pub pre_token_balances: Option<Vec<TokenBalance>>,
    #[serde(rename = "postTokenBalances")]
    pub post_token_balances: Option<Vec<TokenBalance>>,
    pub fee: u64,
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
}

/// Token balance information in transaction metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    #[serde(rename = "accountIndex")]
    pub account_index: u32,
    pub mint: String,
    pub owner: Option<String>,
    #[serde(rename = "programId")]
    pub program_id: Option<String>,
    #[serde(rename = "uiTokenAmount")]
    pub ui_token_amount: UiTokenAmount,
}

/// Token amount with UI representation
#[derive(Debug, Serialize, Deserialize)]
pub struct UiTokenAmount {
    pub amount: String,
    pub decimals: u8,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
}
use tokio::sync::Notify;

/// Error types for RPC and wallet operations
#[derive(Debug)]
pub enum SwapError {
    ApiError(String),
    NetworkError(reqwest::Error),
    InvalidResponse(String),
    InsufficientBalance(String),
    SlippageExceeded(String),
    InvalidAmount(String),
    ConfigError(String),
    TransactionError(String),
    SigningError(String),
    ParseError(String),
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::ApiError(msg) => write!(f, "API Error: {}", msg),
            SwapError::NetworkError(err) => write!(f, "Network Error: {}", err),
            SwapError::InvalidResponse(msg) => write!(f, "Invalid Response: {}", msg),
            SwapError::InsufficientBalance(msg) => write!(f, "Insufficient Balance: {}", msg),
            SwapError::SlippageExceeded(msg) => write!(f, "Slippage Exceeded: {}", msg),
            SwapError::InvalidAmount(msg) => write!(f, "Invalid Amount: {}", msg),
            SwapError::ConfigError(msg) => write!(f, "Config Error: {}", msg),
            SwapError::TransactionError(msg) => write!(f, "Transaction Error: {}", msg),
            SwapError::SigningError(msg) => write!(f, "Signing Error: {}", msg),
            SwapError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
        }
    }
}

impl std::error::Error for SwapError {}

impl From<reqwest::Error> for SwapError {
    fn from(err: reqwest::Error) -> Self {
        SwapError::NetworkError(err)
    }
}

impl From<serde_json::Error> for SwapError {
    fn from(err: serde_json::Error) -> Self {
        SwapError::ParseError(format!("JSON parsing error: {}", err))
    }
}

/// Converts lamports to SOL amount
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / 1_000_000_000.0
}

/// Converts SOL amount to lamports (1 SOL = 1,000,000,000 lamports)
pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * 1_000_000_000.0) as u64
}

/// Statistics tracking for RPC usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcStats {
    /// Total calls per RPC URL
    pub calls_per_url: HashMap<String, u64>,
    /// Total calls per RPC method
    pub calls_per_method: HashMap<String, u64>,
    /// Method calls per URL (URL -> Method -> Count)
    pub calls_per_url_per_method: HashMap<String, HashMap<String, u64>>,
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
            calls_per_url_per_method: HashMap::new(),
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

        // Track method calls per URL
        self.calls_per_url_per_method
            .entry(url.to_string())
            .or_insert_with(HashMap::new)
            .entry(method.to_string())
            .and_modify(|count| {
                *count += 1;
            })
            .or_insert(1);
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

    /// Get method calls for a specific URL
    pub fn get_method_calls_for_url(&self, url: &str) -> HashMap<String, u64> {
        self.calls_per_url_per_method.get(url).cloned().unwrap_or_default()
    }

    /// Get all URLs that have method call data
    pub fn get_urls_with_method_data(&self) -> Vec<String> {
        self.calls_per_url_per_method.keys().cloned().collect()
    }

    /// Check if it's time to save (3 seconds since last save)
    pub fn should_save(&self) -> bool {
        let now = Utc::now();
        let time_since_last_save = now.signed_duration_since(self.last_save_time);
        time_since_last_save.num_seconds() >= 3
    }

    /// Save stats to disk
    pub fn save_to_disk(&mut self) -> Result<(), String> {
        self.last_save_time = Utc::now();
        let json_data = serde_json
            ::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize RPC stats: {}", e))?;

        std::fs
            ::write(RPC_STATS, json_data)
            .map_err(|e| format!("Failed to write RPC stats file: {}", e))?;

        Ok(())
    }

    /// Load stats from disk, merging with current stats
    pub fn load_from_disk(&mut self) -> Result<(), String> {
        match std::fs::read_to_string(RPC_STATS) {
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

                        // Merge method calls per URL stats
                        for (url, method_counts) in loaded_stats.calls_per_url_per_method {
                            let url_entry = self.calls_per_url_per_method
                                .entry(url)
                                .or_insert_with(HashMap::new);
                            for (method, count) in method_counts {
                                *url_entry.entry(method).or_insert(0) += count;
                            }
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

/// Enhanced rate limiter for RPC calls with adaptive backoff and 429 prevention
pub struct RpcRateLimiter {
    /// Base interval between calls for main RPC
    base_interval: Duration,
    /// Current dynamic interval (adjusted based on 429 responses)
    current_interval: Duration,
    /// Maximum interval we'll back off to
    max_interval: Duration,
    /// Last call timestamp for main RPC
    last_main_call: Option<Instant>,
    /// Track consecutive 429 errors for exponential backoff
    consecutive_429s: u32,
    /// Track successful calls to reduce interval back to normal
    consecutive_successes: u32,
    /// Per-URL rate limiting
    url_last_calls: std::collections::HashMap<String, Instant>,
    /// Per-URL intervals (some URLs may have different limits)
    url_intervals: std::collections::HashMap<String, Duration>,
    /// Backoff multiplier for 429 responses
    backoff_multiplier: f64,
}

impl RpcRateLimiter {
    pub fn new(calls_per_second: u64) -> Self {
        let base_interval = Duration::from_millis(1000 / calls_per_second.max(1));
        Self {
            base_interval,
            current_interval: base_interval,
            max_interval: Duration::from_secs(30), // Maximum 30 second backoff
            last_main_call: None,
            consecutive_429s: 0,
            consecutive_successes: 0,
            url_last_calls: std::collections::HashMap::new(),
            url_intervals: std::collections::HashMap::new(),
            backoff_multiplier: 2.0, // Double the interval on each 429
        }
    }

    /// Create a more conservative rate limiter to prevent 429 errors
    pub fn new_conservative() -> Self {
        Self::new(5) // Start with 5 calls per second, very conservative
    }

    /// Wait for rate limit before making a call to main RPC with adaptive backoff
    pub async fn wait_for_main_rpc(&mut self) {
        if let Some(last_call) = self.last_main_call {
            let elapsed = last_call.elapsed();
            if elapsed < self.current_interval {
                let wait_duration = self.current_interval - elapsed;
                log(
                    LogTag::Rpc,
                    "RATE_LIMIT",
                    &format!(
                        "Rate limiting main RPC: waiting {:.2}ms (current interval: {:.2}ms, 429s: {})",
                        wait_duration.as_millis(),
                        self.current_interval.as_millis(),
                        self.consecutive_429s
                    )
                );
                tokio::time::sleep(wait_duration).await;
            }
        }
        self.last_main_call = Some(Instant::now());
    }

    /// Wait for rate limit for a specific URL
    pub async fn wait_for_url(&mut self, url: &str) {
        let url_interval = self.url_intervals.get(url).unwrap_or(&self.current_interval).clone();

        if let Some(last_call) = self.url_last_calls.get(url) {
            let elapsed = last_call.elapsed();
            if elapsed < url_interval {
                let wait_duration = url_interval - elapsed;
                log(
                    LogTag::Rpc,
                    "RATE_LIMIT",
                    &format!(
                        "Rate limiting URL {}: waiting {:.2}ms",
                        url,
                        wait_duration.as_millis()
                    )
                );
                tokio::time::sleep(wait_duration).await;
            }
        }
        self.url_last_calls.insert(url.to_string(), Instant::now());
    }

    /// Record a 429 error and increase backoff
    pub fn record_429_error(&mut self, url: Option<&str>) {
        self.consecutive_429s += 1;
        self.consecutive_successes = 0;

        // Exponential backoff with jitter
        let backoff_factor = self.backoff_multiplier.powi(self.consecutive_429s as i32);
        let new_interval_ms = ((self.base_interval.as_millis() as f64) * backoff_factor) as u64;

        // Add 10% jitter to prevent thundering herd
        let jitter = ((new_interval_ms as f64) * 0.1) as u64;
        let jittered_interval = Duration::from_millis(new_interval_ms + jitter);

        self.current_interval = std::cmp::min(jittered_interval, self.max_interval);

        if let Some(url) = url {
            // Also update per-URL interval
            self.url_intervals.insert(url.to_string(), self.current_interval);
        }

        log(
            LogTag::Rpc,
            "RATE_LIMIT",
            &format!(
                "429 error #{}: increased interval to {:.2}ms (max: {:.2}ms)",
                self.consecutive_429s,
                self.current_interval.as_millis(),
                self.max_interval.as_millis()
            )
        );
    }

    /// Record a successful call and gradually reduce backoff
    pub fn record_success(&mut self, url: Option<&str>) {
        self.consecutive_successes += 1;

        // After 5 consecutive successes, reduce interval back towards normal
        if self.consecutive_successes >= 5 {
            self.consecutive_429s = self.consecutive_429s.saturating_sub(1);
            self.consecutive_successes = 0;

            if self.consecutive_429s == 0 {
                self.current_interval = self.base_interval;
                log(LogTag::Rpc, "RATE_LIMIT", "Rate limit backoff reset to normal");
            } else {
                let backoff_factor = self.backoff_multiplier.powi(self.consecutive_429s as i32);
                let new_interval_ms = ((self.base_interval.as_millis() as f64) *
                    backoff_factor) as u64;
                self.current_interval = Duration::from_millis(new_interval_ms);
                log(
                    LogTag::Rpc,
                    "RATE_LIMIT",
                    &format!(
                        "Reduced rate limit backoff to {:.2}ms (429s remaining: {})",
                        self.current_interval.as_millis(),
                        self.consecutive_429s
                    )
                );
            }

            if let Some(url) = url {
                self.url_intervals.insert(url.to_string(), self.current_interval);
            }
        }
    }

    /// Check if we need to wait for rate limit (without waiting)
    pub fn should_wait_for_main_rpc(&self, min_interval_ms: u64) -> bool {
        if let Some(last_call) = self.last_main_call {
            last_call.elapsed() < Duration::from_millis(min_interval_ms)
        } else {
            false
        }
    }

    /// Get current interval for main RPC
    pub fn get_current_interval(&self) -> Duration {
        self.current_interval
    }

    /// Get current backoff status
    pub fn get_backoff_status(&self) -> (u32, Duration) {
        (self.consecutive_429s, self.current_interval)
    }

    /// Force reset rate limiter (useful after switching RPC endpoints)
    pub fn reset(&mut self) {
        self.current_interval = self.base_interval;
        self.consecutive_429s = 0;
        self.consecutive_successes = 0;
        log(LogTag::Rpc, "RATE_LIMIT", "Rate limiter reset");
    }

    /// Set a custom interval for a specific URL (useful for premium RPCs)
    pub fn set_url_interval(&mut self, url: &str, interval: Duration) {
        self.url_intervals.insert(url.to_string(), interval);
        log(
            LogTag::Rpc,
            "RATE_LIMIT",
            &format!("Set custom interval for {}: {:.2}ms", url, interval.as_millis())
        );
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
    rate_limiter: Arc<tokio::sync::Mutex<RpcRateLimiter>>,
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
        let configs = read_configs().map_err(|e| format!("Failed to read configs: {}", e))?;

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
            rate_limiter: Arc::new(tokio::sync::Mutex::new(RpcRateLimiter::new_conservative())), // Conservative rate limiting to prevent 429s
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
            rate_limiter: Arc::new(tokio::sync::Mutex::new(RpcRateLimiter::new_conservative())), // Conservative rate limiting to prevent 429s
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
            // Always use the actual RPC URL, not a modified version
            let url_to_record = self.rpc_url.clone();
            stats.record_call(&url_to_record, method);
            // Stats are now auto-saved every 3 seconds by background service
        }
    }

    /// Record an RPC call for statistics for a specific URL (used when actual endpoint differs from default)
    fn record_call_for_url(&self, url: &str, method: &str) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(url, method);
            // Stats are now auto-saved every 3 seconds by background service
        }
    }

    /// Check if current URL is a premium RPC (no rate limiting)
    fn is_current_url_premium(&self) -> bool {
        if let Some(premium_url) = &self.premium_url { self.rpc_url == *premium_url } else { false }
    }

    /// Wait for rate limit if using main RPC (excludes premium RPC) with adaptive backoff
    async fn wait_for_rate_limit(&self) {
        // Only rate limit the main RPC URL, not fallbacks or premium URLs
        if self.current_url_index == 0 {
            // Skip rate limiting for premium RPC URLs
            if self.is_current_url_premium() {
                return;
            }

            // Use the enhanced rate limiter with adaptive backoff
            let mut rate_limiter = self.rate_limiter.lock().await;
            rate_limiter.wait_for_main_rpc().await;
        }
    }

    /// Wait for rate limit for a specific URL
    async fn wait_for_rate_limit_url(&self, url: &str) {
        // Skip rate limiting for premium RPC URLs
        if let Some(premium_url) = &self.premium_url {
            if url == premium_url {
                return;
            }
        }

        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.wait_for_url(url).await;
    }

    /// Record a successful RPC call for adaptive rate limiting
    fn record_success(&self, url: Option<&str>) {
        let rate_limiter = self.rate_limiter.clone();
        let url = url.map(|s| s.to_string());
        tokio::spawn(async move {
            let mut rate_limiter = rate_limiter.lock().await;
            rate_limiter.record_success(url.as_deref());
        });
    }

    /// Record a 429 error for adaptive rate limiting
    fn record_429_error(&self, url: Option<&str>) {
        let rate_limiter = self.rate_limiter.clone();
        let url = url.map(|s| s.to_string());
        tokio::spawn(async move {
            let mut rate_limiter = rate_limiter.lock().await;
            rate_limiter.record_429_error(url.as_deref());
        });
    }

    /// Get current rate limiter status for debugging
    pub async fn get_rate_limiter_status(&self) -> Option<(u32, Duration)> {
        let rate_limiter = self.rate_limiter.lock().await;
        Some(rate_limiter.get_backoff_status())
    }

    /// Set a custom rate limit interval for premium URLs
    pub async fn set_premium_rate_limit(&self, interval: Duration) {
        if let Some(premium_url) = &self.premium_url {
            let mut rate_limiter = self.rate_limiter.lock().await;
            rate_limiter.set_url_interval(premium_url, interval);
        }
    }

    /// Reset rate limiter (useful when switching networks or after prolonged downtime)
    pub async fn reset_rate_limiter(&self) {
        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.reset();
    }

    /// Create a new client using premium URL (for wallet operations - no rate limiting)
    pub fn create_premium_client(&self) -> Option<Arc<SolanaRpcClient>> {
        if let Some(premium_url) = &self.premium_url {
            log(
                LogTag::Rpc,
                "PREMIUM",
                &format!("Using premium RPC (no rate limits): {}", premium_url)
            );
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

    /// Check if error is specifically a 429 rate limit error
    fn is_rate_limit_error(error: &str) -> bool {
        let error_lower = error.to_lowercase();
        error_lower.contains("429") ||
            error_lower.contains("too many requests") ||
            error_lower.contains("rate limit")
    }

    /// Check if HTTP response indicates rate limiting
    fn is_rate_limit_response(response: &reqwest::Response) -> bool {
        response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
    }

    /// Get all available URLs (primary + fallbacks)
    pub fn get_all_urls(&self) -> Vec<String> {
        let mut urls = vec![self.rpc_url.clone()];
        urls.extend(self.fallback_urls.clone());
        urls
    }

    /// Switch to next fallback URL
    pub async fn switch_to_fallback(&mut self) -> Result<(), String> {
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

        // Reset rate limiter when switching endpoints to avoid carrying over 429 penalties
        self.reset_rate_limiter().await;

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
                        if let Err(switch_err) = self.switch_to_fallback().await {
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
                        if let Err(switch_err) = self.switch_to_fallback().await {
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
                        if let Err(switch_err) = self.switch_to_fallback().await {
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

    /// Get SOL balance for wallet address using main RPC first
    pub async fn get_sol_balance(&self, wallet_address: &str) -> Result<f64, SwapError> {
        self.wait_for_rate_limit().await;

        if is_debug_wallet_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!("Checking SOL balance for wallet: {}", &wallet_address[..8])
            );
        }

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [wallet_address]
        });

        let client = reqwest::Client::new();

        // Use main RPC URL first
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        let mut should_fallback = false;

        match
            client
                .post(&configs.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    self.record_429_error(Some(&configs.rpc_url)); // Record 429 for adaptive backoff
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit, falling back to premium"
                    );
                } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(balance_lamports) = value.as_u64() {
                                let balance_sol = lamports_to_sol(balance_lamports);
                                if is_debug_wallet_enabled() {
                                    log(
                                        LogTag::Rpc,
                                        "DEBUG",
                                        &format!(
                                            "SOL balance retrieved: {} lamports ({:.6} SOL) from main RPC",
                                            balance_lamports,
                                            balance_sol
                                        )
                                    );
                                }
                                // Record successful main RPC call
                                self.record_call_for_url(&configs.rpc_url, "get_balance");
                                self.record_success(Some(&configs.rpc_url)); // Record success for adaptive rate limiting
                                return Ok(balance_sol);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    self.record_429_error(Some(&configs.rpc_url)); // Record 429 for adaptive backoff
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get balance from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Err(SwapError::NetworkError(e));
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if !should_fallback {
            return Err(
                SwapError::TransactionError("Failed to get balance from main RPC".to_string())
            );
        }

        // Fallback to premium RPC
        let premium_rpc = &configs.rpc_url_premium;

        match
            client
                .post(premium_rpc)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(balance_lamports) = value.as_u64() {
                                let balance_sol = lamports_to_sol(balance_lamports);
                                if is_debug_wallet_enabled() {
                                    log(
                                        LogTag::Rpc,
                                        "DEBUG",
                                        &format!(
                                            "SOL balance retrieved: {} lamports ({:.6} SOL) from premium RPC",
                                            balance_lamports,
                                            balance_sol
                                        )
                                    );
                                }
                                // Record successful premium RPC call
                                self.record_call_for_url(premium_rpc, "get_balance");
                                self.record_success(Some(premium_rpc)); // Record success for adaptive rate limiting
                                return Ok(balance_sol);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(SwapError::NetworkError(e));
            }
        }

        Err(SwapError::TransactionError("Failed to get balance from all RPC endpoints".to_string()))
    }

    /// Get token balance for wallet address using main RPC first
    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        mint: &str
    ) -> Result<u64, SwapError> {
        self.wait_for_rate_limit().await;

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet_address,
                {
                    "mint": mint
                },
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });

        let client = reqwest::Client::new();

        // Use main RPC URL first
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        let mut should_fallback = false;

        match
            client
                .post(&configs.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit for token balance, falling back to premium"
                    );
                } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                if let Some(account) = accounts.first() {
                                    if let Some(account_data) = account.get("account") {
                                        if let Some(data) = account_data.get("data") {
                                            if let Some(parsed) = data.get("parsed") {
                                                if let Some(info) = parsed.get("info") {
                                                    if
                                                        let Some(token_amount) =
                                                            info.get("tokenAmount")
                                                    {
                                                        if
                                                            let Some(amount_str) =
                                                                token_amount.get("amount")
                                                        {
                                                            if
                                                                let Some(amount_str) =
                                                                    amount_str.as_str()
                                                            {
                                                                if
                                                                    let Ok(amount) =
                                                                        amount_str.parse::<u64>()
                                                                {
                                                                    // Record successful main RPC call
                                                                    self.record_call_for_url(
                                                                        &configs.rpc_url,
                                                                        "get_token_accounts_by_owner"
                                                                    );
                                                                    return Ok(amount);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // No token account found, return 0 (don't fallback for missing accounts)
                            self.record_call_for_url(
                                &configs.rpc_url,
                                "get_token_accounts_by_owner"
                            );
                            return Ok(0);
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited for token balance: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get token balance from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Ok(0); // Return 0 for non-rate-limit errors
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if !should_fallback {
            return Ok(0); // Return 0 if main RPC failed for non-rate-limit reasons
        }

        // Fallback to premium RPC
        let premium_rpc = &configs.rpc_url_premium;

        match
            client
                .post(premium_rpc)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                if let Some(account) = accounts.first() {
                                    if let Some(account_data) = account.get("account") {
                                        if let Some(data) = account_data.get("data") {
                                            if let Some(parsed) = data.get("parsed") {
                                                if let Some(info) = parsed.get("info") {
                                                    if
                                                        let Some(token_amount) =
                                                            info.get("tokenAmount")
                                                    {
                                                        if
                                                            let Some(amount_str) =
                                                                token_amount.get("amount")
                                                        {
                                                            if
                                                                let Some(amount_str) =
                                                                    amount_str.as_str()
                                                            {
                                                                if
                                                                    let Ok(amount) =
                                                                        amount_str.parse::<u64>()
                                                                {
                                                                    // Record successful premium RPC call
                                                                    self.record_call_for_url(
                                                                        premium_rpc,
                                                                        "get_token_accounts_by_owner"
                                                                    );
                                                                    return Ok(amount);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Rpc,
                    "WARNING",
                    &format!("Failed to get token balance from premium RPC: {}", e)
                );
            }
        }

        Ok(0) // Return 0 if no token account found or all RPCs failed
    }

    /// Get latest blockhash using main RPC first
    pub async fn get_latest_blockhash(&self) -> Result<Hash, SwapError> {
        self.wait_for_rate_limit().await;

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [
                {
                    "commitment": "finalized"
                }
            ]
        });

        let client = reqwest::Client::new();

        // Use main RPC URL first
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        let mut should_fallback = false;

        match
            client
                .post(&configs.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit for blockhash, falling back to premium"
                    );
                } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if
                                let Some(blockhash_str) = value
                                    .get("blockhash")
                                    .and_then(|b| b.as_str())
                            {
                                if let Ok(blockhash) = Hash::from_str(blockhash_str) {
                                    return Ok(blockhash);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited for blockhash: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get latest blockhash from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Err(SwapError::NetworkError(e));
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if !should_fallback {
            return Err(
                SwapError::TransactionError(
                    "Failed to get latest blockhash from main RPC".to_string()
                )
            );
        }

        // Fallback to premium RPC
        let premium_rpc = &configs.rpc_url_premium;

        match
            client
                .post(premium_rpc)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if
                                let Some(blockhash_str) = value
                                    .get("blockhash")
                                    .and_then(|b| b.as_str())
                            {
                                if let Ok(blockhash) = Hash::from_str(blockhash_str) {
                                    return Ok(blockhash);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(SwapError::NetworkError(e));
            }
        }

        Err(
            SwapError::TransactionError(
                "Failed to get latest blockhash from all RPC endpoints".to_string()
            )
        )
    }

    /// Send transaction using premium RPC
    pub async fn send_transaction(&self, transaction: &Transaction) -> Result<String, SwapError> {
        self.wait_for_rate_limit().await;

        // Serialize transaction
        let serialized_tx = bincode
            ::serialize(transaction)
            .map_err(|e|
                SwapError::TransactionError(format!("Failed to serialize transaction: {}", e))
            )?;

        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&serialized_tx);

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                tx_base64,
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "preflightCommitment": "processed",
                    "maxRetries": 3
                }
            ]
        });

        let client = reqwest::Client::new();

        // Use premium RPC URL first
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        let premium_rpc = &configs.rpc_url_premium;

        log(LogTag::Rpc, "INFO", "Sending transaction to premium RPC...");

        match
            client
                .post(premium_rpc)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(signature) = result.as_str() {
                                log(
                                    LogTag::Rpc,
                                    "SUCCESS",
                                    &format!("Transaction sent successfully via premium RPC: {}", signature)
                                );
                                return Ok(signature.to_string());
                            }
                        }

                        if let Some(error) = rpc_response.get("error") {
                            let error_msg = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown RPC error");
                            log(LogTag::Rpc, "ERROR", &format!("Premium RPC error: {}", error_msg));
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Rpc,
                    "ERROR",
                    &format!("Failed to send transaction to premium RPC: {}", e)
                );
            }
        }

        // Fallback to main RPC
        log(LogTag::Rpc, "INFO", "Fallback: Sending transaction to main RPC...");

        match
            client
                .post(&configs.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(signature) = result.as_str() {
                                log(
                                    LogTag::Rpc,
                                    "SUCCESS",
                                    &format!("Transaction sent successfully via main RPC: {}", signature)
                                );
                                return Ok(signature.to_string());
                            }
                        }

                        if let Some(error) = rpc_response.get("error") {
                            let error_msg = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown RPC error");
                            return Err(
                                SwapError::TransactionError(format!("RPC error: {}", error_msg))
                            );
                        }
                    }
                }
            }
            Err(e) => {
                return Err(SwapError::NetworkError(e));
            }
        }

        Err(
            SwapError::TransactionError(
                "Failed to send transaction to all RPC endpoints".to_string()
            )
        )
    }

    /// Sign and send transaction using premium RPC
    pub async fn sign_and_send_transaction(
        &self,
        swap_transaction_base64: &str
    ) -> Result<String, SwapError> {
        self.wait_for_rate_limit().await;

        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        if is_debug_wallet_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Starting transaction signing: tx_length={} bytes",
                    swap_transaction_base64.len()
                )
            );
        }

        log(
            LogTag::Rpc,
            "SIGN",
            &format!(
                "Signing transaction with wallet (length: {} bytes)",
                swap_transaction_base64.len()
            )
        );

        // Decode the base64 transaction
        let transaction_bytes = base64::engine::general_purpose::STANDARD
            .decode(swap_transaction_base64)
            .map_err(|e| SwapError::SigningError(format!("Failed to decode transaction: {}", e)))?;

        // Deserialize the VersionedTransaction
        let mut transaction: VersionedTransaction = bincode
            ::deserialize(&transaction_bytes)
            .map_err(|e|
                SwapError::SigningError(format!("Failed to deserialize transaction: {}", e))
            )?;

        // Create keypair from private key
        let private_key_bytes = bs58
            ::decode(&configs.main_wallet_private)
            .into_vec()
            .map_err(|e| SwapError::ConfigError(format!("Invalid private key format: {}", e)))?;

        let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
            SwapError::ConfigError(format!("Failed to create keypair: {}", e))
        )?;

        // Sign the transaction
        let signature = keypair.sign_message(&transaction.message.serialize());

        if is_debug_wallet_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Transaction signed successfully: wallet_pubkey={}, signature={}",
                    keypair.pubkey(),
                    signature
                )
            );
        }

        // Add the signature to the transaction
        if transaction.signatures.is_empty() {
            transaction.signatures.push(signature);
        } else {
            transaction.signatures[0] = signature;
        }

        // Serialize the signed transaction back to base64
        let signed_transaction_bytes = bincode
            ::serialize(&transaction)
            .map_err(|e|
                SwapError::SigningError(format!("Failed to serialize signed transaction: {}", e))
            )?;
        let signed_transaction_base64 = base64::engine::general_purpose::STANDARD.encode(
            &signed_transaction_bytes
        );

        // Send the signed transaction using our send method
        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                signed_transaction_base64,
                {
                    "encoding": "base64",
                    "skipPreflight": false,
                    "preflightCommitment": "processed"
                }
            ]
        });

        let client = reqwest::Client::new();

        // Use premium RPC URL first
        let premium_rpc = &configs.rpc_url_premium;

        log(LogTag::Rpc, "SEND", "Sending signed transaction to premium RPC");

        match
            client
                .post(premium_rpc)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(tx_sig) = result.as_str() {
                                log(
                                    LogTag::Rpc,
                                    "SUCCESS",
                                    &format!("Transaction sent successfully via premium RPC: {}", tx_sig)
                                );
                                // Record successful premium RPC call
                                self.record_call_for_url(premium_rpc, "send_transaction");
                                return Ok(tx_sig.to_string());
                            }
                        }

                        if let Some(error) = rpc_response.get("error") {
                            let error_msg = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown RPC error");
                            log(LogTag::Rpc, "ERROR", &format!("Premium RPC error: {}", error_msg));
                        }
                    }
                }
            }
            Err(e) => {
                log(LogTag::Rpc, "ERROR", &format!("Failed to send to premium RPC: {}", e));
            }
        }

        // Try fallback RPCs
        for fallback_rpc in &configs.rpc_fallbacks {
            log(LogTag::Rpc, "SEND", &format!("Trying fallback RPC: {}", fallback_rpc));

            match
                client
                    .post(fallback_rpc)
                    .header("Content-Type", "application/json")
                    .json(&rpc_payload)
                    .send().await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                            if let Some(result) = rpc_response.get("result") {
                                if let Some(tx_sig) = result.as_str() {
                                    log(
                                        LogTag::Rpc,
                                        "SUCCESS",
                                        &format!("Transaction sent via fallback RPC: {}", tx_sig)
                                    );
                                    // Record successful fallback RPC call
                                    self.record_call_for_url(fallback_rpc, "send_transaction");
                                    return Ok(tx_sig.to_string());
                                }
                            }

                            if let Some(error) = rpc_response.get("error") {
                                let error_msg = error
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("Unknown RPC error");
                                log(
                                    LogTag::Rpc,
                                    "ERROR",
                                    &format!("Fallback RPC {} error: {}", fallback_rpc, error_msg)
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Fallback RPC {} failed: {}", fallback_rpc, e)
                    );
                }
            }
        }

        // Try main RPC as last resort
        log(LogTag::Rpc, "SEND", "Trying main RPC as last resort");

        match
            client
                .post(&configs.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(tx_sig) = result.as_str() {
                                log(
                                    LogTag::Rpc,
                                    "SUCCESS",
                                    &format!("Transaction sent via main RPC: {}", tx_sig)
                                );
                                // Record successful main RPC call
                                self.record_call_for_url(&configs.rpc_url, "send_transaction");
                                return Ok(tx_sig.to_string());
                            }
                        }

                        if let Some(error) = rpc_response.get("error") {
                            let error_msg = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown RPC error");
                            return Err(
                                SwapError::TransactionError(format!("RPC error: {}", error_msg))
                            );
                        }
                    }
                }
            }
            Err(e) => {
                return Err(SwapError::NetworkError(e));
            }
        }

        Err(SwapError::TransactionError("All RPC endpoints failed".to_string()))
    }

    /// Gets all token accounts for a wallet (both SPL Token and Token-2022)
    pub async fn get_all_token_accounts(
        &self,
        wallet_address: &str
    ) -> Result<Vec<TokenAccountInfo>, SwapError> {
        // Record call in stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, "getTokenAccountsByOwner");
        }

        let spl_token_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet_address,
                {
                    "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        let token_2022_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet_address,
                {
                    "programId": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        let client = reqwest::Client::new();
        let mut all_accounts = Vec::new();
        let mut should_fallback = false;

        // Try main RPC first
        for payload in [&spl_token_payload, &token_2022_payload] {
            match
                client
                    .post(&self.rpc_url)
                    .header("Content-Type", "application/json")
                    .json(payload)
                    .send().await
            {
                Ok(response) => {
                    // Check if response indicates rate limiting
                    if Self::is_rate_limit_response(&response) {
                        should_fallback = true;
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            "Main RPC returned 429 rate limit for token accounts, will fallback to premium"
                        );
                        break;
                    } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(accounts) = value.as_array() {
                                    let is_token_2022 = payload == &token_2022_payload;
                                    for account in accounts {
                                        if
                                            let Some(parsed_info) = extract_token_account_info(
                                                account,
                                                is_token_2022
                                            )
                                        {
                                            all_accounts.push(parsed_info);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if Self::is_rate_limit_error(&error_msg) {
                        should_fallback = true;
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            &format!("Main RPC rate limited for token accounts: {}, will fallback to premium", error_msg)
                        );
                        break;
                    } else {
                        log(
                            LogTag::Rpc,
                            "ERROR",
                            &format!("Failed to get token accounts from main RPC (non-rate-limit): {}", error_msg)
                        );
                        return Err(SwapError::NetworkError(e));
                    }
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if should_fallback {
            log(LogTag::Rpc, "INFO", "Falling back to premium RPC for token accounts");
            if let Some(premium_url) = &self.premium_url {
                for payload in [&spl_token_payload, &token_2022_payload] {
                    if
                        let Ok(response) = client
                            .post(premium_url)
                            .header("Content-Type", "application/json")
                            .json(payload)
                            .send().await
                    {
                        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                            if let Some(result) = rpc_response.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let Some(accounts) = value.as_array() {
                                        let is_token_2022 = payload == &token_2022_payload;
                                        for account in accounts {
                                            if
                                                let Some(parsed_info) = extract_token_account_info(
                                                    account,
                                                    is_token_2022
                                                )
                                            {
                                                all_accounts.push(parsed_info);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        log(
            LogTag::Rpc,
            "ATA",
            &format!("Found {} total token accounts for wallet via RPC", all_accounts.len())
        );
        Ok(all_accounts)
    }

    /// Checks if a token account (not mint) is a Token-2022 account by checking the account owner
    pub async fn is_token_account_token_2022(
        &self,
        token_account: &str
    ) -> Result<bool, SwapError> {
        // Record call in stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, "getAccountInfo");
        }

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                token_account,
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        let client = reqwest::Client::new();
        let mut should_fallback = false;

        // Try main RPC first
        match
            client
                .post(&self.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit for token account info, falling back to premium"
                    );
                } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(owner) = value.get("owner") {
                                if let Some(owner_str) = owner.as_str() {
                                    // Token Extensions Program ID (Token-2022)
                                    return Ok(
                                        owner_str == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited for token account info: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get token account info from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Ok(false); // Default to false for non-rate-limit errors
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if should_fallback {
            if let Some(premium_url) = &self.premium_url {
                match
                    client
                        .post(premium_url)
                        .header("Content-Type", "application/json")
                        .json(&rpc_payload)
                        .send().await
                {
                    Ok(response) => {
                        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                            if let Some(result) = rpc_response.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let Some(owner) = value.get("owner") {
                                        if let Some(owner_str) = owner.as_str() {
                                            // Token Extensions Program ID (Token-2022)
                                            return Ok(
                                                owner_str ==
                                                    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            &format!("Failed to get token account info from premium RPC: {}", e)
                        );
                    }
                }
            }
        }

        // Default to false if we can't determine
        Ok(false)
    }

    /// Checks if a mint is a Token-2022 mint by checking its owner program
    pub async fn is_token_2022_mint(&self, mint: &str) -> Result<bool, SwapError> {
        // Record call in stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, "getAccountInfo");
        }

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                mint,
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        let client = reqwest::Client::new();
        let mut should_fallback = false;

        // Try main RPC first
        match
            client
                .post(&self.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit for account info, falling back to premium"
                    );
                } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(owner) = value.get("owner") {
                                if let Some(owner_str) = owner.as_str() {
                                    // Token Extensions Program ID (Token-2022)
                                    return Ok(
                                        owner_str == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited for account info: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get account info from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Ok(false); // Default to false for non-rate-limit errors
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if should_fallback {
            if let Some(premium_url) = &self.premium_url {
                match
                    client
                        .post(premium_url)
                        .header("Content-Type", "application/json")
                        .json(&rpc_payload)
                        .send().await
                {
                    Ok(response) => {
                        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                            if let Some(result) = rpc_response.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let Some(owner) = value.get("owner") {
                                        if let Some(owner_str) = owner.as_str() {
                                            // Token Extensions Program ID (Token-2022)
                                            return Ok(
                                                owner_str ==
                                                    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            &format!("Failed to get account info from premium RPC: {}", e)
                        );
                    }
                }
            }
        }

        // Default to false if we can't determine
        Ok(false)
    }

    /// Gets transaction details from RPC to analyze balance changes
    pub async fn get_transaction_details(
        &self,
        transaction_signature: &str
    ) -> Result<TransactionDetails, SwapError> {
        // Record call in stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, "getTransaction");
        }

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                transaction_signature,
                {
                    "encoding": "json",
                    "maxSupportedTransactionVersion": 0
                }
            ]
        });

        let client = reqwest::Client::new();
        let mut should_fallback = false;

        // Try main RPC first
        match
            client
                .post(&self.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit for transaction details, falling back to premium"
                    );
                } else if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(error) = rpc_response.get("error") {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("RPC error getting transaction: {:?}", error)
                            );
                            return Err(
                                SwapError::TransactionError(format!("RPC error: {:?}", error))
                            );
                        }

                        if let Some(result) = rpc_response.get("result") {
                            if result.is_null() {
                                return Err(
                                    SwapError::TransactionError(
                                        "Transaction not found or not confirmed yet".to_string()
                                    )
                                );
                            }

                            let transaction_details: TransactionDetails = serde_json
                                ::from_value(result.clone())
                                .map_err(|e|
                                    SwapError::InvalidResponse(
                                        format!("Failed to parse transaction details: {}", e)
                                    )
                                )?;

                            return Ok(transaction_details);
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited for transaction details: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get transaction details from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Err(SwapError::NetworkError(e));
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if should_fallback {
            if let Some(premium_url) = &self.premium_url {
                match
                    client
                        .post(premium_url)
                        .header("Content-Type", "application/json")
                        .json(&rpc_payload)
                        .send().await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                                if let Some(error) = rpc_response.get("error") {
                                    log(
                                        LogTag::Rpc,
                                        "ERROR",
                                        &format!(
                                            "RPC error getting transaction from premium: {:?}",
                                            error
                                        )
                                    );
                                    return Err(
                                        SwapError::TransactionError(
                                            format!("RPC error: {:?}", error)
                                        )
                                    );
                                }

                                if let Some(result) = rpc_response.get("result") {
                                    if result.is_null() {
                                        return Err(
                                            SwapError::TransactionError(
                                                "Transaction not found or not confirmed yet".to_string()
                                            )
                                        );
                                    }

                                    let transaction_details: TransactionDetails = serde_json
                                        ::from_value(result.clone())
                                        .map_err(|e|
                                            SwapError::InvalidResponse(
                                                format!("Failed to parse transaction details: {}", e)
                                            )
                                        )?;

                                    return Ok(transaction_details);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            &format!("Failed to get transaction details from premium RPC: {}", e)
                        );
                    }
                }
            }
        }

        Err(
            SwapError::TransactionError(
                "Failed to get transaction details from main RPC".to_string()
            )
        )
    }

    /// Gets the associated token account address for a wallet and mint
    pub async fn get_associated_token_account(
        &self,
        wallet_address: &str,
        mint: &str
    ) -> Result<String, SwapError> {
        // Record call in stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, "getTokenAccountsByOwner");
        }

        let rpc_payload =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet_address,
                {
                    "mint": mint
                },
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        let client = reqwest::Client::new();
        let mut should_fallback = false;

        // Try main RPC first
        match
            client
                .post(&self.rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                // Check if response indicates rate limiting
                if Self::is_rate_limit_response(&response) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        "Main RPC returned 429 rate limit for associated token account, falling back to premium"
                    );
                } else if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                if !accounts.is_empty() {
                                    if let Some(account) = accounts.first() {
                                        if let Some(pubkey) = account.get("pubkey") {
                                            if let Some(pubkey_str) = pubkey.as_str() {
                                                return Ok(pubkey_str.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // If no accounts found, this is not an error - return the standard error
                    return Err(
                        SwapError::InvalidResponse("No associated token account found".to_string())
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    should_fallback = true;
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!("Main RPC rate limited for associated token account: {}, falling back to premium", error_msg)
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get associated token account from main RPC (non-rate-limit): {}", error_msg)
                    );
                    return Err(SwapError::NetworkError(e));
                }
            }
        }

        // Only fallback to premium RPC on 429/rate limit errors
        if should_fallback {
            if let Some(premium_url) = &self.premium_url {
                match
                    client
                        .post(premium_url)
                        .header("Content-Type", "application/json")
                        .json(&rpc_payload)
                        .send().await
                {
                    Ok(response) => {
                        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                            if let Some(result) = rpc_response.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let Some(accounts) = value.as_array() {
                                        if !accounts.is_empty() {
                                            if let Some(account) = accounts.first() {
                                                if let Some(pubkey) = account.get("pubkey") {
                                                    if let Some(pubkey_str) = pubkey.as_str() {
                                                        return Ok(pubkey_str.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Rpc,
                            "WARNING",
                            &format!("Failed to get associated token account from premium RPC: {}", e)
                        );
                    }
                }
            }
        }

        Err(SwapError::InvalidResponse("No associated token account found".to_string()))
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

/// Start RPC stats auto-save background task
pub async fn start_rpc_stats_auto_save_service(shutdown: Arc<tokio::sync::Notify>) {
    log(LogTag::Rpc, "START", "Starting RPC stats auto-save service (every 3 seconds)");

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Rpc, "SHUTDOWN", "RPC stats auto-save service stopping");
                // Final save before shutdown
                if let Err(e) = save_global_rpc_stats() {
                    log(LogTag::Rpc, "ERROR", &format!("Final stats save failed: {}", e));
                } else {
                    log(LogTag::Rpc, "INFO", "Final RPC stats saved before shutdown");
                }
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = save_global_rpc_stats() {
                    log(LogTag::Rpc, "ERROR", &format!("Auto-save failed: {}", e));
                }
            }
        }
    }

    log(LogTag::Rpc, "STOP", "RPC stats auto-save service stopped");
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

/// Extracts token account information from RPC response
fn extract_token_account_info(
    account: &serde_json::Value,
    is_token_2022: bool
) -> Option<TokenAccountInfo> {
    let pubkey = account.get("pubkey")?.as_str()?;
    let account_data = account.get("account")?;
    let parsed = account_data.get("data")?.get("parsed")?;
    let info = parsed.get("info")?;

    let mint = info.get("mint")?.as_str()?;
    let token_amount = info.get("tokenAmount")?;
    let amount_str = token_amount.get("amount")?.as_str()?;
    let balance = amount_str.parse::<u64>().ok()?;

    Some(TokenAccountInfo {
        account: pubkey.to_string(),
        mint: mint.to_string(),
        balance,
        is_token_2022,
    })
}
