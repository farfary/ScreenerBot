// token_monitor.rs - Advanced token monitoring with database periodic checks and liquidity-based prioritization
use crate::global::{ Token, TOKEN_DB, LIST_TOKENS };
use crate::token_blacklist::{ check_and_track_liquidity, is_token_blacklisted };
use crate::logger::{ log, LogTag };
use crate::utils::check_shutdown_or_delay;
use std::sync::Arc;
use tokio::sync::{ Notify, Semaphore };
use tokio::time::{ Duration, sleep };
use reqwest::StatusCode;
use serde_json;
use std::collections::HashMap;
use chrono::{ Utc, DateTime };

/// Token monitoring manager with database-driven periodic checks
pub struct TokenMonitor {
    info_rate_limiter: Arc<Semaphore>,
    current_cycle: usize,
}

impl TokenMonitor {
    /// API rate limits: 200 calls per minute for token info
    const INFO_RATE_LIMIT: usize = 200;
    const INFO_CALLS_PER_CYCLE: usize = 100; // Use 100 calls per cycle (50% of limit)
    const CYCLE_DURATION_MINUTES: u64 = 1; // 1 minute cycles

    /// Create new token monitor
    pub fn new() -> Self {
        Self {
            info_rate_limiter: Arc::new(Semaphore::new(Self::INFO_RATE_LIMIT)),
            current_cycle: 0,
        }
    }

    /// Start periodic token monitoring from database
    pub async fn start_periodic_monitoring(&mut self, shutdown: Arc<Notify>) {
        log(LogTag::Monitor, "INFO", "Starting periodic token database monitoring...");

        loop {
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
                log(LogTag::Monitor, "INFO", "Token monitor shutting down...");
                break;
            }

            let cycle_start = Utc::now();
            self.current_cycle += 1;

            log(
                LogTag::Monitor,
                "INFO",
                &format!("Starting token monitoring cycle #{}", self.current_cycle)
            );

            // Get tokens from database for monitoring
            let tokens_to_check = match self.get_tokens_for_monitoring().await {
                Ok(tokens) => tokens,
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "ERROR",
                        &format!("Failed to get tokens for monitoring: {}", e)
                    );
                    self.wait_for_next_cycle(shutdown.clone()).await;
                    continue;
                }
            };

            if tokens_to_check.is_empty() {
                log(LogTag::Monitor, "WARN", "No tokens found in database for monitoring");
                self.wait_for_next_cycle(shutdown.clone()).await;
                continue;
            }

            // Prioritize tokens: 50% high liquidity, 50% others
            let (high_liquidity, others) = self.prioritize_tokens(tokens_to_check);

            log(
                LogTag::Monitor,
                "INFO",
                &format!(
                    "Monitoring {} high liquidity tokens and {} others",
                    high_liquidity.len(),
                    others.len()
                )
            );

            // Check tokens with rate limiting
            let checked_count = self.check_tokens_batch(
                high_liquidity,
                others,
                shutdown.clone()
            ).await;

            let cycle_duration = Utc::now().signed_duration_since(cycle_start);
            log(
                LogTag::Monitor,
                "SUCCESS",
                &format!(
                    "Cycle #{} completed: {} tokens checked in {:.1}s",
                    self.current_cycle,
                    checked_count,
                    (cycle_duration.num_milliseconds() as f64) / 1000.0
                )
            );

            // Wait for next cycle
            self.wait_for_next_cycle(shutdown.clone()).await;
        }
    }

    /// Get tokens from database for monitoring
    async fn get_tokens_for_monitoring(&self) -> Result<Vec<Token>, String> {
        if let Ok(token_db_guard) = TOKEN_DB.lock() {
            if let Some(ref db) = *token_db_guard {
                return db.get_all_tokens().map_err(|e| format!("Database error: {}", e));
            }
        }
        Err("Token database not initialized".to_string())
    }

    /// Prioritize tokens: 50% high liquidity, 50% others
    fn prioritize_tokens(&self, mut tokens: Vec<Token>) -> (Vec<Token>, Vec<Token>) {
        // Remove blacklisted tokens
        tokens.retain(|token| !is_token_blacklisted(&token.mint));

        // Sort by liquidity (highest first)
        tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_tokens = tokens.len();
        let half_calls = Self::INFO_CALLS_PER_CYCLE / 2;

        // Split into high liquidity (first half of calls) and others
        let split_point = std::cmp::min(half_calls, total_tokens);
        let high_liquidity = tokens.drain(..split_point).collect();
        let others = tokens.into_iter().take(half_calls).collect(); // Take remaining calls worth of others

        (high_liquidity, others)
    }

    /// Check tokens in batches with proper rate limiting
    async fn check_tokens_batch(
        &self,
        high_liquidity: Vec<Token>,
        others: Vec<Token>,
        shutdown: Arc<Notify>
    ) -> usize {
        let mut checked_count = 0;
        let mut updated_tokens = Vec::new();

        // Process high liquidity tokens first
        log(LogTag::Monitor, "INFO", "Checking high liquidity tokens...");
        for token in high_liquidity {
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
                break;
            }

            if let Some(updated_token) = self.check_single_token(&token, shutdown.clone()).await {
                updated_tokens.push(updated_token);
                checked_count += 1;
            }
        }

        // Then process other tokens
        log(LogTag::Monitor, "INFO", "Checking other tokens...");
        for token in others {
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
                break;
            }

            if let Some(updated_token) = self.check_single_token(&token, shutdown.clone()).await {
                updated_tokens.push(updated_token);
                checked_count += 1;
            }
        }

        // Update LIST_TOKENS with refreshed data (non-blocking)
        if !updated_tokens.is_empty() {
            self.update_global_token_list(updated_tokens).await;
        }

        checked_count
    }

    /// Check a single token and return updated token data
    async fn check_single_token(&self, token: &Token, shutdown: Arc<Notify>) -> Option<Token> {
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
            return None;
        }

        // Acquire rate limit permit
        let permit = match
            tokio::time::timeout(
                Duration::from_secs(5),
                self.info_rate_limiter.clone().acquire_owned()
            ).await
        {
            Ok(Ok(permit)) => permit,
            _ => {
                log(
                    LogTag::Monitor,
                    "WARN",
                    &format!("Failed to acquire rate limit permit for {}", token.symbol)
                );
                return None;
            }
        };

        // Fetch updated token info from DexScreener
        let updated_token = match self.fetch_token_info(&token.mint).await {
            Ok(Some(mut updated)) => {
                // Preserve important fields from cached token
                updated.created_at = token.created_at;
                Some(updated)
            }
            Ok(None) => {
                log(
                    LogTag::Monitor,
                    "WARN",
                    &format!("No data returned for token {}", token.symbol)
                );
                None
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to fetch token {}: {}", token.symbol, e)
                );
                None
            }
        };

        drop(permit); // Release permit

        // Check for blacklisting if we got updated data
        if let Some(ref updated) = updated_token {
            let liquidity_usd = updated.liquidity.as_ref().and_then(|l| l.usd);

            if
                check_and_track_liquidity(
                    &updated.mint,
                    &updated.symbol,
                    liquidity_usd,
                    updated.created_at
                )
            {
                log(
                    LogTag::Monitor,
                    "BLACKLIST",
                    &format!(
                        "Token {} ({}) was blacklisted due to low liquidity",
                        updated.symbol,
                        updated.mint
                    )
                );
                return None; // Don't include blacklisted tokens
            }

            // Cache updated token to database
            if let Ok(token_db_guard) = TOKEN_DB.lock() {
                if let Some(ref db) = *token_db_guard {
                    if let Err(e) = db.add_or_update_token(updated, "periodic_check") {
                        log(
                            LogTag::Monitor,
                            "ERROR",
                            &format!("Failed to cache token {}: {}", updated.symbol, e)
                        );
                    }
                }
            }
        }

        // Small delay to be gentle on the API
        sleep(Duration::from_millis(300)).await;

        updated_token
    }

    /// Fetch token info from DexScreener API
    async fn fetch_token_info(&self, mint: &str) -> Result<Option<Token>, String> {
        let chain_id = "solana";
        let url = format!("https://api.dexscreener.com/tokens/v1/{}/{}", chain_id, mint);

        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let resp = client
            .get(&url)
            .send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status() != StatusCode::OK {
            return Err(format!("API returned status: {}", resp.status()));
        }

        let data: serde_json::Value = resp
            .json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(tokens_array) = data.as_array() {
            if let Some(first_token) = tokens_array.first() {
                return self.parse_token_from_api(first_token);
            }
        }

        Ok(None)
    }

    /// Parse token data from DexScreener API response
    fn parse_token_from_api(
        &self,
        token_data: &serde_json::Value
    ) -> Result<Option<Token>, String> {
        // Parse the token data similar to discovery.rs but simplified
        let mint = token_data["mint"].as_str().ok_or("Missing mint field")?.to_string();

        let symbol = token_data["symbol"].as_str().unwrap_or("UNKNOWN").to_string();

        let name = token_data["name"].as_str().unwrap_or("Unknown Token").to_string();

        let decimals = token_data["decimals"].as_u64().unwrap_or(9) as u8;

        // Parse liquidity data
        let liquidity = if let Some(liquidity_obj) = token_data["liquidity"].as_object() {
            Some(crate::global::LiquidityInfo {
                usd: liquidity_obj["usd"].as_f64(),
                base: liquidity_obj["base"].as_f64(),
                quote: liquidity_obj["quote"].as_f64(),
            })
        } else {
            None
        };

        // Parse price data
        let price_dexscreener_sol = token_data["price"].as_f64();
        let price_dexscreener_usd = token_data["priceUsd"].as_f64();

        // Create basic token struct with essential fields
        let token = Token {
            mint,
            symbol,
            name,
            decimals,
            chain: "solana".to_string(),
            logo_url: token_data["logoUrl"].as_str().map(|s| s.to_string()),
            coingecko_id: None,
            website: None,
            description: None,
            tags: Vec::new(),
            is_verified: false,
            created_at: None, // Will be preserved from cache
            price_dexscreener_sol,
            price_dexscreener_usd,
            price_pool_sol: None,
            price_pool_usd: None,
            pools: Vec::new(),
            dex_id: None,
            pair_address: None,
            pair_url: None,
            labels: Vec::new(),
            fdv: token_data["fdv"].as_f64(),
            market_cap: token_data["marketCap"].as_f64(),
            txns: None, // Could be parsed if needed
            volume: None, // Could be parsed if needed
            price_change: None, // Could be parsed if needed
            liquidity,
            info: None,
            boosts: None,
        };

        Ok(Some(token))
    }

    /// Update global token list with refreshed data (non-blocking)
    async fn update_global_token_list(&self, updated_tokens: Vec<Token>) {
        tokio::spawn(async move {
            if let Ok(mut list_tokens) = LIST_TOKENS.try_write() {
                // Create a map for quick lookup
                let mut token_map: HashMap<String, Token> = updated_tokens
                    .into_iter()
                    .map(|token| (token.mint.clone(), token))
                    .collect();

                // Update existing tokens in LIST_TOKENS
                for existing_token in list_tokens.iter_mut() {
                    if let Some(updated_token) = token_map.remove(&existing_token.mint) {
                        *existing_token = updated_token;
                    }
                }

                // Add any new tokens that weren't in the list
                for (_, new_token) in token_map {
                    list_tokens.push(new_token);
                }

                log(
                    LogTag::Monitor,
                    "SUCCESS",
                    &format!("Updated global token list with {} tokens", list_tokens.len())
                );
            } else {
                log(
                    LogTag::Monitor,
                    "WARN",
                    "Could not acquire write lock for LIST_TOKENS (non-blocking update)"
                );
            }
        });
    }

    /// Wait for next monitoring cycle
    async fn wait_for_next_cycle(&self, shutdown: Arc<Notify>) {
        let cycle_duration = Duration::from_secs(Self::CYCLE_DURATION_MINUTES * 60);

        if check_shutdown_or_delay(&shutdown, cycle_duration).await {
            return;
        }
    }
}

/// Start the token monitoring background task
pub async fn start_token_monitoring(shutdown: Arc<Notify>) {
    let mut monitor = TokenMonitor::new();
    monitor.start_periodic_monitoring(shutdown).await;
}
