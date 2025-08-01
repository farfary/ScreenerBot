/// Rugcheck API Integration Service
///
/// This module provides a background service for fetching and updating rugcheck data.
/// It handles rate limiting, database storage, and provides fresh data to the trading system.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_rugcheck_enabled;
use crate::tokens::cache::TokenDatabase;
use reqwest::Client;
use serde::{ Deserialize, Serialize, Deserializer };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::{ Notify, RwLock };
use tokio::time::interval;
use chrono::{ DateTime, Utc };

// ===== CUSTOM DESERIALIZERS FOR FLEXIBLE INTEGER/STRING HANDLING =====

/// Custom deserializer that accepts both integers and strings and converts them to String
fn deserialize_int_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{ self, Visitor };
    use std::fmt;

    struct IntOrStringVisitor;

    impl<'de> Visitor<'de> for IntOrStringVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an integer or string")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E> where E: de::Error {
            Ok(Some(value))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> where E: de::Error {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> where E: de::Error {
            Ok(None)
        }
    }

    deserializer.deserialize_any(IntOrStringVisitor)
}

// ===== RATE LIMITING CONSTANTS =====
const RUGCHECK_RATE_LIMIT_DELAY_MS: u64 = 1000; // 5 seconds between requests (more conservative for 429 errors)
const RUGCHECK_REQUEST_TIMEOUT_SECS: u64 = 45; // Increased timeout
const RUGCHECK_UPDATE_INTERVAL_SECS: u64 = 30; // 30 seconds - check for expired data and new tokens
const RUGCHECK_PRIORITY_UPDATE_INTERVAL_SECS: u64 = 60; // 1 minute for open positions
const RUGCHECK_DATA_EXPIRY_HOURS: u64 = 24; // 24 hours - when rugcheck data expires

// ===== RUGCHECK API RESPONSE STRUCTURES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckResponse {
    pub mint: String,
    #[serde(rename = "tokenProgram")]
    pub token_program: Option<String>,
    pub creator: Option<String>,
    #[serde(rename = "creatorBalance")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub creator_balance: Option<String>, // Changed from i64 to String to handle large numbers
    pub token: Option<TokenInfo>,
    #[serde(rename = "token_extensions")]
    pub token_extensions: Option<serde_json::Value>,
    #[serde(rename = "tokenMeta")]
    pub token_meta: Option<TokenMeta>,
    #[serde(rename = "topHolders")]
    pub top_holders: Option<Vec<Holder>>,
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: Option<serde_json::Value>,
    #[serde(rename = "mintAuthority")]
    pub mint_authority: Option<serde_json::Value>,
    pub risks: Option<Vec<Risk>>,
    pub score: Option<i32>,
    #[serde(rename = "score_normalised")]
    pub score_normalised: Option<i32>,
    #[serde(rename = "fileMeta")]
    pub file_meta: Option<FileMeta>,
    #[serde(rename = "lockerOwners")]
    pub locker_owners: Option<HashMap<String, serde_json::Value>>,
    pub lockers: Option<HashMap<String, serde_json::Value>>,
    pub markets: Option<Vec<Market>>,
    #[serde(rename = "totalMarketLiquidity")]
    pub total_market_liquidity: Option<f64>,
    #[serde(rename = "totalStableLiquidity")]
    pub total_stable_liquidity: Option<f64>,
    #[serde(rename = "totalLPProviders")]
    pub total_lp_providers: Option<i32>,
    #[serde(rename = "totalHolders")]
    pub total_holders: Option<i32>,
    pub price: Option<f64>,
    pub rugged: Option<bool>,
    #[serde(rename = "tokenType")]
    pub token_type: Option<String>,
    #[serde(rename = "transferFee")]
    pub transfer_fee: Option<TransferFee>,
    #[serde(rename = "knownAccounts")]
    pub known_accounts: Option<HashMap<String, KnownAccount>>,
    pub events: Option<Vec<Event>>,
    pub verification: Option<Verification>,
    #[serde(rename = "graphInsidersDetected")]
    pub graph_insiders_detected: Option<i32>,
    #[serde(rename = "insiderNetworks")]
    pub insider_networks: Option<serde_json::Value>,
    #[serde(rename = "detectedAt")]
    pub detected_at: Option<String>,
    #[serde(rename = "creatorTokens")]
    pub creator_tokens: Option<serde_json::Value>,
    pub launchpad: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    #[serde(rename = "mintAuthority")]
    pub mint_authority: Option<String>,
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub supply: Option<String>, // Changed from i64 to String to handle large numbers
    pub decimals: Option<i32>,
    #[serde(rename = "isInitialized")]
    pub is_initialized: Option<bool>,
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMeta {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub mutable: Option<bool>,
    #[serde(rename = "updateAuthority")]
    pub update_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holder {
    pub address: String,
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub amount: Option<String>, // Changed from i64 to String to handle large numbers
    pub decimals: Option<i32>,
    pub pct: Option<f64>,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
    pub owner: Option<String>,
    pub insider: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub name: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub score: Option<i32>,
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub description: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferFee {
    pub pct: Option<f64>,
    #[serde(rename = "maxAmount")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub max_amount: Option<String>, // Changed from i64 to String to handle large numbers
    pub authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownAccount {
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event: i32,
    #[serde(rename = "oldValue")]
    pub old_value: Option<String>,
    #[serde(rename = "newValue")]
    pub new_value: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub mint: Option<String>,
    pub payer: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub jup_verified: Option<bool>,
    pub jup_strict: Option<bool>,
    pub links: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub pubkey: Option<String>,
    #[serde(rename = "marketType")]
    pub market_type: Option<String>,
    #[serde(rename = "mintA")]
    pub mint_a: Option<String>,
    #[serde(rename = "mintB")]
    pub mint_b: Option<String>,
    #[serde(rename = "mintLP")]
    pub mint_lp: Option<String>,
    #[serde(rename = "liquidityA")]
    pub liquidity_a: Option<String>,
    #[serde(rename = "liquidityB")]
    pub liquidity_b: Option<String>,
    pub lp: Option<LiquidityPool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    #[serde(rename = "baseMint")]
    pub base_mint: Option<String>,
    #[serde(rename = "quoteMint")]
    pub quote_mint: Option<String>,
    #[serde(rename = "lpMint")]
    pub lp_mint: Option<String>,
    #[serde(rename = "quotePrice")]
    pub quote_price: Option<f64>,
    #[serde(rename = "basePrice")]
    pub base_price: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
    #[serde(rename = "reserveSupply")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub reserve_supply: Option<String>, // Changed from i64 to String to handle large numbers
    #[serde(rename = "currentSupply")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub current_supply: Option<String>, // Changed from i64 to String to handle large numbers
    #[serde(rename = "quoteUSD")]
    pub quote_usd: Option<f64>,
    #[serde(rename = "baseUSD")]
    pub base_usd: Option<f64>,
    #[serde(rename = "pctReserve")]
    pub pct_reserve: Option<f64>,
    #[serde(rename = "pctSupply")]
    pub pct_supply: Option<f64>,
    pub holders: Option<Vec<Holder>>,
    #[serde(rename = "totalTokensUnlocked")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub total_tokens_unlocked: Option<String>, // Changed from i64 to String to handle large numbers
    #[serde(rename = "tokenSupply")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub token_supply: Option<String>, // Changed from i64 to String to handle large numbers
    #[serde(rename = "lpLocked")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub lp_locked: Option<String>, // Changed from i64 to String to handle large numbers
    #[serde(rename = "lpUnlocked")]
    #[serde(deserialize_with = "deserialize_int_or_string")]
    pub lp_unlocked: Option<String>, // Changed from i64 to String to handle large numbers
    #[serde(rename = "lpLockedPct")]
    pub lp_locked_pct: Option<f64>,
    #[serde(rename = "lpLockedUSD")]
    pub lp_locked_usd: Option<f64>,
}

// ===== INTERNAL DATA STRUCTURES =====

#[derive(Debug, Clone)]
pub struct RugcheckCacheEntry {
    pub data: RugcheckResponse,
    pub timestamp: DateTime<Utc>,
}

impl RugcheckCacheEntry {
    pub fn is_expired(&self) -> bool {
        Utc::now().signed_duration_since(self.timestamp).to_std().unwrap_or(Duration::MAX) >
            Duration::from_secs(RUGCHECK_DATA_EXPIRY_HOURS * 60 * 60)
    }
}

// ===== RUGCHECK SERVICE =====

pub struct RugcheckService {
    client: Client,
    database: TokenDatabase,
    shutdown_notify: Arc<Notify>,
    cache: Arc<RwLock<HashMap<String, RugcheckCacheEntry>>>,
    last_request_time: Arc<tokio::sync::Mutex<Instant>>,
}

impl RugcheckService {
    pub fn new(database: TokenDatabase, shutdown_notify: Arc<Notify>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(RUGCHECK_REQUEST_TIMEOUT_SECS))
            .user_agent("ScreenerBot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            database,
            shutdown_notify,
            cache: Arc::new(RwLock::new(HashMap::new())),
            last_request_time: Arc::new(tokio::sync::Mutex::new(Instant::now())),
        }
    }

    /// Start the background rugcheck service
    pub async fn start_background_service(self: Arc<Self>) {
        log(LogTag::Rugcheck, "START", "Starting rugcheck background service");

        // Initialize rugcheck table
        if let Err(e) = self.initialize_rugcheck_table().await {
            log(LogTag::Rugcheck, "ERROR", &format!("Failed to initialize rugcheck table: {}", e));
            return;
        }

        let mut update_interval = interval(Duration::from_secs(RUGCHECK_UPDATE_INTERVAL_SECS));

        loop {
            tokio::select! {
                _ = self.shutdown_notify.notified() => {
                    log(LogTag::Rugcheck, "SHUTDOWN", "Rugcheck service shutting down");
                    break;
                }
                _ = update_interval.tick() => {
                    self.update_all_rugcheck_data().await;
                }
            }
        }
    }

    /// Update rugcheck data for priority tokens (open positions, recent discoveries)
    /// These are updated more frequently regardless of expiry
    pub async fn update_priority_tokens(&self, priority_mints: Vec<String>) -> Result<(), String> {
        if priority_mints.is_empty() {
            return Ok(());
        }

        log(
            LogTag::Rugcheck,
            "PRIORITY_UPDATE",
            &format!("Updating rugcheck data for {} priority tokens", priority_mints.len())
        );

        // Filter to only tokens that actually need updating (older than 1 hour for priority)
        let mut tokens_to_update = Vec::new();
        let priority_expiry = Duration::from_secs(RUGCHECK_PRIORITY_UPDATE_INTERVAL_SECS * 60); // 1 hour for priority tokens

        for mint in priority_mints {
            let should_update = match self.database.get_rugcheck_data_with_timestamp(&mint) {
                Ok(Some((_rugcheck_data, updated_at))) => {
                    let age = Utc::now().signed_duration_since(updated_at);
                    if let Ok(age_duration) = age.to_std() {
                        age_duration > priority_expiry
                    } else {
                        true // Error parsing age, update to be safe
                    }
                }
                Ok(None) => true, // No data, needs fetching
                Err(_) => true, // Database error, update to be safe
            };

            if should_update {
                tokens_to_update.push(mint);
            }
        }

        if tokens_to_update.is_empty() {
            log(LogTag::Rugcheck, "PRIORITY_SKIP", "All priority tokens have recent rugcheck data");
            return Ok(());
        }

        log(
            LogTag::Rugcheck,
            "PRIORITY_FILTERED",
            &format!("Updating {} priority tokens that need refresh", tokens_to_update.len())
        );

        self.update_rugcheck_data_for_mints(tokens_to_update).await
    }

    /// Get list of tokens with expired rugcheck data (24+ hours old)
    async fn get_expired_tokens(&self) -> Result<Vec<String>, String> {
        // Get all tokens from database
        let tokens = self.database
            .get_all_tokens().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        let mut expired_mints = Vec::new();

        for token in tokens {
            let mint = token.mint.clone(); // Clone the mint to avoid ownership issues

            // Check if rugcheck data exists and if it's expired
            match self.database.get_rugcheck_data_with_timestamp(&mint) {
                Ok(Some((_rugcheck_data, updated_at))) => {
                    // Check if rugcheck data is expired
                    let age = Utc::now().signed_duration_since(updated_at);
                    if let Ok(age_duration) = age.to_std() {
                        if age_duration > Duration::from_secs(RUGCHECK_DATA_EXPIRY_HOURS * 60 * 60) {
                            expired_mints.push(mint);
                            if is_debug_rugcheck_enabled() {
                                log(
                                    LogTag::Rugcheck,
                                    "EXPIRED",
                                    &format!(
                                        "Token {} has expired rugcheck data (age: {:?})",
                                        token.mint,
                                        age_duration
                                    )
                                );
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No rugcheck data at all - needs fetching
                    expired_mints.push(mint.clone());
                    if is_debug_rugcheck_enabled() {
                        log(
                            LogTag::Rugcheck,
                            "MISSING",
                            &format!("Token {} has no rugcheck data - needs fetching", mint)
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Rugcheck,
                        "DB_ERROR",
                        &format!("Database error checking rugcheck data for {}: {}", mint, e)
                    );
                    // On database error, include token to be safe
                    expired_mints.push(mint);
                }
            }
        }

        Ok(expired_mints)
    }

    /// Update rugcheck data for expired tokens only (smart expiry-based updates)
    async fn update_all_rugcheck_data(&self) {
        log(LogTag::Rugcheck, "UPDATE", "Starting periodic rugcheck data update");

        // Get expired tokens that need updating (24+ hours old)
        let expired_mints = match self.get_expired_tokens().await {
            Ok(mints) => mints,
            Err(e) => {
                log(LogTag::Rugcheck, "ERROR", &format!("Failed to get expired tokens: {}", e));
                return;
            }
        };

        if expired_mints.is_empty() {
            log(LogTag::Rugcheck, "INFO", "No expired tokens found - all rugcheck data is current");
            return;
        }

        log(
            LogTag::Rugcheck,
            "INFO",
            &format!("Found {} expired tokens to update", expired_mints.len())
        );

        // Update only expired rugcheck data
        if let Err(e) = self.update_rugcheck_data_for_mints(expired_mints).await {
            log(
                LogTag::Rugcheck,
                "ERROR",
                &format!("Failed to update expired rugcheck data: {}", e)
            );
        }
    }

    /// Update rugcheck data for specific list of mints
    pub async fn update_rugcheck_data_for_mints(&self, mints: Vec<String>) -> Result<(), String> {
        let total_mints = mints.len();
        log(
            LogTag::Rugcheck,
            "START",
            &format!("Starting rugcheck update for {} tokens", total_mints)
        );

        let mut success_count = 0;
        let mut error_count = 0;

        let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_flag_clone = shutdown_flag.clone();
        let shutdown_notify_clone = self.shutdown_notify.clone();

        // Spawn a task to watch for shutdown signal
        tokio::spawn(async move {
            shutdown_notify_clone.notified().await;
            shutdown_flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        // Process tokens sequentially with rate limiting
        // Note: Individual fetch_rugcheck_data calls handle rate limiting internally
        for mint in mints {
            // Check for shutdown signal before each token
            if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
                log(
                    LogTag::Rugcheck,
                    "SHUTDOWN",
                    "Shutdown signal received during token processing"
                );
                break;
            }

            // Skip tokens that were updated within the last minute to prevent duplicate processing
            if
                let Ok(Some((_data, updated_at))) = self.database.get_rugcheck_data_with_timestamp(
                    &mint
                )
            {
                let age = Utc::now().signed_duration_since(updated_at);
                if let Ok(age_duration) = age.to_std() {
                    if age_duration < Duration::from_secs(60) {
                        // Skip if updated within last minute
                        if is_debug_rugcheck_enabled() {
                            log(
                                LogTag::Rugcheck,
                                "SKIP_RECENT",
                                &format!(
                                    "Skipping {} - recently updated ({:?} ago)",
                                    mint,
                                    age_duration
                                )
                            );
                        }
                        continue;
                    }
                }
            }

            match self.fetch_and_store_rugcheck_data(mint.clone()).await {
                Ok(_) => {
                    success_count += 1;

                    // Get the stored data for detailed success logging
                    if let Ok(Some(data)) = self.database.get_rugcheck_data(&mint) {
                        let symbol = data.token_meta
                            .as_ref()
                            .and_then(|meta| meta.symbol.as_ref())
                            .map(|s| s.as_str())
                            .unwrap_or(&mint);

                        let score = data.score_normalised.or(data.score);
                        let is_safe = is_token_safe_for_trading(&data);

                        log(
                            LogTag::Rugcheck,
                            "STORED",
                            &format!(
                                "✓ Stored {} | Risk Score: {} | Status: {} | {}/{}",
                                symbol,
                                score.map_or("N/A".to_string(), |s| s.to_string()),
                                if is_safe {
                                    "🟢 SAFE"
                                } else {
                                    "🔴 RISKY"
                                },
                                success_count,
                                total_mints
                            )
                        );
                    } else if is_debug_rugcheck_enabled() {
                        log(
                            LogTag::Rugcheck,
                            "SUCCESS",
                            &format!(
                                "✓ Updated rugcheck data for {} ({}/{})",
                                mint,
                                success_count,
                                total_mints
                            )
                        );
                    }
                }
                Err(e) => {
                    error_count += 1;
                    log(LogTag::Rugcheck, "ERROR", &format!("✗ Failed to update {}: {}", mint, e));
                }
            }
        }

        log(
            LogTag::Rugcheck,
            "COMPLETE",
            &format!("Rugcheck update completed: {} success, {} errors", success_count, error_count)
        );
        Ok(())
    }

    /// Fetch rugcheck data for a single token and store in database and cache
    async fn fetch_and_store_rugcheck_data(&self, mint: String) -> Result<(), String> {
        // Fetch data from rugcheck API
        let rugcheck_data = self.fetch_rugcheck_data(&mint).await?;

        // Store data in database
        self.store_rugcheck_data(&rugcheck_data).await?;

        // Update cache
        let cache_entry = RugcheckCacheEntry {
            data: rugcheck_data,
            timestamp: Utc::now(),
        };

        self.cache.write().await.insert(mint, cache_entry);

        Ok(())
    }

    /// Fetch rugcheck data from API with 3 retry attempts and rate limiting
    async fn fetch_rugcheck_data(&self, mint: &str) -> Result<RugcheckResponse, String> {
        // Apply rate limiting before making request
        {
            let mut last_time = self.last_request_time.lock().await;
            let elapsed = last_time.elapsed();
            let rate_limit_delay = Duration::from_millis(RUGCHECK_RATE_LIMIT_DELAY_MS);
            if elapsed < rate_limit_delay {
                let wait_time = rate_limit_delay - elapsed;
                if is_debug_rugcheck_enabled() {
                    log(
                        LogTag::Rugcheck,
                        "RATE_LIMIT",
                        &format!("Rate limiting: waiting {:?} before fetching {}", wait_time, mint)
                    );
                }
                tokio::time::sleep(wait_time).await;
            }
            *last_time = Instant::now();
        }

        let url = format!("https://api.rugcheck.xyz/v1/tokens/{}/report", mint);
        let max_retries = 3;
        let mut last_error = String::new();

        for attempt in 1..=max_retries {
            // Only log fetch attempts in debug mode to avoid noise
            if is_debug_rugcheck_enabled() {
                log(
                    LogTag::Rugcheck,
                    "FETCH",
                    &format!(
                        "Fetching data for token: {} (attempt {}/{})",
                        mint,
                        attempt,
                        max_retries
                    )
                );
            }

            match
                self.client
                    .get(&url)
                    .header("accept", "application/json")
                    .timeout(Duration::from_secs(RUGCHECK_REQUEST_TIMEOUT_SECS))
                    .send().await
            {
                Ok(response) => {
                    let status = response.status();

                    if !status.is_success() {
                        last_error = format!(
                            "API request failed with status: {} {}",
                            status.as_u16(),
                            status.canonical_reason().unwrap_or("Unknown")
                        );

                        // Handle 429 (rate limit) with longer wait
                        if status.as_u16() == 429 {
                            let wait_time = Duration::from_secs(10 * attempt); // Exponential backoff for 429
                            log(
                                LogTag::Rugcheck,
                                "RATE_LIMITED",
                                &format!(
                                    "Rate limited (429) for {}: waiting {:?} before retry",
                                    mint,
                                    wait_time
                                )
                            );

                            if attempt < max_retries {
                                tokio::time::sleep(wait_time).await;
                                // Update last request time
                                {
                                    let mut last_time = self.last_request_time.lock().await;
                                    *last_time = Instant::now();
                                }
                                continue;
                            }
                        }

                        log(
                            LogTag::Rugcheck,
                            "RETRY",
                            &format!(
                                "Attempt {}/{} failed for {}: {}",
                                attempt,
                                max_retries,
                                mint,
                                last_error
                            )
                        );
                    } else {
                        match response.json::<RugcheckResponse>().await {
                            Ok(rugcheck_data) => {
                                // Extract token information for detailed logging
                                let symbol = rugcheck_data.token_meta
                                    .as_ref()
                                    .and_then(|meta| meta.symbol.as_ref())
                                    .unwrap_or(&rugcheck_data.mint);

                                let name = rugcheck_data.token_meta
                                    .as_ref()
                                    .and_then(|meta| meta.name.as_ref())
                                    .unwrap_or(&symbol);

                                let score = rugcheck_data.score_normalised.or(rugcheck_data.score);
                                let is_rugged = rugcheck_data.rugged.unwrap_or(false);

                                // Count high-risk issues
                                let high_risks = if let Some(risks) = &rugcheck_data.risks {
                                    risks
                                        .iter()
                                        .filter(
                                            |r|
                                                r.level.as_deref() == Some("high") ||
                                                r.level.as_deref() == Some("critical")
                                        )
                                        .count()
                                } else {
                                    0
                                };

                                // Check LP lock status
                                let lp_status = if let Some(markets) = &rugcheck_data.markets {
                                    let mut best_lock_pct = 0.0f64;
                                    for market in markets {
                                        if let Some(lp) = &market.lp {
                                            if let Some(pct) = lp.lp_locked_pct {
                                                best_lock_pct = best_lock_pct.max(pct);
                                            }
                                        }
                                    }
                                    if best_lock_pct > 0.0 {
                                        format!("LP: {:.1}%", best_lock_pct)
                                    } else {
                                        "LP: Unknown".to_string()
                                    }
                                } else {
                                    "LP: No data".to_string()
                                };

                                // Format status indicators - REMEMBER: Higher score = MORE risk
                                let safety_status = if is_rugged {
                                    "🔴 RUGGED"
                                } else if score.map_or(false, |s| s >= 80) {
                                    "� VERY_HIGH_RISK"
                                } else if score.map_or(false, |s| s >= 50) {
                                    "� HIGH_RISK"
                                } else if score.map_or(false, |s| s >= 20) {
                                    "🟡 MEDIUM_RISK"
                                } else {
                                    "� LOW_RISK"
                                };

                                // Log success only in debug mode to avoid duplicate logging
                                // (The main logging is done in update_rugcheck_data_for_mints)
                                if is_debug_rugcheck_enabled() {
                                    log(
                                        LogTag::Rugcheck,
                                        "SUCCESS",
                                        &format!(
                                            "✓ {} ({}) | Score: {} | {} | Risks: {} | {} | Attempt: {}",
                                            symbol,
                                            name,
                                            score.map_or("N/A".to_string(), |s| s.to_string()),
                                            safety_status,
                                            high_risks,
                                            lp_status,
                                            attempt
                                        )
                                    );
                                }

                                return Ok(rugcheck_data);
                            }
                            Err(e) => {
                                last_error = format!("Failed to parse JSON response: {}", e);
                                log(
                                    LogTag::Rugcheck,
                                    "PARSE_ERROR",
                                    &format!(
                                        "JSON parse error for {} (attempt {}): {}",
                                        mint,
                                        attempt,
                                        e
                                    )
                                );

                                // For JSON parse errors, don't retry immediately - might be a schema issue
                                if attempt < max_retries {
                                    let wait_time = Duration::from_secs(2 * attempt);
                                    log(
                                        LogTag::Rugcheck,
                                        "RETRY",
                                        &format!(
                                            "Attempt {}/{} failed for {}: {}",
                                            attempt,
                                            max_retries,
                                            mint,
                                            last_error
                                        )
                                    );
                                    tokio::time::sleep(wait_time).await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("HTTP request failed: {}", e);
                    log(
                        LogTag::Rugcheck,
                        "RETRY",
                        &format!(
                            "Attempt {}/{} failed for {}: {}",
                            attempt,
                            max_retries,
                            mint,
                            last_error
                        )
                    );
                }
            }

            // Wait before retry (exponential backoff + additional rate limiting)
            if attempt < max_retries {
                let base_wait = std::time::Duration::from_millis(2000 * attempt);
                let rate_limit_wait = Duration::from_millis(RUGCHECK_RATE_LIMIT_DELAY_MS);
                let wait_time = std::cmp::max(base_wait, rate_limit_wait);

                log(
                    LogTag::Rugcheck,
                    "WAIT",
                    &format!(
                        "Waiting {:?} before retry for {} (attempt {} failed)",
                        wait_time,
                        mint,
                        attempt
                    )
                );
                tokio::time::sleep(wait_time).await;

                // Update last request time after wait
                {
                    let mut last_time = self.last_request_time.lock().await;
                    *last_time = Instant::now();
                }
            }
        }

        Err(format!("Failed to fetch rugcheck data after {} attempts: {}", max_retries, last_error))
    }

    /// Initialize the rugcheck table in the database
    async fn initialize_rugcheck_table(&self) -> Result<(), String> {
        self.database
            .initialize_rugcheck_table()
            .map_err(|e| format!("Failed to initialize rugcheck table: {}", e))
    }

    /// Store rugcheck data in the database
    async fn store_rugcheck_data(&self, data: &RugcheckResponse) -> Result<(), String> {
        self.database
            .store_rugcheck_data(data)
            .map_err(|e| format!("Database storage error: {}", e))
    }

    /// Get rugcheck data for a specific token (auto-fetch if missing)
    pub async fn get_rugcheck_data(&self, mint: &str) -> Result<Option<RugcheckResponse>, String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(mint) {
                if !entry.is_expired() {
                    log(LogTag::Rugcheck, "CACHE_HIT", &format!("Cache hit for token: {}", mint));
                    return Ok(Some(entry.data.clone()));
                } else {
                    log(
                        LogTag::Rugcheck,
                        "CACHE_EXPIRED",
                        &format!("Cache expired for token: {}", mint)
                    );
                }
            }
        }

        // Check database
        match self.database.get_rugcheck_data(mint) {
            Ok(Some(data)) => {
                // Check if data is not too old (optional - uncomment if needed)
                // let data_age = Utc::now() - data.updated_at.unwrap_or(Utc::now());
                // if data_age > ChronoDuration::hours(24) {
                //     log(LogTag::Rugcheck, "DATA_OLD", &format!("Database data is old for token: {}", mint));
                // } else {
                // Update cache with fresh database data
                let cache_entry = RugcheckCacheEntry {
                    data: data.clone(),
                    timestamp: Utc::now(),
                };
                self.cache.write().await.insert(mint.to_string(), cache_entry);

                log(LogTag::Rugcheck, "DB_HIT", &format!("Database hit for token: {}", mint));
                return Ok(Some(data));
                // }
            }
            Ok(None) => {
                log(
                    LogTag::Rugcheck,
                    "NOT_FOUND",
                    &format!("No rugcheck data found in database for token: {}", mint)
                );
                // Data missing - fetch it now
            }
            Err(e) => {
                log(
                    LogTag::Rugcheck,
                    "DB_ERROR",
                    &format!("Database error for token {}: {}", mint, e)
                );
                // Database error - try to fetch fresh data
            }
        }

        // Auto-fetch missing data from API (without logging to avoid duplication)
        if is_debug_rugcheck_enabled() {
            log(
                LogTag::Rugcheck,
                "AUTO_FETCH",
                &format!("Auto-fetching rugcheck data for token: {}", mint)
            );
        }

        match self.fetch_and_store_rugcheck_data(mint.to_string()).await {
            Ok(_) => {
                // Successfully fetched and stored - now get it from database
                match self.database.get_rugcheck_data(mint) {
                    Ok(Some(data)) => {
                        // Update cache
                        let cache_entry = RugcheckCacheEntry {
                            data: data.clone(),
                            timestamp: Utc::now(),
                        };
                        self.cache.write().await.insert(mint.to_string(), cache_entry);

                        // Only log detailed success in debug mode to avoid duplication
                        // (The main "STORED" logging is handled by batch update operations)
                        if is_debug_rugcheck_enabled() {
                            let symbol = data.token_meta
                                .as_ref()
                                .and_then(|meta| meta.symbol.as_ref())
                                .map(|s| s.as_str())
                                .unwrap_or(mint);

                            let score = data.score_normalised.or(data.score);
                            let is_safe = is_token_safe_for_trading(&data);
                            let high_risks = if let Some(risks) = &data.risks {
                                risks
                                    .iter()
                                    .filter(
                                        |r|
                                            r.level.as_deref() == Some("high") ||
                                            r.level.as_deref() == Some("critical")
                                    )
                                    .count()
                            } else {
                                0
                            };

                            log(
                                LogTag::Rugcheck,
                                "FETCH_SUCCESS",
                                &format!(
                                    "🔄 Auto-fetched {} | Risk Score: {} | Status: {} | High Risks: {}",
                                    symbol,
                                    score.map_or("N/A".to_string(), |s| s.to_string()),
                                    if is_safe {
                                        "🟢 SAFE"
                                    } else {
                                        "🔴 RISKY"
                                    },
                                    high_risks
                                )
                            );
                        }
                        Ok(Some(data))
                    }
                    Ok(None) => {
                        log(
                            LogTag::Rugcheck,
                            "FETCH_MISSING",
                            &format!("Data was fetched but not found in database for: {}", mint)
                        );
                        Ok(None)
                    }
                    Err(e) => {
                        log(
                            LogTag::Rugcheck,
                            "FETCH_DB_ERROR",
                            &format!("Database error after fetch for {}: {}", mint, e)
                        );
                        Ok(None) // Return None instead of error to prevent filtering failures
                    }
                }
            }
            Err(e) => {
                if is_debug_rugcheck_enabled() {
                    log(
                        LogTag::Rugcheck,
                        "FETCH_FAILED",
                        &format!("Failed to auto-fetch rugcheck data for {}: {}", mint, e)
                    );
                }
                Ok(None) // Return None instead of error - let token pass if rugcheck unavailable
            }
        }
    }

    /// Update rugcheck data for a single token immediately (for new discoveries)
    pub async fn update_token_rugcheck_data(&self, mint: &str) -> Result<(), String> {
        log(
            LogTag::Rugcheck,
            "UPDATE_SINGLE",
            &format!("Updating rugcheck data for single token: {}", mint)
        );

        self.fetch_and_store_rugcheck_data(mint.to_string()).await
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache
            .values()
            .filter(|entry| entry.is_expired())
            .count();
        (total_entries, expired_entries)
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.cache.write().await;
        let before_count = cache.len();
        cache.retain(|_, entry| !entry.is_expired());
        let after_count = cache.len();

        if before_count != after_count {
            log(
                LogTag::Rugcheck,
                "CLEANUP",
                &format!("Cleaned up {} expired cache entries", before_count - after_count)
            );
        }
    }
}

// ===== PUBLIC HELPER FUNCTIONS =====

/// Get rugcheck data for a specific token (convenience function)
pub async fn get_token_rugcheck_data(
    mint: &str,
    service: &RugcheckService
) -> Result<Option<RugcheckResponse>, String> {
    service.get_rugcheck_data(mint).await
}

/// Update rugcheck data for a newly discovered token
pub async fn update_new_token_rugcheck_data(
    mint: &str,
    service: &RugcheckService
) -> Result<(), String> {
    service.update_token_rugcheck_data(mint).await
}

/// Check if rugcheck data indicates token is safe for trading
pub fn is_token_safe_for_trading(rugcheck_data: &RugcheckResponse) -> bool {
    // Immediate disqualifiers - these are absolute red flags
    if rugcheck_data.rugged.unwrap_or(false) {
        return false;
    }

    // Check for critical risks first - these override everything
    if let Some(risks) = &rugcheck_data.risks {
        for risk in risks {
            if let Some(level) = &risk.level {
                if level == "critical" {
                    return false;
                }
            }
        }
    }

    // LP LOCK SAFETY CHECK - Critical for preventing rug pulls
    if !is_lp_sufficiently_locked(rugcheck_data) {
        return false;
    }

    // CORRECTED score-based evaluation - HIGHER SCORES MEAN MORE RISK!
    if let Some(risk_score) = rugcheck_data.score_normalised {
        // Very high risk scores (80+) are dangerous - immediate rejection
        if risk_score >= 80 {
            return false;
        }

        // High risk scores (50-79) need careful evaluation
        if risk_score >= 50 {
            // Count high-level risks
            let high_risk_count = if let Some(risks) = &rugcheck_data.risks {
                risks
                    .iter()
                    .filter(|r| r.level.as_deref() == Some("high"))
                    .count()
            } else {
                0
            };

            // For high risk scores, don't allow any high-risk issues
            if high_risk_count > 0 {
                return false;
            }
        }

        // Medium risk scores (20-49) - allow with limited high risks
        if risk_score >= 20 {
            let high_risk_count = if let Some(risks) = &rugcheck_data.risks {
                risks
                    .iter()
                    .filter(|r| r.level.as_deref() == Some("high"))
                    .count()
            } else {
                0
            };

            // For medium risk scores, allow max 1 high risk
            if high_risk_count > 1 {
                return false;
            }
        }

        // Low risk scores (0-19) are generally safe
        // These are typically established tokens with good safety profiles
    }

    // For authority checks, be more nuanced
    // Having authorities isn't always dangerous for established tokens
    let authority_risk_score = calculate_authority_risk_score(rugcheck_data);

    // If combined authority + risk score is too high, reject
    if authority_risk_score > 2 {
        return false;
    }

    true
}

/// Check if LP (Liquidity Pool) tokens are sufficiently locked
/// This is critical for preventing rug pulls where developers can drain liquidity
fn is_lp_sufficiently_locked(rugcheck_data: &RugcheckResponse) -> bool {
    // If we have markets with LP data, check each market
    if let Some(markets) = &rugcheck_data.markets {
        let mut has_valid_market = false;
        let mut at_least_one_sufficiently_locked = false;

        for market in markets {
            if let Some(lp) = &market.lp {
                has_valid_market = true;

                // Check LP locked percentage - should be at least 80%
                if let Some(lp_locked_pct) = lp.lp_locked_pct {
                    if lp_locked_pct >= 80.0 {
                        at_least_one_sufficiently_locked = true;
                        break; // Found at least one sufficiently locked market
                    }
                } else {
                    // If no LP locked percentage data, check raw amounts
                    let lp_locked = parse_string_to_u64(lp.lp_locked.as_deref()).unwrap_or(0);
                    let lp_unlocked = parse_string_to_u64(lp.lp_unlocked.as_deref()).unwrap_or(0);
                    let total_lp = lp_locked + lp_unlocked;

                    if total_lp > 0 {
                        let locked_percentage = ((lp_locked as f64) / (total_lp as f64)) * 100.0;
                        if locked_percentage >= 80.0 {
                            at_least_one_sufficiently_locked = true;
                            break; // Found at least one sufficiently locked market
                        }
                    }
                    // If no LP amounts available, continue checking other markets
                }
            }
        }

        // If we found markets but none had valid LP data, it's risky
        if !has_valid_market {
            return false;
        }

        // Return true only if we found at least one sufficiently locked market
        return at_least_one_sufficiently_locked;
    } else {
        // No market data available - we can't verify LP lock status
        // For safety, we should reject tokens without LP data
        return false;
    }
}

/// Calculate a risk score based on authorities and associated risks
fn calculate_authority_risk_score(rugcheck_data: &RugcheckResponse) -> i32 {
    let mut risk_score = 0;

    // Count high-level risks
    let high_risk_count = if let Some(risks) = &rugcheck_data.risks {
        risks
            .iter()
            .filter(|r| r.level.as_deref() == Some("high"))
            .count()
    } else {
        0
    };

    // Authority presence adds to risk, but not immediately disqualifying
    if rugcheck_data.freeze_authority.is_some() {
        risk_score += 1;
    }

    if rugcheck_data.mint_authority.is_some() {
        risk_score += 1;
    }

    // High risks amplify authority concerns
    risk_score += high_risk_count as i32;

    risk_score
}

/// Get rugcheck score for token (0-10 scale)
pub fn get_rugcheck_score(rugcheck_data: &RugcheckResponse) -> Option<i32> {
    rugcheck_data.score_normalised
}

/// Get high-risk issues from rugcheck data
pub fn get_high_risk_issues(rugcheck_data: &RugcheckResponse) -> Vec<String> {
    let mut issues = Vec::new();

    if rugcheck_data.rugged.unwrap_or(false) {
        issues.push("Token is flagged as rugged".to_string());
    }

    if rugcheck_data.freeze_authority.is_some() {
        issues.push("Freeze authority is present".to_string());
    }

    if rugcheck_data.mint_authority.is_some() {
        issues.push("Mint authority is present".to_string());
    }

    // Check LP lock status
    if !is_lp_sufficiently_locked(rugcheck_data) {
        // Get specific LP lock details for the issue description
        let lp_issue = get_lp_lock_issue_details(rugcheck_data);
        issues.push(lp_issue);
    }

    if let Some(risks) = &rugcheck_data.risks {
        for risk in risks {
            if let Some(level) = &risk.level {
                if level == "critical" || level == "high" {
                    issues.push(
                        format!(
                            "{}: {}",
                            risk.name,
                            risk.description.as_deref().unwrap_or("No description")
                        )
                    );
                }
            }
        }
    }

    issues
}

/// Get detailed LP lock issue description
fn get_lp_lock_issue_details(rugcheck_data: &RugcheckResponse) -> String {
    if let Some(markets) = &rugcheck_data.markets {
        let mut insufficient_markets = Vec::new();

        for market in markets {
            if let Some(lp) = &market.lp {
                if let Some(lp_locked_pct) = lp.lp_locked_pct {
                    if lp_locked_pct < 80.0 {
                        insufficient_markets.push(format!("{:.1}% locked", lp_locked_pct));
                    }
                } else {
                    let lp_locked = parse_string_to_u64(lp.lp_locked.as_deref()).unwrap_or(0);
                    let lp_unlocked = parse_string_to_u64(lp.lp_unlocked.as_deref()).unwrap_or(0);
                    let total_lp = lp_locked + lp_unlocked;

                    if total_lp > 0 {
                        let locked_percentage = ((lp_locked as f64) / (total_lp as f64)) * 100.0;
                        if locked_percentage < 80.0 {
                            insufficient_markets.push(format!("{:.1}% locked", locked_percentage));
                        }
                    } else {
                        insufficient_markets.push("no LP data".to_string());
                    }
                }
            }
        }

        if !insufficient_markets.is_empty() {
            return format!(
                "LP insufficiently locked: {} (minimum 80% required)",
                insufficient_markets.join(", ")
            );
        }
    }

    "LP lock status unknown or missing - potential rug pull risk".to_string()
}

/// Helper function to parse string numbers to u64 (for handling large integers from API)
fn parse_string_to_u64(value: Option<&str>) -> Option<u64> {
    value?.parse::<u64>().ok()
}
