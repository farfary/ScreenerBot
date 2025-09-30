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
use crate::tokens::{ decimals::lamports_to_sol, get_token_decimals, get_token_from_db };
use crate::transactions::{
    analyzer::{ self, infer_swap_router },
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
    debug_enabled: bool,
}

impl TransactionProcessor {
    /// Create new transaction processor
    pub fn new(wallet_pubkey: Pubkey) -> Self {
        Self {
            wallet_pubkey,
            fetcher: TransactionFetcher::new(),
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

        // Step 3: Extract balance changes and transfers using raw metadata
        analyzer::extract_balance_changes(&mut transaction, &tx_data).await?;

        // Step 4: Capture instruction breakdown for downstream debugging
        self.analyze_instructions(&mut transaction, &tx_data).await?;

        // Step 5: Analyze ATA operations (rent impact, ATA lifecycle)
        analyzer::analyze_ata_operations(&mut transaction, &tx_data).await?;

        // Step 6: Classify transaction type and direction
        self.analyze_transaction(&mut transaction).await?;

        // Step 7: Calculate swap P&L when classification indicates a swap
        if self.is_swap_transaction(&transaction) {
            analyzer::calculate_swap_pnl(&mut transaction, &tx_data).await?;
        }

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
    /// Analyze transaction type and direction
    async fn analyze_transaction(&self, transaction: &mut Transaction) -> Result<(), String> {
        let analysis_result = analyzer::analyze_transaction(
            transaction,
            &self.wallet_pubkey
        ).await?;

        transaction.transaction_type = analysis_result.transaction_type;
        transaction.direction = analysis_result.direction;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ANALYZE",
                &format!(
                    "Transaction {}: type={:?}, direction={:?}, confidence={:.2}",
                    transaction.signature,
                    transaction.transaction_type,
                    transaction.direction,
                    analysis_result.confidence_score
                )
            );
        }

        Ok(())
    }

    /// Analyze transaction instructions
    async fn analyze_instructions(
        &self,
        transaction: &mut Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<(), String> {
        let instruction_info = analyzer::extract_instruction_info(tx_data).await?;
        transaction.instructions = instruction_info.clone();
        transaction.instruction_info = instruction_info;
        transaction.instructions_count = transaction.instructions.len();

        Ok(())
    }

    /// Check if transaction is a swap transaction
    fn is_swap_transaction(&self, transaction: &Transaction) -> bool {
        matches!(transaction.transaction_type, TransactionType::Buy | TransactionType::Sell)
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
    // Simple placeholder implementation
    Vec::new()
}

/// Parse UI token amount with graceful fallback to raw representation
fn parse_ui_amount(amount: &crate::rpc::UiTokenAmount) -> f64 {
    // Simple placeholder implementation
    amount.ui_amount.unwrap_or(0.0)
}
