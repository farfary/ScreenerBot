use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::transactions::TransactionsManager;
use crate::transactions_types::*;
use chrono::{ DateTime, Utc };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tabled::{ settings::{ object::Rows, Alignment, Modify, Style }, Table, Tabled };
use tokio::time::Duration;

impl TransactionsManager {
    /// Update transaction status in database when status changes
    pub async fn update_transaction_status_in_db(
        &self,
        signature: &str,
        status: &TransactionStatus,
        success: bool,
        error_message: Option<&str>
    ) -> Result<(), String> {
        if let Some(ref db) = self.transaction_database {
            let status_str = match status {
                TransactionStatus::Pending => "Pending",
                TransactionStatus::Confirmed => "Confirmed",
                TransactionStatus::Finalized => "Finalized",
                TransactionStatus::Failed(ref msg) => "Failed",
            };

            db.update_transaction_status(signature, status_str, success, error_message).await?;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "STATUS_UPDATE",
                    &format!(
                        "Updated transaction {} status to {} in database",
                        &signature[..8],
                        status_str
                    )
                );
            }
        }
        Ok(())
    }

    /// Process transaction directly from blockchain (bypassing cache)
    /// This is similar to process_transaction but forces fresh fetch from RPC
    pub async fn process_transaction_direct(
        &mut self,
        signature: &str
    ) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DIRECT",
                &format!("Processing transaction directly from blockchain: {}", &signature[..8])
            );
        }

        // Create new transaction struct
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: None,
            block_time: None,
            timestamp: Utc::now(),
            status: TransactionStatus::Confirmed,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: false,
            error_message: None,
            fee_sol: 0.0,
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: None,
            log_messages: Vec::new(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            position_impact: None,
            profit_calculation: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cached_analysis: None,
        };

        // Fetch fresh transaction data from blockchain
        let raw_blockchain_transaction_data = self.get_or_fetch_transaction_data(
            &transaction.signature
        ).await?;
        transaction.raw_transaction_data = Some(raw_blockchain_transaction_data);

        // Perform comprehensive analysis
        self.analyze_transaction(&mut transaction).await?;
        // Defensive: if raw data has block_time and no error, treat as finalized
        if transaction.block_time.is_some() && transaction.success {
            transaction.status = TransactionStatus::Finalized;

            // Update status in database
            if
                let Err(e) = self.update_transaction_status_in_db(
                    &transaction.signature,
                    &transaction.status,
                    transaction.success,
                    transaction.error_message.as_deref()
                ).await
            {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to update transaction status in DB: {}", e)
                );
            }
        }

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if
            matches!(transaction.status, TransactionStatus::Finalized) &&
            transaction.raw_transaction_data.is_some()
        {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

        // Cache the processed transaction
        self.cache_transaction(&transaction).await?;

        // Update known signatures
        self.known_signatures.insert(signature.to_string());

        Ok(transaction)
    }

    /// Get transaction data from cache first, fetch from blockchain only if needed
    pub async fn get_or_fetch_transaction_data(
        &self,
        signature: &str
    ) -> Result<serde_json::Value, String> {
        // Try database first
        if let Some(db) = &self.transaction_database {
            if let Some(raw) = db.get_raw_transaction(signature).await? {
                if let Some(json_str) = raw.raw_transaction_data {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "DB_HIT", &format!("Raw {}", &signature[..8]));
                    }
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        return Ok(val);
                    }
                }
            }
        }
        if self.debug_enabled {
            log(LogTag::Transactions, "DB_MISS", &format!("RPC fetch {}", &signature[..8]));
        }

        let rpc_client = get_rpc_client();
        let tx_details = rpc_client
            .get_transaction_details(signature).await
            .map_err(|e| format!("RPC error: {}", e))?;

        // Convert TransactionDetails to JSON for storage
        let raw_blockchain_transaction_data = serde_json
            ::to_value(tx_details)
            .map_err(|e| format!("Failed to serialize transaction data: {}", e))?;

        Ok(raw_blockchain_transaction_data)
    }

    /// Process transaction from encoded data (used for batch processing)
    /// This is optimized for batch processing where we already have the transaction data
    async fn process_transaction_from_encoded_data(
        &mut self,
        signature: &str,
        encoded_tx: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta
    ) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "BATCH_PROCESS",
                &format!("Processing transaction from batch data: {}", &signature[..8])
            );
        }

        // Create new transaction struct
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: None,
            block_time: None,
            timestamp: Utc::now(),
            status: TransactionStatus::Confirmed,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: false,
            error_message: None,
            fee_sol: 0.0,
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: None,
            log_messages: Vec::new(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            position_impact: None,
            profit_calculation: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cached_analysis: None,
        };

        // Convert encoded transaction to raw data format
        let raw_blockchain_transaction_data = serde_json
            ::to_value(&encoded_tx)
            .map_err(|e| format!("Failed to serialize encoded transaction data: {}", e))?;

        transaction.raw_transaction_data = Some(raw_blockchain_transaction_data);

        // Perform comprehensive analysis
        self.analyze_transaction(&mut transaction).await?;
        // Defensive: if raw data has block_time and no error, treat as finalized
        if transaction.block_time.is_some() && transaction.success {
            transaction.status = TransactionStatus::Finalized;

            // Update status in database
            if
                let Err(e) = self.update_transaction_status_in_db(
                    &transaction.signature,
                    &transaction.status,
                    transaction.success,
                    transaction.error_message.as_deref()
                ).await
            {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to update transaction status in DB: {}", e)
                );
            }
        }

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if
            matches!(transaction.status, TransactionStatus::Finalized) &&
            transaction.raw_transaction_data.is_some()
        {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

        // Cache the processed transaction
        self.cache_transaction(&transaction).await?;

        // Update known signatures
        self.known_signatures.insert(signature.to_string());

        Ok(transaction)
    }

    /// Fetch and analyze ALL wallet transactions from blockchain (unlimited)
    /// This method fetches comprehensive transaction history directly from the blockchain
    /// and processes each transaction with full analysis, bypassing the cache
    pub async fn fetch_all_wallet_transactions(&mut self) -> Result<Vec<Transaction>, String> {
        log(
            LogTag::Transactions,
            "INFO",
            &format!(
                "Starting comprehensive blockchain fetch for wallet {} (no limit)",
                self.wallet_pubkey
            )
        );

        // Initialize known signatures from cache so we can skip existing ones
        if let Err(e) = self.initialize_known_signatures().await {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to initialize known signatures: {}", e)
            );
        } else if self.debug_enabled {
            log(
                LogTag::Transactions,
                "INIT",
                &format!(
                    "Cache has {} transactions; will skip these during fetch",
                    self.known_signatures.len()
                )
            );
        }

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = RPC_BATCH_SIZE; // Fetch in batches to avoid rate limits
        let mut total_fetched = 0;
        let mut total_skipped_cached = 0usize;

        log(
            LogTag::Transactions,
            "FETCH",
            "Fetching ALL transaction signatures from blockchain..."
        );

        // Fetch transaction signatures in batches until exhausted
        loop {
            let signatures = match
                rpc_client.get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    batch_size,
                    before_signature.as_deref()
                ).await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!("Failed to fetch signatures batch: {}", e)
                    );
                    break;
                }
            };

            if signatures.is_empty() {
                log(
                    LogTag::Transactions,
                    "INFO",
                    "No more signatures available - completed full fetch"
                );
                break;
            }

            let batch_count = signatures.len();
            total_fetched += batch_count;

            // Build list of signatures we don't already have cached
            let mut signatures_to_process: Vec<String> = Vec::new();
            for s in &signatures {
                if self.known_signatures.contains(&s.signature) {
                    total_skipped_cached += 1;
                } else {
                    signatures_to_process.push(s.signature.clone());
                }
            }

            log(
                LogTag::Transactions,
                "FETCH",
                &format!(
                    "Fetched batch of {} signatures (total seen: {}), to process (not cached): {} | skipped cached: {}",
                    batch_count,
                    total_fetched,
                    signatures_to_process.len(),
                    total_skipped_cached
                )
            );

            for chunk in signatures_to_process.chunks(TRANSACTION_DATA_BATCH_SIZE) {
                let chunk_size = chunk.len();
                log(
                    LogTag::Transactions,
                    "BATCH",
                    &format!("Processing batch of {} transactions using batch RPC call", chunk_size)
                );

                // Use batch RPC call to fetch all transactions in this chunk at once
                match rpc_client.batch_get_transaction_details_premium_rpc(chunk).await {
                    Ok(batch_results) => {
                        log(
                            LogTag::Transactions,
                            "BATCH",
                            &format!(
                                "✅ Batch fetched {}/{} transactions successfully",
                                batch_results.len(),
                                chunk_size
                            )
                        );

                        // Process each transaction from the batch results
                        for (signature, encoded_tx) in batch_results {
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "BATCH",
                                    &format!(
                                        "Processing transaction from batch: {}",
                                        &signature[..8]
                                    )
                                );
                            }

                            match
                                self.process_transaction_from_encoded_data(
                                    &signature,
                                    encoded_tx
                                ).await
                            {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "BATCH",
                                            &format!(
                                                "✅ Processed transaction: {}",
                                                &signature[..8]
                                            )
                                        );
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Failed to batch fetch {} transactions: {}", chunk_size, e)
                        );

                        // Fallback to individual processing if batch fails
                        log(
                            LogTag::Transactions,
                            "FALLBACK",
                            "Falling back to individual transaction processing"
                        );
                        for signature in chunk {
                            match self.process_transaction_direct(&signature).await {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                }

                // Shorter delay between transaction batches
                if chunk_size == TRANSACTION_DATA_BATCH_SIZE {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            // Set the before signature for the next batch
            if let Some(last_sig) = signatures.last() {
                before_signature = Some(last_sig.signature.clone());
            } else {
                // Empty signatures list - should not happen but handle safely
                log(
                    LogTag::Transactions,
                    "WARN",
                    "Empty signatures list in startup discovery batch"
                );
                break;
            }

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(500)).await; // Batch processing delay
        }

        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "Completed comprehensive fetch: {} new transactions processed | {} cached skipped",
                all_transactions.len(),
                total_skipped_cached
            )
        );

        Ok(all_transactions)
    }

    /// Fetch and analyze limited number of wallet transactions from blockchain (for testing)
    /// This method fetches a specific number of transactions for testing purposes
    pub async fn fetch_limited_wallet_transactions(
        &mut self,
        max_count: usize
    ) -> Result<Vec<Transaction>, String> {
        log(
            LogTag::Transactions,
            "INFO",
            &format!(
                "Starting limited blockchain fetch for wallet {} (max {} transactions)",
                self.wallet_pubkey,
                max_count
            )
        );

        // Initialize known signatures from cache so we can skip existing ones
        if let Err(e) = self.initialize_known_signatures().await {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to initialize known signatures: {}", e)
            );
        } else if self.debug_enabled {
            log(
                LogTag::Transactions,
                "INIT",
                &format!(
                    "Cache has {} transactions; will skip these during limited fetch",
                    self.known_signatures.len()
                )
            );
        }

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = RPC_BATCH_SIZE;
        let mut total_fetched = 0; // total signatures seen
        let mut total_skipped_cached = 0usize;
        let mut total_to_process = 0usize; // count of new (not cached) we attempted to process

        log(LogTag::Transactions, "FETCH", "Fetching transaction signatures from blockchain...");

        // Fetch transaction signatures in batches
        while total_to_process < max_count {
            let signatures = match
                rpc_client.get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    batch_size,
                    before_signature.as_deref()
                ).await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!("Failed to fetch signatures batch: {}", e)
                    );
                    break;
                }
            };

            if signatures.is_empty() {
                log(LogTag::Transactions, "INFO", "No more signatures available");
                break;
            }

            let batch_count = signatures.len();
            total_fetched += batch_count;

            // Filter out cached signatures; only process unknown ones, but cap by remaining_needed
            let mut signatures_to_process: Vec<String> = Vec::new();
            for s in &signatures {
                if self.known_signatures.contains(&s.signature) {
                    total_skipped_cached += 1;
                } else if signatures_to_process.len() + total_to_process < max_count {
                    signatures_to_process.push(s.signature.clone());
                }
            }

            total_to_process += signatures_to_process.len();

            log(
                LogTag::Transactions,
                "FETCH",
                &format!(
                    "Fetched batch of {} signatures (seen total: {}), to process (not cached): {} (goal {}), skipped cached so far: {}",
                    batch_count,
                    total_fetched,
                    signatures_to_process.len(),
                    max_count,
                    total_skipped_cached
                )
            );

            for chunk in signatures_to_process.chunks(TRANSACTION_DATA_BATCH_SIZE) {
                let chunk_size = chunk.len();
                log(
                    LogTag::Transactions,
                    "BATCH",
                    &format!("Processing batch of {} transactions using batch RPC call", chunk_size)
                );

                // Use batch RPC call to fetch all transactions in this chunk at once
                match rpc_client.batch_get_transaction_details_premium_rpc(chunk).await {
                    Ok(batch_results) => {
                        log(
                            LogTag::Transactions,
                            "BATCH",
                            &format!(
                                "✅ Batch fetched {}/{} transactions successfully",
                                batch_results.len(),
                                chunk_size
                            )
                        );

                        // Process each transaction from the batch results
                        for (signature, encoded_tx) in batch_results {
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "BATCH",
                                    &format!(
                                        "Processing transaction from batch: {}",
                                        &signature[..8]
                                    )
                                );
                            }

                            match
                                self.process_transaction_from_encoded_data(
                                    &signature,
                                    encoded_tx
                                ).await
                            {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "BATCH",
                                            &format!(
                                                "✅ Processed transaction: {}",
                                                &signature[..8]
                                            )
                                        );
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Failed to batch fetch {} transactions: {}", chunk_size, e)
                        );

                        // Fallback to individual processing if batch fails
                        log(
                            LogTag::Transactions,
                            "FALLBACK",
                            "Falling back to individual transaction processing"
                        );
                        for signature in chunk {
                            match self.process_transaction_direct(&signature).await {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to process transaction {}: {}",
                                            &signature[..8],
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }
                }

                // Shorter delay between transaction batches
                if chunk_size == TRANSACTION_DATA_BATCH_SIZE {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            // Set the before signature for the next batch
            if let Some(last_sig) = signatures.last() {
                before_signature = Some(last_sig.signature.clone());
            } else {
                // Empty signatures list - should not happen but handle safely
                log(LogTag::Transactions, "WARN", "Empty signatures list in gap backfill batch");
                break;
            }

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "Completed limited fetch: {} new transactions processed | {} cached skipped",
                all_transactions.len(),
                total_skipped_cached
            )
        );

        Ok(all_transactions)
    }

    /// Display comprehensive swap analysis table with shortened signatures for better readability
    /// Signatures are displayed as first8...last4 format (e.g., "2iPhXfdK...oGiM")
    /// Full signatures are still logged and searchable in transaction data
    pub fn display_swap_analysis_table_full_signatures(&self, swaps: &[SwapPnLInfo]) {
        if swaps.is_empty() {
            log(LogTag::Transactions, "INFO", "No swap transactions found");
            return;
        }

        log(
            LogTag::Transactions,
            "TABLE",
            "=== COMPREHENSIVE SWAP ANALYSIS WITH SHORTENED SIGNATURES ==="
        );

        // Convert swaps to display rows with full signatures
        let mut display_rows: Vec<SwapDisplayRow> = Vec::new();
        let mut total_fees = 0.0;
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut total_sol_spent = 0.0;
        let mut total_sol_received = 0.0;

        for swap in swaps {
            let slot_str = match swap.slot {
                Some(slot) => format!("{}", slot),
                None => "Unknown".to_string(),
            };

            // Use shortened signature for better table readability
            // Full signature is still available in logs and for searching

            // Apply intuitive sign conventions for final display:
            // SOL: negative for outflow (spent), positive for inflow (received)
            // Token: negative for outflow (sold), positive for inflow (bought)
            let (display_sol_amount, display_token_amount) = if swap.swap_type == "Buy" {
                // Buy: SOL spent (negative), tokens received (positive)
                (-swap.sol_amount, swap.token_amount.abs())
            } else {
                // Sell: SOL received (positive), tokens sold (negative)
                (swap.sol_amount, -swap.token_amount.abs())
            };

            // Color coding for better readability
            let type_display = if swap.swap_type == "Buy" {
                "🟢 Buy".to_string() // Green for buy
            } else {
                "🔴 Sell".to_string() // Red for sell
            };

            // Format SOL amount with colored sign
            let sol_formatted = if display_sol_amount >= 0.0 {
                format!("+{:.6}", display_sol_amount)
            } else {
                format!("{:.6}", display_sol_amount)
            };

            // Format token amount with colored sign
            let token_formatted = if display_token_amount >= 0.0 {
                format!("+{:.2}", display_token_amount)
            } else {
                format!("{:.2}", display_token_amount)
            };

            let effective_sol = if swap.swap_type == "Buy" {
                swap.effective_sol_spent
            } else {
                swap.effective_sol_received
            };

            let effective_price_str = if swap.token_amount.abs() > 0.0 && effective_sol > 0.0 {
                let price = effective_sol / swap.token_amount.abs();
                format!("{:.9}", price)
            } else {
                "N/A".to_string()
            };

            // Shorten signature for table display (keeps full signatures in logs)
            let shortened_signature = if swap.signature.len() <= 16 {
                swap.signature.clone()
            } else {
                crate::utils::safe_format_signature(&swap.signature)
            };

            display_rows.push(SwapDisplayRow {
                date: swap.timestamp.format("%m-%d").to_string(),
                time: swap.timestamp.format("%H:%M").to_string(),
                signature: shortened_signature,
                slot: slot_str,
                swap_type: type_display,
                token: swap.token_symbol[..(15).min(swap.token_symbol.len())].to_string(),
                sol_amount: sol_formatted,
                token_amount: token_formatted,
                price: format!("{:.9}", swap.calculated_price_sol),
                effective_sol: format!("{:.6}", effective_sol),
                effective_price: effective_price_str,
                ata_rents: format!("{:.6}", swap.ata_rents),
                router: swap.router[..(12).min(swap.router.len())].to_string(),
                fee: format!("{:.6}", swap.fee_sol),
                status: swap.status.clone(),
            });

            total_fees += swap.fee_sol;
            if swap.swap_type == "Buy" {
                buy_count += 1;
                total_sol_spent += swap.sol_amount;
            } else {
                sell_count += 1;
                total_sol_received += swap.sol_amount;
            }
        }

        // Create and display the table
        let table_string = Table::new(display_rows)
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();

        // Print the entire table directly to console
        println!("{}", table_string);

        // Print summary
        println!(
            "📊 SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
            buy_count,
            total_sol_spent,
            sell_count,
            total_sol_received,
            total_fees,
            total_sol_received - total_sol_spent - total_fees
        );
        println!("=== END ANALYSIS ===");

        log(
            LogTag::Transactions,
            "TABLE",
            &format!(
                "📊 SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
                buy_count,
                total_sol_spent,
                sell_count,
                total_sol_received,
                total_fees,
                total_sol_received - total_sol_spent - total_fees
            )
        );
        log(LogTag::Transactions, "TABLE", "=== END ANALYSIS ===");
    }

    /// Display comprehensive swap analysis table with proper sign conventions
    pub fn display_swap_analysis_table(&self, swaps: &[SwapPnLInfo]) {
        if swaps.is_empty() {
            log(LogTag::Transactions, "INFO", "No swap transactions found");
            return;
        }

        log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE SWAP ANALYSIS ===");

        // Convert swaps to display rows
        let mut display_rows: Vec<SwapDisplayRow> = Vec::new();
        let mut total_fees = 0.0;
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut total_sol_spent = 0.0;
        let mut total_sol_received = 0.0;

        for swap in swaps {
            let slot_str = match swap.slot {
                Some(slot) => format!("{}", slot),
                None => "Unknown".to_string(),
            };

            // Apply intuitive sign conventions for final display:
            // SOL: negative for outflow (spent), positive for inflow (received)
            // Token: negative for outflow (sold), positive for inflow (bought)
            let (display_sol_amount, display_token_amount) = if swap.swap_type == "Buy" {
                // Buy: SOL spent (negative), tokens received (positive)
                (-swap.sol_amount, swap.token_amount.abs())
            } else {
                // Sell: SOL received (positive), tokens sold (negative)
                (swap.sol_amount, -swap.token_amount.abs())
            };

            // Color coding for better readability
            let type_display = if swap.swap_type == "Buy" {
                "🟢 Buy".to_string() // Green for buy
            } else {
                "🔴 Sell".to_string() // Red for sell
            };

            // Format SOL amount with colored sign
            let sol_formatted = if display_sol_amount >= 0.0 {
                format!("+{:.6}", display_sol_amount)
            } else {
                format!("{:.6}", display_sol_amount)
            };

            // Format token amount with colored sign
            let token_formatted = if display_token_amount >= 0.0 {
                format!("+{:.2}", display_token_amount)
            } else {
                format!("{:.2}", display_token_amount)
            };

            let effective_sol = if swap.swap_type == "Buy" {
                swap.effective_sol_spent
            } else {
                swap.effective_sol_received
            };

            let effective_price_str = if swap.token_amount.abs() > 0.0 && effective_sol > 0.0 {
                let price = effective_sol / swap.token_amount.abs();
                format!("{:.9}", price)
            } else {
                "N/A".to_string()
            };

            // Shorten signature for table display (keeps full signatures in logs)
            let shortened_signature = if swap.signature.len() <= 16 {
                swap.signature.clone()
            } else {
                crate::utils::safe_format_signature(&swap.signature)
            };

            display_rows.push(SwapDisplayRow {
                date: swap.timestamp.format("%m-%d").to_string(),
                time: swap.timestamp.format("%H:%M").to_string(),
                signature: shortened_signature,
                slot: slot_str,
                swap_type: type_display,
                token: swap.token_symbol[..(15).min(swap.token_symbol.len())].to_string(),
                sol_amount: sol_formatted,
                token_amount: token_formatted,
                price: format!("{:.9}", swap.calculated_price_sol),
                effective_sol: format!("{:.6}", effective_sol),
                effective_price: effective_price_str,
                ata_rents: format!("{:.6}", swap.ata_rents),
                router: swap.router[..(12).min(swap.router.len())].to_string(),
                fee: format!("{:.6}", swap.fee_sol),
                status: swap.status.clone(),
            });

            total_fees += swap.fee_sol;
            if swap.swap_type == "Buy" {
                buy_count += 1;
                total_sol_spent += swap.sol_amount;
            } else {
                sell_count += 1;
                total_sol_received += swap.sol_amount;
            }
        }

        // Create and display the table
        let table_string = Table::new(display_rows)
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();

        // Print the entire table directly to console
        println!("{}", table_string);

        // Print summary
        println!(
            "📊 SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
            buy_count,
            total_sol_spent,
            sell_count,
            total_sol_received,
            total_fees,
            total_sol_received - total_sol_spent - total_fees
        );
        println!("=== END ANALYSIS ===");

        log(
            LogTag::Transactions,
            "TABLE",
            &format!(
                "📊 SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
                buy_count,
                total_sol_spent,
                sell_count,
                total_sol_received,
                total_fees,
                total_sol_received - total_sol_spent - total_fees
            )
        );
        log(LogTag::Transactions, "TABLE", "=== END ANALYSIS ===");
    }

    /// Display comprehensive position analysis table
    pub fn display_position_analysis_table(&self, positions: &[PositionAnalysis]) {
        if positions.is_empty() {
            log(LogTag::Transactions, "INFO", "No positions found");
            return;
        }

        log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE POSITION ANALYSIS ===");

        // Print header
        println!("=== COMPREHENSIVE POSITION ANALYSIS ===");

        // Convert positions to display rows
        let mut display_rows: Vec<PositionDisplayRow> = Vec::new();
        let mut total_invested = 0.0;
        let mut total_received = 0.0;
        let mut total_fees = 0.0;
        let mut total_pnl = 0.0;
        let mut open_positions = 0;
        let mut closed_positions = 0;

        for position in positions {
            let status_display = match position.status {
                PositionStatus::Open => "🟢 Open".to_string(),
                PositionStatus::Closed => "🔴 Closed".to_string(),
                PositionStatus::PartiallyReduced => "🟡 Partial".to_string(),
                PositionStatus::Oversold => "🟣 Oversold".to_string(),
            };

            // Format SOL amounts with proper signs for intuitive display
            // Invested: negative (outflow), Received: positive (inflow)
            let sol_in_display = if position.total_sol_invested > 0.0 {
                format!("-{:.3}", position.total_sol_invested)
            } else {
                format!("{:.3}", position.total_sol_invested)
            };

            let sol_out_display = if position.total_sol_received > 0.0 {
                format!("+{:.3}", position.total_sol_received)
            } else {
                format!("{:.3}", position.total_sol_received)
            };

            // Format PnL
            let pnl_display = if position.total_pnl > 0.0 {
                format!("+{:.3}", position.total_pnl)
            } else if position.total_pnl < 0.0 {
                format!("{:.3}", position.total_pnl)
            } else {
                format!("{:.3}", position.total_pnl)
            };

            // Format token amounts
            let bought_display = format!("{}", position.buy_count);
            let sold_display = if position.total_tokens_sold > 0.0 {
                format!("{:.2}", position.total_tokens_sold)
            } else {
                "0.00".to_string()
            };
            let remaining_display = if position.remaining_tokens > 0.0 {
                format!("{:.2}", position.remaining_tokens)
            } else {
                "0.00".to_string()
            };

            // Format duration - fix negative duration issue
            let duration_display = if position.duration_hours > 0.0 {
                if position.duration_hours > 24.0 {
                    format!("{:.1}d", position.duration_hours / 24.0)
                } else {
                    format!("{:.1}h", position.duration_hours)
                }
            } else {
                "0.0h".to_string()
            };

            display_rows.push(PositionDisplayRow {
                token: position.token_symbol[..(15).min(position.token_symbol.len())].to_string(),
                status: status_display,
                opened: if let Some(timestamp) = position.first_buy_timestamp {
                    format!("{} {}", timestamp.format("%m-%d"), timestamp.format("%H:%M"))
                } else {
                    "N/A".to_string()
                },
                closed: match position.status {
                    PositionStatus::Closed | PositionStatus::Oversold => {
                        // For closed positions, use the last activity timestamp (when position was actually closed)
                        if let Some(timestamp) = position.last_activity_timestamp {
                            format!("{} {}", timestamp.format("%m-%d"), timestamp.format("%H:%M"))
                        } else {
                            "N/A".to_string()
                        }
                    }
                    PositionStatus::Open | PositionStatus::PartiallyReduced => "Open".to_string(),
                },
                buys: bought_display,
                sold: sold_display,
                remaining: remaining_display,
                sol_in: sol_in_display,
                sol_out: sol_out_display,
                net_pnl: pnl_display,
                avg_price: format!("{:.9}", position.average_buy_price),
                fees: format!("{:.6}", position.total_fees), // Only trading fees, not ATA rents
                duration: duration_display,
            });

            // Update totals
            total_invested += position.total_sol_invested;
            total_received += position.total_sol_received;
            total_fees += position.total_fees + position.total_ata_rents;
            total_pnl += position.total_pnl;

            match position.status {
                PositionStatus::Open | PositionStatus::PartiallyReduced => {
                    open_positions += 1;
                }
                PositionStatus::Closed | PositionStatus::Oversold => {
                    closed_positions += 1;
                }
            }
        }

        // Create and display the table
        let table_string = Table::new(display_rows)
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();

        // Print the entire table directly to console
        println!("{}", table_string);

        let net_pnl_display = if total_pnl > 0.0 {
            format!("+{:.3}", total_pnl)
        } else if total_pnl < 0.0 {
            format!("{:.3}", total_pnl)
        } else {
            format!("{:.3}", total_pnl)
        };

        // Print summary
        println!(
            "📊 SUMMARY: {} Open, {} Closed | Invested: {:.3} SOL | Received: {:.3} SOL | Fees: {:.3} SOL | Net PnL: {}",
            open_positions,
            closed_positions,
            total_invested,
            total_received,
            total_fees,
            net_pnl_display
        );
        println!("=== END POSITION ANALYSIS ===");

        log(
            LogTag::Transactions,
            "TABLE",
            &format!(
                "📊 SUMMARY: {} Open, {} Closed | Invested: {:.3} SOL | Received: {:.3} SOL | Fees: {:.3} SOL | Net PnL: {}",
                open_positions,
                closed_positions,
                total_invested,
                total_received,
                total_fees,
                net_pnl_display
            )
        );
        log(LogTag::Transactions, "TABLE", "=== END POSITION ANALYSIS ===");
    }

    /// Analyze and display position lifecycle with PnL calculations
    pub async fn analyze_positions(&mut self, max_count: Option<usize>) -> Result<(), String> {
        let transactions = self.get_recent_transactions(1000).await?;
        let token_cache = std::collections::HashMap::new();
        let swaps: Vec<SwapPnLInfo> = transactions
            .into_iter()
            .filter(|tx| self.is_swap_transaction(tx))
            .filter_map(|tx| self.convert_to_swap_pnl_info(&tx, &token_cache, true))
            .collect();
        let positions = self.calculate_position_analysis(&swaps);
        self.display_position_analysis_table(&positions);
        Ok(())
    }

    /// Calculate position analysis from swap transactions
    fn calculate_position_analysis(&self, swaps: &[SwapPnLInfo]) -> Vec<PositionAnalysis> {
        use std::collections::HashMap;

        let mut positions: HashMap<String, PositionState> = HashMap::new();
        let mut completed_positions: Vec<PositionAnalysis> = Vec::new();

        // Sort swaps by slot for proper chronological processing
        let mut sorted_swaps = swaps.to_vec();
        sorted_swaps.sort_by(|a, b| {
            match (a.slot, b.slot) {
                (Some(a_slot), Some(b_slot)) => a_slot.cmp(&b_slot),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.timestamp.cmp(&b.timestamp),
            }
        });

        log(
            LogTag::Transactions,
            "POSITION_CALC",
            &format!("Processing {} swaps for position analysis", sorted_swaps.len())
        );

        for swap in &sorted_swaps {
            // Skip failed transactions
            if swap.swap_type.starts_with("Failed") {
                continue;
            }

            let position_state = positions
                .entry(swap.token_mint.clone())
                .or_insert_with(|| PositionState {
                    token_mint: swap.token_mint.clone(),
                    token_symbol: swap.token_symbol.clone(),
                    total_tokens: 0.0,
                    total_sol_invested: 0.0,
                    total_sol_received: 0.0,
                    total_fees: 0.0,
                    total_ata_rents: 0.0,
                    buy_count: 0,
                    sell_count: 0,
                    first_buy_slot: None,
                    last_activity_slot: None,
                    first_buy_timestamp: None,
                    last_activity_timestamp: None,
                    average_buy_price: 0.0,
                    transactions: Vec::new(),
                });

            // Track transaction
            position_state.transactions.push(PositionTransaction {
                signature: swap.signature.clone(),
                swap_type: swap.swap_type.clone(),
                sol_amount: swap.sol_amount,
                token_amount: swap.token_amount,
                price: swap.calculated_price_sol,
                timestamp: swap.timestamp,
                slot: swap.slot,
                router: swap.router.clone(),
                fee_sol: swap.fee_sol,
                ata_rents: swap.ata_rents,
            });

            // Update position state
            match swap.swap_type.as_str() {
                "Buy" => {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "DEBUG_BUY",
                            &format!(
                                "Processing BUY for {}: +{:.2} tokens, current total: {:.2} -> {:.2}",
                                swap.token_symbol,
                                swap.token_amount,
                                position_state.total_tokens,
                                position_state.total_tokens + swap.token_amount
                            )
                        );
                    }

                    // If this is the first buy after a position was closed (total_tokens <= 0), this is a new position opening
                    if position_state.total_tokens <= 0.0001 {
                        position_state.first_buy_timestamp = Some(swap.timestamp);
                        position_state.first_buy_slot = swap.slot;
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "DEBUG_POSITION",
                                &format!(
                                    "New position opened for {} at {}",
                                    swap.token_symbol,
                                    swap.timestamp
                                )
                            );
                        }
                    }

                    position_state.total_tokens += swap.token_amount;
                    position_state.total_sol_invested += swap.sol_amount;
                    position_state.total_fees += swap.fee_sol;
                    position_state.total_ata_rents += swap.ata_rents;
                    position_state.buy_count += 1;

                    // Calculate average buy price (weighted by amount)
                    if position_state.total_tokens > 0.0 {
                        position_state.average_buy_price =
                            position_state.total_sol_invested / position_state.total_tokens;
                    }
                }
                "Sell" => {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "DEBUG_SELL",
                            &format!(
                                "Processing SELL for {}: -{:.2} tokens, current total: {:.2} -> {:.2}",
                                swap.token_symbol,
                                swap.token_amount.abs(),
                                position_state.total_tokens,
                                position_state.total_tokens - swap.token_amount.abs()
                            )
                        );
                    }

                    let previous_total = position_state.total_tokens;
                    position_state.total_tokens -= swap.token_amount.abs(); // Always use absolute value for sells
                    position_state.total_sol_received += swap.sol_amount;
                    position_state.total_fees += swap.fee_sol;
                    position_state.total_ata_rents += swap.ata_rents;
                    position_state.sell_count += 1;

                    // If position was just closed (went from > 0 to <= 0), this is the closing timestamp
                    if previous_total > 0.0001 && position_state.total_tokens <= 0.0001 {
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "DEBUG_POSITION",
                                &format!(
                                    "Position closed for {} at {} (tokens went from {:.2} to {:.2})",
                                    swap.token_symbol,
                                    swap.timestamp,
                                    previous_total,
                                    position_state.total_tokens
                                )
                            );
                        }

                        // This swap closed the position - position analysis is now handled by positions manager
                        // No longer using this old position analysis system
                        log(
                            LogTag::Transactions,
                            "POSITION_COMPLETED",
                            &format!(
                                "Position completed for {} - now managed by positions manager",
                                swap.token_symbol
                            )
                        );

                        // Reset the position state for potential future reopening
                        *position_state = PositionState {
                            token_mint: swap.token_mint.clone(),
                            token_symbol: swap.token_symbol.clone(),
                            total_tokens: position_state.total_tokens.min(0.0), // Keep negative if oversold
                            total_sol_invested: 0.0,
                            total_sol_received: position_state.total_sol_received,
                            total_fees: position_state.total_fees,
                            total_ata_rents: position_state.total_ata_rents,
                            buy_count: 0, // Reset to 0 to prevent re-addition
                            sell_count: position_state.sell_count,
                            first_buy_slot: None,
                            last_activity_slot: swap.slot,
                            first_buy_timestamp: None,
                            last_activity_timestamp: Some(swap.timestamp),
                            average_buy_price: 0.0,
                            transactions: if let Some(last_tx) = position_state.transactions.last() {
                                vec![last_tx.clone()]
                            } else {
                                Vec::new() // Handle empty transactions list safely
                            },
                        };
                    }
                }
                _ => {} // Ignore other transaction types
            }

            // Update last activity (for open positions)
            position_state.last_activity_slot = swap.slot;
            position_state.last_activity_timestamp = Some(swap.timestamp);
        }

        // Add remaining open positions - now handled by positions manager
        for (_, position_state) in positions {
            if position_state.total_tokens > 0.0001 || position_state.buy_count > 0 {
                log(
                    LogTag::Transactions,
                    "OPEN_POSITION",
                    &format!(
                        "Open position for {} - now managed by positions manager",
                        position_state.token_symbol
                    )
                );
            }
        }

        // Position analysis is now handled by the new positions manager system
        // This old analysis method is deprecated
        log(
            LogTag::Transactions,
            "DEPRECATED",
            "Position analysis moved to positions manager - returning empty result"
        );

        Vec::new() // Return empty vector as positions are now managed elsewhere
    }

    /// Get transaction summary for logging
    pub fn get_transaction_summary(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, router } => {
                format!("BUY {} SOL → {} tokens via {}", sol_amount, token_amount, router)
            }
            TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, router } => {
                format!("SELL {} tokens → {} SOL via {}", token_amount, sol_amount, router)
            }
            TransactionType::SwapTokenToToken {
                from_mint,
                to_mint,
                from_amount,
                to_amount,
                router,
            } => {
                format!(
                    "SWAP {} {} → {} {} via {}",
                    from_amount,
                    &from_mint[..8],
                    to_amount,
                    &to_mint[..8],
                    router
                )
            }
            TransactionType::SolTransfer { amount, .. } => {
                format!("SOL Transfer: {} SOL", amount)
            }
            TransactionType::TokenTransfer { mint, amount, .. } => {
                format!("Token Transfer: {} of {}", amount, &mint[..8])
            }
            TransactionType::AtaClose { recovered_sol, token_mint } => {
                format!("ATA Close: Recovered {} SOL from {}", recovered_sol, &token_mint[..8])
            }
            TransactionType::Other { description, .. } => { format!("Other: {}", description) }
            TransactionType::Unknown => "Unknown Transaction".to_string(),
        }
    }
}
