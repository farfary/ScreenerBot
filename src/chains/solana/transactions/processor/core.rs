//! Transaction processor core — main processing loop that decodes and analyzes transactions.
//
// Transaction processing pipeline - Core processor struct and main pipeline
//
// This module contains the TransactionProcessor struct and its main
// processing methods that coordinate the complete transaction pipeline.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

use crate::chains::solana::transactions::{
    analyzer::TransactionAnalyzer, fetcher::TransactionFetcher,
};
use crate::chains::Error as ChainError;
use crate::logger::{self, LogTag};
use crate::transactions::types::*;

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
    pub(super) retain_raw_json: bool,
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
            retain_raw_json: true,
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
            retain_raw_json: true,
        }
    }

    /// Builds a processor for a chain-neutral subject.
    pub fn for_subject(subject: &Subject) -> Result<Self, ChainError> {
        Ok(Self::new(
            crate::chains::solana::transactions::subject::try_to_pubkey(subject)?,
        ))
    }

    /// Builds a processor for a chain-neutral subject with cache options.
    pub fn for_subject_with_cache_options(
        subject: &Subject,
        cache_only: bool,
        force_refresh: bool,
    ) -> Result<Self, ChainError> {
        Ok(Self::new_with_cache_options(
            crate::chains::solana::transactions::subject::try_to_pubkey(subject)?,
            cache_only,
            force_refresh,
        ))
    }

    /// Create a processor for a watched (non-own) subject: the raw jsonParsed
    /// response is fetched (needed to decode) but never persisted, only the decoded
    /// analytics row is. A busy target's raw blobs would otherwise be far larger than
    /// its decoded rows and dwarf `transactions.db` -- see the wallet-watch recording
    /// policy (own wallet: full retention; watched target: decoded row only).
    pub fn new_for_watch_target(wallet_pubkey: Pubkey) -> Self {
        Self {
            wallet_pubkey,
            fetcher: TransactionFetcher::new(),
            analyzer: TransactionAnalyzer::new(false),
            debug_enabled: false,
            cache_only: false,
            force_refresh: false,
            retain_raw_json: false,
        }
    }

    /// The wallet this processor decodes for.
    pub fn subject(&self) -> crate::transactions::types::Subject {
        crate::chains::solana::transactions::subject::from_pubkey(self.wallet_pubkey)
    }

    /// Fetch, analyze and project one transaction onto this processor's subject.
    ///
    /// Records no event and writes no analysis row -- persisting the decoded result
    /// is the caller's decision, which is what lets a watched wallet be decoded
    /// without landing in our own history. Own-wallet processors cache the raw RPC
    /// response; `new_for_watch_target` retains it on the returned decoded value for
    /// classification but does not persist the JSON blob (see `extraction.rs`).
    pub async fn decode(&self, signature: &str) -> crate::chains::solana::Result<Transaction> {
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

        Ok(transaction)
    }

    /// Decode a transaction and persist it for this processor's subject.
    ///
    /// This is the own-wallet path: decode, record the processing event, store the
    /// processed row.
    pub async fn process_transaction(
        &self,
        signature: &str,
    ) -> crate::chains::solana::Result<Transaction> {
        let transaction = self.decode(signature).await?;

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
            if let Err(e) = database
                .store_processed_transaction(self.subject(), &transaction)
                .await
            {
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

            // Extract and store this subject's own asset-relative deltas -- the
            // ledger `positions::ledger::reduce_rounds` derives wallet-history positions
            // from. Own wallet only: a watched target's deltas are not our ledger
            // (see `new_for_watch_target`, which never persists raw JSON either).
            if self.retain_raw_json {
                if let Err(e) = database
                    .store_transaction_deltas(&self.wallet_pubkey.to_string(), &transaction)
                    .await
                {
                    logger::warning(
                        LogTag::Transactions,
                        &format!("Failed to store subject deltas for {signature}: {e}"),
                    );
                }
            }
        }

        Ok(transaction)
    }

    /// Process multiple transactions concurrently
    pub async fn process_transactions_batch(
        &self,
        signatures: Vec<String>,
    ) -> HashMap<String, crate::chains::solana::Result<Transaction>> {
        let mut results = HashMap::new();

        // Simple sequential processing for now
        for signature in signatures {
            let result = self.process_transaction(&signature).await;
            results.insert(signature, result);
        }

        results
    }
}
