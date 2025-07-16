pub mod types;

use crate::config::RpcConfig;
use anyhow::{ Context, Result };
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{ RpcSendTransactionConfig, RpcSimulateTransactionConfig };
use solana_client::rpc_request::TokenAccountsFilter;
use solana_client::rpc_response::{ RpcKeyedAccount, RpcSimulateTransactionResult };
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::{ Transaction, VersionedTransaction },
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;
use types::{ RpcEndpoint, RpcError, RpcResult, RpcStats, TransactionConfig };

#[derive(Debug)]
pub struct RpcManager {
    endpoints: Arc<RwLock<Vec<RpcEndpoint>>>,
    config: RpcConfig,
    stats: Arc<RwLock<RpcStats>>,
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

    pub async fn execute_with_fallback<T, F>(&self, operation: F) -> RpcResult<T>
        where F: Fn(&RpcClient) -> Result<T> + Send + Sync, T: Send + 'static
    {
        let mut attempts = 0;
        let max_attempts = self.config.max_retries as usize;

        while attempts < max_attempts {
            let (client, endpoint_idx) = self.get_healthy_client().await?;
            let start_time = Instant::now();

            match operation(&client) {
                Ok(result) => {
                    let response_time = start_time.elapsed().as_millis() as u64;
                    self.record_success(endpoint_idx, response_time).await;
                    return Ok(result);
                }
                Err(e) => {
                    self.record_error(endpoint_idx).await;
                    log::warn!(
                        "RPC call failed (attempt {}/{}): {}",
                        attempts + 1,
                        max_attempts,
                        e
                    );

                    if attempts < max_attempts - 1 {
                        tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    }
                }
            }

            attempts += 1;
        }

        Err(RpcError::AllEndpointsFailed)
    }

    async fn record_success(&self, endpoint_idx: usize, response_time_ms: u64) {
        let mut endpoints = self.endpoints.write().await;
        if let Some(endpoint) = endpoints.get_mut(endpoint_idx) {
            endpoint.record_success(response_time_ms);
        }

        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.successful_requests += 1;
        stats.current_endpoint_index = endpoint_idx;

        // Update average response time
        let total_successful = stats.successful_requests;
        stats.average_response_time_ms =
            (stats.average_response_time_ms * (total_successful - 1) + response_time_ms) /
            total_successful;
    }

    async fn record_error(&self, endpoint_idx: usize) {
        let mut endpoints = self.endpoints.write().await;
        if let Some(endpoint) = endpoints.get_mut(endpoint_idx) {
            endpoint.record_error();
        }

        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.failed_requests += 1;
    }

    // RPC Methods

    pub async fn get_balance(&self, pubkey: &Pubkey) -> RpcResult<u64> {
        self.execute_with_fallback(|client| {
            client.get_balance(pubkey).map_err(|e| anyhow::anyhow!("Failed to get balance: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_account(&self, pubkey: &Pubkey) -> RpcResult<solana_sdk::account::Account> {
        self.execute_with_fallback(|client| {
            client.get_account(pubkey).map_err(|e| anyhow::anyhow!("Failed to get account: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
        filter: TokenAccountsFilter
    ) -> RpcResult<Vec<RpcKeyedAccount>> {
        let owner = *owner;

        self.execute_with_fallback(move |client| {
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

        self.execute_with_fallback(|client| {
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
        self.execute_with_fallback(|client| {
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
        self.execute_with_fallback(|client| {
            client
                .get_latest_blockhash()
                .map_err(|e| anyhow::anyhow!("Failed to get latest blockhash: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn confirm_transaction(&self, signature: &Signature) -> RpcResult<bool> {
        self.execute_with_fallback(|client| {
            client
                .confirm_transaction(signature)
                .map_err(|e| anyhow::anyhow!("Failed to confirm transaction: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_transaction_status(
        &self,
        signature: &Signature
    ) -> RpcResult<Option<Result<(), solana_sdk::transaction::TransactionError>>> {
        self.execute_with_fallback(|client| {
            client
                .get_signature_status(signature)
                .map_err(|e| anyhow::anyhow!("Failed to get transaction status: {}", e))
        }).await.map_err(|_| RpcError::AllEndpointsFailed)
    }

    pub async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: Option<usize>
    ) -> RpcResult<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>> {
        let (client, _) = self.get_healthy_client().await?;
        client
            .get_signatures_for_address(address)
            .map_err(|e| types::RpcError::RequestFailed(e.to_string()))
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

        let (client, _) = self.get_healthy_client().await?;
        client
            .get_transaction_with_config(&signature, config)
            .map_err(|e| types::RpcError::RequestFailed(e.to_string()))
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
}

// Background task for health checks
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
}
