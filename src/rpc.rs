/// Centralized RPC Client for Solana
///
/// This module provides a centralized RPC client that can be used throughout the application
/// for consistent RPC configuration and connection management.

/// FORCE PREMIUM RPC ONLY - When set to true, ALL RPC calls will use only premium RPC URL
/// This bypasses all fallback logic and main RPC usage for maximum reliability
/// 
/// USAGE:
/// - Set to `true` to force all RPC operations to use only the premium RPC endpoint
/// - Set to `false` for normal operation (main RPC with premium fallback)
/// 
/// When enabled, the following methods use ONLY premium RPC:
/// - get_ata_rent_lamports()
/// - get_sol_balance()
/// - get_token_balance()
/// - get_latest_blockhash()
/// - send_transaction()
/// - sign_and_send_transaction()
/// - get_transaction_details()
/// - get_wallet_signatures_main_rpc()
/// 
/// WARNING: If premium RPC fails when this is enabled, operations will fail
/// instead of falling back to other endpoints.
const FORCE_PREMIUM_RPC_ONLY: bool = false;

use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_rpc_enabled, is_debug_transactions_enabled, is_debug_wallet_enabled, read_configs, RPC_STATS };
use crate::tokens::decimals::{LAMPORTS_PER_SOL};
use solana_client::rpc_client::RpcClient as SolanaRpcClient;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    commitment_config::CommitmentConfig,
    client::SyncClient,
    transaction::VersionedTransaction,
    signer::Signer,
    signature::{Keypair, Signature},
    transaction::Transaction,
    hash::Hash,
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::sync::Arc;
use std::str::FromStr;
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };
use base64::Engine as _;
use reqwest;
use serde_json;
use bincode;
use bs58;
use futures;
use once_cell::sync::Lazy;
use std::sync::{Arc as StdArc, Mutex as StdMutex};
use url::Url;

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
    #[serde(rename = "innerInstructions")]
    pub inner_instructions: Option<Vec<serde_json::Value>>,
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

/// Signature status response structure for getSignatureStatuses
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureStatusResponse {
    pub result: SignatureStatusResult,
}

/// Result wrapper for signature status response  
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureStatusResult {
    pub value: Vec<Option<SignatureStatusData>>,
}

/// Individual signature status data
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureStatusData {
    #[serde(rename = "confirmationStatus")]
    pub confirmation_status: Option<String>,
    pub err: Option<serde_json::Value>,
}

/// Cached ATA rent information
#[derive(Debug, Clone)]
pub struct AtaRentInfo {
    pub rent_lamports: u64,
    pub cached_at: Instant,
}

/// Global cache for ATA rent amounts (10-second cache)
static ATA_RENT_CACHE: Lazy<StdArc<StdMutex<Option<AtaRentInfo>>>> = 
    Lazy::new(|| StdArc::new(StdMutex::new(None)));

/// Check if premium RPC only mode is active
fn is_premium_rpc_only() -> bool {
    FORCE_PREMIUM_RPC_ONLY
}

/// Get current ATA rent amount from chain with 10-second cache
pub async fn get_ata_rent_lamports() -> Result<u64, SwapError> {
    // Check cache first with safe lock handling
    {
        let cache = match ATA_RENT_CACHE.try_lock() {
            Ok(cache) => cache,
            Err(_) => {
                // If we can't get the cache lock, fall back to default value
                log(LogTag::Rpc, "WARN", "ATA rent cache lock contention - using default ATA rent");
                return Ok(2039280); // Default ATA rent: 0.00203928 SOL
            }
        };
        if let Some(ref info) = *cache {
            if info.cached_at.elapsed() < Duration::from_secs(10) {
                return Ok(info.rent_lamports);
            }
        }
    }

    // Cache miss or expired, fetch from chain
    let rpc_client = get_rpc_client();
    
    let rpc_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getMinimumBalanceForRentExemption",
        "params": [165] // ATA account size: 165 bytes for standard token account
    });

    let client = reqwest::Client::new();
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // If premium RPC only mode is active, use only premium RPC
    if is_premium_rpc_only() {
        if is_debug_rpc_enabled(){
            log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - using only premium RPC for ATA rent");
        }

        let response = client
            .post(&configs.rpc_url_premium)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await?;

        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
            if let Some(result) = rpc_response.get("result") {
                if let Some(rent_lamports) = result.as_u64() {
                    // Update cache with safe lock handling
                    {
                        if let Ok(mut cache) = ATA_RENT_CACHE.try_lock() {
                            *cache = Some(AtaRentInfo {
                                rent_lamports,
                                cached_at: Instant::now(),
                            });
                        } else {
                            log(LogTag::Rpc, "WARN", "Failed to update ATA rent cache - lock contention");
                        }
                    }
                    
                    log(LogTag::Rpc, "ATA_RENT", &format!("Retrieved ATA rent from premium RPC: {} lamports ({:.9} SOL)", 
                        rent_lamports, lamports_to_sol(rent_lamports)));
                    
                    return Ok(rent_lamports);
                }
            }
        }

        // If premium RPC fails and we're in premium-only mode, fallback to typical rent
        const FALLBACK_ATA_RENT: u64 = 2_039_280;
        log(LogTag::Rpc, "ATA_RENT_FALLBACK", 
            &format!("Premium RPC failed in premium-only mode, using fallback: {} lamports", FALLBACK_ATA_RENT));
        
        return Ok(FALLBACK_ATA_RENT);
    }

    // Normal mode: Try premium RPC first for better reliability
    let response = client
        .post(&configs.rpc_url_premium)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send()
        .await?;

    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
        if let Some(result) = rpc_response.get("result") {
            if let Some(rent_lamports) = result.as_u64() {
                // Update cache
                {
                    match ATA_RENT_CACHE.try_lock() {
                        Ok(mut cache) => {
                            *cache = Some(AtaRentInfo {
                                rent_lamports,
                                cached_at: Instant::now(),
                            });
                        }
                        Err(_) => {
                            log(LogTag::Rpc, "WARN", "ATA_RENT_CACHE lock contention during update - cache not updated");
                        }
                    }
                }
                
                log(LogTag::Rpc, "ATA_RENT", &format!("Retrieved ATA rent from chain: {} lamports ({:.9} SOL)", 
                    rent_lamports, lamports_to_sol(rent_lamports)));
                
                return Ok(rent_lamports);
            }
        }
    }

    // Fallback to main RPC if premium fails
    let response = client
        .post(&configs.rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send()
        .await?;

    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
        if let Some(result) = rpc_response.get("result") {
            if let Some(rent_lamports) = result.as_u64() {
                // Update cache
                {
                    match ATA_RENT_CACHE.try_lock() {
                        Ok(mut cache) => {
                            *cache = Some(AtaRentInfo {
                                rent_lamports,
                                cached_at: Instant::now(),
                            });
                        }
                        Err(_) => {
                            log(LogTag::Rpc, "WARN", "ATA_RENT_CACHE lock contention during fallback update - cache not updated");
                        }
                    }
                }
                
                log(LogTag::Rpc, "ATA_RENT", &format!("Retrieved ATA rent from chain (fallback): {} lamports ({:.9} SOL)", 
                    rent_lamports, lamports_to_sol(rent_lamports)));
                
                return Ok(rent_lamports);
            }
        }
    }

    // If all fails, return typical ATA rent as last resort
    const FALLBACK_ATA_RENT: u64 = 2_039_280;
    log(LogTag::Rpc, "ATA_RENT_FALLBACK", 
        &format!("Using fallback ATA rent: {} lamports", FALLBACK_ATA_RENT));
    
    Ok(FALLBACK_ATA_RENT)
}

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

/// Return the SPL Token program id (legacy Tokenkeg)
pub fn spl_token_program_id() -> &'static str {
    // SPL Token Program (legacy)
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
}

/// Derive a websocket URL from the configured HTTP RPC URL
/// Examples:
///  - https://api.mainnet-beta.solana.com -> wss://api.mainnet-beta.solana.com
///  - http://localhost:8899 -> ws://localhost:8899
pub fn get_websocket_url() -> Result<String, SwapError> {
    let cfg = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let http = cfg.rpc_url;
    let parsed = Url::parse(&http).map_err(|e| SwapError::ConfigError(format!("Invalid RPC URL: {}", e)))?;
    let ws_scheme = match parsed.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            // Default to secure websocket for unknown schemes
            if is_debug_rpc_enabled() {
                log(LogTag::Rpc, "WS_URL_SCHEME_WARN", &format!("Unknown scheme '{}', defaulting to wss", other));
            }
            "wss"
        }
    };
    let mut ws_url = parsed.clone();
    ws_url.set_scheme(ws_scheme).map_err(|_| SwapError::ConfigError("Failed to set WS scheme".to_string()))?;
    Ok(ws_url.to_string())
}

/// Derive a websocket URL from the configured PREMIUM HTTP RPC URL
/// Examples:
///  - https://premium.rpc.provider -> wss://premium.rpc.provider
///  - http://localhost:8899 -> ws://localhost:8899
pub fn get_premium_websocket_url() -> Result<String, SwapError> {
    let cfg = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let http = cfg.rpc_url_premium;
    let parsed = Url::parse(&http).map_err(|e| SwapError::ConfigError(format!("Invalid PREMIUM RPC URL: {}", e)))?;
    let ws_scheme = match parsed.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            if is_debug_rpc_enabled() {
                log(LogTag::Rpc, "WS_URL_SCHEME_WARN", &format!("Unknown scheme '{}' for premium URL, defaulting to wss", other));
            }
            "wss"
        }
    };
    let mut ws_url = parsed.clone();
    ws_url.set_scheme(ws_scheme).map_err(|_| SwapError::ConfigError("Failed to set WS scheme (premium)".to_string()))?;
    Ok(ws_url.to_string())
}

/// Build a JSON-RPC logsSubscribe payload for mentions filter
pub fn build_logs_subscribe_payload(mentions: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            { "mentions": mentions },
            { "commitment": "finalized" }
        ]
    })
}

/// Check if a logs array contains an InitializeMint instruction
pub fn logs_contains_initialize_mint(logs: &[String]) -> bool {
    logs.iter().any(|l| l.contains("InitializeMint"))
}

/// Check if a logs array contains an InitializeAccount instruction (including v3)
pub fn logs_contains_initialize_account(logs: &[String]) -> bool {
    logs.iter().any(|l| l.contains("InitializeAccount"))
        || logs.iter().any(|l| l.contains("InitializeAccount3"))
}

/// Converts lamports to SOL amount
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / LAMPORTS_PER_SOL as f64
}

/// Converts SOL amount to lamports (1 SOL = 1,000,000,000 lamports)
pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * LAMPORTS_PER_SOL as f64) as u64
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
        match self.stats.try_lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                log(LogTag::Rpc, "WARN", "RPC stats lock contention - returning default stats");
                RpcStats::default()
            }
        }
    }

    /// Save RPC statistics to disk
    pub fn save_stats(&self) -> Result<(), String> {
        match self.stats.try_lock() {
            Ok(mut stats) => stats.save_to_disk(),
            Err(_) => {
                log(LogTag::Rpc, "WARN", "RPC stats lock contention during save - stats not saved");
                Err("Failed to acquire stats lock for saving".to_string())
            }
        }
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

    /// Create a client specifically for main RPC (for lightweight operations like checking signatures)
    pub fn create_main_client(&self) -> Arc<SolanaRpcClient> {
        log(
            LogTag::Rpc,
            "MAIN",
            &format!("Using main RPC for lightweight operations: {}", self.rpc_url)
        );
        let client = SolanaRpcClient::new_with_commitment(
            self.rpc_url.clone(),
            CommitmentConfig::confirmed()
        );
        Arc::new(client)
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
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        // If premium RPC only mode is active, use only premium RPC
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - using only premium RPC for SOL balance");
            }

            match
                client
                    .post(&configs.rpc_url_premium)
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
                                                "SOL balance retrieved: {} lamports ({:.6} SOL) from premium RPC only",
                                                balance_lamports,
                                                balance_sol
                                            )
                                        );
                                    }
                                    self.record_call_for_url(&configs.rpc_url_premium, "get_balance");
                                    self.record_success(Some(&configs.rpc_url_premium));
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

            return Err(
                SwapError::TransactionError("Premium RPC failed in premium-only mode".to_string())
            );
        }

        // Normal mode: Use main RPC first
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
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        // If premium RPC only mode is active, use only premium RPC
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - using only premium RPC for token balance");
            }

            match
                client
                    .post(&configs.rpc_url_premium)
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
                                                                        self.record_call_for_url(
                                                                            &configs.rpc_url_premium,
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
                                // No token account found
                                self.record_call_for_url(
                                    &configs.rpc_url_premium,
                                    "get_token_accounts_by_owner"
                                );
                                return Ok(0);
                            }
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Premium RPC failed in premium-only mode for token balance: {}", e)
                    );
                    return Ok(0);
                }
            }
            
            return Ok(0);
        }

        // Normal mode: Use main RPC first
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
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        // If premium RPC only mode is active, use only premium RPC
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - using only premium RPC for blockhash");
            }

            match
                client
                    .post(&configs.rpc_url_premium)
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

            return Err(
                SwapError::TransactionError("Premium RPC failed in premium-only mode for blockhash".to_string())
            );
        }

        // Normal mode: Use main RPC first
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
        let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

        // If premium RPC only mode is active, use only premium RPC
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - sending transaction to premium RPC only");
            }

            match
                client
                    .post(&configs.rpc_url_premium)
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
                                        &format!("Transaction sent successfully via premium RPC only: {}", signature)
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
                                    SwapError::TransactionError(format!("Premium RPC error: {}", error_msg))
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(SwapError::NetworkError(e));
                }
            }

            return Err(
                SwapError::TransactionError("Premium RPC failed in premium-only mode".to_string())
            );
        }

        // Normal mode: Use premium RPC URL first
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

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_START",
                &format!("🚀 Starting sign_and_send_transaction process with {} byte transaction", swap_transaction_base64.len())
            );
        }

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

        // If premium RPC only mode is active, use only premium RPC
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - sending signed transaction to premium RPC only");
            }

            if is_debug_transactions_enabled() {
                log(
                    LogTag::Rpc,
                    "TX_DEBUG_SEND_PREMIUM_ONLY", 
                    &format!("🎯 Sending transaction to premium RPC only: {}", &configs.rpc_url_premium)
                );
            }

            match
                client
                    .post(&configs.rpc_url_premium)
                    .header("Content-Type", "application/json")
                    .json(&rpc_payload)
                    .send().await
            {
                Ok(response) => {
                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "TX_DEBUG_RESPONSE_STATUS",
                            &format!("📡 Premium RPC response status: {}", response.status())
                        );
                    }

                    if response.status().is_success() {
                        if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                            if is_debug_transactions_enabled() {
                                log(
                                    LogTag::Rpc,
                                    "TX_DEBUG_RESPONSE_BODY",
                                    &format!("📄 Premium RPC response: {}", serde_json::to_string_pretty(&rpc_response).unwrap_or_else(|_| "Failed to serialize".to_string()))
                                );
                            }

                            if let Some(result) = rpc_response.get("result") {
                                if let Some(tx_sig) = result.as_str() {
                                    log(
                                        LogTag::Rpc,
                                        "SUCCESS",
                                        &format!("Signed transaction sent successfully via premium RPC only: {}", tx_sig)
                                    );
                                    self.record_call_for_url(&configs.rpc_url_premium, "send_transaction");
                                    
                                    if is_debug_transactions_enabled() {
                                        log(
                                            LogTag::Rpc,
                                            "TX_DEBUG_SUCCESS",
                                            &format!("🎉 Transaction successfully submitted! Signature: {}", tx_sig)
                                        );
                                    }
                                    
                                    return Ok(tx_sig.to_string());
                                }
                            }

                            if let Some(error) = rpc_response.get("error") {
                                let error_msg = error
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("Unknown RPC error");
                                
                                if is_debug_transactions_enabled() {
                                    log(
                                        LogTag::Rpc,
                                        "TX_DEBUG_ERROR",
                                        &format!("❌ Premium RPC returned error: {}", error_msg)
                                    );
                                    if let Some(error_code) = error.get("code") {
                                        log(
                                            LogTag::Rpc,
                                            "TX_DEBUG_ERROR_CODE",
                                            &format!("📍 Error code: {}", error_code)
                                        );
                                    }
                                }
                                
                                return Err(
                                    SwapError::TransactionError(format!("Premium RPC error: {}", error_msg))
                                );
                            }
                        } else {
                            if is_debug_transactions_enabled() {
                                log(
                                    LogTag::Rpc,
                                    "TX_DEBUG_PARSE_ERROR",
                                    "❌ Failed to parse premium RPC response as JSON"
                                );
                            }
                        }
                    } else {
                        if is_debug_transactions_enabled() {
                            log(
                                LogTag::Rpc,
                                "TX_DEBUG_HTTP_ERROR",
                                &format!("❌ Premium RPC returned HTTP error: {}", response.status())
                            );
                        }
                    }
                }
                Err(e) => {
                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "TX_DEBUG_NETWORK_ERROR",
                            &format!("❌ Network error sending to premium RPC: {}", e)
                        );
                    }
                    return Err(SwapError::NetworkError(e));
                }
            }

            return Err(SwapError::TransactionError("Premium RPC failed in premium-only mode".to_string()));
        }

        // Normal mode: Use premium RPC URL first
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

        // If premium RPC only mode is active, use only premium RPC
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - getting transaction details from premium RPC only");
            }

            if let Some(premium_url) = &self.premium_url {
                self.record_call_for_url(premium_url, "getTransaction");
                
                match client
                    .post(premium_url)
                    .header("Content-Type", "application/json")
                    .json(&rpc_payload)
                    .send().await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                                if let Some(error) = rpc_response.get("error") {
                                    return Err(SwapError::TransactionError(format!("Premium RPC error: {:?}", error)));
                                }

                                if let Some(result) = rpc_response.get("result") {
                                    if result.is_null() {
                                        return Err(SwapError::TransactionError("Transaction not found or not confirmed yet".to_string()));
                                    }

                                    let transaction_details: TransactionDetails = serde_json::from_value(result.clone())
                                        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse transaction details: {}", e)))?;

                                    return Ok(transaction_details);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(SwapError::NetworkError(e));
                    }
                }
            }

            return Err(SwapError::TransactionError("Premium RPC failed in premium-only mode".to_string()));
        }

        // Record call in stats for normal mode
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_call(&self.rpc_url, "getTransaction");
        }

        let mut should_fallback = false;

        // Normal mode: Try main RPC first
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
                            let error_str = error.to_string();
                            // Check if the JSON error response contains a 429 rate limit error
                            if Self::is_rate_limit_error(&error_str) {
                                should_fallback = true;
                                self.record_429_error(Some(&self.rpc_url)); // Record 429 for adaptive backoff
                                log(
                                    LogTag::Rpc,
                                    "WARNING",
                                    "Main RPC returned 429 rate limit for transaction details, falling back to premium"
                                );
                            } else {
                                log(
                                    LogTag::Rpc,
                                    "ERROR",
                                    &format!("RPC error getting transaction: {:?}", error)
                                );
                                return Err(
                                    SwapError::TransactionError(format!("RPC error: {:?}", error))
                                );
                            }
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

    /// Premium-only variant of get_transaction_details, bypassing main RPC and fallbacks
    pub async fn get_transaction_details_premium(
        &self,
        transaction_signature: &str
    ) -> Result<TransactionDetails, SwapError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                transaction_signature,
                { "encoding": "json", "maxSupportedTransactionVersion": 0 }
            ]
        });

        let client = reqwest::Client::new();
        if let Some(premium_url) = &self.premium_url {
            self.record_call_for_url(premium_url, "getTransaction");
            let response = client
                .post(premium_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send()
                .await
                .map_err(SwapError::NetworkError)?;

            if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                if let Some(error) = rpc_response.get("error") {
                    return Err(SwapError::TransactionError(format!("Premium RPC error: {:?}", error)));
                }
                if let Some(result) = rpc_response.get("result") {
                    if result.is_null() {
                        return Err(SwapError::TransactionError("Transaction not found or not confirmed yet".to_string()));
                    }
                    let transaction_details: TransactionDetails = serde_json::from_value(result.clone())
                        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse transaction details: {}", e)))?;
                    return Ok(transaction_details);
                }
            }
            return Err(SwapError::TransactionError("Invalid premium RPC response".to_string()));
        }
        Err(SwapError::TransactionError("Premium RPC URL not configured".to_string()))
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

    /// Helper method to get signature status using getSignatureStatuses  
    async fn get_signature_status(&self, signature: &str) -> Result<Option<SignatureStatusData>, SwapError> {
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_START",
                &format!("🔍 Checking signature status for {} using RPC client", &signature[..8])
            );
        }
        
        log(
            LogTag::Rpc,
            "STATUS_API_CALL_START",
            &format!("🌐 Making getSignatureStatuses API call for {}", &signature[..8])
        );

        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSignatureStatuses",
            "params": [
                [signature],
                {
                    "searchTransactionHistory": true
                }
            ]
        });

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_PAYLOAD",
                &format!("📤 Request payload for {}: {}", &signature[..8], serde_json::to_string(&rpc_payload).unwrap_or_else(|_| "Failed to serialize".to_string()))
            );
        }

        let client = reqwest::Client::new();
        let rpc_url = if is_premium_rpc_only() {
            &self.premium_url.as_ref().ok_or(SwapError::ConfigError("No premium RPC available".to_string()))?
        } else {
            &self.rpc_url
        };

        log(
            LogTag::Rpc,
            "STATUS_API_URL",
            &format!("📡 Using RPC endpoint: {} for signature {}", rpc_url, &signature[..8])
        );

        let response = client
            .post(rpc_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                log(
                    LogTag::Rpc,
                    "STATUS_API_NETWORK_ERROR",
                    &format!("🔌 Network error in getSignatureStatuses for {}: {}", &signature[..8], e)
                );
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Rpc,
                        "STATUS_DEBUG_NETWORK_ERROR_DETAIL",
                        &format!("❌ Detailed network error for {}: {}", &signature[..8], e)
                    );
                }
                SwapError::NetworkError(e)
            })?;

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_HTTP_STATUS",
                &format!("📡 HTTP status for signature status check {}: {}", &signature[..8], response.status())
            );
        }

        if !response.status().is_success() {
            log(
                LogTag::Rpc,
                "STATUS_API_HTTP_ERROR",
                &format!("📉 HTTP error in getSignatureStatuses for {}: {}", &signature[..8], response.status())
            );
            return Err(SwapError::ApiError(format!("RPC error: {}", response.status())));
        }

        log(
            LogTag::Rpc,
            "STATUS_API_RESPONSE_OK",
            &format!("✅ Received HTTP 200 response from getSignatureStatuses for {}", &signature[..8])
        );

        let response_text = response.text().await.map_err(|e| {
            if is_debug_transactions_enabled() {
                log(
                    LogTag::Rpc,
                    "STATUS_DEBUG_TEXT_ERROR",
                    &format!("❌ Failed to get response text for {}: {}", &signature[..8], e)
                );
            }
            SwapError::NetworkError(e)
        })?;

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_RAW_RESPONSE",
                &format!("📄 Raw response for {}: {}", &signature[..8], &response_text)
            );
        }

        let rpc_response: SignatureStatusResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                log(
                    LogTag::Rpc,
                    "STATUS_API_PARSE_ERROR",
                    &format!("🔍 Failed to parse getSignatureStatuses response for {}: {}", &signature[..8], e)
                );
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Rpc,
                        "STATUS_DEBUG_PARSE_ERROR_DETAIL",
                        &format!("❌ Parse error detail for {}: Response was: {}", &signature[..8], &response_text)
                    );
                }
                SwapError::InvalidResponse(format!("Failed to parse signature status: {}", e))
            })?;

        let result = rpc_response.result.value.into_iter().next().flatten();
        
        log(
            LogTag::Rpc,
            "STATUS_API_RESULT",
            &format!(
                "📊 getSignatureStatuses result for {}: {}",
                &signature[..8], 
                result.as_ref().map(|r| format!("confirmation_status={:?}, err={:?}", r.confirmation_status, r.err))
                    .unwrap_or_else(|| "null".to_string())
            )
        );

        if is_debug_transactions_enabled() {
            if result.is_none() {
                log(
                    LogTag::Rpc,
                    "STATUS_DEBUG_NULL_RESULT",
                    &format!("⚠️ Signature {} returned null status - transaction may not be visible on network yet", &signature[..8])
                );
            } else if let Some(ref status) = result {
                log(
                    LogTag::Rpc,
                    "STATUS_DEBUG_FOUND",
                    &format!("✅ Found status for {}: confirmation={:?}, error={:?}", 
                        &signature[..8], 
                        status.confirmation_status, 
                        status.err
                    )
                );
            }
        }

        Ok(result)
    }

    /// Wait for a freshly submitted transaction signature to propagate so that
    /// getSignatureStatuses returns a non-null entry (even if still pending).
    /// This avoids immediately treating a just-sent transaction as failed when
    /// RPC propagation is slightly delayed.
    /// Returns Ok(true) if a status record (any) appears within timeout, Ok(false) if not.
    pub async fn wait_for_signature_propagation(&self, signature: &str) -> Result<bool, SwapError> {
        // Extended timing for better reliability: 4 attempts at t=2,7,12,17 seconds 
        const ATTEMPTS: u32 = 4;
        const FIRST_DELAY_SECS: u64 = 2; // Initial delay before first check
        const SLEEP_SECS: u64 = 5;
        
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "PROPAGATION_DEBUG_START",
                &format!("🚀 Starting propagation wait for signature {} with {} attempts starting after {}s delay", &signature[..8], ATTEMPTS, FIRST_DELAY_SECS)
            );
        }
        
        log(
            LogTag::Rpc,
            "STATUS_PROPAGATION_WAIT_START",
            &format!(
                "🚦 Propagation wait start for {} ({} attempts with {}s initial delay)",
                &signature[..8], ATTEMPTS, FIRST_DELAY_SECS
            )
        );
        
        // Initial delay to allow transaction to propagate
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "PROPAGATION_DEBUG_INITIAL_DELAY",
                &format!("⏳ Waiting {}s before first propagation check for {}", FIRST_DELAY_SECS, &signature[..8])
            );
        }
        tokio::time::sleep(Duration::from_secs(FIRST_DELAY_SECS)).await;
        
        let start = Instant::now();
        for attempt in 1..=ATTEMPTS {
            if is_debug_transactions_enabled() {
                log(
                    LogTag::Rpc,
                    "PROPAGATION_DEBUG_ATTEMPT_START",
                    &format!("📋 Starting attempt {}/{} for signature {}", attempt, ATTEMPTS, &signature[..8])
                );
            }
            
            match self.get_signature_status(signature).await {
                Ok(Some(status)) => {
                    log(
                        LogTag::Rpc,
                        "STATUS_PROPAGATION_SUCCESS",
                        &format!(
                            "🟢 Propagated {} after {:.2}s on attempt {} (confirmation={:?} err={:?})",
                            &signature[..8],
                            start.elapsed().as_secs_f64(),
                            attempt,
                            status.confirmation_status,
                            status.err
                        )
                    );
                    
                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "PROPAGATION_DEBUG_SUCCESS_DETAIL",
                            &format!("✅ Propagation successful for {}: Found status after {:.2}s", &signature[..8], start.elapsed().as_secs_f64())
                        );
                    }
                    
                    return Ok(true);
                }
                Ok(None) => {
                    log(
                        LogTag::Rpc,
                        "STATUS_PROPAGATION_ATTEMPT",
                        &format!(
                            "⏳ Attempt {}/{}: signature {} not yet visible (elapsed {:.2}s)",
                            attempt,
                            ATTEMPTS,
                            &signature[..8],
                            start.elapsed().as_secs_f64()
                        )
                    );
                    
                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "PROPAGATION_DEBUG_NULL_ATTEMPT",
                            &format!("⏳ Attempt {}/{} returned null for {} - trying again in {}s", attempt, ATTEMPTS, &signature[..8], SLEEP_SECS)
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "STATUS_PROPAGATION_ERROR",
                        &format!(
                            "⚠️ Attempt {}/{} error checking {}: {}",
                            attempt,
                            ATTEMPTS,
                            &signature[..8],
                            e
                        )
                    );
                    
                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "PROPAGATION_DEBUG_ERROR_DETAIL",
                            &format!("❌ Detailed error on attempt {}/{} for {}: {}", attempt, ATTEMPTS, &signature[..8], e)
                        );
                    }
                }
            }
            
            if attempt < ATTEMPTS { 
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Rpc,
                        "PROPAGATION_DEBUG_SLEEP",
                        &format!("😴 Sleeping {}s before next attempt for {}", SLEEP_SECS, &signature[..8])
                    );
                }
                tokio::time::sleep(Duration::from_secs(SLEEP_SECS)).await; 
            }
        }
        
        log(
            LogTag::Rpc,
            "STATUS_PROPAGATION_FAILED",
            &format!(
                "❌ Propagation failed for {} after {} attempts (~{}s)",
                &signature[..8], ATTEMPTS, start.elapsed().as_secs_f64() as u64
            )
        );
        
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "PROPAGATION_DEBUG_FAILED",
                &format!("❌ Transaction {} failed to propagate - likely dropped by network", &signature[..8])
            );
        }
        
        Ok(false)
    }

    /// Get wallet signatures using main RPC (lightweight operation)
    /// This is optimized for checking how many new transactions exist without heavy data transfer
    pub async fn get_wallet_signatures_main_rpc(
        &self,
        wallet_pubkey: &Pubkey,
        limit: usize,
        before: Option<&str>
    ) -> Result<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>, SwapError> {
        let config = solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
            before: before.and_then(|s| solana_sdk::signature::Signature::from_str(s).ok()),
            until: None,
            limit: Some(limit),
            commitment: Some(CommitmentConfig::confirmed()),
        };

        // If premium RPC only mode is active, use premium RPC even for signature fetching
        if is_premium_rpc_only() {
            if is_debug_rpc_enabled(){
                log(LogTag::Rpc, "PREMIUM_ONLY", "FORCE_PREMIUM_RPC_ONLY is active - fetching signatures from premium RPC only");
            }

            if let Some(premium_url) = &self.premium_url {
                let premium_client = SolanaRpcClient::new_with_commitment(
                    premium_url.clone(),
                    CommitmentConfig::confirmed()
                );
                
                self.record_call_for_url(premium_url, "get_signatures_for_address");
                
                let signatures = premium_client
                    .get_signatures_for_address_with_config(wallet_pubkey, config)
                    .map_err(|e| SwapError::ApiError(format!("Failed to get signatures from premium RPC: {}", e)))?;
                
                log(LogTag::Rpc, "SUCCESS", &format!("Retrieved {} signatures from premium RPC (premium-only mode)", signatures.len()));
                return Ok(signatures);
            } else {
                return Err(SwapError::ConfigError("Premium RPC URL not configured but premium-only mode is active".to_string()));
            }
        }

        // Normal mode: Use main RPC for this lightweight operation
        let main_client = self.create_main_client();
        
        // Apply rate limiting for main RPC
        self.wait_for_rate_limit().await;
        self.record_call_for_url(&self.rpc_url, "get_signatures_for_address");
        
        log(LogTag::Rpc, "MAIN", &format!("Fetching {} signatures using main RPC", limit));
        
        let signatures = main_client
            .get_signatures_for_address_with_config(wallet_pubkey, config)
            .map_err(|e| SwapError::ApiError(format!("Failed to get signatures from main RPC: {}", e)))?;
        
        log(LogTag::Rpc, "SUCCESS", &format!("Retrieved {} signatures from main RPC", signatures.len()));
        Ok(signatures)
    }

    /// Get transaction details using premium RPC (data-intensive operation)
    /// This is optimized for fetching full transaction data with minimal rate limiting
    pub async fn get_transaction_details_premium_rpc(
        &self,
        transaction_signature: &str
    ) -> Result<solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta, SwapError> {
        // Use premium RPC for this data-intensive operation
        let premium_client = if let Some(client) = self.create_premium_client() {
            client
        } else {
            // Fallback to main client if no premium available
            log(LogTag::Rpc, "WARNING", "No premium RPC available, using main RPC for transaction details");
            self.client.clone()
        };
        
        // No rate limiting for premium RPC, but record the call
        if self.premium_url.is_some() {
            self.record_call_for_url(self.premium_url.as_ref().unwrap(), "get_transaction");
        } else {
            self.wait_for_rate_limit().await;
            self.record_call("get_transaction");
        }
        
        let signature = solana_sdk::signature::Signature::from_str(transaction_signature)
            .map_err(|e| SwapError::ParseError(format!("Invalid signature: {}", e)))?;
        
        log(LogTag::Rpc, "PREMIUM", &format!("Fetching transaction details for {} using premium RPC", &transaction_signature[..8]));
        
        let transaction = premium_client
            .get_transaction_with_config(
                &signature,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                }
            )
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("Transaction not found") || 
                   error_msg.contains("invalid type: null, expected struct EncodedConfirmedTransactionWithStatusMeta") {
                    SwapError::ApiError(format!("Transaction {} not found or no longer available", transaction_signature))
                } else {
                    SwapError::ApiError(format!("Failed to get transaction from premium RPC: {}", e))
                }
            })?;
        
        log(LogTag::Rpc, "SUCCESS", &format!("Retrieved transaction details for {} from premium RPC", &transaction_signature[..8]));
        Ok(transaction)
    }

    /// Batch get transaction details using premium RPC (optimized for multiple transactions)
    /// This uses premium RPC to minimize rate limiting when fetching multiple transactions
    pub async fn batch_get_transaction_details_premium_rpc(
        &self,
        signatures: &[String]
    ) -> Result<Vec<(String, solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta)>, SwapError> {
        if signatures.is_empty() {
            return Ok(Vec::new());
        }
        // Limit concurrent RPC calls to avoid overwhelming the endpoint
        use tokio::sync::Semaphore;
        use std::sync::Arc;
        const MAX_CONCURRENT_TX_FETCHES: usize = 25;
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TX_FETCHES));

        // Use premium RPC for batch operations
        let premium_client = if let Some(client) = self.create_premium_client() {
            client
        } else {
            log(LogTag::Rpc, "WARNING", "No premium RPC available, using main RPC for batch transaction details");
            self.client.clone()
        };
        
        log(LogTag::Rpc, "PREMIUM", &format!("Batch fetching {} transaction details using premium RPC", signatures.len()));
        
        // Create futures for parallel processing with concurrency cap
        let mut futures = Vec::with_capacity(signatures.len());
        
        for signature_str in signatures {
            let signature_str = signature_str.clone();
            let premium_client = premium_client.clone();
            let permit = semaphore.clone().acquire_owned();
            
            // Record call for each transaction
            if self.premium_url.is_some() {
                self.record_call_for_url(self.premium_url.as_ref().unwrap(), "get_transaction");
            } else {
                self.record_call("get_transaction");
            }
            
            let future = async move {
                // Acquire permit to respect concurrency limit
                // If semaphore is closed or acquisition fails, skip this item gracefully
                let _permit = match permit.await {
                    Ok(p) => p,
                    Err(_) => {
                        log(LogTag::Rpc, "ERROR", "Semaphore closed while fetching transactions");
                        return None;
                    }
                };
                if let Ok(signature) = solana_sdk::signature::Signature::from_str(&signature_str) {
                    match premium_client.get_transaction_with_config(
                        &signature,
                        solana_client::rpc_config::RpcTransactionConfig {
                            encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                            commitment: Some(CommitmentConfig::confirmed()),
                            max_supported_transaction_version: Some(0),
                        }
                    ) {
                        Ok(tx) => Some((signature_str, tx)),
                        Err(e) => {
                            let error_msg = e.to_string();
                            if error_msg.contains("Transaction not found") || error_msg.contains("null, expected struct") {
                                log(LogTag::Rpc, "SKIP", &format!("Transaction {} not yet available", &signature_str[..8]));
                            } else {
                                log(LogTag::Rpc, "ERROR", &format!("Failed to fetch transaction {}: {}", &signature_str[..8], e));
                            }
                            None
                        }
                    }
                } else {
                    None
                }
            };
            
            futures.push(future);
        }
        
        // Execute all futures in parallel
        let results_futures = futures::future::join_all(futures).await;
        
        // Collect successful results
        let mut results = Vec::new();
        let mut successful_fetches = 0;
        
        for result in results_futures {
            if let Some((signature, tx)) = result {
                results.push((signature, tx));
                successful_fetches += 1;
            }
        }
        
        log(LogTag::Rpc, "SUCCESS", &format!("Successfully fetched {}/{} transactions from premium RPC", successful_fetches, signatures.len()));
        Ok(results)
    }

    /// Get transaction details using finalized commitment level for critical operations
    pub async fn get_transaction_details_finalized_rpc(
        &self,
        signature: &str,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, SwapError> {
        self.wait_for_rate_limit().await;
        self.record_call("get_transaction_finalized");

        let client = self.create_premium_client().unwrap_or_else(|| {
            log(LogTag::Rpc, "WARNING", "No premium RPC available for finalized transaction, using main RPC");
            self.client.clone()
        });

        let signature = Signature::from_str(signature)
            .map_err(|e| SwapError::InvalidResponse(format!("Invalid signature: {}", e)))?;

        tokio::task::spawn_blocking({
            let client = client.clone();
            move || {
                client
                    .get_transaction_with_config(
                        &signature,
                        solana_client::rpc_config::RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::Json),
                            commitment: Some(CommitmentConfig::finalized()),
                            max_supported_transaction_version: Some(0),
                        }
                    )
                    .map_err(|e| {
                        let error_msg = e.to_string();
                        if error_msg.contains("not found") {
                            SwapError::TransactionError(
                                "Transaction not found or not finalized yet".to_string()
                            )
                        } else {
                            SwapError::TransactionError(format!("RPC error: {}", error_msg))
                        }
                    })
            }
        })
        .await
        .map_err(|e| SwapError::TransactionError(format!("Task error: {}", e)))?
    }
    
    /// Batch get transaction details using finalized commitment level for transaction verification
    /// This ensures we get the final, immutable state of transactions for important operations
    pub async fn batch_get_transaction_details_finalized_rpc(
        &self,
        signatures: &[String]
    ) -> Result<Vec<(String, solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta)>, SwapError> {
        if signatures.is_empty() {
            return Ok(Vec::new());
        }
        
        // Use premium RPC for finalized operations to ensure reliability
        let premium_client = if let Some(client) = self.create_premium_client() {
            client
        } else {
            log(LogTag::Rpc, "WARNING", "No premium RPC available for finalized transactions, using main RPC");
            self.client.clone()
        };
        
        log(LogTag::Rpc, "FINALIZED", &format!("Batch fetching {} transaction details with finalized commitment", signatures.len()));
        
        let mut results = Vec::new();
        let mut successful_fetches = 0;
        let mut not_finalized_count = 0;
        
        for signature_str in signatures {
            // Record call for each transaction
            if self.premium_url.is_some() {
                self.record_call_for_url(self.premium_url.as_ref().unwrap(), "get_transaction_finalized");
            } else {
                self.wait_for_rate_limit().await;
                self.record_call("get_transaction_finalized");
            }
            
            if let Ok(signature) = solana_sdk::signature::Signature::from_str(signature_str) {
                match premium_client.get_transaction_with_config(
                    &signature,
                    solana_client::rpc_config::RpcTransactionConfig {
                        encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                        commitment: Some(CommitmentConfig::finalized()), // Use finalized commitment
                        max_supported_transaction_version: Some(0),
                    }
                ) {
                    Ok(tx) => {
                        results.push((signature_str.clone(), tx));
                        successful_fetches += 1;
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        if error_msg.contains("Transaction not found") || error_msg.contains("null, expected struct") {
                            not_finalized_count += 1;
                            if is_debug_transactions_enabled() {
                                log(LogTag::Rpc, "PENDING", &format!("Transaction {} not yet finalized", &signature_str[..8]));
                            }
                        } else {
                            log(LogTag::Rpc, "ERROR", &format!("Failed to fetch finalized transaction {}: {}", &signature_str[..8], e));
                        }
                    }
                }
            }
            
            // Small delay between requests to avoid overwhelming RPC
            if signatures.len() > 3 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        if not_finalized_count > 0 {
            log(LogTag::Rpc, "INFO", &format!("{}/{} transactions not yet finalized, will retry later", not_finalized_count, signatures.len()));
        }
        
        log(LogTag::Rpc, "SUCCESS", &format!("Successfully fetched {}/{} finalized transactions", successful_fetches, signatures.len()));
        Ok(results)
    }
    
    /// Get single transaction details using processed commitment level for immediate feedback
    /// This provides the fastest possible response for transaction status checking
    pub async fn get_transaction_details_processed_rpc(
        &self,
        signature: &str
    ) -> Result<solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta, SwapError> {
        // Record call
        self.wait_for_rate_limit().await;
        self.record_call("get_transaction_processed");
        
        if let Ok(signature_obj) = solana_sdk::signature::Signature::from_str(signature) {
            // Use main RPC for processed transactions (faster response, lower latency)
            match self.client.get_transaction_with_config(
                &signature_obj,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::processed()), // Use processed commitment for speed
                    max_supported_transaction_version: Some(0),
                }
            ) {
                Ok(tx) => {
                    log(LogTag::Rpc, "PROCESSED", &format!("Retrieved transaction {} with processed commitment", &signature[..8]));
                    Ok(tx)
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("Transaction not found") || error_msg.contains("null, expected struct") {
                        log(LogTag::Rpc, "NOT_PROCESSED", &format!("Transaction {} not yet processed", &signature[..8]));
                    } else {
                        log(LogTag::Rpc, "ERROR", &format!("Failed to fetch processed transaction {}: {}", &signature[..8], e));
                    }
                    Err(SwapError::TransactionError(format!("Failed to fetch processed transaction: {}", e)))
                }
            }
        } else {
            Err(SwapError::InvalidAmount(format!("Invalid signature format: {}", signature)))
        }
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
