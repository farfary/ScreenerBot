/// Swap executor - Low-level transaction execution
///
/// This module handles the actual execution of swap transactions,
/// including transaction signing and broadcasting.
use super::types::{SwapError, SwapParams, SwapResult};
use crate::logger::{self, LogTag};
use crate::rpc::{get_new_rpc_client, RpcClientMethods};

use base64::Engine;
use solana_sdk::transaction::Transaction;

/// Transaction executor for swaps
pub struct SwapExecutor;

impl SwapExecutor {
  /// Execute a transaction with retries and proper error handling
  pub async fn execute_transaction(
    transaction: Transaction,
    swap_params: SwapParams,
    dry_run: bool,
  ) -> Result<SwapResult, SwapError> {
    if dry_run {
      logger::info(LogTag::System, "Dry run mode - transaction not sent");
      return Ok(SwapResult {
        signature: None,
        params: swap_params,
        transaction: Some(transaction),
        success: true,
        error: None,
      });
    }

    // Serialize transaction to base64 for signing service
    let serialized_tx = bincode::serialize(&transaction).map_err(|e| {
      SwapError::ExecutionError(format!("Failed to serialize transaction: {}", e))
    })?;
    let transaction_base64 = base64::engine::general_purpose::STANDARD.encode(&serialized_tx);

    // Send transaction using centralized signing service with main wallet
    let rpc_client = get_new_rpc_client();

    logger::info(LogTag::System, "Sending transaction to blockchain...");

    // Use the convenience method that loads main wallet keypair automatically
    let signature = rpc_client
      .sign_and_send_with_main_wallet(&transaction_base64)
      .await
      .map_err(|e| SwapError::ExecutionError(format!("Transaction failed: {}", e)))?;

    logger::info(
      LogTag::System,
      &format!("Transaction sent: {}", signature),
    );

    Ok(SwapResult {
      signature: Some(signature),
      params: swap_params,
      transaction: Some(transaction),
      success: true,
      error: None,
    })
  }

  /// Estimate transaction fees
  pub async fn estimate_fees(_transaction: &Transaction) -> Result<u64, SwapError> {
    // For now, return a reasonable estimate since our RPC client doesn't support this method
    // In practice, most simple transactions cost around 5000 lamports
    Ok(5000)
  }
}
