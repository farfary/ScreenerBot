/// Finalization Guard System
/// Ensures critical operations only proceed with finalized transactions

use crate::logger::{log, LogTag};
use crate::transactions_manager::get_transaction_details_global;
use crate::rpc::get_rpc_client;

/// Check if a transaction is finalized before proceeding with critical operations
pub async fn ensure_transaction_finalized(signature: &str) -> Result<bool, String> {
    log(LogTag::Position, "FINALIZATION_CHECK", 
        &format!("🔍 Checking finalization status for transaction: {}", &signature[..8]));
    
    let rpc_client = get_rpc_client();
    
    // Try to get transaction with finalized commitment level
    match rpc_client.get_transaction_details_finalized_rpc(signature).await {
        Ok(_) => {
            log(LogTag::Position, "FINALIZED", 
                &format!("✅ Transaction {} is finalized", &signature[..8]));
            Ok(true)
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("not found") || error_msg.contains("not finalized") {
                log(LogTag::Position, "NOT_FINALIZED", 
                    &format!("⏳ Transaction {} not yet finalized", &signature[..8]));
                Ok(false)
            } else {
                log(LogTag::Position, "ERROR", 
                    &format!("❌ Error checking finalization for {}: {}", &signature[..8], error_msg));
                Err(format!("Failed to check finalization: {}", error_msg))
            }
        }
    }
}

/// Wait for transaction finalization with timeout
pub async fn wait_for_finalization(signature: &str, max_attempts: u32) -> Result<bool, String> {
    log(LogTag::Position, "FINALIZATION_WAIT", 
        &format!("⏳ Waiting for transaction finalization: {} (max {} attempts)", 
                &signature[..8], max_attempts));
    
    for attempt in 1..=max_attempts {
        match ensure_transaction_finalized(signature).await {
            Ok(true) => {
                log(LogTag::Position, "FINALIZED", 
                    &format!("✅ Transaction {} finalized after {} attempts", 
                            &signature[..8], attempt));
                return Ok(true);
            }
            Ok(false) => {
                if attempt < max_attempts {
                    // Wait 10 seconds between attempts (finalization typically takes 30-60 seconds)
                    log(LogTag::Position, "WAITING", 
                        &format!("⏳ Attempt {}/{}, waiting 10s for finalization: {}", 
                                attempt, max_attempts, &signature[..8]));
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                } else {
                    log(LogTag::Position, "TIMEOUT", 
                        &format!("⏰ Finalization timeout after {} attempts: {}", 
                                max_attempts, &signature[..8]));
                    return Ok(false);
                }
            }
            Err(e) => {
                log(LogTag::Position, "ERROR", 
                    &format!("❌ Finalization check error for {}: {}", &signature[..8], e));
                return Err(e);
            }
        }
    }
    
    Ok(false)
}

/// Guard for position creation - only allows finalized transactions
pub async fn guard_position_creation(signature: &str) -> Result<(), String> {
    log(LogTag::Position, "CREATION_GUARD", 
        &format!("🛡️ Position creation guard checking transaction: {}", &signature[..8]));
    
    match ensure_transaction_finalized(signature).await {
        Ok(true) => {
            log(LogTag::Position, "CREATION_APPROVED", 
                &format!("✅ Position creation approved - transaction finalized: {}", &signature[..8]));
            Ok(())
        }
        Ok(false) => {
            log(LogTag::Position, "CREATION_DENIED", 
                &format!("❌ Position creation denied - transaction not finalized: {}", &signature[..8]));
            Err(format!("Transaction {} not finalized - cannot create position", signature))
        }
        Err(e) => {
            log(LogTag::Position, "CREATION_ERROR", 
                &format!("❌ Position creation guard error for {}: {}", &signature[..8], e));
            Err(e)
        }
    }
}

/// Guard for position closure - only allows finalized transactions
pub async fn guard_position_closure(entry_signature: &str, exit_signature: &str) -> Result<(), String> {
    log(LogTag::Position, "CLOSURE_GUARD", 
        &format!("🛡️ Position closure guard checking entry: {} exit: {}", 
                &entry_signature[..8], &exit_signature[..8]));
    
    // Check entry transaction finalization
    match ensure_transaction_finalized(entry_signature).await {
        Ok(true) => {
            log(LogTag::Position, "ENTRY_FINALIZED", 
                &format!("✅ Entry transaction finalized: {}", &entry_signature[..8]));
        }
        Ok(false) => {
            return Err(format!("Entry transaction {} not finalized - cannot close position", entry_signature));
        }
        Err(e) => {
            return Err(format!("Error checking entry finalization: {}", e));
        }
    }
    
    // Check exit transaction finalization
    match ensure_transaction_finalized(exit_signature).await {
        Ok(true) => {
            log(LogTag::Position, "EXIT_FINALIZED", 
                &format!("✅ Exit transaction finalized: {}", &exit_signature[..8]));
        }
        Ok(false) => {
            return Err(format!("Exit transaction {} not finalized - cannot close position", exit_signature));
        }
        Err(e) => {
            return Err(format!("Error checking exit finalization: {}", e));
        }
    }
    
    log(LogTag::Position, "CLOSURE_APPROVED", 
        &format!("✅ Position closure approved - both transactions finalized"));
    Ok(())
}
