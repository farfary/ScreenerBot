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
use crate::arguments::{ is_cache_only_enabled, is_force_refresh_enabled };
use crate::logger::{ log, LogTag };

use crate::tokens::{ decimals::lamports_to_sol, get_token_decimals, get_token_from_db };
use crate::transactions::{ analyzer, fetcher::TransactionFetcher, program_ids, types::*, utils::* };

// =============================================================================
// CONSTANTS
// =============================================================================

/// Known MEV/Jito tip addresses that should be excluded from swap calculations
const KNOWN_MEV_TIP_ADDRESSES: &[&str] = &[
    "BB5dnY55FXS1e1NXqZDwCzgdYJdMCj3B92PU6Q5Fb6DT", // Jito tip address
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5", // Jito tip address
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe", // Jito tip address
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY", // Jito tip address
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49", // Jito tip address
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh", // Jito tip address
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt", // Jito tip address
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL", // Jito tip address
];

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

    /// Check if an address is a known MEV/Jito tip address
    fn is_mev_tip_address(address: &str) -> bool {
        KNOWN_MEV_TIP_ADDRESSES.contains(&address)
    }

    /// Calculate total tip amount from system transfers to MEV addresses
    fn calculate_tip_amount(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> f64 {
        let wallet = self.wallet_pubkey.to_string();
        let mut total_tips = 0.0;
        let mut counted_transfers: HashSet<(String, String, u64)> = HashSet::new();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "TIP_DEBUG",
                &format!("{}: Starting tip detection for wallet {}", &transaction.signature, wallet)
            );
        }

        let message = &tx_data.transaction.message;
        let account_keys = account_keys_from_message(message);

        // Pre-scan to identify accounts that should NOT be considered tips recipients:
        // - Token accounts created/initialized in this tx (likely ATA funding)
        // - Wallet's WSOL ATA (funding WSOL for buys)
        // - Accounts that are SyncNative'd (WSOL native accounts)
        let mut created_token_accounts: HashSet<String> = HashSet::new();
        let mut wallet_wsol_ata: Option<String> = None;
        let mut sync_native_accounts: HashSet<String> = HashSet::new();

        let record_inst_for_exclusions = |
            inst: &Value,
            created_token_accounts: &mut HashSet<String>,
            wallet_wsol_ata: &mut Option<String>,
            sync_native_accounts: &mut HashSet<String>,
            account_keys: &Vec<String>,
            wallet: &str
        | {
            let program_id = inst
                .get("programId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    inst.get("programIdIndex")
                        .and_then(|v| v.as_u64())
                        .and_then(|idx| account_keys.get(idx as usize).cloned())
                })
                .unwrap_or_default();
            if program_id.is_empty() {
                return;
            }
            if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                let itype = parsed
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                // Associated Token Program: capture created accounts and detect wallet WSOL ATA
                if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                    if itype == "create" || itype == "createidempotent" {
                        if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                            if let Some(acc) = info.get("account").and_then(|v| v.as_str()) {
                                created_token_accounts.insert(acc.to_string());
                            }
                            // Detect wallet WSOL ATA
                            let is_wsol = info
                                .get("mint")
                                .and_then(|v| v.as_str())
                                .map(|m| m == WSOL_MINT)
                                .unwrap_or(false);
                            let wallet_matches = info
                                .get("wallet")
                                .and_then(|v| v.as_str())
                                .map(|w| w == wallet)
                                .unwrap_or(false);
                            if is_wsol && wallet_matches {
                                if let Some(acc) = info.get("account").and_then(|v| v.as_str()) {
                                    *wallet_wsol_ata = Some(acc.to_string());
                                }
                            }
                        }
                    }
                }
                // Token Program: capture initializeAccount* and SyncNative
                if program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
                    if
                        itype == "initializeaccount" ||
                        itype == "initializeaccount2" ||
                        itype == "initializeaccount3"
                    {
                        if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                            if let Some(acc) = info.get("account").and_then(|v| v.as_str()) {
                                created_token_accounts.insert(acc.to_string());
                            }
                        }
                    } else if itype == "syncnative" {
                        if
                            let Some(acc) = parsed
                                .get("info")
                                .and_then(|v| v.as_object())
                                .and_then(|i| i.get("account"))
                                .and_then(|v| v.as_str())
                        {
                            sync_native_accounts.insert(acc.to_string());
                        }
                    }
                }
            }
        };

        // Scan outer instructions
        if let Some(outer) = message.get("instructions").and_then(|v| v.as_array()) {
            for inst in outer {
                record_inst_for_exclusions(
                    inst,
                    &mut created_token_accounts,
                    &mut wallet_wsol_ata,
                    &mut sync_native_accounts,
                    &account_keys,
                    &wallet
                );
            }
        }
        // Scan inner instructions
        if let Some(inner) = tx_data.meta.as_ref().and_then(|m| m.inner_instructions.as_ref()) {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for inst in instructions {
                        record_inst_for_exclusions(
                            inst,
                            &mut created_token_accounts,
                            &mut wallet_wsol_ata,
                            &mut sync_native_accounts,
                            &account_keys,
                            &wallet
                        );
                    }
                }
            }
        }

        // Check outer instructions for system transfers to MEV addresses
        if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
            for (idx, instruction) in instructions.iter().enumerate() {
                // Handle both programId (string) and programIdIndex (number) formats like extract_instruction_info does
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
                    .unwrap_or_else(|| "unknown".to_string());

                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "TIP_DEBUG",
                        &format!("Instruction {}: program_id={}", idx, program_id)
                    );
                }

                // Check for system program transfers
                if program_id == "11111111111111111111111111111111" {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "TIP_DEBUG",
                            &format!("System instruction {}: {:?}", idx, instruction)
                        );

                        // Decode accounts and check for MEV transfers
                        if
                            let Some(accounts_array) = instruction
                                .get("accounts")
                                .and_then(|v| v.as_array())
                        {
                            let mut source_account = None;
                            let mut dest_account = None;

                            for (acc_idx, acc_val) in accounts_array.iter().enumerate() {
                                if let Some(account_index) = acc_val.as_u64() {
                                    let account_key = account_keys.get(account_index as usize);
                                    let account_key_str = account_key
                                        .map(|s| s.as_str())
                                        .unwrap_or("unknown");
                                    let is_mev = Self::is_mev_tip_address(account_key_str);

                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "TIP_DEBUG",
                                            &format!(
                                                "  Account {}: index={} key={} is_mev={}",
                                                acc_idx,
                                                account_index,
                                                account_key_str,
                                                is_mev
                                            )
                                        );
                                    }

                                    if acc_idx == 0 {
                                        source_account = account_key.cloned();
                                    } else if acc_idx == 1 {
                                        dest_account = account_key.cloned();
                                    }
                                }
                            }

                            // Check if this is a transfer from wallet to MEV address
                            if let (Some(source), Some(dest)) = (source_account, dest_account) {
                                // Decode lamports amount from raw data when parsed info isn't available
                                let decode_lamports = || -> Option<u64> {
                                    let data_str = instruction
                                        .get("data")
                                        .and_then(|v| v.as_str())?;
                                    let decoded = bs58::decode(data_str).into_vec().ok()?;
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "TIP_DEBUG",
                                            &format!(
                                                "System instruction data: {:?} (len={})",
                                                decoded,
                                                decoded.len()
                                            )
                                        );
                                    }
                                    if decoded.len() >= 12 && decoded[0] == 2 {
                                        let lamports_bytes = &decoded[4..12];
                                        let lamports = u64::from_le_bytes(
                                            lamports_bytes.try_into().unwrap_or([0; 8])
                                        );
                                        Some(lamports)
                                    } else {
                                        None
                                    }
                                };

                                if source == wallet {
                                    if let Some(lamports) = decode_lamports() {
                                        let destination_is_mev = Self::is_mev_tip_address(&dest);
                                        let destination_is_created =
                                            created_token_accounts.contains(&dest);
                                        let destination_is_wallet_wsol_ata = wallet_wsol_ata
                                            .as_deref()
                                            .map(|w| w == dest)
                                            .unwrap_or(false);
                                        let destination_is_syncnative =
                                            sync_native_accounts.contains(&dest);

                                        let transfer_key = (source.clone(), dest.clone(), lamports);
                                        if counted_transfers.insert(transfer_key) {
                                            if
                                                destination_is_mev ||
                                                (!destination_is_created &&
                                                    !destination_is_wallet_wsol_ata &&
                                                    !destination_is_syncnative)
                                            {
                                                let tip_amount = lamports_to_sol(lamports);
                                                total_tips += tip_amount;
                                                if self.debug_enabled {
                                                    log(
                                                        LogTag::Transactions,
                                                        if destination_is_mev {
                                                            "TIP_DETECTED"
                                                        } else {
                                                            "TIP_DETECTED"
                                                        },
                                                        &format!(
                                                            "{}: {} {} SOL ({} lamports) to {} (system instruction {})",
                                                            &transaction.signature,
                                                            if destination_is_mev {
                                                                "MEV tip"
                                                            } else {
                                                                "Priority fee"
                                                            },
                                                            tip_amount,
                                                            lamports,
                                                            &dest,
                                                            idx
                                                        )
                                                    );
                                                }
                                            } else if self.debug_enabled {
                                                log(
                                                    LogTag::Transactions,
                                                    "TIP_DEBUG",
                                                    &format!(
                                                        "Ignoring system transfer in tip calc: dest_created={} dest_wallet_wsol_ata={} dest_syncnative={}",
                                                        destination_is_created,
                                                        destination_is_wallet_wsol_ata,
                                                        destination_is_syncnative
                                                    )
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(parsed) = instruction.get("parsed").and_then(|v| v.as_object()) {
                        if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                            let instruction_type = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();

                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "TIP_DEBUG",
                                    &format!("System instruction type: {}", instruction_type)
                                );
                            }

                            if instruction_type == "transfer" {
                                let source = info
                                    .get("source")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let destination = info
                                    .get("destination")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let lamports = info
                                    .get("lamports")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let source_is_wallet = source == wallet;
                                let destination_is_mev = Self::is_mev_tip_address(destination);
                                let destination_is_created =
                                    created_token_accounts.contains(destination);
                                let destination_is_wallet_wsol_ata = wallet_wsol_ata
                                    .as_deref()
                                    .map(|w| w == destination)
                                    .unwrap_or(false);
                                let destination_is_syncnative =
                                    sync_native_accounts.contains(destination);

                                if self.debug_enabled {
                                    log(
                                        LogTag::Transactions,
                                        "TIP_DEBUG",
                                        &format!(
                                            "Transfer: {} lamports from {} (is_wallet={}) to {} (is_mev={}) excl:created={} wsol_ata={} syncnative={}",
                                            lamports,
                                            source,
                                            source_is_wallet,
                                            destination,
                                            destination_is_mev,
                                            destination_is_created,
                                            destination_is_wallet_wsol_ata,
                                            destination_is_syncnative
                                        )
                                    );
                                }

                                if source_is_wallet && destination_is_mev {
                                    let tip_amount = lamports_to_sol(lamports);
                                    let key = (
                                        source.to_string(),
                                        destination.to_string(),
                                        lamports,
                                    );
                                    if counted_transfers.insert(key) {
                                        total_tips += tip_amount;
                                    }

                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "TIP_DETECTED",
                                            &format!(
                                                "{}: MEV tip {} SOL ({} lamports) to {}",
                                                &transaction.signature,
                                                tip_amount,
                                                lamports,
                                                destination
                                            )
                                        );
                                    }
                                } else if source_is_wallet && lamports > 0 {
                                    // Generalize: count non-MEV System transfers as priority fees when they are not ATA/WSOL funding
                                    if
                                        !destination_is_created &&
                                        !destination_is_wallet_wsol_ata &&
                                        !destination_is_syncnative
                                    {
                                        let tip_amount = lamports_to_sol(lamports);
                                        let key = (
                                            source.to_string(),
                                            destination.to_string(),
                                            lamports,
                                        );
                                        if counted_transfers.insert(key) {
                                            total_tips += tip_amount;
                                        }
                                        if self.debug_enabled {
                                            log(
                                                LogTag::Transactions,
                                                "TIP_DETECTED",
                                                &format!(
                                                    "{}: Priority fee {} SOL ({} lamports) to {} (non-MEV)",
                                                    &transaction.signature,
                                                    tip_amount,
                                                    lamports,
                                                    destination
                                                )
                                            );
                                        }
                                    } else if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "TIP_DEBUG",
                                            &format!(
                                                "Ignoring wallet System transfer as non-tip: dest_created={} dest_wallet_wsol_ata={} dest_syncnative={}",
                                                destination_is_created,
                                                destination_is_wallet_wsol_ata,
                                                destination_is_syncnative
                                            )
                                        );
                                    }
                                } else if source_is_wallet {
                                    // Log all transfers from wallet for investigation
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "TIP_DEBUG",
                                            &format!(
                                                "Wallet transfer: {} lamports to {} (not in MEV list)",
                                                lamports,
                                                destination
                                            )
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "TIP_DEBUG",
                &format!("{}: Total tips detected: {} SOL", &transaction.signature, total_tips)
            );
        }

        total_tips
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
            self.calculate_swap_pnl(&mut transaction, &tx_data).await?;
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
        // Cache-first path: try DB unless force-refresh is requested
        let cache_only = is_cache_only_enabled();
        let force_refresh = is_force_refresh_enabled();

        if !force_refresh {
            if let Some(db) = crate::transactions::database::get_transaction_database().await {
                match db.get_raw_transaction_details(signature).await {
                    Ok(Some(details)) => {
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "DB_HIT",
                                &format!("Raw tx cache hit for {}", signature)
                            );
                        }
                        return Ok(details);
                    }
                    Ok(None) => {
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "DB_MISS",
                                &format!("No cached raw tx for {}", signature)
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!("DB read error for {}: {}", signature, e)
                        );
                    }
                }
            }
        }

        if cache_only {
            return Err(
                format!("Cache-only mode: raw transaction not available in DB for {}", signature)
            );
        }

        // Otherwise fetch from RPC and persist raw blob immediately
        let details = self.fetcher.fetch_transaction_details(signature).await.map_err(|e| {
            if e.contains("not found") || e.contains("no longer available") {
                format!("Transaction not found: {}", signature)
            } else {
                format!("Failed to fetch transaction data: {}", e)
            }
        })?;

        // Persist raw snapshot for future cache hits
        if let Some(db) = crate::transactions::database::get_transaction_database().await {
            // Build a minimal Transaction record just to store raw blob and metadata
            let mut tx = Transaction::new(signature.to_string());
            tx.slot = Some(details.slot);
            tx.block_time = details.block_time;
            tx.timestamp = if let Some(bt) = details.block_time {
                DateTime::<Utc>::from_timestamp(bt, 0).unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };
            let success = details.meta
                .as_ref()
                .map_or(
                    false,
                    |m| (m.err.is_none() || m.err.as_ref().map_or(false, |e| e.is_null()))
                );
            tx.status = if success {
                TransactionStatus::Finalized
            } else {
                TransactionStatus::Failed("Failed".to_string())
            };
            tx.success = success;
            tx.fee_lamports = details.meta.as_ref().map(|m| m.fee);
            tx.instructions_count = details.transaction.message
                .get("instructions")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            tx.accounts_count = details.transaction.message
                .get("accountKeys")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            tx.raw_transaction_data = serde_json::to_value(&details).ok();

            if let Err(e) = db.store_raw_transaction(&tx).await {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to persist raw tx {}: {}", signature, e)
                );
            } else if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "CACHE_STORE",
                    &format!("Stored raw {} to cache", signature)
                );
            }
        }

        Ok(details)
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
            transaction.sol_balance_changes = vec![sol_change.clone()];

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "SOL_BALANCE_DEBUG",
                    &format!(
                        "{}: SOL pre={:.9} post={:.9} change={:.9} lamports_delta={}",
                        &transaction.signature,
                        sol_change.pre_balance,
                        sol_change.post_balance,
                        sol_change.change,
                        lamport_delta
                    )
                );
            }
        } else {
            transaction.sol_balance_changes.clear();
        }

        let token_changes = self.extract_token_balance_changes(tx_data).await?;
        transaction.token_balance_changes = token_changes;

        // Debug log token balance changes
        if self.debug_enabled && !transaction.token_balance_changes.is_empty() {
            for token_change in &transaction.token_balance_changes {
                log(
                    LogTag::Transactions,
                    "TOKEN_BALANCE_DEBUG",
                    &format!(
                        "{}: token={} pre={} post={} change={:.9} decimals={}",
                        &transaction.signature,
                        token_change.mint,
                        token_change.pre_balance
                            .map(|v| format!("{:.9}", v))
                            .unwrap_or("None".to_string()),
                        token_change.post_balance
                            .map(|v| format!("{:.9}", v))
                            .unwrap_or("None".to_string()),
                        token_change.change,
                        token_change.decimals
                    )
                );
            }
        }

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

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "TOKEN_EXTRACT_DEBUG",
                &format!(
                    "Extracting token balances for wallet: {}, pre_count: {}, post_count: {}",
                    wallet_str,
                    meta.pre_token_balances.as_ref().map_or(0, |v| v.len()),
                    meta.post_token_balances.as_ref().map_or(0, |v| v.len())
                )
            );
        }

        if let Some(pre_balances) = &meta.pre_token_balances {
            for balance in pre_balances {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "TOKEN_EXTRACT_DEBUG",
                        &format!(
                            "Pre-balance: mint={}, owner={:?}, amount={:?}",
                            balance.mint,
                            balance.owner,
                            balance.ui_token_amount
                        )
                    );
                }
                if balance.owner.as_deref() == Some(wallet_str.as_str()) {
                    let decimals = balance.ui_token_amount.decimals;
                    let amount = parse_ui_amount(&balance.ui_token_amount);
                    let entry = pre_totals.entry(balance.mint.clone()).or_insert((decimals, 0.0));
                    entry.0 = decimals;
                    entry.1 += amount;

                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "TOKEN_EXTRACT_DEBUG",
                            &format!("Added pre-balance: mint={}, amount={}", balance.mint, amount)
                        );
                    }
                }
            }
        }

        if let Some(post_balances) = &meta.post_token_balances {
            for balance in post_balances {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "TOKEN_EXTRACT_DEBUG",
                        &format!(
                            "Post-balance: mint={}, owner={:?}, amount={:?}",
                            balance.mint,
                            balance.owner,
                            balance.ui_token_amount
                        )
                    );
                }
                if balance.owner.as_deref() == Some(wallet_str.as_str()) {
                    let decimals = balance.ui_token_amount.decimals;
                    let amount = parse_ui_amount(&balance.ui_token_amount);
                    let entry = post_totals.entry(balance.mint.clone()).or_insert((decimals, 0.0));
                    entry.0 = decimals;
                    entry.1 += amount;

                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "TOKEN_EXTRACT_DEBUG",
                            &format!("Added post-balance: mint={}, amount={}", balance.mint, amount)
                        );
                    }
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
                .unwrap_or((pre_decimals, 0.0)); // Default to 0.0 when missing from post-balances
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
        let mut sync_native_accounts: HashSet<String> = HashSet::new();
        // Track which accounts were created by the wallet in this tx (to attribute rent even if not wallet-owned)
        let mut account_creators: HashMap<String, String> = HashMap::new();
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
                            // Capture SyncNative accounts (WSOL native accounts)
                            if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                                let program_id = inst
                                    .get("programId")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if program_id == TOKEN_PROGRAM_ID {
                                    let itype = parsed
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_ascii_lowercase();
                                    if itype == "syncnative" {
                                        if
                                            let Some(acc) = parsed
                                                .get("info")
                                                .and_then(|v| v.as_object())
                                                .and_then(|i| i.get("account"))
                                                .and_then(|v| v.as_str())
                                        {
                                            sync_native_accounts.insert(acc.to_string());
                                        }
                                    }
                                }
                            }
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

        // Track ATAs created (account addresses) via the associated token program in this tx
        let mut ata_created_accounts: HashSet<String> = HashSet::new();
        // Track token accounts created via Token Program initializeAccount* in this tx
        // Some aggregators do allocate+assign+transfer and then initialize via Token program without AToken CPI.
        let mut created_token_accounts: HashSet<String> = HashSet::new();
        let mut record_ata_account = |inst: &Value| {
            if
                let (Some(program_id), Some(parsed)) = (
                    inst.get("programId").and_then(|v| v.as_str()),
                    inst.get("parsed").and_then(|v| v.as_object()),
                )
            {
                if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                    let itype = parsed
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if itype == "create" || itype == "createidempotent" {
                        if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                            if let Some(acc) = info.get("account").and_then(|v| v.as_str()) {
                                ata_created_accounts.insert(acc.to_string());
                            }
                        }
                    }
                }
            }
        };

        for inst in &outer_instructions {
            record_ata_account(inst);
        }
        if let Some(inner) = &meta.inner_instructions {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for inst in instructions {
                        record_ata_account(inst);
                    }
                }
            }
        }

        // Also capture SyncNative accounts from outer instructions
        for inst in &outer_instructions {
            if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                let program_id = inst
                    .get("programId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if program_id == TOKEN_PROGRAM_ID {
                    let itype = parsed
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if itype == "syncnative" {
                        if
                            let Some(acc) = parsed
                                .get("info")
                                .and_then(|v| v.as_object())
                                .and_then(|i| i.get("account"))
                                .and_then(|v| v.as_str())
                        {
                            sync_native_accounts.insert(acc.to_string());
                        }
                    }
                }
            }
        }

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
                            // Record token accounts initialized in this tx (may be ATAs or regular token accounts)
                            created_token_accounts.insert(account.to_string());
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
                                    } else if pre == 0 && post == 0 {
                                        // Created and closed within same transaction: rebate equals previously spent rent
                                        if let Some(spent) = account_rent_spent.get(account) {
                                            if *spent > 0 {
                                                account_rent_recovered.insert(
                                                    account.to_string(),
                                                    *spent
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // SystemProgram createAccount often precedes token account initialization; capture rent lamports
            if program_id == "11111111111111111111111111111111" {
                if instruction_type == "createaccount" {
                    if let Some(lamports_val) = info.get("lamports").and_then(|v| v.as_u64()) {
                        if let Some(new_acc) = info.get("newAccount").and_then(|v| v.as_str()) {
                            // Attribute rent to the created account; will be classified as WSOL via SyncNative later if applicable
                            if !new_acc.is_empty() && lamports_val > 0 {
                                account_rent_spent
                                    .entry(new_acc.to_string())
                                    .and_modify(|v| {
                                        *v += lamports_val;
                                    })
                                    .or_insert(lamports_val);
                                if let Some(source) = info.get("source").and_then(|v| v.as_str()) {
                                    account_creators.insert(
                                        new_acc.to_string(),
                                        source.to_string()
                                    );
                                }
                                if self.debug_enabled {
                                    log(
                                        LogTag::Transactions,
                                        "ATA_DEBUG",
                                        &format!(
                                            "{}: System createAccount -> new={} lamports={} source={}",
                                            &transaction.signature,
                                            new_acc,
                                            lamports_val,
                                            info
                                                .get("source")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("?")
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
                // Also capture direct system transfers that fund newly created ATAs (allocate/assign + transfer path)
                if instruction_type == "transfer" {
                    if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                        let dest = info.get("destination").and_then(|v| v.as_str());
                        let src = info.get("source").and_then(|v| v.as_str());
                        let lamports_opt = info.get("lamports").and_then(|v| v.as_u64());
                        if let (Some(dest), Some(src), Some(lamports)) = (dest, src, lamports_opt) {
                            // Consider both ATAs created via AToken program and token accounts initialized via Token program
                            if
                                (ata_created_accounts.contains(dest) ||
                                    created_token_accounts.contains(dest)) &&
                                lamports > 0
                            {
                                account_rent_spent
                                    .entry(dest.to_string())
                                    .and_modify(|v| {
                                        *v += lamports;
                                    })
                                    .or_insert(lamports);
                                account_creators.insert(dest.to_string(), src.to_string());
                                if self.debug_enabled {
                                    log(
                                        LogTag::Transactions,
                                        "ATA_DEBUG",
                                        &format!(
                                            "{}: System transfer funding new token account -> dest={} lamports={} src={} (dest_is_created_in_tx={})",
                                            &transaction.signature,
                                            dest,
                                            lamports,
                                            src,
                                            true
                                        )
                                    );
                                }
                            }
                        }
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

        // Fallback: if we detected ATAs created in this tx but didn't attribute their rent yet,
        // scan for any SystemProgram transfers from the wallet to those ATA accounts and record them.
        // This covers allocate+assign+transfer -> initializeAccount patterns where ordering or CPI parsing
        // caused us to miss the funding transfer in the first pass.
        // Use union of ATA-created and Token-program-initialized accounts for fallback detection
        if !ata_created_accounts.is_empty() || !created_token_accounts.is_empty() {
            let mut ensure_rent_for_atas = |inst: &Value| {
                let program_id = inst
                    .get("programId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        inst.get("programIdIndex")
                            .and_then(|v| v.as_u64())
                            .and_then(|idx| account_keys.get(idx as usize).cloned())
                    })
                    .unwrap_or_default();
                if program_id != "11111111111111111111111111111111" {
                    return;
                }
                if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                    let itype = parsed
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if itype != "transfer" {
                        return;
                    }
                    if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                        let dest = info.get("destination").and_then(|v| v.as_str());
                        let src = info.get("source").and_then(|v| v.as_str());
                        let lamports_opt = info.get("lamports").and_then(|v| v.as_u64());
                        if let (Some(dest), Some(src), Some(lamports)) = (dest, src, lamports_opt) {
                            let created_in_tx =
                                ata_created_accounts.contains(dest) ||
                                created_token_accounts.contains(dest);
                            if src == wallet && created_in_tx && lamports > 0 {
                                if !account_rent_spent.contains_key(dest) {
                                    account_rent_spent.insert(dest.to_string(), lamports);
                                    account_creators.insert(dest.to_string(), src.to_string());
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "ATA_DEBUG",
                                            &format!(
                                                "{}: FALLBACK transfer funding new token account -> dest={} lamports={} src={} (dest_is_created_in_tx={})",
                                                &transaction.signature,
                                                dest,
                                                lamports,
                                                src,
                                                true
                                            )
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            };

            // Outer first
            for inst in &outer_instructions {
                ensure_rent_for_atas(inst);
            }
            // Then inner
            if let Some(inner) = &meta.inner_instructions {
                for entry in inner {
                    if
                        let Some(instructions) = entry
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for inst in instructions {
                            ensure_rent_for_atas(inst);
                        }
                    }
                }
            }
        }

        // Pure rent detection based on lamport deltas for wallet-owned token accounts
        for (account, owner) in account_owners.iter() {
            if owner != &wallet {
                continue;
            }
            if let Some(index) = account_index_map.get(account) {
                let pre = meta.pre_balances.get(*index).copied().unwrap_or(0);
                let post = meta.post_balances.get(*index).copied().unwrap_or(0);
                if post > pre {
                    account_rent_spent
                        .entry(account.clone())
                        .and_modify(|v| {
                            *v += post - pre;
                        })
                        .or_insert(post - pre);
                } else if pre > post {
                    account_rent_recovered
                        .entry(account.clone())
                        .and_modify(|v| {
                            *v += pre - post;
                        })
                        .or_insert(pre - post);
                }
            }
        }

        let mut operations = Vec::new();

        // Record ATA creations from rent spent
        for (account, lamports) in account_rent_spent.iter() {
            if *lamports == 0 {
                continue;
            }
            let owner_matches_wallet = account_owners
                .get(account)
                .map(|owner| owner == &wallet)
                .unwrap_or(false);
            // Include ephemeral WSOL/native accounts or accounts later closed to wallet even if owner not recorded
            let later_closed_to_wallet = account_rent_recovered.contains_key(account);
            let created_by_wallet = account_creators
                .get(account)
                .map(|s| s == &wallet)
                .unwrap_or(false);
            let mut include = owner_matches_wallet || later_closed_to_wallet || created_by_wallet;

            let mut mint = account_mints
                .get(account)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            // If we saw SyncNative on this account, it's WSOL even if mint wasn't recorded
            if mint == "unknown" && sync_native_accounts.contains(account) {
                mint = WSOL_MINT.to_string();
                include = true; // sync-native implies wallet created native account for wrapping
            }
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "ATA_DEBUG",
                    &format!(
                        "{}: RENT_SPENT account={} lamports={} owner_matches_wallet={} created_by_wallet={} later_closed_to_wallet={} mint={} include={}",
                        &transaction.signature,
                        account,
                        lamports,
                        owner_matches_wallet,
                        created_by_wallet,
                        later_closed_to_wallet,
                        mint,
                        include
                    )
                );
            }
            if !include {
                continue;
            }
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

        // Record ATA closures from rent recovered
        for (account, lamports) in account_rent_recovered.iter() {
            if *lamports == 0 {
                continue;
            }
            // Destination already verified to be wallet when inserting into account_rent_recovered,
            // so record closure regardless of whether owner mapping exists.

            let mut mint = account_mints
                .get(account)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            // If we saw SyncNative on this account, it's WSOL even if mint wasn't recorded
            if mint == "unknown" && sync_native_accounts.contains(account) {
                mint = WSOL_MINT.to_string();
            }
            let rent_sol = lamports_to_sol(*lamports);
            let is_wsol = mint == WSOL_MINT;

            let mint_for_log = mint.clone();
            operations.push(AtaOperation {
                operation_type: AtaOperationType::Closure,
                account_address: account.clone(),
                token_mint: mint.clone(),
                rent_amount: rent_sol,
                is_wsol,
                mint,
                rent_cost_sol: Some(rent_sol),
            });
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "ATA_DEBUG",
                    &format!(
                        "{}: RENT_RECOVERED account={} lamports={} mint={} is_wsol={} (destination was wallet)",
                        &transaction.signature,
                        account,
                        lamports,
                        mint_for_log,
                        is_wsol
                    )
                );
            }
        }

        // Enhanced fallback ATA detection for missed closures
        // Look for accounts that decreased to zero but weren't detected as closures
        const STANDARD_ATA_RENT: u64 = 2_039_280;
        let tolerance = STANDARD_ATA_RENT / 20; // 5% tolerance

        for (account, &pre_balance) in meta.pre_balances.iter().enumerate() {
            let post_balance = meta.post_balances.get(account).copied().unwrap_or(0);

            // Look for accounts that went to zero (closed) with ATA-sized rent
            if pre_balance > 0 && post_balance == 0 {
                // Check if this looks like ATA rent (close to standard amount)
                let diff = if
                    pre_balance >= STANDARD_ATA_RENT - tolerance &&
                    pre_balance <= STANDARD_ATA_RENT + tolerance
                {
                    pre_balance
                } else {
                    continue;
                };

                let account_address = format!("account_index_{}", account);

                // Only add if we haven't already detected this closure and it's not already in our operations
                let already_detected_as_operation = operations.iter().any(|op| {
                    op.operation_type == AtaOperationType::Closure &&
                        (op.rent_amount - lamports_to_sol(diff)).abs() < 0.0001 // Within tolerance
                });

                if
                    !account_rent_recovered.contains_key(&account_address) &&
                    !already_detected_as_operation
                {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "ATA_FALLBACK_DETECT",
                            &format!(
                                "{}: Detected missed ATA closure at index {} with {} lamports (close to standard ATA rent)",
                                &transaction.signature,
                                account,
                                pre_balance
                            )
                        );
                    }

                    // Create a synthetic closure entry for the missed rent recovery
                    operations.push(AtaOperation {
                        operation_type: AtaOperationType::Closure,
                        account_address: account_address,
                        token_mint: "unknown_token".to_string(),
                        rent_amount: lamports_to_sol(diff),
                        is_wsol: false,
                        mint: "unknown_token".to_string(),
                        rent_cost_sol: Some(lamports_to_sol(diff)),
                    });
                }
            }
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
            // Also log a compact summary of rent maps for troubleshooting
            if !account_rent_spent.is_empty() {
                for (acc, lam) in &account_rent_spent {
                    let owner = account_owners.get(acc).cloned().unwrap_or("?".to_string());
                    let creator = account_creators.get(acc).cloned().unwrap_or("?".to_string());
                    let mint = account_mints.get(acc).cloned().unwrap_or("?".to_string());
                    log(
                        LogTag::Transactions,
                        "ATA_SUMMARY",
                        &format!(
                            "{}: SPENT acc={} lamports={} owner={} creator={} mint={}",
                            &transaction.signature,
                            acc,
                            lam,
                            owner,
                            creator,
                            mint
                        )
                    );
                }
            }
            if !account_rent_recovered.is_empty() {
                for (acc, lam) in &account_rent_recovered {
                    let owner = account_owners.get(acc).cloned().unwrap_or("?".to_string());
                    let mint = account_mints.get(acc).cloned().unwrap_or("?".to_string());
                    log(
                        LogTag::Transactions,
                        "ATA_SUMMARY",
                        &format!(
                            "{}: RECOVERED acc={} lamports={} owner={} mint={}",
                            &transaction.signature,
                            acc,
                            lam,
                            owner,
                            mint
                        )
                    );
                }
            }
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
    async fn calculate_swap_pnl(
        &self,
        transaction: &mut Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<(), String> {
        // Extract swap information first
        if let Some(swap_info) = self.extract_swap_info(transaction, tx_data).await? {
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
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<Option<TokenSwapInfo>, String> {
        let epsilon = 1e-9;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "EXTRACT_SWAP_ENTRY",
                &format!(
                    "{}: Extracting swap info, token_balance_changes={}, sol_change={:.9}",
                    &transaction.signature,
                    transaction.token_balance_changes.len(),
                    transaction.sol_balance_change
                )
            );
        }

        if transaction.token_balance_changes.is_empty() {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "EXTRACT_SWAP_SKIP",
                    &format!("{}: No token balance changes", &transaction.signature)
                );
            }
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

        // Handle small sells where fee exceeds the swap amount
        // For very small swaps, the net SOL change can be negative due to fees
        // but we can still detect it as a sell if tokens went out and the net loss is small
        let is_small_sell =
            !is_buy &&
            !is_sell &&
            sol_change < 0.0 &&
            primary_change.change < -epsilon &&
            sol_change.abs() < 0.01; // Net loss is less than 0.01 SOL (could be fee-dominated small sell)

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "SWAP_DETECTION_DEBUG",
                &format!(
                    "Swap detection: sol_change={:.9}, token_change={:.9}, is_buy={}, is_sell={}, is_small_sell={}, epsilon={:.9}",
                    sol_change,
                    primary_change.change,
                    is_buy,
                    is_sell,
                    is_small_sell,
                    epsilon
                )
            );
        }

        if !is_buy && !is_sell && !is_small_sell {
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

        // Default token amount as the net delta observed in balances
        let mut token_amount_ui = primary_change.change.abs();

        // CSV-alignment tweak for sells routed via Jupiter/Pumpfun:
        // In some edge cases where the wallet effectively drains the token ATA
        // and leaves only tiny “dust”, Solscan CSV reports Token1 amount as the
        // entire pre-balance rather than the net transferred amount. To match
        // those rows without affecting normal sells, detect a near-drain and
        // use the pre-balance as the token input amount.
        if token_amount_ui > epsilon {
            if let Some(router_str) = Some(Self::infer_swap_router(transaction)) {
                let router_lc = router_str.to_ascii_lowercase();
                let is_router_jup_or_pump = router_lc == "jupiter" || router_lc == "pumpfun";
                if is_router_jup_or_pump && sol_change > epsilon {
                    if
                        let (Some(pre_bal), Some(post_bal)) = (
                            primary_change.pre_balance,
                            primary_change.post_balance,
                        )
                    {
                        // Treat as a near-drain if the residual post balance is <0.1% of pre (very small dust)
                        if pre_bal > 0.0 && post_bal > 0.0 && post_bal / pre_bal < 0.001 {
                            token_amount_ui = pre_bal;
                        }
                    }
                }
            }
        }
        if token_amount_ui <= epsilon {
            return Ok(None);
        }

        let fee_sol = transaction.fee_sol;
        let mut tip_amount = self.calculate_tip_amount(transaction, tx_data);

        // TEMPORARY WORKAROUND: If no tips detected but we have a common Jito tip amount difference
        // This handles cases where the tip address isn't in our known list
        if tip_amount == 0.0 {
            let total_sol_spent = (-sol_change).max(0.0);
            let sol_spent_for_tokens_raw = (total_sol_spent - fee_sol).max(0.0);
            let sol_spent_lamports = (sol_spent_for_tokens_raw * 1_000_000_000.0) as u64;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "TIP_HEURISTIC",
                    &format!(
                        "Heuristic check: total_sol_spent={:.9} fee_sol={:.9} raw={:.9} lamports={}",
                        total_sol_spent,
                        fee_sol,
                        sol_spent_for_tokens_raw,
                        sol_spent_lamports
                    )
                );
            }

            // Common Jito tip amounts to check for and subtract
            let common_tip_amounts = [50_000, 100_000, 150_000]; // lamports

            for &tip_lamports in &common_tip_amounts {
                let adjusted_lamports = sol_spent_lamports.saturating_sub(tip_lamports);
                let adjusted_sol = (adjusted_lamports as f64) / 1_000_000_000.0;

                // Check if removing this tip amount results in a round number that's more likely to be intentional
                let remainder = adjusted_lamports % 1_000_000; // Check if close to increments of 0.001 SOL
                if remainder < 10_000 {
                    // Within 0.00001 SOL tolerance
                    tip_amount = (tip_lamports as f64) / 1_000_000_000.0;
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "TIP_HEURISTIC",
                            &format!(
                                "{}: Detected likely Jito tip via heuristic: {} SOL ({} lamports)",
                                &transaction.signature,
                                tip_amount,
                                tip_lamports
                            )
                        );
                    }
                    break;
                }
            }
        }

        let total_sol_spent = (-sol_change).max(0.0);
        let sol_spent_for_tokens = (total_sol_spent - fee_sol - tip_amount).max(0.0);

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "SWAP_CALC_DEBUG",
                &format!(
                    "Swap calculation: total_spent={:.9} fee={:.9} tip={:.9} for_tokens={:.9}",
                    total_sol_spent,
                    fee_sol,
                    tip_amount,
                    sol_spent_for_tokens
                )
            );
        }

        let router = Self::infer_swap_router(transaction);

        if self.debug_enabled {
            log(LogTag::Transactions, "SWAP_CALC_DEBUG", &format!("Router detected: {}", router));
        }

        let mut sol_spent_effective = if sol_spent_for_tokens > epsilon {
            sol_spent_for_tokens
        } else {
            (total_sol_spent - tip_amount).max(0.0)
        };

        let net_sol_received = sol_change.max(0.0);
        let mut sol_received_from_swap = if is_small_sell && sol_change < 0.0 {
            // For small sells where fee exceeds swap proceeds, calculate actual swap proceeds
            // Swap proceeds = Fee + Net SOL change (which is negative)
            (fee_sol + sol_change).max(0.0)
        } else {
            (net_sol_received + fee_sol).max(0.0)
        };

        // Precise WSOL extraction overrides (attempt for all routers)
        {
            if is_buy {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "TIP_ADJUSTMENT",
                        "Checking WSOL transfer amount detection..."
                    );
                }
                if let Some(mut exact) = self.detect_wallet_wsol_transfer_amount(transaction) {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "TIP_ADJUSTMENT",
                            &format!(
                                "{}: WSOL exact amount detected: {:.9} SOL ({} lamports)",
                                &transaction.signature,
                                exact,
                                (exact * 1_000_000_000.0) as u64
                            )
                        );
                    }

                    // Post-process to remove likely Jito tips that weren't caught by the main detection
                    let exact_lamports = (exact * 1_000_000_000.0) as u64;

                    // Check if removing common tip amounts results in a rounder number
                    let common_tips = [50_000, 100_000, 150_000];
                    for &tip in &common_tips {
                        let adjusted = exact_lamports.saturating_sub(tip);
                        // Check if the adjusted amount is a round increment of 0.001 SOL (1,000,000 lamports)
                        if adjusted > 0 && adjusted % 1_000_000 == 0 {
                            exact = (adjusted as f64) / 1_000_000_000.0;
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "TIP_ADJUSTMENT",
                                    &format!(
                                        "{}: Adjusted WSOL amount from {:.9} to {:.9} SOL (removed {} lamport tip)",
                                        &transaction.signature,
                                        (exact_lamports as f64) / 1_000_000_000.0,
                                        exact,
                                        tip
                                    )
                                );
                            }
                            break;
                        }
                    }
                    sol_spent_effective = exact;
                }
            } else if is_sell {
                if let Some(exact) = self.detect_wallet_wsol_receive_amount(transaction) {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "SWAP_CALC_DEBUG",
                            &format!(
                                "{}: Detected exact WSOL receive amount: {:.9} SOL ({} lamports)",
                                &transaction.signature,
                                exact,
                                (exact * 1_000_000_000.0) as u64
                            )
                        );
                    }
                    sol_received_from_swap = exact;
                } else if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "SWAP_CALC_DEBUG",
                        &format!(
                            "{}: No exact WSOL receive amount detected; using net_sol+fee path",
                            &transaction.signature
                        )
                    );
                }
            }
        }

        // Note: ATA rent recovery is already included in sol_change, so no need to add it again
        // The sol_change represents the net balance change which includes ATA rent recovery

        // If we couldn't detect the precise WSOL input (fallback path), remove TOTAL ATA rent impact from buy input
        if is_buy {
            // Do not adjust buys using ata_analysis (that over-corrects common WSOL-funded flows).
            // We'll use a conservative fallback only when ata_analysis is None further below.
            if transaction.ata_analysis.is_some() && self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "ATA_RENT_ADJUST",
                    &format!(
                        "{}: ATA analysis present; skipping buy-side rent adjustment (handled by core calc)",
                        &transaction.signature
                    )
                );
            }

            // Fallback for buys: if ATA analysis is missing, detect rent funding from System transfers
            if transaction.ata_analysis.is_none() {
                if let Some(meta) = tx_data.meta.as_ref() {
                    // Build account keys list (strings or { pubkey })
                    let account_keys: Vec<String> = tx_data.transaction.message
                        .get("accountKeys")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|k| {
                                    if let Some(s) = k.as_str() {
                                        Some(s.to_string())
                                    } else {
                                        k.get("pubkey")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string())
                                    }
                                })
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();

                    // Gather created accounts (AToken create* and Token initializeAccount*) and their owners
                    let mut created_in_tx: std::collections::HashSet<String> = std::collections::HashSet::new();
                    let mut created_owners: std::collections::HashMap<
                        String,
                        String
                    > = std::collections::HashMap::new();
                    let wallet_str = self.wallet_pubkey.to_string();
                    let mut record_created = |inst: &serde_json::Value| {
                        // Resolve program id from programId or programIdIndex
                        let mut program_id: Option<String> = inst
                            .get("programId")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if program_id.is_none() {
                            if let Some(idx) = inst.get("programIdIndex").and_then(|v| v.as_u64()) {
                                program_id = account_keys.get(idx as usize).cloned();
                            }
                        }
                        let program_id = program_id.as_deref().unwrap_or("");
                        if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                            let itype = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                                if itype == "create" || itype == "createidempotent" {
                                    if
                                        let Some(info) = parsed
                                            .get("info")
                                            .and_then(|i| i.as_object())
                                    {
                                        if
                                            let Some(acc) = info
                                                .get("account")
                                                .and_then(|v| v.as_str())
                                        {
                                            created_in_tx.insert(acc.to_string());
                                            if
                                                let Some(owner) = info
                                                    .get("wallet")
                                                    .and_then(|v| v.as_str())
                                            {
                                                created_owners.insert(
                                                    acc.to_string(),
                                                    owner.to_string()
                                                );
                                            }
                                        }
                                    }
                                }
                            } else if program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
                                if
                                    itype == "initializeaccount" ||
                                    itype == "initializeaccount2" ||
                                    itype == "initializeaccount3"
                                {
                                    if
                                        let Some(info) = parsed
                                            .get("info")
                                            .and_then(|i| i.as_object())
                                    {
                                        if
                                            let Some(acc) = info
                                                .get("account")
                                                .and_then(|v| v.as_str())
                                        {
                                            created_in_tx.insert(acc.to_string());
                                            if
                                                let Some(owner) = info
                                                    .get("owner")
                                                    .and_then(|v| v.as_str())
                                            {
                                                created_owners.insert(
                                                    acc.to_string(),
                                                    owner.to_string()
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    };

                    if
                        let Some(arr) = tx_data.transaction.message
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for inst in arr {
                            record_created(inst);
                        }
                    }
                    if let Some(meta_inst) = meta.inner_instructions.as_ref() {
                        for inner in meta_inst {
                            if
                                let Some(instructions) = inner
                                    .get("instructions")
                                    .and_then(|v| v.as_array())
                            {
                                for inst in instructions {
                                    record_created(inst);
                                }
                            }
                        }
                    }

                    // Derive candidate accounts (non-wallet owners) and detect funding system transfers/createAccount from wallet
                    // Build owners_by_key from both pre and post token balances
                    let owners_by_key: std::collections::HashMap<String, String> = {
                        let mut map = std::collections::HashMap::new();
                        if let Some(pre_toks) = &meta.pre_token_balances {
                            for tb in pre_toks {
                                if let Some(owner) = tb.owner.as_ref() {
                                    let idx = tb.account_index as usize;
                                    if let Some(k) = account_keys.get(idx) {
                                        map.insert(k.clone(), owner.clone());
                                    }
                                }
                            }
                        }
                        if let Some(post_toks) = &meta.post_token_balances {
                            for tb in post_toks {
                                if let Some(owner) = tb.owner.as_ref() {
                                    let idx = tb.account_index as usize;
                                    if let Some(k) = account_keys.get(idx) {
                                        map.insert(k.clone(), owner.clone());
                                    }
                                }
                            }
                        }
                        map
                    };

                    // Build candidate set: created accounts not owned by wallet
                    let mut candidate_accounts: std::collections::HashSet<String> = created_in_tx
                        .into_iter()
                        .filter(|acc| {
                            let owner = created_owners
                                .get(acc)
                                .or_else(|| owners_by_key.get(acc))
                                .cloned()
                                .unwrap_or_default();
                            !owner.is_empty() && owner != wallet_str
                        })
                        .collect();

                    // If none, derive via balance heuristic (pre=0, post≈rent, owner != wallet)
                    if candidate_accounts.is_empty() {
                        let standard: u64 = 2_039_280;
                        let tol: u64 = standard / 20; // 5%
                        let mut derived = std::collections::HashSet::new();
                        for (idx, pre) in meta.pre_balances.iter().enumerate() {
                            let post = *meta.post_balances.get(idx).unwrap_or(&0);
                            if *pre == 0 && post >= standard - tol && post <= standard + tol {
                                if let Some(key) = account_keys.get(idx) {
                                    let owner = owners_by_key.get(key).cloned().unwrap_or_default();
                                    if owner != wallet_str {
                                        derived.insert(key.clone());
                                    }
                                }
                            }
                        }
                        if self.debug_enabled && !derived.is_empty() {
                            let mut details: Vec<String> = derived
                                .iter()
                                .map(|k| {
                                    let owner = owners_by_key
                                        .get(k)
                                        .cloned()
                                        .unwrap_or_else(|| "?".to_string());
                                    format!("{}->{}", k, owner)
                                })
                                .collect();
                            details.sort();
                            log(
                                LogTag::Transactions,
                                "ATA_RENT_ADJUST",
                                &format!(
                                    "{}: BUY fallback candidates (derived from balances): {}",
                                    &transaction.signature,
                                    details.join(", ")
                                )
                            );
                        }
                        candidate_accounts.extend(derived.into_iter());
                    }

                    // Detect wallet funding to these candidates via System transfer/createAccount and subtract
                    let mut rent_lamports: u64 = 0;
                    let mut handle_system_inst = |inst: &serde_json::Value| {
                        // Resolve program id
                        let mut program_id: Option<String> = inst
                            .get("programId")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if program_id.is_none() {
                            if let Some(idx) = inst.get("programIdIndex").and_then(|v| v.as_u64()) {
                                program_id = account_keys.get(idx as usize).cloned();
                            }
                        }
                        let program_id = program_id.as_deref().unwrap_or("");
                        if program_id != "11111111111111111111111111111111" {
                            return;
                        }
                        if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                            let itype = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            if itype == "transfer" {
                                if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                                    let src = info.get("source").and_then(|v| v.as_str());
                                    let dest = info.get("destination").and_then(|v| v.as_str());
                                    let lam = info
                                        .get("lamports")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    if let (Some(s), Some(d)) = (src, dest) {
                                        if s == wallet_str && candidate_accounts.contains(d) {
                                            rent_lamports = rent_lamports.saturating_add(lam);
                                        }
                                    }
                                }
                            } else if itype == "createaccount" {
                                if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                                    let src = info.get("source").and_then(|v| v.as_str());
                                    let new_acc = info.get("newAccount").and_then(|v| v.as_str());
                                    let lam = info
                                        .get("lamports")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                    if let (Some(s), Some(d)) = (src, new_acc) {
                                        if s == wallet_str && candidate_accounts.contains(d) {
                                            rent_lamports = rent_lamports.saturating_add(lam);
                                        }
                                    }
                                }
                            }
                        }
                    };

                    if
                        let Some(arr) = tx_data.transaction.message
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for inst in arr {
                            handle_system_inst(inst);
                        }
                    }
                    if let Some(meta_inst) = meta.inner_instructions.as_ref() {
                        for inner in meta_inst {
                            if
                                let Some(instructions) = inner
                                    .get("instructions")
                                    .and_then(|v| v.as_array())
                            {
                                for inst in instructions {
                                    handle_system_inst(inst);
                                }
                            }
                        }
                    }

                    // If still zero, fallback to balance check over candidates
                    if rent_lamports == 0 {
                        let standard: u64 = 2_039_280;
                        let tol: u64 = standard / 20; // 5%
                        for (idx, pre) in meta.pre_balances.iter().enumerate() {
                            if *pre != 0 {
                                continue;
                            }
                            let post = *meta.post_balances.get(idx).unwrap_or(&0);
                            if post == 0 {
                                continue;
                            }
                            if let Some(key) = account_keys.get(idx) {
                                if
                                    candidate_accounts.contains(key) &&
                                    post >= standard - tol &&
                                    post <= standard + tol
                                {
                                    rent_lamports = rent_lamports.saturating_add(post);
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "ATA_RENT_ADJUST",
                                            &format!(
                                                "{}: BUY fallback balance-check: acc={} idx={} pre={} post={} looks_like_rent={}",
                                                &transaction.signature,
                                                key,
                                                idx,
                                                0,
                                                post,
                                                true
                                            )
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if rent_lamports > 0 {
                        let rent_sol = (rent_lamports as f64) / 1_000_000_000.0;
                        let before = sol_spent_effective;
                        sol_spent_effective = (sol_spent_effective - rent_sol).max(0.0);
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "ATA_RENT_ADJUST",
                                &format!(
                                    "{}: BUY rent fallback adjust: total_spent={:.9} -> {:.9} (rent_spent_detected={:.9} SOL)",
                                    &transaction.signature,
                                    before,
                                    sol_spent_effective,
                                    rent_sol
                                )
                            );
                        }
                    }
                }
            }
        } else if is_sell {
            // For sells, amount2 should be the pure SOL output (swap proceeds + fee), excluding any rent recovered and including any rent spent.
            if let Some(ata) = transaction.ata_analysis.as_ref() {
                let rent_adjustment = ata.total_rent_spent - ata.total_rent_recovered; // spent minus recovered
                if rent_adjustment.abs() > epsilon {
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "ATA_RENT_ADJUST",
                            &format!(
                                "{}: SELL rent adjust: output={:.9} -> {:.9} (spent={:.9}, recovered={:.9})",
                                &transaction.signature,
                                sol_received_from_swap,
                                (sol_received_from_swap + rent_adjustment).max(0.0),
                                ata.total_rent_spent,
                                ata.total_rent_recovered
                            )
                        );
                    }
                    sol_received_from_swap = (sol_received_from_swap + rent_adjustment).max(0.0);
                }
            } else if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "ATA_RENT_ADJUST",
                    &format!(
                        "{}: No ATA analysis available; skipping rent adjustment",
                        &transaction.signature
                    )
                );
            }

            // Fallback: if ATA analysis is missing, detect rent funding transfers to newly created token accounts in this tx.
            if transaction.ata_analysis.is_none() {
                if let Some(meta) = tx_data.meta.as_ref() {
                    // Build account keys, supporting both string and { pubkey } object forms
                    let account_keys: Vec<String> = tx_data.transaction.message
                        .get("accountKeys")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|k| {
                                    if let Some(s) = k.as_str() {
                                        Some(s.to_string())
                                    } else {
                                        k.get("pubkey")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string())
                                    }
                                })
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();

                    // Collect created token accounts from AToken and Token Program initializeAccount*
                    // Track both the account pubkey and its intended owner to avoid counting wallet-owned accounts (e.g., WSOL ATA)
                    let mut created_in_tx: std::collections::HashSet<String> = std::collections::HashSet::new();
                    let mut created_owners: std::collections::HashMap<
                        String,
                        String
                    > = std::collections::HashMap::new();
                    let wallet_str = self.wallet_pubkey.to_string();
                    let mut record_created = |inst: &serde_json::Value| {
                        // Resolve program id from either programId or programIdIndex
                        let mut program_id: Option<String> = inst
                            .get("programId")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if program_id.is_none() {
                            if let Some(idx) = inst.get("programIdIndex").and_then(|v| v.as_u64()) {
                                program_id = account_keys.get(idx as usize).cloned();
                            }
                        }
                        let program_id = program_id.as_deref().unwrap_or("");
                        if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                            let itype = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                                if itype == "create" || itype == "createidempotent" {
                                    if
                                        let Some(info) = parsed
                                            .get("info")
                                            .and_then(|i| i.as_object())
                                    {
                                        if
                                            let Some(acc) = info
                                                .get("account")
                                                .and_then(|v| v.as_str())
                                        {
                                            created_in_tx.insert(acc.to_string());
                                            if
                                                let Some(owner) = info
                                                    .get("wallet")
                                                    .and_then(|v| v.as_str())
                                            {
                                                created_owners.insert(
                                                    acc.to_string(),
                                                    owner.to_string()
                                                );
                                            }
                                        }
                                    }
                                }
                            } else if program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
                                if
                                    itype == "initializeaccount" ||
                                    itype == "initializeaccount2" ||
                                    itype == "initializeaccount3"
                                {
                                    if
                                        let Some(info) = parsed
                                            .get("info")
                                            .and_then(|i| i.as_object())
                                    {
                                        if
                                            let Some(acc) = info
                                                .get("account")
                                                .and_then(|v| v.as_str())
                                        {
                                            created_in_tx.insert(acc.to_string());
                                            if
                                                let Some(owner) = info
                                                    .get("owner")
                                                    .and_then(|v| v.as_str())
                                            {
                                                created_owners.insert(
                                                    acc.to_string(),
                                                    owner.to_string()
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    };

                    // Outer instructions
                    if
                        let Some(outer) = tx_data.transaction.message
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for inst in outer {
                            record_created(inst);
                        }
                    }
                    // Inner instructions
                    if let Some(inner) = &meta.inner_instructions {
                        for entry in inner {
                            if
                                let Some(instructions) = entry
                                    .get("instructions")
                                    .and_then(|v| v.as_array())
                            {
                                for inst in instructions {
                                    record_created(inst);
                                }
                            }
                        }
                    }

                    // Build owners map from token balance metadata to help filter wallet-owned accounts
                    let mut owners_by_key: std::collections::HashMap<
                        String,
                        String
                    > = std::collections::HashMap::new();
                    if let Some(pre_toks) = &meta.pre_token_balances {
                        for tb in pre_toks {
                            if let Some(owner) = tb.owner.as_ref() {
                                let idx = tb.account_index as usize;
                                if let Some(key) = account_keys.get(idx) {
                                    owners_by_key.insert(key.clone(), owner.clone());
                                }
                            }
                        }
                    }
                    if let Some(post_toks) = &meta.post_token_balances {
                        for tb in post_toks {
                            if let Some(owner) = tb.owner.as_ref() {
                                let idx = tb.account_index as usize;
                                if let Some(key) = account_keys.get(idx) {
                                    owners_by_key.insert(key.clone(), owner.clone());
                                }
                            }
                        }
                    }

                    // Seed candidate accounts with any we explicitly detected via parsed instructions
                    let mut candidate_accounts: std::collections::HashSet<String> = created_in_tx.clone();

                    // If none were detected via parsed instructions, derive candidates via balance heuristics
                    if candidate_accounts.is_empty() {
                        const STANDARD_ATA_RENT: u64 = 2_039_280;
                        let tolerance = STANDARD_ATA_RENT / 20; // 5%
                        for (idx, key) in account_keys.iter().enumerate() {
                            // Skip obvious non-candidates
                            if key == &wallet_str {
                                continue;
                            }
                            if
                                key == "11111111111111111111111111111111" ||
                                key == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" ||
                                key == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" ||
                                key == "ComputeBudget111111111111111111111111111111"
                            {
                                continue;
                            }
                            let pre = meta.pre_balances.get(idx).copied().unwrap_or(0);
                            let post = meta.post_balances.get(idx).copied().unwrap_or(0);
                            let looks_like_rent =
                                post >= STANDARD_ATA_RENT.saturating_sub(tolerance) &&
                                post <= STANDARD_ATA_RENT.saturating_add(tolerance);
                            // Prefer accounts that appear to be token accounts (owner known) and not owned by wallet
                            let owner_opt = owners_by_key.get(key);
                            let owner_differs = owner_opt.map(|o| o != &wallet_str).unwrap_or(true);
                            if pre == 0 && looks_like_rent && owner_differs {
                                candidate_accounts.insert(key.clone());
                            }
                        }
                        if self.debug_enabled && !candidate_accounts.is_empty() {
                            let mut details: Vec<String> = candidate_accounts
                                .iter()
                                .map(|k| {
                                    let owner = owners_by_key
                                        .get(k)
                                        .cloned()
                                        .unwrap_or_else(|| "?".to_string());
                                    format!("{}->{}", k, owner)
                                })
                                .collect();
                            details.sort();
                            log(
                                LogTag::Transactions,
                                "ATA_RENT_ADJUST",
                                &format!(
                                    "{}: SELL fallback candidates (derived from balances): {}",
                                    &transaction.signature,
                                    details.join(", ")
                                )
                            );
                        }
                    }

                    // If we have any candidate accounts, look for SystemProgram transfers/createAccount from wallet to them
                    if !candidate_accounts.is_empty() {
                        if self.debug_enabled {
                            // Log discovered created accounts and owners
                            let mut details: Vec<String> = Vec::new();
                            for acc in &candidate_accounts {
                                let owner = created_owners
                                    .get(acc)
                                    .cloned()
                                    .or_else(|| owners_by_key.get(acc).cloned())
                                    .unwrap_or_else(|| "?".to_string());
                                details.push(format!("{}->{}", acc, owner));
                            }
                            details.sort();
                            log(
                                LogTag::Transactions,
                                "ATA_RENT_ADJUST",
                                &format!(
                                    "{}: SELL fallback created accounts: {}",
                                    &transaction.signature,
                                    details.join(", ")
                                )
                            );
                        }
                        let mut detect_funding = |
                            inst: &serde_json::Value,
                            acc_keys: &Vec<String>,
                            wallet: &str
                        | -> u64 {
                            // Resolve program id
                            let mut pid: Option<String> = inst
                                .get("programId")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            if pid.is_none() {
                                if
                                    let Some(idx) = inst
                                        .get("programIdIndex")
                                        .and_then(|v| v.as_u64())
                                {
                                    pid = acc_keys.get(idx as usize).cloned();
                                }
                            }
                            if pid.as_deref() != Some("11111111111111111111111111111111") {
                                return 0;
                            }
                            let parsed = match inst.get("parsed").and_then(|v| v.as_object()) {
                                Some(p) => p,
                                None => {
                                    return 0;
                                }
                            };
                            let itype = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            let info = match parsed.get("info").and_then(|v| v.as_object()) {
                                Some(i) => i,
                                None => {
                                    return 0;
                                }
                            };
                            if itype == "transfer" {
                                let dest = info
                                    .get("destination")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let src = info
                                    .get("source")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let lamports = info
                                    .get("lamports")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                // Only attribute if the created account's owner is not the wallet (avoid counting WSOL ATA etc.)
                                let owner_differs = created_owners
                                    .get(dest)
                                    .map(|o| o != wallet)
                                    .or_else(|| owners_by_key.get(dest).map(|o| o != wallet))
                                    .unwrap_or(true);
                                if self.debug_enabled {
                                    log(
                                        LogTag::Transactions,
                                        "ATA_RENT_ADJUST",
                                        &format!(
                                            "{}: SELL fallback check transfer: src={} dest={} lamports={} created_in_tx={} owner_differs={}",
                                            &transaction.signature,
                                            src,
                                            dest,
                                            lamports,
                                            candidate_accounts.contains(dest),
                                            owner_differs
                                        )
                                    );
                                }
                                if
                                    src == wallet &&
                                    lamports > 0 &&
                                    candidate_accounts.contains(dest) &&
                                    owner_differs
                                {
                                    return lamports;
                                }
                            } else if itype == "createaccount" {
                                // Some flows fund token accounts via createAccount directly
                                let dest = info
                                    .get("newAccount")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                // Payer can be in "source" (most common) but support "from" as well just in case
                                let src = info
                                    .get("source")
                                    .and_then(|v| v.as_str())
                                    .or_else(|| info.get("from").and_then(|v| v.as_str()))
                                    .unwrap_or("");
                                let lamports = info
                                    .get("lamports")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);
                                let owner_differs = created_owners
                                    .get(dest)
                                    .map(|o| o != wallet)
                                    .or_else(|| owners_by_key.get(dest).map(|o| o != wallet))
                                    .unwrap_or(true);
                                if self.debug_enabled {
                                    log(
                                        LogTag::Transactions,
                                        "ATA_RENT_ADJUST",
                                        &format!(
                                            "{}: SELL fallback check createAccount: src={} dest={} lamports={} created_in_tx={} owner_differs={}",
                                            &transaction.signature,
                                            src,
                                            dest,
                                            lamports,
                                            candidate_accounts.contains(dest),
                                            owner_differs
                                        )
                                    );
                                }
                                if
                                    src == wallet &&
                                    lamports > 0 &&
                                    candidate_accounts.contains(dest) &&
                                    owner_differs
                                {
                                    return lamports;
                                }
                            }
                            0
                        };

                        let mut rent_lamports: u64 = 0;
                        // Outer first
                        if
                            let Some(outer) = tx_data.transaction.message
                                .get("instructions")
                                .and_then(|v| v.as_array())
                        {
                            for inst in outer {
                                rent_lamports = rent_lamports.saturating_add(
                                    detect_funding(inst, &account_keys, &wallet_str)
                                );
                            }
                        }
                        // Then inner
                        if let Some(inner) = &meta.inner_instructions {
                            for entry in inner {
                                if
                                    let Some(instructions) = entry
                                        .get("instructions")
                                        .and_then(|v| v.as_array())
                                {
                                    for inst in instructions {
                                        rent_lamports = rent_lamports.saturating_add(
                                            detect_funding(inst, &account_keys, &wallet_str)
                                        );
                                    }
                                }
                            }
                        }

                        // Balance-based fallback for when inner instructions lack parsed data
                        if rent_lamports == 0 {
                            const STANDARD_ATA_RENT: u64 = 2_039_280;
                            let tolerance = STANDARD_ATA_RENT / 20; // 5%
                            for acc in &candidate_accounts {
                                // Skip wallet-owned created accounts (e.g., WSOL ATA)
                                let owner_differs = created_owners
                                    .get(acc)
                                    .map(|o| o != &wallet_str)
                                    .or_else(|| owners_by_key.get(acc).map(|o| o != &wallet_str))
                                    .unwrap_or(true);
                                if !owner_differs {
                                    continue;
                                }
                                if let Some(idx) = account_keys.iter().position(|k| k == acc) {
                                    let pre = meta.pre_balances.get(idx).copied().unwrap_or(0);
                                    let post = meta.post_balances.get(idx).copied().unwrap_or(0);
                                    let looks_like_rent =
                                        post >= STANDARD_ATA_RENT.saturating_sub(tolerance) &&
                                        post <= STANDARD_ATA_RENT.saturating_add(tolerance);
                                    if self.debug_enabled {
                                        log(
                                            LogTag::Transactions,
                                            "ATA_RENT_ADJUST",
                                            &format!(
                                                "{}: SELL fallback balance-check: acc={} idx={} pre={} post={} looks_like_rent={}",
                                                &transaction.signature,
                                                acc,
                                                idx,
                                                pre,
                                                post,
                                                looks_like_rent
                                            )
                                        );
                                    }
                                    if pre == 0 && looks_like_rent {
                                        rent_lamports = rent_lamports.saturating_add(post);
                                    }
                                }
                            }
                        }

                        if rent_lamports > 0 {
                            let rent_sol = lamports_to_sol(rent_lamports);
                            let before = sol_received_from_swap;
                            sol_received_from_swap = (sol_received_from_swap + rent_sol).max(0.0);
                            if self.debug_enabled {
                                log(
                                    LogTag::Transactions,
                                    "ATA_RENT_ADJUST",
                                    &format!(
                                        "{}: SELL rent fallback adjust: output={:.9} -> {:.9} (rent_spent_detected={:.9} SOL)",
                                        &transaction.signature,
                                        before,
                                        sol_received_from_swap,
                                        rent_sol
                                    )
                                );
                            }
                        }
                    }
                }
            }
        }

        // Final adjustments for common issues not detected by other methods
        if is_buy {
            // Adjustment for likely Jito tips
            let input_lamports = (sol_spent_effective * 1_000_000_000.0) as u64;

            // Check if removing common tip amounts results in a round number
            let common_tips = [50_000, 100_000, 150_000];
            for &tip in &common_tips {
                let adjusted = input_lamports.saturating_sub(tip);
                // Check if adjusted amount is a round increment (multiple of 0.001 SOL = 1M lamports)
                if adjusted > 0 && adjusted % 1_000_000 == 0 {
                    let adjusted_sol = (adjusted as f64) / 1_000_000_000.0;
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "FINAL_TIP_ADJUSTMENT",
                            &format!(
                                "{}: Final tip adjustment: {:.9} -> {:.9} SOL (removed {} lamport tip)",
                                &transaction.signature,
                                sol_spent_effective,
                                adjusted_sol,
                                tip
                            )
                        );
                    }
                    sol_spent_effective = adjusted_sol;
                    break;
                }
            }
        } else if is_sell {
            // Apply priority fees (tips) back to sell outputs to match CSV expectations
            let tip_for_sell = self.calculate_tip_amount(transaction, tx_data);
            if tip_for_sell > 0.0 {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "TIP_APPLY_SELL",
                        &format!(
                            "{}: Adding detected tips back to sell output: +{:.9} SOL",
                            &transaction.signature,
                            tip_for_sell
                        )
                    );
                }
                sol_received_from_swap += tip_for_sell;
            }
            // No further ATA heuristics for sells; rent flows are handled by the principled adjustment above.
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
            // Use TOTAL rent delta to reflect all rent flows this tx
            let total_rent_delta = ata.total_rent_spent - ata.total_rent_recovered; // spent minus recovered
            pnl_info.ata_created_count = ata.token_ata_creations;
            pnl_info.ata_closed_count = ata.token_ata_closures;
            pnl_info.ata_rents = total_rent_delta;

            if swap_direction == "Buy" {
                // Effective SOL spent for tokens should exclude rent flows
                pnl_info.effective_sol_spent = (
                    pnl_info.effective_sol_spent - total_rent_delta.max(0.0)
                ).max(0.0);
            } else if swap_direction == "Sell" {
                // Effective SOL received for tokens should include rent spent and exclude rent recovered
                if total_rent_delta.abs() > f64::EPSILON {
                    pnl_info.effective_sol_received = (
                        pnl_info.effective_sol_received + total_rent_delta
                    ).max(0.0);
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
        if matches!(router, "jupiter" | "pumpfun") {
            if is_buy {
                if let Some(amount) = self.detect_wallet_wsol_transfer_amount(transaction) {
                    return amount;
                }
            } else {
                if let Some(amount) = self.detect_wallet_wsol_receive_amount(transaction) {
                    return amount;
                }
            }
        }

        fallback_amount
    }

    fn detect_wallet_wsol_transfer_amount(&self, transaction: &Transaction) -> Option<f64> {
        const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        let wallet = self.wallet_pubkey.to_string();
        let raw = transaction.raw_transaction_data.as_ref()?;

        // Build a map of token account -> owner using pre/post token balances for better filtering
        use std::collections::HashMap;
        let mut owner_map: HashMap<String, String> = HashMap::new();
        let mut mint_map: HashMap<String, String> = HashMap::new();

        let account_keys: Vec<String> = raw
            .get("transaction")
            .and_then(|tx| tx.get("message"))
            .and_then(|m| m.get("accountKeys"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|k| {
                        if let Some(s) = k.as_str() {
                            Some(s.to_string())
                        } else if let Some(obj) = k.as_object() {
                            obj.get("pubkey")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut insert_owner_mint = |entry: &Value| {
            let idx_opt = entry.get("accountIndex").and_then(|v| v.as_u64());
            if let Some(idx) = idx_opt {
                if let Some(key) = account_keys.get(idx as usize) {
                    if let Some(owner) = entry.get("owner").and_then(|v| v.as_str()) {
                        owner_map.insert(key.clone(), owner.to_string());
                    }
                    if let Some(mint) = entry.get("mint").and_then(|v| v.as_str()) {
                        mint_map.insert(key.clone(), mint.to_string());
                    }
                }
            }
        };

        if
            let Some(pre) = raw
                .get("meta")
                .and_then(|m| m.get("preTokenBalances"))
                .and_then(|v| v.as_array())
        {
            for entry in pre {
                insert_owner_mint(entry);
            }
        }
        if
            let Some(post) = raw
                .get("meta")
                .and_then(|m| m.get("postTokenBalances"))
                .and_then(|v| v.as_array())
        {
            for entry in post {
                insert_owner_mint(entry);
            }
        }

        let mut amounts: Vec<f64> = Vec::new();

        // Discover wallet WSOL ATA from AToken createIdempotent (if any)
        let mut wallet_wsol_ata: Option<String> = None;
        if
            let Some(outer) = raw
                .get("transaction")
                .and_then(|tx| tx.get("message"))
                .and_then(|message| message.get("instructions"))
                .and_then(|v| v.as_array())
        {
            for instruction in outer {
                if
                    let (Some(program_id), Some(parsed)) = (
                        instruction.get("programId").and_then(|v| v.as_str()),
                        instruction.get("parsed").and_then(|v| v.as_object()),
                    )
                {
                    if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                        let itype = parsed
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if itype == "createidempotent" {
                            if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                                let mint_matches_wsol = info
                                    .get("mint")
                                    .and_then(|v| v.as_str())
                                    .map(|m| m == WSOL_MINT)
                                    .unwrap_or(false);
                                let wallet_matches = info
                                    .get("wallet")
                                    .and_then(|v| v.as_str())
                                    .map(|w| w == wallet)
                                    .unwrap_or(false);
                                if mint_matches_wsol && wallet_matches {
                                    if let Some(acc) = info.get("account").and_then(|v| v.as_str()) {
                                        wallet_wsol_ata = Some(acc.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Collect SyncNative accounts before parsing transfers
        let meta = raw.get("meta")?;
        let mut sync_native_accounts: std::collections::HashSet<String> = std::collections::HashSet::new();
        if let Some(inner) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for instruction in instructions {
                        if
                            let (Some(program_id), Some(parsed)) = (
                                instruction.get("programId").and_then(|v| v.as_str()),
                                instruction.get("parsed").and_then(|v| v.as_object()),
                            )
                        {
                            if program_id == TOKEN_PROGRAM_ID {
                                let itype = parsed
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase();
                                if itype == "syncnative" {
                                    if
                                        let Some(acc) = parsed
                                            .get("info")
                                            .and_then(|v| v.as_object())
                                            .and_then(|i| i.get("account"))
                                            .and_then(|v| v.as_str())
                                    {
                                        sync_native_accounts.insert(acc.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Outer fast path: detect a direct transferChecked WSOL with wallet authority
        if
            let Some(outer) = raw
                .get("transaction")
                .and_then(|tx| tx.get("message"))
                .and_then(|m| m.get("instructions"))
                .and_then(|v| v.as_array())
        {
            for instruction in outer {
                let parsed = match instruction.get("parsed").and_then(|v| v.as_object()) {
                    Some(p) => p,
                    None => {
                        continue;
                    }
                };
                let program_id = instruction
                    .get("programId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(||
                        instruction
                            .get("programIdIndex")
                            .and_then(|v| v.as_u64())
                            .and_then(|idx| account_keys.get(idx as usize).cloned())
                    )
                    .unwrap_or_default();
                if program_id != TOKEN_PROGRAM_ID {
                    continue;
                }
                let itype = parsed
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if itype != "transferchecked" {
                    continue;
                }
                let info = match parsed.get("info").and_then(|v| v.as_object()) {
                    Some(i) => i,
                    None => {
                        continue;
                    }
                };
                let authority_is_wallet = info
                    .get("authority")
                    .and_then(|v| v.as_str())
                    .map(|s| s == wallet)
                    .unwrap_or(false);
                let mint_is_wsol = info
                    .get("mint")
                    .and_then(|v| v.as_str())
                    .map(|m| m == WSOL_MINT)
                    .unwrap_or(false);

                // Check if this is a transfer to a MEV/tip address - exclude these from swap calculations
                let destination_is_mev = info
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .map(|addr| Self::is_mev_tip_address(addr))
                    .unwrap_or(false);

                if mint_is_wsol && authority_is_wallet && !destination_is_mev {
                    if let Some(ta) = info.get("tokenAmount").and_then(|v| v.as_object()) {
                        if let Some(ui) = ta.get("uiAmount").and_then(|v| v.as_f64()) {
                            if ui > 0.0 {
                                return Some(ui);
                            }
                        }
                        if
                            let (Some(amount_str), Some(dec)) = (
                                ta.get("amount").and_then(|v| v.as_str()),
                                ta.get("decimals").and_then(|v| v.as_u64()),
                            )
                        {
                            if let Ok(raw) = amount_str.parse::<u128>() {
                                let scale = (10_f64).powi(dec.min(18) as i32);
                                if scale > 0.0 {
                                    return Some((raw as f64) / scale);
                                }
                            }
                        }
                    }
                }
            }
        }
        if
            let Some(outer) = raw
                .get("transaction")
                .and_then(|tx| tx.get("message"))
                .and_then(|m| m.get("instructions"))
                .and_then(|v| v.as_array())
        {
            for instruction in outer {
                if
                    let (Some(program_id), Some(parsed)) = (
                        instruction.get("programId").and_then(|v| v.as_str()),
                        instruction.get("parsed").and_then(|v| v.as_object()),
                    )
                {
                    if program_id == TOKEN_PROGRAM_ID {
                        let itype = parsed
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if itype == "syncnative" {
                            if
                                let Some(acc) = parsed
                                    .get("info")
                                    .and_then(|v| v.as_object())
                                    .and_then(|i| i.get("account"))
                                    .and_then(|v| v.as_str())
                            {
                                sync_native_accounts.insert(acc.to_string());
                            }
                        }
                    }
                }
            }
        }

        let mut parse_amount = |instruction: &Value, wallet: &str| -> Option<f64> {
            // Support both programId (string) and programIdIndex (u64) forms
            let program_id = instruction
                .get("programId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    instruction
                        .get("programIdIndex")
                        .and_then(|v| v.as_u64())
                        .and_then(|idx| account_keys.get(idx as usize).cloned())
                })?;
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

            // Must be WSOL transfer. If mint not present (plain transfer), infer via account mints or account identity.
            let src_acc = info.get("source").and_then(|v| v.as_str());
            let dst_acc = info.get("destination").and_then(|v| v.as_str());
            let is_wsol = if let Some(m) = info.get("mint").and_then(|v| v.as_str()) {
                m == WSOL_MINT
            } else {
                let src_mint_is_wsol = src_acc
                    .and_then(|a| mint_map.get(a))
                    .map(|m| m == WSOL_MINT)
                    .unwrap_or(false);
                let dst_mint_is_wsol = dst_acc
                    .and_then(|a| mint_map.get(a))
                    .map(|m| m == WSOL_MINT)
                    .unwrap_or(false);
                let src_is_wallet_ata = src_acc
                    .and_then(|a| wallet_wsol_ata.as_ref().map(|w| a == w))
                    .unwrap_or(false);
                let dst_is_wallet_ata = dst_acc
                    .and_then(|a| wallet_wsol_ata.as_ref().map(|w| a == w))
                    .unwrap_or(false);
                let src_is_sync = src_acc
                    .map(|a| sync_native_accounts.contains(a))
                    .unwrap_or(false);
                let dst_is_sync = dst_acc
                    .map(|a| sync_native_accounts.contains(a))
                    .unwrap_or(false);
                src_mint_is_wsol ||
                    dst_mint_is_wsol ||
                    src_is_wallet_ata ||
                    dst_is_wallet_ata ||
                    src_is_sync ||
                    dst_is_sync
            };
            if !is_wsol {
                return None;
            }

            // Filter: prefer that the source is wallet-owned or a known wallet WSOL/native account; if unknown, allow
            if let Some(src) = src_acc {
                let src_owner = owner_map.get(src);
                let src_is_sync = sync_native_accounts.contains(src);
                let src_is_wallet_ata = wallet_wsol_ata
                    .as_deref()
                    .map(|w| w == src)
                    .unwrap_or(false);
                if let Some(src_owner) = src_owner {
                    if !src_is_sync && !src_is_wallet_ata && src_owner != &wallet {
                        return None;
                    }
                }
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
                            .unwrap_or(9);
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

        // Fast path: inner transferChecked WSOL with authority == wallet (or source is a SyncNative'd account)
        if let Some(inner) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for instruction in instructions {
                        let parsed = match instruction.get("parsed").and_then(|v| v.as_object()) {
                            Some(p) => p,
                            None => {
                                continue;
                            }
                        };
                        // Resolve program id from either programId or programIdIndex (v0 compiled)
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
                        if program_id != TOKEN_PROGRAM_ID {
                            continue;
                        }
                        let itype = parsed
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if itype != "transferchecked" {
                            continue;
                        }
                        let info = match parsed.get("info").and_then(|v| v.as_object()) {
                            Some(i) => i,
                            None => {
                                continue;
                            }
                        };
                        let authority_is_wallet = info
                            .get("authority")
                            .and_then(|v| v.as_str())
                            .map(|s| s == wallet)
                            .unwrap_or(false);
                        let mint_is_wsol = info
                            .get("mint")
                            .and_then(|v| v.as_str())
                            .map(|m| m == WSOL_MINT)
                            .unwrap_or(false);
                        let src_acc = info.get("source").and_then(|v| v.as_str());
                        let src_is_syncnative = src_acc
                            .map(|s| sync_native_accounts.contains(s))
                            .unwrap_or(false);
                        if mint_is_wsol && (authority_is_wallet || src_is_syncnative) {
                            if let Some(ta) = info.get("tokenAmount").and_then(|v| v.as_object()) {
                                if let Some(ui) = ta.get("uiAmount").and_then(|v| v.as_f64()) {
                                    if ui > 0.0 {
                                        return Some(ui);
                                    }
                                }
                                if
                                    let (Some(amount_str), Some(dec)) = (
                                        ta.get("amount").and_then(|v| v.as_str()),
                                        ta.get("decimals").and_then(|v| v.as_u64()),
                                    )
                                {
                                    if let Ok(raw) = amount_str.parse::<u128>() {
                                        let scale = (10_f64).powi(dec.min(18) as i32);
                                        if scale > 0.0 {
                                            return Some((raw as f64) / scale);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(inner) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for instruction in instructions {
                        if let Some(amount) = parse_amount(instruction, &wallet) {
                            amounts.push(amount);
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
                    amounts.push(amount);
                }
            }
        }

        // If we collected SPL WSOL transfer amounts, prefer the largest positive amount (main input)
        let mut spl_min_amount = amounts
            .iter()
            .copied()
            .filter(|a| *a > 0.0 && a.is_finite())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if spl_min_amount.is_none() {
            if let Some(wsol_ata) = wallet_wsol_ata.clone() {
                // Find a system transfer to this ATA and take lamports as input amount
                if
                    let Some(outer) = raw
                        .get("transaction")
                        .and_then(|tx| tx.get("message"))
                        .and_then(|message| message.get("instructions"))
                        .and_then(|v| v.as_array())
                {
                    for instruction in outer {
                        if
                            let (Some(program_id), Some(parsed)) = (
                                instruction.get("programId").and_then(|v| v.as_str()),
                                instruction.get("parsed").and_then(|v| v.as_object()),
                            )
                        {
                            if program_id == "11111111111111111111111111111111" {
                                let itype = parsed
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase();
                                if itype == "transfer" {
                                    if
                                        let Some(info) = parsed
                                            .get("info")
                                            .and_then(|v| v.as_object())
                                    {
                                        let dest_matches = info
                                            .get("destination")
                                            .and_then(|v| v.as_str())
                                            .map(|d| d == wsol_ata)
                                            .unwrap_or(false);
                                        let src_matches_wallet = info
                                            .get("source")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s == wallet)
                                            .unwrap_or(false);
                                        if dest_matches && src_matches_wallet {
                                            if
                                                let Some(lamports) = info
                                                    .get("lamports")
                                                    .and_then(|v| v.as_u64())
                                            {
                                                spl_min_amount = Some(
                                                    (lamports as f64) / 1_000_000_000.0
                                                );
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Another robust path: derive funding from any account that is later SyncNative'd
        if spl_min_amount.is_none() && !sync_native_accounts.is_empty() {
            if
                let Some(outer) = raw
                    .get("transaction")
                    .and_then(|tx| tx.get("message"))
                    .and_then(|message| message.get("instructions"))
                    .and_then(|v| v.as_array())
            {
                let mut best: Option<f64> = None;
                for instruction in outer {
                    if
                        let (Some(program_id), Some(parsed)) = (
                            instruction.get("programId").and_then(|v| v.as_str()),
                            instruction.get("parsed").and_then(|v| v.as_object()),
                        )
                    {
                        if program_id == "11111111111111111111111111111111" {
                            let itype = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            if itype == "transfer" {
                                if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                                    let dest = info.get("destination").and_then(|v| v.as_str());
                                    let src_is_wallet = info
                                        .get("source")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s == wallet)
                                        .unwrap_or(false);
                                    if let (Some(dest), true) = (dest, src_is_wallet) {
                                        if sync_native_accounts.contains(dest) {
                                            if
                                                let Some(lamports) = info
                                                    .get("lamports")
                                                    .and_then(|v| v.as_u64())
                                            {
                                                let val = (lamports as f64) / 1_000_000_000.0;
                                                best = Some(match best {
                                                    Some(b) => b.min(val),
                                                    None => val,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if spl_min_amount.is_none() {
                    spl_min_amount = best;
                }
            }
        }

        // Prefer SPL-detected amounts first; fall back to heuristics only if none found
        if let Some(val) = spl_min_amount {
            return Some(val);
        }

        // Heuristic: choose the smallest positive transfer out of wallet-owned WSOL accounts
        amounts
            .into_iter()
            .filter(|a| *a > 0.0 && a.is_finite())
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    fn detect_wallet_wsol_receive_amount(&self, transaction: &Transaction) -> Option<f64> {
        const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

        let wallet = self.wallet_pubkey.to_string();
        let raw = transaction.raw_transaction_data.as_ref()?;

        let account_keys: Vec<String> = raw
            .get("transaction")
            .and_then(|tx| tx.get("message"))
            .and_then(|m| m.get("accountKeys"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|k| {
                        if let Some(s) = k.as_str() {
                            Some(s.to_string())
                        } else if let Some(obj) = k.as_object() {
                            obj.get("pubkey")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Build map token account -> owner
        let mut owner_map: HashMap<String, String> = HashMap::new();
        let mut insert_owner = |entry: &Value| {
            if
                let (Some(idx), Some(owner)) = (
                    entry.get("accountIndex").and_then(|v| v.as_u64()),
                    entry.get("owner").and_then(|v| v.as_str()),
                )
            {
                if let Some(key) = account_keys.get(idx as usize) {
                    owner_map.insert(key.clone(), owner.to_string());
                }
            }
        };
        if
            let Some(pre) = raw
                .get("meta")
                .and_then(|m| m.get("preTokenBalances"))
                .and_then(|v| v.as_array())
        {
            for e in pre {
                insert_owner(e);
            }
        }
        if
            let Some(post) = raw
                .get("meta")
                .and_then(|m| m.get("postTokenBalances"))
                .and_then(|v| v.as_array())
        {
            for e in post {
                insert_owner(e);
            }
        }

        let mut amounts: Vec<f64> = Vec::new();
        let push_amount = |info: &serde_json::Map<String, Value>, amounts: &mut Vec<f64>| {
            if let Some(ta) = info.get("tokenAmount").and_then(|v| v.as_object()) {
                if let Some(ui) = ta.get("uiAmount").and_then(|v| v.as_f64()) {
                    if ui > 0.0 {
                        amounts.push(ui);
                        return;
                    }
                }
                if
                    let (Some(amount_str), Some(dec)) = (
                        ta.get("amount").and_then(|v| v.as_str()),
                        ta.get("decimals").and_then(|v| v.as_u64()),
                    )
                {
                    if let Ok(raw) = amount_str.parse::<u128>() {
                        let scale = (10_f64).powi(dec.min(18) as i32);
                        if scale > 0.0 {
                            amounts.push((raw as f64) / scale);
                            return;
                        }
                    }
                }
            }
            if
                let (Some(amount_str), Some(dec)) = (
                    info.get("amount").and_then(|v| v.as_str()),
                    info.get("decimals").and_then(|v| v.as_u64()),
                )
            {
                if let Ok(raw) = amount_str.parse::<u128>() {
                    let scale = (10_f64).powi(dec.min(18) as i32);
                    if scale > 0.0 {
                        amounts.push((raw as f64) / scale);
                        return;
                    }
                }
            }
        };

        // Helper: determine if a token account was created/initialized for this wallet in this tx
        let dst_was_initialized_for_wallet = |dst: &str| -> bool {
            // Check AToken createIdempotent for WSOL + this wallet
            if
                let Some(outer) = raw
                    .get("transaction")
                    .and_then(|tx| tx.get("message"))
                    .and_then(|m| m.get("instructions"))
                    .and_then(|v| v.as_array())
            {
                for instruction in outer {
                    if
                        let (Some(program_id), Some(parsed)) = (
                            instruction.get("programId").and_then(|v| v.as_str()),
                            instruction.get("parsed").and_then(|v| v.as_object()),
                        )
                    {
                        if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                            let itype = parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_ascii_lowercase();
                            if itype == "createidempotent" {
                                if let Some(info) = parsed.get("info").and_then(|v| v.as_object()) {
                                    let mint_matches_wsol = info
                                        .get("mint")
                                        .and_then(|v| v.as_str())
                                        .map(|m| m == WSOL_MINT)
                                        .unwrap_or(false);
                                    let wallet_matches = info
                                        .get("wallet")
                                        .and_then(|v| v.as_str())
                                        .map(|w| w == wallet)
                                        .unwrap_or(false);
                                    let acc = info.get("account").and_then(|v| v.as_str());
                                    if mint_matches_wsol && wallet_matches {
                                        if let Some(acc) = acc {
                                            if acc == dst {
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check Token initializeAccount/3 for owner == wallet and account == dst
            let mut check_parsed_init = |inst: &serde_json::Map<String, Value>| -> bool {
                let itype = inst
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if
                    itype == "initializeaccount" ||
                    itype == "initializeaccount2" ||
                    itype == "initializeaccount3"
                {
                    if let Some(info) = inst.get("info").and_then(|v| v.as_object()) {
                        let acc = info.get("account").and_then(|v| v.as_str());
                        let owner = info.get("owner").and_then(|v| v.as_str());
                        if let (Some(acc), Some(owner)) = (acc, owner) {
                            if acc == dst && owner == wallet {
                                return true;
                            }
                        }
                    }
                }
                false
            };

            if
                let Some(inner) = raw
                    .get("meta")
                    .and_then(|m| m.get("innerInstructions"))
                    .and_then(|v| v.as_array())
            {
                for entry in inner {
                    if
                        let Some(instructions) = entry
                            .get("instructions")
                            .and_then(|v| v.as_array())
                    {
                        for inst in instructions {
                            if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                                // programId can be token or others; we only care about parsed type
                                if check_parsed_init(parsed) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            if
                let Some(outer) = raw
                    .get("transaction")
                    .and_then(|tx| tx.get("message"))
                    .and_then(|m| m.get("instructions"))
                    .and_then(|v| v.as_array())
            {
                for inst in outer {
                    if let Some(parsed) = inst.get("parsed").and_then(|v| v.as_object()) {
                        if check_parsed_init(parsed) {
                            return true;
                        }
                    }
                }
            }

            false
        };

        // Scan inner first (support programIdIndex indirection too)
        if
            let Some(inner) = raw
                .get("meta")
                .and_then(|m| m.get("innerInstructions"))
                .and_then(|v| v.as_array())
        {
            for entry in inner {
                if let Some(instructions) = entry.get("instructions").and_then(|v| v.as_array()) {
                    for inst in instructions {
                        let parsed = match inst.get("parsed").and_then(|v| v.as_object()) {
                            Some(p) => p,
                            None => {
                                continue;
                            }
                        };
                        // Resolve program id possibly via index
                        let program_id = inst
                            .get("programId")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(||
                                inst
                                    .get("programIdIndex")
                                    .and_then(|v| v.as_u64())
                                    .and_then(|idx| account_keys.get(idx as usize).cloned())
                            );
                        if program_id.as_deref() != Some(TOKEN_PROGRAM_ID) {
                            continue;
                        }
                        let itype = parsed
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if itype != "transfer" && itype != "transferchecked" {
                            continue;
                        }
                        let info = match parsed.get("info").and_then(|v| v.as_object()) {
                            Some(i) => i,
                            None => {
                                continue;
                            }
                        };
                        let mint_is_wsol = info
                            .get("mint")
                            .and_then(|v| v.as_str())
                            .map(|m| m == WSOL_MINT);
                        // If mint field missing, infer via account identity/owner map later
                        // destination must be wallet-owned; source must not be wallet-owned
                        let src = info.get("source").and_then(|v| v.as_str());
                        let dst = info.get("destination").and_then(|v| v.as_str());
                        if let (Some(src), Some(dst)) = (src, dst) {
                            let src_owner = owner_map.get(src);
                            let dst_owner = owner_map.get(dst);
                            // Accept if owner_map shows wallet OR we initialized/created it in this tx
                            let dst_ok = match dst_owner {
                                Some(o) => o == &wallet,
                                None => dst_was_initialized_for_wallet(dst),
                            };
                            if !dst_ok {
                                continue;
                            }
                            if let Some(src_owner) = src_owner {
                                if src_owner == &wallet {
                                    continue;
                                }
                            }
                            // If mint isn't explicitly WSOL, infer using account identity
                            if mint_is_wsol != Some(true) {
                                let dst_is_wallet_ata = raw
                                    .get("transaction")
                                    .and_then(|tx| tx.get("message"))
                                    .and_then(|m| m.get("instructions"))
                                    .and_then(|v| v.as_array())
                                    .and_then(|arr| {
                                        for instruction in arr {
                                            if
                                                let (Some(program_id), Some(parsed)) = (
                                                    instruction
                                                        .get("programId")
                                                        .and_then(|v| v.as_str()),
                                                    instruction
                                                        .get("parsed")
                                                        .and_then(|v| v.as_object()),
                                                )
                                            {
                                                if
                                                    program_id ==
                                                    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
                                                {
                                                    let itype = parsed
                                                        .get("type")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("")
                                                        .to_ascii_lowercase();
                                                    if itype == "createidempotent" {
                                                        if
                                                            let Some(info) = parsed
                                                                .get("info")
                                                                .and_then(|v| v.as_object())
                                                        {
                                                            let mint_matches_wsol = info
                                                                .get("mint")
                                                                .and_then(|v| v.as_str())
                                                                .map(|m| m == WSOL_MINT)
                                                                .unwrap_or(false);
                                                            let wallet_matches = info
                                                                .get("wallet")
                                                                .and_then(|v| v.as_str())
                                                                .map(|w| w == wallet)
                                                                .unwrap_or(false);
                                                            let acc = info
                                                                .get("account")
                                                                .and_then(|v| v.as_str());
                                                            if mint_matches_wsol && wallet_matches {
                                                                if let Some(acc) = acc {
                                                                    if acc == dst {
                                                                        return Some(true);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        None
                                    })
                                    .unwrap_or(false);
                                if !dst_is_wallet_ata {
                                    continue;
                                }
                            }
                            push_amount(info, &mut amounts);
                        }
                    }
                }
            }
        }

        // Then outer
        if
            let Some(outer) = raw
                .get("transaction")
                .and_then(|tx| tx.get("message"))
                .and_then(|m| m.get("instructions"))
                .and_then(|v| v.as_array())
        {
            for inst in outer {
                let parsed = match inst.get("parsed").and_then(|v| v.as_object()) {
                    Some(p) => p,
                    None => {
                        continue;
                    }
                };
                let program_id = inst
                    .get("programId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(||
                        inst
                            .get("programIdIndex")
                            .and_then(|v| v.as_u64())
                            .and_then(|idx| account_keys.get(idx as usize).cloned())
                    );
                if program_id.as_deref() != Some(TOKEN_PROGRAM_ID) {
                    continue;
                }
                let itype = parsed
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if itype != "transfer" && itype != "transferchecked" {
                    continue;
                }
                let info = match parsed.get("info").and_then(|v| v.as_object()) {
                    Some(i) => i,
                    None => {
                        continue;
                    }
                };
                let mint_is_wsol = info
                    .get("mint")
                    .and_then(|v| v.as_str())
                    .map(|m| m == WSOL_MINT)
                    .unwrap_or(false);
                if !mint_is_wsol {
                    continue;
                }
                let src = info.get("source").and_then(|v| v.as_str());
                let dst = info.get("destination").and_then(|v| v.as_str());
                if let (Some(src), Some(dst)) = (src, dst) {
                    let src_owner = owner_map.get(src);
                    let dst_owner = owner_map.get(dst);
                    if let Some(dst_owner) = dst_owner {
                        if dst_owner != &wallet {
                            continue;
                        }
                    }
                    if let Some(src_owner) = src_owner {
                        if src_owner == &wallet {
                            continue;
                        }
                    }
                    push_amount(info, &mut amounts);
                }
            }
        }

        amounts
            .into_iter()
            .filter(|a| *a > 0.0 && a.is_finite())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
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
            for (idx, inst) in array.iter().enumerate() {
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

                let instruction_info = InstructionInfo {
                    program_id: program_id.clone(),
                    instruction_type: instruction_type.clone(),
                    accounts: accounts.clone(),
                    data: data.clone(),
                };

                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "INSTRUCTION_DEBUG",
                        &format!(
                            "Instruction {}: program_id={} type={} accounts_count={}",
                            idx,
                            program_id,
                            instruction_type,
                            accounts.len()
                        )
                    );
                }

                instructions.push(instruction_info);
            }
        }

        // Handle compiled instructions for v0 transactions
        if instructions.is_empty() {
            if let Some(compiled) = message.get("compiledInstructions").and_then(|v| v.as_array()) {
                for (idx, inst) in compiled.iter().enumerate() {
                    let program_id = inst
                        .get("programIdIndex")
                        .and_then(|v| v.as_u64())
                        .and_then(|idx| account_keys.get(idx as usize).cloned())
                        .unwrap_or_else(|| "unknown".to_string());

                    let accounts: Vec<String> = inst
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

                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "INSTRUCTION_DEBUG",
                            &format!(
                                "CompiledInstruction {}: program_id={} type=compiled accounts_count={}",
                                idx,
                                program_id,
                                accounts.len()
                            )
                        );
                    }

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
