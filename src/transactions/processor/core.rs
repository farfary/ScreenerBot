//! Transaction processor core — main processing loop that decodes and analyzes transactions.
//
// Transaction processing pipeline - Core processor struct and main pipeline
//
// This module contains the TransactionProcessor struct and its main
// processing methods that coordinate the complete transaction pipeline.

use chrono::{DateTime, Utc};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::tokens::get_decimals;
use crate::transactions::{
    analyzer::TransactionAnalyzer, fetcher::TransactionFetcher, program_ids::*, types::*, utils::*,
};
use crate::utils::lamports_to_sol;

// =============================================================================
// TRANSACTION PROCESSOR
// =============================================================================

/// Core transaction processor that coordinates the processing pipeline
pub struct TransactionProcessor {
    pub(super) wallet_pubkey: Pubkey,
    pub(super) fetcher: TransactionFetcher,
    pub(super) analyzer: TransactionAnalyzer,
    pub(super) debug_enabled: bool,
    pub(super) cache_only: bool,
    pub(super) force_refresh: bool,
}

impl TransactionProcessor {
    /// Create new transaction processor
    pub fn new(wallet_pubkey: Pubkey) -> Self {
        Self {
            wallet_pubkey,
            fetcher: TransactionFetcher::new(),
            analyzer: TransactionAnalyzer::new(false),
            debug_enabled: false,
            cache_only: false,
            force_refresh: false,
        }
    }

    /// Create new transaction processor with cache options
    pub fn new_with_cache_options(
        wallet_pubkey: Pubkey,
        cache_only: bool,
        force_refresh: bool,
    ) -> Self {
        Self {
            wallet_pubkey,
            fetcher: TransactionFetcher::new(),
            analyzer: TransactionAnalyzer::new(false),
            debug_enabled: false,
            cache_only,
            force_refresh,
        }
    }

    /// Process a single transaction through the complete pipeline
    pub async fn process_transaction(&self, signature: &str) -> Result<Transaction, String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Processing transaction: {} for wallet: {}",
                    signature,
                    &self.wallet_pubkey.to_string()
                ),
            );
        }

        // Step 1: Fetch transaction details from blockchain
        let tx_data = self.fetch_transaction_data(signature).await?;

        // Step 2: Create Transaction structure from raw data snapshot
        let mut transaction = self
            .create_transaction_from_data(signature, &tx_data)
            .await?;

        // Use new analyzer to get complete analysis
        let analysis = self
            .analyzer
            .analyze_transaction(&transaction, &tx_data)
            .await?;

        // Map analyzer results to transaction fields
        self.map_analysis_to_transaction(&mut transaction, &analysis, &tx_data)
            .await?;

        let processing_duration = start_time.elapsed();
        transaction.analysis_duration_ms = Some(processing_duration.as_millis() as u64);

        if self.debug_enabled {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Processed {}: type={:?}, direction={:?}, duration={}ms",
                    signature,
                    transaction.transaction_type,
                    transaction.direction,
                    processing_duration.as_millis()
                ),
            );
        }

        // Record processing event
        crate::events::record_transaction_event(
            signature,
            "processed",
            transaction.success,
            transaction.fee_lamports,
            transaction.slot,
            None,
        )
        .await;

        // Store processed transaction in database for future retrieval
        if let Some(database) = crate::transactions::database::get_transaction_database().await {
            if let Err(e) = database.store_processed_transaction(&transaction).await {
                if self.debug_enabled {
                    logger::info(
                        LogTag::Transactions,
                        &format!("Failed to cache processed transaction: {e}"),
                    );
                }
            } else if self.debug_enabled {
                logger::info(
                    LogTag::Transactions,
                    &format!("Cached processed transaction: {signature}"),
                );
            }
        }

        Ok(transaction)
    }

    /// Process multiple transactions concurrently
    pub async fn process_transactions_batch(
        &self,
        signatures: Vec<String>,
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
