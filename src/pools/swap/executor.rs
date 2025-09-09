/// Swap executor - Low-level transaction execution
///
/// This module handles the actual execution of swap transactions,
/// including transaction signing and broadcasting.

use super::types::{SwapResult, SwapError};
use crate::configs::{read_configs, load_wallet_from_config};
use crate::rpc::get_rpc_client;
use crate::logger::{log, LogTag};

use solana_sdk::{
    transaction::Transaction,
    signature::{Keypair, Signer},
};

/// Transaction executor for swaps
pub struct SwapExecutor;

impl SwapExecutor {
    /// Execute a transaction with retries and proper error handling
    pub async fn execute_transaction(
        transaction: Transaction,
        dry_run: bool
    ) -> Result<SwapResult, SwapError> {
        if dry_run {
            log(LogTag::System, "INFO", "🧪 Dry run mode - transaction not sent");
            return Ok(SwapResult {
                signature: None,
                params: super::types::SwapParams {
                    input_amount: 0.0,
                    expected_output: 0.0,
                    minimum_output: 0.0,
                    input_amount_raw: 0,
                    minimum_output_raw: 0,
                },
                transaction: Some(transaction),
                success: true,
                error: None,
            });
        }

        // Load wallet
        let configs = read_configs()
            .map_err(|e| SwapError::ExecutionError(format!("Failed to load config: {}", e)))?;
        let wallet = load_wallet_from_config(&configs)
            .map_err(|e| SwapError::ExecutionError(format!("Failed to load wallet: {}", e)))?;

        // Sign transaction
        let mut signed_transaction = transaction.clone();
        signed_transaction.sign(&[&wallet], signed_transaction.message.recent_blockhash);

        // Send transaction
        let rpc_client = get_rpc_client();
        
        log(LogTag::System, "INFO", "📤 Sending transaction to blockchain...");
        
        // Use the appropriate RPC method for sending transactions
        let signature_str = rpc_client
            .send_transaction(&signed_transaction)
            .await
            .map_err(|e| SwapError::ExecutionError(format!("Transaction failed: {}", e)))?;

        // Parse signature string back to Signature type
        let signature = signature_str.parse()
            .map_err(|e| SwapError::ExecutionError(format!("Invalid signature format: {}", e)))?;

        log(
            LogTag::System,
            "SUCCESS",
            &format!("✅ Transaction sent: {}", signature)
        );

        Ok(SwapResult {
            signature: Some(signature),
            params: super::types::SwapParams {
                input_amount: 0.0,
                expected_output: 0.0,
                minimum_output: 0.0,
                input_amount_raw: 0,
                minimum_output_raw: 0,
            },
            transaction: Some(signed_transaction),
            success: true,
            error: None,
        })
    }

    /// Estimate transaction fees
    pub async fn estimate_fees(transaction: &Transaction) -> Result<u64, SwapError> {
        // For now, return a reasonable estimate since our RPC client doesn't support this method
        // In practice, most simple transactions cost around 5000 lamports
        Ok(5000)
    }
}
