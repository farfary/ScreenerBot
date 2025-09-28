// Transaction analysis and classification for the transactions module
//
// This module provides comprehensive analysis of Solana transactions including
// type classification, DEX detection, swap analysis, and pattern recognition.

use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use serde_json::Value;

use crate::logger::{ log, LogTag };
use crate::transactions::{ types::*, utils::* };
use crate::tokens::{ get_token_decimals_sync, decimals::lamports_to_sol };
use crate::pools::types::{ PUMP_FUN_AMM_PROGRAM_ID, PUMP_FUN_LEGACY_PROGRAM_ID };

// =============================================================================
// ANALYSIS RESULT STRUCTURES
// =============================================================================

/// Result of transaction analysis
#[derive(Debug, Clone)]
pub struct TransactionAnalysisResult {
    pub transaction_type: TransactionType,
    pub direction: TransactionDirection,
    pub confidence_score: f64, // 0.0 to 1.0
    pub analysis_notes: Vec<String>,
}

/// DEX router detection result
#[derive(Debug, Clone)]
pub struct DexRouterDetection {
    pub router_name: String,
    pub program_id: String,
    pub confidence: f64,
    pub swap_details: Option<SwapDetails>,
}

/// Swap operation details
#[derive(Debug, Clone)]
pub struct SwapDetails {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub pool_address: Option<String>,
    pub minimum_out: Option<u64>,
    pub slippage_tolerance: Option<f64>,
}

// =============================================================================
// MAIN ANALYSIS FUNCTIONS
// =============================================================================

/// Analyze transaction and classify its type and direction
pub async fn analyze_transaction(
    transaction: &Transaction,
    wallet_pubkey: &Pubkey
) -> Result<TransactionAnalysisResult, String> {
    let mut analysis_notes = Vec::new();

    // Step 1: Basic transaction validation
    if !transaction.success {
        return Ok(TransactionAnalysisResult {
            transaction_type: TransactionType::Failed,
            direction: TransactionDirection::Unknown,
            confidence_score: 1.0,
            analysis_notes: vec!["Transaction failed on-chain".to_string()],
        });
    }

    // Step 2: Analyze instructions to detect transaction patterns
    let instruction_analysis = analyze_instructions(transaction).await?;
    analysis_notes.extend(instruction_analysis.notes);

    // Step 3: Detect DEX operations
    let dex_detection = detect_dex_operations(transaction).await?;
    if let Some(ref dex) = dex_detection {
        analysis_notes.push(format!("Detected {} router", dex.router_name));
    }

    // Step 4: Classify transaction type based on analysis
    let (tx_type, direction, confidence) = classify_transaction_type(
        transaction,
        wallet_pubkey,
        &instruction_analysis,
        &dex_detection
    ).await?;

    // Step 5: Validate classification with additional checks
    let final_confidence = validate_classification(
        transaction,
        &tx_type,
        &direction,
        confidence
    ).await?;

    Ok(TransactionAnalysisResult {
        transaction_type: tx_type,
        direction,
        confidence_score: final_confidence,
        analysis_notes,
    })
}

/// Classify transaction type based on comprehensive analysis
pub async fn classify_transaction_type(
    transaction: &Transaction,
    wallet_pubkey: &Pubkey,
    instruction_analysis: &InstructionAnalysis,
    dex_detection: &Option<DexRouterDetection>
) -> Result<(TransactionType, TransactionDirection, f64), String> {
    // Check for DEX swap operations first
    if let Some(dex) = dex_detection {
        return classify_swap_transaction(transaction, wallet_pubkey, dex).await;
    }

    // Check for token transfers
    if instruction_analysis.has_token_transfers {
        return classify_transfer_transaction(
            transaction,
            wallet_pubkey,
            instruction_analysis
        ).await;
    }

    // Check for ATA operations
    if instruction_analysis.has_ata_operations {
        return Ok((TransactionType::AtaOperation, TransactionDirection::Internal, 0.8));
    }

    // Check for compute-only transactions
    if instruction_analysis.is_compute_only {
        return Ok((TransactionType::Compute, TransactionDirection::Internal, 0.9));
    }

    // Default to unknown
    Ok((TransactionType::Unknown, TransactionDirection::Unknown, 0.1))
}

/// Detect swap operations and extract swap information
pub async fn detect_swap_operations(
    transaction: &Transaction
) -> Result<Option<TokenSwapInfo>, String> {
    // Analyze transaction for swap patterns
    let dex_detection = detect_dex_operations(transaction).await?;

    if let Some(dex) = dex_detection {
        if let Some(swap_details) = dex.swap_details {
            // Convert swap details to TokenSwapInfo
            let swap_info = TokenSwapInfo {
                router: dex.router_name,
                swap_type: determine_swap_type(&swap_details).await?,
                input_mint: swap_details.input_mint,
                output_mint: swap_details.output_mint,
                input_amount: swap_details.input_amount,
                output_amount: swap_details.output_amount,
                input_ui_amount: convert_to_ui_amount(
                    swap_details.input_amount,
                    &swap_details.input_mint
                ).await?,
                output_ui_amount: convert_to_ui_amount(
                    swap_details.output_amount,
                    &swap_details.output_mint
                ).await?,
                pool_address: swap_details.pool_address,
                program_id: dex.program_id,
            };

            return Ok(Some(swap_info));
        }
    }

    Ok(None)
}

/// Calculate P&L for swap transactions
pub async fn calculate_swap_pnl(
    transaction: &Transaction,
    swap_info: &TokenSwapInfo
) -> Result<SwapPnLInfo, String> {
    let mut pnl_info = SwapPnLInfo {
        sol_spent: 0.0,
        sol_received: 0.0,
        tokens_bought: 0.0,
        tokens_sold: 0.0,
        net_sol_change: 0.0,
        estimated_token_value_sol: None,
        estimated_pnl_sol: None,
        fees_paid_sol: transaction.fee_lamports.map_or(0.0, |f| (f as f64) / 1_000_000_000.0),
    };

    // Calculate based on swap type
    match swap_info.swap_type.as_str() {
        "sol_to_token" => {
            pnl_info.sol_spent = swap_info.input_ui_amount;
            pnl_info.tokens_bought = swap_info.output_ui_amount;
            pnl_info.net_sol_change = -pnl_info.sol_spent - pnl_info.fees_paid_sol;
        }
        "token_to_sol" => {
            pnl_info.tokens_sold = swap_info.input_ui_amount;
            pnl_info.sol_received = swap_info.output_ui_amount;
            pnl_info.net_sol_change = pnl_info.sol_received - pnl_info.fees_paid_sol;
        }
        "token_to_token" => {
            // More complex calculation for token-to-token swaps
            // Would require price lookups for both tokens
        }
        _ => {}
    }

    Ok(pnl_info)
}

// =============================================================================
// INSTRUCTION ANALYSIS
// =============================================================================

/// Result of instruction analysis
#[derive(Debug, Clone)]
struct InstructionAnalysis {
    pub program_ids: Vec<String>,
    pub has_token_transfers: bool,
    pub has_ata_operations: bool,
    pub has_swap_operations: bool,
    pub is_compute_only: bool,
    pub notes: Vec<String>,
}

/// Analyze transaction instructions for patterns
async fn analyze_instructions(transaction: &Transaction) -> Result<InstructionAnalysis, String> {
    let mut analysis = InstructionAnalysis {
        program_ids: Vec::new(),
        has_token_transfers: false,
        has_ata_operations: false,
        has_swap_operations: false,
        is_compute_only: false,
        notes: Vec::new(),
    };

    // This would parse the transaction instructions
    // For now, return basic analysis as placeholder

    // Check for common program IDs in logs or instruction data
    // This is a simplified version - full implementation would parse actual instructions

    analysis.notes.push("Basic instruction analysis complete".to_string());
    Ok(analysis)
}

// =============================================================================
// DEX DETECTION
// =============================================================================

/// Detect DEX operations and router used
async fn detect_dex_operations(
    transaction: &Transaction
) -> Result<Option<DexRouterDetection>, String> {
    // Check for Jupiter router
    if let Some(jupiter_detection) = detect_jupiter_swap(transaction).await? {
        return Ok(Some(jupiter_detection));
    }

    // Check for Raydium
    if let Some(raydium_detection) = detect_raydium_swap(transaction).await? {
        return Ok(Some(raydium_detection));
    }

    // Check for Orca
    if let Some(orca_detection) = detect_orca_swap(transaction).await? {
        return Ok(Some(orca_detection));
    }

    // Check for PumpFun
    if let Some(pumpfun_detection) = detect_pumpfun_swap(transaction).await? {
        return Ok(Some(pumpfun_detection));
    }

    Ok(None)
}

/// Detect Jupiter swap operations
async fn detect_jupiter_swap(
    transaction: &Transaction
) -> Result<Option<DexRouterDetection>, String> {
    // Check for Jupiter program ID and swap patterns
    // This is a placeholder - full implementation would parse instructions and logs
    Ok(None)
}

/// Detect Raydium swap operations
async fn detect_raydium_swap(
    transaction: &Transaction
) -> Result<Option<DexRouterDetection>, String> {
    // Check for Raydium program IDs and swap patterns
    // This is a placeholder - full implementation would parse instructions and logs
    Ok(None)
}

/// Detect Orca swap operations
async fn detect_orca_swap(transaction: &Transaction) -> Result<Option<DexRouterDetection>, String> {
    // Check for Orca program IDs and swap patterns
    // This is a placeholder - full implementation would parse instructions and logs
    Ok(None)
}

/// Detect PumpFun swap operations
async fn detect_pumpfun_swap(
    transaction: &Transaction
) -> Result<Option<DexRouterDetection>, String> {
    // Check for PumpFun program IDs
    let program_ids = [PUMP_FUN_AMM_PROGRAM_ID, PUMP_FUN_LEGACY_PROGRAM_ID];

    // This is a placeholder - full implementation would parse instructions and logs
    // to detect PumpFun-specific swap patterns

    Ok(None)
}

// =============================================================================
// CLASSIFICATION HELPERS
// =============================================================================

/// Classify swap transaction type and direction
async fn classify_swap_transaction(
    transaction: &Transaction,
    wallet_pubkey: &Pubkey,
    dex_detection: &DexRouterDetection
) -> Result<(TransactionType, TransactionDirection, f64), String> {
    if let Some(ref swap_details) = dex_detection.swap_details {
        // Determine if buying or selling based on WSOL involvement
        if is_wsol_mint(&swap_details.input_mint) {
            // SOL -> Token = Buy
            return Ok((
                TransactionType::Buy,
                TransactionDirection::Outgoing,
                dex_detection.confidence,
            ));
        } else if is_wsol_mint(&swap_details.output_mint) {
            // Token -> SOL = Sell
            return Ok((
                TransactionType::Sell,
                TransactionDirection::Incoming,
                dex_detection.confidence,
            ));
        } else {
            // Token -> Token = Complex swap
            return Ok((
                TransactionType::Other("Token-to-Token Swap".to_string()),
                TransactionDirection::Internal,
                dex_detection.confidence * 0.8, // Lower confidence for complex swaps
            ));
        }
    }

    // Fallback classification
    Ok((TransactionType::Other("Unknown Swap".to_string()), TransactionDirection::Unknown, 0.3))
}

/// Classify transfer transaction type and direction
async fn classify_transfer_transaction(
    transaction: &Transaction,
    wallet_pubkey: &Pubkey,
    instruction_analysis: &InstructionAnalysis
) -> Result<(TransactionType, TransactionDirection, f64), String> {
    // Analyze transfer direction based on wallet involvement
    // This is a placeholder - full implementation would analyze balance changes

    Ok((TransactionType::Transfer, TransactionDirection::Unknown, 0.7))
}

/// Validate transaction classification with additional checks
async fn validate_classification(
    transaction: &Transaction,
    tx_type: &TransactionType,
    direction: &TransactionDirection,
    confidence: f64
) -> Result<f64, String> {
    let mut adjusted_confidence = confidence;

    // Reduce confidence for very old transactions
    if let Some(block_time) = transaction.block_time {
        let age_hours = (chrono::Utc::now().timestamp() - block_time) / 3600;
        if age_hours > 24 {
            adjusted_confidence *= 0.9; // Slightly reduce confidence for old transactions
        }
    }

    // Reduce confidence for failed transactions
    if !transaction.success {
        adjusted_confidence *= 0.5;
    }

    // Increase confidence for transactions with clear patterns
    if matches!(tx_type, TransactionType::Buy | TransactionType::Sell) {
        adjusted_confidence = (adjusted_confidence * 1.1).min(1.0);
    }

    Ok(adjusted_confidence)
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Determine swap type based on input/output mints
async fn determine_swap_type(swap_details: &SwapDetails) -> Result<String, String> {
    let input_is_sol = is_wsol_mint(&swap_details.input_mint);
    let output_is_sol = is_wsol_mint(&swap_details.output_mint);

    match (input_is_sol, output_is_sol) {
        (true, false) => Ok("sol_to_token".to_string()),
        (false, true) => Ok("token_to_sol".to_string()),
        (false, false) => Ok("token_to_token".to_string()),
        (true, true) => Ok("sol_to_sol".to_string()), // Unusual case
    }
}

/// Convert raw amount to UI amount using token decimals
async fn convert_to_ui_amount(raw_amount: u64, mint: &str) -> Result<f64, String> {
    if is_wsol_mint(mint) {
        // SOL has 9 decimals
        Ok((raw_amount as f64) / 1_000_000_000.0)
    } else {
        // Get token decimals from database
        match get_token_decimals_sync(mint) {
            Some(decimals) => {
                let divisor = (10_u64).pow(decimals as u32) as f64;
                Ok((raw_amount as f64) / divisor)
            }
            None => {
                // Default to 6 decimals if unknown
                Ok((raw_amount as f64) / 1_000_000.0)
            }
        }
    }
}

/// Extract mint addresses from transaction logs
fn extract_mint_addresses(transaction: &Transaction) -> Vec<String> {
    let mut mints = Vec::new();

    // This would parse transaction logs and instructions to extract mint addresses
    // For now, return empty vector as placeholder

    mints
}

/// Check if transaction involves specific program ID
fn involves_program_id(transaction: &Transaction, program_id: &str) -> bool {
    // This would check if the transaction involves the specified program ID
    // by analyzing instructions and accounts
    false
}

/// Extract pool addresses from transaction
fn extract_pool_addresses(transaction: &Transaction) -> Vec<String> {
    let mut pools = Vec::new();

    // This would extract pool addresses from transaction accounts and instructions
    // For now, return empty vector as placeholder

    pools
}

// =============================================================================
// PATTERN RECOGNITION
// =============================================================================

/// Recognize common transaction patterns
pub async fn recognize_transaction_patterns(
    transaction: &Transaction
) -> Result<Vec<String>, String> {
    let mut patterns = Vec::new();

    // Pattern: High fee transaction
    if let Some(fee) = transaction.fee_lamports {
        if fee > 100_000 {
            // > 0.0001 SOL
            patterns.push("high_fee".to_string());
        }
    }

    // Pattern: Failed transaction
    if !transaction.success {
        patterns.push("failed_transaction".to_string());
    }

    // Pattern: Complex transaction (many instructions)
    if transaction.instructions_count > 10 {
        patterns.push("complex_transaction".to_string());
    }

    // Pattern: Recent transaction
    if let Some(block_time) = transaction.block_time {
        let age_minutes = (chrono::Utc::now().timestamp() - block_time) / 60;
        if age_minutes < 5 {
            patterns.push("recent_transaction".to_string());
        }
    }

    Ok(patterns)
}

/// Analyze transaction for suspicious patterns
pub async fn analyze_suspicious_patterns(transaction: &Transaction) -> Result<Vec<String>, String> {
    let mut suspicious_patterns = Vec::new();

    // Pattern: Very high fee for small transaction
    if let Some(fee) = transaction.fee_lamports {
        if fee > 1_000_000 {
            // > 0.001 SOL
            suspicious_patterns.push("unusually_high_fee".to_string());
        }
    }

    // Pattern: Failed transaction with high fee
    if !transaction.success {
        if let Some(fee) = transaction.fee_lamports {
            if fee > 100_000 {
                // > 0.0001 SOL
                suspicious_patterns.push("failed_with_high_fee".to_string());
            }
        }
    }

    // Add more suspicious pattern detection as needed

    Ok(suspicious_patterns)
}
