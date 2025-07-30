/// Rugcheck API Integration Service
///
/// This module provides a background service for fetching and updating rugcheck data.
/// It handles rate limiting, database storage, and provides fresh data to the trading system.

use crate::logger::{ log, LogTag };
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
const RUGCHECK_RATE_LIMIT_DELAY: Duration = Duration::from_millis(5000); // 5 seconds between requests (more conservative for 429 errors)
const RUGCHECK_BATCH_SIZE: usize = 1; // Process 1 token at a time for better rate limiting
const RUGCHECK_REQUEST_TIMEOUT: Duration = Duration::from_secs(45); // Increased timeout
const RUGCHECK_UPDATE_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes
const RUGCHECK_DATA_EXPIRY: Duration = Duration::from_secs(60 * 60); // 5 minutes

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
            RUGCHECK_DATA_EXPIRY
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
            .timeout(RUGCHECK_REQUEST_TIMEOUT)
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

        let mut update_interval = interval(RUGCHECK_UPDATE_INTERVAL);

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

    /// Update rugcheck data for all tokens in database
    async fn update_all_rugcheck_data(&self) {
        log(LogTag::Rugcheck, "UPDATE", "Starting periodic rugcheck data update");

        // Get all token mints from database
        let tokens = match self.database.get_all_tokens().await {
            Ok(tokens) => tokens,
            Err(e) => {
                log(
                    LogTag::Rugcheck,
                    "ERROR",
                    &format!("Failed to get tokens from database: {}", e)
                );
                return;
            }
        };

        let mints: Vec<String> = tokens
            .into_iter()
            .map(|token| token.mint)
            .collect();

        if mints.is_empty() {
            log(LogTag::Rugcheck, "INFO", "No tokens found to update");
            return;
        }

        log(LogTag::Rugcheck, "INFO", &format!("Found {} tokens to update", mints.len()));

        // Update rugcheck data with rate limiting
        if let Err(e) = self.update_rugcheck_data_for_mints(mints).await {
            log(LogTag::Rugcheck, "ERROR", &format!("Failed to update rugcheck data: {}", e));
        }
    }

    /// Update rugcheck data for specific list of mints
    pub async fn update_rugcheck_data_for_mints(&self, mints: Vec<String>) -> Result<(), String> {
        log(
            LogTag::Rugcheck,
            "START",
            &format!("Starting rugcheck update for {} tokens", mints.len())
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

        for chunk in mints.chunks(RUGCHECK_BATCH_SIZE) {
            // Check for shutdown signal before processing each batch
            if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
                log(LogTag::Rugcheck, "SHUTDOWN", "Shutdown signal received during batch processing");
                break;
            }

            // Process batch sequentially to avoid rate limiting
            // Note: Individual fetch_rugcheck_data calls now handle rate limiting
            for mint in chunk {
                // Check for shutdown signal before each token
                if shutdown_flag.load(std::sync::atomic::Ordering::SeqCst) {
                    log(LogTag::Rugcheck, "SHUTDOWN", "Shutdown signal received during token processing");
                    break;
                }

                match self.fetch_and_store_rugcheck_data(mint.clone()).await {
                    Ok(_) => {
                        success_count += 1;
                        log(
                            LogTag::Rugcheck,
                            "SUCCESS",
                            &format!("✓ Updated rugcheck data for {}", mint)
                        );
                    }
                    Err(e) => {
                        error_count += 1;
                        log(
                            LogTag::Rugcheck,
                            "ERROR",
                            &format!("✗ Failed to update {}: {}", mint, e)
                        );
                    }
                }
            }

            log(
                LogTag::Rugcheck,
                "BATCH",
                &format!("Batch completed: {} success, {} errors", success_count, error_count)
            );
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
            if elapsed < RUGCHECK_RATE_LIMIT_DELAY {
                let wait_time = RUGCHECK_RATE_LIMIT_DELAY - elapsed;
                log(
                    LogTag::Rugcheck,
                    "RATE_LIMIT",
                    &format!("Rate limiting: waiting {:?} before fetching {}", wait_time, mint)
                );
                tokio::time::sleep(wait_time).await;
            }
            *last_time = Instant::now();
        }

        let url = format!("https://api.rugcheck.xyz/v1/tokens/{}/report", mint);
        let max_retries = 3;
        let mut last_error = String::new();

        for attempt in 1..=max_retries {
            log(
                LogTag::Rugcheck,
                "FETCH",
                &format!("Fetching data for token: {} (attempt {}/{})", mint, attempt, max_retries)
            );

            match
                self.client
                    .get(&url)
                    .header("accept", "application/json")
                    .timeout(RUGCHECK_REQUEST_TIMEOUT)
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
                                log(
                                    LogTag::Rugcheck,
                                    "SUCCESS",
                                    &format!(
                                        "Successfully fetched rugcheck data for {} on attempt {}",
                                        mint,
                                        attempt
                                    )
                                );
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
                let rate_limit_wait = RUGCHECK_RATE_LIMIT_DELAY;
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

        // Auto-fetch missing data from API
        log(
            LogTag::Rugcheck,
            "AUTO_FETCH",
            &format!("Auto-fetching rugcheck data for token: {}", mint)
        );

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

                        log(
                            LogTag::Rugcheck,
                            "FETCH_SUCCESS",
                            &format!("Successfully auto-fetched rugcheck data for: {}", mint)
                        );
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
                log(
                    LogTag::Rugcheck,
                    "FETCH_FAILED",
                    &format!("Failed to auto-fetch rugcheck data for {}: {}", mint, e)
                );
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

    // Improved score-based evaluation for trading
    if let Some(score) = rugcheck_data.score_normalised {
        // Very low scores (negative) are dangerous
        if score < 0 {
            return false;
        }

        // For low scores (0-4), check if there are high-risk issues
        if score < 5 {
            // Count high-level risks
            let high_risk_count = if let Some(risks) = &rugcheck_data.risks {
                risks
                    .iter()
                    .filter(|r| r.level.as_deref() == Some("high"))
                    .count()
            } else {
                0
            };

            // If score is 0-2 with no high risks, it's likely a legitimate established token
            // (USDC, SOL, USDT all fall into this category)
            if score <= 2 && high_risk_count == 0 {
                return true;
            }

            // For scores 3-4, allow if less than 2 high risks
            if score >= 3 && high_risk_count < 2 {
                return true;
            }

            // Otherwise, too risky for low score
            return false;
        }
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
