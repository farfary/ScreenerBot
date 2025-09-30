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
            // Treat JSON null as success (Solana encodes no-error as null)
            let success = match &meta.err {
                None => true,
                Some(v) => v.is_null(),
            };
            transaction.success = success;
            if !success {
                transaction.error_message = meta.err.as_ref().map(|v| v.to_string());
            }
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

        // Map token swap info and swap PnL info based on analysis outputs
        // This fills Transaction.token_swap_info and swap_pnl_info so downstream tools (CSV verifier)
        // can validate amounts, mints, and router detection.
        if
            matches!(
                analysis.classification.transaction_type,
                crate::transactions::analyzer::classify::ClassifiedType::Buy |
                    crate::transactions::analyzer::classify::ClassifiedType::Sell |
                    crate::transactions::analyzer::classify::ClassifiedType::Swap
            )
        {
            // Determine swap orientation and primary token
            let direction_opt = &analysis.classification.direction;
            let primary_token_opt = &analysis.classification.primary_token;

            if
                let (Some(direction), Some(primary_mint)) = (
                    direction_opt.as_ref(),
                    primary_token_opt.as_ref(),
                )
            {
                // Resolve router string from DEX detection
                let router_str = (
                    match analysis.dex.detected_dex.as_ref() {
                        Some(crate::transactions::analyzer::dex::DetectedDex::Jupiter) => "jupiter",
                        Some(crate::transactions::analyzer::dex::DetectedDex::Raydium) => "raydium",
                        Some(crate::transactions::analyzer::dex::DetectedDex::RaydiumCLMM) =>
                            "raydium",
                        Some(crate::transactions::analyzer::dex::DetectedDex::Orca) => "orca",
                        Some(crate::transactions::analyzer::dex::DetectedDex::OrcaWhirlpool) =>
                            "orca",
                        Some(crate::transactions::analyzer::dex::DetectedDex::PumpFun) => "pumpfun",
                        Some(crate::transactions::analyzer::dex::DetectedDex::Meteora) => "meteora",
                        Some(_) => "unknown",
                        None => "unknown",
                    }
                ).to_string();

                // Helper to get wallet key string
                let wallet_key = self.wallet_pubkey.to_string();

                // Fetch token decimals once
                let token_decimals: u8 = crate::tokens
                    ::get_token_decimals(primary_mint).await
                    .unwrap_or(9) as u8;

                // Locate token balance change for the wallet and mint
                let token_ui_change: Option<f64> = analysis.balance.token_changes
                    .get(&wallet_key)
                    .and_then(|changes|
                        changes
                            .iter()
                            .find(|c| c.mint == *primary_mint)
                            .map(|c| c.change)
                    )
                    // fallback: largest change across owners for this mint
                    .or_else(|| {
                        analysis.balance.token_changes
                            .values()
                            .flat_map(|v| v.iter())
                            .filter(|c| c.mint == *primary_mint)
                            .max_by(|a, b|
                                a.change
                                    .abs()
                                    .partial_cmp(&b.change.abs())
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            )
                            .map(|c| c.change)
                    });

                // Locate SOL change for the wallet
                let sol_change_wallet = analysis.balance.sol_changes
                    .get(&wallet_key)
                    .map(|c| c.change);
                // fallback: use largest SOL change if wallet-specific not found
                let sol_change_any = analysis.balance.sol_changes
                    .values()
                    .max_by(|a, b|
                        a.change
                            .abs()
                            .partial_cmp(&b.change.abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    )
                    .map(|c| c.change);

                // Compute raw amounts and UI amounts based on direction
                let mut input_mint = String::new();
                let mut output_mint = String::new();
                let mut input_ui: f64 = 0.0;
                let mut output_ui: f64 = 0.0;
                let mut input_raw: u64 = 0;
                let mut output_raw: u64 = 0;
                let swap_type_str: &str;

                match direction {
                    crate::transactions::analyzer::classify::SwapDirection::SolToToken => {
                        swap_type_str = "sol_to_token";
                        input_mint = WSOL_MINT.to_string();
                        output_mint = primary_mint.clone();

                        let sol_abs = sol_change_wallet.or(sol_change_any).unwrap_or(0.0).abs();
                        // Compute swap input SOL excluding non-swap costs (base fee, priority, tips, rent)
                        let fb = &analysis.pnl.fee_breakdown;
                        let non_swap_costs =
                            fb.base_fee + fb.priority_fee + fb.mev_tips + fb.rent_costs;
                        let sol_for_swap = (sol_abs - non_swap_costs).max(0.0);
                        input_ui = sol_for_swap;
                        input_raw = (sol_for_swap * 1_000_000_000.0)
                            .round()
                            .clamp(0.0, u64::MAX as f64) as u64;

                        let token_abs = token_ui_change.unwrap_or(0.0).abs();
                        output_ui = token_abs;
                        let scale = (10f64).powi(token_decimals as i32);
                        output_raw = (token_abs * scale).round().clamp(0.0, u64::MAX as f64) as u64;
                    }
                    crate::transactions::analyzer::classify::SwapDirection::TokenToSol => {
                        swap_type_str = "token_to_sol";
                        input_mint = primary_mint.clone();
                        output_mint = WSOL_MINT.to_string();

                        let token_abs = token_ui_change.unwrap_or(0.0).abs();
                        input_ui = token_abs;
                        let scale = (10f64).powi(token_decimals as i32);
                        input_raw = (token_abs * scale).round().clamp(0.0, u64::MAX as f64) as u64;

                        let sol_abs = sol_change_wallet.or(sol_change_any).unwrap_or(0.0).abs();
                        output_ui = sol_abs;
                        output_raw = (sol_abs * 1_000_000_000.0)
                            .round()
                            .clamp(0.0, u64::MAX as f64) as u64;
                    }
                    crate::transactions::analyzer::classify::SwapDirection::TokenToToken => {
                        swap_type_str = "token_to_token";
                        // For token-to-token, use primary as input and try to infer secondary from classification
                        input_mint = primary_mint.clone();
                        output_mint = analysis.classification.secondary_token
                            .clone()
                            .unwrap_or_default();

                        let token_abs = token_ui_change.unwrap_or(0.0).abs();
                        input_ui = token_abs;
                        let scale_in = (10f64).powi(token_decimals as i32);
                        input_raw = (token_abs * scale_in)
                            .round()
                            .clamp(0.0, u64::MAX as f64) as u64;
                        // Output side unknown without deeper decoding; leave zeros
                        output_ui = 0.0;
                        output_raw = 0;
                    }
                }

                // Build TokenSwapInfo snapshot
                let token_swap_info = TokenSwapInfo {
                    mint: primary_mint.clone(),
                    symbol: String::new(), // enrichment optional
                    decimals: token_decimals,
                    current_price_sol: None,
                    is_verified: false,
                    router: router_str.clone(),
                    swap_type: swap_type_str.to_string(),
                    input_mint: input_mint.clone(),
                    output_mint: output_mint.clone(),
                    input_amount: input_raw,
                    output_amount: output_raw,
                    input_ui_amount: input_ui,
                    output_ui_amount: output_ui,
                    pool_address: analysis.dex.pool_address.clone(),
                    program_id: analysis.dex.program_ids.get(0).cloned().unwrap_or_default(),
                };

                // Map PnL main component if present
                let swap_pnl_info = if let Some(main) = &analysis.pnl.main_pnl {
                    let swap_type = match direction {
                        crate::transactions::analyzer::classify::SwapDirection::SolToToken => "Buy",
                        crate::transactions::analyzer::classify::SwapDirection::TokenToSol =>
                            "Sell",
                        crate::transactions::analyzer::classify::SwapDirection::TokenToToken =>
                            "Swap",
                    };

                    let fees_total = analysis.pnl.fee_breakdown.total_fees;
                    let status_str = if transaction.success { "✅ Success" } else { "❌ Failed" };

                    Some(SwapPnLInfo {
                        token_mint: primary_mint.clone(),
                        token_symbol: String::new(),
                        swap_type: swap_type.to_string(),
                        sol_amount: main.sol_amount_adjusted.abs(),
                        token_amount: main.token_amount.abs(),
                        calculated_price_sol: main.price_per_token,
                        timestamp: transaction.timestamp,
                        signature: transaction.signature.clone(),
                        router: router_str.clone(),
                        fee_sol: analysis.pnl.fee_breakdown.base_fee,
                        ata_rents: analysis.pnl.fee_breakdown.rent_costs,
                        effective_sol_spent: if
                            matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::SolToToken
                            )
                        {
                            main.sol_amount_adjusted.abs()
                        } else {
                            0.0
                        },
                        effective_sol_received: if
                            matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::TokenToSol
                            )
                        {
                            main.sol_amount_adjusted.abs()
                        } else {
                            0.0
                        },
                        ata_created_count: transaction.ata_analysis
                            .as_ref()
                            .map(|a| a.total_ata_creations)
                            .unwrap_or(0),
                        ata_closed_count: transaction.ata_analysis
                            .as_ref()
                            .map(|a| a.total_ata_closures)
                            .unwrap_or(0),
                        slot: transaction.slot,
                        status: status_str.to_string(),
                        // Legacy fields for debug tools
                        sol_spent: if
                            matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::SolToToken
                            )
                        {
                            main.sol_amount_raw.abs()
                        } else {
                            0.0
                        },
                        sol_received: if
                            matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::TokenToSol
                            )
                        {
                            main.sol_amount_raw.abs()
                        } else {
                            0.0
                        },
                        tokens_bought: if
                            matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::SolToToken
                            )
                        {
                            main.token_amount.abs()
                        } else {
                            0.0
                        },
                        tokens_sold: if
                            matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::TokenToSol
                            )
                        {
                            main.token_amount.abs()
                        } else {
                            0.0
                        },
                        net_sol_change: analysis.balance.sol_changes
                            .values()
                            .map(|c| c.change)
                            .sum(),
                        estimated_token_value_sol: None,
                        estimated_pnl_sol: None,
                        fees_paid_sol: fees_total,
                    })
                } else {
                    None
                };

                transaction.token_swap_info = Some(token_swap_info.clone());
                transaction.token_info = Some(token_swap_info);
                transaction.swap_pnl_info = swap_pnl_info;

                if self.debug_enabled {
                    if
                        matches!(
                            direction,
                            crate::transactions::analyzer::classify::SwapDirection::SolToToken
                        )
                    {
                        let fb = &analysis.pnl.fee_breakdown;
                        log(
                            LogTag::Transactions,
                            "MAP_SWAP_FEES",
                            &format!(
                                "fee_components: base={:.9} priority={:.9} tips={:.9} rent={:.9} swap_fees={:.9}",
                                fb.base_fee,
                                fb.priority_fee,
                                fb.mev_tips,
                                fb.rent_costs,
                                fb.swap_fees
                            )
                        );
                    }
                    log(
                        LogTag::Transactions,
                        "MAP_SWAP",
                        &format!(
                            "Mapped swap: dir={:?} router={} in {} (ui={:.9}) -> out {} (ui={:.6})",
                            direction,
                            router_str,
                            input_raw,
                            input_ui,
                            output_raw,
                            output_ui
                        )
                    );
                }
            } else if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "MAP_SWAP_SKIPPED",
                    &"Skipping swap mapping: missing direction or primary token".to_string()
                );
            }
        }

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
