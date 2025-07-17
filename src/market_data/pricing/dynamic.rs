use std::sync::Arc;
use std::time::{ Duration, Instant };
use std::collections::{ HashMap, BTreeMap };
use tokio::sync::{ RwLock, Mutex };
use serde::{ Deserialize, Serialize };
use chrono::Utc;
use crate::config::DynamicPricingConfig;
use crate::database::Database;
use crate::logger::Logger;
use crate::market_data::sources::GeckoTerminalClient;
use crate::market_data::models::TokenPrice;

/// Dynamic pricing manager that continuously updates token prices based on liquidity
pub struct DynamicPricingManager {
    config: DynamicPricingConfig,
    gecko_client: Arc<GeckoTerminalClient>,
    database: Arc<Database>,
    logger: Arc<Logger>,

    // Token tracking
    token_priorities: Arc<RwLock<BTreeMap<String, TokenPriority>>>,
    blacklisted_tokens: Arc<RwLock<HashMap<String, BlacklistInfo>>>,

    // Rate limiting
    rate_limiter: Arc<Mutex<RateLimiter>>,

    // Update scheduling
    update_scheduler: Arc<RwLock<UpdateScheduler>>,

    // Statistics
    stats: Arc<RwLock<DynamicPricingStats>>,

    // Placeholder for other price sources
    other_sources: Vec<Box<dyn PriceSource + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct TokenPriority {
    pub address: String,
    pub liquidity_usd: f64,
    pub last_updated: Instant,
    pub update_interval: Duration,
    pub consecutive_failures: u32,
    pub priority_score: f64,
    pub dead_since: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct BlacklistInfo {
    pub address: String,
    pub reason: BlacklistReason,
    pub blacklisted_at: Instant,
    pub last_liquidity: f64,
    pub consecutive_zero_liquidity_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlacklistReason {
    DeadToken,
    ZeroLiquidity,
    ConsecutiveFailures,
    ManuallyBlacklisted,
}

#[derive(Debug)]
pub struct RateLimiter {
    requests_per_minute: u32,
    requests_per_hour: u32,
    burst_limit: u32,

    minute_requests: HashMap<u64, u32>,
    hour_requests: HashMap<u64, u32>,
    burst_tokens: u32,
    last_reset: Instant,
}

#[derive(Debug)]
pub struct UpdateScheduler {
    pending_updates: BTreeMap<Instant, Vec<String>>,
    current_batch: Vec<String>,
    next_update_time: Instant,
}

#[derive(Debug, Clone)]
pub struct DynamicPricingStats {
    pub total_tokens_tracked: usize,
    pub high_priority_tokens: usize,
    pub medium_priority_tokens: usize,
    pub low_priority_tokens: usize,
    pub blacklisted_tokens: usize,
    pub requests_made_last_minute: u32,
    pub requests_made_last_hour: u32,
    pub rate_limit_usage_percentage: f64,
    pub average_update_interval: Duration,
    pub last_update_time: Option<Instant>,
}

// Placeholder trait for other price sources
pub trait PriceSource {
    fn get_name(&self) -> &str;
    fn get_token_price(
        &self,
        token_address: &str
    ) -> Result<TokenPrice, Box<dyn std::error::Error>>;
    fn get_rate_limit_info(&self) -> RateLimitInfo;
}

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub requests_remaining: u32,
    pub reset_time: Instant,
    pub usage_percentage: f64,
}

impl DynamicPricingManager {
    pub fn new(
        config: DynamicPricingConfig,
        gecko_client: Arc<GeckoTerminalClient>,
        database: Arc<Database>,
        logger: Arc<Logger>
    ) -> Self {
        let rate_limiter = RateLimiter::new(
            config.gecko_terminal_rate_limit.requests_per_minute,
            config.gecko_terminal_rate_limit.requests_per_hour,
            config.gecko_terminal_rate_limit.burst_limit
        );

        Self {
            config,
            gecko_client,
            database,
            logger,
            token_priorities: Arc::new(RwLock::new(BTreeMap::new())),
            blacklisted_tokens: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(Mutex::new(rate_limiter)),
            update_scheduler: Arc::new(RwLock::new(UpdateScheduler::new())),
            stats: Arc::new(RwLock::new(DynamicPricingStats::default())),
            other_sources: Vec::new(),
        }
    }

    /// Start the dynamic pricing manager
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        Logger::info("Starting dynamic pricing manager");

        // Load existing token priorities from database
        self.load_token_priorities().await?;

        // Start main update loop
        let manager = self.clone();
        tokio::spawn(async move {
            manager.main_update_loop().await;
        });

        // Start blacklist cleanup task
        let manager = self.clone();
        tokio::spawn(async move {
            manager.blacklist_cleanup_task().await;
        });

        // Start statistics reporting
        let manager = self.clone();
        tokio::spawn(async move {
            manager.stats_reporting_task().await;
        });

        // Start token reloading task
        let manager = self.clone();
        tokio::spawn(async move {
            manager.token_reloading_task().await;
        });

        Ok(())
    }

    /// Main update loop that continuously updates token prices
    async fn main_update_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            if let Err(e) = self.process_update_cycle().await {
                Logger::error(&format!("Error in update cycle: {}", e));
            }
        }
    }

    /// Process a single update cycle
    async fn process_update_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();

        // Check if we have capacity for updates
        if !self.can_make_request().await? {
            tokio::time::sleep(Duration::from_millis(100)).await;
            return Ok(());
        }

        // Get tokens that need updating
        let tokens_to_update = self.get_tokens_for_update(now).await?;

        if tokens_to_update.is_empty() {
            return Ok(());
        }

        // Sort tokens by priority (liquidity-based)
        let mut sorted_tokens = tokens_to_update;
        let priorities = self.token_priorities.read().await;
        sorted_tokens.sort_by(|a, b| {
            let priority_a = priorities
                .get(a)
                .map(|p| p.priority_score)
                .unwrap_or(0.0);
            let priority_b = priorities
                .get(b)
                .map(|p| p.priority_score)
                .unwrap_or(0.0);
            priority_b.partial_cmp(&priority_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        drop(priorities);

        // Update tokens in batches
        for token_address in sorted_tokens {
            if !self.can_make_request().await? {
                break;
            }

            self.update_token_price(&token_address).await?;

            // Small delay to prevent overwhelming the API
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(())
    }

    /// Update price for a specific token
    async fn update_token_price(
        &self,
        token_address: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();

        // Get previous price for comparison
        let previous_price = match self.database.get_token_price(token_address).await {
            Ok(Some(price)) => Some(price),
            _ => None,
        };

        // Consume rate limit
        self.consume_rate_limit().await?;

        // Get price from GeckoTerminal
        let price_result = self.gecko_client.get_token_info(token_address).await;

        match price_result {
            Ok(token_info) => {
                if let Some(price) = token_info.price {
                    // Update token priority based on new liquidity info
                    self.update_token_priority(token_address, &price).await?;

                    // Check if token should be blacklisted
                    self.check_token_health(token_address, &price).await?;

                    // Store price in database
                    self.store_token_price(token_address, &price).await?;

                    // Enhanced logging with price change detection
                    if let Some(prev_price) = previous_price {
                        let price_change = price.price_usd - prev_price.price_usd;
                        let price_change_percent = if prev_price.price_usd > 0.0 {
                            (price_change / prev_price.price_usd) * 100.0
                        } else {
                            0.0
                        };

                        // Log significant price changes
                        if price_change_percent.abs() > 0.5 {
                            let change_symbol = if price_change > 0.0 { "🚀" } else { "📉" };
                            let urgency = if price_change_percent.abs() > 10.0 { "🔥" } else { "💫" };
                            
                            Logger::info(
                                &format!(
                                    "{} {} DYNAMIC PRICE UPDATE: {} | ${:.8} → ${:.8} ({:+.2}%) | Liquidity: ${:.0}",
                                    change_symbol,
                                    urgency,
                                    token_address,
                                    prev_price.price_usd,
                                    price.price_usd,
                                    price_change_percent,
                                    price.liquidity_usd
                                )
                            );
                        } else {
                            Logger::debug(
                                &format!(
                                    "⚡ Dynamic update for {} - ${:.8} | Liquidity: ${:.0}",
                                    token_address,
                                    price.price_usd,
                                    price.liquidity_usd
                                )
                            );
                        }
                    } else {
                        Logger::info(
                            &format!(
                                "🎯 NEW DYNAMIC TRACKING: {} | ${:.8} | Liquidity: ${:.0}",
                                token_address,
                                price.price_usd,
                                price.liquidity_usd
                            )
                        );
                    }
                } else {
                    Logger::warn(&format!("⚠️  No price data available for token {}", token_address));
                }
            }
            Err(e) => {
                self.handle_update_failure(token_address, &format!("{}", e)).await?;
                Logger::warn(&format!("❌ Failed to update price for token {}: {}", token_address, e));
            }
        }

        // Update statistics
        self.update_stats(start_time).await?;

        Ok(())
    }

    /// Calculate dynamic update interval based on liquidity
    fn calculate_update_interval(&self, liquidity_usd: f64) -> Duration {
        let fastest = self.config.fastest_interval_secs;
        let slowest = self.config.slowest_interval_secs;
        let high_threshold = self.config.high_liquidity_threshold;
        let low_threshold = self.config.low_liquidity_threshold;

        let interval_secs = if liquidity_usd >= high_threshold {
            fastest // High liquidity = fastest updates (5 seconds)
        } else if liquidity_usd <= low_threshold {
            slowest // Low liquidity = slowest updates (5 minutes)
        } else {
            // Linear interpolation between thresholds
            let ratio = (liquidity_usd - low_threshold) / (high_threshold - low_threshold);
            let interval_range = (slowest as f64) - (fastest as f64);
            let calculated_interval = (slowest as f64) - ratio * interval_range;
            calculated_interval as u64
        };

        Duration::from_secs(interval_secs)
    }

    /// Calculate priority score for token (higher = more priority)
    fn calculate_priority_score(&self, liquidity_usd: f64, volume_24h: f64) -> f64 {
        // Base score from liquidity (logarithmic scale)
        let liquidity_score = if liquidity_usd > 0.0 { (liquidity_usd + 1.0).ln() } else { 0.0 };

        // Volume bonus (also logarithmic)
        let volume_score = if volume_24h > 0.0 { (volume_24h + 1.0).ln() * 0.5 } else { 0.0 };

        liquidity_score + volume_score
    }

    /// Check if we can make another request within rate limits
    async fn can_make_request(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let rate_limiter = self.rate_limiter.lock().await;
        let usage = rate_limiter.get_usage_percentage();

        Ok(usage < self.config.rate_limit_usage_threshold)
    }

    /// Get tokens that need updating based on their schedule
    async fn get_tokens_for_update(
        &self,
        now: Instant
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let priorities = self.token_priorities.read().await;
        let blacklisted = self.blacklisted_tokens.read().await;

        let mut tokens_to_update = Vec::new();

        for (address, priority) in priorities.iter() {
            // Skip blacklisted tokens
            if blacklisted.contains_key(address) {
                continue;
            }

            // Check if token needs updating
            if now.duration_since(priority.last_updated) >= priority.update_interval {
                tokens_to_update.push(address.clone());
            }
        }

        Ok(tokens_to_update)
    }

    /// Update token priority based on latest price data
    async fn update_token_priority(
        &self,
        token_address: &str,
        price: &TokenPrice
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut priorities = self.token_priorities.write().await;
        let now = Instant::now();

        let update_interval = self.calculate_update_interval(price.liquidity_usd);
        let priority_score = self.calculate_priority_score(price.liquidity_usd, price.volume_24h);

        let token_priority = TokenPriority {
            address: token_address.to_string(),
            liquidity_usd: price.liquidity_usd,
            last_updated: now,
            update_interval,
            consecutive_failures: 0,
            priority_score,
            dead_since: None,
        };

        priorities.insert(token_address.to_string(), token_priority);

        Ok(())
    }

    /// Check if token should be blacklisted due to low/zero liquidity
    async fn check_token_health(
        &self,
        token_address: &str,
        price: &TokenPrice
    ) -> Result<(), Box<dyn std::error::Error>> {
        if price.liquidity_usd <= self.config.dead_token_threshold {
            let mut priorities = self.token_priorities.write().await;
            let mut blacklisted = self.blacklisted_tokens.write().await;

            // Mark token as potentially dead
            if let Some(priority) = priorities.get_mut(token_address) {
                if priority.dead_since.is_none() {
                    priority.dead_since = Some(Instant::now());
                } else {
                    // Check if token has been dead for too long
                    let dead_duration = Instant::now().duration_since(priority.dead_since.unwrap());
                    if
                        dead_duration >=
                        Duration::from_secs(self.config.dead_token_timeout_hours * 3600)
                    {
                        // Blacklist the token
                        let blacklist_info = BlacklistInfo {
                            address: token_address.to_string(),
                            reason: BlacklistReason::DeadToken,
                            blacklisted_at: Instant::now(),
                            last_liquidity: price.liquidity_usd,
                            consecutive_zero_liquidity_hours: self.config.dead_token_timeout_hours,
                        };

                        blacklisted.insert(token_address.to_string(), blacklist_info);
                        priorities.remove(token_address);

                        Logger::info(
                            &format!(
                                "Blacklisted dead token {} - Liquidity: ${:.2}",
                                token_address,
                                price.liquidity_usd
                            )
                        );
                    }
                }
            }
        } else {
            // Token is alive, remove dead marker
            let mut priorities = self.token_priorities.write().await;
            if let Some(priority) = priorities.get_mut(token_address) {
                priority.dead_since = None;
            }
        }

        Ok(())
    }

    /// Handle update failure for a token
    async fn handle_update_failure(
        &self,
        token_address: &str,
        _error: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut priorities = self.token_priorities.write().await;

        if let Some(priority) = priorities.get_mut(token_address) {
            priority.consecutive_failures += 1;

            // If too many failures, reduce priority or blacklist
            if priority.consecutive_failures >= 10 {
                let mut blacklisted = self.blacklisted_tokens.write().await;
                let blacklist_info = BlacklistInfo {
                    address: token_address.to_string(),
                    reason: BlacklistReason::ConsecutiveFailures,
                    blacklisted_at: Instant::now(),
                    last_liquidity: priority.liquidity_usd,
                    consecutive_zero_liquidity_hours: 0,
                };

                blacklisted.insert(token_address.to_string(), blacklist_info);
                priorities.remove(token_address);

                Logger::warn(
                    &format!("Blacklisted token {} due to consecutive failures", token_address)
                );
            }
        }

        Ok(())
    }

    /// Store token price in database
    async fn store_token_price(
        &self,
        token_address: &str,
        price: &TokenPrice
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Store in database (implementation depends on your database schema)
        // For now, this is a placeholder
        match self.database.store_token_price(token_address, price).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Logger::error(&format!("Failed to store token price in database: {}", e));
                Err(e.into())
            }
        }
    }

    /// Consume rate limit token
    async fn consume_rate_limit(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.consume_token()?;
        Ok(())
    }

    /// Load token priorities from database
    async fn load_token_priorities(&self) -> Result<(), Box<dyn std::error::Error>> {
        Logger::info("🔄 Loading token priorities from database...");
        
        // Load from existing price data (tokens with pricing history)
        match self.database.get_tracked_tokens().await {
            Ok(tokens) => {
                let mut priorities = self.token_priorities.write().await;
                Logger::info(&format!("📊 Found {} tokens with existing price data", tokens.len()));

                for token in tokens {
                    let update_interval = self.calculate_update_interval(token.liquidity_usd);
                    let priority_score = self.calculate_priority_score(
                        token.liquidity_usd,
                        token.volume_24h
                    );

                    let token_priority = TokenPriority {
                        address: token.address.clone(),
                        liquidity_usd: token.liquidity_usd,
                        last_updated: Instant::now() -
                        Duration::from_secs(update_interval.as_secs()),
                        update_interval,
                        consecutive_failures: 0,
                        priority_score,
                        dead_since: None,
                    };

                    priorities.insert(token.address, token_priority);
                }
            }
            Err(e) => {
                Logger::warn(&format!("Failed to load tokens with price data: {}", e));
            }
        }

        // Load newly discovered tokens from mints table (tokens without price data yet)
        match self.database.get_all_mints() {
            Ok(mints) => {
                let mut priorities = self.token_priorities.write().await;
                let mut new_tokens_added = 0;

                Logger::info(&format!("🔍 Found {} total mint addresses", mints.len()));

                for mint in mints {
                    // Skip if we already have this token in priorities
                    if !priorities.contains_key(&mint) {
                        // Add new token with default values (will be updated on first price fetch)
                        let update_interval = self.calculate_update_interval(0.0); // Start with minimum priority
                        let priority_score = self.calculate_priority_score(0.0, 0.0);

                        let token_priority = TokenPriority {
                            address: mint.clone(),
                            liquidity_usd: 0.0, // Will be updated on first price fetch
                            last_updated: Instant::now() - update_interval, // Schedule immediate update
                            update_interval,
                            consecutive_failures: 0,
                            priority_score,
                            dead_since: None,
                        };

                        priorities.insert(mint, token_priority);
                        new_tokens_added += 1;
                    }
                }

                Logger::info(&format!("🆕 Added {} new tokens for tracking", new_tokens_added));
                Logger::info(&format!("📈 Total tokens being tracked: {}", priorities.len()));
            }
            Err(e) => {
                Logger::warn(&format!("Failed to load mint addresses: {}", e));
            }
        }

        Ok(())
    }

    /// Blacklist cleanup task
    async fn blacklist_cleanup_task(&self) {
        let mut interval = tokio::time::interval(
            Duration::from_secs(self.config.blacklist_cleanup_interval_hours * 3600)
        );

        loop {
            interval.tick().await;

            if let Err(e) = self.cleanup_blacklist().await {
                Logger::error(&format!("Error cleaning up blacklist: {}", e));
            }
        }
    }

    /// Clean up old blacklisted tokens
    async fn cleanup_blacklist(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut blacklisted = self.blacklisted_tokens.write().await;
        let now = Instant::now();
        let cleanup_threshold = Duration::from_secs(24 * 3600); // 24 hours

        blacklisted.retain(|_, info| {
            now.duration_since(info.blacklisted_at) < cleanup_threshold
        });

        Ok(())
    }

    /// Update statistics
    async fn update_stats(&self, start_time: Instant) -> Result<(), Box<dyn std::error::Error>> {
        let mut stats = self.stats.write().await;
        let priorities = self.token_priorities.read().await;
        let blacklisted = self.blacklisted_tokens.read().await;
        let rate_limiter = self.rate_limiter.lock().await;

        stats.total_tokens_tracked = priorities.len();
        stats.blacklisted_tokens = blacklisted.len();
        stats.rate_limit_usage_percentage = rate_limiter.get_usage_percentage();
        stats.last_update_time = Some(start_time);

        // Calculate priority distribution
        let mut high_priority = 0;
        let mut medium_priority = 0;
        let mut low_priority = 0;

        for priority in priorities.values() {
            if priority.liquidity_usd >= self.config.high_liquidity_threshold {
                high_priority += 1;
            } else if priority.liquidity_usd >= self.config.low_liquidity_threshold {
                medium_priority += 1;
            } else {
                low_priority += 1;
            }
        }

        stats.high_priority_tokens = high_priority;
        stats.medium_priority_tokens = medium_priority;
        stats.low_priority_tokens = low_priority;

        Ok(())
    }

    /// Statistics reporting task
    async fn stats_reporting_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Report every minute

        loop {
            interval.tick().await;

            let stats = self.stats.read().await;
            let priorities = self.token_priorities.read().await;
            let _blacklisted = self.blacklisted_tokens.read().await;
            let rate_limiter = self.rate_limiter.lock().await;

            // Count tokens by priority levels
            let mut high_priority = 0;
            let mut medium_priority = 0;
            let mut low_priority = 0;
            let mut total_liquidity = 0.0;

            for priority in priorities.values() {
                total_liquidity += priority.liquidity_usd;
                if priority.liquidity_usd >= self.config.high_liquidity_threshold {
                    high_priority += 1;
                } else if priority.liquidity_usd >= self.config.low_liquidity_threshold {
                    medium_priority += 1;
                } else {
                    low_priority += 1;
                }
            }

            let rate_usage = rate_limiter.get_usage_percentage();
            
            Logger::info(
                &format!(
                    "🔥 DYNAMIC PRICING STATS | Total: {} tokens | High Priority: {} ({}s) | Medium: {} | Low: {} ({}s) | Blacklisted: {} | Rate Limit: {:.1}% | Total Liquidity: ${:.0}M",
                    stats.total_tokens_tracked,
                    high_priority,
                    self.config.fastest_interval_secs,
                    medium_priority,
                    low_priority,
                    self.config.slowest_interval_secs,
                    stats.blacklisted_tokens,
                    rate_usage * 100.0,
                    total_liquidity / 1_000_000.0
                )
            );

            // Additional detailed stats every 5 minutes
            if chrono::Utc::now().timestamp() % 300 == 0 {
                let top_tokens: Vec<_> = priorities
                    .iter()
                    .filter(|(_, p)| p.liquidity_usd > 100_000.0)
                    .take(5)
                    .collect();

                if !top_tokens.is_empty() {
                    Logger::info("🏆 TOP TRACKED TOKENS:");
                    for (addr, priority) in top_tokens {
                        Logger::info(
                            &format!(
                                "   {} | Liquidity: ${:.0}K | Update: {}s | Score: {:.1}",
                                &addr[..8],
                                priority.liquidity_usd / 1000.0,
                                priority.update_interval.as_secs(),
                                priority.priority_score
                            )
                        );
                    }
                }
            }
        }
    }

    /// Token reloading task - periodically checks for new tokens from discovery
    async fn token_reloading_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Check every 5 minutes

        loop {
            interval.tick().await;

            Logger::debug("🔄 Checking for new tokens from discovery module...");

            // Load new tokens from mints table
            match self.database.get_all_mints() {
                Ok(mints) => {
                    let mut priorities = self.token_priorities.write().await;
                    let mut new_tokens_added = 0;

                    for mint in mints {
                        // Skip if we already have this token in priorities
                        if !priorities.contains_key(&mint) {
                            // Add new token with default values (will be updated on first price fetch)
                            let update_interval = self.calculate_update_interval(0.0); // Start with minimum priority
                            let priority_score = self.calculate_priority_score(0.0, 0.0);

                            let token_priority = TokenPriority {
                                address: mint.clone(),
                                liquidity_usd: 0.0, // Will be updated on first price fetch
                                last_updated: Instant::now() - update_interval, // Schedule immediate update
                                update_interval,
                                consecutive_failures: 0,
                                priority_score,
                                dead_since: None,
                            };

                            priorities.insert(mint, token_priority);
                            new_tokens_added += 1;
                        }
                    }

                    if new_tokens_added > 0 {
                        Logger::info(&format!("🆕 Added {} new tokens for tracking", new_tokens_added));
                        Logger::info(&format!("📈 Total tokens being tracked: {}", priorities.len()));
                    }
                }
                Err(e) => {
                    Logger::warn(&format!("Failed to reload mint addresses: {}", e));
                }
            }
        }
    }

    /// Add a new token to track
    pub async fn add_token(
        &self,
        token_address: String,
        initial_liquidity: f64
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut priorities = self.token_priorities.write().await;
        let update_interval = self.calculate_update_interval(initial_liquidity);
        let priority_score = self.calculate_priority_score(initial_liquidity, 0.0);

        let token_priority = TokenPriority {
            address: token_address.clone(),
            liquidity_usd: initial_liquidity,
            last_updated: Instant::now() - update_interval, // Schedule immediate update
            update_interval,
            consecutive_failures: 0,
            priority_score,
            dead_since: None,
        };

        priorities.insert(token_address, token_priority);

        Ok(())
    }

    /// Remove a token from tracking
    pub async fn remove_token(
        &self,
        token_address: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut priorities = self.token_priorities.write().await;
        priorities.remove(token_address);

        Ok(())
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> DynamicPricingStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

// Clone implementation for DynamicPricingManager
impl Clone for DynamicPricingManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            gecko_client: self.gecko_client.clone(),
            database: self.database.clone(),
            logger: self.logger.clone(),
            token_priorities: self.token_priorities.clone(),
            blacklisted_tokens: self.blacklisted_tokens.clone(),
            rate_limiter: self.rate_limiter.clone(),
            update_scheduler: self.update_scheduler.clone(),
            stats: self.stats.clone(),
            other_sources: Vec::new(), // Empty for clone
        }
    }
}

impl RateLimiter {
    fn new(requests_per_minute: u32, requests_per_hour: u32, burst_limit: u32) -> Self {
        Self {
            requests_per_minute,
            requests_per_hour,
            burst_limit,
            minute_requests: HashMap::new(),
            hour_requests: HashMap::new(),
            burst_tokens: burst_limit,
            last_reset: Instant::now(),
        }
    }

    fn consume_token(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.cleanup_old_requests();

        let now = Instant::now();
        let current_minute = now.duration_since(self.last_reset).as_secs() / 60;
        let current_hour = now.duration_since(self.last_reset).as_secs() / 3600;

        // Check minute limit
        let minute_count = self.minute_requests.get(&current_minute).unwrap_or(&0);
        if *minute_count >= self.requests_per_minute {
            return Err("Rate limit exceeded: requests per minute".into());
        }

        // Check hour limit
        let hour_count = self.hour_requests.get(&current_hour).unwrap_or(&0);
        if *hour_count >= self.requests_per_hour {
            return Err("Rate limit exceeded: requests per hour".into());
        }

        // Check burst limit
        if self.burst_tokens == 0 {
            return Err("Rate limit exceeded: burst limit".into());
        }

        // Consume tokens
        self.minute_requests.insert(current_minute, minute_count + 1);
        self.hour_requests.insert(current_hour, hour_count + 1);
        self.burst_tokens = self.burst_tokens.saturating_sub(1);

        Ok(())
    }

    fn get_usage_percentage(&self) -> f64 {
        let now = Instant::now();
        let current_minute = now.duration_since(self.last_reset).as_secs() / 60;
        let current_hour = now.duration_since(self.last_reset).as_secs() / 3600;

        let minute_usage =
            (*self.minute_requests.get(&current_minute).unwrap_or(&0) as f64) /
            (self.requests_per_minute as f64);
        let hour_usage =
            (*self.hour_requests.get(&current_hour).unwrap_or(&0) as f64) /
            (self.requests_per_hour as f64);
        let burst_usage =
            ((self.burst_limit - self.burst_tokens) as f64) / (self.burst_limit as f64);

        minute_usage.max(hour_usage).max(burst_usage)
    }

    fn cleanup_old_requests(&mut self) {
        let now = Instant::now();
        let current_minute = now.duration_since(self.last_reset).as_secs() / 60;
        let current_hour = now.duration_since(self.last_reset).as_secs() / 3600;

        // Keep only recent requests
        self.minute_requests.retain(|&k, _| k >= current_minute.saturating_sub(1));
        self.hour_requests.retain(|&k, _| k >= current_hour.saturating_sub(1));

        // Reset burst tokens periodically
        if now.duration_since(self.last_reset) >= Duration::from_secs(60) {
            self.burst_tokens = self.burst_limit;
            self.last_reset = now;
        }
    }
}

impl UpdateScheduler {
    fn new() -> Self {
        Self {
            pending_updates: BTreeMap::new(),
            current_batch: Vec::new(),
            next_update_time: Instant::now(),
        }
    }
}

impl Default for DynamicPricingStats {
    fn default() -> Self {
        Self {
            total_tokens_tracked: 0,
            high_priority_tokens: 0,
            medium_priority_tokens: 0,
            low_priority_tokens: 0,
            blacklisted_tokens: 0,
            requests_made_last_minute: 0,
            requests_made_last_hour: 0,
            rate_limit_usage_percentage: 0.0,
            average_update_interval: Duration::from_secs(60),
            last_update_time: None,
        }
    }
}

// Placeholder implementations for other price sources
pub struct JupiterPriceSource {
    name: String,
}

impl JupiterPriceSource {
    pub fn new() -> Self {
        Self {
            name: "Jupiter".to_string(),
        }
    }
}

impl PriceSource for JupiterPriceSource {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_token_price(
        &self,
        _token_address: &str
    ) -> Result<TokenPrice, Box<dyn std::error::Error>> {
        // Placeholder implementation
        Err("Jupiter price source not implemented yet".into())
    }

    fn get_rate_limit_info(&self) -> RateLimitInfo {
        RateLimitInfo {
            requests_remaining: 100,
            reset_time: Instant::now() + Duration::from_secs(60),
            usage_percentage: 0.0,
        }
    }
}

pub struct RaydiumPriceSource {
    name: String,
}

impl RaydiumPriceSource {
    pub fn new() -> Self {
        Self {
            name: "Raydium".to_string(),
        }
    }
}

impl PriceSource for RaydiumPriceSource {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_token_price(
        &self,
        _token_address: &str
    ) -> Result<TokenPrice, Box<dyn std::error::Error>> {
        // Placeholder implementation
        Err("Raydium price source not implemented yet".into())
    }

    fn get_rate_limit_info(&self) -> RateLimitInfo {
        RateLimitInfo {
            requests_remaining: 50,
            reset_time: Instant::now() + Duration::from_secs(60),
            usage_percentage: 0.0,
        }
    }
}
