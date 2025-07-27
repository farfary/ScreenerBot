/// Rugcheck API Integration
///
/// This module handles integration with the rugcheck.xyz API to fetch comprehensive
/// security analysis data for tokens. It includes rate limiting, batch processing,
/// and database storage for all rugcheck response data.

use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use tokio::time::sleep;

// Rate limiting constants
const RUGCHECK_RATE_LIMIT_DELAY: Duration = Duration::from_millis(1000); // 1 second between requests
const RUGCHECK_BATCH_SIZE: usize = 3; // Process 3 tokens concurrently
const RUGCHECK_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// ===== RUGCHECK API RESPONSE STRUCTURES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckResponse {
    pub mint: String,
    #[serde(rename = "tokenProgram")]
    pub token_program: Option<String>,
    pub creator: Option<String>,
    #[serde(rename = "creatorBalance")]
    pub creator_balance: Option<i64>,
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
    pub verification: Option<serde_json::Value>,
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
    pub supply: Option<i64>,
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
    pub amount: Option<i64>,
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
    pub max_amount: Option<i64>,
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
    pub reserve_supply: Option<i64>,
    #[serde(rename = "currentSupply")]
    pub current_supply: Option<i64>,
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
    pub total_tokens_unlocked: Option<i64>,
    #[serde(rename = "tokenSupply")]
    pub token_supply: Option<i64>,
    #[serde(rename = "lpLocked")]
    pub lp_locked: Option<i64>,
    #[serde(rename = "lpUnlocked")]
    pub lp_unlocked: Option<i64>,
    #[serde(rename = "lpLockedPct")]
    pub lp_locked_pct: Option<f64>,
    #[serde(rename = "lpLockedUSD")]
    pub lp_locked_usd: Option<f64>,
}

// ===== RUGCHECK SERVICE =====

pub struct RugcheckService {
    client: Client,
    database: TokenDatabase,
}

impl RugcheckService {
    pub fn new(database: TokenDatabase) -> Self {
        let client = Client::builder()
            .timeout(RUGCHECK_REQUEST_TIMEOUT)
            .user_agent("ScreenerBot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, database }
    }

    /// Main function to update rugcheck data for a list of token mints
    pub async fn update_rugcheck_data(&self, mints: Vec<String>) -> Result<(), String> {
        log(
            LogTag::Rugcheck,
            "START",
            &format!("Starting rugcheck update for {} tokens", mints.len())
        );

        // Initialize rugcheck table
        self.initialize_rugcheck_table().await?;

        // Process tokens in batches with rate limiting
        let mut last_request_time = Instant::now();

        for chunk in mints.chunks(RUGCHECK_BATCH_SIZE) {
            // Rate limiting: ensure we don't exceed API limits
            let elapsed = last_request_time.elapsed();
            if elapsed < RUGCHECK_RATE_LIMIT_DELAY {
                let wait_time = RUGCHECK_RATE_LIMIT_DELAY - elapsed;
                log(LogTag::Rugcheck, "WAIT", &format!("Rate limiting: waiting {:?}", wait_time));
                sleep(wait_time).await;
            }

            // Process batch concurrently
            let futures: Vec<_> = chunk
                .iter()
                .map(|mint| self.fetch_and_store_rugcheck_data(mint.clone()))
                .collect();

            let results = futures::future::join_all(futures).await;

            // Count successes and failures
            let mut success_count = 0;
            let mut error_count = 0;

            for (i, result) in results.iter().enumerate() {
                match result {
                    Ok(_) => {
                        success_count += 1;
                        log(
                            LogTag::Rugcheck,
                            "SUCCESS",
                            &format!("✓ Updated rugcheck data for {}", chunk[i])
                        );
                    }
                    Err(e) => {
                        error_count += 1;
                        log(
                            LogTag::Rugcheck,
                            "ERROR",
                            &format!("✗ Failed to update {}: {}", chunk[i], e)
                        );
                    }
                }
            }

            log(
                LogTag::Rugcheck,
                "BATCH",
                &format!("Batch completed: {} success, {} errors", success_count, error_count)
            );
            last_request_time = Instant::now();
        }

        log(LogTag::Rugcheck, "COMPLETE", "Rugcheck data update completed");
        Ok(())
    }

    /// Fetch rugcheck data for a single token and store in database
    async fn fetch_and_store_rugcheck_data(&self, mint: String) -> Result<(), String> {
        // Fetch data from rugcheck API
        let rugcheck_data = self.fetch_rugcheck_data(&mint).await?;

        // Store data in database
        self.store_rugcheck_data(&rugcheck_data).await?;

        Ok(())
    }

    /// Fetch rugcheck data from API
    async fn fetch_rugcheck_data(&self, mint: &str) -> Result<RugcheckResponse, String> {
        let url = format!("https://api.rugcheck.xyz/v1/tokens/{}/report", mint);

        log(LogTag::Rugcheck, "FETCH", &format!("Fetching data for token: {}", mint));

        let response = self.client
            .get(&url)
            .header("accept", "application/json")
            .send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()));
        }

        let rugcheck_data: RugcheckResponse = response
            .json().await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

        Ok(rugcheck_data)
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

    /// Get rugcheck data for a specific token
    pub async fn get_rugcheck_data(&self, mint: &str) -> Result<Option<RugcheckResponse>, String> {
        self.database
            .get_rugcheck_data(mint)
            .map_err(|e| format!("Failed to get rugcheck data: {}", e))
    }
}

// ===== CONVENIENCE FUNCTIONS =====

/// Main function to update rugcheck data for all tokens in database
pub async fn update_all_tokens_rugcheck_data() -> Result<(), String> {
    log(LogTag::Rugcheck, "START", "Starting rugcheck data update for all tokens");

    // Initialize database and service
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let rugcheck_service = RugcheckService::new(database.clone());

    // Get all token mints from database
    let tokens = database
        .get_all_tokens().await
        .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

    let mints: Vec<String> = tokens
        .into_iter()
        .map(|token| token.mint)
        .collect();

    log(LogTag::Rugcheck, "INFO", &format!("Found {} tokens to process", mints.len()));

    // Update rugcheck data
    rugcheck_service.update_rugcheck_data(mints).await?;

    log(LogTag::Rugcheck, "COMPLETE", "Completed rugcheck data update for all tokens");
    Ok(())
}

/// Update rugcheck data for specific list of mints
pub async fn update_rugcheck_data_for_mints(mints: Vec<String>) -> Result<(), String> {
    log(
        LogTag::Rugcheck,
        "START",
        &format!("Starting rugcheck data update for {} specific tokens", mints.len())
    );

    // Initialize database and service
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let rugcheck_service = RugcheckService::new(database);

    // Update rugcheck data
    rugcheck_service.update_rugcheck_data(mints).await?;

    log(LogTag::Rugcheck, "COMPLETE", "Completed rugcheck data update for specific tokens");
    Ok(())
}

/// Get rugcheck data for a specific token
pub async fn get_token_rugcheck_data(mint: &str) -> Result<Option<RugcheckResponse>, String> {
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let rugcheck_service = RugcheckService::new(database);

    rugcheck_service.get_rugcheck_data(mint).await
}
