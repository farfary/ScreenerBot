// Transaction processing pipeline for the transactions module
//
// This module handles the core transaction processing logic including
// data extraction, analysis, and classification of blockchain transactions.

use std::collections::HashMap;
use std::time::{ Duration, Instant };
use chrono::{ DateTime, Utc };
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;

use crate::logger::{ log, LogTag };
use crate::global::is_debug_transactions_enabled;
use crate::transactions::{ types::*, utils::*, fetcher::TransactionFetcher, analyzer };

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

    /// Get wallet pubkey
    pub fn get_wallet_pubkey(&self) -> Pubkey {
        self.wallet_pubkey
    }
}

// =============================================================================
// MAIN PROCESSING PIPELINE
// =============================================================================

impl TransactionProcessor {
    /// Process a single transaction through the complete pipeline
    pub async fn process_transaction(&self, signature: &str) -> Result<Transaction, String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PROCESS",
                &format!(
                    "Processing transaction: {} for wallet: {}",
                    format_signature_short(signature),
                    format_pubkey_short(&self.wallet_pubkey.to_string())
                )
            );
        }

        // Step 1: Fetch transaction details from blockchain
        let tx_data = self.fetch_transaction_data(signature).await?;

        // Step 2: Create Transaction structure from raw data
        let mut transaction = self.create_transaction_from_data(signature, tx_data).await?;

        // Step 3: Analyze transaction and classify type
        self.analyze_transaction(&mut transaction).await?;

        // Step 4: Extract balance changes and transfers
        self.extract_balance_changes(&mut transaction).await?;

        // Step 5: Analyze ATA operations if applicable
        self.analyze_ata_operations(&mut transaction).await?;

        // Step 6: Calculate P&L for swap transactions
        if self.is_swap_transaction(&transaction) {
            self.calculate_swap_pnl(&mut transaction).await?;
        }

        let processing_duration = start_time.elapsed();
        transaction.analysis_duration_ms = Some(processing_duration.as_millis() as u64);

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "PROCESS_COMPLETE",
                &format!(
                    "✅ Processed {}: type={:?}, direction={:?}, duration={}ms",
                    format_signature_short(signature),
                    transaction.transaction_type,
                    transaction.direction,
                    processing_duration.as_millis()
                )
            );
        }

        Ok(transaction)
    }

    /// Process multiple transactions concurrently
    pub async fn process_transactions_batch(
        &self,
        signatures: Vec<String>
    ) -> HashMap<String, Result<Transaction, String>> {
        let start_time = Instant::now();
        let batch_size = signatures.len();

        log(
            LogTag::Transactions,
            "BATCH_PROCESS",
            &format!("Processing batch of {} transactions", batch_size)
        );

        // Process transactions concurrently
        let tasks: Vec<_> = signatures
            .into_iter()
            .map(|signature| {
                let sig_clone = signature.clone();
                async move {
                    let result = self.process_transaction(&sig_clone).await;
                    (sig_clone, result)
                }
            })
            .collect();

        let results = futures::future::join_all(tasks).await;

        let mut batch_results = HashMap::new();
        let mut success_count = 0;

        for (signature, result) in results {
            if result.is_ok() {
                success_count += 1;
            }
            batch_results.insert(signature, result);
        }

        let duration = start_time.elapsed();

        log(
            LogTag::Transactions,
            "BATCH_COMPLETE",
            &format!(
                "Batch processing complete: {}/{} successful in {}ms (avg: {}ms/tx)",
                success_count,
                batch_size,
                duration.as_millis(),
                if batch_size > 0 {
                    duration.as_millis() / (batch_size as u128)
                } else {
                    0
                }
            )
        );

        batch_results
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
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
        self.fetcher.fetch_transaction_details(signature).await.map_err(|e| {
            if e.contains("not found") || e.contains("no longer available") {
                format!("Transaction not found: {}", signature)
            } else {
                format!("Failed to fetch transaction data: {}", e)
            }
        })
    }

    /// Create Transaction structure from raw blockchain data
    async fn create_transaction_from_data(
        &self,
        signature: &str,
        tx_data: EncodedConfirmedTransactionWithStatusMeta
    ) -> Result<Transaction, String> {
        // Extract timestamp
        let timestamp = if let Some(block_time) = tx_data.block_time {
            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
        } else {
            Utc::now()
        };

        // Determine success status
        let success = tx_data.meta.as_ref().map_or(false, |meta| meta.err.is_none());

        // Extract error message if transaction failed
        let error_message = tx_data.meta
            .as_ref()
            .and_then(|meta| meta.err.as_ref())
            .map(|err| {
                // Use structured error parsing for comprehensive error handling
                let structured_error = crate::errors::blockchain::parse_structured_solana_error(
                    &serde_json::to_value(err).unwrap_or_default(),
                    Some(signature)
                );
                format!(
                    "[{}] {}: {} (code: {})",
                    structured_error.error_type_name(),
                    structured_error.error_name,
                    structured_error.description,
                    structured_error.error_code.map_or("N/A".to_string(), |c| c.to_string())
                )
            });

        // Extract fee information
        let fee_lamports = tx_data.meta.as_ref().map(|meta| meta.fee);

        // Extract compute units consumed
        let compute_units_consumed = tx_data.meta
            .as_ref()
            .and_then(|meta| meta.compute_units_consumed);

        // Count instructions and accounts
        let instructions_count = tx_data.transaction
            .as_object()
            .and_then(|obj| obj.get("message"))
            .and_then(|msg| msg.get("instructions"))
            .and_then(|inst| inst.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        let accounts_count = tx_data.transaction
            .as_object()
            .and_then(|obj| obj.get("message"))
            .and_then(|msg| msg.get("accountKeys"))
            .and_then(|keys| keys.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        // Create transaction structure
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: Some(tx_data.slot),
            block_time: tx_data.block_time,
            timestamp,
            status: if success {
                TransactionStatus::Finalized
            } else {
                TransactionStatus::Failed(error_message.clone().unwrap_or_default())
            },
            success,
            error_message,
            fee_lamports,
            compute_units_consumed,
            instructions_count,
            accounts_count,
            // Analysis fields will be populated by subsequent steps
            ..Transaction::new(signature.to_string())
        };

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DATA_EXTRACT",
                &format!(
                    "Extracted data for {}: success={}, fee={}SOL, instructions={}, accounts={}",
                    format_signature_short(signature),
                    success,
                    fee_lamports.map_or(0.0, |f| (f as f64) / 1_000_000_000.0),
                    instructions_count,
                    accounts_count
                )
            );
        }

        Ok(transaction)
    }
}

// =============================================================================
// TRANSACTION ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Analyze transaction to determine type and direction
    async fn analyze_transaction(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Use the analyzer module for transaction classification
        let analysis_result = analyzer::analyze_transaction(
            transaction,
            &self.wallet_pubkey
        ).await?;

        // Update transaction with analysis results
        transaction.transaction_type = analysis_result.transaction_type;
        transaction.direction = analysis_result.direction;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ANALYZE",
                &format!(
                    "Analyzed {}: type={:?}, direction={:?}",
                    format_signature_short(&transaction.signature),
                    transaction.transaction_type,
                    transaction.direction
                )
            );
        }

        Ok(())
    }

    /// Extract balance changes from transaction
    async fn extract_balance_changes(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Extract SOL balance changes
        if let Some(sol_change) = self.extract_sol_balance_change(transaction).await? {
            transaction.sol_balance_change = Some(sol_change);
        }

        // Extract token balance changes
        transaction.token_balance_changes = self.extract_token_balance_changes(transaction).await?;

        // Extract token transfers
        transaction.token_transfers = self.extract_token_transfers(transaction).await?;

        if
            self.debug_enabled &&
            (transaction.sol_balance_change.is_some() ||
                !transaction.token_balance_changes.is_empty())
        {
            log(
                LogTag::Transactions,
                "BALANCE_EXTRACT",
                &format!(
                    "Extracted balances for {}: SOL={}, tokens={}",
                    format_signature_short(&transaction.signature),
                    transaction.sol_balance_change.is_some(),
                    transaction.token_balance_changes.len()
                )
            );
        }

        Ok(())
    }

    /// Extract SOL balance change for wallet
    async fn extract_sol_balance_change(
        &self,
        transaction: &Transaction
    ) -> Result<Option<SolBalanceChange>, String> {
        // This would extract SOL balance changes from transaction metadata
        // For now, return None as placeholder - will be implemented with full analysis
        Ok(None)
    }

    /// Extract token balance changes for wallet
    async fn extract_token_balance_changes(
        &self,
        transaction: &Transaction
    ) -> Result<Vec<TokenBalanceChange>, String> {
        // This would extract token balance changes from transaction metadata
        // For now, return empty vector as placeholder - will be implemented with full analysis
        Ok(Vec::new())
    }

    /// Extract token transfers from transaction
    async fn extract_token_transfers(
        &self,
        transaction: &Transaction
    ) -> Result<Vec<TokenTransfer>, String> {
        // This would extract token transfers from transaction instructions
        // For now, return empty vector as placeholder - will be implemented with full analysis
        Ok(Vec::new())
    }
}

// =============================================================================
// ATA OPERATIONS ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Analyze ATA (Associated Token Account) operations
    async fn analyze_ata_operations(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Extract ATA operations from transaction instructions
        let ata_operations = self.extract_ata_operations(transaction).await?;

        if !ata_operations.is_empty() {
            transaction.ata_operations = ata_operations;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "ATA_ANALYZE",
                    &format!(
                        "Found {} ATA operations in {}",
                        transaction.ata_operations.len(),
                        format_signature_short(&transaction.signature)
                    )
                );
            }
        }

        Ok(())
    }

    /// Extract ATA operations from transaction instructions
    async fn extract_ata_operations(
        &self,
        transaction: &Transaction
    ) -> Result<Vec<AtaOperation>, String> {
        // This would analyze transaction instructions for ATA operations
        // For now, return empty vector as placeholder - will be implemented with full instruction analysis
        Ok(Vec::new())
    }
}

// =============================================================================
// SWAP P&L CALCULATION
// =============================================================================

impl TransactionProcessor {
    /// Check if transaction is a swap transaction
    fn is_swap_transaction(&self, transaction: &Transaction) -> bool {
        matches!(transaction.transaction_type, TransactionType::Buy | TransactionType::Sell)
    }

    /// Calculate P&L for swap transactions
    async fn calculate_swap_pnl(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Extract swap information first
        if let Some(swap_info) = self.extract_swap_info(transaction).await? {
            transaction.token_swap_info = Some(swap_info);

            // Calculate P&L based on swap information
            if
                let Some(pnl_info) = self.calculate_pnl_from_swap(
                    &transaction.token_swap_info
                ).await?
            {
                transaction.swap_pnl_info = Some(pnl_info);

                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "PNL_CALC",
                        &format!(
                            "Calculated P&L for {}: net_sol={:.6}",
                            format_signature_short(&transaction.signature),
                            transaction.swap_pnl_info.as_ref().unwrap().net_sol_change
                        )
                    );
                }
            }
        }

        Ok(())
    }

    /// Extract swap information from transaction
    async fn extract_swap_info(
        &self,
        transaction: &Transaction
    ) -> Result<Option<TokenSwapInfo>, String> {
        // This would analyze transaction instructions and logs to extract swap details
        // For now, return None as placeholder - will be implemented with full swap analysis
        Ok(None)
    }

    /// Calculate P&L from swap information
    async fn calculate_pnl_from_swap(
        &self,
        swap_info: &Option<TokenSwapInfo>
    ) -> Result<Option<SwapPnLInfo>, String> {
        // This would calculate P&L based on swap details and current prices
        // For now, return None as placeholder - will be implemented with price integration
        Ok(None)
    }
}

// =============================================================================
// INSTRUCTION ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Analyze transaction instructions for detailed breakdown
    async fn analyze_instructions(&self, transaction: &mut Transaction) -> Result<(), String> {
        let instruction_info = self.extract_instruction_info(transaction).await?;
        transaction.instruction_info = instruction_info;

        if self.debug_enabled && !transaction.instruction_info.is_empty() {
            log(
                LogTag::Transactions,
                "INSTRUCTION_ANALYZE",
                &format!(
                    "Analyzed {} instructions in {}",
                    transaction.instruction_info.len(),
                    format_signature_short(&transaction.signature)
                )
            );
        }

        Ok(())
    }

    /// Extract instruction information from transaction
    async fn extract_instruction_info(
        &self,
        transaction: &Transaction
    ) -> Result<Vec<InstructionInfo>, String> {
        // This would parse transaction instructions for detailed analysis
        // For now, return empty vector as placeholder - will be implemented with full instruction parsing
        Ok(Vec::new())
    }
}

// =============================================================================
// ERROR HANDLING AND RECOVERY
// =============================================================================

impl TransactionProcessor {
    /// Handle processing errors with appropriate recovery strategies
    pub async fn handle_processing_error(
        &self,
        signature: &str,
        error: &str
    ) -> Result<(), String> {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!("Processing error for {}: {}", format_signature_short(signature), error)
        );

        // Record error event for analytics
        crate::events::record_transaction_event(
            signature,
            "process_error",
            false,
            None,
            None,
            Some(error)
        ).await;

        // Determine if error is recoverable
        if self.is_recoverable_error(error) {
            log(
                LogTag::Transactions,
                "RECOVERY",
                &format!(
                    "Error is recoverable for {}, will retry later",
                    format_signature_short(signature)
                )
            );
            // Add to deferred retries (would be handled by service layer)
        } else {
            log(
                LogTag::Transactions,
                "PERMANENT_ERROR",
                &format!("Error is permanent for {}, skipping", format_signature_short(signature))
            );
        }

        Ok(())
    }

    /// Check if error is recoverable and worth retrying
    fn is_recoverable_error(&self, error: &str) -> bool {
        // Network errors, timeouts, and temporary RPC issues are recoverable
        error.contains("timeout") ||
            error.contains("network") ||
            error.contains("connection") ||
            error.contains("rate limit") ||
            error.contains("server error")
    }
}

// =============================================================================
// PERFORMANCE MONITORING
// =============================================================================

/// Processing performance metrics
#[derive(Debug, Clone)]
pub struct ProcessingMetrics {
    pub total_processed: u64,
    pub successful_processed: u64,
    pub failed_processed: u64,
    pub average_processing_time_ms: f64,
    pub last_processing_time: Option<DateTime<Utc>>,
}

impl ProcessingMetrics {
    pub fn new() -> Self {
        Self {
            total_processed: 0,
            successful_processed: 0,
            failed_processed: 0,
            average_processing_time_ms: 0.0,
            last_processing_time: None,
        }
    }

    pub fn update_processing(&mut self, duration: Duration, success: bool) {
        self.total_processed += 1;
        self.last_processing_time = Some(Utc::now());

        if success {
            self.successful_processed += 1;
        } else {
            self.failed_processed += 1;
        }

        let duration_ms = duration.as_millis() as f64;
        self.average_processing_time_ms = if self.total_processed == 1 {
            duration_ms
        } else {
            (self.average_processing_time_ms * ((self.total_processed - 1) as f64) + duration_ms) /
                (self.total_processed as f64)
        };
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            100.0
        } else {
            ((self.successful_processed as f64) / (self.total_processed as f64)) * 100.0
        }
    }
}
