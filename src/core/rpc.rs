use crate::core::{ BotError, BotResult };
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use std::time::Duration;
use tokio::time::timeout;

/// RPC client manager with automatic retry and error handling
#[derive(Debug)]
pub struct RpcManager {
    client: RpcClient,
    timeout_duration: Duration,
    max_retries: u32,
}

impl RpcManager {
    /// Create a new RPC manager
    pub fn new(rpc_url: &str) -> BotResult<Self> {
        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed()
        );

        Ok(Self {
            client,
            timeout_duration: Duration::from_secs(30),
            max_retries: 3,
        })
    }

    /// Get account balance
    pub async fn get_balance(&self, pubkey: &Pubkey) -> BotResult<u64> {
        self.retry_operation(|| { self.client.get_balance(pubkey) }).await
    }

    /// Get token balance
    pub async fn get_token_balance(&self, pubkey: &Pubkey) -> BotResult<u64> {
        self.retry_operation(|| {
            self.client
                .get_token_account_balance(pubkey)
                .map(|balance| balance.amount.parse::<u64>().unwrap_or(0))
        }).await
    }

    /// Send transaction
    pub async fn send_transaction(&self, transaction: &Transaction) -> BotResult<Signature> {
        self.retry_operation(|| { self.client.send_and_confirm_transaction(transaction) }).await
    }

    /// Get recent signatures for address
    pub async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: usize
    ) -> BotResult<Vec<solana_transaction_status::ConfirmedSignatureInfo>> {
        self.retry_operation(|| {
            self.client.get_signatures_for_address_with_config(
                address,
                solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
                    limit: Some(limit),
                    ..Default::default()
                }
            )
        }).await
    }

    /// Generic retry wrapper for RPC operations
    async fn retry_operation<T, F>(&self, operation: F) -> BotResult<T>
        where F: Fn() -> Result<T, solana_client::client_error::ClientError>, T: Send
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match
                timeout(
                    self.timeout_duration,
                    tokio::task::spawn_blocking({
                        let op = &operation;
                        move || op()
                    })
                ).await
            {
                Ok(Ok(Ok(result))) => {
                    return Ok(result);
                }
                Ok(Ok(Err(e))) => {
                    last_error = Some(BotError::Rpc(e.to_string()));

                    if attempt < self.max_retries {
                        let delay = Duration::from_secs((2u64).pow(attempt));
                        tokio::time::sleep(delay).await;
                    }
                }
                Ok(Err(_)) => {
                    last_error = Some(BotError::Rpc("Task panicked".to_string()));
                }
                Err(_) => {
                    last_error = Some(BotError::Timeout {
                        seconds: self.timeout_duration.as_secs(),
                    });
                }
            }
        }

        Err(last_error.unwrap_or(BotError::Rpc("Unknown RPC error".to_string())))
    }

    /// Set custom timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout_duration = timeout;
    }

    /// Set max retries
    pub fn set_max_retries(&mut self, retries: u32) {
        self.max_retries = retries;
    }

    /// Check if RPC is healthy
    pub async fn health_check(&self) -> BotResult<bool> {
        match self.retry_operation(|| { self.client.get_health() }).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get current slot
    pub async fn get_slot(&self) -> BotResult<u64> {
        self.retry_operation(|| { self.client.get_slot() }).await
    }

    /// Get block time
    pub async fn get_block_time(&self, slot: u64) -> BotResult<Option<i64>> {
        self.retry_operation(|| { self.client.get_block_time(slot) }).await
    }
}
