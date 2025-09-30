// Transaction processing pipeline for the transactions module
//
// This module handles the core transaction processing logic including
// data extraction, analysis, and classification of blockchain transactions.

use chrono::{ DateTime, Utc };
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::{ Duration, Instant };

use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::types::SOL_MINT;
use crate::tokens::{ decimals::lamports_to_sol, get_token_decimals, get_token_from_db };
use crate::transactions::{
    analyzer::TransactionAnalyzer,
    fetcher::TransactionFetcher,
    program_ids::*,
    types::*,
    utils::*,
};

// =============================================================================
// TRANSACTION PROCESSOR
// =============================================================================

/// Core transaction processor that coordinates the processing pipeline
pub struct TransactionProcessor {
    wallet_pubkey: Pubkey,
    fetcher: TransactionFetcher,
    analyzer: TransactionAnalyzer,
    debug_enabled: bool,
}

impl TransactionProcessor {
    /// Create new transaction processor
    pub fn new(wallet_pubkey: Pubkey) -> Self {
        Self {
            wallet_pubkey,
            fetcher: TransactionFetcher::new(),
            analyzer: TransactionAnalyzer::new(is_debug_transactions_enabled()),
            debug_enabled: is_debug_transactions_enabled(),
        }
    }

    /// Process a single transaction through the complete pipeline
    pub async fn process_transaction(&self, signature: &str) -> Result<Transaction, String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PROCESS",
                &format!(
                    "Processing transaction: {} for wallet: {}",
                    signature,
                    &self.wallet_pubkey.to_string()
                )
            );
        }

        // Step 1: Fetch transaction details from blockchain
        let tx_data = self.fetch_transaction_data(signature).await?;

        // Step 2: Create Transaction structure from raw data snapshot
        let mut transaction = self.create_transaction_from_data(signature, &tx_data).await?;

        // Use new analyzer to get complete analysis
        let analysis = self.analyzer.analyze_transaction(&transaction, &tx_data).await?;

        // Map analyzer results to transaction fields
        self.map_analysis_to_transaction(&mut transaction, &analysis).await?;

        let processing_duration = start_time.elapsed();
        transaction.analysis_duration_ms = Some(processing_duration.as_millis() as u64);

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PROCESS_COMPLETE",
                &format!(
                    "✅ Processed {}: type={:?}, direction={:?}, duration={}ms",
                    signature,
                    transaction.transaction_type,
                    transaction.direction,
                    processing_duration.as_millis()
                )
            );
        }

        // Record processing event
        crate::events::record_transaction_event(
            signature,
            "processed",
            transaction.success,
            transaction.fee_lamports,
            transaction.slot,
            None
        ).await;

        Ok(transaction)
    }

    /// Process multiple transactions concurrently
    pub async fn process_transactions_batch(
        &self,
        signatures: Vec<String>
    ) -> HashMap<String, Result<Transaction, String>> {
        let mut results = HashMap::new();

        // Simple sequential processing for now
        for signature in signatures {
            let result = self.process_transaction(&signature).await;
            results.insert(signature, result);
        }

        results
    }
}

// =============================================================================
// DATA EXTRACTION
// =============================================================================

impl TransactionProcessor {
    /// Fetch transaction data from blockchain
    async fn fetch_transaction_data(
        &self,
        signature: &str
    ) -> Result<crate::rpc::TransactionDetails, String> {
        // Delegate to fetcher
        self.fetcher.fetch_transaction_details(signature).await
    }

    /// Create Transaction structure from raw blockchain data
    async fn create_transaction_from_data(
        &self,
        signature: &str,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<Transaction, String> {
        let mut transaction = Transaction::new(signature.to_string());

        // Extract basic data from tx_data
        if let Some(meta) = &tx_data.meta {
            transaction.success = meta.err.is_none();
            transaction.fee_lamports = Some(meta.fee);
            transaction.fee_sol = lamports_to_sol(meta.fee);
        }

        if let Some(block_time) = tx_data.block_time {
            transaction.block_time = Some(block_time);
            transaction.timestamp = DateTime::from_timestamp(block_time, 0).unwrap_or_else(||
                Utc::now()
            );
        }

        transaction.slot = Some(tx_data.slot);
        transaction.status = TransactionStatus::Confirmed;
        transaction.last_updated = Utc::now();

        Ok(transaction)
    }
}

// =============================================================================
// TRANSACTION ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Map analysis results to transaction fields
    async fn map_analysis_to_transaction(
        &self,
        transaction: &mut Transaction,
        analysis: &crate::transactions::analyzer::CompleteAnalysis
    ) -> Result<(), String> {
        // Map classification results
        transaction.transaction_type = match analysis.classification.transaction_type {
            crate::transactions::analyzer::classify::ClassifiedType::Buy => TransactionType::Buy,
            crate::transactions::analyzer::classify::ClassifiedType::Sell => TransactionType::Sell,
            crate::transactions::analyzer::classify::ClassifiedType::Swap =>
                TransactionType::Unknown, // Could be Buy or Sell depending on direction
            crate::transactions::analyzer::classify::ClassifiedType::Transfer =>
                TransactionType::Transfer,
            crate::transactions::analyzer::classify::ClassifiedType::AddLiquidity =>
                TransactionType::Unknown,
            crate::transactions::analyzer::classify::ClassifiedType::RemoveLiquidity =>
                TransactionType::Unknown,
            crate::transactions::analyzer::classify::ClassifiedType::NftOperation =>
                TransactionType::Unknown,
            crate::transactions::analyzer::classify::ClassifiedType::ProgramInteraction =>
                TransactionType::Compute,
            crate::transactions::analyzer::classify::ClassifiedType::Failed =>
                TransactionType::Failed,
            crate::transactions::analyzer::classify::ClassifiedType::Unknown =>
                TransactionType::Unknown,
        };

        transaction.direction = match analysis.classification.direction {
            Some(crate::transactions::analyzer::classify::SwapDirection::SolToToken) =>
                TransactionDirection::Incoming,
            Some(crate::transactions::analyzer::classify::SwapDirection::TokenToSol) =>
                TransactionDirection::Outgoing,
            Some(crate::transactions::analyzer::classify::SwapDirection::TokenToToken) =>
                TransactionDirection::Internal,
            None => TransactionDirection::Unknown,
        };

        // Map balance changes
        transaction.sol_balance_changes = analysis.balance.sol_changes.values().cloned().collect();
        transaction.token_balance_changes = analysis.balance.token_changes
            .values()
            .flatten()
            .cloned()
            .collect();
        transaction.sol_balance_change = analysis.balance.sol_changes
            .values()
            .map(|change| change.change)
            .sum();

        // Map ATA analysis
        transaction.ata_analysis = Some(crate::transactions::types::AtaAnalysis {
            total_ata_creations: analysis.ata.rent_summary.accounts_created,
            total_ata_closures: analysis.ata.rent_summary.accounts_closed,
            token_ata_creations: analysis.ata.account_lifecycle.created_accounts
                .iter()
                .filter(
                    |acc| acc.mint.is_some() && acc.mint.as_ref().unwrap() != &SOL_MINT.to_string()
                )
                .count() as u32,
            token_ata_closures: 0, // ClosedAccount doesn't have mint info, calculate from operations instead
            wsol_ata_creations: analysis.ata.account_lifecycle.created_accounts
                .iter()
                .filter(|acc| acc.mint.as_ref() == Some(&SOL_MINT.to_string()))
                .count() as u32,
            wsol_ata_closures: 0, // ClosedAccount doesn't have mint info, calculate from operations instead
            total_rent_spent: analysis.ata.rent_summary.total_rent_paid,
            total_rent_recovered: analysis.ata.rent_summary.total_rent_recovered,
            net_rent_impact: analysis.ata.rent_summary.net_rent_cost,
            token_rent_spent: analysis.ata.account_lifecycle.created_accounts
                .iter()
                .filter(
                    |acc| acc.mint.is_some() && acc.mint.as_ref().unwrap() != &SOL_MINT.to_string()
                )
                .map(|acc| acc.rent_paid)
                .sum(),
            token_rent_recovered: analysis.ata.account_lifecycle.closed_accounts
                .iter()
                .map(|acc| acc.rent_recovered)
                .sum::<f64>() * 0.5, // Estimate half for tokens (since we can't distinguish)
            token_net_rent_impact: {
                let spent: f64 = analysis.ata.account_lifecycle.created_accounts
                    .iter()
                    .filter(
                        |acc|
                            acc.mint.is_some() &&
                            acc.mint.as_ref().unwrap() != &SOL_MINT.to_string()
                    )
                    .map(|acc| acc.rent_paid)
                    .sum();
                let recovered: f64 =
                    analysis.ata.account_lifecycle.closed_accounts
                        .iter()
                        .map(|acc| acc.rent_recovered)
                        .sum::<f64>() * 0.5; // Estimate half for tokens
                recovered - spent
            },
            wsol_rent_spent: analysis.ata.account_lifecycle.created_accounts
                .iter()
                .filter(|acc| acc.mint.as_ref() == Some(&SOL_MINT.to_string()))
                .map(|acc| acc.rent_paid)
                .sum(),
            wsol_rent_recovered: analysis.ata.account_lifecycle.closed_accounts
                .iter()
                .map(|acc| acc.rent_recovered)
                .sum::<f64>() * 0.5, // Estimate half for WSOL (since we can't distinguish)
            wsol_net_rent_impact: {
                let spent: f64 = analysis.ata.account_lifecycle.created_accounts
                    .iter()
                    .filter(|acc| acc.mint.as_ref() == Some(&SOL_MINT.to_string()))
                    .map(|acc| acc.rent_paid)
                    .sum();
                let recovered: f64 =
                    analysis.ata.account_lifecycle.closed_accounts
                        .iter()
                        .map(|acc| acc.rent_recovered)
                        .sum::<f64>() * 0.5; // Estimate half for WSOL
                recovered - spent
            },
            detected_operations: analysis.ata.ata_operations
                .iter()
                .map(|op| {
                    crate::transactions::types::AtaOperation {
                        operation_type: match op.operation_type {
                            crate::transactions::analyzer::ata::AtaOperationType::Create =>
                                crate::transactions::types::AtaOperationType::Creation,
                            crate::transactions::analyzer::ata::AtaOperationType::Initialize =>
                                crate::transactions::types::AtaOperationType::Creation,
                            crate::transactions::analyzer::ata::AtaOperationType::Close =>
                                crate::transactions::types::AtaOperationType::Closure,
                            crate::transactions::analyzer::ata::AtaOperationType::Transfer =>
                                crate::transactions::types::AtaOperationType::Creation, // Map as creation for simplicity
                            crate::transactions::analyzer::ata::AtaOperationType::SetAuthority =>
                                crate::transactions::types::AtaOperationType::Creation, // Map as creation for simplicity
                            crate::transactions::analyzer::ata::AtaOperationType::CreateNative =>
                                crate::transactions::types::AtaOperationType::Creation,
                        },
                        account_address: op.account_address.clone(),
                        token_mint: op.mint.clone().unwrap_or_default(),
                        rent_amount: op.rent_amount,
                        is_wsol: op.mint.as_ref() == Some(&SOL_MINT.to_string()),
                        mint: op.mint.clone().unwrap_or_default(),
                        rent_cost_sol: Some(op.rent_amount),
                    }
                })
                .collect(),
        });

        // Map token swap info if available from P&L analysis
        // TODO: Map P&L analysis - complex mapping needed
        // For now, skip complex P&L mapping until type alignment is complete

        Ok(())
    }

    /// Calculate total tip amount from system transfers to MEV addresses
    fn calculate_tip_amount(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> f64 {
        // Simple placeholder implementation
        0.0
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Extract account keys from a transaction message (legacy and v0 support)
fn account_keys_from_message(message: &Value) -> Vec<String> {
    // Legacy format: array of strings
    if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
        return array
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
    }

    // v0 format: object with staticAccountKeys and loadedAddresses
    if let Some(obj) = message.get("accountKeys").and_then(|v| v.as_object()) {
        let mut keys = Vec::new();

        // Static account keys
        if let Some(static_keys) = obj.get("staticAccountKeys").and_then(|v| v.as_array()) {
            keys.extend(
                static_keys
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
            );
        }

        // Loaded addresses
        if let Some(loaded) = obj.get("loadedAddresses").and_then(|v| v.as_object()) {
            if let Some(writable) = loaded.get("writable").and_then(|v| v.as_array()) {
                keys.extend(
                    writable
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                );
            }
            if let Some(readonly) = loaded.get("readonly").and_then(|v| v.as_array()) {
                keys.extend(
                    readonly
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                );
            }
        }

        return keys;
    }

    Vec::new()
}

/// Parse UI token amount with graceful fallback to raw representation
fn parse_ui_amount(amount: &crate::rpc::UiTokenAmount) -> f64 {
    // Try ui_amount first
    if let Some(ui_amount) = amount.ui_amount {
        return ui_amount;
    }

    // Fallback to amount string parsing with decimals
    if let Ok(raw_amount) = amount.amount.parse::<u64>() {
        return (raw_amount as f64) / (10f64).powi(amount.decimals as i32);
    }

    0.0
}
