//! Transaction processor analysis — high-level analysis pipeline for processed transactions.
//
// Transaction processing pipeline - Transaction analysis methods
//
// This module contains methods for analyzing and classifying transactions,
// mapping analysis results to transaction fields.

use crate::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::tokens::get_decimals;
use crate::transactions::{types::*, utils::WSOL_MINT};

use super::core::TransactionProcessor;
use super::helpers::*;

// =============================================================================
// TRANSACTION ANALYSIS
// =============================================================================

impl TransactionProcessor {
    /// Map analysis results to transaction fields
    pub(super) async fn map_analysis_to_transaction(
        &self,
        transaction: &mut Transaction,
        analysis: &crate::transactions::analyzer::CompleteAnalysis,
        tx_data: &crate::rpc::TransactionDetails,
    ) -> Result<(), String> {
        // Map classification results
        transaction.transaction_type = match analysis.classification.transaction_type {
            crate::transactions::analyzer::classify::ClassifiedType::Buy => TransactionType::Buy,
            crate::transactions::analyzer::classify::ClassifiedType::Sell => TransactionType::Sell,
            crate::transactions::analyzer::classify::ClassifiedType::Swap => {
                TransactionType::Unknown
            } // Could be Buy or Sell depending on direction
            crate::transactions::analyzer::classify::ClassifiedType::Transfer => {
                TransactionType::Transfer
            }
            crate::transactions::analyzer::classify::ClassifiedType::AddLiquidity => {
                TransactionType::Unknown
            }
            crate::transactions::analyzer::classify::ClassifiedType::RemoveLiquidity => {
                TransactionType::Unknown
            }
            crate::transactions::analyzer::classify::ClassifiedType::NftOperation => {
                TransactionType::Unknown
            }
            crate::transactions::analyzer::classify::ClassifiedType::ProgramInteraction => {
                TransactionType::Compute
            }
            crate::transactions::analyzer::classify::ClassifiedType::Failed => {
                TransactionType::Failed
            }
            crate::transactions::analyzer::classify::ClassifiedType::Unknown => {
                TransactionType::Unknown
            }
        };

        transaction.direction = match analysis.classification.direction {
            Some(crate::transactions::analyzer::classify::SwapDirection::SolToToken) => {
                TransactionDirection::Incoming
            }
            Some(crate::transactions::analyzer::classify::SwapDirection::TokenToSol) => {
                TransactionDirection::Outgoing
            }
            Some(crate::transactions::analyzer::classify::SwapDirection::TokenToToken) => {
                TransactionDirection::Internal
            }
            None => TransactionDirection::Unknown,
        };

        // Map balance changes
        transaction.sol_balance_changes = analysis.balance.sol_changes.values().cloned().collect();
        transaction.token_balance_changes = analysis
            .balance
            .token_changes
            .values()
            .flatten()
            .cloned()
            .collect();
        transaction.sol_balance_change = analysis
            .balance
            .sol_changes
            .values()
            .map(|change| change.change)
            .sum();

        // Map ATA analysis
        transaction.ata_analysis =
            Some(crate::transactions::types::AtaAnalysis {
                total_ata_creations: analysis.ata.rent_summary.accounts_created,
                total_ata_closures: analysis.ata.rent_summary.accounts_closed,
                token_ata_creations: analysis
                    .ata
                    .account_lifecycle
                    .created_accounts
                    .iter()
                    .filter(|acc| {
                        acc.mint.as_deref().is_some_and(|m| m != SOL_MINT)
                    })
                    .count() as u32,
                token_ata_closures: 0, // ClosedAccount doesn't have mint info, calculate from operations instead
                wsol_ata_creations: analysis
                    .ata
                    .account_lifecycle
                    .created_accounts
                    .iter()
                    .filter(|acc| acc.mint.as_deref() == Some(SOL_MINT))
                    .count() as u32,
                wsol_ata_closures: 0, // ClosedAccount doesn't have mint info, calculate from operations instead
                total_rent_spent: analysis.ata.rent_summary.total_rent_paid,
                total_rent_recovered: analysis.ata.rent_summary.total_rent_recovered,
                net_rent_impact: analysis.ata.rent_summary.net_rent_cost,
                token_rent_spent: analysis
                    .ata
                    .account_lifecycle
                    .created_accounts
                    .iter()
                    .filter(|acc| {
                        acc.mint.as_deref().is_some_and(|m| m != SOL_MINT)
                    })
                    .map(|acc| acc.rent_paid)
                    .sum(),
                token_rent_recovered: analysis
                    .ata
                    .account_lifecycle
                    .closed_accounts
                    .iter()
                    .map(|acc| acc.rent_recovered)
                    .sum::<f64>()
                    * 0.5, // Estimate half for tokens (since we can't distinguish)
                token_net_rent_impact: {
                    let spent: f64 = analysis
                        .ata
                        .account_lifecycle
                        .created_accounts
                        .iter()
                        .filter(|acc| {
                            acc.mint.as_deref().is_some_and(|m| m != SOL_MINT)
                        })
                        .map(|acc| acc.rent_paid)
                        .sum();
                    let recovered: f64 = analysis
                        .ata
                        .account_lifecycle
                        .closed_accounts
                        .iter()
                        .map(|acc| acc.rent_recovered)
                        .sum::<f64>()
                        * 0.5; // Estimate half for tokens
                    recovered - spent
                },
                wsol_rent_spent: analysis
                    .ata
                    .account_lifecycle
                    .created_accounts
                    .iter()
                    .filter(|acc| acc.mint.as_deref() == Some(SOL_MINT))
                    .map(|acc| acc.rent_paid)
                    .sum(),
                wsol_rent_recovered: analysis
                    .ata
                    .account_lifecycle
                    .closed_accounts
                    .iter()
                    .map(|acc| acc.rent_recovered)
                    .sum::<f64>()
                    * 0.5, // Estimate half for WSOL (since we can't distinguish)
                wsol_net_rent_impact: {
                    let spent: f64 = analysis
                        .ata
                        .account_lifecycle
                        .created_accounts
                        .iter()
                        .filter(|acc| acc.mint.as_deref() == Some(SOL_MINT))
                        .map(|acc| acc.rent_paid)
                        .sum();
                    let recovered: f64 = analysis
                        .ata
                        .account_lifecycle
                        .closed_accounts
                        .iter()
                        .map(|acc| acc.rent_recovered)
                        .sum::<f64>()
                        * 0.5; // Estimate half for WSOL
                    recovered - spent
                },
                detected_operations: analysis
                    .ata
                    .ata_operations
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
            is_wsol: op.mint.as_deref() == Some(SOL_MINT),
            mint: op.mint.clone().unwrap_or_default(),
            rent_cost_sol: Some(op.rent_amount),
          }
                    })
                    .collect(),
            });

        // Map token swap info and swap PnL info based on analysis outputs
        // This fills Transaction.token_swap_info and swap_pnl_info so downstream tools (CSV verifier)
        // can validate amounts, mints, and router detection.
        if matches!(
            analysis.classification.transaction_type,
            crate::transactions::analyzer::classify::ClassifiedType::Buy
                | crate::transactions::analyzer::classify::ClassifiedType::Sell
                | crate::transactions::analyzer::classify::ClassifiedType::Swap
        ) {
            // Determine swap orientation and primary token
            let direction_opt = &analysis.classification.direction;
            let primary_token_opt = &analysis.classification.primary_token;

            if let (Some(direction), Some(primary_mint)) =
                (direction_opt.as_ref(), primary_token_opt.as_ref())
            {
                // Resolve router string from DEX detection
                let router_str = (match analysis.dex.detected_dex.as_ref() {
                    Some(crate::transactions::analyzer::dex::DetectedDex::Jupiter) => "jupiter",
                    Some(crate::transactions::analyzer::dex::DetectedDex::Raydium) => "raydium",
                    Some(crate::transactions::analyzer::dex::DetectedDex::RaydiumCLMM) => "raydium",
                    Some(crate::transactions::analyzer::dex::DetectedDex::Orca) => "orca",
                    Some(crate::transactions::analyzer::dex::DetectedDex::OrcaWhirlpool) => "orca",
                    Some(crate::transactions::analyzer::dex::DetectedDex::PumpFun) => "pumpfun",
                    Some(crate::transactions::analyzer::dex::DetectedDex::Meteora) => "meteora",
                    Some(_) => "unknown",
                    None => "unknown",
                })
                .to_string();

                // Helper to get wallet key string
                let wallet_key = self.wallet_pubkey.to_string();

                // CRITICAL FIX: Fetch token decimals with RPC metadata as primary source
                // This prevents 1000x errors from wrong decimals (e.g., USDC 6 decimals)
                let token_decimals: u8 = extract_token_decimals_from_rpc(&tx_data, primary_mint)
                    .unwrap_or_else(|| {
                        // Fallback to DB lookup only if RPC doesn't have it
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(async { get_decimals(primary_mint).await.unwrap_or(9) })
                        })
                    }) as u8;

                // Locate token balance change for the wallet and mint
                let token_ui_change: Option<f64> = analysis
                    .balance
                    .token_changes
                    .get(&wallet_key)
                    .and_then(|changes| {
                        changes
                            .iter()
                            .find(|c| c.mint == *primary_mint)
                            .map(|c| c.change)
                    })
                    // fallback: largest change across owners for this mint
                    .or_else(|| {
                        analysis
                            .balance
                            .token_changes
                            .values()
                            .flat_map(|v| v.iter())
                            .filter(|c| c.mint == *primary_mint)
                            .max_by(|a, b| {
                                a.change
                                    .abs()
                                    .partial_cmp(&b.change.abs())
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|c| c.change)
                    });

                // Locate SOL change for the wallet only (do not fallback to any account),
                // because the largest SOL change might be WSOL ATA credit/debit which corrupts swap I/O.
                let sol_change_wallet = analysis
                    .balance
                    .sol_changes
                    .get(&wallet_key)
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

                        // Prefer authoritative WSOL wrap deposit:
                        // 1) Sum of system transfers from wallet -> wallet-owned WSOL ATA(s)
                        if let Some(lamports) =
                            find_wrap_deposit_via_sys_transfers_to_wsol_atas(&tx_data, &wallet_key)
                        {
                            input_raw = lamports;
                            input_ui = (lamports as f64) / 1_000_000_000.0;
                        } else if let Some(lamports) =
                            find_wrap_deposit_via_transfer_to_sync_account(&tx_data, &wallet_key)
                        {
                            // 1b) Direct: match syncNative account with the exact preceding system transfer from wallet
                            input_raw = lamports;
                            input_ui = (lamports as f64) / 1_000_000_000.0;
                        } else if let Some((sync_account, lamports)) =
                            find_wrap_sync_account_and_delta(&tx_data)
                        {
                            // 2) If we detected syncNative, attempt to sum system transfers from wallet to that account
                            if let Some(lamports_precise) =
                                sum_system_transfers_to_account_from_wallet(
                                    &tx_data,
                                    &wallet_key,
                                    &sync_account,
                                )
                            {
                                input_raw = lamports_precise;
                                input_ui = (lamports_precise as f64) / 1_000_000_000.0;
                            } else {
                                input_raw = lamports;
                                input_ui = (lamports as f64) / 1_000_000_000.0;
                            }
                        } else if let Some(lamports) =
                            find_wsol_wrap_deposit_lamports(&tx_data, &wallet_key)
                        {
                            input_raw = lamports;
                            input_ui = (lamports as f64) / 1_000_000_000.0;
                        } else if let Some(wsol_ui) =
                            find_owner_wsol_change_ui(&analysis, &wallet_key)
                        {
                            // Secondary: owner-aggregated WSOL outflow
                            input_ui = wsol_ui;
                            input_raw = (wsol_ui * 1_000_000_000.0)
                                .round()
                                .clamp(0.0, u64::MAX as f64)
                                as u64;
                        } else {
                            // Fallbacks: instruction-derived system transfer, then SOL delta for swap calculation
                            if let Some(lamports) =
                                find_largest_system_transfer_from_wallet(&tx_data, &wallet_key)
                            {
                                input_raw = lamports;
                                input_ui = (lamports as f64) / 1_000_000_000.0;
                            } else {
                                // For pure swap calculation, use the change amount before fees are applied
                                // The CSV amount represents the intended swap input, not wallet change
                                let sol_abs = sol_change_wallet.unwrap_or_default().abs();
                                let fb = &analysis.pnl.fee_breakdown;
                                // Add back transaction fees to get the original intended swap amount
                                let sol_for_swap =
                                    sol_abs + fb.base_fee + fb.priority_fee + fb.mev_tips;
                                input_ui = sol_for_swap;
                                input_raw = (sol_for_swap * 1_000_000_000.0)
                                    .round()
                                    .clamp(0.0, u64::MAX as f64)
                                    as u64;
                            }
                        }

                        // Reconcile with SOL delta minus non-swap costs to capture any missed micro outflows
                        if let Some(sol_delta_ui) = sol_change_wallet.map(|v| v.abs()) {
                            let fb = &analysis.pnl.fee_breakdown;
                            let non_swap_costs =
                                fb.base_fee + fb.priority_fee + fb.mev_tips + fb.rent_costs;
                            let derived_swap_ui = (sol_delta_ui - non_swap_costs).max(0.0);
                            let derived_swap_raw = (derived_swap_ui * 1_000_000_000.0)
                                .round()
                                .clamp(0.0, u64::MAX as f64)
                                as u64;
                            if derived_swap_raw > input_raw {
                                input_raw = derived_swap_raw;
                                input_ui = derived_swap_ui;
                            }
                            // Also reconcile with the largest non-tip system transfer from wallet (authoritative WSOL deposit)
                            if let Some(deposit_raw) =
                                find_largest_system_transfer_from_wallet_excluding_tips(
                                    &tx_data,
                                    &wallet_key,
                                )
                            {
                                if deposit_raw > input_raw {
                                    input_raw = deposit_raw;
                                    input_ui = (deposit_raw as f64) / 1_000_000_000.0;
                                }
                            }
                        }

                        // For SOL-to-token, check for gross outflow from wallet (aggregator or direct)
                        // This captures cases where wrapping/fees obscure the pure swap input.
                        {
                            // Look for total SOL outflow from user wallet to get gross amount (before protocol fees)
                            for sol_change in &analysis.balance.sol_changes {
                                let account = sol_change.1.account.clone();
                                let change = sol_change.1.change;

                                // Look specifically for the user wallet outflow
                                if account == wallet_key && change < 0.0 {
                                    let total_outflow = change.abs();
                                    let fb = &analysis.pnl.fee_breakdown;
                                    let transaction_costs =
                                        fb.base_fee + fb.priority_fee + fb.mev_tips + fb.rent_costs;

                                    // The pure swap amount should be total outflow minus transaction costs
                                    let gross_input_ui =
                                        (total_outflow - transaction_costs).max(0.0);

                                    if gross_input_ui > 0.0 {
                                        let gross_input_raw = (gross_input_ui * 1_000_000_000.0)
                                            .round()
                                            .clamp(0.0, u64::MAX as f64)
                                            as u64;

                                        if gross_input_raw > input_raw {
                                            logger::info(
    LogTag::Transactions,
                        &format!(
                          "Found wallet gross outflow: total={:.9} SOL tx_costs={:.9} SOL gross_swap={:.9} SOL (raw={})",
                          total_outflow,
                          transaction_costs,
                          gross_input_ui,
                          gross_input_raw
                        )
                      );
                                            input_raw = gross_input_raw;
                                            input_ui = gross_input_ui;
                                        }
                                    }
                                    break;
                                }
                            }
                        }

                        // Apply DEX-specific corrections based on instruction analysis and program patterns
                        if let Some(corrected_amount) = self.apply_dex_specific_corrections(
                            &router_str,
                            input_raw,
                            &tx_data,
                            &analysis.balance,
                            direction,
                        ) {
                            if corrected_amount != input_raw {
                                logger::info(
                                    LogTag::Transactions,
                                    &format!(
                                        "Applied {router} correction: {} -> {} ({:.2}% diff)",
                                        input_raw,
                                        corrected_amount,
                                        (((input_raw as f64) - (corrected_amount as f64)).abs()
                                            / (corrected_amount as f64))
                                            * 100.0,
                                        router = router_str
                                    ),
                                );
                                input_raw = corrected_amount;
                                input_ui = (corrected_amount as f64) / 1_000_000_000.0;
                            }
                        }

                        let token_abs = token_ui_change.unwrap_or_default().abs();
                        output_ui = token_abs;
                        let scale = (10f64).powi(token_decimals as i32);
                        output_raw = (token_abs * scale).round().clamp(0.0, u64::MAX as f64) as u64;
                    }
                    crate::transactions::analyzer::classify::SwapDirection::TokenToSol => {
                        swap_type_str = "token_to_sol";
                        input_mint = primary_mint.clone();
                        output_mint = WSOL_MINT.to_string();

                        let token_abs = token_ui_change.unwrap_or_default().abs();
                        input_ui = token_abs;
                        let scale = (10f64).powi(token_decimals as i32);
                        input_raw = (token_abs * scale).round().clamp(0.0, u64::MAX as f64) as u64;

                        // For sells (token-to-SOL), prefer instruction-derived WSOL credits to the wallet's WSOL ATAs
                        // to avoid counting rent-like artifacts or unrelated WSOL flows.
                        let mut sol_from_swap = {
                            let ui = sum_inner_wsol_transfers_ui_to_wallet(&tx_data, &wallet_key);
                            if ui > 0.0 {
                                ui
                            } else {
                                0.0
                            }
                        };

                        // If not found, look for SOL outflows from non-wallet accounts that match the token sale
                        if sol_from_swap == 0.0 {
                            for sol_change in &analysis.balance.sol_changes {
                                let account = sol_change.1.account.clone();
                                let change = sol_change.1.change;

                                // Skip wallet account - we want intermediary accounts
                                if account == wallet_key {
                                    continue;
                                }

                                // Negative change = outflow paying user
                                if change < 0.0 {
                                    let outflow_amount = change.abs();

                                    // Skip rent-pattern outflows (e.g., ATA close rent ~0.00203928 SOL)
                                    let outflow_lamports = (outflow_amount * 1_000_000_000.0)
                                        .round()
                                        .clamp(0.0, u64::MAX as f64)
                                        as u64;
                                    if crate::transactions::analyzer::balance::is_rent_amount(
                                        outflow_lamports,
                                    ) {
                                        continue;
                                    }

                                    // Accept any positive outflow up to a sane upper bound to avoid
                                    // capturing unrelated large transfers. This avoids hardcoded
                                    // micro-thresholds and preserves very small aggregator payouts.
                                    if outflow_amount > 0.0 && outflow_amount <= 1.0 {
                                        if outflow_amount > sol_from_swap {
                                            sol_from_swap = outflow_amount;
                                            if self.debug_enabled {
                                                logger::info(
                                                    LogTag::Transactions,
                                                    &format!(
                            "Found intermediary SOL outflow: account={} amount={:.9} SOL",
                            account,
                            outflow_amount
                          ),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Fallback to wallet-based calculation if no intermediary flows found
                        if sol_from_swap == 0.0 {
                            let sol_abs = sol_change_wallet.unwrap_or_default().abs();
                            let fb = &analysis.pnl.fee_breakdown;
                            let mut tips = fb.mev_tips;
                            let scanned = detect_mev_tips_from_instructions_light(&tx_data);
                            if scanned > tips {
                                tips = scanned;
                            }

                            sol_from_swap =
                                (sol_abs + fb.base_fee + fb.priority_fee + tips).max(0.0);

                            if self.debug_enabled {
                                logger::info(
                                    LogTag::Transactions,
                                    &format!(
                    "Using fallback calculation: wallet_change={:.9} + fees={:.9} = {:.9} SOL",
                    sol_abs,
                    fb.base_fee + fb.priority_fee + tips,
                    sol_from_swap
                  ),
                                );
                            }
                        }

                        output_ui = sol_from_swap;
                        output_raw = (sol_from_swap * 1_000_000_000.0)
                            .round()
                            .clamp(0.0, u64::MAX as f64)
                            as u64;

                        // Apply DEX-specific corrections based on instruction analysis and program patterns
                        if let Some(corrected_amount) = self.apply_dex_specific_corrections(
                            &router_str,
                            output_raw,
                            &tx_data,
                            &analysis.balance,
                            direction,
                        ) {
                            if corrected_amount != output_raw {
                                logger::info(
                                    LogTag::Transactions,
                                    &format!(
                                        "Applied {router} correction: {} -> {} ({:.2}% diff)",
                                        output_raw,
                                        corrected_amount,
                                        (((output_raw as f64) - (corrected_amount as f64)).abs()
                                            / (corrected_amount as f64))
                                            * 100.0,
                                        router = router_str
                                    ),
                                );
                                output_raw = corrected_amount;
                                output_ui = (corrected_amount as f64) / 1_000_000_000.0;
                            }
                        }

                        if self.debug_enabled {
                            logger::info(
                                LogTag::Transactions,
                                &format!(
                                    "Final swap calculation: output_ui={:.9} SOL (raw={})",
                                    output_ui, output_raw
                                ),
                            );
                        }
                    }
                    crate::transactions::analyzer::classify::SwapDirection::TokenToToken => {
                        swap_type_str = "token_to_token";
                        input_mint = primary_mint.clone();
                        output_mint = analysis
                            .classification
                            .secondary_token
                            .clone()
                            .unwrap_or_default();

                        let token_abs = token_ui_change.unwrap_or_default().abs();
                        input_ui = token_abs;
                        let scale_in = (10f64).powi(token_decimals as i32);
                        input_raw =
                            (token_abs * scale_in).round().clamp(0.0, u64::MAX as f64) as u64;

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
                    program_id: analysis.dex.program_ids.first().cloned().unwrap_or_default(),
                };

                // Add sanity checks for unreasonable swap amounts (user trades max 0.01 SOL)
                if self.debug_enabled {
                    match direction {
                        crate::transactions::analyzer::classify::SwapDirection::SolToToken => {
                            if input_ui > 0.011 {
                                // Allow small buffer above 0.01
                                logger::info(
                                    LogTag::Transactions,
                                    &format!(
                    "Buy amount {:.9} SOL exceeds expected max of 0.01 SOL for wallet {}",
                    input_ui,
                    self.wallet_pubkey
                  ),
                                );
                            }
                        }
                        crate::transactions::analyzer::classify::SwapDirection::TokenToSol => {
                            // For sells, allow larger amounts due to profit/loss but warn if extremely large
                            if output_ui > 0.1 {
                                // 10x the normal buy amount
                                logger::info(
                                    LogTag::Transactions,
                                    &format!(
                    "Sell output {:.9} SOL is unusually large for wallet {} (expected < 0.1 SOL)",
                    output_ui,
                    self.wallet_pubkey
                  ),
                                );
                            }
                        }
                        _ => {}
                    }
                }

                // Map PnL main component if present
                let swap_pnl_info =
                    if let Some(main) = &analysis.pnl.main_pnl {
                        let swap_type = match direction {
            crate::transactions::analyzer::classify::SwapDirection::SolToToken => "Buy",
            crate::transactions::analyzer::classify::SwapDirection::TokenToSol =>
              "Sell",
            crate::transactions::analyzer::classify::SwapDirection::TokenToToken =>
              "Swap",
          };

                        let fees_total = analysis.pnl.fee_breakdown.total_fees;
                        let status_str = if transaction.success {
                            "Success"
                        } else {
                            "Failed"
                        };

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
                            // CRITICAL FIX: Use total_rent_paid for all swap types
                            // ATAs can be created in any swap (e.g., WSOL ATA during sells)
                            // The verifier expects this value to normalize CSV amounts that include rent
                            ata_rents: analysis.ata.rent_summary.total_rent_paid,
                            effective_sol_spent: if matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::SolToToken
                            ) {
                                main.sol_amount_adjusted.abs()
                            } else {
                                0.0
                            },
                            effective_sol_received: if matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::TokenToSol
                            ) {
                                main.sol_amount_adjusted.abs()
                            } else {
                                0.0
                            },
                            ata_created_count: transaction
                                .ata_analysis
                                .as_ref()
                                .map(|a| a.total_ata_creations)
                                .unwrap_or_default(),
                            ata_closed_count: transaction
                                .ata_analysis
                                .as_ref()
                                .map(|a| a.total_ata_closures)
                                .unwrap_or_default(),
                            slot: transaction.slot,
                            status: status_str.to_string(),
                            // Legacy fields for debug tools
                            sol_spent: if matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::SolToToken
                            ) {
                                main.sol_amount_raw.abs()
                            } else {
                                0.0
                            },
                            sol_received: if matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::TokenToSol
                            ) {
                                main.sol_amount_raw.abs()
                            } else {
                                0.0
                            },
                            tokens_bought: if matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::SolToToken
                            ) {
                                main.token_amount.abs()
                            } else {
                                0.0
                            },
                            tokens_sold: if matches!(
                                direction,
                                crate::transactions::analyzer::classify::SwapDirection::TokenToSol
                            ) {
                                main.token_amount.abs()
                            } else {
                                0.0
                            },
                            net_sol_change: analysis
                                .balance
                                .sol_changes
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
                    if matches!(
                        direction,
                        crate::transactions::analyzer::classify::SwapDirection::SolToToken
                    ) {
                        let fb = &analysis.pnl.fee_breakdown;
                        logger::info(
                            LogTag::Transactions,
                            &format!(
                "fee_components: base={:.9} priority={:.9} tips={:.9} rent={:.9} swap_fees={:.9}",
                fb.base_fee,
                fb.priority_fee,
                fb.mev_tips,
                fb.rent_costs,
                fb.swap_fees
              ),
                        );
                    }
                    logger::info(
                        LogTag::Transactions,
                        &format!(
                            "Mapped swap: dir={:?} router={} in {} (ui={:.9}) -> out {} (ui={:.6})",
                            direction, router_str, input_raw, input_ui, output_raw, output_ui
                        ),
                    );
                }
            } else if self.debug_enabled {
                logger::info(
                    LogTag::Transactions,
                    &"Skipping swap mapping: missing direction or primary token".to_owned(),
                );
            }
        }

        Ok(())
    }

    /// Calculate total tip amount from system transfers to MEV addresses
    pub(super) fn calculate_tip_amount(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails,
    ) -> f64 {
        // Simple placeholder implementation
        0.0
    }
}
