// Transaction processing pipeline for the transactions module
//
// This module handles the core transaction processing logic including
// data extraction, analysis, and classification of blockchain transactions.

use chrono::{ DateTime, Utc };
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::{ HashMap, HashSet };
use std::time::{ Duration, Instant };
// Using our centralized RPC TransactionDetails type

use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };

use crate::tokens::{ decimals::lamports_to_sol, get_token_decimals, get_token_from_db };
use crate::transactions::{ analyzer, fetcher::TransactionFetcher, program_ids, types::*, utils::* };

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
        self.extract_balance_changes(&mut transaction, &tx_data).await?;

        // Step 4: Capture instruction breakdown for downstream debugging
        self.analyze_instructions(&mut transaction, &tx_data).await?;

        // Step 5: Analyze ATA operations (rent impact, ATA lifecycle)
        self.analyze_ata_operations(&mut transaction, &tx_data).await?;

        // Step 6: Classify transaction type and direction
        self.analyze_transaction(&mut transaction).await?;

        // Step 7: Calculate swap P&L when classification indicates a swap
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
                    signature,
                    transaction.transaction_type,
                    transaction.direction,
                    processing_duration.as_millis()
                )
            );
        }

        // Persist transaction snapshots to the database (best-effort)
        if let Some(db) = crate::transactions::database::get_transaction_database().await {
            match db.upsert_full_transaction(&transaction).await {
                Ok(_) => {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "CACHE_STORE",
                            &format!(
                                "Cached {} (status={:?}, analysis_v={}, success={})",
                                signature,
                                transaction.status,
                                ANALYSIS_CACHE_VERSION,
                                transaction.success
                            )
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to persist transaction {}: {}", signature, e)
                    );
                }
            }
        }

        // Record event for analytics
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
    ) -> Result<crate::rpc::TransactionDetails, String> {
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
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<Transaction, String> {
        // Extract timestamp
        let timestamp = if let Some(block_time) = tx_data.block_time {
            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
        } else {
            Utc::now()
        };

        // Determine success status
        let success = tx_data.meta
            .as_ref()
            .map_or(false, |meta| {
                meta.err.is_none() || meta.err.as_ref().map_or(false, |e| e.is_null())
            });

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
        // Our TransactionMeta doesn't currently track compute units consumed
        let compute_units_consumed = None;

        // Count instructions and accounts
        let instructions_count = tx_data.transaction.message
            .get("instructions")
            .and_then(|inst| inst.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        let accounts_count = tx_data.transaction.message
            .get("accountKeys")
            .and_then(|keys| keys.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);

        // Create transaction structure
        let mut transaction = Transaction::new(signature.to_string());
        transaction.slot = Some(tx_data.slot);
        transaction.block_time = tx_data.block_time;
        transaction.timestamp = timestamp;
        transaction.status = if success {
            TransactionStatus::Finalized
        } else {
            TransactionStatus::Failed(error_message.clone().unwrap_or_default())
        };
        transaction.success = success;
        transaction.error_message = error_message;
        transaction.fee_lamports = fee_lamports;
        transaction.fee_sol = fee_lamports.map_or(0.0, |f| (f as f64) / 1_000_000_000.0);
        transaction.compute_units_consumed = compute_units_consumed;
        transaction.instructions_count = instructions_count;
        transaction.accounts_count = accounts_count;

        // Capture raw metadata for downstream debugging
        transaction.raw_transaction_data = serde_json::to_value(tx_data).ok();
        if let Some(meta) = tx_data.meta.as_ref() {
            if let Some(logs) = &meta.log_messages {
                transaction.log_messages = logs.clone();
            }
        }

        // Determine whether the configured wallet signed the transaction
        let wallet_str = self.wallet_pubkey.to_string();
        let account_keys = account_keys_from_message(&tx_data.transaction.message);
        if let Some(header) = tx_data.transaction.message.get("header") {
            if let Some(required) = header.get("numRequiredSignatures").and_then(|v| v.as_u64()) {
                if
                    account_keys
                        .iter()
                        .take(required as usize)
                        .any(|key| key == &wallet_str)
                {
                    transaction.wallet_signed = true;
                }
            }
        }

        if !transaction.wallet_signed {
            if
                let Some(array) = tx_data.transaction.message
                    .get("accountKeys")
                    .and_then(|v| v.as_array())
            {
                for entry in array {
                    if let Some(obj) = entry.as_object() {
                        if
                            obj
                                .get("pubkey")
                                .and_then(|v| v.as_str())
                                .map(|s| s == wallet_str)
                                .unwrap_or(false)
                        {
                            if
                                obj
                                    .get("signer")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                            {
                                transaction.wallet_signed = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DATA_EXTRACT",
                &format!(
                    "Extracted data for {}: success={}, fee={}SOL, instructions={}, accounts={}",
                    signature,
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
                    &transaction.signature,
                    transaction.transaction_type,
                    transaction.direction
                )
            );
        }

        Ok(())
    }

    /// Extract balance changes from transaction metadata
    async fn extract_balance_changes(
        &self,
        transaction: &mut Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<(), String> {
        if let Some((sol_change, lamport_delta)) = self.extract_sol_balance_change(tx_data).await? {
            transaction.sol_balance_change = sol_change.change;
            transaction.wallet_lamport_change = lamport_delta;
            transaction.sol_balance_changes = vec![sol_change];
        } else {
            transaction.sol_balance_changes.clear();
        }

        let token_changes = self.extract_token_balance_changes(tx_data).await?;
        transaction.token_balance_changes = token_changes;

        transaction.token_transfers = self.derive_token_transfers(
            &transaction.token_balance_changes
        );

        if
            self.debug_enabled &&
            (transaction.sol_balance_change.abs() > f64::EPSILON ||
                !transaction.token_balance_changes.is_empty())
        {
            log(
                LogTag::Transactions,
                "BALANCE_EXTRACT",
                &format!(
                    "Extracted balances for {}: SOL change = {:.9}, tokens = {}",
                    &transaction.signature,
                    transaction.sol_balance_change,
                    transaction.token_balance_changes.len()
                )
            );
        }

        Ok(())
    }

    /// Extract SOL balance change for the configured wallet
    async fn extract_sol_balance_change(
        &self,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<Option<(SolBalanceChange, i64)>, String> {
        let meta = match tx_data.meta.as_ref() {
            Some(meta) => meta,
            None => {
                return Ok(None);
            }
        };

        let account_keys = account_keys_from_message(&tx_data.transaction.message);
        let wallet_str = self.wallet_pubkey.to_string();

        if let Some(index) = account_keys.iter().position(|key| key == &wallet_str) {
            let pre_lamports = *meta.pre_balances.get(index).unwrap_or(&0);
            let post_lamports = *meta.post_balances.get(index).unwrap_or(&0);
            let lamport_delta = (post_lamports as i64) - (pre_lamports as i64);

            let sol_change = SolBalanceChange {
                account: wallet_str,
                pre_balance: lamports_to_sol(pre_lamports),
                post_balance: lamports_to_sol(post_lamports),
                change: lamports_to_sol(post_lamports) - lamports_to_sol(pre_lamports),
            };

            Ok(Some((sol_change, lamport_delta)))
        } else {
            Ok(None)
        }
    }

    /// Extract token balance changes that impact the configured wallet
    async fn extract_token_balance_changes(
        &self,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<Vec<TokenBalanceChange>, String> {
        let mut changes = Vec::new();
        let meta = match tx_data.meta.as_ref() {
            Some(meta) => meta,
            None => {
                return Ok(changes);
            }
        };

        let wallet_str = self.wallet_pubkey.to_string();
        let mut pre_totals: HashMap<String, (u8, f64)> = HashMap::new();
        let mut post_totals: HashMap<String, (u8, f64)> = HashMap::new();

        if let Some(pre_balances) = &meta.pre_token_balances {
            for balance in pre_balances {
                if balance.owner.as_deref() == Some(wallet_str.as_str()) {
                    let decimals = balance.ui_token_amount.decimals;
                    let amount = parse_ui_amount(&balance.ui_token_amount);
                    let entry = pre_totals.entry(balance.mint.clone()).or_insert((decimals, 0.0));
                    entry.0 = decimals;
                    entry.1 += amount;
                }
            }
        }

        if let Some(post_balances) = &meta.post_token_balances {
            for balance in post_balances {
                if balance.owner.as_deref() == Some(wallet_str.as_str()) {
                    let decimals = balance.ui_token_amount.decimals;
                    let amount = parse_ui_amount(&balance.ui_token_amount);
                    let entry = post_totals.entry(balance.mint.clone()).or_insert((decimals, 0.0));
                    entry.0 = decimals;
                    entry.1 += amount;
                }
            }
        }

        let mut all_mints: HashSet<String> = pre_totals.keys().cloned().collect();
        all_mints.extend(post_totals.keys().cloned());

        for mint in all_mints {
            let (pre_decimals, pre_amount) = pre_totals.get(&mint).cloned().unwrap_or((0, 0.0));
            let (post_decimals, post_amount) = post_totals
                .get(&mint)
                .cloned()
                .unwrap_or((pre_decimals, pre_amount));
            let decimals = if post_decimals != 0 { post_decimals } else { pre_decimals };
            let change = post_amount - pre_amount;

            if change.abs() < 1e-12 {
                continue;
            }

            changes.push(TokenBalanceChange {
                mint: mint.clone(),
                decimals,
                pre_balance: if pre_amount.abs() < 1e-12 {
                    None
                } else {
                    Some(pre_amount)
                },
                post_balance: if post_amount.abs() < 1e-12 {
                    None
                } else {
                    Some(post_amount)
                },
                change,
                usd_value: None,
            });
        }

        Ok(changes)
    }

    /// Derive simple token transfer summaries from balance changes
    fn derive_token_transfers(&self, token_changes: &[TokenBalanceChange]) -> Vec<TokenTransfer> {
        if token_changes.is_empty() {
            return Vec::new();
        }

        let wallet = self.wallet_pubkey.to_string();
        token_changes
            .iter()
            .filter(|change| change.change.abs() > 1e-9)
            .map(|change| TokenTransfer {
                mint: change.mint.clone(),
                amount: change.change.abs(),
                from: if change.change < 0.0 {
                    wallet.clone()
                } else {
                    "external".to_string()
                },
                to: if change.change > 0.0 {
                    wallet.clone()
                } else {
                    "external".to_string()
                },
                program_id: "unknown".to_string(),
            })
            .collect()
    }
}

/// Extract account keys from a transaction message (legacy and v0 support)
fn account_keys_from_message(message: &Value) -> Vec<String> {
    // Legacy format: array of strings
    if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
        return array
            .iter()
            .filter_map(|entry| {
                if let Some(key) = entry.as_str() {
                    Some(key.to_string())
                } else if let Some(obj) = entry.as_object() {
                    obj.get("pubkey")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
    }

    // v0 format: object with static and loaded keys
    if let Some(obj) = message.get("accountKeys").and_then(|v| v.as_object()) {
        let mut keys = Vec::new();
        if let Some(static_keys) = obj.get("staticAccountKeys").and_then(|v| v.as_array()) {
            keys.extend(
                static_keys.iter().filter_map(|entry| entry.as_str().map(|s| s.to_string()))
            );
        }
        if let Some(loaded_keys) = obj.get("accountKeys").and_then(|v| v.as_array()) {
            keys.extend(
                loaded_keys.iter().filter_map(|entry| entry.as_str().map(|s| s.to_string()))
            );
        }
        if !keys.is_empty() {
            return keys;
        }
    }

    Vec::new()
}

/// Parse UI token amount with graceful fallback to raw representation
fn parse_ui_amount(amount: &crate::rpc::UiTokenAmount) -> f64 {
    if let Some(ui) = amount.ui_amount {
        return ui;
    }

    if let Some(ui_str) = &amount.ui_amount_string {
        if let Ok(parsed) = ui_str.parse::<f64>() {
            return parsed;
        }
    }

    if let Ok(raw) = amount.amount.parse::<u128>() {
        if amount.decimals == 0 {
            return raw as f64;
        }
        let scale = (10u128).saturating_pow(amount.decimals as u32);
        if scale == 0 {
            return 0.0;
        }
        return (raw as f64) / (scale as f64);
    }

    0.0
}

// =============================================================================
// ATA OPERATIONS ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Analyze ATA (Associated Token Account) operations
    async fn analyze_ata_operations(
        &self,
        transaction: &mut Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<(), String> {
        let (ata_operations, ata_analysis) = self.extract_ata_operations(
            transaction,
            tx_data
        ).await?;

        if ata_operations.is_empty() {
            return Ok(());
        }

        if let Some(analysis) = ata_analysis {
            transaction.ata_analysis = Some(analysis);
        }

        transaction.ata_operations = ata_operations;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ATA_ANALYZE",
                &format!(
                    "Detected {} ATA operations in {}",
                    transaction.ata_operations.len(),
                    &transaction.signature
                )
            );
        }

        Ok(())
    }

    /// Extract ATA operations from transaction instructions (best-effort heuristic)
    async fn extract_ata_operations(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<(Vec<AtaOperation>, Option<AtaAnalysis>), String> {
        const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        let Some(meta) = tx_data.meta.as_ref() else {
            return Ok((Vec::new(), None));
        };

        let wallet = self.wallet_pubkey.to_string();
        let account_keys = account_keys_from_message(&tx_data.transaction.message);
        let account_index_map: HashMap<String, usize> = account_keys
            .iter()
            .enumerate()
            .map(|(idx, key)| (key.clone(), idx))
            .collect();

        let mut account_mints: HashMap<String, String> = HashMap::new();
        let mut account_owners: HashMap<String, String> = HashMap::new();

        let populate_balances = |
            balances: &Option<Vec<crate::rpc::TokenBalance>>,
            account_mints: &mut HashMap<String, String>,
            account_owners: &mut HashMap<String, String>
        | {
            if let Some(entries) = balances {
                for balance in entries {
                    if let Some(address) = account_keys.get(balance.account_index as usize) {
                        account_mints.insert(address.clone(), balance.mint.clone());
                        if let Some(owner) = &balance.owner {
                            account_owners.insert(address.clone(), owner.clone());
                        }
                    }
                }
            }
        };

        populate_balances(&meta.pre_token_balances, &mut account_mints, &mut account_owners);
        populate_balances(&meta.post_token_balances, &mut account_mints, &mut account_owners);

        let mut inner_instruction_map: HashMap<usize, Vec<Value>> = HashMap::new();
        if let Some(inner) = &meta.inner_instructions {
            for entry in inner {
                if let Some(index) = entry.get("index").and_then(|v| v.as_u64()) {
                    if
                        let Some(instructions) = entry
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        let bucket = inner_instruction_map.entry(index as usize).or_default();
                        for inst in instructions {
                            bucket.push(inst.clone());
                        }
                    }
                }
            }
        }

        let outer_instructions: Vec<Value> = tx_data.transaction.message
            .get("instructions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().cloned().collect())
            .unwrap_or_default();

        let value_to_u64 = |value: &Value| -> Option<u64> {
            if let Some(lamports) = value.as_u64() {
                Some(lamports)
            } else if let Some(s) = value.as_str() {
                s.parse().ok()
            } else {
                None
            }
        };

        let mut account_rent_spent: HashMap<String, u64> = HashMap::new();
        let mut account_rent_recovered: HashMap<String, u64> = HashMap::new();

        let mut handle_instruction = |instruction: &Value| {
            let program_id = instruction
                .get("programId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    instruction
                        .get("programIdIndex")
                        .and_then(|v| v.as_u64())
                        .and_then(|idx| account_keys.get(idx as usize).cloned())
                })
                .unwrap_or_default();

            if program_id.is_empty() {
                return;
            }

            let parsed = match instruction.get("parsed").and_then(|v| v.as_object()) {
                Some(parsed) => parsed,
                None => {
                    return;
                }
            };

            let instruction_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            let info = match parsed.get("info").and_then(|v| v.as_object()) {
                Some(info) => info,
                None => {
                    return;
                }
            };

            if program_id == TOKEN_PROGRAM_ID {
                match instruction_type.as_str() {
                    "initializeaccount" | "initializeaccount2" | "initializeaccount3" => {
                        let account = info
                            .get("account")
                            .or_else(|| info.get("newAccount"))
                            .and_then(|v| v.as_str());
                        if let Some(account) = account {
                            if let Some(mint) = info.get("mint").and_then(|v| v.as_str()) {
                                account_mints.insert(account.to_string(), mint.to_string());
                            }
                            if let Some(owner) = info.get("owner").and_then(|v| v.as_str()) {
                                account_owners.insert(account.to_string(), owner.to_string());
                            }
                        }
                    }
                    "closeaccount" => {
                        if let Some(account) = info.get("account").and_then(|v| v.as_str()) {
                            let destination_matches_wallet = info
                                .get("destination")
                                .and_then(|v| v.as_str())
                                .map(|dest| dest == wallet)
                                .unwrap_or(false);

                            if destination_matches_wallet {
                                if let Some(index) = account_index_map.get(account) {
                                    let pre = meta.pre_balances.get(*index).copied().unwrap_or(0);
                                    let post = meta.post_balances.get(*index).copied().unwrap_or(0);
                                    if pre > post {
                                        account_rent_recovered.insert(
                                            account.to_string(),
                                            pre - post
                                        );
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            } else if
                program_id == "11111111111111111111111111111111" &&
                instruction_type == "createaccount"
            {
                let Some(source) = info.get("source").and_then(|v| v.as_str()) else {
                    return;
                };

                if source != wallet {
                    return;
                }

                let Some(owner) = info.get("owner").and_then(|v| v.as_str()) else {
                    return;
                };

                if owner != TOKEN_PROGRAM_ID {
                    return;
                }

                let Some(new_account) = info.get("newAccount").and_then(|v| v.as_str()) else {
                    return;
                };

                if let Some(lamports) = info.get("lamports").and_then(|v| value_to_u64(v)) {
                    if lamports > 0 {
                        account_rent_spent.insert(new_account.to_string(), lamports);
                    }
                }
            }
        };

        let mut processed_inner_indexes: HashSet<usize> = HashSet::new();

        for (idx, instruction) in outer_instructions.iter().enumerate() {
            handle_instruction(instruction);

            if let Some(inner_list) = inner_instruction_map.get(&idx) {
                for inner_inst in inner_list {
                    handle_instruction(inner_inst);
                }
                processed_inner_indexes.insert(idx);
            }
        }

        for (idx, inner_list) in inner_instruction_map.iter() {
            if processed_inner_indexes.contains(idx) {
                continue;
            }
            for inner_inst in inner_list {
                handle_instruction(inner_inst);
            }
        }

        let mut operations = Vec::new();

        for (account, lamports) in account_rent_spent.iter() {
            if *lamports == 0 {
                continue;
            }

            let owner_matches_wallet = account_owners
                .get(account)
                .map(|owner| owner == &wallet)
                .unwrap_or(false);

            if !owner_matches_wallet {
                continue;
            }

            let mint = account_mints
                .get(account)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let rent_sol = lamports_to_sol(*lamports);
            let is_wsol = mint == WSOL_MINT;

            operations.push(AtaOperation {
                operation_type: AtaOperationType::Creation,
                account_address: account.clone(),
                token_mint: mint.clone(),
                rent_amount: rent_sol,
                is_wsol,
                mint,
                rent_cost_sol: Some(rent_sol),
            });
        }

        for (account, lamports) in account_rent_recovered.iter() {
            if *lamports == 0 {
                continue;
            }

            let owner_matches_wallet = account_owners
                .get(account)
                .map(|owner| owner == &wallet)
                .unwrap_or(false);

            if !owner_matches_wallet {
                continue;
            }

            let mint = account_mints
                .get(account)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let rent_sol = lamports_to_sol(*lamports);
            let is_wsol = mint == WSOL_MINT;

            operations.push(AtaOperation {
                operation_type: AtaOperationType::Closure,
                account_address: account.clone(),
                token_mint: mint.clone(),
                rent_amount: rent_sol,
                is_wsol,
                mint,
                rent_cost_sol: Some(rent_sol),
            });
        }

        if operations.is_empty() {
            return Ok((operations, None));
        }

        let mut analysis = AtaAnalysis {
            total_ata_creations: 0,
            total_ata_closures: 0,
            token_ata_creations: 0,
            token_ata_closures: 0,
            wsol_ata_creations: 0,
            wsol_ata_closures: 0,
            total_rent_spent: 0.0,
            total_rent_recovered: 0.0,
            net_rent_impact: 0.0,
            token_rent_spent: 0.0,
            token_rent_recovered: 0.0,
            token_net_rent_impact: 0.0,
            wsol_rent_spent: 0.0,
            wsol_rent_recovered: 0.0,
            wsol_net_rent_impact: 0.0,
            detected_operations: Vec::new(),
        };

        for op in &operations {
            match op.operation_type {
                AtaOperationType::Creation => {
                    analysis.total_ata_creations += 1;
                    analysis.total_rent_spent += op.rent_amount;
                    if op.is_wsol {
                        analysis.wsol_ata_creations += 1;
                        analysis.wsol_rent_spent += op.rent_amount;
                    } else {
                        analysis.token_ata_creations += 1;
                        analysis.token_rent_spent += op.rent_amount;
                    }
                }
                AtaOperationType::Closure => {
                    analysis.total_ata_closures += 1;
                    analysis.total_rent_recovered += op.rent_amount;
                    if op.is_wsol {
                        analysis.wsol_ata_closures += 1;
                        analysis.wsol_rent_recovered += op.rent_amount;
                    } else {
                        analysis.token_ata_closures += 1;
                        analysis.token_rent_recovered += op.rent_amount;
                    }
                }
            }
        }

        analysis.net_rent_impact = analysis.total_rent_recovered - analysis.total_rent_spent;
        analysis.token_net_rent_impact = analysis.token_rent_recovered - analysis.token_rent_spent;
        analysis.wsol_net_rent_impact = analysis.wsol_rent_recovered - analysis.wsol_rent_spent;
        analysis.detected_operations = operations.clone();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ATA_ANALYZE",
                &format!(
                    "{}: detected {} ATA operations (rent_spent={:.6} SOL, rent_recovered={:.6} SOL)",
                    &transaction.signature,
                    operations.len(),
                    analysis.total_rent_spent,
                    analysis.total_rent_recovered
                )
            );
        }

        Ok((operations, Some(analysis)))
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
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "SWAP_INFO",
                    &format!(
                        "Derived swap info for {}: type={}, token={}, sol_change={:.6}",
                        &transaction.signature,
                        swap_info.swap_type,
                        &swap_info.mint,
                        transaction.sol_balance_change
                    )
                );
            }

            transaction.token_symbol = Some(swap_info.symbol.clone());
            transaction.token_decimals = Some(swap_info.decimals);
            transaction.token_swap_info = Some(swap_info.clone());
            transaction.token_info = Some(swap_info.clone());

            if let Some(pnl_info) = self.calculate_pnl_from_swap(transaction, &swap_info).await? {
                transaction.calculated_token_price_sol = Some(pnl_info.calculated_price_sol);
                transaction.swap_pnl_info = Some(pnl_info);

                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "PNL_CALC",
                        &format!(
                            "Calculated P&L for {}: net_sol={:.6}",
                            &transaction.signature,
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
        let epsilon = 1e-9;

        if transaction.token_balance_changes.is_empty() {
            return Ok(None);
        }

        let Some(primary_change) = transaction.token_balance_changes
            .iter()
            .filter(|change| !is_wsol_mint(&change.mint))
            .max_by(|a, b| {
                a.change.abs().partial_cmp(&b.change.abs()).unwrap_or(std::cmp::Ordering::Equal)
            }) else {
            return Ok(None);
        };

        if primary_change.change.abs() <= epsilon {
            return Ok(None);
        }

        let sol_change = transaction.sol_balance_change;

        let is_buy = sol_change < -epsilon && primary_change.change > epsilon;
        let is_sell = sol_change > epsilon && primary_change.change < -epsilon;

        if !is_buy && !is_sell {
            return Ok(None);
        }

        let mut decimals = primary_change.decimals;
        let token_mint = primary_change.mint.clone();

        if decimals == 0 {
            if let Some(db_decimals) = get_token_decimals(&token_mint).await {
                decimals = db_decimals;
            } else {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "SWAP_SKIP",
                        &format!("Missing decimals for {}, cannot derive swap info", &token_mint)
                    );
                }
                return Ok(None);
            }
        }

        let mut symbol = format!("{}", &token_mint);
        let mut current_price_sol = None;
        let mut is_verified = false;

        if let Some(token) = get_token_from_db(&token_mint).await {
            if let Some(db_decimals) = token.decimals {
                decimals = db_decimals;
            }

            if !token.symbol.is_empty() {
                symbol = token.symbol.clone();
            }

            current_price_sol = token.price_pool_sol.or(token.price_dexscreener_sol);
            is_verified = token.is_verified;
        }

        if decimals == 0 {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "SWAP_SKIP",
                    &format!(
                        "Unable to determine decimals for {}, swap analysis skipped",
                        &token_mint
                    )
                );
            }
            return Ok(None);
        }

        let token_amount_ui = primary_change.change.abs();
        if token_amount_ui <= epsilon {
            return Ok(None);
        }

        let fee_sol = transaction.fee_sol;

        let total_sol_spent = (-sol_change).max(0.0);
        let sol_spent_for_tokens = (total_sol_spent - fee_sol).max(0.0);

        let router = Self::infer_swap_router(transaction);
        let mut sol_spent_effective = if matches!(router.as_str(), "jupiter" | "pumpfun") {
            self.extract_dex_swap_amount(transaction, sol_spent_for_tokens, is_buy, &router)
        } else if sol_spent_for_tokens > epsilon {
            sol_spent_for_tokens
        } else {
            total_sol_spent.max(0.0)
        };

        let net_sol_received = sol_change.max(0.0);
        let mut sol_received_from_swap = (net_sol_received + fee_sol).max(0.0);

        let net_wsol_rent = transaction.ata_analysis
            .as_ref()
            .map(|ata| ata.wsol_rent_spent - ata.wsol_rent_recovered)
            .unwrap_or(0.0);

        if net_wsol_rent.abs() > epsilon {
            if is_buy {
                sol_spent_effective = (sol_spent_effective - net_wsol_rent).max(0.0);
            } else if is_sell {
                sol_received_from_swap = (sol_received_from_swap + net_wsol_rent).max(0.0);
            }
        }

        let (swap_type, input_mint, output_mint, input_ui_amount, output_ui_amount) = if is_buy {
            (
                "sol_to_token".to_string(),
                WSOL_MINT.to_string(),
                token_mint.clone(),
                sol_spent_effective,
                token_amount_ui,
            )
        } else {
            (
                "token_to_sol".to_string(),
                token_mint.clone(),
                WSOL_MINT.to_string(),
                token_amount_ui,
                sol_received_from_swap,
            )
        };

        let input_decimals = if input_mint == WSOL_MINT { 9 } else { decimals };

        let output_decimals = if output_mint == WSOL_MINT { 9 } else { decimals };

        let input_amount = Self::ui_to_raw_amount(input_ui_amount, input_decimals);
        let output_amount = Self::ui_to_raw_amount(output_ui_amount, output_decimals);

        Ok(
            Some(TokenSwapInfo {
                mint: token_mint,
                symbol,
                decimals,
                current_price_sol,
                is_verified,
                router,
                swap_type,
                input_mint,
                output_mint,
                input_amount,
                output_amount,
                input_ui_amount,
                output_ui_amount,
                pool_address: None,
                program_id: "heuristic".to_string(),
            })
        )
    }

    /// Calculate P&L from swap information
    async fn calculate_pnl_from_swap(
        &self,
        transaction: &Transaction,
        swap_info: &TokenSwapInfo
    ) -> Result<Option<SwapPnLInfo>, String> {
        let swap_direction = match swap_info.swap_type.as_str() {
            "sol_to_token" => "Buy",
            "token_to_sol" => "Sell",
            _ => {
                return Ok(None);
            }
        };

        let mut pnl_info = SwapPnLInfo {
            token_mint: swap_info.mint.clone(),
            token_symbol: swap_info.symbol.clone(),
            swap_type: swap_direction.to_string(),
            sol_amount: 0.0,
            token_amount: 0.0,
            calculated_price_sol: 0.0,
            timestamp: transaction.timestamp,
            signature: transaction.signature.clone(),
            router: swap_info.router.clone(),
            fee_sol: transaction.fee_sol,
            ata_rents: 0.0,
            effective_sol_spent: 0.0,
            effective_sol_received: 0.0,
            ata_created_count: 0,
            ata_closed_count: 0,
            slot: transaction.slot,
            status: if transaction.success {
                "✅ Success".to_string()
            } else {
                "❌ Failed".to_string()
            },
            sol_spent: 0.0,
            sol_received: 0.0,
            tokens_bought: 0.0,
            tokens_sold: 0.0,
            net_sol_change: transaction.sol_balance_change,
            estimated_token_value_sol: None,
            estimated_pnl_sol: None,
            fees_paid_sol: transaction.fee_sol,
        };

        let price_numerator = match swap_direction {
            "Buy" => swap_info.input_ui_amount,
            "Sell" => swap_info.output_ui_amount,
            _ => 0.0,
        };

        match swap_direction {
            "Buy" => {
                let gross_sol_spent = swap_info.input_ui_amount + transaction.fee_sol;
                pnl_info.sol_amount = gross_sol_spent;
                pnl_info.sol_spent = gross_sol_spent;
                pnl_info.effective_sol_spent = swap_info.input_ui_amount;
                pnl_info.token_amount = swap_info.output_ui_amount;
                pnl_info.tokens_bought = swap_info.output_ui_amount;
                pnl_info.net_sol_change = transaction.sol_balance_change;
            }
            "Sell" => {
                let net_sol_received = (swap_info.output_ui_amount - transaction.fee_sol).max(0.0);
                pnl_info.sol_amount = net_sol_received;
                pnl_info.sol_received = net_sol_received;
                pnl_info.effective_sol_received = net_sol_received;
                pnl_info.token_amount = swap_info.input_ui_amount;
                pnl_info.tokens_sold = swap_info.input_ui_amount;
                pnl_info.net_sol_change = transaction.sol_balance_change;
            }
            _ => {}
        }

        if pnl_info.token_amount > f64::EPSILON {
            pnl_info.calculated_price_sol = price_numerator / pnl_info.token_amount;
        }

        if let Some(ata) = &transaction.ata_analysis {
            let rent_delta = ata.token_rent_spent - ata.token_rent_recovered;
            pnl_info.ata_created_count = ata.token_ata_creations;
            pnl_info.ata_closed_count = ata.token_ata_closures;
            pnl_info.ata_rents = rent_delta;

            if swap_direction == "Buy" {
                pnl_info.effective_sol_spent = (
                    pnl_info.effective_sol_spent - rent_delta.max(0.0)
                ).max(0.0);
            } else if swap_direction == "Sell" {
                if rent_delta < 0.0 {
                    pnl_info.effective_sol_received += rent_delta.abs();
                }
            }
        }

        Ok(Some(pnl_info))
    }

    fn ui_to_raw_amount(amount: f64, decimals: u8) -> u64 {
        if !amount.is_finite() || amount <= 0.0 {
            return 0;
        }

        let scale = (10_f64).powi(decimals as i32);
        let raw = (amount * scale).round();

        if !raw.is_finite() || raw <= 0.0 {
            return 0;
        }

        raw.min(u64::MAX as f64) as u64
    }

    /// Extract the pure swap amount from DEX transactions (Jupiter, PumpFun, etc.)
    /// This tries to find the actual input amount excluding platform fees and routing costs
    fn extract_dex_swap_amount(
        &self,
        transaction: &Transaction,
        fallback_amount: f64,
        is_buy: bool,
        router: &str
    ) -> f64 {
        if !is_buy {
            return fallback_amount;
        }

        if matches!(router, "jupiter" | "pumpfun") {
            if let Some(amount) = self.detect_wallet_wsol_transfer_amount(transaction) {
                return amount;
            }
        }

        fallback_amount
    }

    fn detect_wallet_wsol_transfer_amount(&self, transaction: &Transaction) -> Option<f64> {
        const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        let wallet = self.wallet_pubkey.to_string();
        let raw = transaction.raw_transaction_data.as_ref()?;

        let parse_amount = |instruction: &Value, wallet: &str| -> Option<f64> {
            let program_id = instruction.get("programId").and_then(|v| v.as_str())?;
            if program_id != TOKEN_PROGRAM_ID {
                return None;
            }

            let parsed = instruction.get("parsed")?.as_object()?;
            let instruction_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if instruction_type != "transfer" && instruction_type != "transferchecked" {
                return None;
            }

            let info = parsed.get("info")?.as_object()?;

            if
                info
                    .get("authority")
                    .and_then(|v| v.as_str())
                    .map(|s| s != wallet)
                    .unwrap_or(true)
            {
                return None;
            }

            if
                info
                    .get("mint")
                    .and_then(|v| v.as_str())
                    .map(|mint| mint != WSOL_MINT)
                    .unwrap_or(true)
            {
                return None;
            }

            if let Some(token_amount) = info.get("tokenAmount").and_then(|v| v.as_object()) {
                if let Some(ui_amount) = token_amount.get("uiAmount").and_then(|v| v.as_f64()) {
                    if ui_amount > 0.0 {
                        return Some(ui_amount);
                    }
                }

                if let Some(amount_str) = token_amount.get("amount").and_then(|v| v.as_str()) {
                    if let Ok(raw_amount) = amount_str.parse::<u128>() {
                        let decimals = token_amount
                            .get("decimals")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let scale_power = decimals.min(18) as i32;
                        let scale = (10_f64).powi(scale_power);
                        if scale > 0.0 {
                            return Some((raw_amount as f64) / scale);
                        }
                    }
                }
            }

            if let Some(amount_str) = info.get("amount").and_then(|v| v.as_str()) {
                if let Ok(raw_amount) = amount_str.parse::<u128>() {
                    let decimals = info
                        .get("decimals")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(9);
                    let scale_power = decimals.min(18) as i32;
                    let scale = (10_f64).powi(scale_power);
                    if scale > 0.0 {
                        return Some((raw_amount as f64) / scale);
                    }
                }
            }

            None
        };

        let meta = raw.get("meta")?;

        if let Some(inner) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for instruction in instructions {
                        if let Some(amount) = parse_amount(instruction, &wallet) {
                            return Some(amount);
                        }
                    }
                }
            }
        }

        if
            let Some(outer) = raw
                .get("transaction")
                .and_then(|tx| tx.get("message"))
                .and_then(|message| message.get("instructions"))
                .and_then(|v| v.as_array())
        {
            for instruction in outer {
                if let Some(amount) = parse_amount(instruction, &wallet) {
                    return Some(amount);
                }
            }
        }

        None
    }

    fn infer_swap_router(transaction: &Transaction) -> String {
        // First, try to detect router from program IDs (more reliable)
        for instruction in &transaction.instructions {
            if
                let Some(router) = program_ids::detect_router_from_program_id(
                    &instruction.program_id
                )
            {
                return router.to_string();
            }
        }

        // Fallback to log message detection
        if let Some(router) = program_ids::detect_router_from_logs(&transaction.log_messages) {
            return router.to_string();
        }

        "unknown".to_string()
    }
}

// =============================================================================
// INSTRUCTION ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Analyze transaction instructions for detailed breakdown
    async fn analyze_instructions(
        &self,
        transaction: &mut Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<(), String> {
        let instruction_info = self.extract_instruction_info(tx_data).await?;
        transaction.instruction_info = instruction_info.clone();
        transaction.instructions = instruction_info;

        if self.debug_enabled && !transaction.instruction_info.is_empty() {
            log(
                LogTag::Transactions,
                "INSTRUCTION_ANALYZE",
                &format!(
                    "Analyzed {} instructions in {}",
                    transaction.instruction_info.len(),
                    &transaction.signature
                )
            );
        }

        Ok(())
    }

    /// Extract instruction information from transaction message (legacy + v0)
    async fn extract_instruction_info(
        &self,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<Vec<InstructionInfo>, String> {
        let mut instructions = Vec::new();
        let message = &tx_data.transaction.message;
        let account_keys = account_keys_from_message(message);

        if let Some(array) = message.get("instructions").and_then(|v| v.as_array()) {
            for inst in array {
                let program_id = inst
                    .get("programId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        inst.get("programIdIndex")
                            .and_then(|v| v.as_u64())
                            .and_then(|idx| account_keys.get(idx as usize).cloned())
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                let instruction_type = inst
                    .get("parsed")
                    .and_then(|parsed| parsed.get("type").and_then(|v| v.as_str()))
                    .or_else(|| inst.get("type").and_then(|v| v.as_str()))
                    .unwrap_or("unknown")
                    .to_string();

                let accounts = if let Some(accs) = inst.get("accounts").and_then(|v| v.as_array()) {
                    accs.iter()
                        .filter_map(|acc| {
                            if let Some(s) = acc.as_str() {
                                Some(s.to_string())
                            } else if let Some(idx) = acc.as_u64() {
                                account_keys.get(idx as usize).cloned()
                            } else if let Some(obj) = acc.as_object() {
                                obj.get("pubkey")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                let data = inst
                    .get("data")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                instructions.push(InstructionInfo {
                    program_id,
                    instruction_type,
                    accounts,
                    data,
                });
            }
        }

        // Handle compiled instructions for v0 transactions
        if instructions.is_empty() {
            if let Some(compiled) = message.get("compiledInstructions").and_then(|v| v.as_array()) {
                for inst in compiled {
                    let program_id = inst
                        .get("programIdIndex")
                        .and_then(|v| v.as_u64())
                        .and_then(|idx| account_keys.get(idx as usize).cloned())
                        .unwrap_or_else(|| "unknown".to_string());

                    let accounts = inst
                        .get("accountIndexes")
                        .and_then(|v| v.as_array())
                        .map(|accs| {
                            accs.iter()
                                .filter_map(|idx| idx.as_u64())
                                .filter_map(|idx| account_keys.get(idx as usize).cloned())
                                .collect()
                        })
                        .unwrap_or_default();

                    let data = inst
                        .get("data")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    instructions.push(InstructionInfo {
                        program_id,
                        instruction_type: "compiled".to_string(),
                        accounts,
                        data,
                    });
                }
            }
        }

        Ok(instructions)
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
            &format!("Processing error for {}: {}", signature, error)
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
                &format!("Error is recoverable for {}, will retry later", signature)
            );
            // Add to deferred retries (would be handled by service layer)
        } else {
            log(
                LogTag::Transactions,
                "PERMANENT_ERROR",
                &format!("Error is permanent for {}, skipping", signature)
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
