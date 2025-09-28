// RPC fetching and batch operations for the transactions module
//
// This module handles all blockchain data retrieval operations including
// batch signature fetching, transaction details retrieval, and RPC optimization.

use std::collections::HashMap;
use std::time::{ Duration, Instant };
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_transaction_status::{ EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding };
use tokio::time::sleep;

use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::transactions::{ types::*, utils::* };

// =============================================================================
// FETCHER CONFIGURATION
// =============================================================================

/// Configuration for RPC fetching operations
#[derive(Debug, Clone)]
pub struct FetcherConfig {
    /// Maximum number of signatures to fetch per batch
    pub max_signatures_per_batch: usize,
    /// Maximum number of transaction details to fetch concurrently
    pub max_concurrent_details: usize,
    /// Delay between batch operations to avoid rate limiting
    pub batch_delay_ms: u64,
    /// Maximum retries for failed RPC calls
    pub max_retries: usize,
    /// Base delay for exponential backoff (milliseconds)
    pub retry_base_delay_ms: u64,
    /// Commitment level for transaction queries
    pub commitment: CommitmentConfig,
}

impl Default for FetcherConfig {
    fn default() -> Self {
        Self {
            max_signatures_per_batch: RPC_BATCH_SIZE,
            max_concurrent_details: PROCESS_BATCH_SIZE,
            batch_delay_ms: 100, // 100ms delay between batches
            max_retries: 3,
            retry_base_delay_ms: 1000, // 1 second base delay
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

// =============================================================================
// TRANSACTION FETCHER
// =============================================================================

/// High-performance transaction fetcher with batching and rate limiting
pub struct TransactionFetcher {
    config: FetcherConfig,
    metrics: FetcherMetrics,
}

impl TransactionFetcher {
    /// Create new transaction fetcher with default configuration
    pub fn new() -> Self {
        Self {
            config: FetcherConfig::default(),
            metrics: FetcherMetrics::new(),
        }
    }

    /// Create new transaction fetcher with custom configuration
    pub fn with_config(config: FetcherConfig) -> Self {
        Self {
            config,
            metrics: FetcherMetrics::new(),
        }
    }

    /// Get current fetcher metrics
    pub fn get_metrics(&self) -> &FetcherMetrics {
        &self.metrics
    }
}

// =============================================================================
// SIGNATURE FETCHING
// =============================================================================

impl TransactionFetcher {
    /// Fetch recent signatures for wallet with optimized batching
    pub async fn fetch_recent_signatures(
        &self,
        wallet_pubkey: Pubkey,
        limit: usize
    ) -> Result<Vec<String>, String> {
        let start_time = Instant::now();

        log(
            LogTag::Transactions,
            "FETCH",
            &format!(
                "Fetching {} recent signatures for wallet: {}",
                limit,
                format_address_full(&wallet_pubkey.to_string())
            )
        );

        let rpc_client = get_rpc_client();

        // Use optimized signature fetching with rate limiting
        let signatures = self.fetch_signatures_with_retry(
            &rpc_client,
            wallet_pubkey,
            limit,
            None
        ).await?;

        let duration = start_time.elapsed();

        log(
            LogTag::Transactions,
            "FETCH",
            &format!(
                "Fetched {} signatures in {}ms for wallet: {}",
                signatures.len(),
                duration.as_millis(),
                format_pubkey_short(&wallet_pubkey.to_string())
            )
        );

        Ok(signatures)
    }

    /// Fetch signatures with automatic retry and exponential backoff
    async fn fetch_signatures_with_retry(
        &self,
        rpc_client: &crate::rpc::RpcClient,
        wallet_pubkey: Pubkey,
        limit: usize,
        before: Option<Signature>
    ) -> Result<Vec<String>, String> {
        let mut attempts = 0;
        let mut delay = self.config.retry_base_delay_ms;

        loop {
            match self.fetch_signatures_batch(rpc_client, wallet_pubkey, limit, before).await {
                Ok(signatures) => {
                    if attempts > 0 {
                        log(
                            LogTag::Transactions,
                            "INFO",
                            &format!(
                                "Signature fetch succeeded after {} retries for wallet: {}",
                                attempts,
                                format_pubkey_short(&wallet_pubkey.to_string())
                            )
                        );
                    }
                    return Ok(signatures);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.config.max_retries {
                        return Err(
                            format!("Failed to fetch signatures after {} attempts: {}", attempts, e)
                        );
                    }

                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!(
                            "Signature fetch attempt {} failed, retrying in {}ms: {}",
                            attempts,
                            delay,
                            e
                        )
                    );

                    sleep(Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Fetch a batch of signatures from RPC
    async fn fetch_signatures_batch(
        &self,
        rpc_client: &crate::rpc::RpcClient,
        wallet_pubkey: Pubkey,
        limit: usize,
        before: Option<Signature>
    ) -> Result<Vec<String>, String> {
        // Use the existing RPC client method
        let sig_infos = rpc_client
            .get_wallet_signatures_main_rpc(&wallet_pubkey, limit, before).await
            .map_err(|e| format!("RPC signature fetch failed: {}", e))?;

        let signatures: Vec<String> = sig_infos
            .into_iter()
            .map(|info| info.signature)
            .collect();

        Ok(signatures)
    }
}

// =============================================================================
// TRANSACTION DETAILS FETCHING
// =============================================================================

impl TransactionFetcher {
    /// Fetch transaction details with retry logic
    pub async fn fetch_transaction_details(
        &self,
        signature: &str
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
        let start_time = Instant::now();

        let details = self.fetch_transaction_details_with_retry(signature).await?;

        let duration = start_time.elapsed();

        if duration.as_millis() > 5000 {
            // Log slow fetches
            log(
                LogTag::Transactions,
                "SLOW",
                &format!(
                    "Slow transaction fetch: {} took {}ms",
                    format_signature_short(signature),
                    duration.as_millis()
                )
            );
        }

        Ok(details)
    }

    /// Fetch transaction details with automatic retry
    async fn fetch_transaction_details_with_retry(
        &self,
        signature: &str
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
        let mut attempts = 0;
        let mut delay = self.config.retry_base_delay_ms;

        loop {
            match self.fetch_single_transaction_details(signature).await {
                Ok(details) => {
                    if attempts > 0 {
                        log(
                            LogTag::Transactions,
                            "INFO",
                            &format!(
                                "Transaction details fetch succeeded after {} retries: {}",
                                attempts,
                                format_signature_short(signature)
                            )
                        );
                    }
                    return Ok(details);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.config.max_retries {
                        return Err(
                            format!(
                                "Failed to fetch transaction details after {} attempts: {}",
                                attempts,
                                e
                            )
                        );
                    }

                    // Don't retry for "not found" errors
                    if e.contains("not found") || e.contains("no longer available") {
                        return Err(e);
                    }

                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!(
                            "Transaction details fetch attempt {} failed, retrying in {}ms: {}",
                            attempts,
                            delay,
                            e
                        )
                    );

                    sleep(Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }

    /// Fetch single transaction details from RPC
    async fn fetch_single_transaction_details(
        &self,
        signature: &str
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
        let rpc_client = get_rpc_client();

        // Use existing RPC client method
        rpc_client
            .get_transaction_details(signature).await
            .map_err(|e| format!("Failed to fetch transaction details: {}", e))
    }

    /// Fetch multiple transaction details concurrently with rate limiting
    pub async fn fetch_multiple_transaction_details(
        &self,
        signatures: Vec<String>
    ) -> HashMap<String, Result<EncodedConfirmedTransactionWithStatusMeta, String>> {
        let start_time = Instant::now();
        let total_count = signatures.len();

        log(
            LogTag::Transactions,
            "BATCH_FETCH",
            &format!("Fetching details for {} transactions", total_count)
        );

        let mut results = HashMap::new();

        // Process in chunks to respect concurrency limits
        let chunks = chunk_signatures(signatures, self.config.max_concurrent_details);

        for (chunk_idx, chunk) in chunks.into_iter().enumerate() {
            let chunk_start = Instant::now();

            // Process chunk concurrently
            let chunk_tasks: Vec<_> = chunk
                .into_iter()
                .map(|signature| {
                    let sig_clone = signature.clone();
                    async move {
                        let result = self.fetch_transaction_details(&sig_clone).await;
                        (sig_clone, result)
                    }
                })
                .collect();

            let chunk_results = futures::future::join_all(chunk_tasks).await;

            // Collect results
            for (signature, result) in chunk_results {
                results.insert(signature, result);
            }

            let chunk_duration = chunk_start.elapsed();

            log(
                LogTag::Transactions,
                "BATCH_CHUNK",
                &format!(
                    "Processed chunk {}/{} ({} transactions) in {}ms",
                    chunk_idx + 1,
                    chunks.len(),
                    results.len(),
                    chunk_duration.as_millis()
                )
            );

            // Add delay between chunks to avoid rate limiting
            if chunk_idx < chunks.len() - 1 {
                sleep(Duration::from_millis(self.config.batch_delay_ms)).await;
            }
        }

        let total_duration = start_time.elapsed();
        let success_count = results
            .values()
            .filter(|r| r.is_ok())
            .count();

        log(
            LogTag::Transactions,
            "BATCH_COMPLETE",
            &format!(
                "Batch fetch complete: {}/{} successful in {}ms (avg: {}ms/tx)",
                success_count,
                total_count,
                total_duration.as_millis(),
                if total_count > 0 {
                    total_duration.as_millis() / (total_count as u128)
                } else {
                    0
                }
            )
        );

        results
    }
}

// =============================================================================
// FETCHER METRICS
// =============================================================================

/// Performance metrics for the transaction fetcher
#[derive(Debug, Clone)]
pub struct FetcherMetrics {
    /// Total number of signature fetch operations
    pub signature_fetches: u64,
    /// Total number of transaction detail fetches
    pub detail_fetches: u64,
    /// Total number of failed operations
    pub failed_operations: u64,
    /// Total number of retry attempts
    pub retry_attempts: u64,
    /// Average response time for signature fetches (milliseconds)
    pub avg_signature_fetch_ms: f64,
    /// Average response time for detail fetches (milliseconds)
    pub avg_detail_fetch_ms: f64,
    /// Last operation timestamp
    pub last_operation: Option<chrono::DateTime<chrono::Utc>>,
}

impl FetcherMetrics {
    fn new() -> Self {
        Self {
            signature_fetches: 0,
            detail_fetches: 0,
            failed_operations: 0,
            retry_attempts: 0,
            avg_signature_fetch_ms: 0.0,
            avg_detail_fetch_ms: 0.0,
            last_operation: None,
        }
    }

    /// Update signature fetch metrics
    pub fn update_signature_fetch(&mut self, duration: Duration, success: bool) {
        self.signature_fetches += 1;
        self.last_operation = Some(chrono::Utc::now());

        if success {
            let duration_ms = duration.as_millis() as f64;
            self.avg_signature_fetch_ms = if self.signature_fetches == 1 {
                duration_ms
            } else {
                (self.avg_signature_fetch_ms * ((self.signature_fetches - 1) as f64) +
                    duration_ms) /
                    (self.signature_fetches as f64)
            };
        } else {
            self.failed_operations += 1;
        }
    }

    /// Update detail fetch metrics
    pub fn update_detail_fetch(&mut self, duration: Duration, success: bool) {
        self.detail_fetches += 1;
        self.last_operation = Some(chrono::Utc::now());

        if success {
            let duration_ms = duration.as_millis() as f64;
            self.avg_detail_fetch_ms = if self.detail_fetches == 1 {
                duration_ms
            } else {
                (self.avg_detail_fetch_ms * ((self.detail_fetches - 1) as f64) + duration_ms) /
                    (self.detail_fetches as f64)
            };
        } else {
            self.failed_operations += 1;
        }
    }

    /// Update retry metrics
    pub fn update_retry(&mut self) {
        self.retry_attempts += 1;
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total_operations = self.signature_fetches + self.detail_fetches;
        if total_operations == 0 {
            100.0
        } else {
            (((total_operations - self.failed_operations) as f64) / (total_operations as f64)) *
                100.0
        }
    }

    /// Get operations per second (based on recent activity)
    pub fn operations_per_second(&self) -> f64 {
        // This would calculate based on a sliding window
        // For now, return 0 as placeholder
        0.0
    }
}

// =============================================================================
// BATCH OPTIMIZATION
// =============================================================================

/// Batch signature fetching with automatic pagination
pub struct BatchSignatureFetcher {
    fetcher: TransactionFetcher,
    batch_size: usize,
}

impl BatchSignatureFetcher {
    pub fn new(batch_size: usize) -> Self {
        Self {
            fetcher: TransactionFetcher::new(),
            batch_size,
        }
    }

    /// Fetch all signatures for a wallet with automatic pagination
    pub async fn fetch_all_signatures(
        &self,
        wallet_pubkey: Pubkey,
        max_signatures: Option<usize>
    ) -> Result<Vec<String>, String> {
        let mut all_signatures = Vec::new();
        let mut before: Option<Signature> = None;
        let limit = max_signatures.unwrap_or(usize::MAX);

        loop {
            let batch_limit = std::cmp::min(self.batch_size, limit - all_signatures.len());
            if batch_limit == 0 {
                break;
            }

            let batch_signatures = self.fetcher.fetch_signatures_with_retry(
                &get_rpc_client(),
                wallet_pubkey,
                batch_limit,
                before
            ).await?;

            if batch_signatures.is_empty() {
                break; // No more signatures
            }

            // Set 'before' for next batch
            if let Some(last_sig) = batch_signatures.last() {
                before = Some(
                    Signature::from_str(last_sig).map_err(|e|
                        format!("Invalid signature format: {}", e)
                    )?
                );

                all_signatures.extend(batch_signatures);

                // Add delay between batches
                tokio::time::sleep(Duration::from_millis(self.fetcher.config.batch_delay_ms)).await;
            } else {
                break;
            }

            // Check if we've reached the limit
            if all_signatures.len() >= limit {
                all_signatures.truncate(limit);
                break;
            }
        }

        log(
            LogTag::Transactions,
            "BATCH_COMPLETE",
            &format!(
                "Fetched {} total signatures for wallet: {}",
                all_signatures.len(),
                format_pubkey_short(&wallet_pubkey.to_string())
            )
        );

        Ok(all_signatures)
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Create optimized fetcher configuration for high-volume operations
pub fn create_high_volume_config() -> FetcherConfig {
    FetcherConfig {
        max_signatures_per_batch: 1000,
        max_concurrent_details: 20,
        batch_delay_ms: 50, // Reduced delay for higher throughput
        max_retries: 5,
        retry_base_delay_ms: 500,
        commitment: CommitmentConfig::confirmed(),
    }
}

/// Create conservative fetcher configuration for stable operations
pub fn create_conservative_config() -> FetcherConfig {
    FetcherConfig {
        max_signatures_per_batch: 100,
        max_concurrent_details: 5,
        batch_delay_ms: 200, // Increased delay for stability
        max_retries: 3,
        retry_base_delay_ms: 1000,
        commitment: CommitmentConfig::finalized(),
    }
}

/// Validate signature format before fetching
pub fn validate_signature_format(signature: &str) -> Result<(), String> {
    if !is_valid_signature(signature) {
        return Err(format!("Invalid signature format: {}", signature));
    }
    Ok(())
}

/// Calculate optimal batch size based on available memory and network conditions
pub fn calculate_optimal_batch_size(available_memory_mb: usize, network_latency_ms: u64) -> usize {
    // Basic heuristic for batch size calculation
    let base_size = 50;
    let memory_factor = (available_memory_mb / 100).max(1).min(10);
    let latency_factor = if network_latency_ms < 100 { 2 } else { 1 };

    base_size * memory_factor * latency_factor
}
