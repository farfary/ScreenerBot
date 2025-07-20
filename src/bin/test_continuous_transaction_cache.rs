// bin/test_continuous_transaction_cache.rs - Continuous background transaction caching
use screenerbot::transactions::*;
use screenerbot::global::{ read_configs };
use screenerbot::wallet::get_wallet_address;
use screenerbot::logger::{ log, LogTag };
use tokio::time::{ Duration, sleep };
use std::sync::Arc;
use tokio::sync::{ Mutex, Semaphore };
use chrono::{ Utc, DateTime };
use std::collections::HashSet;

/// Configuration for continuous transaction caching
#[derive(Debug, Clone)]
pub struct CachingConfig {
    pub target_transaction_count: usize,
    pub batch_size: usize,
    pub rate_limit_per_second: u32,
    pub max_concurrent_requests: usize,
    pub historical_days: u32,
    pub cache_update_interval_secs: u64,
    pub status_report_interval_secs: u64,
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            target_transaction_count: 1000,
            batch_size: 100,
            rate_limit_per_second: 10, // Conservative for mainnet RPC
            max_concurrent_requests: 3,
            historical_days: 7,
            cache_update_interval_secs: 30,
            status_report_interval_secs: 300, // 5 minutes
        }
    }
}

/// Background transaction caching system
pub struct ContinuousTransactionCache {
    config: CachingConfig,
    db: Arc<Mutex<TransactionDatabase>>,
    fetcher: Arc<TransactionFetcher>,
    rate_limiter: Arc<Semaphore>,
    wallet_address: String,
    cached_signatures: Arc<Mutex<HashSet<String>>>,
    stats: Arc<Mutex<CachingStats>>,
    configs: screenerbot::global::Configs,
    client: reqwest::Client,
}

/// Statistics for caching performance
#[derive(Debug, Default, Clone)]
pub struct CachingStats {
    pub total_cached: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub rpc_calls: usize,
    pub errors: usize,
    pub last_update: Option<DateTime<Utc>>,
    pub oldest_transaction: Option<DateTime<Utc>>,
    pub newest_transaction: Option<DateTime<Utc>>,
}

impl ContinuousTransactionCache {
    /// Create a new continuous transaction cache
    pub async fn new(config: CachingConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Load configs and ensure we use official mainnet RPC
        let configs = read_configs("configs.json")?;

        // Verify we're using official mainnet RPC
        if !configs.rpc_url.contains("api.mainnet-beta.solana.com") {
            log(
                LogTag::System,
                "WARNING",
                &format!("RPC URL is not official mainnet: {}", configs.rpc_url)
            );
            log(
                LogTag::System,
                "INFO",
                "Proceeding with configured RPC, but recommend using wss://api.mainnet-beta.solana.com"
            );
        }

        let wallet_address = get_wallet_address().map_err(
            |e|
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                    dyn std::error::Error
                >
        )?;

        let db = Arc::new(Mutex::new(TransactionDatabase::new()?));

        // Create fetcher with rate limiting
        let batch_config = BatchConfig {
            batch_size: 100,
            max_concurrent: 10,
            delay_between_batches_ms: 1000,
            delay_between_requests_ms: 100,
            max_retries: 3,
        };

        let fetcher = Arc::new(TransactionFetcher::new(configs.clone(), Some(batch_config))?);
        let rate_limiter = Arc::new(Semaphore::new(config.max_concurrent_requests));
        let client = reqwest::Client::new();

        Ok(Self {
            config,
            db,
            fetcher,
            rate_limiter,
            wallet_address,
            cached_signatures: Arc::new(Mutex::new(HashSet::new())),
            stats: Arc::new(Mutex::new(CachingStats::default())),
            configs,
            client,
        })
    }

    /// Start continuous background caching
    pub async fn start_continuous_caching(&self) -> Result<(), Box<dyn std::error::Error>> {
        log(LogTag::System, "INFO", "🚀 Starting continuous transaction caching...");

        // Initial cache population
        self.populate_initial_cache().await?;

        // Start background tasks sequentially for this test
        log(LogTag::System, "INFO", "� Starting continuous cache update cycle...");

        // Run cache update indefinitely
        self.start_cache_update_task().await?;

        Ok(())
    }

    /// Populate initial cache with target number of transactions
    async fn populate_initial_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        log(
            LogTag::System,
            "INFO",
            &format!(
                "📊 Populating initial cache with {} transactions...",
                self.config.target_transaction_count
            )
        );

        let mut total_fetched = 0;
        let mut before_signature: Option<String> = None;

        while total_fetched < self.config.target_transaction_count {
            // Rate limiting
            let _permit = self.rate_limiter.acquire().await?;

            let batch_size = (self.config.target_transaction_count - total_fetched).min(
                self.config.batch_size
            );

            log(
                LogTag::System,
                "INFO",
                &format!(
                    "🔍 Fetching batch of {} transactions (total: {}/{})",
                    batch_size,
                    total_fetched,
                    self.config.target_transaction_count
                )
            );

            // Fetch signatures
            let signatures = match before_signature {
                Some(ref before) => {
                    self.fetcher.get_signatures_before(
                        &self.wallet_address,
                        batch_size,
                        before
                    ).await?
                }
                None => {
                    self.fetcher.get_recent_signatures(&self.wallet_address, batch_size).await?
                }
            };

            if signatures.is_empty() {
                log(LogTag::System, "WARNING", "No more signatures available");
                break;
            }

            // Update stats
            {
                let mut stats = self.stats.lock().await;
                stats.rpc_calls += 1;
            }

            // Process each signature
            for sig_info in &signatures {
                // Check if already cached
                {
                    let cached_sigs = self.cached_signatures.lock().await;
                    if cached_sigs.contains(&sig_info.signature) {
                        let mut stats = self.stats.lock().await;
                        stats.cache_hits += 1;
                        continue;
                    }
                }

                // Fetch full transaction with rate limiting
                let _permit = self.rate_limiter.acquire().await?;

                let transactions = get_transactions_with_cache_and_fallback(
                    &self.client,
                    &[sig_info.clone()],
                    &self.configs,
                    Some(1)
                ).await;

                if !transactions.is_empty() {
                    for (_, transaction) in transactions {
                        // Store in database
                        {
                            let db = self.db.lock().await;
                            if let Err(e) = db.store_transaction(&transaction) {
                                log(
                                    LogTag::System,
                                    "ERROR",
                                    &format!("Failed to store transaction: {}", e)
                                );
                                let mut stats = self.stats.lock().await;
                                stats.errors += 1;
                            } else {
                                // Add to cached signatures
                                {
                                    let mut cached_sigs = self.cached_signatures.lock().await;
                                    cached_sigs.insert(sig_info.signature.clone());
                                }

                                // Update stats
                                {
                                    let mut stats = self.stats.lock().await;
                                    stats.total_cached += 1;
                                    stats.cache_misses += 1;

                                    // Update transaction time bounds
                                    if let Some(block_time) = transaction.block_time {
                                        let tx_time = DateTime::from_timestamp(
                                            block_time as i64,
                                            0
                                        ).unwrap_or_else(|| Utc::now());

                                        if
                                            stats.oldest_transaction.is_none() ||
                                            Some(tx_time) < stats.oldest_transaction
                                        {
                                            stats.oldest_transaction = Some(tx_time);
                                        }
                                        if
                                            stats.newest_transaction.is_none() ||
                                            Some(tx_time) > stats.newest_transaction
                                        {
                                            stats.newest_transaction = Some(tx_time);
                                        }
                                    }
                                }
                            }
                        }

                        // Update stats
                        {
                            let mut stats = self.stats.lock().await;
                            stats.rpc_calls += 1;
                        }
                    }
                } else {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("No transaction data found for signature: {}", sig_info.signature)
                    );
                }

                total_fetched += 1;

                // Rate limiting delay
                sleep(
                    Duration::from_millis(1000 / (self.config.rate_limit_per_second as u64))
                ).await;
            }

            // Set before signature for next batch
            before_signature = signatures.last().map(|s| s.signature.clone());

            // Progress update
            if total_fetched % 100 == 0 {
                let stats = self.stats.lock().await;
                log(
                    LogTag::System,
                    "INFO",
                    &format!(
                        "📈 Progress: {}/{} transactions cached (errors: {})",
                        total_fetched,
                        self.config.target_transaction_count,
                        stats.errors
                    )
                );
            }
        }

        let final_stats = self.stats.lock().await;
        log(
            LogTag::System,
            "SUCCESS",
            &format!(
                "✅ Initial cache population complete: {} transactions cached",
                final_stats.total_cached
            )
        );

        Ok(())
    }

    /// Start background task for continuous cache updates
    async fn start_cache_update_task(&self) -> Result<(), Box<dyn std::error::Error>> {
        let fetcher = self.fetcher.clone();
        let db = self.db.clone();
        let rate_limiter = self.rate_limiter.clone();
        let cached_signatures = self.cached_signatures.clone();
        let stats = self.stats.clone();
        let wallet_address = self.wallet_address.clone();
        let update_interval = self.config.cache_update_interval_secs;
        let rate_limit = self.config.rate_limit_per_second;

        loop {
            log(LogTag::System, "INFO", "🔄 Starting cache update cycle...");

            // Rate limiting
            let _permit = rate_limiter.acquire().await?;

            // Fetch latest signatures
            match fetcher.get_recent_signatures(&wallet_address, 50).await {
                Ok(signatures) => {
                    let mut new_transactions = 0;

                    for sig_info in signatures {
                        // Check if already cached
                        {
                            let cached_sigs = cached_signatures.lock().await;
                            if cached_sigs.contains(&sig_info.signature) {
                                continue;
                            }
                        }

                        // Rate limiting for individual requests
                        let _permit = rate_limiter.acquire().await?;

                        // Fetch and cache new transaction
                        let transactions = get_transactions_with_cache_and_fallback(
                            &self.client,
                            &[sig_info.clone()],
                            &self.configs,
                            Some(1)
                        ).await;

                        if !transactions.is_empty() {
                            for (_, transaction) in transactions {
                                let db = db.lock().await;
                                if let Err(e) = db.store_transaction(&transaction) {
                                    log(
                                        LogTag::System,
                                        "ERROR",
                                        &format!("Failed to store new transaction: {}", e)
                                    );
                                } else {
                                    // Add to cached signatures
                                    {
                                        let mut cached_sigs = cached_signatures.lock().await;
                                        cached_sigs.insert(sig_info.signature.clone());
                                    }

                                    new_transactions += 1;

                                    // Update stats
                                    {
                                        let mut stats = stats.lock().await;
                                        stats.total_cached += 1;
                                        stats.last_update = Some(Utc::now());
                                    }
                                }
                            }
                        } else {
                            log(
                                LogTag::System,
                                "WARNING",
                                &format!(
                                    "No transaction data found for new signature: {}",
                                    sig_info.signature
                                )
                            );
                        }

                        // Rate limiting delay
                        sleep(Duration::from_millis(1000 / (rate_limit as u64))).await;
                    }

                    if new_transactions > 0 {
                        log(
                            LogTag::System,
                            "SUCCESS",
                            &format!("📥 Cached {} new transactions", new_transactions)
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to fetch recent signatures: {}", e)
                    );
                    let mut stats = stats.lock().await;
                    stats.errors += 1;
                }
            }

            // Wait before next update
            sleep(Duration::from_secs(update_interval)).await;
        }
    }

    /// Start background task for cleaning up old transactions
    async fn start_cleanup_task(&self) -> Result<(), Box<dyn std::error::Error>> {
        let db = self.db.clone();
        let historical_days = self.config.historical_days;

        loop {
            // Wait 1 hour between cleanup cycles
            sleep(Duration::from_secs(3600)).await;

            log(LogTag::System, "INFO", "🧹 Starting cleanup of old transactions...");

            let cutoff_time = Utc::now() - chrono::Duration::days(historical_days as i64);
            let cutoff_timestamp = cutoff_time.timestamp();

            let db = db.lock().await;
            match db.cleanup_transactions_by_timestamp(cutoff_timestamp) {
                Ok(deleted_count) => {
                    if deleted_count > 0 {
                        log(
                            LogTag::System,
                            "SUCCESS",
                            &format!(
                                "🗑️ Cleaned up {} old transactions (older than {} days)",
                                deleted_count,
                                historical_days
                            )
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to cleanup old transactions: {}", e)
                    );
                }
            }
        }
    }

    /// Start background task for stats reporting
    async fn start_stats_reporting_task(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stats = self.stats.clone();
        let db = self.db.clone();
        let report_interval = self.config.status_report_interval_secs;

        loop {
            sleep(Duration::from_secs(report_interval)).await;

            // Generate comprehensive stats report
            let stats = stats.lock().await;
            let db = db.lock().await;

            match db.get_transaction_count() {
                Ok(total_in_db) => {
                    log(LogTag::System, "INFO", "📊 CACHING STATS REPORT:");
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("   💾 Total in database: {}", total_in_db)
                    );
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("   📈 Total cached: {}", stats.total_cached)
                    );
                    log(LogTag::System, "INFO", &format!("   🎯 Cache hits: {}", stats.cache_hits));
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("   ❌ Cache misses: {}", stats.cache_misses)
                    );
                    log(LogTag::System, "INFO", &format!("   🌐 RPC calls: {}", stats.rpc_calls));
                    log(LogTag::System, "INFO", &format!("   ⚠️ Errors: {}", stats.errors));

                    if let Some(last_update) = stats.last_update {
                        log(
                            LogTag::System,
                            "INFO",
                            &format!(
                                "   🕐 Last update: {}",
                                last_update.format("%Y-%m-%d %H:%M:%S UTC")
                            )
                        );
                    }

                    if
                        let (Some(oldest), Some(newest)) = (
                            stats.oldest_transaction,
                            stats.newest_transaction,
                        )
                    {
                        let timespan = newest - oldest;
                        log(
                            LogTag::System,
                            "INFO",
                            &format!(
                                "   📅 Time range: {} to {} ({} days)",
                                oldest.format("%Y-%m-%d"),
                                newest.format("%Y-%m-%d"),
                                timespan.num_days()
                            )
                        );
                    }

                    // Performance metrics
                    let cache_hit_rate = if stats.cache_hits + stats.cache_misses > 0 {
                        ((stats.cache_hits as f64) /
                            ((stats.cache_hits + stats.cache_misses) as f64)) *
                            100.0
                    } else {
                        0.0
                    };

                    let error_rate = if stats.rpc_calls > 0 {
                        ((stats.errors as f64) / (stats.rpc_calls as f64)) * 100.0
                    } else {
                        0.0
                    };

                    log(
                        LogTag::System,
                        "INFO",
                        &format!("   📊 Cache hit rate: {:.1}%", cache_hit_rate)
                    );
                    log(LogTag::System, "INFO", &format!("   📊 Error rate: {:.1}%", error_rate));
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to get database transaction count: {}", e)
                    );
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "🚀 Starting Continuous Transaction Cache Test...");

    // Create caching configuration
    let config = CachingConfig {
        target_transaction_count: 1000,
        batch_size: 50,
        rate_limit_per_second: 10, // Conservative for mainnet RPC
        max_concurrent_requests: 3,
        historical_days: 7,
        cache_update_interval_secs: 30, // Check for new transactions every 30 seconds
        status_report_interval_secs: 300, // Report stats every 5 minutes
    };

    log(LogTag::System, "INFO", &format!("📋 Configuration:"));
    log(
        LogTag::System,
        "INFO",
        &format!("   🎯 Target transactions: {}", config.target_transaction_count)
    );
    log(LogTag::System, "INFO", &format!("   📦 Batch size: {}", config.batch_size));
    log(
        LogTag::System,
        "INFO",
        &format!("   ⏱️ Rate limit: {} req/sec", config.rate_limit_per_second)
    );
    log(
        LogTag::System,
        "INFO",
        &format!("   🔄 Max concurrent: {}", config.max_concurrent_requests)
    );
    log(
        LogTag::System,
        "INFO",
        &format!("   📅 Historical retention: {} days", config.historical_days)
    );
    log(
        LogTag::System,
        "INFO",
        &format!("   🔄 Update interval: {} seconds", config.cache_update_interval_secs)
    );

    // Create and start continuous caching system
    let cache_system = ContinuousTransactionCache::new(config).await?;

    log(LogTag::System, "SUCCESS", "✅ Continuous Transaction Cache initialized");
    log(LogTag::System, "INFO", "🔄 Starting background caching tasks...");
    log(LogTag::System, "INFO", "⏹️ Press Ctrl+C to stop");

    // Start the continuous caching system
    cache_system.start_continuous_caching().await?;

    Ok(())
}
