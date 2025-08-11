/// Transaction Analyzer for Detailed Transaction Analysis
/// 
/// This module provides comprehensive analysis of Solana transactions including:
/// - Transaction type detection (swaps, transfers, spam, etc.)
/// - DEX identification (Jupiter, Pump.fun, GMGN, Raydium, etc.)
/// - Balance change calculation
/// - Position impact analysis
/// - Fee extraction and calculation
/// - Instruction parsing and log analysis

use crate::logger::{log, LogTag};
use crate::global::is_debug_transactions_enabled;
use crate::rpc::get_rpc_client;
use crate::tokens::{get_token_decimals_sync, get_pool_service, decimals::{LAMPORTS_PER_SOL}};
use crate::transactions_manager::{
    Transaction, TransactionType, TransactionState, TokenBalanceChange, 
    SolBalanceChange, PositionImpact
};

use std::collections::HashMap;
use std::str::FromStr;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    commitment_config::CommitmentConfig,
};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta,
    EncodedTransaction,
    UiTransactionEncoding,
    UiTransactionStatusMeta,
    UiInstruction,
    UiParsedInstruction,
    UiCompiledInstruction,
    UiAccountInfo,
    UiPartiallyDecodedInstruction,
    UiTransactionTokenBalance,
};
use serde_json::Value;
use regex::Regex;
use once_cell::sync::Lazy;

// =============================================================================
// KNOWN PROGRAM IDs FOR DEX IDENTIFICATION
// =============================================================================

/// Jupiter V6 program ID
const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

/// Jupiter V4 program ID (legacy)
const JUPITER_V4_PROGRAM_ID: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

/// Pump.fun program ID
const PUMP_FUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Raydium AMM program ID
const RAYDIUM_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Raydium V4 program ID
const RAYDIUM_V4_PROGRAM_ID: &str = "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv";

/// Orca program ID
const ORCA_PROGRAM_ID: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";

/// Saber program ID
const SABER_PROGRAM_ID: &str = "SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ";

/// Serum DEX program ID
const SERUM_DEX_PROGRAM_ID: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";

/// Solana system program ID
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111112";

/// SPL Token program ID
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// SPL Token 2022 program ID
const SPL_TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// Associated Token Account program ID
const ATA_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// GMGN program ID (if known)
const GMGN_PROGRAM_ID: &str = ""; // To be filled when known

/// Compute Budget program ID
const COMPUTE_BUDGET_PROGRAM_ID: &str = "ComputeBudget111111111111111111111111111111";

// Known spam/scam token mints (can be expanded)
static SPAM_TOKEN_MINTS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        // Add known spam token mints here
        "SiLVN1gF3W2DKmTK8KGQAPe9s7kqFjFQfG1TMt2dGUJ", // Example spam token
    ]
});

// Minimum transfer amounts to consider (to filter dust)
const MIN_SOL_TRANSFER: f64 = 0.001; // 0.001 SOL
const MIN_TOKEN_USD_VALUE: f64 = 0.01; // $0.01 USD

// =============================================================================
// TRANSACTION ANALYSIS IMPLEMENTATION
// =============================================================================

impl crate::transactions_manager::TransactionManager {
    /// Analyze a transaction and populate all its fields
    pub async fn analyze_transaction(&self, transaction: &mut Transaction) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        
        if is_debug_transactions_enabled() {
            log(
                LogTag::Transactions,
                "DEBUG",
                &format!("Starting analysis of transaction: {}", transaction.signature)
            );
        }

        // Get the full transaction data from RPC
        match self.fetch_transaction_data(&transaction.signature).await {
            Ok(Some(tx_data)) => {
                // Store minimal reference, avoid full clone
                // transaction.raw_transaction = Some(tx_data);
                
                // Extract basic information
                self.extract_basic_info(transaction, &tx_data).await?;
                
                // Analyze transaction type and details
                self.analyze_transaction_type(transaction, &tx_data).await?;
                
                // Calculate balance changes
                self.calculate_balance_changes(transaction, &tx_data).await?;
                
                // Calculate position impacts
                self.calculate_position_impacts(transaction).await?;
                
                // Mark as analyzed
                transaction.mark_analyzed();
                
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Transactions,
                        "DEBUG",
                        &format!(
                            "Completed analysis of {} in {:?} - Type: {:?}",
                            transaction.signature,
                            start_time.elapsed(),
                            transaction.transaction_type
                        )
                    );
                }
            }
            Ok(None) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Transaction data not found for: {}", transaction.signature)
                );
                transaction.confirmation_status = TransactionState::Dropped;
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "ERROR",
                    &format!("Failed to fetch transaction data for {}: {}", transaction.signature, e)
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// Fetch full transaction data from RPC
    async fn fetch_transaction_data(&self, signature: &str) -> Result<Option<EncodedConfirmedTransactionWithStatusMeta>, Box<dyn std::error::Error>> {
        let rpc_client = get_rpc_client();
        
        // Update RPC call counter
        {
            let mut stats = self.stats.write().await;
            stats.rpc_calls_made += 1;
        }

        let signature_obj = Signature::from_str(signature)?;
        
        match rpc_client.get_transaction_with_config(
            &signature_obj,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        ) {
            Ok(tx_data) => Ok(Some(tx_data)),
            Err(e) => {
                // Check if it's a "not found" error vs other errors
                let error_str = e.to_string();
                if error_str.contains("not found") || error_str.contains("Transaction not found") {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Extract basic transaction information
    async fn extract_basic_info(
        &self,
        transaction: &mut Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update slot and block time if available
        if transaction.slot.is_none() {
            transaction.slot = Some(tx_data.slot);
        }
        
        if transaction.block_time.is_none() {
            transaction.block_time = tx_data.block_time;
        }

        // Extract fee information
        if let Some(meta) = &tx_data.transaction.meta {
            transaction.fee = meta.fee as f64 / LAMPORTS_PER_SOL as f64;
            
            // Extract logs
            match &meta.log_messages {
                solana_transaction_status::option_serializer::OptionSerializer::Some(logs) => {
                    transaction.logs = logs.clone();
                }
                _ => {}
            }

            // Update success status based on meta
            transaction.success = meta.err.is_none();
            
            if let Some(err) = &meta.err {
                transaction.error_message = Some(format!("{:?}", err));
                transaction.confirmation_status = TransactionState::Failed(format!("{:?}", err));
            }
        }

        // Extract instructions and program IDs
        if let UiTransactionEncoding::Json = tx_data.transaction.transaction {
            // Handle JSON encoded transaction
            // For now, just extract basic info from meta
        } else {
            // For other encodings, extract what we can from meta
        }

        Ok(())
    }

    /// Analyze transaction type based on instructions and involved programs
    async fn analyze_transaction_type(
        &self,
        transaction: &mut Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check for DEX involvement
        self.detect_dex_involvement(transaction);
        
        // Analyze instructions to determine transaction type
        if let Some(ui_transaction) = &tx_data.transaction.transaction {
            if let Some(message) = &ui_transaction.message {
                if let Some(instructions) = &message.instructions {
                    
                    // Check for swap patterns
                    if self.is_jupiter_swap(instructions) {
                        transaction.jupiter_swap = true;
                        transaction.dex_involved = Some("Jupiter".to_string());
                        transaction.transaction_type = self.analyze_jupiter_swap(transaction, tx_data).await?;
                    }
                    else if self.is_pump_fun_transaction(instructions) {
                        transaction.pump_fun_involved = true;
                        transaction.dex_involved = Some("Pump.fun".to_string());
                        transaction.transaction_type = self.analyze_pump_fun_swap(transaction, tx_data).await?;
                    }
                    else if self.is_raydium_swap(instructions) {
                        transaction.dex_involved = Some("Raydium".to_string());
                        transaction.transaction_type = self.analyze_raydium_swap(transaction, tx_data).await?;
                    }
                    else if self.is_orca_swap(instructions) {
                        transaction.dex_involved = Some("Orca".to_string());
                        transaction.transaction_type = self.analyze_orca_swap(transaction, tx_data).await?;
                    }
                    // Check for simple transfers
                    else if self.is_sol_transfer(instructions) {
                        transaction.transaction_type = self.analyze_sol_transfer(transaction, tx_data).await?;
                    }
                    else if self.is_token_transfer(instructions) {
                        transaction.transaction_type = self.analyze_token_transfer(transaction, tx_data).await?;
                    }
                    // Check for system operations
                    else if self.is_system_operation(instructions) {
                        transaction.transaction_type = self.analyze_system_operation(transaction, tx_data).await?;
                    }
                    // Default to unknown
                    else {
                        transaction.transaction_type = TransactionType::Unknown {
                            program_ids: transaction.program_ids.clone(),
                            fee: transaction.fee,
                        };
                    }
                }
            }
        }

        // Check if transaction should be classified as spam
        if self.is_spam_transaction(transaction) {
            let reason = self.get_spam_reason(transaction);
            transaction.transaction_type = TransactionType::Spam {
                reason,
                amount: self.get_transaction_amount(transaction),
            };
        }

        Ok(())
    }

    /// Detect which DEXes are involved in the transaction
    fn detect_dex_involvement(&self, transaction: &mut Transaction) {
        for program_id in &transaction.program_ids {
            match program_id.as_str() {
                JUPITER_V6_PROGRAM_ID | JUPITER_V4_PROGRAM_ID => {
                    transaction.jupiter_swap = true;
                }
                PUMP_FUN_PROGRAM_ID => {
                    transaction.pump_fun_involved = true;
                }
                GMGN_PROGRAM_ID if !GMGN_PROGRAM_ID.is_empty() => {
                    transaction.gmgn_involved = true;
                }
                ATA_PROGRAM_ID => {
                    transaction.ata_created = true;
                }
                _ => {}
            }
        }
    }

    /// Check if transaction is a Jupiter swap
    fn is_jupiter_swap(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::PartiallyDecoded(partial) => {
                    partial.program_id == JUPITER_V6_PROGRAM_ID || 
                    partial.program_id == JUPITER_V4_PROGRAM_ID
                }
                UiInstruction::Parsed(parsed) => {
                    if let Some(program) = &parsed.program {
                        program == "jupiter" || program.contains("jupiter")
                    } else {
                        false
                    }
                }
                _ => false,
            }
        })
    }

    /// Check if transaction is a Pump.fun transaction
    fn is_pump_fun_transaction(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::PartiallyDecoded(partial) => {
                    partial.program_id == PUMP_FUN_PROGRAM_ID
                }
                UiInstruction::Parsed(parsed) => {
                    if let Some(program) = &parsed.program {
                        program.contains("pump") || program.contains("pumpfun")
                    } else {
                        false
                    }
                }
                _ => false,
            }
        })
    }

    /// Check if transaction is a Raydium swap
    fn is_raydium_swap(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::PartiallyDecoded(partial) => {
                    partial.program_id == RAYDIUM_AMM_PROGRAM_ID || 
                    partial.program_id == RAYDIUM_V4_PROGRAM_ID
                }
                UiInstruction::Parsed(parsed) => {
                    if let Some(program) = &parsed.program {
                        program.contains("raydium")
                    } else {
                        false
                    }
                }
                _ => false,
            }
        })
    }

    /// Check if transaction is an Orca swap
    fn is_orca_swap(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::PartiallyDecoded(partial) => {
                    partial.program_id == ORCA_PROGRAM_ID
                }
                UiInstruction::Parsed(parsed) => {
                    if let Some(program) = &parsed.program {
                        program.contains("orca")
                    } else {
                        false
                    }
                }
                _ => false,
            }
        })
    }

    /// Check if transaction is a simple SOL transfer
    fn is_sol_transfer(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::Parsed(parsed) => {
                    parsed.program == "system" && 
                    parsed.parsed.get("type").and_then(|v| v.as_str()) == Some("transfer")
                }
                _ => false,
            }
        })
    }

    /// Check if transaction is a token transfer
    fn is_token_transfer(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::Parsed(parsed) => {
                    (parsed.program == "spl-token" || parsed.program == "spl-token-2022") &&
                    parsed.parsed.get("type").and_then(|v| v.as_str()) == Some("transfer")
                }
                _ => false,
            }
        })
    }

    /// Check if transaction is a system operation
    fn is_system_operation(&self, instructions: &[UiInstruction]) -> bool {
        instructions.iter().any(|instruction| {
            match instruction {
                UiInstruction::Parsed(parsed) => {
                    parsed.program == "system" && 
                    matches!(
                        parsed.parsed.get("type").and_then(|v| v.as_str()),
                        Some("createAccount") | Some("allocate") | Some("assign")
                    )
                }
                _ => false,
            }
        })
    }

    /// Check if transaction should be classified as spam
    fn is_spam_transaction(&self, transaction: &Transaction) -> bool {
        // Check for known spam tokens
        for change in &transaction.token_balance_changes {
            if SPAM_TOKEN_MINTS.contains(&change.mint.as_str()) {
                return true;
            }
        }

        // Check for very small amounts (potential dust/spam)
        if let Some(sol_change) = &transaction.sol_balance_change {
            if sol_change.change.abs() < MIN_SOL_TRANSFER && sol_change.change.abs() > 0.0 {
                return true;
            }
        }

        // Check for token transfers with very small USD value
        for change in &transaction.token_balance_changes {
            if let Some(usd_value) = change.usd_value {
                if usd_value < MIN_TOKEN_USD_VALUE && usd_value > 0.0 {
                    return true;
                }
            }
        }

        false
    }

    /// Get reason for spam classification
    fn get_spam_reason(&self, transaction: &Transaction) -> String {
        // Check for known spam tokens
        for change in &transaction.token_balance_changes {
            if SPAM_TOKEN_MINTS.contains(&change.mint.as_str()) {
                return format!("Known spam token: {}", change.mint);
            }
        }

        // Check for dust amounts
        if let Some(sol_change) = &transaction.sol_balance_change {
            if sol_change.change.abs() < MIN_SOL_TRANSFER && sol_change.change.abs() > 0.0 {
                return format!("Dust SOL amount: {}", sol_change.change);
            }
        }

        // Check for low value token transfers
        for change in &transaction.token_balance_changes {
            if let Some(usd_value) = change.usd_value {
                if usd_value < MIN_TOKEN_USD_VALUE && usd_value > 0.0 {
                    return format!("Low value token transfer: ${:.4}", usd_value);
                }
            }
        }

        "Unknown spam pattern".to_string()
    }

    /// Get total transaction amount for spam detection
    fn get_transaction_amount(&self, transaction: &Transaction) -> f64 {
        let mut total = 0.0;

        if let Some(sol_change) = &transaction.sol_balance_change {
            total += sol_change.change.abs();
        }

        for change in &transaction.token_balance_changes {
            if let Some(usd_value) = change.usd_value {
                total += usd_value;
            }
        }

        total
    }

    // =============================================================================
    // SPECIFIC TRANSACTION TYPE ANALYZERS
    // =============================================================================

    /// Analyze Jupiter swap transaction
    async fn analyze_jupiter_swap(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        // Analyze balance changes to determine swap type
        let sol_changes = transaction.sol_balance_change.as_ref();
        let token_changes = &transaction.token_balance_changes;

        // Jupiter swaps typically involve SOL and one or more tokens
        match (sol_changes, token_changes.len()) {
            // SOL to Token swap
            (Some(sol_change), 1) if sol_change.change < 0.0 && token_changes[0].change > 0.0 => {
                Ok(TransactionType::SwapSolToToken {
                    token_mint: token_changes[0].mint.clone(),
                    sol_amount: sol_change.change.abs(),
                    token_amount: token_changes[0].change,
                    dex: "Jupiter".to_string(),
                    fee: transaction.fee,
                })
            }
            // Token to SOL swap
            (Some(sol_change), 1) if sol_change.change > 0.0 && token_changes[0].change < 0.0 => {
                Ok(TransactionType::SwapTokenToSol {
                    token_mint: token_changes[0].mint.clone(),
                    token_amount: token_changes[0].change.abs(),
                    sol_amount: sol_change.change,
                    dex: "Jupiter".to_string(),
                    fee: transaction.fee,
                })
            }
            // Token to Token swap
            (_, 2) if token_changes[0].change < 0.0 && token_changes[1].change > 0.0 => {
                Ok(TransactionType::SwapTokenToToken {
                    from_mint: token_changes[0].mint.clone(),
                    to_mint: token_changes[1].mint.clone(),
                    from_amount: token_changes[0].change.abs(),
                    to_amount: token_changes[1].change,
                    dex: "Jupiter".to_string(),
                    fee: transaction.fee,
                })
            }
            _ => {
                // Complex swap or unrecognized pattern
                Ok(TransactionType::Unknown {
                    program_ids: transaction.program_ids.clone(),
                    fee: transaction.fee,
                })
            }
        }
    }

    /// Analyze Pump.fun transaction
    async fn analyze_pump_fun_swap(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        // Pump.fun transactions are typically SOL to token swaps
        let sol_changes = transaction.sol_balance_change.as_ref();
        let token_changes = &transaction.token_balance_changes;

        match (sol_changes, token_changes.len()) {
            (Some(sol_change), 1) if sol_change.change < 0.0 && token_changes[0].change > 0.0 => {
                Ok(TransactionType::SwapSolToToken {
                    token_mint: token_changes[0].mint.clone(),
                    sol_amount: sol_change.change.abs(),
                    token_amount: token_changes[0].change,
                    dex: "Pump.fun".to_string(),
                    fee: transaction.fee,
                })
            }
            (Some(sol_change), 1) if sol_change.change > 0.0 && token_changes[0].change < 0.0 => {
                Ok(TransactionType::SwapTokenToSol {
                    token_mint: token_changes[0].mint.clone(),
                    token_amount: token_changes[0].change.abs(),
                    sol_amount: sol_change.change,
                    dex: "Pump.fun".to_string(),
                    fee: transaction.fee,
                })
            }
            _ => {
                Ok(TransactionType::Unknown {
                    program_ids: transaction.program_ids.clone(),
                    fee: transaction.fee,
                })
            }
        }
    }

    /// Analyze Raydium swap transaction
    async fn analyze_raydium_swap(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        // Similar to Jupiter analysis but with Raydium branding
        let sol_changes = transaction.sol_balance_change.as_ref();
        let token_changes = &transaction.token_balance_changes;

        match (sol_changes, token_changes.len()) {
            (Some(sol_change), 1) if sol_change.change < 0.0 && token_changes[0].change > 0.0 => {
                Ok(TransactionType::SwapSolToToken {
                    token_mint: token_changes[0].mint.clone(),
                    sol_amount: sol_change.change.abs(),
                    token_amount: token_changes[0].change,
                    dex: "Raydium".to_string(),
                    fee: transaction.fee,
                })
            }
            (Some(sol_change), 1) if sol_change.change > 0.0 && token_changes[0].change < 0.0 => {
                Ok(TransactionType::SwapTokenToSol {
                    token_mint: token_changes[0].mint.clone(),
                    token_amount: token_changes[0].change.abs(),
                    sol_amount: sol_change.change,
                    dex: "Raydium".to_string(),
                    fee: transaction.fee,
                })
            }
            (_, 2) if token_changes[0].change < 0.0 && token_changes[1].change > 0.0 => {
                Ok(TransactionType::SwapTokenToToken {
                    from_mint: token_changes[0].mint.clone(),
                    to_mint: token_changes[1].mint.clone(),
                    from_amount: token_changes[0].change.abs(),
                    to_amount: token_changes[1].change,
                    dex: "Raydium".to_string(),
                    fee: transaction.fee,
                })
            }
            _ => {
                Ok(TransactionType::Unknown {
                    program_ids: transaction.program_ids.clone(),
                    fee: transaction.fee,
                })
            }
        }
    }

    /// Analyze Orca swap transaction
    async fn analyze_orca_swap(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        // Similar to other DEX analysis
        let sol_changes = transaction.sol_balance_change.as_ref();
        let token_changes = &transaction.token_balance_changes;

        match (sol_changes, token_changes.len()) {
            (Some(sol_change), 1) if sol_change.change < 0.0 && token_changes[0].change > 0.0 => {
                Ok(TransactionType::SwapSolToToken {
                    token_mint: token_changes[0].mint.clone(),
                    sol_amount: sol_change.change.abs(),
                    token_amount: token_changes[0].change,
                    dex: "Orca".to_string(),
                    fee: transaction.fee,
                })
            }
            (Some(sol_change), 1) if sol_change.change > 0.0 && token_changes[0].change < 0.0 => {
                Ok(TransactionType::SwapTokenToSol {
                    token_mint: token_changes[0].mint.clone(),
                    token_amount: token_changes[0].change.abs(),
                    sol_amount: sol_change.change,
                    dex: "Orca".to_string(),
                    fee: transaction.fee,
                })
            }
            (_, 2) if token_changes[0].change < 0.0 && token_changes[1].change > 0.0 => {
                Ok(TransactionType::SwapTokenToToken {
                    from_mint: token_changes[0].mint.clone(),
                    to_mint: token_changes[1].mint.clone(),
                    from_amount: token_changes[0].change.abs(),
                    to_amount: token_changes[1].change,
                    dex: "Orca".to_string(),
                    fee: transaction.fee,
                })
            }
            _ => {
                Ok(TransactionType::Unknown {
                    program_ids: transaction.program_ids.clone(),
                    fee: transaction.fee,
                })
            }
        }
    }

    /// Analyze SOL transfer transaction
    async fn analyze_sol_transfer(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        if let Some(sol_change) = &transaction.sol_balance_change {
            // Try to extract sender and receiver from transaction data
            let (from, to) = self.extract_transfer_accounts(tx_data).await;
            
            Ok(TransactionType::SolTransfer {
                from: from.unwrap_or_else(|| "Unknown".to_string()),
                to: to.unwrap_or_else(|| "Unknown".to_string()),
                amount: sol_change.change.abs(),
                fee: transaction.fee,
            })
        } else {
            Ok(TransactionType::Unknown {
                program_ids: transaction.program_ids.clone(),
                fee: transaction.fee,
            })
        }
    }

    /// Analyze token transfer transaction
    async fn analyze_token_transfer(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        if let Some(token_change) = transaction.token_balance_changes.first() {
            // Try to extract sender and receiver from transaction data
            let (from, to) = self.extract_transfer_accounts(tx_data).await;
            
            Ok(TransactionType::TokenTransfer {
                mint: token_change.mint.clone(),
                from: from.unwrap_or_else(|| "Unknown".to_string()),
                to: to.unwrap_or_else(|| "Unknown".to_string()),
                amount: token_change.change.abs(),
                decimals: token_change.decimals,
            })
        } else {
            Ok(TransactionType::Unknown {
                program_ids: transaction.program_ids.clone(),
                fee: transaction.fee,
            })
        }
    }

    /// Analyze system operation transaction
    async fn analyze_system_operation(
        &self,
        transaction: &Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<TransactionType, Box<dyn std::error::Error>> {
        // Try to determine the type of system operation
        let operation_type = if transaction.ata_created {
            "Create Associated Token Account".to_string()
        } else if transaction.logs.iter().any(|log| log.contains("CreateAccount")) {
            "Create Account".to_string()
        } else if transaction.logs.iter().any(|log| log.contains("Allocate")) {
            "Allocate Space".to_string()
        } else {
            "Unknown System Operation".to_string()
        };

        Ok(TransactionType::SystemOperation {
            operation_type,
            fee: transaction.fee,
        })
    }

    /// Extract sender and receiver accounts from transaction data
    async fn extract_transfer_accounts(
        &self,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> (Option<String>, Option<String>) {
        // This is a simplified implementation
        // In a real implementation, you would parse the instruction data
        // to extract the actual sender and receiver accounts
        
        if let Some(ui_transaction) = &tx_data.transaction.transaction {
            if let Some(message) = &ui_transaction.message {
                if let Some(instructions) = &message.instructions {
                    for instruction in instructions {
                        if let UiInstruction::Parsed(parsed) = instruction {
                            if let Some(info) = parsed.parsed.get("info") {
                                if let Some(source) = info.get("source").and_then(|v| v.as_str()) {
                                    if let Some(destination) = info.get("destination").and_then(|v| v.as_str()) {
                                        return (Some(source.to_string()), Some(destination.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        (None, None)
    }

    // =============================================================================
    // BALANCE CHANGE CALCULATION
    // =============================================================================

    /// Calculate SOL and token balance changes
    async fn calculate_balance_changes(
        &self,
        transaction: &mut Transaction,
        tx_data: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(meta) = &tx_data.transaction.meta {
            // Calculate SOL balance changes
            self.calculate_sol_balance_changes(transaction, meta).await?;
            
            // Calculate token balance changes
            self.calculate_token_balance_changes(transaction, meta).await?;
        }
        
        Ok(())
    }

    /// Calculate SOL balance changes
    async fn calculate_sol_balance_changes(
        &self,
        transaction: &mut Transaction,
        meta: &UiTransactionStatusMeta,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(pre_balances), Some(post_balances)) = (&meta.pre_balances, &meta.post_balances) {
            // Find our wallet's balance change (first account is usually the fee payer/main account)
            if !pre_balances.is_empty() && !post_balances.is_empty() {
                let pre_balance = pre_balances[0] as f64 / LAMPORTS_PER_SOL;
                let post_balance = post_balances[0] as f64 / LAMPORTS_PER_SOL;
                let change = post_balance - pre_balance;
                let net_change = change + transaction.fee; // Add back the fee to see net change

                transaction.sol_balance_change = Some(SolBalanceChange {
                    pre_balance,
                    post_balance,
                    change,
                    fee: transaction.fee,
                    net_change,
                });
            }
        }
        
        Ok(())
    }

    /// Calculate token balance changes
    async fn calculate_token_balance_changes(
        &self,
        transaction: &mut Transaction,
        meta: &UiTransactionStatusMeta,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use solana_transaction_status::option_serializer::OptionSerializer;
        
        let pre_token_balances = match &meta.pre_token_balances {
            OptionSerializer::Some(balances) => balances,
            _ => return Ok(()),
        };
        
        let post_token_balances = match &meta.post_token_balances {
            OptionSerializer::Some(balances) => balances,
            _ => return Ok(()),
        };
        
        let mut balance_changes: HashMap<String, TokenBalanceChange> = HashMap::new();
        
        // Process pre-balances
        for pre_balance in pre_token_balances {
            if let Some(mint) = &pre_balance.mint {
                let amount = pre_balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                    let decimals = pre_balance.ui_token_amount.decimals;
                    
                    balance_changes.insert(mint.clone(), TokenBalanceChange {
                        mint: mint.clone(),
                        decimals,
                        pre_balance: Some(amount),
                        post_balance: None,
                        change: 0.0,
                        usd_value: None,
                    });
                }
            }
            
            // Process post-balances
            for post_balance in post_token_balances {
                if let Some(mint) = &post_balance.mint {
                    let amount = post_balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                    let decimals = post_balance.ui_token_amount.decimals;
                    
                    if let Some(change) = balance_changes.get_mut(mint) {
                        change.post_balance = Some(amount);
                        change.change = amount - change.pre_balance.unwrap_or(0.0);
                    } else {
                        balance_changes.insert(mint.clone(), TokenBalanceChange {
                            mint: mint.clone(),
                            decimals,
                            pre_balance: None,
                            post_balance: Some(amount),
                            change: amount,
                            usd_value: None,
                        });
                    }
                }
            }
            
            // Calculate USD values for token changes
            for change in balance_changes.values_mut() {
                if change.change != 0.0 {
                    change.usd_value = self.calculate_token_usd_value(&change.mint, change.change).await;
                }
            }
            
            // Store only changes that are non-zero
            transaction.token_balance_changes = balance_changes
                .into_values()
                .filter(|change| change.change != 0.0)
                .collect();
        }
        
        Ok(())
    }

    /// Calculate USD value of token amount
    async fn calculate_token_usd_value(&self, mint: &str, amount: f64) -> Option<f64> {
        // Try to get token price from pool service
        if let Some(pool_service) = get_pool_service() {
            // Pool service doesn't have get_token_price method, skip for now
        }
        
        // For now, return None until we have proper price integration
        None
    }

    // =============================================================================
    // POSITION IMPACT CALCULATION
    // =============================================================================

    /// Calculate position impacts for the transaction
    async fn calculate_position_impacts(&self, transaction: &mut Transaction) -> Result<(), Box<dyn std::error::Error>> {
        let mut impacts = Vec::new();
        let mut total_usd_impact = 0.0;
        
        // Calculate impact for each token balance change
        for token_change in &transaction.token_balance_changes {
            if let Some(impact) = self.calculate_token_position_impact(token_change).await {
                if let Some(usd_impact) = impact.realized_pnl.or(impact.unrealized_pnl) {
                    total_usd_impact += usd_impact;
                }
                impacts.push(impact);
            }
        }
        
        transaction.position_impacts = impacts;
        
        if total_usd_impact != 0.0 {
            transaction.total_usd_impact = Some(total_usd_impact);
        }
        
        Ok(())
    }

    /// Calculate position impact for a single token
    async fn calculate_token_position_impact(&self, token_change: &TokenBalanceChange) -> Option<PositionImpact> {
        // This would integrate with the positions module to calculate actual P&L
        // For now, we'll create a basic structure
        
        let current_price = self.calculate_token_usd_value(&token_change.mint, 1.0).await;
        
        Some(PositionImpact {
            mint: token_change.mint.clone(),
            position_change: token_change.change,
            realized_pnl: None, // Would be calculated based on position history
            unrealized_pnl: token_change.usd_value,
            average_entry_price: None, // Would be fetched from positions
            current_price,
        })
    }
}
