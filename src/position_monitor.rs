// position_monitor.rs - Dedicated monitoring for open position tokens with faster updates
use crate::global::{ Token, LIST_TOKENS };
use crate::positions::SAVED_POSITIONS;
use crate::token_blacklist::is_token_blacklisted;
use crate::logger::{ log, LogTag };
use crate::utils::check_shutdown_or_delay;
use std::sync::Arc;
use tokio::sync::{ Notify, Semaphore };
use tokio::time::{ Duration, sleep };
use reqwest::StatusCode;
use serde_json;
use std::collections::{ HashMap, HashSet };
use chrono::Utc;

/// Position token monitor with faster updates for open positions
pub struct PositionMonitor {
    info_rate_limiter: Arc<Semaphore>,
    current_cycle: usize,
}

impl PositionMonitor {
    /// Fast position monitoring rate limits: 100 calls per minute (more aggressive)
    const POSITION_INFO_RATE_LIMIT: usize = 100;
    const POSITION_CALLS_PER_CYCLE: usize = 50; // Use 50 calls per cycle
    const POSITION_CYCLE_DURATION_SECONDS: u64 = 15; // 15 second cycles for fast updates

    /// Minimum liquidity to return token to main watch list
    const MIN_LIQUIDITY_FOR_MAIN_WATCH: f64 = 1000.0; // $1000 USD

    /// Create new position monitor
    pub fn new() -> Self {
        Self {
            info_rate_limiter: Arc::new(Semaphore::new(Self::POSITION_INFO_RATE_LIMIT)),
            current_cycle: 0,
        }
    }

    /// Start periodic position token monitoring
    pub async fn start_position_monitoring(&mut self, shutdown: Arc<Notify>) {
        log(LogTag::Trader, "INFO", "Starting dedicated position token monitoring...");

        loop {
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
                log(LogTag::Trader, "INFO", "Position monitor shutting down...");
                break;
            }

            let cycle_start = Utc::now();
            self.current_cycle += 1;

            log(
                LogTag::Trader,
                "INFO",
                &format!("Starting position monitoring cycle #{}", self.current_cycle)
            );

            // Get open position tokens for fast monitoring
            let position_tokens = self.get_open_position_tokens().await;

            if position_tokens.is_empty() {
                log(LogTag::Trader, "INFO", "No open positions to monitor, waiting...");
                self.wait_for_next_position_cycle(shutdown.clone()).await;
                continue;
            }

            log(
                LogTag::Trader,
                "INFO",
                &format!("Fast monitoring {} position tokens", position_tokens.len())
            );

            // Monitor position tokens with fast updates
            let checked_count = self.check_position_tokens_batch(
                position_tokens,
                shutdown.clone()
            ).await;

            let cycle_duration = Utc::now().signed_duration_since(cycle_start);
            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "Position cycle #{} completed: {} tokens checked in {:.1}s",
                    self.current_cycle,
                    checked_count,
                    (cycle_duration.num_milliseconds() as f64) / 1000.0
                )
            );

            // Wait for next fast cycle
            self.wait_for_next_position_cycle(shutdown.clone()).await;
        }
    }

    /// Get tokens that have open positions for monitoring
    async fn get_open_position_tokens(&self) -> Vec<String> {
        let mut position_mints = HashSet::new();

        if let Ok(positions) = SAVED_POSITIONS.lock() {
            for position in positions.iter() {
                // Only monitor tokens with open positions (exit_price is None)
                if position.position_type == "buy" && position.exit_price.is_none() {
                    position_mints.insert(position.mint.clone());
                }
            }
        }

        position_mints.into_iter().collect()
    }

    /// Check position tokens with faster rate limiting
    async fn check_position_tokens_batch(
        &self,
        position_mints: Vec<String>,
        shutdown: Arc<Notify>
    ) -> usize {
        let mut checked_count = 0;
        let mut updated_tokens = Vec::new();
        let mut closed_position_tokens = Vec::new();

        // Check each position token
        for mint in position_mints {
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
                break;
            }

            // Check if position is still open (might have been closed during cycle)
            if !self.is_position_still_open(&mint).await {
                // Position was closed, check if token should return to main watch list
                closed_position_tokens.push(mint.clone());
                continue;
            }

            if
                let Some(updated_token) = self.check_single_position_token(
                    &mint,
                    shutdown.clone()
                ).await
            {
                updated_tokens.push(updated_token);
                checked_count += 1;
            }
        }

        // Handle closed position tokens
        if !closed_position_tokens.is_empty() {
            self.handle_closed_position_tokens(closed_position_tokens).await;
        }

        // Update LIST_TOKENS with refreshed position data
        if !updated_tokens.is_empty() {
            self.update_position_tokens_in_list(updated_tokens).await;
        }

        checked_count
    }

    /// Check if a position is still open
    async fn is_position_still_open(&self, mint: &str) -> bool {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            return positions
                .iter()
                .any(|p| p.mint == mint && p.position_type == "buy" && p.exit_price.is_none());
        }
        false
    }

    /// Check a single position token and return updated token data
    async fn check_single_position_token(
        &self,
        mint: &str,
        shutdown: Arc<Notify>
    ) -> Option<Token> {
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(0)).await {
            return None;
        }

        // Acquire rate limit permit
        let permit = match
            tokio::time::timeout(
                Duration::from_secs(3),
                self.info_rate_limiter.clone().acquire_owned()
            ).await
        {
            Ok(Ok(permit)) => permit,
            _ => {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!("Failed to acquire rate limit permit for position token {}", mint)
                );
                return None;
            }
        };

        // Fetch updated token info from DexScreener
        let updated_token = match self.fetch_position_token_info(mint).await {
            Ok(Some(updated)) => Some(updated),
            Ok(None) => {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!("No data returned for position token {}", mint)
                );
                None
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Failed to fetch position token {}: {}", mint, e)
                );
                None
            }
        };

        drop(permit); // Release permit

        // Small delay to be gentle on the API (shorter for position monitoring)
        sleep(Duration::from_millis(200)).await;

        updated_token
    }

    /// Fetch token info from DexScreener API for position tokens
    async fn fetch_position_token_info(&self, mint: &str) -> Result<Option<Token>, String> {
        let chain_id = "solana";
        let url = format!("https://api.dexscreener.com/tokens/v1/{}/{}", chain_id, mint);

        let client = reqwest::Client
            ::builder()
            .timeout(Duration::from_secs(8))
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

    /// Parse token data from DexScreener API response (pair structure)
    fn parse_token_from_api(&self, pair_data: &serde_json::Value) -> Result<Option<Token>, String> {
        // The API returns pairs, so we need to extract token info from baseToken
        let base_token = pair_data.get("baseToken").ok_or("Missing baseToken field")?;

        let mint = base_token
            .get("address")
            .and_then(|a| a.as_str())
            .ok_or("Missing token address field")?
            .to_string();

        let symbol = base_token
            .get("symbol")
            .and_then(|s| s.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();

        let name = base_token
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Token")
            .to_string();

        let decimals = 9; // Default to 9, we'll need to fetch this separately if needed

        // Parse liquidity data from pair level
        let liquidity = pair_data.get("liquidity").map(|liquidity_obj| {
            crate::global::LiquidityInfo {
                usd: liquidity_obj.get("usd").and_then(|v| v.as_f64()),
                base: liquidity_obj.get("base").and_then(|v| v.as_f64()),
                quote: liquidity_obj.get("quote").and_then(|v| v.as_f64()),
            }
        });

        // Parse price data from pair level
        let price_dexscreener_sol = pair_data.get("priceNative").and_then(|v| v.as_f64());
        let price_dexscreener_usd = pair_data.get("priceUsd").and_then(|v| v.as_f64());

        // Parse created_at from pair
        let created_at = pair_data
            .get("pairCreatedAt")
            .and_then(|v| v.as_i64())
            .and_then(|ts| chrono::DateTime::from_timestamp_millis(ts));

        // Create token struct with essential fields
        let token = Token {
            mint,
            symbol,
            name,
            decimals,
            chain: "solana".to_string(),
            logo_url: None, // We'd need to parse this from info if available
            coingecko_id: None,
            website: None,
            description: None,
            tags: Vec::new(),
            is_verified: false,
            created_at,
            price_dexscreener_sol,
            price_dexscreener_usd,
            price_pool_sol: None,
            price_pool_usd: None,
            pools: Vec::new(),
            dex_id: pair_data
                .get("dexId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            pair_address: pair_data
                .get("pairAddress")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            pair_url: pair_data
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            labels: pair_data
                .get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            fdv: pair_data.get("fdv").and_then(|v| v.as_f64()),
            market_cap: pair_data.get("marketCap").and_then(|v| v.as_f64()),
            txns: None, // Could be parsed if needed
            volume: None, // Could be parsed if needed
            price_change: None, // Could be parsed if needed
            liquidity,
            info: None,
            boosts: None,
        };

        Ok(Some(token))
    }

    /// Handle tokens whose positions were closed - check if they should return to main watch list
    async fn handle_closed_position_tokens(&self, closed_mints: Vec<String>) {
        log(
            LogTag::Trader,
            "INFO",
            &format!("Processing {} closed position tokens for main watch list", closed_mints.len())
        );

        for mint in closed_mints {
            // Check if token has sufficient liquidity to return to main watch list
            if let Some(token) = self.fetch_position_token_info(&mint).await.unwrap_or(None) {
                let liquidity_usd = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);

                if
                    liquidity_usd >= Self::MIN_LIQUIDITY_FOR_MAIN_WATCH &&
                    !is_token_blacklisted(&mint)
                {
                    // Add token back to main watch list
                    if let Ok(mut list_tokens) = LIST_TOKENS.try_write() {
                        // Check if token is already in the list
                        if !list_tokens.iter().any(|t| t.mint == token.mint) {
                            list_tokens.push(token.clone());
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!(
                                    "Added {} back to main watch list (${:.0} liquidity)",
                                    token.symbol,
                                    liquidity_usd
                                )
                            );
                        }
                    }
                } else {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!(
                            "Token {} not returned to watch list (${:.0} liquidity < ${:.0} threshold)",
                            mint,
                            liquidity_usd,
                            Self::MIN_LIQUIDITY_FOR_MAIN_WATCH
                        )
                    );
                }
            }
        }
    }

    /// Update position tokens in LIST_TOKENS (priority update)
    async fn update_position_tokens_in_list(&self, updated_tokens: Vec<Token>) {
        tokio::spawn(async move {
            if let Ok(mut list_tokens) = LIST_TOKENS.try_write() {
                // Create a map for quick lookup
                let token_map: HashMap<String, Token> = updated_tokens
                    .into_iter()
                    .map(|token| (token.mint.clone(), token))
                    .collect();

                // Update existing position tokens in LIST_TOKENS (priority update)
                let mut updated_count = 0;
                for existing_token in list_tokens.iter_mut() {
                    if let Some(updated_token) = token_map.get(&existing_token.mint) {
                        *existing_token = updated_token.clone();
                        updated_count += 1;
                    }
                }

                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!("Fast updated {} position tokens in global list", updated_count)
                );
            } else {
                log(
                    LogTag::Trader,
                    "WARN",
                    "Could not acquire write lock for LIST_TOKENS (position update)"
                );
            }
        });
    }

    /// Wait for next position monitoring cycle (faster than main monitoring)
    async fn wait_for_next_position_cycle(&self, shutdown: Arc<Notify>) {
        let cycle_duration = Duration::from_secs(Self::POSITION_CYCLE_DURATION_SECONDS);

        if check_shutdown_or_delay(&shutdown, cycle_duration).await {
            return;
        }
    }
}

/// Get list of open position token mints for exclusion from main watch list
pub fn get_open_position_mints() -> HashSet<String> {
    let mut position_mints = HashSet::new();

    if let Ok(positions) = SAVED_POSITIONS.lock() {
        for position in positions.iter() {
            // Only exclude tokens with open positions
            if position.position_type == "buy" && position.exit_price.is_none() {
                position_mints.insert(position.mint.clone());
            }
        }
    }

    position_mints
}

/// Start the position monitoring background task
pub async fn start_position_monitoring(shutdown: Arc<Notify>) {
    let mut monitor = PositionMonitor::new();
    monitor.start_position_monitoring(shutdown).await;
}
