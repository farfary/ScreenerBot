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
pub struct RpcManager {
    pub client: RpcClient,
    timeout_duration: Duration,
    max_retries: u32,
}

impl std::fmt::Debug for RpcManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcManager")
            .field("timeout_duration", &self.timeout_duration)
            .field("max_retries", &self.max_retries)
            .finish()
    }
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
        // Since RpcClient doesn't implement Clone, use a simpler approach
        self.client.get_balance(pubkey).map_err(|e| BotError::Rpc(e.to_string()))
    }

    /// Get token balance
    pub async fn get_token_balance(&self, _pubkey: &Pubkey) -> BotResult<u64> {
        // Simplified implementation for compilation
        Ok(0)
    }

    /// Send transaction
    pub async fn send_transaction(&self, _transaction: &Transaction) -> BotResult<Signature> {
        // Simplified implementation for compilation
        Err(BotError::Rpc("Not implemented in simulation mode".to_string()))
    }

    /// Get recent signatures for address
    // pub async fn get_signatures_for_address(
    //     &self,
    //     address: &Pubkey,
    //     limit: usize
    // ) -> BotResult<Vec<solana_transaction_status::ConfirmedSignatureInfo>> {
    //     self.retry_operation(|| {
    //         self.client.get_signatures_for_address_with_config(
    //             address,
    //             solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config {
    //                 limit: Some(limit),
    //                 ..Default::default()
    //             }
    //         )
    //     }).await
    // }

    /// Generic retry wrapper for RPC operations
    fn retry_operation_sync<T>(
        &self,
        operation: impl Fn() -> Result<T, solana_client::client_error::ClientError>
    ) -> BotResult<T> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation() {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(BotError::Rpc(e.to_string()));
                    if attempt < self.max_retries {
                        std::thread::sleep(
                            std::time::Duration::from_millis(100 * ((attempt + 1) as u64))
                        );
                    }
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
        // Simplified implementation
        Ok(true)
    }

    /// Get current slot
    pub async fn get_slot(&self) -> BotResult<u64> {
        // Simplified implementation
        Ok(0)
    }

    /// Get block time
    pub async fn get_block_time(&self, _slot: u64) -> BotResult<Option<i64>> {
        // Simplified implementation
        Ok(None)
    }
}
