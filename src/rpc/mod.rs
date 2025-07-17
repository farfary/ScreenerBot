pub mod types;

use crate::config::RpcConfig;
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{ RpcSendTransactionConfig, RpcSimulateTransactionConfig };
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::{ RpcKeyedAccount, RpcSimulateTransactionResult };
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::VersionedTransaction,
};
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;
use types::{ RpcEndpoint, RpcError, RpcResult, RpcStats, TransactionConfig };

#[derive(Debug)]
pub struct RpcManager {
    endpoints: Arc<RwLock<Vec<RpcEndpoint>>>,
    config: RpcConfig,
    stats: Arc<RwLock<RpcStats>>,
    #[allow(dead_code)]
    current_index: Arc<std::sync::atomic::AtomicUsize>,
}

impl RpcManager {
    pub fn new(primary_url: String, fallback_urls: Vec<String>, config: RpcConfig) -> Result<Self> {
        let mut endpoints = Vec::new();

        // Add primary endpoint with highest weight
        endpoints.push(RpcEndpoint::new(primary_url, 100));

        // Add fallback endpoints with lower weights
        for url in fallback_urls {
            endpoints.push(RpcEndpoint::new(url, 50));
        }

        if endpoints.is_empty() {
            return Err(anyhow::anyhow!("No RPC endpoints provided"));
        }

        log::info!("🔗 RPC Manager initialized with {} endpoints", endpoints.len());
        for endpoint in &endpoints {
            log::info!("   - {} (weight: {})", endpoint.url, endpoint.weight);
        }

        Ok(Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            config,
            stats: Arc::new(RwLock::new(RpcStats::new())),
            current_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    pub async fn get_healthy_client(&self) -> RpcResult<(RpcClient, usize)> {
        let endpoints = self.endpoints.read().await;
        let mut best_endpoint_idx = 0;
        let mut best_score = 0.0;

        // Find the best healthy endpoint based on success rate and response time
        for (idx, endpoint) in endpoints.iter().enumerate() {
            if !endpoint.healthy {
                continue;
            }

            let score = Self::calculate_endpoint_score(endpoint);
            if score > best_score {
                best_score = score;
                best_endpoint_idx = idx;
            }
        }

        if best_score == 0.0 {
            return Err(RpcError::AllEndpointsFailed);
        }

        let endpoint = &endpoints[best_endpoint_idx];
        let client = RpcClient::new_with_timeout_and_commitment(
            endpoint.url.clone(),
            Duration::from_secs(self.config.timeout_seconds),
            Self::commitment_from_string(&self.config.commitment)
        );

        Ok((client, best_endpoint_idx))
    }

    fn calculate_endpoint_score(endpoint: &RpcEndpoint) -> f64 {
        if !endpoint.healthy {
            return 0.0;
        }

        let success_rate = endpoint.success_rate();
        let response_time_factor = if endpoint.response_time_ms > 0 {
            1000.0 / (endpoint.response_time_ms as f64)
        } else {
            1.0
        };

        let weight_factor = (endpoint.weight as f64) / 100.0;

        success_rate * response_time_factor * weight_factor
    }

    pub async fn execute_with_fallback<T, F>(&self, method_name: &str, operation: F) -> RpcResult<T>
        where F: Fn(&RpcClient) -> Result<T> + Send + Sync, T: Send + 'static
    {
        // First, try the primary endpoint (mainnet) 3 times with 10-second timeout
        let primary_attempts = 3;
        let primary_timeout_secs = 10;

        for attempt in 1..=primary_attempts {
            if let Ok(primary_client) = self.get_primary_client(primary_timeout_secs).await {
                let start_time = Instant::now();

                match operation(&primary_client.0) {
                    Ok(result) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        self.record_success(primary_client.1, response_time, method_name).await;
                        log::debug!(
                            "✅ Primary endpoint succeeded on attempt {}/{}",
                            attempt,
                            primary_attempts
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        self.record_error(primary_client.1, response_time, method_name).await;
                        log::warn!(
                            "❌ Primary endpoint failed (attempt {}/{}): {}",
                            attempt,
                            primary_attempts,
                            e
                        );

                        if attempt < primary_attempts {
                            tokio::time::sleep(
                                Duration::from_millis(self.config.retry_delay_ms)
                            ).await;
                        }
                    }
                }
            } else {
                log::warn!(
                    "❌ Primary endpoint unhealthy on attempt {}/{}",
                    attempt,
                    primary_attempts
                );
            }
        }

        log::warn!(
            "⚠️ Primary endpoint failed all {} attempts, trying fallback endpoints...",
            primary_attempts
        );

        // If primary endpoint failed all attempts, try fallback endpoints
        let fallback_attempts = self.config.max_retries as usize;

        for attempt in 1..=fallback_attempts {
            if let Ok(fallback_client) = self.get_fallback_client().await {
                let start_time = Instant::now();

                match operation(&fallback_client.0) {
                    Ok(result) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        self.record_success(fallback_client.1, response_time, method_name).await;
                        log::info!(
                            "✅ Fallback endpoint succeeded on attempt {}/{}",
                            attempt,
                            fallback_attempts
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        let response_time = start_time.elapsed().as_millis() as u64;
                        self.record_error(fallback_client.1, response_time, method_name).await;
                        log::warn!(
                            "❌ Fallback endpoint failed (attempt {}/{}): {}",
                            attempt,
                            fallback_attempts,
                            e
                        );

                        if attempt < fallback_attempts {
                            tokio::time::sleep(
                                Duration::from_millis(self.config.retry_delay_ms)
                            ).await;
                        }
                    }
                }
            } else {
                log::warn!(
                    "❌ No healthy fallback endpoints available on attempt {}/{}",
                    attempt,
                    fallback_attempts
                );
            }
        }

        log::error!(
            "💥 All endpoints failed after {} primary + {} fallback attempts",
            primary_attempts,
            fallback_attempts
        );
        Err(RpcError::AllEndpointsFailed)
    }

    async fn record_success(&self, endpoint_idx: usize, response_time_ms: u64, method_name: &str) {
        let endpoint_url = {
            let endpoints = self.endpoints.read().await;
            if let Some(endpoint) = endpoints.get(endpoint_idx) {
                endpoint.url.clone()
            } else {
                "unknown".to_string()
            }
        };

        let mut endpoints = self.endpoints.write().await;
        if let Some(endpoint) = endpoints.get_mut(endpoint_idx) {
            endpoint.record_success(response_time_ms);
        }

        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.successful_requests += 1;
        stats.current_endpoint_index = endpoint_idx;
        stats.record_method_call(method_name, response_time_ms, true);
        stats.record_endpoint_call(&endpoint_url, response_time_ms, true);

        // Update average response time
        let total_successful = stats.successful_requests;
        stats.average_response_time_ms =
            (stats.average_response_time_ms * (total_successful - 1) + response_time_ms) /
            total_successful;
    }

    async fn record_error(&self, endpoint_idx: usize, response_time_ms: u64, method_name: &str) {
        let endpoint_url = {
            let endpoints = self.endpoints.read().await;
            if let Some(endpoint) = endpoints.get(endpoint_idx) {
                endpoint.url.clone()
            } else {
                "unknown".to_string()
            }
        };

        let mut endpoints = self.endpoints.write().await;
        if let Some(endpoint) = endpoints.get_mut(endpoint_idx) {
            endpoint.record_error();
        }

        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.failed_requests += 1;
        stats.record_method_call(method_name, response_time_ms, false);
        stats.record_endpoint_call(&endpoint_url, response_time_ms, false);
    }

    // RPC Methods

    pub async fn get_balance(&self, pubkey: &Pubkey) -> RpcResult<u64> {
        self.execute_with_fallback("get_balance", |client| {
            client.get_balance(pubkey).map_err(|e| anyhow::anyhow!("Failed to get balance: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_account(&self, pubkey: &Pubkey) -> RpcResult<solana_sdk::account::Account> {
        self.execute_with_fallback("get_account", |client| {
            client.get_account(pubkey).map_err(|e| anyhow::anyhow!("Failed to get account: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        filter: TokenAccountsFilter
    ) -> RpcResult<Vec<RpcKeyedAccount>> {
        let owner = *owner;

        self.execute_with_fallback("get_token_accounts_by_owner", move |client| {
            let filter_copy = match filter {
                TokenAccountsFilter::Mint(mint) => TokenAccountsFilter::Mint(mint),
                TokenAccountsFilter::ProgramId(program_id) =>
                    TokenAccountsFilter::ProgramId(program_id),
            };
            client
                .get_token_accounts_by_owner(&owner, filter_copy)
                .map_err(|e| anyhow::anyhow!("Failed to get token accounts: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn send_transaction(
        &self,
        transaction: &VersionedTransaction,
        config: Option<TransactionConfig>
    ) -> RpcResult<Signature> {
        let tx_config = config.unwrap_or_default();

        self.execute_with_fallback("send_transaction", |client| {
            let rpc_config = RpcSendTransactionConfig {
                skip_preflight: tx_config.skip_preflight,
                preflight_commitment: Some(tx_config.commitment.commitment),
                encoding: None,
                max_retries: tx_config.max_retries,
                min_context_slot: None,
            };

            client
                .send_transaction_with_config(transaction, rpc_config)
                .map_err(|e| anyhow::anyhow!("Failed to send transaction: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn simulate_transaction(
        &self,
        transaction: &VersionedTransaction
    ) -> RpcResult<RpcSimulateTransactionResult> {
        self.execute_with_fallback("simulate_transaction", |client| {
            let config = RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: true,
                commitment: Some(CommitmentConfig::confirmed()),
                encoding: None,
                accounts: None,
                min_context_slot: None,
                inner_instructions: false,
            };

            client
                .simulate_transaction_with_config(transaction, config)
                .map(|response| response.value)
                .map_err(|e| anyhow::anyhow!("Failed to simulate transaction: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_latest_blockhash(&self) -> RpcResult<solana_sdk::hash::Hash> {
        self.execute_with_fallback("get_latest_blockhash", |client| {
            client
                .get_latest_blockhash()
                .map_err(|e| anyhow::anyhow!("Failed to get latest blockhash: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn confirm_transaction(&self, signature: &Signature) -> RpcResult<bool> {
        self.execute_with_fallback("confirm_transaction", |client| {
            client
                .confirm_transaction(signature)
                .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_transaction_status(
        &self,
        signature: &Signature
    ) -> RpcResult<Option<Result<(), solana_sdk::transaction::TransactionError>>> {
        self.execute_with_fallback("get_transaction_status", |client| {
            client
                .get_signature_status(signature)
                .map_err(|e| anyhow::anyhow!("Failed to get transaction status: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        _limit: Option<usize>
    ) -> RpcResult<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>> {
        let address = *address;
        self.execute_with_fallback("get_signatures_for_address", move |client| {
            client
                .get_signatures_for_address(&address)
                .map_err(|e| anyhow::anyhow!("Failed to get signatures for address: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_transaction(
        &self,
        signature: &str
    ) -> RpcResult<solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta> {
        use solana_client::rpc_config::RpcTransactionConfig;
        use solana_transaction_status::UiTransactionEncoding;

        let signature = signature
            .parse::<Signature>()
            .map_err(|e| types::RpcError::RequestFailed(format!("Invalid signature: {}", e)))?;

        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        self.execute_with_fallback("get_transaction", move |client| {
            client
                .get_transaction_with_config(&signature, config)
                .map_err(|e| anyhow::anyhow!("Failed to get transaction: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    // Enhanced transaction signature fetching methods for transaction caching

    pub async fn get_signatures_for_address_with_config(
        &self,
        address: &Pubkey,
        _limit: Option<usize>,
        _before: Option<&str>
    ) -> RpcResult<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>> {
        let address = *address;
        self.execute_with_fallback("get_signatures_for_address_with_config", move |client| {
            client
                .get_signatures_for_address(&address)
                .map_err(|e|
                    anyhow::anyhow!("Failed to get signatures for address with config: {}", e)
                )
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_signatures_for_address_until(
        &self,
        address: &Pubkey,
        _limit: usize,
        _before: Option<&str>,
        _until: Option<&str>
    ) -> RpcResult<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>> {
        let address = *address;
        self.execute_with_fallback("get_signatures_for_address_until", move |client| {
            client
                .get_signatures_for_address(&address)
                .map_err(|e| anyhow::anyhow!("Failed to get signatures for address until: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    // Health check and maintenance

    pub async fn health_check(&self) -> Result<()> {
        let mut endpoints = self.endpoints.write().await;

        for endpoint in endpoints.iter_mut() {
            let elapsed = endpoint.last_health_check.elapsed();
            if elapsed.as_secs() < self.config.health_check_interval_seconds {
                continue;
            }

            let client = RpcClient::new_with_timeout(endpoint.url.clone(), Duration::from_secs(5));

            let start_time = Instant::now();
            match client.get_health() {
                Ok(_) => {
                    let response_time = start_time.elapsed().as_millis() as u64;
                    endpoint.record_success(response_time);
                    log::debug!("Health check passed for {}", endpoint.url);
                }
                Err(e) => {
                    endpoint.record_error();
                    log::warn!("Health check failed for {}: {}", endpoint.url, e);
                }
            }

            endpoint.update_health_check();
        }

        Ok(())
    }

    pub async fn get_stats(&self) -> RpcStats {
        self.stats.read().await.clone()
    }

    pub async fn get_endpoint_stats(&self) -> Vec<RpcEndpoint> {
        self.endpoints.read().await.clone()
    }

    // Utility methods

    fn commitment_from_string(commitment: &str) -> CommitmentConfig {
        match commitment.to_lowercase().as_str() {
            "processed" => CommitmentConfig::processed(),
            "confirmed" => CommitmentConfig::confirmed(),
            "finalized" => CommitmentConfig::finalized(),
            _ => CommitmentConfig::confirmed(),
        }
    }

    pub async fn add_endpoint(&self, url: String, weight: u32) -> Result<()> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.push(RpcEndpoint::new(url.clone(), weight));
        log::info!("Added new RPC endpoint: {} (weight: {})", url, weight);
        Ok(())
    }

    pub async fn remove_endpoint(&self, url: &str) -> Result<bool> {
        let mut endpoints = self.endpoints.write().await;
        let initial_len = endpoints.len();
        endpoints.retain(|endpoint| endpoint.url != url);

        let removed = endpoints.len() < initial_len;
        if removed {
            log::info!("Removed RPC endpoint: {}", url);
        }

        Ok(removed)
    }

    pub async fn set_endpoint_weight(&self, url: &str, weight: u32) -> Result<bool> {
        let mut endpoints = self.endpoints.write().await;
        for endpoint in endpoints.iter_mut() {
            if endpoint.url == url {
                endpoint.weight = weight;
                log::info!("Updated weight for {} to {}", url, weight);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get a direct RpcClient for specialized use cases
    pub fn get_rpc_client(&self) -> Result<RpcClient> {
        // Use the primary endpoint for direct client access
        let endpoints = futures::executor::block_on(self.endpoints.read());
        if endpoints.is_empty() {
            return Err(anyhow::anyhow!("No RPC endpoints available"));
        }

        // Get the first healthy endpoint
        let endpoint = endpoints
            .iter()
            .find(|e| e.healthy)
            .unwrap_or(&endpoints[0]);

        let client = RpcClient::new_with_timeout_and_commitment(
            endpoint.url.clone(),
            Duration::from_secs(self.config.timeout_seconds),
            Self::commitment_from_string(&self.config.commitment)
        );

        Ok(client)
    }

    /// Display RPC usage statistics in a table format
    pub async fn display_usage_table(&self) {
        let stats = self.stats.read().await;
        let method_stats = stats.get_method_stats();
        let endpoint_stats = stats.get_endpoint_stats();

        if method_stats.is_empty() {
            log::info!("📊 RPC Usage Stats: No calls made yet");
            return;
        }

        // Display Method Statistics
        println!(
            "\n╭──────────────────────────────────────────────────────────────────────────────────────╮"
        );
        println!(
            "│                               🔗 RPC METHOD STATISTICS                               │"
        );
        println!(
            "├──────────────────────────────────────────────────────────────────────────────────────┤"
        );
        println!(
            "│ Method                      │ Calls │ Success │ Errors │ Avg Time │ Success Rate │"
        );
        println!(
            "├──────────────────────────────────────────────────────────────────────────────────────┤"
        );

        // Sort methods by call count (descending)
        let mut sorted_methods: Vec<_> = method_stats.iter().collect();
        sorted_methods.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));

        // Display each method's stats
        for (method_name, method_stat) in sorted_methods {
            println!(
                "│ {:<27} │ {:>5} │ {:>7} │ {:>6} │ {:>8.1}ms │ {:>11.1}% │",
                if method_name.len() > 27 {
                    &method_name[..27]
                } else {
                    method_name
                },
                method_stat.call_count,
                method_stat.success_count,
                method_stat.error_count,
                method_stat.average_response_time(),
                method_stat.success_rate()
            );
        }

        println!(
            "├──────────────────────────────────────────────────────────────────────────────────────┤"
        );
        println!(
            "│ TOTAL                       │ {:>5} │ {:>7} │ {:>6} │ {:>8.1}ms │ {:>11.1}% │",
            stats.total_requests,
            stats.successful_requests,
            stats.failed_requests,
            stats.average_response_time_ms,
            stats.success_rate() * 100.0
        );
        println!(
            "╰──────────────────────────────────────────────────────────────────────────────────────╯"
        );

        // Display Endpoint Usage Statistics
        if !endpoint_stats.is_empty() {
            println!(
                "\n╭──────────────────────────────────────────────────────────────────────────────────────╮"
            );
            println!(
                "│                               🌐 RPC ENDPOINT USAGE                                  │"
            );
            println!(
                "├──────────────────────────────────────────────────────────────────────────────────────┤"
            );
            println!(
                "│ Endpoint                                  │ Calls │ Success │ Errors │ Success Rate │"
            );
            println!(
                "├──────────────────────────────────────────────────────────────────────────────────────┤"
            );

            // Sort endpoints by call count (descending)
            let mut sorted_endpoints: Vec<_> = endpoint_stats.iter().collect();
            sorted_endpoints.sort_by(|a, b| b.1.call_count.cmp(&a.1.call_count));

            // Display each endpoint's stats
            for (endpoint_url, endpoint_stat) in sorted_endpoints {
                // Truncate or format the URL for display
                let display_url = if endpoint_url.len() > 41 {
                    format!("{}...", &endpoint_url[..38])
                } else {
                    endpoint_url.clone()
                };

                println!(
                    "│ {:<41} │ {:>5} │ {:>7} │ {:>6} │ {:>11.1}% │",
                    display_url,
                    endpoint_stat.call_count,
                    endpoint_stat.success_count,
                    endpoint_stat.error_count,
                    endpoint_stat.success_rate()
                );
            }

            println!(
                "╰──────────────────────────────────────────────────────────────────────────────────────╯\n"
            );
        }
    }

    /// Start the usage statistics monitor that displays stats every 30 seconds
    pub fn start_usage_monitor(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;
                self.display_usage_table().await;
            }
        })
    }
}

/// Background task for health checks
impl RpcManager {
    pub fn start_health_monitor(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(self.config.health_check_interval_seconds)
            );

            loop {
                interval.tick().await;
                if let Err(e) = self.health_check().await {
                    log::error!("Health check failed: {}", e);
                }
            }
        })
    }
    /// Get the primary endpoint client with specified timeout
    async fn get_primary_client(&self, timeout_secs: u64) -> RpcResult<(RpcClient, usize)> {
        let endpoints = self.endpoints.read().await;

        // Primary endpoint is always the first one (index 0) with highest weight
        if let Some(primary_endpoint) = endpoints.get(0) {
            if primary_endpoint.healthy {
                let client = RpcClient::new_with_timeout_and_commitment(
                    primary_endpoint.url.clone(),
                    Duration::from_secs(timeout_secs),
                    Self::commitment_from_string(&self.config.commitment)
                );
                return Ok((client, 0));
            }
        }

        Err(RpcError::AllEndpointsFailed)
    }

    /// Get a fallback endpoint client (skipping the primary endpoint)
    async fn get_fallback_client(&self) -> RpcResult<(RpcClient, usize)> {
        let endpoints = self.endpoints.read().await;
        let mut best_endpoint_idx = None;
        let mut best_score = 0.0;

        // Skip the first endpoint (primary) and find the best fallback
        for (idx, endpoint) in endpoints.iter().enumerate().skip(1) {
            if !endpoint.healthy {
                continue;
            }

            let score = Self::calculate_endpoint_score(endpoint);
            if score > best_score {
                best_score = score;
                best_endpoint_idx = Some(idx);
            }
        }

        if let Some(endpoint_idx) = best_endpoint_idx {
            let endpoint = &endpoints[endpoint_idx];
            let client = RpcClient::new_with_timeout_and_commitment(
                endpoint.url.clone(),
                Duration::from_secs(self.config.timeout_seconds),
                Self::commitment_from_string(&self.config.commitment)
            );
            return Ok((client, endpoint_idx));
        }

        Err(RpcError::AllEndpointsFailed)
    }
}
