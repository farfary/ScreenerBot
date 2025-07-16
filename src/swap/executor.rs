use crate::swap::types::*;
use crate::rpc_manager::RpcManager;
use anyhow::{ Context, Result };
use solana_sdk::{
    signature::{ Keypair, Signature, Signer },
    transaction::Transaction,
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
};
use solana_client::rpc_config::RpcSendTransactionConfig;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use log::{ info, warn, error, debug };
use base64::{ Engine as _, engine::general_purpose };

/// Handles transaction signing and execution for swaps
pub struct SwapExecutor {
    rpc_manager: Arc<RpcManager>,
    keypair: Keypair,
    max_retries: u32,
    retry_delay: Duration,
}

impl SwapExecutor {
    pub fn new(rpc_manager: Arc<RpcManager>, keypair: Keypair) -> Self {
        Self {
            rpc_manager,
            keypair,
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
        }
    }

    /// Execute a swap by signing and sending the transaction
    pub async fn execute_swap(
        &self,
        swap_transaction: &SwapTransaction,
        route: &SwapRoute
    ) -> Result<SwapResult> {
        let start_time = Instant::now();

        info!(
            "🚀 Executing swap: {} {} → {} {}",
            route.in_amount,
            route.input_mint,
            route.out_amount,
            route.output_mint
        );

        // Try to decode the base64 transaction with better error handling
        let transaction_bytes = match
            general_purpose::STANDARD.decode(&swap_transaction.swap_transaction)
        {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(
                    anyhow::anyhow!(
                        "Failed to decode base64 transaction: {}. Transaction string length: {}",
                        e,
                        swap_transaction.swap_transaction.len()
                    )
                );
            }
        };

        debug!("Decoded transaction bytes length: {}", transaction_bytes.len());

        // Try to execute the transaction with proper format detection based on DEX type
        let signature = self.execute_transaction_bytes(
            &transaction_bytes,
            &swap_transaction.transaction_format,
            &swap_transaction.dex_type
        ).await?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        info!("✅ Swap executed successfully! Signature: {}", signature);
        Ok(SwapResult {
            success: true,
            signature: Some(signature.to_string()),
            dex_used: route.dex.to_string(),
            input_amount: route.in_amount.parse().unwrap_or(0),
            output_amount: route.out_amount.parse().unwrap_or(0),
            slippage: (route.slippage_bps as f64) / 10000.0,
            fee: 0, // TODO: Calculate actual fee from transaction
            fee_lamports: 5000, // Approximate base fee
            price_impact: route.price_impact_pct.parse().unwrap_or(0.0),
            execution_time_ms: execution_time,
            error: None,
            route: route.clone(),
            block_height: Some(0), // TODO: Get actual block height
        })
    }

    /// Execute transaction bytes with proper format handling based on DEX requirements
    async fn execute_transaction_bytes(
        &self,
        transaction_bytes: &[u8],
        transaction_format: &crate::swap::types::TransactionFormat,
        dex_type: &DexType
    ) -> Result<Signature> {
        if transaction_bytes.is_empty() {
            return Err(anyhow::anyhow!("Empty transaction bytes"));
        }

        info!("🔄 Executing {:?} transaction for {:?}", transaction_format, dex_type);

        // Route to appropriate transaction handler based on format
        match transaction_format {
            crate::swap::types::TransactionFormat::Versioned => {
                // Handle VersionedTransaction (GMGN)
                self.execute_versioned_transaction(transaction_bytes, dex_type).await
            }
            crate::swap::types::TransactionFormat::Legacy => {
                // Handle Legacy Transaction (Jupiter, Raydium)
                self.execute_legacy_transaction(transaction_bytes, dex_type).await
            }
        }
    }

    /// Execute a Legacy Transaction (for Jupiter and Raydium)
    async fn execute_legacy_transaction(
        &self,
        transaction_bytes: &[u8],
        dex_type: &DexType
    ) -> Result<Signature> {
        // Deserialize as legacy transaction
        let mut transaction: Transaction = match bincode::deserialize(transaction_bytes) {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to deserialize as legacy Transaction for {:?}: {}", dex_type, e);
                error!(
                    "Transaction bytes (first 32): {:?}",
                    &transaction_bytes[..std::cmp::min(32, transaction_bytes.len())]
                );
                return Err(anyhow::anyhow!("Failed to deserialize legacy transaction: {}", e));
            }
        };

        info!("Successfully deserialized Legacy Transaction for {:?}", dex_type);

        // Get latest blockhash and update the transaction
        let recent_blockhash = self.rpc_manager.get_latest_blockhash().await?;
        transaction.message.recent_blockhash = recent_blockhash;

        // Sign the legacy transaction
        transaction.sign(&[&self.keypair], recent_blockhash);

        // Send the transaction
        debug!("Sending legacy transaction for {:?}", dex_type);
        let signature = self.rpc_manager.send_and_confirm_transaction(&transaction).await?;
        info!("✅ Legacy transaction confirmed: {}", signature);

        Ok(signature)
    }

    /// Execute a VersionedTransaction (for GMGN and newer formats)
    async fn execute_versioned_transaction(
        &self,
        transaction_bytes: &[u8],
        dex_type: &DexType
    ) -> Result<Signature> {
        use solana_sdk::transaction::VersionedTransaction;
        use solana_sdk::message::VersionedMessage;

        let mut versioned_tx: VersionedTransaction = match bincode::deserialize(transaction_bytes) {
            Ok(tx) => tx,
            Err(e) => {
                debug!("Failed to deserialize as VersionedTransaction for {:?}: {}", dex_type, e);
                return Err(anyhow::anyhow!("Not a VersionedTransaction: {}", e));
            }
        };

        info!("Successfully deserialized VersionedTransaction for {:?}", dex_type);

        // Get latest blockhash and update the transaction
        let recent_blockhash = self.rpc_manager.get_latest_blockhash().await?;

        // Update the recent blockhash in the versioned transaction
        match &mut versioned_tx.message {
            VersionedMessage::Legacy(ref mut legacy_msg) => {
                legacy_msg.recent_blockhash = recent_blockhash;
            }
            VersionedMessage::V0(ref mut v0_msg) => {
                v0_msg.recent_blockhash = recent_blockhash;
            }
        }

        // Sign the versioned transaction
        let message = &versioned_tx.message;
        let signature = self.keypair.sign_message(&message.serialize());
        versioned_tx.signatures = vec![signature];

        // Send the versioned transaction
        debug!("Sending VersionedTransaction for {:?}", dex_type);
        let signature = self.rpc_manager.send_and_confirm_versioned_transaction(
            &versioned_tx
        ).await?;
        info!("✅ VersionedTransaction confirmed: {}", signature);

        Ok(signature)
    }

    /// Add priority fee instructions to the transaction
    fn add_priority_fee(
        &self,
        transaction: &mut Transaction,
        swap_tx: &SwapTransaction
    ) -> Result<()> {
        // For now, skip adding priority fees to avoid complex instruction manipulation
        // The transaction from the DEX should already include appropriate fees
        debug!("Priority fee addition skipped - using DEX provided transaction as-is");

        if let Some(priority_fee_info) = &swap_tx.priority_fee_info {
            if let Some(fee_estimate) = priority_fee_info.priority_fee_estimate {
                debug!("DEX suggested priority fee: {} microlamports", fee_estimate);
            }
        }

        Ok(())
    }

    /// Send transaction with retry logic
    async fn send_transaction_with_retries(&self, transaction: &Transaction) -> Result<Signature> {
        let config = RpcSendTransactionConfig {
            skip_preflight: false,
            preflight_commitment: Some(CommitmentConfig::confirmed().commitment),
            encoding: None,
            max_retries: Some(self.max_retries as usize),
            min_context_slot: None,
        };

        for attempt in 1..=self.max_retries {
            match self.rpc_manager.send_transaction_with_config(transaction, config).await {
                Ok(signature) => {
                    debug!("Transaction sent successfully on attempt {}: {}", attempt, signature);
                    return Ok(signature);
                }
                Err(e) => {
                    warn!("Transaction send attempt {} failed: {}", attempt, e);
                    if attempt < self.max_retries {
                        tokio::time::sleep(self.retry_delay * attempt).await;
                    } else {
                        return Err(
                            anyhow::anyhow!(
                                "Failed to send transaction after {} attempts: {}",
                                self.max_retries,
                                e
                            )
                        );
                    }
                }
            }
        }

        Err(anyhow::anyhow!("All transaction send attempts failed"))
    }

    /// Wait for transaction confirmation with timeout
    async fn wait_for_confirmation(
        &self,
        signature: &Signature,
        last_valid_block_height: u64
    ) -> Result<u64> {
        let timeout = Duration::from_secs(60); // 60 second timeout
        let start_time = Instant::now();
        let poll_interval = Duration::from_millis(1000); // Poll every second

        info!("⏳ Waiting for confirmation of transaction: {}", signature);

        while start_time.elapsed() < timeout {
            // Check current block height to avoid waiting past expiration
            let current_height_result = self.rpc_manager.get_block_height().await;

            if let Ok(current_height) = current_height_result {
                if current_height > last_valid_block_height {
                    return Err(
                        anyhow::anyhow!(
                            "Transaction expired: current height {} > last valid height {}",
                            current_height,
                            last_valid_block_height
                        )
                    );
                }
            } else {
                warn!("Failed to get current block height");
            }

            // Check transaction status
            let signature_clone = *signature;
            let status_result = self.rpc_manager.get_signature_status_with_commitment(
                &signature_clone,
                CommitmentConfig::confirmed()
            ).await;

            match status_result {
                Ok(Some(status)) => {
                    match status {
                        Ok(()) => {
                            // Transaction confirmed successfully
                            let block_height_result = self.rpc_manager.get_block_height().await;

                            match block_height_result {
                                Ok(block_height) => {
                                    info!("✅ Transaction confirmed at block height: {}", block_height);
                                    return Ok(block_height);
                                }
                                Err(_) => {
                                    info!("✅ Transaction confirmed");
                                    return Ok(0); // Return 0 if we can't get block height
                                }
                            }
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Transaction failed: {}", e));
                        }
                    }
                }
                Ok(None) => {
                    // Transaction not yet confirmed, continue polling
                    debug!("Transaction not yet confirmed, continuing to wait...");
                }
                Err(e) => {
                    warn!("Error checking transaction status: {}", e);
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        Err(anyhow::anyhow!("Transaction confirmation timeout after {:?}", timeout))
    }

    /// Get wallet public key
    pub fn get_public_key(&self) -> String {
        self.keypair.pubkey().to_string()
    }

    /// Get wallet keypair (for testing purposes)
    pub fn get_keypair(&self) -> &Keypair {
        &self.keypair
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;

    #[test]
    fn test_swap_executor_creation() {
        let keypair = Keypair::new();
        let rpc_manager = Arc::new(
            RpcManager::new(vec!["https://api.mainnet-beta.solana.com".to_string()]).unwrap()
        );
        let executor = SwapExecutor::new(rpc_manager, keypair);
        assert_eq!(executor.max_retries, 3);
    }
}
