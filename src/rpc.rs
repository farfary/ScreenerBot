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

/// WARNING: If premium RPC fails when this is enabled, operations will fail
/// instead of falling back to other endpoints.
const FORCE_PREMIUM_RPC_ONLY: bool = true;

/// RPC RATE LIMITING CONFIGURATION
/// Maximum calls per second for main RPC (premium RPC has no limits)
const MAX_RPC_CALLS_PER_SECOND: u64 = 20;

use crate::constants::LAMPORTS_PER_SOL;
use crate::errors::blockchain::CommitmentLevel;
use crate::errors::{parse_solana_error, BlockchainError, ScreenerBotError};
use crate::errors::{ConfigurationError, DataError, NetworkError, RpcProviderError};
use crate::global::{
    is_debug_rpc_enabled, is_debug_transactions_enabled, is_debug_wallet_enabled, RPC_STATS,
};
use crate::logger::{log, LogTag};
use crate::utils::lamports_to_sol;
use base64::Engine as _;
use bincode;
use bs58;
use chrono::{DateTime, Utc};
use futures;
use once_cell::sync::Lazy;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;
use solana_client::rpc_client::RpcClient as SolanaRpcClient;
use solana_sdk::{
    account::Account,
    client::SyncClient,
    commitment_config::CommitmentConfig,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::{Arc as StdArc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
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
    pub block_time: Option<i64>,
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
    #[serde(rename = "computeUnitsConsumed")]
    pub compute_units_consumed: Option<u64>,
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

/// Response structure for getProgramAccountsV2 with pagination support
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedAccountsResponse {
    /// The accounts returned in this page
    pub accounts: Vec<serde_json::Value>,
    /// Pagination key for next page (None if this is the last page)
    pub pagination_key: Option<String>,
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

// Short-lived cache for block height to avoid frequent getBlockHeight RPC calls.
struct BlockHeightCache {
    height: Option<u64>,
    fetched_at: Option<Instant>,
}

static BLOCK_HEIGHT_CACHE: Lazy<AsyncMutex<BlockHeightCache>> = Lazy::new(|| {
    AsyncMutex::new(BlockHeightCache {
        height: None,
        fetched_at: None,
    })
});

// TTL for cached block height (seconds)
const BLOCK_HEIGHT_CACHE_TTL_SECS: u64 = 90;

/// Check if premium RPC only mode is active
fn is_premium_rpc_only() -> bool {
    FORCE_PREMIUM_RPC_ONLY
}

/// Get current ATA rent amount from chain with 10-second cache
pub async fn get_ata_rent_lamports() -> Result<u64, ScreenerBotError> {
    // Check cache first with safe lock handling
    {
        let cache = match ATA_RENT_CACHE.try_lock() {
            Ok(cache) => cache,
            Err(_) => {
                // If we can't get the cache lock, fall back to default value
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "ATA rent cache lock contention - using default ATA rent",
                    );
                }
                return Ok(2039280); // Default ATA rent: 0.00203928 SOL
            }
        };
        if let Some(ref info) = *cache {
            if info.cached_at.elapsed() < Duration::from_secs(10) {
                return Ok(info.rent_lamports);
            }
        }
    }

    // Cache miss or expired, fetch from chain using round-robin RPC
    let rpc_client = get_rpc_client();

    let rpc_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getMinimumBalanceForRentExemption",
        "params": [165] // ATA account size: 165 bytes for standard token account
    });

    let client = reqwest::Client::new();
    let rpc_urls = crate::config::with_config(|cfg| cfg.rpc.urls.clone());

    if rpc_urls.is_empty() {
        return Err(ScreenerBotError::Configuration(
            crate::errors::ConfigurationError::Generic {
                message: "No RPC URLs configured".to_string(),
            },
        ));
    }

    // Use round-robin RPC rotation - get next URL from client
    let current_url = rpc_client.rotate_to_next_url();

    if is_debug_rpc_enabled() {
        log(LogTag::Rpc, "ATA_RENT", "Fetching ATA rent from RPC");
    }

    // Apply rate limiting
    rpc_client.wait_for_rate_limit().await;

    // Make the RPC call
    let response = client
        .post(&current_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send()
        .await;

    match response {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(rpc_response) => {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(rent_lamports) = result.as_u64() {
                                // Record successful call
                                rpc_client.record_success(Some(&current_url));

                                // Update cache with safe lock handling
                                {
                                    if let Ok(mut cache) = ATA_RENT_CACHE.try_lock() {
                                        *cache = Some(AtaRentInfo {
                                            rent_lamports,
                                            cached_at: Instant::now(),
                                        });
                                    } else {
                                        if is_debug_rpc_enabled() {
                                            log(
                                                LogTag::Rpc,
                                                "WARN",
                                                "Failed to update ATA rent cache - lock contention",
                                            );
                                        }
                                    }
                                }

                                if is_debug_rpc_enabled() {
                                    log(
                                        LogTag::Rpc,
                                        "ATA_RENT",
                                        &format!(
                                            "Retrieved ATA rent from RPC: {} lamports ({:.9} SOL)",
                                            rent_lamports,
                                            lamports_to_sol(rent_lamports)
                                        ),
                                    );
                                }

                                return Ok(rent_lamports);
                            }
                        }

                        if is_debug_rpc_enabled() {
                            log(
                                LogTag::Rpc,
                                "WARN",
                                "RPC response missing result for ATA rent",
                            );
                        }
                    }
                    Err(e) => {
                        if is_debug_rpc_enabled() {
                            log(
                                LogTag::Rpc,
                                "WARN",
                                &format!("Failed to parse ATA rent RPC response: {}", e),
                            );
                        }
                    }
                }
            } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // Record 429 error for adaptive rate limiting
                rpc_client.record_429_error(Some(&current_url));
                if is_debug_rpc_enabled() {
                    log(LogTag::Rpc, "WARN", "Rate limited on RPC");
                }
            } else {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("RPC error status: {}", response.status()),
                    );
                }
            }
        }
        Err(e) => {
            if is_debug_rpc_enabled() {
                log(
                    LogTag::Rpc,
                    "WARN",
                    &format!("Failed to connect to RPC: {}", e),
                );
            }
        }
    }

    // If the current RPC call failed, return fallback value
    // Next call will automatically use the next RPC in round-robin rotation
    const FALLBACK_ATA_RENT: u64 = 2_039_280;
    log(
        LogTag::Rpc,
        "ATA_RENT_FALLBACK",
        &format!(
            "RPC call failed, using fallback ATA rent: {} lamports",
            FALLBACK_ATA_RENT
        ),
    );

    Ok(FALLBACK_ATA_RENT)
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
pub fn get_websocket_url() -> Result<String, ScreenerBotError> {
    let rpc_urls = crate::config::with_config(|cfg| cfg.rpc.urls.clone());

    // Use the first RPC URL from the list for websocket derivation
    let http = rpc_urls.get(0).ok_or_else(|| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::Generic {
            message: "No RPC URLs configured".to_string(),
        })
    })?;

    let parsed = Url::parse(http).map_err(|e| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidUrl {
            url: http.clone(),
            error: e.to_string(),
        })
    })?;
    let ws_scheme = match parsed.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            if is_debug_rpc_enabled() {
                log(
                    LogTag::Rpc,
                    "WS_URL_SCHEME_WARN",
                    &format!("Unknown scheme '{}', defaulting to wss", other),
                );
            }
            "wss"
        }
    };
    let mut ws_url = parsed.clone();
    ws_url.set_scheme(ws_scheme).map_err(|_| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidUrl {
            url: http.clone(),
            error: "Failed to set WS scheme".to_string(),
        })
    })?;
    Ok(ws_url.to_string())
}

/// Derive a websocket URL from a specific HTTP RPC URL
/// Examples:
///  - https://premium.rpc.provider -> wss://premium.rpc.provider
///  - http://localhost:8899 -> ws://localhost:8899
pub fn get_websocket_url_from_http(http_url: &str) -> Result<String, ScreenerBotError> {
    let parsed = Url::parse(http_url).map_err(|e| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidUrl {
            url: http_url.to_string(),
            error: e.to_string(),
        })
    })?;
    let ws_scheme = match parsed.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            if is_debug_rpc_enabled() {
                log(
                    LogTag::Rpc,
                    "WS_URL_SCHEME_WARN",
                    &format!("Unknown scheme '{}', defaulting to wss", other),
                );
            }
            "wss"
        }
    };
    let mut ws_url = parsed.clone();
    ws_url.set_scheme(ws_scheme).map_err(|_| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidUrl {
            url: http_url.to_string(),
            error: "Failed to set WS scheme".to_string(),
        })
    })?;
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

/// Converts SOL amount to lamports (1 SOL = 1,000,000,000 lamports)
pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * (LAMPORTS_PER_SOL as f64)) as u64
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
    /// Failed calls per URL
    #[serde(default)]
    pub errors_per_url: HashMap<String, u64>,
    /// Failed calls per method
    #[serde(default)]
    pub errors_per_method: HashMap<String, u64>,
    /// Response time tracking per URL (sum of milliseconds)
    #[serde(default)]
    pub response_time_sum_ms_per_url: HashMap<String, u64>,
    /// Response time tracking per method (sum of milliseconds)
    #[serde(default)]
    pub response_time_sum_ms_per_method: HashMap<String, u64>,
    /// Count of timed calls per URL (for average calculation)
    #[serde(default)]
    pub timed_calls_per_url: HashMap<String, u64>,
    /// Count of timed calls per method (for average calculation)
    #[serde(default)]
    pub timed_calls_per_method: HashMap<String, u64>,
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
            errors_per_url: HashMap::new(),
            errors_per_method: HashMap::new(),
            response_time_sum_ms_per_url: HashMap::new(),
            response_time_sum_ms_per_method: HashMap::new(),
            timed_calls_per_url: HashMap::new(),
            timed_calls_per_method: HashMap::new(),
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

    /// Record a failed call to an RPC method on a specific URL
    pub fn record_error(&mut self, url: &str, method: &str) {
        *self.errors_per_url.entry(url.to_string()).or_insert(0) += 1;
        *self
            .errors_per_method
            .entry(method.to_string())
            .or_insert(0) += 1;
    }

    /// Record response time for a call
    pub fn record_response_time(&mut self, url: &str, method: &str, duration_ms: u64) {
        *self
            .response_time_sum_ms_per_url
            .entry(url.to_string())
            .or_insert(0) += duration_ms;
        *self
            .response_time_sum_ms_per_method
            .entry(method.to_string())
            .or_insert(0) += duration_ms;
        *self.timed_calls_per_url.entry(url.to_string()).or_insert(0) += 1;
        *self
            .timed_calls_per_method
            .entry(method.to_string())
            .or_insert(0) += 1;
    }

    /// Get total calls across all URLs
    pub fn total_calls(&self) -> u64 {
        self.calls_per_url.values().sum()
    }

    /// Get total errors across all URLs
    pub fn total_errors(&self) -> u64 {
        self.errors_per_url.values().sum()
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f32 {
        let total = self.total_calls();
        if total == 0 {
            return 100.0;
        }
        let errors = self.total_errors();
        (((total - errors) as f32) / (total as f32)) * 100.0
    }

    /// Get average response time in milliseconds for a URL
    pub fn average_response_time_ms(&self, url: &str) -> f64 {
        let sum = self
            .response_time_sum_ms_per_url
            .get(url)
            .copied()
            .unwrap_or(0);
        let count = self.timed_calls_per_url.get(url).copied().unwrap_or(0);
        if count > 0 {
            (sum as f64) / (count as f64)
        } else {
            0.0
        }
    }

    /// Get average response time in milliseconds across all URLs
    pub fn average_response_time_ms_global(&self) -> f64 {
        let total_sum: u64 = self.response_time_sum_ms_per_url.values().sum();
        let total_count: u64 = self.timed_calls_per_url.values().sum();
        if total_count > 0 {
            (total_sum as f64) / (total_count as f64)
        } else {
            0.0
        }
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
        self.calls_per_url_per_method
            .get(url)
            .cloned()
            .unwrap_or_default()
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
        let json_data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize RPC stats: {}", e))?;

        std::fs::write(RPC_STATS, json_data)
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
                            let url_entry = self
                                .calls_per_url_per_method
                                .entry(url)
                                .or_insert_with(HashMap::new);
                            for (method, count) in method_counts {
                                *url_entry.entry(method).or_insert(0) += count;
                            }
                        }

                        // Merge error stats
                        for (url, count) in loaded_stats.errors_per_url {
                            *self.errors_per_url.entry(url).or_insert(0) += count;
                        }

                        for (method, count) in loaded_stats.errors_per_method {
                            *self.errors_per_method.entry(method).or_insert(0) += count;
                        }

                        // Merge response time sums
                        for (url, sum_ms) in loaded_stats.response_time_sum_ms_per_url {
                            *self.response_time_sum_ms_per_url.entry(url).or_insert(0) += sum_ms;
                        }

                        for (method, sum_ms) in loaded_stats.response_time_sum_ms_per_method {
                            *self
                                .response_time_sum_ms_per_method
                                .entry(method)
                                .or_insert(0) += sum_ms;
                        }

                        // Merge timed call counts
                        for (url, count) in loaded_stats.timed_calls_per_url {
                            *self.timed_calls_per_url.entry(url).or_insert(0) += count;
                        }

                        for (method, count) in loaded_stats.timed_calls_per_method {
                            *self.timed_calls_per_method.entry(method).or_insert(0) += count;
                        }

                        log(
                            LogTag::Rpc,
                            "STATS",
                            &format!(
                                "Loaded RPC stats from disk: {} total calls, {} URLs, {} methods",
                                total_calls, url_count, method_count
                            ),
                        );
                        Ok(())
                    }
                    Err(e) => {
                        if is_debug_rpc_enabled() {
                            log(
                                LogTag::Rpc,
                                "WARNING",
                                &format!("Failed to parse RPC stats file, starting fresh: {}", e),
                            );
                        }
                        Ok(())
                    }
                }
            }
            Err(_) => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "INFO",
                        "No existing RPC stats file found, starting fresh",
                    );
                }
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
        Self::new(MAX_RPC_CALLS_PER_SECOND) // Use hardcoded rate limit constant
    }

    /// Wait for rate limit before making a call to main RPC with adaptive backoff
    pub async fn wait_for_main_rpc(&mut self) {
        if let Some(last_call) = self.last_main_call {
            let elapsed = last_call.elapsed();
            if elapsed < self.current_interval {
                let wait_duration = self.current_interval - elapsed;
                if is_debug_rpc_enabled() {
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
                }
                tokio::time::sleep(wait_duration).await;
            }
        }
        self.last_main_call = Some(Instant::now());
    }

    /// Wait for rate limit for a specific URL
    pub async fn wait_for_url(&mut self, url: &str) {
        let url_interval = self
            .url_intervals
            .get(url)
            .unwrap_or(&self.current_interval)
            .clone();

        if let Some(last_call) = self.url_last_calls.get(url) {
            let elapsed = last_call.elapsed();
            if elapsed < url_interval {
                let wait_duration = url_interval - elapsed;
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "RATE_LIMIT",
                        &format!(
                            "Rate limiting URL {}: waiting {:.2}ms",
                            url,
                            wait_duration.as_millis()
                        ),
                    );
                }
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
            self.url_intervals
                .insert(url.to_string(), self.current_interval);
        }

        log(
            LogTag::Rpc,
            "RATE_LIMIT",
            &format!(
                "429 error #{}: increased interval to {:.2}ms (max: {:.2}ms)",
                self.consecutive_429s,
                self.current_interval.as_millis(),
                self.max_interval.as_millis()
            ),
        );
    }

    /// Record a successful call and gradually reduce backoff
    pub fn record_success(&mut self, url: Option<&str>) {
        self.consecutive_successes += 1;

        // After 5 consecutive successes, reduce interval back towards normal
        if self.consecutive_successes >= 5 {
            let had_previous_429s = self.consecutive_429s > 0;
            self.consecutive_429s = self.consecutive_429s.saturating_sub(1);
            self.consecutive_successes = 0;

            if self.consecutive_429s == 0 {
                self.current_interval = self.base_interval;
                // Only log rate limit reset if we actually had 429 errors to recover from
                // This prevents spam when using premium-only RPC mode
                if had_previous_429s && is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "RATE_LIMIT",
                        "Rate limit backoff reset to normal",
                    );
                }
            } else {
                let backoff_factor = self.backoff_multiplier.powi(self.consecutive_429s as i32);
                let new_interval_ms =
                    ((self.base_interval.as_millis() as f64) * backoff_factor) as u64;
                self.current_interval = Duration::from_millis(new_interval_ms);
                log(
                    LogTag::Rpc,
                    "RATE_LIMIT",
                    &format!(
                        "Reduced rate limit backoff to {:.2}ms (429s remaining: {})",
                        self.current_interval.as_millis(),
                        self.consecutive_429s
                    ),
                );
            }

            if let Some(url) = url {
                self.url_intervals
                    .insert(url.to_string(), self.current_interval);
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
        if is_debug_rpc_enabled() {
            log(LogTag::Rpc, "RATE_LIMIT", "Rate limiter reset");
        }
    }

    /// Set a custom interval for a specific URL (useful for premium RPCs)
    pub fn set_url_interval(&mut self, url: &str, interval: Duration) {
        self.url_intervals.insert(url.to_string(), interval);
        log(
            LogTag::Rpc,
            "RATE_LIMIT",
            &format!(
                "Set custom interval for {}: {:.2}ms",
                url,
                interval.as_millis()
            ),
        );
    }
}

/// Centralized RPC client with round-robin load balancing and error handling
pub struct RpcClient {
    client: Arc<SolanaRpcClient>,
    /// List of all available RPC URLs for round-robin usage
    rpc_urls: Vec<String>,
    /// Current index for round-robin rotation
    current_url_index: Arc<std::sync::Mutex<usize>>,
    /// Current active URL (changes with round-robin)
    current_url: Arc<std::sync::Mutex<String>>,
    stats: Arc<std::sync::Mutex<RpcStats>>,
    rate_limiter: Arc<tokio::sync::Mutex<RpcRateLimiter>>,
}

impl Clone for RpcClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            rpc_urls: self.rpc_urls.clone(),
            current_url_index: self.current_url_index.clone(),
            current_url: self.current_url.clone(),
            stats: self.stats.clone(),
            rate_limiter: self.rate_limiter.clone(),
        }
    }
}

impl RpcClient {
    /// Create new RPC client with configuration from config.toml
    pub fn new() -> Self {
        Self::from_config().unwrap_or_else(|e| {
            if is_debug_rpc_enabled() {
                log(
                    LogTag::Rpc,
                    "ERROR",
                    &format!("Failed to load config: {}", e),
                );
            }
            log(
                LogTag::Rpc,
                "FATAL",
                "Cannot initialize RPC client without valid configuration. Please check config.toml"
            );
            std::process::exit(1);
        })
    }

    /// Create new RPC client from config
    pub fn from_config() -> Result<Self, String> {
        let rpc_urls = crate::config::with_config(|cfg| cfg.rpc.urls.clone());

        if rpc_urls.is_empty() {
            return Err("No RPC URLs configured".to_string());
        }

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "INIT",
                &format!(
                    "Initializing RPC client with {} URLs for round-robin rotation",
                    rpc_urls.len()
                ),
            );
        }

        if is_debug_rpc_enabled() {
            for (i, url) in rpc_urls.iter().enumerate() {
                log(
                    LogTag::Rpc,
                    "RPC_URL",
                    &format!("RPC URL {}: {}", i + 1, url),
                );
            }
        }

        Self::new_with_urls(rpc_urls)
    }

    /// Create new RPC client with a list of URLs for round-robin rotation
    pub fn new_with_urls(rpc_urls: Vec<String>) -> Result<Self, String> {
        if rpc_urls.is_empty() {
            return Err("RPC URLs list cannot be empty".to_string());
        }

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "INIT",
                &format!("Initializing RPC client with {} URLs", rpc_urls.len()),
            );
        }

        // Start with the first URL
        let first_url = rpc_urls[0].clone();
        let client =
            SolanaRpcClient::new_with_commitment(first_url.clone(), CommitmentConfig::confirmed());

        let mut stats = RpcStats::default();
        let _ = stats.load_from_disk(); // Load existing stats, ignore errors

        Ok(Self {
            client: Arc::new(client),
            rpc_urls: rpc_urls.clone(),
            current_url_index: Arc::new(std::sync::Mutex::new(0)),
            current_url: Arc::new(std::sync::Mutex::new(first_url)),
            stats: Arc::new(std::sync::Mutex::new(stats)),
            rate_limiter: Arc::new(tokio::sync::Mutex::new(RpcRateLimiter::new_conservative())),
        })
    }

    /// Create new RPC client with custom URL (legacy method)
    pub fn new_with_url(rpc_url: &str) -> Self {
        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "INIT",
                &format!("Initializing RPC client with URL: {}", rpc_url),
            );
        }

        let client = SolanaRpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        let mut stats = RpcStats::default();
        let _ = stats.load_from_disk(); // Load existing stats, ignore errors

        Self {
            client: Arc::new(client),
            rpc_urls: vec![rpc_url.to_string()],
            current_url_index: Arc::new(std::sync::Mutex::new(0)),
            current_url: Arc::new(std::sync::Mutex::new(rpc_url.to_string())),
            stats: Arc::new(std::sync::Mutex::new(stats)),
            rate_limiter: Arc::new(tokio::sync::Mutex::new(RpcRateLimiter::new_conservative())),
        }
    }

    /// Get the underlying RPC client
    pub fn client(&self) -> Arc<SolanaRpcClient> {
        self.client.clone()
    }

    /// Get current RPC URL
    pub fn url(&self) -> String {
        match self.current_url.lock() {
            Ok(url) => url.clone(),
            Err(_) => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "Failed to lock current_url - using first URL",
                    );
                }
                self.rpc_urls.get(0).unwrap_or(&"".to_string()).clone()
            }
        }
    }

    /// Get all available RPC URLs
    pub fn get_all_urls(&self) -> Vec<String> {
        self.rpc_urls.clone()
    }

    /// Get RPC statistics
    pub fn get_stats(&self) -> RpcStats {
        match self.stats.try_lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "RPC stats lock contention - returning default stats",
                    );
                }
                RpcStats::default()
            }
        }
    }

    /// Save RPC statistics to disk
    pub fn save_stats(&self) -> Result<(), String> {
        match self.stats.try_lock() {
            Ok(mut stats) => stats.save_to_disk(),
            Err(_) => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "RPC stats lock contention during save - stats not saved",
                    );
                }
                Err("Failed to acquire stats lock for saving".to_string())
            }
        }
    }

    /// Rotate to the next RPC URL in round-robin fashion
    pub fn rotate_to_next_url(&self) -> String {
        let next_url = match (self.current_url_index.lock(), self.current_url.lock()) {
            (Ok(mut index), Ok(mut current_url)) => {
                *index = (*index + 1) % self.rpc_urls.len();
                let new_url = self.rpc_urls[*index].clone();
                *current_url = new_url.clone();

                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "ROTATE",
                        &format!("Rotated to RPC URL {} (index {})", *index + 1, *index),
                    );
                }

                new_url
            }
            _ => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "Failed to rotate URL - lock contention",
                    );
                }
                self.rpc_urls.get(0).unwrap_or(&"".to_string()).clone()
            }
        };

        // Update the underlying client to use the new URL
        let new_client =
            SolanaRpcClient::new_with_commitment(next_url.clone(), CommitmentConfig::confirmed());

        // Note: We can't directly update self.client since it's behind Arc
        // The client will be updated on the next method call that creates a new client

        next_url
    }

    /// Get the next RPC URL without rotating (for preview)
    pub fn get_next_url(&self) -> String {
        match self.current_url_index.lock() {
            Ok(index) => {
                let next_index = (*index + 1) % self.rpc_urls.len();
                self.rpc_urls
                    .get(next_index)
                    .unwrap_or(&"".to_string())
                    .clone()
            }
            Err(_) => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "Failed to get next URL - lock contention",
                    );
                }
                self.rpc_urls.get(0).unwrap_or(&"".to_string()).clone()
            }
        }
    }

    /// Get current RPC URL index
    pub fn get_current_url_index(&self) -> usize {
        match self.current_url_index.lock() {
            Ok(index) => *index,
            Err(_) => {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "Failed to get URL index - lock contention",
                    );
                }
                0
            }
        }
    }

    /// Create a new client using the current URL for actual RPC calls
    fn create_current_client(&self) -> Arc<SolanaRpcClient> {
        let current_url = self.url();
        let client =
            SolanaRpcClient::new_with_commitment(current_url, CommitmentConfig::confirmed());
        Arc::new(client)
    }

    /// Perform round-robin rotation and create a client for the next URL
    /// This should be called before each RPC operation to ensure load balancing
    fn prepare_next_rpc_call(&self) -> Arc<SolanaRpcClient> {
        // Rotate to next URL for round-robin load balancing
        let current_url = self.rotate_to_next_url();

        // Create a new client with the rotated URL
        let client =
            SolanaRpcClient::new_with_commitment(current_url, CommitmentConfig::confirmed());
        Arc::new(client)
    }

    /// Record an RPC call for statistics
    fn record_call(&self, method: &str) {
        if let Ok(mut stats) = self.stats.lock() {
            // Use the current URL for statistics
            let url_to_record = self.url();
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

    /// Record an RPC error for statistics
    fn record_error(&self, url: &str, method: &str) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_error(url, method);
        }
    }

    /// Record response time for an RPC call
    fn record_response_time(&self, url: &str, method: &str, duration_ms: u64) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.record_response_time(url, method, duration_ms);
        }
    }

    /// Wait for rate limit with adaptive backoff
    /// Now applies to all RPC URLs in the round-robin list
    async fn wait_for_rate_limit(&self) {
        let current_url = self.url();
        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.wait_for_url(&current_url).await;
    }

    /// Wait for rate limit for a specific URL
    async fn wait_for_rate_limit_url(&self, url: &str) {
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

    /// Set a custom rate limit interval for a specific URL
    pub async fn set_url_rate_limit(&self, url: &str, interval: Duration) {
        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.set_url_interval(url, interval);
    }

    /// Reset rate limiter (useful when switching networks or after prolonged downtime)
    pub async fn reset_rate_limiter(&self) {
        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.reset();
    }

    /// Create a client for the current URL in the round-robin rotation
    pub fn create_current_rpc_client(&self) -> Arc<SolanaRpcClient> {
        let current_url = self.url();
        if is_debug_rpc_enabled() {
            log(LogTag::Rpc, "CLIENT", "Creating client for current URL");
        }
        let client =
            SolanaRpcClient::new_with_commitment(current_url, CommitmentConfig::confirmed());
        Arc::new(client)
    }

    /// Create a client specifically for a given URL (for specific operations)
    pub fn create_client_for_url(&self, url: &str) -> Arc<SolanaRpcClient> {
        if is_debug_rpc_enabled() {
            log(LogTag::Rpc, "CLIENT", "Creating client for specific URL");
        }
        let client =
            SolanaRpcClient::new_with_commitment(url.to_string(), CommitmentConfig::confirmed());
        Arc::new(client)
    }

    /// Get next available URL from round-robin (backward compatibility)
    /// This is used to replace the old premium RPC concept
    pub fn get_best_available_url(&self) -> String {
        // Use the current URL, which is already rotated via round-robin
        self.url()
    }

    /// Backward compatibility: get premium URL (now returns current round-robin URL)
    pub fn premium_url(&self) -> Option<String> {
        Some(self.url())
    }

    /// Backward compatibility: create premium client (now returns current client)
    pub fn create_premium_client(&self) -> Option<Arc<SolanaRpcClient>> {
        Some(self.create_current_rpc_client())
    }

    /// Backward compatibility: create main client (now returns current client)
    pub fn create_main_client(&self) -> Arc<SolanaRpcClient> {
        self.create_current_rpc_client()
    }

    /// Check if error should trigger fallback (rate limits, timeouts) vs real errors (account not found)
    fn should_fallback_on_error(error: &str) -> bool {
        let error_lower = error.to_lowercase();

        // Rate limiting and temporary issues - should fallback
        if error_lower.contains("429")
            || error_lower.contains("too many requests")
            || error_lower.contains("rate limit")
            || error_lower.contains("timeout")
            || error_lower.contains("connection")
            || error_lower.contains("network")
        {
            return true;
        }

        // Real blockchain state - don't fallback, cache as failed
        if error_lower.contains("account not found")
            || error_lower.contains("invalid account")
            || error_lower.contains("account does not exist")
        {
            return false;
        }

        // Default to fallback for unknown errors
        true
    }

    /// Check if error is specifically a 429 rate limit error
    fn is_rate_limit_error(error: &str) -> bool {
        let error_lower = error.to_lowercase();
        error_lower.contains("429")
            || error_lower.contains("too many requests")
            || error_lower.contains("rate limit")
    }

    /// Check if HTTP response indicates rate limiting
    fn is_rate_limit_response(response: &reqwest::Response) -> bool {
        response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
    }

    /// Backward compatibility: access to current URL as rpc_url field
    pub fn rpc_url(&self) -> String {
        self.url()
    }

    /// Backward compatibility: fallback URLs (now returns all URLs except current)
    pub fn fallback_urls(&self) -> Vec<String> {
        let current_url = self.url();
        self.rpc_urls
            .iter()
            .filter(|url| *url != &current_url)
            .cloned()
            .collect()
    }

    /// Get single account data
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String> {
        let start = std::time::Instant::now();
        self.wait_for_rate_limit().await;
        self.record_call("get_account");

        // Use round-robin to get the next RPC URL and create client
        let client = self.prepare_next_rpc_call();
        let current_url = self.url();

        let result = tokio::task::spawn_blocking({
            let pubkey = *pubkey;
            let url = current_url.clone();
            move || {
                client.get_account(&pubkey).map_err(|e| {
                    let es = e.to_string();
                    if es.contains("AccountNotFound") || es.contains("could not find account") {
                        let blockchain_error = BlockchainError::AccountNotFound {
                            pubkey: pubkey.to_string(),
                            context: "get_account".to_string(),
                            rpc_endpoint: Some(url.clone()),
                        };
                        format!("blockchain_error:{}:{}:{}", pubkey, url, blockchain_error)
                    } else {
                        let blockchain_error = parse_solana_error(&es, None, "rpc_call");
                        format!("blockchain_error:{}:{}:{}", pubkey, url, blockchain_error)
                    }
                })
            }
        })
        .await
        .map_err(|e| format!("Task error: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok(_) => {
                self.record_response_time(&current_url, "get_account", duration_ms);
                self.record_success(Some(&current_url));
            }
            Err(_) => {
                self.record_error(&current_url, "get_account");
            }
        }

        result
    }

    /// Get single account data with custom commitment level (for debugging)
    pub async fn get_account_with_commitment(
        &self,
        pubkey: &Pubkey,
        commitment: CommitmentConfig,
    ) -> Result<Account, String> {
        let start = std::time::Instant::now();
        self.wait_for_rate_limit().await;
        self.record_call("get_account_with_commitment");
        let url = self.url().to_string();

        let result = tokio::task::spawn_blocking({
            let client = SolanaRpcClient::new_with_commitment(url.clone(), commitment);
            let pubkey = *pubkey;
            let url = url.clone();
            move || {
                client.get_account(&pubkey).map_err(|e| {
                    let es = e.to_string();
                    if es.contains("AccountNotFound") || es.contains("could not find account") {
                        let blockchain_error = BlockchainError::AccountNotFound {
                            pubkey: pubkey.to_string(),
                            context: "get_account_with_commitment".to_string(),
                            rpc_endpoint: Some(url.clone()),
                        };
                        format!("blockchain_error:{}:{}:{}", pubkey, url, blockchain_error)
                    } else {
                        let blockchain_error = parse_solana_error(&es, None, "rpc_call");
                        format!("blockchain_error:{}:{}:{}", pubkey, url, blockchain_error)
                    }
                })
            }
        })
        .await
        .map_err(|e| format!("Task error: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok(_) => {
                self.record_response_time(&url, "get_account_with_commitment", duration_ms);
                self.record_success(Some(&url));
            }
            Err(_) => {
                self.record_error(&url, "get_account_with_commitment");
            }
        }

        result
    }

    /// Get multiple accounts data (batch request for efficiency)
    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, String> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();
        let start = std::time::Instant::now();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "BATCH",
                &format!(
                    "Fetching {} accounts from RPC: {}",
                    pubkeys.len(),
                    current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("get_multiple_accounts");

        // Create client for the rotated URL
        let client = SolanaRpcClient::new_with_commitment(
            current_url.clone(),
            CommitmentConfig::confirmed(),
        );

        let url_for_closure = current_url.clone();
        let result = tokio::task::spawn_blocking({
            let keys = pubkeys.to_vec();
            move || {
                client.get_multiple_accounts(&keys).map_err(|e| {
                    let error_str = e.to_string();

                    // Check for rate limiting errors
                    if error_str.to_lowercase().contains("429")
                        || error_str.to_lowercase().contains("too many requests")
                        || error_str.to_lowercase().contains("rate limit")
                    {
                        format!("rate_limit:{}:{}", url_for_closure, error_str)
                    } else {
                        let blockchain_error =
                            parse_solana_error(&error_str, None, "get_multiple_accounts");
                        format!(
                            "blockchain_error:multi:{}:{}",
                            url_for_closure, blockchain_error
                        )
                    }
                })
            }
        })
        .await
        .map_err(|e| format!("Task error: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok(_) => {
                self.record_response_time(&current_url, "get_multiple_accounts", duration_ms);
                self.record_success(Some(&current_url));
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "BATCH",
                        &format!("Successfully fetched {} accounts", pubkeys.len()),
                    );
                }
            }
            Err(e) => {
                self.record_error(&current_url, "get_multiple_accounts");
                if e.starts_with("rate_limit:") {
                    self.record_429_error(Some(&current_url));
                    if is_debug_rpc_enabled() {
                        log(LogTag::Rpc, "WARN", "Rate limited on RPC for batch request");
                    }
                } else {
                    if is_debug_rpc_enabled() {
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!("Failed to fetch accounts: {}", e),
                        );
                    }
                }
            }
        }

        result
    }

    /// Get current slot
    pub async fn get_slot(&self) -> Result<u64, String> {
        let start = std::time::Instant::now();
        let current_url = self.url();
        self.wait_for_rate_limit().await;
        self.record_call("get_slot");

        let result = tokio::task::spawn_blocking({
            let client = self.client.clone();
            move || {
                client
                    .get_slot()
                    .map_err(|e| format!("Failed to get slot: {}", e))
            }
        })
        .await
        .map_err(|e| format!("Task error: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok(_) => {
                self.record_response_time(&current_url, "get_slot", duration_ms);
                self.record_success(Some(&current_url));
            }
            Err(_) => {
                self.record_error(&current_url, "get_slot");
            }
        }

        result
    }

    /// Get SOL balance for wallet address using round-robin RPC rotation
    pub async fn get_sol_balance(&self, wallet_address: &str) -> Result<f64, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [wallet_address]
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();
        let start = std::time::Instant::now();

        if is_debug_wallet_enabled() || is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!("Checking SOL balance for wallet: {}", wallet_address),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getBalance");

        let client = reqwest::Client::new();

        let result: Result<f64, ()> = match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(balance_lamports) = value.as_u64() {
                                    let balance_sol = lamports_to_sol(balance_lamports);

                                    // Record successful call
                                    let duration_ms = start.elapsed().as_millis() as u64;
                                    self.record_response_time(
                                        &current_url,
                                        "getBalance",
                                        duration_ms,
                                    );
                                    self.record_success(Some(&current_url));

                                    if is_debug_wallet_enabled() || is_debug_rpc_enabled() {
                                        log(
                                            LogTag::Rpc,
                                            "SUCCESS",
                                            &format!(
                                                "SOL balance retrieved: {} lamports ({:.6} SOL)",
                                                balance_lamports, balance_sol
                                            ),
                                        );
                                    }

                                    return Ok(balance_sol);
                                }
                            }
                        }
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.record_429_error(Some(&current_url));
                    log(LogTag::Rpc, "WARN", "Rate limited on RPC for SOL balance");
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("HTTP error {} for SOL balance", response.status()),
                    );
                }
                Err(())
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(LogTag::Rpc, "WARN", "Rate limited on RPC for SOL balance");
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get SOL balance: {}", e),
                    );
                }
                Err(())
            }
        };

        // Record error if we failed
        if result.is_err() {
            self.record_error(&current_url, "getBalance");
        }

        Err(ScreenerBotError::RpcProvider(
            crate::errors::RpcProviderError::Generic {
                provider_name: current_url,
                message: "Failed to get SOL balance from RPC endpoint".to_string(),
            },
        ))
    }

    /// Get token balance for wallet address using round-robin RPC rotation
    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> Result<u64, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [
                wallet_address,
                { "mint": mint },
                { "encoding": "jsonParsed", "commitment": "confirmed" }
            ]
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "TOKEN_BALANCE",
                &format!(
                    "Fetching token balance for wallet {} mint {} from RPC: {}",
                    wallet_address, mint, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getTokenAccountsByOwner");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(accounts) = value.as_array() {
                                    if let Some(account) = accounts.first() {
                                        if let Some(account_data) = account.get("account") {
                                            if let Some(data) = account_data.get("data") {
                                                if let Some(parsed) = data.get("parsed") {
                                                    if let Some(info) = parsed.get("info") {
                                                        if let Some(token_amount) =
                                                            info.get("tokenAmount")
                                                        {
                                                            if let Some(amount_str) =
                                                                token_amount.get("amount")
                                                            {
                                                                if let Some(amount_str) =
                                                                    amount_str.as_str()
                                                                {
                                                                    if let Ok(amount) =
                                                                        amount_str.parse::<u64>()
                                                                    {
                                                                        // Record successful call
                                                                        self.record_success(Some(
                                                                            &current_url,
                                                                        ));

                                                                        if is_debug_rpc_enabled() {
                                                                            log(
                                                                                LogTag::Rpc,
                                                                                "SUCCESS",
                                                                                &format!(
                                                                                    "Found token balance {} for wallet {} mint {} from RPC: {}",
                                                                                    amount,
                                                                                    wallet_address,
                                                                                    mint,
                                                                                    current_url
                                                                                )
                                                                            );
                                                                        }

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

                                // No token account found - this is normal, not an error
                                log(
                                    LogTag::Rpc,
                                    "INFO",
                                    &format!(
                                        "No token account found for wallet {} mint {} on RPC {}",
                                        wallet_address, mint, current_url
                                    ),
                                );

                                // Record successful call (even though balance is 0)
                                self.record_success(Some(&current_url));
                                return Ok(0);
                            }
                        }

                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!(
                                "Invalid response format for token balance from RPC {}",
                                current_url
                            ),
                        );
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for token balance", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "HTTP error {} from RPC {} for token balance",
                            response.status(),
                            current_url
                        ),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for token balance", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "Failed to get token balance from RPC {}: {}",
                            current_url, e
                        ),
                    );
                }
            }
        }

        // Default to 0 if we can't determine the balance
        log(
            LogTag::Rpc,
            "WARN",
            &format!(
                "Defaulting to 0 token balance for wallet {} mint {} due to RPC issues",
                wallet_address, mint
            ),
        );

        Ok(0)
    }

    /// Get latest blockhash using round-robin RPC rotation
    pub async fn get_latest_blockhash(&self) -> Result<Hash, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [
                {
                    "commitment": "finalized"
                }
            ]
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();
        let start = std::time::Instant::now();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "BLOCKHASH",
                "Fetching latest blockhash from RPC",
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getLatestBlockhash");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(rpc_response) => {
                            // Check for RPC errors first
                            if let Some(error) = rpc_response.get("error") {
                                let error_str = error.to_string();

                                // Check if it's a rate limit error
                                if Self::is_rate_limit_error(&error_str) {
                                    self.record_429_error(Some(&current_url));
                                    log(LogTag::Rpc, "WARN", "Rate limited on RPC for blockhash");
                                } else {
                                    let blockchain_error = parse_solana_error(
                                        &error_str,
                                        None,
                                        "get_latest_blockhash",
                                    );
                                    log(
                                        LogTag::Rpc,
                                        "ERROR",
                                        &format!(
                                            "RPC error getting latest blockhash: {}",
                                            blockchain_error
                                        ),
                                    );
                                    return Err(ScreenerBotError::Blockchain(blockchain_error));
                                }
                            }

                            // Check for successful result
                            if let Some(result) = rpc_response.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let Some(blockhash_str) =
                                        value.get("blockhash").and_then(|b| b.as_str())
                                    {
                                        if let Ok(blockhash) = Hash::from_str(blockhash_str) {
                                            // Record successful call with timing
                                            let duration_ms = start.elapsed().as_millis() as u64;
                                            self.record_response_time(
                                                &current_url,
                                                "getLatestBlockhash",
                                                duration_ms,
                                            );
                                            self.record_success(Some(&current_url));

                                            if is_debug_rpc_enabled() {
                                                log(
                                                    LogTag::Rpc,
                                                    "BLOCKHASH",
                                                    &format!(
                                                        "Successfully fetched blockhash {}",
                                                        blockhash
                                                    ),
                                                );
                                            }

                                            return Ok(blockhash);
                                        }
                                    }
                                }
                            }

                            log(LogTag::Rpc, "WARN", "No valid blockhash found in response");
                        }
                        Err(e) => {
                            log(
                                LogTag::Rpc,
                                "WARN",
                                &format!("Failed to parse blockhash response: {}", e),
                            );
                        }
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    // Record 429 error for adaptive rate limiting
                    self.record_429_error(Some(&current_url));
                    log(LogTag::Rpc, "WARN", "Rate limited on RPC for blockhash");
                } else {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!(
                            "RPC error status {} for blockhash: {}",
                            response.status(),
                            current_url
                        ),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC for blockhash: {}", error_msg),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to connect to RPC for blockhash: {}", e),
                    );
                    return Err(ScreenerBotError::Network(
                        crate::errors::NetworkError::Generic {
                            message: format!("Failed to get latest blockhash from RPC: {}", e),
                        },
                    ));
                }
            }
        }

        // Record error for failed call
        self.record_error(&current_url, "getLatestBlockhash");

        // If we reach here, the call failed but it may be a rate limit - next call will use next RPC
        Err(ScreenerBotError::RpcProvider(
            crate::errors::RpcProviderError::Generic {
                provider_name: "round_robin_rpc".to_string(),
                message: "Failed to get latest blockhash from current RPC endpoint".to_string(),
            },
        ))
    }

    /// Get latest blockhash with validity information for transaction expiration checking
    pub async fn get_latest_blockhash_with_commitment(
        &self,
    ) -> Result<(Hash, u64), ScreenerBotError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [
                {
                    "commitment": "finalized"
                }
            ]
        });

        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "BLOCKHASH_WITH_HEIGHT",
                &format!(
                    "Fetching latest blockhash with height from RPC: {}",
                    current_url
                ),
            );
        }

        self.wait_for_rate_limit().await;
        self.record_call("getLatestBlockhash");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(rpc_response) => {
                            if let Some(error) = rpc_response.get("error") {
                                let error_str = error.to_string();
                                if Self::is_rate_limit_error(&error_str) {
                                    self.record_429_error(Some(&current_url));
                                }
                                return Err(ScreenerBotError::Blockchain(parse_solana_error(
                                    &error_str,
                                    None,
                                    "get_latest_blockhash_with_commitment",
                                )));
                            }

                            if let Some(result) = rpc_response.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let (Some(blockhash_str), Some(last_valid_block_height)) = (
                                        value.get("blockhash").and_then(|v| v.as_str()),
                                        value.get("lastValidBlockHeight").and_then(|v| v.as_u64()),
                                    ) {
                                        if let Ok(blockhash) = Hash::from_str(blockhash_str) {
                                            if is_debug_rpc_enabled() {
                                                log(
                                                    LogTag::Rpc,
                                                    "BLOCKHASH_WITH_HEIGHT",
                                                    &format!(
                                                        "Successfully fetched blockhash {} with last valid height {} from {}",
                                                        blockhash,
                                                        last_valid_block_height,
                                                        current_url
                                                    )
                                                );
                                            }
                                            return Ok((blockhash, last_valid_block_height));
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to parse blockhash with height response: {}", e),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                }
                return Err(ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to get latest blockhash with height from RPC: {}", e),
                }));
            }
        }

        Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
            provider_name: "round_robin_rpc".to_string(),
            message: "Failed to get latest blockhash with height from current RPC endpoint"
                .to_string(),
        }))
    }

    /// Get current block height for transaction expiration checking
    pub async fn get_block_height(&self) -> Result<u64, ScreenerBotError> {
        // Try cached block height first (short TTL)
        {
            let cache_guard = BLOCK_HEIGHT_CACHE.lock().await;
            if let (Some(height), Some(fetched_at)) = (cache_guard.height, cache_guard.fetched_at) {
                if fetched_at.elapsed().as_secs() < BLOCK_HEIGHT_CACHE_TTL_SECS {
                    if is_debug_rpc_enabled() {
                        log(
                            LogTag::Rpc,
                            "BLOCK_HEIGHT",
                            &format!(
                                "Using cached block height {} (age {:.3}s)",
                                height,
                                fetched_at.elapsed().as_secs_f32()
                            ),
                        );
                    }
                    return Ok(height);
                }
            }
        }

        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBlockHeight",
            "params": [
                {
                    "commitment": "finalized"
                }
            ]
        });

        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "BLOCK_HEIGHT",
                &format!("Fetching current block height from RPC: {}", current_url),
            );
        }

        self.wait_for_rate_limit().await;
        self.record_call("getBlockHeight");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(rpc_response) => {
                            if let Some(error) = rpc_response.get("error") {
                                let error_str = error.to_string();
                                if Self::is_rate_limit_error(&error_str) {
                                    self.record_429_error(Some(&current_url));
                                }
                                return Err(ScreenerBotError::Blockchain(parse_solana_error(
                                    &error_str,
                                    None,
                                    "get_block_height",
                                )));
                            }

                            if let Some(result) = rpc_response.get("result") {
                                if let Some(block_height) = result.as_u64() {
                                    // Update cache before returning
                                    {
                                        let mut cache_guard = BLOCK_HEIGHT_CACHE.lock().await;
                                        cache_guard.height = Some(block_height);
                                        cache_guard.fetched_at = Some(Instant::now());
                                    }

                                    if is_debug_rpc_enabled() {
                                        log(
                                            LogTag::Rpc,
                                            "BLOCK_HEIGHT",
                                            &format!(
                                                "Successfully fetched block height {} from {}",
                                                block_height, current_url
                                            ),
                                        );
                                    }

                                    return Ok(block_height);
                                }
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Rpc,
                                "ERROR",
                                &format!("Failed to parse block height response: {}", e),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                }
                return Err(ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to get block height from RPC: {}", e),
                }));
            }
        }

        Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
            provider_name: "round_robin_rpc".to_string(),
            message: "Failed to get block height from current RPC endpoint".to_string(),
        }))
    }

    /// Send transaction using round-robin RPC rotation
    pub async fn send_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<String, ScreenerBotError> {
        // Serialize transaction
        let serialized_tx = bincode::serialize(transaction).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("Failed to serialize transaction: {}", e),
            })
        })?;

        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&serialized_tx);

        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [
                tx_base64,
                {
                    "encoding": "base64",
                    "skipPreflight": true,
                    "preflightCommitment": "processed",
                    "maxRetries": 3
                }
            ]
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();
        let start = std::time::Instant::now();

        if is_debug_rpc_enabled() {
            log(LogTag::Rpc, "TX_SEND", "Sending transaction to RPC");
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("sendTransaction");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(signature) = result.as_str() {
                                // Record successful call with timing
                                let duration_ms = start.elapsed().as_millis() as u64;
                                self.record_response_time(
                                    &current_url,
                                    "sendTransaction",
                                    duration_ms,
                                );
                                self.record_success(Some(&current_url));

                                log(
                                    LogTag::Rpc,
                                    "SUCCESS",
                                    &format!("Transaction sent successfully: {}", signature),
                                );
                                return Ok(signature.to_string());
                            }
                        }

                        if let Some(error) = rpc_response.get("error") {
                            let error_msg = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown RPC error");

                            log(LogTag::Rpc, "ERROR", &format!("RPC error: {}", error_msg));

                            // Parse Solana-specific error using new structured approach
                            let blockchain_error =
                                parse_solana_error(error_msg, None, "transaction_send");

                            return Err(ScreenerBotError::Blockchain(blockchain_error));
                        }
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "Rate limited on RPC for transaction send",
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("HTTP error {} for transaction send", response.status()),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        "Rate limited on RPC for transaction send",
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to send transaction: {}", e),
                    );

                    return Err(ScreenerBotError::Network(
                        crate::errors::NetworkError::Generic {
                            message: format!("Failed to send transaction to RPC: {}", e),
                        },
                    ));
                }
            }
        }

        // Record error for failed call
        self.record_error(&current_url, "sendTransaction");

        Err(ScreenerBotError::Blockchain(
            crate::errors::BlockchainError::TransactionDropped {
                signature: "unknown".to_string(),
                reason: "Failed to send transaction to current RPC endpoint".to_string(),
                fee_paid: None,
                attempts: 1,
            },
        ))
    }

    /// Sign and send transaction using round-robin RPC rotation
    pub async fn sign_and_send_transaction(
        &self,
        swap_transaction_base64: &str,
    ) -> Result<String, ScreenerBotError> {
        if is_debug_wallet_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Starting transaction signing: tx_length={} bytes",
                    swap_transaction_base64.len()
                ),
            );
        }

        log(
            LogTag::Rpc,
            "SIGN",
            &format!(
                "Signing transaction with wallet (length: {} bytes)",
                swap_transaction_base64.len()
            ),
        );

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_START",
                &format!(
                    "🚀 Starting sign_and_send_transaction process with {} byte transaction",
                    swap_transaction_base64.len()
                ),
            );
        }

        // Decode the base64 transaction once (we will mutate & re-sign on retries)
        let original_tx_bytes = base64::engine::general_purpose::STANDARD
            .decode(swap_transaction_base64)
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::ParseError {
                    data_type: "base64_transaction".to_string(),
                    error: format!("Failed to decode transaction: {}", e),
                })
            })?;

        // Deserialize the VersionedTransaction
        let mut transaction: VersionedTransaction = bincode::deserialize(&original_tx_bytes)
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::ParseError {
                    data_type: "VersionedTransaction".to_string(),
                    error: format!("Failed to deserialize transaction: {}", e),
                })
            })?;

        // Create keypair from config
        let keypair = crate::config::get_wallet_keypair().map_err(|e| {
            ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidPrivateKey {
                error: format!("Failed to load wallet keypair: {}", e),
            })
        })?;

        // Helper to (re)sign & serialize current transaction state
        let mut sign_and_serialize =
            |tx: &mut VersionedTransaction| -> Result<String, ScreenerBotError> {
                let sig = keypair.sign_message(&tx.message.serialize());
                if tx.signatures.is_empty() {
                    tx.signatures.push(sig);
                } else {
                    tx.signatures[0] = sig;
                }
                if is_debug_wallet_enabled() {
                    log(
                        LogTag::Rpc,
                        "DEBUG",
                        &format!(
                            "Transaction signed: wallet_pubkey={}, signature={}",
                            keypair.pubkey(),
                            sig
                        ),
                    );
                }
                let bytes = bincode::serialize(tx).map_err(|e| {
                    ScreenerBotError::Data(crate::errors::DataError::ParseError {
                        data_type: "signed_transaction".to_string(),
                        error: format!("Failed to serialize signed transaction: {}", e),
                    })
                })?;
                Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
            };

        // Extract current recent blockhash (for logging / comparison)
        let initial_blockhash = match &transaction.message {
            solana_sdk::message::VersionedMessage::Legacy(m) => m.recent_blockhash,
            solana_sdk::message::VersionedMessage::V0(m) => m.recent_blockhash,
        };
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_BLOCKHASH",
                &format!("Initial tx blockhash: {}", initial_blockhash),
            );
        }

        // Retry loop with blockhash refresh on specific errors
        const MAX_ATTEMPTS: usize = 3; // 1 initial + up to 2 refresh attempts
        let mut attempt = 0usize;
        let client = reqwest::Client::new();
        let mut last_err: Option<ScreenerBotError> = None;

        while attempt < MAX_ATTEMPTS {
            if attempt > 0 {
                log(
                    LogTag::Rpc,
                    "RETRY",
                    &format!("Retrying transaction send attempt {}", attempt + 1),
                );
            }

            // Re-sign (possibly after blockhash update)
            let signed_transaction_base64 = sign_and_serialize(&mut transaction)?;

            // Build payload each attempt (fresh signed tx)
            let rpc_payload = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "sendTransaction",
                "params": [
                    signed_transaction_base64,
                    { "encoding": "base64", "skipPreflight": true, "preflightCommitment": "processed" }
                ]
            });

            // Use round-robin RPC rotation - get next URL from client
            let current_url = self.rotate_to_next_url();

            if is_debug_transactions_enabled() {
                log(
                    LogTag::Rpc,
                    "TX_DEBUG_SEND",
                    &format!("Attempt {} -> RPC: {}", attempt + 1, current_url),
                );
            }

            // Apply rate limiting
            self.wait_for_rate_limit().await;
            self.record_call("sendTransaction");

            let send_result = client
                .post(&current_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send()
                .await;

            match send_result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<serde_json::Value>().await {
                            Ok(v) => {
                                if let Some(result) = v.get("result").and_then(|r| r.as_str()) {
                                    // Record successful call
                                    self.record_success(Some(&current_url));

                                    log(
                                        LogTag::Rpc,
                                        "SUCCESS",
                                        &format!(
                                            "Transaction sent successfully via {}: {}",
                                            current_url, result
                                        ),
                                    );
                                    return Ok(result.to_string());
                                }

                                if let Some(err_obj) = v.get("error") {
                                    // Extract error message for classification
                                    let msg = err_obj
                                        .get("message")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("Unknown RPC error");

                                    if is_debug_transactions_enabled() {
                                        log(
                                            LogTag::Rpc,
                                            "TX_DEBUG_ERROR",
                                            &format!("RPC {} error: {}", current_url, msg),
                                        );
                                    }

                                    // Parse Solana-specific error using new structured approach
                                    let blockchain_error =
                                        parse_solana_error(msg, None, "transaction_send");

                                    match blockchain_error {
                                        BlockchainError::BlockhashExpired { .. } => {
                                            log(
                                                LogTag::Rpc,
                                                "BLOCKHASH_EXPIRED",
                                                &format!(
                                                    "Blockhash expired at RPC {} (attempt {})",
                                                    current_url,
                                                    attempt + 1
                                                ),
                                            );
                                            // Refresh blockhash for next attempt (if attempts remain)
                                            if attempt + 1 < MAX_ATTEMPTS {
                                                match self.get_latest_blockhash().await {
                                                    Ok(new_bh) => {
                                                        if is_debug_transactions_enabled() {
                                                            log(
                                                                LogTag::Rpc,
                                                                "TX_DEBUG_BLOCKHASH_REFRESH",
                                                                &format!(
                                                                    "Fetched fresh blockhash: {}",
                                                                    new_bh
                                                                ),
                                                            );
                                                        }
                                                        // Update message blockhash
                                                        match &mut transaction.message {
                                                            solana_sdk::message::VersionedMessage::Legacy(
                                                                m,
                                                            ) => {
                                                                m.recent_blockhash = new_bh;
                                                            }
                                                            solana_sdk::message::VersionedMessage::V0(
                                                                m,
                                                            ) => {
                                                                m.recent_blockhash = new_bh;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log(
                                                            LogTag::Rpc,
                                                            "ERROR",
                                                            &format!(
                                                                "Failed to refresh blockhash: {:?}",
                                                                e
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                            // Set error to retry
                                            last_err = Some(ScreenerBotError::Blockchain(
                                                blockchain_error,
                                            ));
                                        }
                                        _ => {
                                            last_err = Some(ScreenerBotError::Blockchain(
                                                blockchain_error,
                                            ));
                                            break; // No retry for other blockchain errors
                                        }
                                    }
                                } else {
                                    last_err = Some(ScreenerBotError::RpcProvider(
                                        crate::errors::RpcProviderError::MalformedResponse {
                                            provider_name: current_url.clone(),
                                            endpoint: "sendTransaction".to_string(),
                                            response_body:
                                                "Missing result or error in RPC response"
                                                    .to_string(),
                                        },
                                    ));
                                    break; // No retry for malformed responses
                                }
                            }
                            Err(e) => {
                                last_err = Some(ScreenerBotError::Data(
                                    crate::errors::DataError::ParseError {
                                        data_type: "RPC_JSON".to_string(),
                                        error: format!("Failed parsing RPC JSON: {}", e),
                                    },
                                ));
                                break; // No retry for JSON parse errors
                            }
                        }
                    } else if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        self.record_429_error(Some(&current_url));
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!("Rate limited on RPC {} for transaction send", current_url),
                        );
                        last_err = Some(ScreenerBotError::RpcProvider(
                            crate::errors::RpcProviderError::Generic {
                                provider_name: current_url.clone(),
                                message: format!("HTTP status {}", resp.status()),
                            },
                        ));
                        break; // No retry for HTTP errors in this simplified version
                    } else {
                        last_err = Some(ScreenerBotError::RpcProvider(
                            crate::errors::RpcProviderError::Generic {
                                provider_name: current_url.clone(),
                                message: format!("HTTP status {}", resp.status()),
                            },
                        ));
                        break; // No retry for HTTP errors
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();

                    // Check for rate limiting errors
                    if Self::is_rate_limit_error(&error_msg) {
                        self.record_429_error(Some(&current_url));
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!("Rate limited on RPC {} for transaction send", current_url),
                        );
                    } else {
                        log(
                            LogTag::Rpc,
                            "ERROR",
                            &format!("Failed to send transaction to RPC {}: {}", current_url, e),
                        );
                    }

                    last_err = Some(ScreenerBotError::Network(
                        crate::errors::NetworkError::Generic {
                            message: format!("Failed to send transaction: {}", e),
                        },
                    ));
                    break; // No retry for network errors in this simplified version
                }
            }

            // Check if we should retry for blockhash expired errors
            if let Some(ScreenerBotError::Blockchain(ref blockchain_err)) = last_err {
                if let BlockchainError::BlockhashExpired { .. } = blockchain_err {
                    if attempt + 1 < MAX_ATTEMPTS {
                        attempt += 1;
                        continue; // Retry with refreshed blockhash
                    }
                }
            }
            break; // No retry condition met
        }

        Err(last_err.unwrap_or_else(|| {
            ScreenerBotError::Blockchain(crate::errors::BlockchainError::TransactionDropped {
                signature: "unknown".to_string(),
                reason: "Failed to send transaction after retries".to_string(),
                fee_paid: None,
                attempts: MAX_ATTEMPTS as u32,
            })
        }))
    }

    /// Sign, send, and confirm a transaction using Solana SDK's RpcClient::send_and_confirm_transaction
    ///
    /// This uses the current round-robin RPC URL to build a blocking SolanaRpcClient, then
    /// calls send_and_confirm_transaction in a blocking thread to avoid stalling the async runtime.
    pub async fn sign_send_and_confirm_transaction(
        &self,
        swap_transaction_base64: &str,
    ) -> Result<String, ScreenerBotError> {
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_START",
                &format!(
                    "🚀 Starting sign_send_and_confirm_transaction with {} byte transaction",
                    swap_transaction_base64.len()
                ),
            );
        }

        // Decode base64 transaction
        let original_tx_bytes = base64::engine::general_purpose::STANDARD
            .decode(swap_transaction_base64)
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::ParseError {
                    data_type: "base64_transaction".to_string(),
                    error: format!("Failed to decode transaction: {}", e),
                })
            })?;

        // Deserialize the VersionedTransaction
        let mut transaction: VersionedTransaction = bincode::deserialize(&original_tx_bytes)
            .map_err(|e| {
                ScreenerBotError::Data(crate::errors::DataError::ParseError {
                    data_type: "VersionedTransaction".to_string(),
                    error: format!("Failed to deserialize transaction: {}", e),
                })
            })?;

        // Create keypair from config
        let keypair = crate::config::get_wallet_keypair().map_err(|e| {
            ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidPrivateKey {
                error: format!("Failed to load wallet keypair: {}", e),
            })
        })?;

        // Sign the transaction (first signature index assumed to be wallet)
        let sig = keypair.sign_message(&transaction.message.serialize());
        if transaction.signatures.is_empty() {
            transaction.signatures.push(sig);
        } else {
            transaction.signatures[0] = sig;
        }

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_SIGNED",
                &format!(
                    "Transaction signed, wallet={}, sig={}",
                    keypair.pubkey(),
                    sig
                ),
            );
        }

        // Use current round-robin URL
        let url = self.url();
        let commitment = CommitmentConfig::confirmed();

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_CLIENT",
                &format!(
                    "Creating blocking RpcClient for send_and_confirm at {}",
                    url
                ),
            );
        }

        // Build blocking client and send+confirm in blocking thread
        // Record submission event (no signature yet)
        crate::events::record_transaction_event("unknown", "submitted", true, None, None, None)
            .await;

        let join_res = tokio::task::spawn_blocking(move || {
            let client = SolanaRpcClient::new_with_commitment(url, commitment);
            client.send_and_confirm_transaction(&transaction)
        })
        .await
        .map_err(|e| {
            ScreenerBotError::Network(crate::errors::NetworkError::Generic {
                message: format!("Join error in send_and_confirm: {}", e),
            })
        })?;

        match join_res {
            Ok(signature) => {
                log(
                    LogTag::Rpc,
                    "CONFIRMED",
                    &format!("Transaction confirmed: {}", signature),
                );
                // Record success event
                crate::events::record_transaction_event(
                    &signature.to_string(),
                    "confirmed",
                    true,
                    None,
                    None,
                    None,
                )
                .await;
                Ok(signature.to_string())
            }
            Err(client_err) => {
                log(
                    LogTag::Rpc,
                    "ERROR",
                    &format!("send_and_confirm failed: {}", client_err),
                );
                // Record failure event
                crate::events::record_transaction_event(
                    "unknown",
                    "failed",
                    false,
                    None,
                    None,
                    Some(&client_err.to_string()),
                )
                .await;
                Err(ScreenerBotError::Blockchain(
                    crate::errors::BlockchainError::TransactionDropped {
                        signature: "unknown".to_string(),
                        reason: format!("send_and_confirm_transaction failed: {}", client_err),
                        fee_paid: None,
                        attempts: 1,
                    },
                ))
            }
        }
    }

    /// Send and confirm a signed transaction with robust confirmation logic
    /// This method accepts an already-signed Transaction object and uses the same
    /// confirmation approach as sign_send_and_confirm_transaction but for pre-signed transactions
    pub async fn send_and_confirm_signed_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<String, ScreenerBotError> {
        use crate::arguments::is_debug_ata_enabled;

        if is_debug_ata_enabled() {
            log(
                LogTag::Rpc,
                "TX_DEBUG_START",
                &format!("🚀 Starting send_and_confirm_signed_transaction"),
            );
        }

        // Convert transaction to bytes for sending
        let serialized_tx = bincode::serialize(transaction).map_err(|e| {
            ScreenerBotError::Data(crate::errors::DataError::ParseError {
                data_type: "transaction".to_string(),
                error: format!("Failed to serialize transaction: {}", e),
            })
        })?;

        // Get current URL for blocking client
        let url = {
            let guard = self.current_url.lock().unwrap();
            guard.clone()
        };

        if is_debug_ata_enabled() {
            log(
                LogTag::Rpc,
                "TX_CONFIRM_PREP",
                &format!(
                    "Creating blocking RpcClient for send_and_confirm at {}",
                    &url[..50]
                ),
            );
        }

        // Build blocking client and send+confirm in blocking thread
        // Record submission event (no signature yet)
        crate::events::record_transaction_event("unknown", "submitted", true, None, None, None)
            .await;
        let signature = tokio::task::spawn_blocking(move || {
            let client = SolanaRpcClient::new_with_commitment(url, CommitmentConfig::confirmed());

            // Deserialize transaction for confirmation
            let transaction: Transaction = bincode::deserialize(&serialized_tx)
                .map_err(|e| format!("Failed to deserialize transaction: {}", e))?;

            client
                .send_and_confirm_transaction(&transaction)
                .map_err(|e| e.to_string())
        })
        .await;

        match signature {
            Ok(Ok(sig_result)) => {
                let signature_str = sig_result.to_string();

                if is_debug_ata_enabled() {
                    log(
                        LogTag::Rpc,
                        "TX_CONFIRM_SUCCESS",
                        &format!("✅ Transaction confirmed: {}", &signature_str[..8]),
                    );
                }

                log(
                    LogTag::Rpc,
                    "SUCCESS",
                    &format!("Transaction confirmed: {}", signature_str),
                );

                // Record success event
                crate::events::record_transaction_event(
                    &signature_str,
                    "confirmed",
                    true,
                    None,
                    None,
                    None,
                )
                .await;

                Ok(signature_str)
            }
            Ok(Err(client_err)) => {
                if is_debug_ata_enabled() {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("send_and_confirm failed: {}", client_err),
                    );
                }

                // Record failure event
                crate::events::record_transaction_event(
                    "unknown",
                    "failed",
                    false,
                    None,
                    None,
                    Some(&client_err),
                )
                .await;
                Err(ScreenerBotError::Blockchain(
                    crate::errors::BlockchainError::TransactionDropped {
                        signature: "unknown".to_string(),
                        reason: format!("send_and_confirm_transaction failed: {}", client_err),
                        fee_paid: None,
                        attempts: 1,
                    },
                ))
            }
            Err(e) => Err(ScreenerBotError::Network(
                crate::errors::NetworkError::Generic {
                    message: format!("Join error in send_and_confirm: {}", e),
                },
            )),
        }
    }

    /// Gets all token accounts for a wallet (both SPL Token and Token-2022)
    pub async fn get_all_token_accounts(
        &self,
        wallet_address: &str,
    ) -> Result<Vec<TokenAccountInfo>, ScreenerBotError> {
        let spl_token_payload = serde_json::json!({
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

        let token_2022_payload = serde_json::json!({
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

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "ATA",
                &format!("Fetching token accounts from RPC: {}", current_url),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;

        // Record call in stats
        self.record_call("getTokenAccountsByOwner");

        // Process both SPL Token and Token-2022 accounts
        for payload in [&spl_token_payload, &token_2022_payload] {
            let is_token_2022 = payload == &token_2022_payload;
            let token_type = if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            };

            match client
                .post(&current_url)
                .header("Content-Type", "application/json")
                .json(payload)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(rpc_response) => {
                                if let Some(result) = rpc_response.get("result") {
                                    if let Some(value) = result.get("value") {
                                        if let Some(accounts) = value.as_array() {
                                            for account in accounts {
                                                if let Some(parsed_info) =
                                                    extract_token_account_info(
                                                        account,
                                                        is_token_2022,
                                                    )
                                                {
                                                    all_accounts.push(parsed_info);
                                                }
                                            }

                                            if is_debug_rpc_enabled() {
                                                log(
                                                    LogTag::Rpc,
                                                    "ATA",
                                                    &format!(
                                                        "Found {} {} accounts for wallet",
                                                        accounts.len(),
                                                        token_type
                                                    ),
                                                );
                                            }
                                        }
                                    } else {
                                        log(
                                            LogTag::Rpc,
                                            "WARN",
                                            &format!("No value field in {} response", token_type),
                                        );
                                    }
                                } else {
                                    log(
                                        LogTag::Rpc,
                                        "WARN",
                                        &format!("No result field in {} response", token_type),
                                    );
                                }
                            }
                            Err(e) => {
                                log(
                                    LogTag::Rpc,
                                    "WARN",
                                    &format!("Failed to parse {} response: {}", token_type, e),
                                );
                            }
                        }
                    } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        // Record 429 error for adaptive rate limiting
                        self.record_429_error(Some(&current_url));
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!(
                                "Rate limited on RPC {} for {} accounts",
                                current_url, token_type
                            ),
                        );
                    } else {
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!(
                                "RPC error status {} for {}: {}",
                                response.status(),
                                token_type,
                                current_url
                            ),
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!(
                            "Failed to connect to RPC {} for {} accounts: {}",
                            current_url, token_type, e
                        ),
                    );
                }
            }
        }

        // Record successful call if we got any accounts
        if !all_accounts.is_empty() {
            self.record_success(Some(&current_url));
        }

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "ATA",
                &format!(
                    "Found {} total token accounts for wallet",
                    all_accounts.len()
                ),
            );
        }

        Ok(all_accounts)
    }

    /// Checks if a token account (not mint) is a Token-2022 account by checking the account owner using round-robin RPC rotation
    pub async fn is_token_account_token_2022(
        &self,
        token_account: &str,
    ) -> Result<bool, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
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

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Checking if token account {} is Token-2022 using RPC: {}",
                    token_account, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getAccountInfo");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(owner) = value.get("owner") {
                                    if let Some(owner_str) = owner.as_str() {
                                        let is_token_2022 = owner_str
                                            == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

                                        // Record successful call
                                        self.record_success(Some(&current_url));

                                        if is_debug_rpc_enabled() {
                                            log(
                                                LogTag::Rpc,
                                                "SUCCESS",
                                                &format!(
                                                    "Token account {} is {} (owner: {}) from RPC: {}",
                                                    token_account,
                                                    if is_token_2022 {
                                                        "Token-2022"
                                                    } else {
                                                        "SPL Token"
                                                    },
                                                    owner_str,
                                                    current_url
                                                )
                                            );
                                        }

                                        return Ok(is_token_2022);
                                    }
                                }
                            }
                        }

                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!(
                                "Could not determine owner for token account {} on RPC {}",
                                token_account, current_url
                            ),
                        );
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for token account info", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "HTTP error {} from RPC {} for token account info",
                            response.status(),
                            current_url
                        ),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for token account info", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "Failed to get token account info from RPC {}: {}",
                            current_url, e
                        ),
                    );

                    // Default to false for non-rate-limit errors
                    return Ok(false);
                }
            }
        }

        // Default to false if we can't determine
        log(
            LogTag::Rpc,
            "WARN",
            &format!(
                "Defaulting to SPL Token for account {} due to inconclusive response from RPC {}",
                token_account, current_url
            ),
        );

        Ok(false)
    }

    /// Checks if a mint is a Token-2022 mint by checking its owner program using round-robin RPC rotation
    pub async fn is_token_2022_mint(&self, mint: &str) -> Result<bool, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
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

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Checking if mint {} is Token-2022 using RPC: {}",
                    mint, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getAccountInfo");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(owner) = value.get("owner") {
                                    if let Some(owner_str) = owner.as_str() {
                                        // Record successful call
                                        self.record_success(Some(&current_url));

                                        // Token Extensions Program ID (Token-2022)
                                        let is_token_2022 = owner_str
                                            == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

                                        if is_debug_rpc_enabled() {
                                            log(
                                                LogTag::Rpc,
                                                "SUCCESS",
                                                &format!(
                                                    "Mint {} owner: {} (Token-2022: {}) from RPC: {}",
                                                    mint,
                                                    owner_str,
                                                    is_token_2022,
                                                    current_url
                                                )
                                            );
                                        }

                                        return Ok(is_token_2022);
                                    }
                                }
                            }
                        }
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for mint check", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "HTTP error {} from RPC {} for mint check",
                            response.status(),
                            current_url
                        ),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for mint check", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get mint info from RPC {}: {}", current_url, e),
                    );
                }
            }
        }

        // Default to false if we can't determine
        Ok(false)
    }

    /// Get transaction details using round-robin RPC rotation
    pub async fn get_transaction_details(
        &self,
        transaction_signature: &str,
    ) -> Result<TransactionDetails, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [
                transaction_signature,
                { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0 }
            ]
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();
        let start = std::time::Instant::now();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "TX_DETAILS",
                &format!(
                    "Fetching transaction details for {} from RPC: {}",
                    transaction_signature, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getTransaction");

        let client = reqwest::Client::new();

        let response = client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
            .map_err(|e| {
                log(
                    LogTag::Rpc,
                    "ERROR",
                    &format!(
                        "Failed to connect to RPC {} for transaction details: {}",
                        current_url, e
                    ),
                );
                ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to connect to RPC: {}", e),
                })
            })?;

        if response.status().is_success() {
            if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                if let Some(error) = rpc_response.get("error") {
                    let error_msg = format!("RPC error: {:?}", error);
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("RPC error from {}: {}", current_url, error_msg),
                    );

                    return Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
                        provider_name: current_url,
                        message: error_msg,
                    }));
                }

                if let Some(result) = rpc_response.get("result") {
                    if result.is_null() {
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!(
                                "Transaction {} not found on RPC {}",
                                transaction_signature, current_url
                            ),
                        );

                        return Err(ScreenerBotError::Blockchain(
                            crate::errors::BlockchainError::TransactionNotFound {
                                signature: transaction_signature.to_string(),
                                commitment_level: "confirmed".to_string(),
                                searched_endpoints: vec![current_url],
                                age_seconds: None,
                            },
                        ));
                    }

                    // Parse transaction details manually from RPC response
                    let slot = result.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
                    let block_time = result.get("blockTime").and_then(|bt| bt.as_i64());

                    let transaction_data = if let Some(transaction) = result.get("transaction") {
                        TransactionData {
                            message: transaction
                                .get("message")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null),
                            signatures: transaction
                                .get("signatures")
                                .and_then(|s| s.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default(),
                        }
                    } else {
                        TransactionData {
                            message: serde_json::Value::Null,
                            signatures: vec![],
                        }
                    };

                    let meta = result.get("meta").map(|meta_value| TransactionMeta {
                        err: meta_value.get("err").cloned(),
                        fee: meta_value.get("fee").and_then(|f| f.as_u64()).unwrap_or(0),
                        compute_units_consumed: meta_value
                            .get("computeUnitsConsumed")
                            .and_then(|v| v.as_u64()),
                        pre_balances: meta_value
                            .get("preBalances")
                            .and_then(|pb| pb.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
                            .unwrap_or_default(),
                        post_balances: meta_value
                            .get("postBalances")
                            .and_then(|pb| pb.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
                            .unwrap_or_default(),
                        pre_token_balances: meta_value
                            .get("preTokenBalances")
                            .and_then(|ptb| serde_json::from_value(ptb.clone()).ok()),
                        post_token_balances: meta_value
                            .get("postTokenBalances")
                            .and_then(|ptb| serde_json::from_value(ptb.clone()).ok()),
                        log_messages: meta_value
                            .get("logMessages")
                            .and_then(|lm| lm.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            }),
                        inner_instructions: meta_value
                            .get("innerInstructions")
                            .and_then(|ii| serde_json::from_value(ii.clone()).ok()),
                    });

                    let transaction_details = TransactionDetails {
                        slot,
                        transaction: transaction_data,
                        meta,
                        block_time,
                    };

                    // Record successful call with timing
                    let duration_ms = start.elapsed().as_millis() as u64;
                    self.record_response_time(&current_url, "getTransaction", duration_ms);
                    self.record_success(Some(&current_url));

                    if is_debug_rpc_enabled() {
                        log(
                            LogTag::Rpc,
                            "SUCCESS",
                            &format!(
                                "Successfully fetched transaction details for {} from RPC: {}",
                                transaction_signature, current_url
                            ),
                        );
                    }

                    return Ok(transaction_details);
                }
            }
        } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            self.record_429_error(Some(&current_url));
            log(
                LogTag::Rpc,
                "WARN",
                &format!(
                    "Rate limited on RPC {} for transaction details",
                    current_url
                ),
            );
        } else {
            log(
                LogTag::Rpc,
                "ERROR",
                &format!(
                    "HTTP error {} from RPC {} for transaction details",
                    response.status(),
                    current_url
                ),
            );
        }

        // Record error for failed call
        self.record_error(&current_url, "getTransaction");

        Err(ScreenerBotError::RpcProvider(
            RpcProviderError::MalformedResponse {
                provider_name: current_url,
                endpoint: "getTransaction".to_string(),
                response_body: "Invalid RPC response for transaction details".to_string(),
            },
        ))
    }

    /// Gets the associated token account address for a wallet and mint using round-robin RPC rotation
    pub async fn get_associated_token_account(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> Result<String, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
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

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "ATA",
                &format!(
                    "Fetching associated token account for wallet {} mint {} from RPC: {}",
                    wallet_address, mint, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getTokenAccountsByOwner");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(accounts) = value.as_array() {
                                    if !accounts.is_empty() {
                                        if let Some(account) = accounts.first() {
                                            if let Some(pubkey) = account.get("pubkey") {
                                                if let Some(pubkey_str) = pubkey.as_str() {
                                                    // Record successful call
                                                    self.record_success(Some(&current_url));

                                                    if is_debug_rpc_enabled() {
                                                        log(
                                                            LogTag::Rpc,
                                                            "SUCCESS",
                                                            &format!(
                                                                "Found ATA {} for wallet {} mint {} from RPC: {}",
                                                                pubkey_str,
                                                                wallet_address,
                                                                mint,
                                                                current_url
                                                            )
                                                        );
                                                    }

                                                    return Ok(pubkey_str.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // If no accounts found, this is not an error - return the standard error
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!(
                                "No associated token account found for wallet {} mint {} on RPC {}",
                                wallet_address, mint, current_url
                            ),
                        );

                        return Err(ScreenerBotError::Data(DataError::Generic {
                            message: format!("No associated token account found for mint {}", mint),
                        }));
                    }
                } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!(
                            "Rate limited on RPC {} for associated token account",
                            current_url
                        ),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "HTTP error {} from RPC {} for associated token account",
                            response.status(),
                            current_url
                        ),
                    );
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!(
                            "Rate limited on RPC {} for associated token account",
                            current_url
                        ),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "Failed to get associated token account from RPC {}: {}",
                            current_url, e
                        ),
                    );

                    return Err(ScreenerBotError::Network(NetworkError::Generic {
                        message: format!("Failed to get associated token account from RPC: {}", e),
                    }));
                }
            }
        }

        Err(ScreenerBotError::Blockchain(
            crate::errors::BlockchainError::AccountNotFound {
                pubkey: format!("ATA for wallet {} mint {}", wallet_address, mint),
                context: "get_associated_token_account".to_string(),
                rpc_endpoint: Some(current_url),
            },
        ))
    }

    /// Helper method to get signature status using getSignatureStatuses with round-robin RPC rotation
    async fn get_signature_status(
        &self,
        signature: &str,
    ) -> Result<Option<SignatureStatusData>, ScreenerBotError> {
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

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_START",
                &format!(
                    "🔍 Checking signature status for {} using RPC: {}",
                    &signature[..8],
                    current_url
                ),
            );

            log(
                LogTag::Rpc,
                "STATUS_DEBUG_PAYLOAD",
                &format!(
                    "📤 Request payload for {}: {}",
                    &signature[..8],
                    serde_json::to_string(&rpc_payload)
                        .unwrap_or_else(|_| "Failed to serialize".to_string())
                ),
            );
        }

        log(
            LogTag::Rpc,
            "STATUS_API_CALL_START",
            &format!(
                "🌐 Making getSignatureStatuses API call for {} to {}",
                &signature[..8],
                current_url
            ),
        );

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getSignatureStatuses");

        let client = reqwest::Client::new();

        let response = client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                log(
                    LogTag::Rpc,
                    "STATUS_API_NETWORK_ERROR",
                    &format!(
                        "🔌 Network error in getSignatureStatuses for {} on RPC {}: {}",
                        &signature[..8],
                        current_url,
                        e
                    ),
                );
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Rpc,
                        "STATUS_DEBUG_NETWORK_ERROR_DETAIL",
                        &format!("❌ Detailed network error for {}: {}", &signature[..8], e),
                    );
                }
                ScreenerBotError::Network(crate::errors::NetworkError::Generic {
                    message: format!("Network error in getSignatureStatuses: {}", e),
                })
            })?;

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_HTTP_STATUS",
                &format!(
                    "📡 HTTP status for signature status check {}: {}",
                    &signature[..8],
                    response.status()
                ),
            );
        }

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                self.record_429_error(Some(&current_url));
                log(
                    LogTag::Rpc,
                    "WARN",
                    &format!("Rate limited on RPC {} for signature status", current_url),
                );
            } else {
                log(
                    LogTag::Rpc,
                    "STATUS_API_HTTP_ERROR",
                    &format!(
                        "📉 HTTP error in getSignatureStatuses for {}: {}",
                        &signature[..8],
                        response.status()
                    ),
                );
            }
            return Err(ScreenerBotError::api_error(format!(
                "RPC error: {}",
                response.status()
            )));
        }

        log(
            LogTag::Rpc,
            "STATUS_API_RESPONSE_OK",
            &format!(
                "✅ Received HTTP 200 response from getSignatureStatuses for {}",
                &signature[..8]
            ),
        );

        let response_text = response.text().await.map_err(|e| {
            if is_debug_transactions_enabled() {
                log(
                    LogTag::Rpc,
                    "STATUS_DEBUG_TEXT_ERROR",
                    &format!(
                        "❌ Failed to get response text for {}: {}",
                        &signature[..8],
                        e
                    ),
                );
            }
            ScreenerBotError::Network(crate::errors::NetworkError::Generic {
                message: format!("Failed to get response text for signature status: {}", e),
            })
        })?;

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "STATUS_DEBUG_RAW_RESPONSE",
                &format!(
                    "📄 Raw response for {}: {}",
                    &signature[..8],
                    &response_text
                ),
            );
        }

        let rpc_response: SignatureStatusResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                log(
                    LogTag::Rpc,
                    "STATUS_API_PARSE_ERROR",
                    &format!(
                        "🔍 Failed to parse getSignatureStatuses response for {}: {}",
                        &signature[..8],
                        e
                    ),
                );
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Rpc,
                        "STATUS_DEBUG_PARSE_ERROR_DETAIL",
                        &format!(
                            "❌ Parse error detail for {}: Response was: {}",
                            &signature[..8],
                            &response_text
                        ),
                    );
                }
                ScreenerBotError::invalid_response(format!(
                    "Failed to parse signature status: {}",
                    e
                ))
            })?;

        let result = rpc_response.result.value.into_iter().next().flatten();

        // Record successful call if we got a valid response
        if result.is_some() {
            self.record_success(Some(&current_url));
        }

        log(
            LogTag::Rpc,
            "STATUS_API_RESULT",
            &format!(
                "📊 getSignatureStatuses result for {}: {}",
                &signature[..8],
                result
                    .as_ref()
                    .map(|r| format!(
                        "confirmation_status={:?}, err={:?}",
                        r.confirmation_status, r.err
                    ))
                    .unwrap_or_else(|| "null".to_string())
            ),
        );

        if is_debug_transactions_enabled() {
            if result.is_none() {
                log(
                    LogTag::Rpc,
                    "STATUS_DEBUG_NULL_RESULT",
                    &format!(
                        "⚠️ Signature {} returned null status - transaction may not be visible on network yet",
                        &signature[..8]
                    )
                );
            } else if let Some(ref status) = result {
                log(
                    LogTag::Rpc,
                    "STATUS_DEBUG_FOUND",
                    &format!(
                        "✅ Found status for {}: confirmation={:?}, error={:?}",
                        &signature[..8],
                        status.confirmation_status,
                        status.err
                    ),
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
    pub async fn wait_for_signature_propagation(
        &self,
        signature: &str,
    ) -> Result<bool, ScreenerBotError> {
        // Extended timing for better reliability: 4 attempts at t=2,7,12,17 seconds
        const ATTEMPTS: u32 = 4;
        const FIRST_DELAY_SECS: u64 = 2; // Initial delay before first check
        const SLEEP_SECS: u64 = 5;

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "PROPAGATION_DEBUG_START",
                &format!(
                    "🚀 Starting propagation wait for signature {} with {} attempts starting after {}s delay",
                    &signature[..8],
                    ATTEMPTS,
                    FIRST_DELAY_SECS
                )
            );
        }

        log(
            LogTag::Rpc,
            "STATUS_PROPAGATION_WAIT_START",
            &format!(
                "🚦 Propagation wait start for {} ({} attempts with {}s initial delay)",
                &signature[..8],
                ATTEMPTS,
                FIRST_DELAY_SECS
            ),
        );

        // Initial delay to allow transaction to propagate
        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "PROPAGATION_DEBUG_INITIAL_DELAY",
                &format!(
                    "⏳ Waiting {}s before first propagation check for {}",
                    FIRST_DELAY_SECS,
                    &signature[..8]
                ),
            );
        }
        tokio::time::sleep(Duration::from_secs(FIRST_DELAY_SECS)).await;

        let start = Instant::now();
        for attempt in 1..=ATTEMPTS {
            if is_debug_transactions_enabled() {
                log(
                    LogTag::Rpc,
                    "PROPAGATION_DEBUG_ATTEMPT_START",
                    &format!(
                        "📋 Starting attempt {}/{} for signature {}",
                        attempt,
                        ATTEMPTS,
                        &signature[..8]
                    ),
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
                            &format!(
                                "✅ Propagation successful for {}: Found status after {:.2}s",
                                &signature[..8],
                                start.elapsed().as_secs_f64()
                            ),
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
                        ),
                    );

                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "PROPAGATION_DEBUG_NULL_ATTEMPT",
                            &format!(
                                "⏳ Attempt {}/{} returned null for {} - trying again in {}s",
                                attempt,
                                ATTEMPTS,
                                &signature[..8],
                                SLEEP_SECS
                            ),
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
                        ),
                    );

                    if is_debug_transactions_enabled() {
                        log(
                            LogTag::Rpc,
                            "PROPAGATION_DEBUG_ERROR_DETAIL",
                            &format!(
                                "❌ Detailed error on attempt {}/{} for {}: {}",
                                attempt,
                                ATTEMPTS,
                                &signature[..8],
                                e
                            ),
                        );
                    }
                }
            }

            if attempt < ATTEMPTS {
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Rpc,
                        "PROPAGATION_DEBUG_SLEEP",
                        &format!(
                            "😴 Sleeping {}s before next attempt for {}",
                            SLEEP_SECS,
                            &signature[..8]
                        ),
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
                &signature[..8],
                ATTEMPTS,
                start.elapsed().as_secs_f64() as u64
            ),
        );

        if is_debug_transactions_enabled() {
            log(
                LogTag::Rpc,
                "PROPAGATION_DEBUG_FAILED",
                &format!(
                    "❌ Transaction {} failed to propagate - likely dropped by network",
                    &signature[..8]
                ),
            );
        }

        Ok(false)
    }

    /// Get wallet signatures using round-robin RPC rotation
    /// This is optimized for checking how many new transactions exist without heavy data transfer
    pub async fn get_wallet_signatures_main_rpc(
        &self,
        wallet_pubkey: &Pubkey,
        limit: usize,
        before: Option<&str>,
    ) -> Result<
        Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>,
        ScreenerBotError,
    > {
        let config = solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
            before: before.and_then(|s| solana_sdk::signature::Signature::from_str(s).ok()),
            until: None,
            limit: Some(limit),
            commitment: Some(CommitmentConfig::confirmed()),
        };

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "SIGNATURES",
                &format!("Fetching {} signatures from RPC: {}", limit, current_url),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("get_signatures_for_address");

        // Create client for the current URL
        let client = SolanaRpcClient::new_with_commitment(
            current_url.clone(),
            CommitmentConfig::confirmed(),
        );

        match client.get_signatures_for_address_with_config(wallet_pubkey, config) {
            Ok(signatures) => {
                // Record successful call
                self.record_success(Some(&current_url));

                log(
                    LogTag::Rpc,
                    "SUCCESS",
                    &format!(
                        "Retrieved {} signatures from {}",
                        signatures.len(),
                        current_url
                    ),
                );

                Ok(signatures)
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for signatures", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get signatures from RPC {}: {}", current_url, e),
                    );
                }

                Err(ScreenerBotError::api_error(format!(
                    "Failed to get signatures from RPC: {}",
                    e
                )))
            }
        }
    }

    /// Get transaction details using round-robin RPC rotation
    /// Supports both single transaction and batch processing
    /// Get multiple transaction details using round-robin RPC rotation (batch processing)
    pub async fn get_transaction_details_batch(
        &self,
        transaction_signatures: &[String],
    ) -> Result<Vec<(String, TransactionDetails)>, ScreenerBotError> {
        let mut results = Vec::new();

        if transaction_signatures.is_empty() {
            return Ok(results);
        }

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "TX_BATCH",
                &format!(
                    "Fetching {} transaction details in batch",
                    transaction_signatures.len()
                ),
            );
        }

        // Process transactions individually but efficiently
        for signature in transaction_signatures {
            match self.get_transaction_details(signature).await {
                Ok(tx_details) => {
                    results.push((signature.clone(), tx_details));

                    if is_debug_rpc_enabled() {
                        log(
                            LogTag::Rpc,
                            "TX_BATCH_SUCCESS",
                            &format!("Retrieved transaction details for {}", signature),
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "TX_BATCH_ERROR",
                        &format!("Failed to get transaction details for {}: {}", signature, e),
                    );
                    // Continue with other transactions even if one fails
                }
            }

            // Small delay between requests to avoid overwhelming RPC
            if transaction_signatures.len() > 1 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }

        log(
            LogTag::Rpc,
            "TX_BATCH_COMPLETE",
            &format!(
                "Batch completed: {}/{} transactions retrieved",
                results.len(),
                transaction_signatures.len()
            ),
        );

        Ok(results)
    }

    /// Get program accounts with filters using round-robin RPC rotation
    pub async fn get_program_accounts(
        &self,
        program_id: &str,
        filters: Option<serde_json::Value>,
        encoding: Option<&str>,
        timeout_seconds: Option<u64>,
    ) -> Result<Vec<serde_json::Value>, ScreenerBotError> {
        let mut params = vec![serde_json::Value::String(program_id.to_string())];

        // Build config object
        let mut config = serde_json::Map::new();
        config.insert(
            "encoding".to_string(),
            serde_json::Value::String(encoding.unwrap_or("jsonParsed").to_string()),
        );

        if let Some(filters_value) = filters {
            config.insert("filters".to_string(), filters_value);
        }

        params.push(serde_json::Value::Object(config));

        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getProgramAccounts",
            "params": params
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Getting program accounts for program: {} from RPC: {}",
                    program_id, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getProgramAccounts");

        // Create client with extended timeout for large queries
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                timeout_seconds.unwrap_or(60),
            ))
            .build()
            .map_err(|e| {
                ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to create HTTP client: {}", e),
                })
            })?;

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        self.record_429_error(Some(&current_url));
                        return Err(ScreenerBotError::RpcProvider(
                            RpcProviderError::RateLimitExceeded {
                                provider_name: current_url.clone(),
                                limit_type: "requests_per_second".to_string(),
                                reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                            },
                        ));
                    } else if status == reqwest::StatusCode::REQUEST_TIMEOUT {
                        return Err(ScreenerBotError::Network(NetworkError::ConnectionTimeout {
                            endpoint: current_url.clone(),
                            timeout_ms: timeout_seconds.unwrap_or(60) * 1000,
                        }));
                    } else {
                        return Err(ScreenerBotError::Network(NetworkError::HttpStatusError {
                            endpoint: current_url.clone(),
                            status: status.as_u16(),
                            body: Some(error_text),
                        }));
                    }
                }

                let rpc_response = response.json::<serde_json::Value>().await.map_err(|e| {
                    ScreenerBotError::Data(DataError::ParseError {
                        data_type: "RPC response".to_string(),
                        error: e.to_string(),
                    })
                })?;

                // Check for RPC-level errors
                if let Some(error) = rpc_response.get("error") {
                    if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
                        let error_msg = message.to_lowercase();

                        if error_msg.contains("timeout") || error_msg.contains("too many accounts")
                        {
                            return Err(ScreenerBotError::Network(
                                NetworkError::ConnectionTimeout {
                                    endpoint: current_url.clone(),
                                    timeout_ms: timeout_seconds.unwrap_or(60) * 1000,
                                },
                            ));
                        } else if error_msg.contains("rate limit") || error_msg.contains("429") {
                            self.record_429_error(Some(&current_url));
                            return Err(ScreenerBotError::RpcProvider(
                                RpcProviderError::RateLimitExceeded {
                                    provider_name: current_url.clone(),
                                    limit_type: "requests_per_second".to_string(),
                                    reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                                },
                            ));
                        } else {
                            return Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
                                provider_name: current_url.clone(),
                                message: format!("RPC error: {}", message),
                            }));
                        }
                    }
                }

                if let Some(result) = rpc_response.get("result") {
                    if let Some(accounts) = result.as_array() {
                        // Record successful call
                        self.record_success(Some(&current_url));

                        if is_debug_rpc_enabled() {
                            log(
                                LogTag::Rpc,
                                "SUCCESS",
                                &format!(
                                    "Retrieved {} program accounts from RPC: {}",
                                    accounts.len(),
                                    current_url
                                ),
                            );
                        }

                        return Ok(accounts.clone());
                    }
                }

                Err(ScreenerBotError::Data(DataError::ParseError {
                    data_type: "program accounts".to_string(),
                    error: "No accounts found or invalid response format".to_string(),
                }))
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for program accounts", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "Failed to get program accounts from RPC {}: {}",
                            current_url, e
                        ),
                    );
                }

                Err(ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to get program accounts from RPC: {}", e),
                }))
            }
        }
    }

    /// Enhanced get program accounts with dataSlice support for efficient token account counting
    /// This method supports dataSlice to minimize data transfer when only counting accounts
    pub async fn get_program_accounts_with_dateslice(
        &self,
        program_id: &str,
        filters: Option<serde_json::Value>,
        encoding: Option<&str>,
        data_slice: Option<serde_json::Value>,
        timeout_seconds: Option<u64>,
    ) -> Result<Vec<serde_json::Value>, ScreenerBotError> {
        let mut params = vec![serde_json::Value::String(program_id.to_string())];

        // Build config object
        let mut config = serde_json::Map::new();
        config.insert(
            "encoding".to_string(),
            serde_json::Value::String(encoding.unwrap_or("base64").to_string()),
        );

        if let Some(filters_value) = filters {
            config.insert("filters".to_string(), filters_value);
        }

        // Add dataSlice if provided (this is the key optimization)
        if let Some(slice_value) = data_slice {
            config.insert("dataSlice".to_string(), slice_value);
        }

        params.push(serde_json::Value::Object(config));

        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getProgramAccounts",
            "params": params
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Getting program accounts with dataSlice for program: {} from RPC: {}",
                    program_id, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getProgramAccounts");

        // Create client with timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                timeout_seconds.unwrap_or(30),
            ))
            .build()
            .map_err(|e| {
                ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to create HTTP client: {}", e),
                })
            })?;

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        self.record_429_error(Some(&current_url));
                        return Err(ScreenerBotError::RpcProvider(
                            RpcProviderError::RateLimitExceeded {
                                provider_name: current_url.clone(),
                                limit_type: "requests_per_second".to_string(),
                                reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                            },
                        ));
                    } else if status == reqwest::StatusCode::REQUEST_TIMEOUT {
                        return Err(ScreenerBotError::Network(NetworkError::ConnectionTimeout {
                            endpoint: current_url.clone(),
                            timeout_ms: timeout_seconds.unwrap_or(30) * 1000,
                        }));
                    } else {
                        return Err(ScreenerBotError::Network(NetworkError::HttpStatusError {
                            endpoint: current_url.clone(),
                            status: status.as_u16(),
                            body: Some(error_text),
                        }));
                    }
                }

                let rpc_response = response.json::<serde_json::Value>().await.map_err(|e| {
                    ScreenerBotError::Data(DataError::ParseError {
                        data_type: "RPC response".to_string(),
                        error: e.to_string(),
                    })
                })?;

                if let Some(error) = rpc_response.get("error") {
                    if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
                        if Self::is_rate_limit_error(message) {
                            self.record_429_error(Some(&current_url));
                            return Err(ScreenerBotError::RpcProvider(
                                RpcProviderError::RateLimitExceeded {
                                    provider_name: current_url.clone(),
                                    limit_type: "requests_per_second".to_string(),
                                    reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                                },
                            ));
                        } else {
                            return Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
                                provider_name: current_url.clone(),
                                message: format!("RPC error: {}", message),
                            }));
                        }
                    }
                }

                if let Some(result) = rpc_response.get("result") {
                    if let Some(accounts) = result.as_array() {
                        // Record successful call
                        self.record_success(Some(&current_url));

                        if is_debug_rpc_enabled() {
                            log(
                                LogTag::Rpc,
                                "SUCCESS",
                                &format!(
                                    "Retrieved {} program accounts (with dataSlice) from RPC: {}",
                                    accounts.len(),
                                    current_url
                                ),
                            );
                        }

                        return Ok(accounts.clone());
                    }
                }

                Err(ScreenerBotError::Data(DataError::ParseError {
                    data_type: "program accounts".to_string(),
                    error: "No accounts found or invalid response format".to_string(),
                }))
            }
            Err(e) => {
                let error_msg = e.to_string();

                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for program accounts", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "Failed to get program accounts from RPC {}: {}",
                            current_url, e
                        ),
                    );
                }

                Err(ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to get program accounts from RPC: {}", e),
                }))
            }
        }
    }

    /// Enhanced getProgramAccountsV2 with cursor-based pagination for large program account sets
    /// This method is required for handling large programs like Pump.fun AMM that exceed regular RPC limits
    ///
    /// Parameters:
    /// - program_id: The program ID to query accounts for
    /// - filters: Optional filters to apply
    /// - encoding: Data encoding format (default: "base64")
    /// - data_slice: Optional data slice configuration
    /// - limit: Maximum accounts per request (1-10,000, optimal 1,000-5,000)
    /// - pagination_key: Cursor for pagination (use None for first request)
    /// - changed_since_slot: Optional slot for incremental updates
    /// - timeout_seconds: Request timeout
    pub async fn get_program_accounts_v2(
        &self,
        program_id: &str,
        filters: Option<serde_json::Value>,
        encoding: Option<&str>,
        data_slice: Option<serde_json::Value>,
        limit: Option<u32>,
        pagination_key: Option<String>,
        changed_since_slot: Option<u64>,
        timeout_seconds: Option<u64>,
    ) -> Result<PaginatedAccountsResponse, ScreenerBotError> {
        let mut params = vec![serde_json::Value::String(program_id.to_string())];

        // Build config object for getProgramAccountsV2
        let mut config = serde_json::Map::new();
        config.insert(
            "encoding".to_string(),
            serde_json::Value::String(encoding.unwrap_or("base64").to_string()),
        );

        if let Some(filters_value) = filters {
            config.insert("filters".to_string(), filters_value);
        }

        // Add dataSlice if provided
        if let Some(slice_value) = data_slice {
            config.insert("dataSlice".to_string(), slice_value);
        }

        // Add limit (default to 1000 for optimal performance)
        config.insert(
            "limit".to_string(),
            serde_json::Value::Number(serde_json::Number::from(limit.unwrap_or(1000))),
        );

        // Add pagination key if provided
        if let Some(key) = pagination_key {
            config.insert("paginationKey".to_string(), serde_json::Value::String(key));
        }

        // Add changedSinceSlot if provided
        if let Some(slot) = changed_since_slot {
            config.insert(
                "changedSinceSlot".to_string(),
                serde_json::Value::Number(serde_json::Number::from(slot)),
            );
        }

        params.push(serde_json::Value::Object(config));

        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getProgramAccountsV2",
            "params": params
        });

        // Use round-robin RPC rotation
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "DEBUG",
                &format!(
                    "Getting program accounts V2 (paginated) for program: {} limit: {} from RPC: {}",
                    program_id,
                    limit.unwrap_or(1000),
                    current_url
                )
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getProgramAccountsV2");

        // Create client with extended timeout for pagination
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                timeout_seconds.unwrap_or(120),
            ))
            .build()
            .map_err(|e| {
                ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to create HTTP client: {}", e),
                })
            })?;

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());

                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        self.record_429_error(Some(&current_url));
                        return Err(ScreenerBotError::RpcProvider(
                            RpcProviderError::RateLimitExceeded {
                                provider_name: current_url.clone(),
                                limit_type: "requests_per_second".to_string(),
                                reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                            },
                        ));
                    } else if status == reqwest::StatusCode::REQUEST_TIMEOUT {
                        return Err(ScreenerBotError::Network(NetworkError::ConnectionTimeout {
                            endpoint: current_url.clone(),
                            timeout_ms: timeout_seconds.unwrap_or(120) * 1000,
                        }));
                    } else {
                        return Err(ScreenerBotError::Network(NetworkError::HttpStatusError {
                            endpoint: current_url.clone(),
                            status: status.as_u16(),
                            body: Some(error_text),
                        }));
                    }
                }

                let rpc_response = response.json::<serde_json::Value>().await.map_err(|e| {
                    ScreenerBotError::Data(DataError::ParseError {
                        data_type: "RPC V2 response".to_string(),
                        error: e.to_string(),
                    })
                })?;

                // Check for RPC-level errors
                if let Some(error) = rpc_response.get("error") {
                    if let Some(message) = error.get("message").and_then(|m| m.as_str()) {
                        let error_msg = message.to_lowercase();

                        if error_msg.contains("timeout") || error_msg.contains("too many accounts")
                        {
                            return Err(ScreenerBotError::Network(
                                NetworkError::ConnectionTimeout {
                                    endpoint: current_url.clone(),
                                    timeout_ms: timeout_seconds.unwrap_or(120) * 1000,
                                },
                            ));
                        } else if error_msg.contains("rate limit") || error_msg.contains("429") {
                            self.record_429_error(Some(&current_url));
                            return Err(ScreenerBotError::RpcProvider(
                                RpcProviderError::RateLimitExceeded {
                                    provider_name: current_url.clone(),
                                    limit_type: "requests_per_second".to_string(),
                                    reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
                                },
                            ));
                        } else {
                            return Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
                                provider_name: current_url.clone(),
                                message: format!("RPC V2 error: {}", message),
                            }));
                        }
                    }
                }

                if let Some(result) = rpc_response.get("result") {
                    // Parse accounts array
                    let accounts = result
                        .get("accounts")
                        .and_then(|a| a.as_array())
                        .cloned()
                        .unwrap_or_default();

                    // Extract pagination key (can be null)
                    let next_pagination_key = result
                        .get("paginationKey")
                        .and_then(|k| k.as_str())
                        .map(|s| s.to_string());

                    // Record successful call
                    self.record_success(Some(&current_url));

                    if is_debug_rpc_enabled() {
                        log(
                            LogTag::Rpc,
                            "SUCCESS",
                            &format!(
                                "Retrieved {} program accounts V2 from RPC: {} (hasMore: {})",
                                accounts.len(),
                                current_url,
                                next_pagination_key.is_some()
                            ),
                        );
                    }

                    return Ok(PaginatedAccountsResponse {
                        accounts,
                        pagination_key: next_pagination_key,
                    });
                }

                Err(ScreenerBotError::Data(DataError::ParseError {
                    data_type: "program accounts V2".to_string(),
                    error: "No accounts found or invalid response format".to_string(),
                }))
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!(
                            "Rate limited on RPC {} for program accounts V2",
                            current_url
                        ),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!(
                            "Failed to get program accounts V2 from RPC {}: {}",
                            current_url, e
                        ),
                    );
                }

                Err(ScreenerBotError::Network(NetworkError::Generic {
                    message: format!("Failed to get program accounts V2 from RPC: {}", e),
                }))
            }
        }
    }

    /// Fetch all program accounts using getProgramAccountsV2 pagination
    /// This method handles pagination automatically and returns all accounts
    /// Use this for complete data collection when you need all accounts for a program
    pub async fn get_all_program_accounts_v2(
        &self,
        program_id: &str,
        filters: Option<serde_json::Value>,
        encoding: Option<&str>,
        data_slice: Option<serde_json::Value>,
        batch_size: Option<u32>,
        timeout_seconds: Option<u64>,
    ) -> Result<Vec<serde_json::Value>, ScreenerBotError> {
        let mut all_accounts = Vec::new();
        let mut pagination_key: Option<String> = None;
        let batch_size = batch_size.unwrap_or(2000); // Optimal batch size

        loop {
            let response = self
                .get_program_accounts_v2(
                    program_id,
                    filters.clone(),
                    encoding,
                    data_slice.clone(),
                    Some(batch_size),
                    pagination_key.clone(),
                    None, // changedSinceSlot
                    timeout_seconds,
                )
                .await?;

            // Add accounts from this batch
            all_accounts.extend(response.accounts);

            // Check if we have more pages
            if let Some(next_key) = response.pagination_key {
                pagination_key = Some(next_key);

                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "DEBUG",
                        &format!(
                            "Fetched {} accounts so far, continuing pagination...",
                            all_accounts.len()
                        ),
                    );
                }
            } else {
                // No more pages
                break;
            }
        }

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "SUCCESS",
                &format!(
                    "Completed pagination fetch: {} total accounts for program {}",
                    all_accounts.len(),
                    program_id
                ),
            );
        }

        Ok(all_accounts)
    }

    /// Get token holder count using Helius DAS API
    /// Uses the configured Helius RPC URLs with API keys from config.toml
    pub async fn get_token_holder_count(&self, mint_address: &str) -> Result<u32, String> {
        // Build DAS API request for getAssetOwners
        let request_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAssetOwners",
            "params": {
                "assetId": mint_address,
                "limit": 1, // Minimal limit to get total count
                "sortBy": "amount"
            }
        });

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "HELIUS_DAS",
                &format!("Requesting holder count for mint {}", mint_address),
            );
        }

        // Use round-robin RPC rotation to get the current Helius URL
        let current_url = self.rotate_to_next_url();

        // Apply rate limiting
        self.wait_for_rate_limit().await;

        // Create HTTP client for DAS API call
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Make the request to Helius DAS API
        let response = client
            .post(&current_url)
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send DAS request to {}: {}", current_url, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Helius DAS API returned error status: {} from {}",
                response.status(),
                current_url
            ));
        }

        // Parse the response
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse DAS response from {}: {}", current_url, e))?;

        // Extract the total count from the response
        if let Some(result) = response_json.get("result") {
            // If total is provided, use it directly
            if let Some(total) = result.get("total").and_then(|t| t.as_u64()) {
                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "HELIUS_DAS",
                        &format!(
                            "Helius DAS returned total holder count: {} for mint {}",
                            total, mint_address
                        ),
                    );
                }
                return Ok(total as u32);
            }

            // If no total, count the owners in the first response
            if let Some(owners) = result.get("owners").and_then(|o| o.as_array()) {
                let initial_count = owners.len() as u32;

                // If there's a cursor, we need to paginate to get the full count
                if result.get("cursor").is_some() {
                    log(
                        LogTag::Rpc,
                        "WARNING",
                        &format!(
                            "DAS API returned cursor but no total count for mint {}, using initial count: {}",
                            mint_address,
                            initial_count
                        )
                    );
                }

                if is_debug_rpc_enabled() {
                    log(
                        LogTag::Rpc,
                        "HELIUS_DAS",
                        &format!(
                            "Helius DAS counted {} holders for mint {}",
                            initial_count, mint_address
                        ),
                    );
                }
                return Ok(initial_count);
            }
        }

        Err(format!("Invalid DAS response format from {}", current_url))
    }

    /// Get detailed token holders using Helius DAS API with pagination support
    /// Returns up to 'limit' holders sorted by balance (largest first)
    pub async fn get_token_holders_detailed(
        &self,
        mint_address: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut all_holders = Vec::new();
        let mut cursor: Option<String> = None;
        let mut remaining_limit = limit;

        while remaining_limit > 0 {
            let page_limit = std::cmp::min(remaining_limit, 1000); // Max 1000 per request

            let request_payload = serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getAssetOwners",
                "params": {
                    "assetId": mint_address,
                    "limit": page_limit,
                    "cursor": cursor,
                    "sortBy": "amount"
                }
            });

            if is_debug_rpc_enabled() {
                log(
                    LogTag::Rpc,
                    "HELIUS_DAS",
                    &format!(
                        "Requesting {} holders from Helius DAS for mint {}",
                        page_limit, mint_address
                    ),
                );
            }

            // Use round-robin RPC rotation
            let current_url = self.rotate_to_next_url();

            // Apply rate limiting
            self.wait_for_rate_limit().await;

            // Create HTTP client
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

            // Make the request
            let response = client
                .post(&current_url)
                .json(&request_payload)
                .send()
                .await
                .map_err(|e| format!("Failed to send DAS request to {}: {}", current_url, e))?;

            if !response.status().is_success() {
                return Err(format!(
                    "Helius DAS API returned error status: {} from {}",
                    response.status(),
                    current_url
                ));
            }

            // Parse response
            let response_json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse DAS response from {}: {}", current_url, e))?;

            // Extract owners and cursor
            if let Some(result) = response_json.get("result") {
                if let Some(owners) = result.get("owners").and_then(|o| o.as_array()) {
                    // Add non-zero balance holders
                    for owner in owners {
                        if let Some(amount) = owner.get("amount").and_then(|a| a.as_str()) {
                            if amount != "0" {
                                all_holders.push(owner.clone());
                            }
                        }
                    }
                }

                // Update pagination cursor
                cursor = result
                    .get("cursor")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                remaining_limit = remaining_limit.saturating_sub(page_limit);

                // Break if no more pages
                if cursor.is_none() {
                    break;
                }
            } else {
                return Err(format!("Invalid DAS response format from {}", current_url));
            }
        }

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "HELIUS_DAS",
                &format!(
                    "Retrieved {} holders from Helius DAS for mint {}",
                    all_holders.len(),
                    mint_address
                ),
            );
        }

        Ok(all_holders)
    }

    /// Get mint account data to check authorities (minting, freeze, update metadata)
    /// Returns the raw mint account data for authority parsing using round-robin RPC rotation
    pub async fn get_mint_account(
        &self,
        mint: &str,
    ) -> Result<serde_json::Value, ScreenerBotError> {
        let rpc_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                mint,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        });

        // Use round-robin RPC rotation - get next URL from client
        let current_url = self.rotate_to_next_url();

        if is_debug_rpc_enabled() {
            log(
                LogTag::Rpc,
                "INFO",
                &format!(
                    "Getting mint account data for: {} from RPC: {}",
                    mint, current_url
                ),
            );
        }

        // Apply rate limiting
        self.wait_for_rate_limit().await;
        self.record_call("getAccountInfo");

        let client = reqwest::Client::new();

        match client
            .post(&current_url)
            .header("Content-Type", "application/json")
            .json(&rpc_payload)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                        if let Some(result) = rpc_response.get("result") {
                            // Record successful call
                            self.record_success(Some(&current_url));

                            if is_debug_rpc_enabled() {
                                log(
                                    LogTag::Rpc,
                                    "SUCCESS",
                                    &format!(
                                        "Retrieved mint account data from RPC: {}",
                                        current_url
                                    ),
                                );
                            }

                            return Ok(result.clone());
                        }
                    }
                } else {
                    // Check for rate limiting
                    if Self::is_rate_limit_response(&response) {
                        self.record_429_error(Some(&current_url));
                        log(
                            LogTag::Rpc,
                            "WARN",
                            &format!("Rate limited on RPC {} for mint account", current_url),
                        );
                    } else {
                        log(
                            LogTag::Rpc,
                            "ERROR",
                            &format!(
                                "HTTP error {} from RPC {} for mint account",
                                response.status(),
                                current_url
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Check for rate limiting errors
                if Self::is_rate_limit_error(&error_msg) {
                    self.record_429_error(Some(&current_url));
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!("Rate limited on RPC {} for mint account", current_url),
                    );
                } else {
                    log(
                        LogTag::Rpc,
                        "ERROR",
                        &format!("Failed to get mint account from RPC {}: {}", current_url, e),
                    );
                }
            }
        }

        Err(ScreenerBotError::RpcProvider(RpcProviderError::Generic {
            provider_name: current_url,
            message: "Failed to get mint account from RPC endpoint".to_string(),
        }))
    }
}

/// Global RPC client instance
static mut GLOBAL_RPC_CLIENT: Option<RpcClient> = None;
static RPC_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global RPC client from configuration
pub fn init_rpc_client() -> Result<&'static RpcClient, String> {
    unsafe {
        let mut init_error: Option<String> = None;

        RPC_INIT.call_once(|| match RpcClient::from_config() {
            Ok(client) => {
                log(
                    LogTag::Rpc,
                    "SUCCESS",
                    "Global RPC client initialized from configuration",
                );
                GLOBAL_RPC_CLIENT = Some(client);
            }
            Err(e) => {
                init_error = Some(e.clone());
                log(
                    LogTag::Rpc,
                    "ERROR",
                    &format!("Failed to init RPC client from config: {}", e),
                );
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
    log(
        LogTag::Rpc,
        "START",
        "Starting RPC stats auto-save service (every 3 seconds)",
    );

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
    crate::utils::parse_pubkey_safe(address)
}

/// Backward compatibility structure for old config access patterns
#[derive(Debug, Clone)]
pub struct BackwardCompatibleConfig {
    pub main_wallet_private: String,
    pub rpc_url: String,
    pub rpc_url_premium: String,
    pub rpc_url_ws_premium: String,
    pub rpc_fallbacks: Vec<String>,
}

/// Extracts token account information from RPC response
fn extract_token_account_info(
    account: &serde_json::Value,
    is_token_2022: bool,
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
