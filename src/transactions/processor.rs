// Transaction processing pipeline for the transactions module
//
// This module handles the core transaction processing logic including
// data extraction, analysis, and classification of blockchain transactions.

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
  wallet_pubkey: Pubkey,
  fetcher: TransactionFetcher,
  analyzer: TransactionAnalyzer,
  debug_enabled: bool,
  cache_only: bool,
  force_refresh: bool,
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
            &format!("Failed to cache processed transaction: {}", e),
          );
        }
      } else if self.debug_enabled {
        logger::info(
          LogTag::Transactions,
          &format!("Cached processed transaction: {}", signature),
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

// =============================================================================
// DATA EXTRACTION
// =============================================================================

impl TransactionProcessor {
  /// Fetch transaction data with cache-first strategy
  async fn fetch_transaction_data(
    &self,
    signature: &str,
  ) -> Result<crate::rpc::TransactionDetails, String> {
    // Import the global database (avoiding multiple instances for now)
    let database = crate::transactions::database::get_transaction_database()
      .await
      .ok_or_else(|| "Transaction database not initialized".to_string())?;

    // Step 1: Handle cache-only mode - only try cache, never fetch from RPC
    if self.cache_only {
      if let Some(cached_details) = database.get_raw_transaction_details(signature).await? {
        if self.debug_enabled {
          logger::info(
            LogTag::Transactions,
            &format!(
              "Using cached raw transaction data (cache-only mode): {}",
              signature
            ),
          );
        }
        return Ok(cached_details);
      } else {
        return Err(format!(
          "Transaction {} not found in cache (cache-only mode)",
          signature
        ));
      }
    }

    // Step 2: Handle force-refresh mode - skip cache and fetch fresh
    if self.force_refresh {
      if self.debug_enabled {
        logger::info(
          LogTag::Transactions,
          &format!("Force fetching fresh transaction data: {}", signature),
        );
      }
    } else {
      // Step 3: Normal mode - try cache first
      if let Some(cached_details) = database.get_raw_transaction_details(signature).await? {
        if self.debug_enabled {
          logger::info(
            LogTag::Transactions,
            &format!("Using cached raw transaction data for: {}", signature),
          );
        }
        return Ok(cached_details);
      }

      if self.debug_enabled {
        logger::info(
          LogTag::Transactions,
          &format!("Fetching fresh transaction data for: {}", signature),
        );
      }
    }

    // Step 4: Fetch from blockchain
    let tx_details = self.fetcher.fetch_transaction_details(signature).await?;

    // Step 5: Store in cache for future use (unless cache-only mode)
    if !self.cache_only {
      // Create a minimal transaction for caching raw data
      let mut temp_transaction = Transaction::new(signature.to_string());
      temp_transaction.raw_transaction_data = Some(
        serde_json::to_value(&tx_details)
          .map_err(|e| format!("Failed to serialize transaction details: {}", e))?,
      );
      temp_transaction.slot = Some(tx_details.slot);
      temp_transaction.block_time = tx_details.block_time;
      if let Some(block_time) = tx_details.block_time {
        temp_transaction.timestamp =
          DateTime::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now());
      }
      if let Some(meta) = &tx_details.meta {
        temp_transaction.success = match &meta.err {
          None => true,
          Some(v) => v.is_null(),
        };
        if !temp_transaction.success {
          temp_transaction.error_message = meta.err.as_ref().map(|v| v.to_string());
        }
        temp_transaction.fee_lamports = Some(meta.fee);
      }
      temp_transaction.status = TransactionStatus::Confirmed;

      // Store raw transaction data in cache
      if let Err(e) = database.store_raw_transaction(&temp_transaction).await {
        if self.debug_enabled {
          logger::info(
            LogTag::Transactions,
            &format!(
              "Failed to cache raw transaction data for {}: {}",
              signature, e
            ),
          );
        }
      } else if self.debug_enabled {
        logger::info(
          LogTag::Transactions,
          &format!("Cached raw transaction data for: {}", signature),
        );
      }
    }

    Ok(tx_details)
  }

  /// Create Transaction structure from raw blockchain data
  async fn create_transaction_from_data(
    &self,
    signature: &str,
    tx_data: &crate::rpc::TransactionDetails,
  ) -> Result<Transaction, String> {
    let mut transaction = Transaction::new(signature.to_string());

    // Store raw transaction data for future reference
    transaction.raw_transaction_data = Some(
      serde_json::to_value(tx_data)
        .map_err(|e| format!("Failed to serialize transaction data: {}", e))?,
    );

    // Add comprehensive debug logging for transaction structure
    if self.debug_enabled {
      // Parse instruction count from the transaction message
      let instructions_info =
        if let Some(instructions) = tx_data.transaction.message.get("instructions") {
          if let Some(instructions_array) = instructions.as_array() {
            format!("{} instructions found", instructions_array.len())
          } else {
            "instructions field not an array".to_string()
          }
        } else {
          "no instructions field found".to_string()
        };

      logger::info(
        LogTag::Transactions,
        &format!("Transaction {} structure: {}", signature, instructions_info),
      );

      if let Some(meta) = &tx_data.meta {
        if let Some(log_messages) = &meta.log_messages {
          let log_preview = if log_messages.len() > 5 {
            format!(
              "{} logs (showing first 5): {}",
              log_messages.len(),
              log_messages
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
 .join("| ")
            )
          } else {
 format!("{} logs: {}", log_messages.len(), log_messages.join("| "))
          };

          logger::info(
            LogTag::Transactions,
            &format!("Transaction {} {}", signature, log_preview),
          );
        }

        logger::info(
          LogTag::Transactions,
          &format!(
            "Transaction {} balance changes: pre_count={}, post_count={}",
            signature,
            meta.pre_balances.len(),
            meta.post_balances.len()
          ),
        );
      }

      // Parse account keys count
      let account_keys_info =
        if let Some(account_keys) = tx_data.transaction.message.get("accountKeys") {
          if let Some(keys_array) = account_keys.as_array() {
            format!("{} account keys", keys_array.len())
          } else {
            "accountKeys field not an array".to_string()
          }
        } else {
          "no accountKeys field found".to_string()
        };

      logger::info(
        LogTag::Transactions,
        &format!("Transaction {} accounts: {}", signature, account_keys_info),
      );
    }

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
      transaction.timestamp =
        DateTime::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now());
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
            acc.mint.is_some() && acc.mint.as_ref().unwrap() != &SOL_MINT.to_string()
          })
          .count() as u32,
        token_ata_closures: 0, // ClosedAccount doesn't have mint info, calculate from operations instead
        wsol_ata_creations: analysis
          .ata
          .account_lifecycle
          .created_accounts
          .iter()
          .filter(|acc| acc.mint.as_ref() == Some(&SOL_MINT.to_string()))
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
            acc.mint.is_some() && acc.mint.as_ref().unwrap() != &SOL_MINT.to_string()
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
              acc.mint.is_some()
                && acc.mint.as_ref().unwrap() != &SOL_MINT.to_string()
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
          .filter(|acc| acc.mint.as_ref() == Some(&SOL_MINT.to_string()))
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
            .filter(|acc| acc.mint.as_ref() == Some(&SOL_MINT.to_string()))
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
                let sol_abs = sol_change_wallet.unwrap_or(0.0).abs();
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
                          )
                        );
                      }
                    }
                  }
                }
              }
            }

            // Fallback to wallet-based calculation if no intermediary flows found
            if sol_from_swap == 0.0 {
              let sol_abs = sol_change_wallet.unwrap_or(0.0).abs();
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
                  )
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

            let token_abs = token_ui_change.unwrap_or(0.0).abs();
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
          program_id: analysis.dex.program_ids.get(0).cloned().unwrap_or_default(),
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
                  )
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
                  )
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
                .unwrap_or(0),
              ata_closed_count: transaction
                .ata_analysis
                .as_ref()
                .map(|a| a.total_ata_closures)
                .unwrap_or(0),
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
              )
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
          &"Skipping swap mapping: missing direction or primary token".to_string(),
        );
      }
    }

    Ok(())
  }

  /// Calculate total tip amount from system transfers to MEV addresses
  fn calculate_tip_amount(
    &self,
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
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
  // Support multiple jsonParsed shapes for message.accountKeys
  // 1) Legacy/compact: array of strings
  if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
    // Try strings first
    let mut keys: Vec<String> = array
      .iter()
      .filter_map(|v| v.as_str().map(|s| s.to_string()))
      .collect();
    if !keys.is_empty() {
      return keys;
    }
    // Fallback: array of objects containing { pubkey, signer, writable, source }
    keys = array
      .iter()
      .filter_map(|v| {
        v.get("pubkey")
          .and_then(|p| p.as_str())
          .map(|s| s.to_string())
      })
      .collect();
    if !keys.is_empty() {
      return keys;
    }
  }

  // 2) v0 format: object with staticAccountKeys and loadedAddresses
  if let Some(obj) = message.get("accountKeys").and_then(|v| v.as_object()) {
    let mut keys = Vec::new();

    // Static account keys
    if let Some(static_keys) = obj.get("staticAccountKeys").and_then(|v| v.as_array()) {
      // staticAccountKeys itself may be strings or objects with pubkey
      for item in static_keys {
        if let Some(s) = item.as_str() {
          keys.push(s.to_string());
        } else if let Some(pk) = item.get("pubkey").and_then(|p| p.as_str()) {
          keys.push(pk.to_string());
        }
      }
    }

    // Loaded addresses: writable + readonly
    if let Some(loaded) = obj.get("loadedAddresses").and_then(|v| v.as_object()) {
      if let Some(writable) = loaded.get("writable").and_then(|v| v.as_array()) {
        for item in writable {
          if let Some(s) = item.as_str() {
            keys.push(s.to_string());
          } else if let Some(pk) = item.get("pubkey").and_then(|p| p.as_str()) {
            keys.push(pk.to_string());
          }
        }
      }
      if let Some(readonly) = loaded.get("readonly").and_then(|v| v.as_array()) {
        for item in readonly {
          if let Some(s) = item.as_str() {
            keys.push(s.to_string());
          } else if let Some(pk) = item.get("pubkey").and_then(|p| p.as_str()) {
            keys.push(pk.to_string());
          }
        }
      }
    }

    if !keys.is_empty() {
      return keys;
    }
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

/// Extract token decimals from RPC transaction metadata (authoritative source)
/// Returns None if the token is not found in pre/post token balances
fn extract_token_decimals_from_rpc(
  tx_data: &crate::rpc::TransactionDetails,
  mint: &str,
) -> Option<u8> {
  let meta = tx_data.meta.as_ref()?;

  // Check post token balances first (most recent state)
  if let Some(post_balances) = &meta.post_token_balances {
    for balance in post_balances {
      if balance.mint == mint {
        return Some(balance.ui_token_amount.decimals);
      }
    }
  }

  // Fallback to pre token balances
  if let Some(pre_balances) = &meta.pre_token_balances {
    for balance in pre_balances {
      if balance.mint == mint {
        return Some(balance.ui_token_amount.decimals);
      }
    }
  }

  None
}

/// Find the largest parsed system transfer amount from the wallet in inner/outer instructions
fn find_largest_system_transfer_from_wallet(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Option<u64> {
  let mut best: Option<u64> = None;

  // Helper to process a single instruction value
  let mut consider_ix = |ix: &serde_json::Value| {
    // Prefer parsed format
    if let Some(parsed) = ix.get("parsed") {
      let ix_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
      if let Some(info) = parsed.get("info") {
        let source = info
          .get("source")
          .and_then(|v| v.as_str())
          .or_else(|| info.get("from").and_then(|v| v.as_str()))
          .unwrap_or("");
        if source == wallet_key {
          let lamports = info
            .get("lamports")
            .and_then(|v| v.as_u64())
            .or_else(|| info.get("amount").and_then(|v| v.as_u64()));
 if lamports.is_some() && (ix_type == "transfer"|| ix_type == "createAccount") {
            let lamports = lamports.unwrap();
            if best.map(|b| lamports > b).unwrap_or(true) {
              best = Some(lamports);
            }
          }
        }
      }
    }
  };

  // Outer instructions
  if let Some(instructions) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in instructions {
      consider_ix(ix);
    }
  }

  // Inner instructions
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }

  best
}

/// Find the WSOL token amount that left the wallet (owner-aggregated token change) as UI amount
fn find_owner_wsol_change_ui(
  analysis: &crate::transactions::analyzer::CompleteAnalysis,
  wallet_key: &str,
) -> Option<f64> {
  analysis
    .balance
    .token_changes
    .get(wallet_key)
    .and_then(|changes| {
      changes
        .iter()
        .find(|c| c.mint == WSOL_MINT && c.change < 0.0)
        .map(|c| c.change.abs())
    })
}

/// Sum explicit MEV/Jito tip lamports sent from wallet by scanning parsed instructions
fn find_mev_tips_from_wallet(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Option<u64> {
  use crate::transactions::program_ids::is_mev_tip_address;
  let mut total: u64 = 0;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      let ix_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
 if ix_type == "transfer"{
        if let Some(info) = parsed.get("info") {
          let source = info
            .get("source")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("from").and_then(|v| v.as_str()))
            .unwrap_or("");
          let dest = info
            .get("destination")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("to").and_then(|v| v.as_str()))
            .unwrap_or("");
          if source == wallet_key && is_mev_tip_address(dest) {
            if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
              total = total.saturating_add(lamports);
            }
          }
        }
      }
    }
  };
  if let Some(instructions) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in instructions {
      consider_ix(ix);
    }
  }
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }
  if total > 0 {
    Some(total)
  } else {
    None
  }
}

/// Find lamports deposited into the wallet's WSOL ATA by inspecting pre/post balances at that account index
fn find_wsol_wrap_deposit_lamports(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Option<u64> {
  let meta = tx_data.meta.as_ref()?;
  let empty_vec: Vec<crate::rpc::TokenBalance> = Vec::new();
  let pre_token = meta.pre_token_balances.as_ref().unwrap_or(&empty_vec);
  let post_empty: Vec<crate::rpc::TokenBalance> = Vec::new();
  let post_token = meta.post_token_balances.as_ref().unwrap_or(&post_empty);

  // Build candidate account indices for WSOL accounts owned by wallet
  let mut indices: std::collections::HashSet<u32> = std::collections::HashSet::new();
  for bal in pre_token.iter().chain(post_token.iter()) {
    if bal.mint == WSOL_MINT {
      indices.insert(bal.account_index);
    }
  }
  if indices.is_empty() {
    return None;
  }

  let pre = &meta.pre_balances;
  let post = &meta.post_balances;
  let mut best: u64 = 0;
  for idx in indices {
    let i = idx as usize;
    if i >= pre.len() || i >= post.len() {
      continue;
    }
    let pre_b = pre[i];
    let post_b = post[i];
    if post_b > pre_b {
      let delta = post_b - pre_b;
      if delta > best {
        best = delta;
      }
    }
  }
  if best > 0 {
    Some(best)
  } else {
    None
  }
}

/// Detect wrap deposit by looking for Token Program syncNative instructions and measuring lamport delta on that account index
fn find_wrap_deposit_via_sync_native(tx_data: &crate::rpc::TransactionDetails) -> Option<u64> {
  let meta = tx_data.meta.as_ref()?;
  let pre = &meta.pre_balances;
  let post = &meta.post_balances;
  let keys = account_keys_from_message(&tx_data.transaction.message);

  let mut best: u64 = 0;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      if let Some(ix_type) = parsed.get("type").and_then(|v| v.as_str()) {
 if ix_type == "syncNative"{
          if let Some(info) = parsed.get("info") {
            if let Some(account) = info.get("account").and_then(|v| v.as_str()) {
              if let Some(index) = keys.iter().position(|k| k == account) {
                if index < pre.len() && index < post.len() {
                  let pre_b = pre[index];
                  let post_b = post[index];
                  if post_b > pre_b {
                    let delta = post_b - pre_b;
                    if delta > best {
                      best = delta;
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  };

  if let Some(ixs) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in ixs {
      consider_ix(ix);
    }
  }
  if let Some(inner) = &meta.inner_instructions {
    for group in inner {
      if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
        for ix in ixs {
          consider_ix(ix);
        }
      }
    }
  }

  if best > 0 {
    Some(best)
  } else {
    None
  }
}

/// Resolve account keys vector (supports legacy array and v0 object forms)
fn resolve_account_keys_vec(message: &serde_json::Value) -> Vec<String> {
  account_keys_from_message(message)
}

/// Collect wallet-owned WSOL ATA addresses from pre/post token balances
fn get_wallet_wsol_ata_addresses(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Vec<String> {
  let meta = match tx_data.meta.as_ref() {
    Some(m) => m,
    None => {
      return Vec::new();
    }
  };
  let message = &tx_data.transaction.message;
  let account_keys = resolve_account_keys_vec(message);

  let mut indices: std::collections::HashSet<u32> = std::collections::HashSet::new();
  if let Some(pre) = &meta.pre_token_balances {
    for bal in pre {
      if bal.mint == WSOL_MINT && bal.owner.as_deref() == Some(wallet_key) {
        indices.insert(bal.account_index);
      }
    }
  }
  if let Some(post) = &meta.post_token_balances {
    for bal in post {
      if bal.mint == WSOL_MINT && bal.owner.as_deref() == Some(wallet_key) {
        indices.insert(bal.account_index);
      }
    }
  }

  // Also look for createIdempotent that targets a WSOL ATA owned by the wallet; include that account even if
  // it has zero pre/post token balance entries (e.g., created and closed within the same tx)
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      if parsed.get("type").and_then(|v| v.as_str()) == Some("createIdempotent") {
        if let Some(info) = parsed.get("info") {
          let account = info.get("account").and_then(|v| v.as_str());
          let mint = info.get("mint").and_then(|v| v.as_str());
          let wallet = info.get("wallet").and_then(|v| v.as_str());
          if account.is_some() && mint == Some(WSOL_MINT) && wallet == Some(wallet_key) {
            // Map account pubkey to index if present
            if let Some(acc) = account {
              if let Some(index) = account_keys.iter().position(|k| k == acc) {
                indices.insert(index as u32);
              }
            }
          }
        }
      }
    }
  };
  if let Some(ixs) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in ixs {
      consider_ix(ix);
    }
  }
  if let Some(inner) = &meta.inner_instructions {
    for group in inner {
      if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
        for ix in ixs {
          consider_ix(ix);
        }
      }
    }
  }

  let mut addrs = Vec::new();
  for idx in indices {
    let i = idx as usize;
    if i < account_keys.len() {
      addrs.push(account_keys[i].clone());
    }
  }
  addrs
}

/// Find wrap deposit by summing system transfers from wallet to their WSOL ATA addresses
fn find_wrap_deposit_via_sys_transfers_to_wsol_atas(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Option<u64> {
  let wsol_atas = get_wallet_wsol_ata_addresses(tx_data, wallet_key);
  if wsol_atas.is_empty() {
    return None;
  }

  let mut total: u64 = 0;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      let ix_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
 if ix_type == "transfer"{
        if let Some(info) = parsed.get("info") {
          let source = info
            .get("source")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("from").and_then(|v| v.as_str()))
            .unwrap_or("");
          let dest = info
            .get("destination")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("to").and_then(|v| v.as_str()))
            .unwrap_or("");
          if source == wallet_key && wsol_atas.iter().any(|a| a == dest) {
            if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
              total = total.saturating_add(lamports);
            }
          }
        }
      }
    }
  };

  if let Some(instructions) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in instructions {
      consider_ix(ix);
    }
  }
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }

  if total > 0 {
    Some(total)
  } else {
    None
  }
}

/// Detect syncNative account and its lamport delta; returns (account_pubkey, delta_lamports)
fn find_wrap_sync_account_and_delta(
  tx_data: &crate::rpc::TransactionDetails,
) -> Option<(String, u64)> {
  let meta = tx_data.meta.as_ref()?;
  let pre = &meta.pre_balances;
  let post = &meta.post_balances;
  let keys = account_keys_from_message(&tx_data.transaction.message);

  let mut result: Option<(String, u64)> = None;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      if let Some(ix_type) = parsed.get("type").and_then(|v| v.as_str()) {
 if ix_type == "syncNative"{
          if let Some(info) = parsed.get("info") {
            if let Some(account) = info.get("account").and_then(|v| v.as_str()) {
              if let Some(index) = keys.iter().position(|k| k == account) {
                if index < pre.len() && index < post.len() {
                  let pre_b = pre[index];
                  let post_b = post[index];
                  // Even if delta is zero (e.g., wrap then full spend and close), the account is still the WSOL ATA
                  let delta = if post_b > pre_b { post_b - pre_b } else { 0 };
                  result = Some((account.to_string(), delta));
                }
              }
            }
          }
        }
      }
    }
  };

  if let Some(ixs) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in ixs {
      consider_ix(ix);
    }
  }
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }

  result
}

/// Sum all system transfers from wallet to a specific account address
fn sum_system_transfers_to_account_from_wallet(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
  dest_account: &str,
) -> Option<u64> {
  let mut total: u64 = 0;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      if parsed.get("type").and_then(|v| v.as_str()) == Some("transfer") {
        if let Some(info) = parsed.get("info") {
          let source = info
            .get("source")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("from").and_then(|v| v.as_str()))
            .unwrap_or("");
          let dest = info
            .get("destination")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("to").and_then(|v| v.as_str()))
            .unwrap_or("");
          if source == wallet_key && dest == dest_account {
            if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
              total = total.saturating_add(lamports);
            } else if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
              total = total.saturating_add(amount);
            }
          }
        }
      }
    }
  };
  if let Some(ixs) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in ixs {
      consider_ix(ix);
    }
  }
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }
  if total > 0 {
    Some(total)
  } else {
    None
  }
}

/// Find the largest system transfer amount sent from the wallet to any destination excluding known tip addresses
fn find_largest_system_transfer_from_wallet_excluding_tips(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Option<u64> {
  use crate::transactions::program_ids::is_mev_tip_address;
  let mut best: u64 = 0;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      if parsed.get("type").and_then(|v| v.as_str()) == Some("transfer") {
        if let Some(info) = parsed.get("info") {
          let source = info
            .get("source")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("from").and_then(|v| v.as_str()))
            .unwrap_or("");
          let dest = info
            .get("destination")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("to").and_then(|v| v.as_str()))
            .unwrap_or("");
          if source == wallet_key && !is_mev_tip_address(dest) {
            if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
              if lamports > best {
                best = lamports;
              }
            } else if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
              if amount > best {
                best = amount;
              }
            }
          }
        }
      }
    }
  };
  if let Some(ixs) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in ixs {
      consider_ix(ix);
    }
  }
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }
  if best > 0 {
    Some(best)
  } else {
    None
  }
}

/// Lightweight instruction scan for MEV/Jito tips (outer + inner), returning SOL units
fn detect_mev_tips_from_instructions_light(tx_data: &crate::rpc::TransactionDetails) -> f64 {
  use crate::transactions::program_ids::is_mev_tip_address;
  let mut total_lamports: u64 = 0;
  let mut consider_ix = |ix: &serde_json::Value| {
    if let Some(parsed) = ix.get("parsed") {
      if parsed.get("type").and_then(|v| v.as_str()) == Some("transfer") {
        if let Some(info) = parsed.get("info") {
          let dest = info
            .get("destination")
            .and_then(|v| v.as_str())
            .or_else(|| info.get("to").and_then(|v| v.as_str()))
            .unwrap_or("");
          if is_mev_tip_address(dest) {
            if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
              total_lamports = total_lamports.saturating_add(lamports);
            } else if let Some(amount) = info.get("amount").and_then(|v| v.as_u64()) {
              total_lamports = total_lamports.saturating_add(amount);
            }
          }
        }
      }
    }
  };
  if let Some(ixs) = tx_data
    .transaction
    .message
    .get("instructions")
    .and_then(|v| v.as_array())
  {
    for ix in ixs {
      consider_ix(ix);
    }
  }
  if let Some(meta) = &tx_data.meta {
    if let Some(inner) = &meta.inner_instructions {
      for group in inner {
        if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
          for ix in ixs {
            consider_ix(ix);
          }
        }
      }
    }
  }
  (total_lamports as f64) / 1_000_000_000.0
}

/// Detect wrap deposit by matching the syncNative account with explicit system transfer from wallet
fn find_wrap_deposit_via_transfer_to_sync_account(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> Option<u64> {
  let (sync_account, _delta) = find_wrap_sync_account_and_delta(tx_data)?;
  sum_system_transfers_to_account_from_wallet(tx_data, wallet_key, &sync_account)
}

/// Sum uiAmount across all inner instructions where the parsed info indicates a
/// transferChecked of WSOL (So1111...), returning SOL units. Robust to split/referral fees.
fn sum_inner_wsol_transferchecked_ui(tx_data: &crate::rpc::TransactionDetails) -> f64 {
  let meta = match tx_data.meta.as_ref() {
    Some(m) => m,
    None => {
      return 0.0;
    }
  };
  let inner = match meta.inner_instructions.as_ref() {
    Some(v) => v,
    None => {
      return 0.0;
    }
  };
  let mut total_ui: f64 = 0.0;
  for group in inner {
    if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
      for ix in ixs {
        if let Some(parsed) = ix.get("parsed") {
          if let Some(info) = parsed.get("info") {
            let mint = info.get("mint").and_then(|v| v.as_str()).unwrap_or("");
            if mint == crate::transactions::utils::WSOL_MINT {
              if let Some(token_amount) = info.get("tokenAmount") {
                if let Some(ui) =
                  token_amount.get("uiAmount").and_then(|v| v.as_f64())
                {
                  if ui > 0.0 {
                    total_ui += ui;
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  total_ui
}

/// Sum WSOL inner transfers (transfer or transferChecked) that credit the wallet's WSOL ATAs, returning SOL units
fn sum_inner_wsol_transfers_ui_to_wallet(
  tx_data: &crate::rpc::TransactionDetails,
  wallet_key: &str,
) -> f64 {
  let meta = match tx_data.meta.as_ref() {
    Some(m) => m,
    None => {
      return 0.0;
    }
  };
  let inner = match meta.inner_instructions.as_ref() {
    Some(v) => v,
    None => {
      return 0.0;
    }
  };
  // Resolve wallet's WSOL ATA addresses within this tx context
  let wallet_wsol_atas = get_wallet_wsol_ata_addresses(tx_data, wallet_key);
  if wallet_wsol_atas.is_empty() {
    return 0.0;
  }

  let mut total_ui: f64 = 0.0;
  for group in inner {
    if let Some(ixs) = group.get("instructions").and_then(|v| v.as_array()) {
      for ix in ixs {
        if let Some(parsed) = ix.get("parsed") {
          let ix_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
 if ix_type == "transfer"|| ix_type == "transferChecked"{
            if let Some(info) = parsed.get("info") {
              let mint = info.get("mint").and_then(|v| v.as_str()).unwrap_or("");
              if mint == crate::transactions::utils::WSOL_MINT {
                let dest = info
                  .get("destination")
                  .and_then(|v| v.as_str())
                  .or_else(|| info.get("to").and_then(|v| v.as_str()))
                  .unwrap_or("");
                if !dest.is_empty() && wallet_wsol_atas.iter().any(|a| a == dest) {
                  // Prefer tokenAmount.uiAmount (transferChecked). Fallback to amount lamports (transfer)
                  let mut ui_amount = 0.0f64;
                  if let Some(token_amount) = info.get("tokenAmount") {
                    if let Some(ui) =
                      token_amount.get("uiAmount").and_then(|v| v.as_f64())
                    {
                      ui_amount = ui;
                    }
                  }
                  if ui_amount == 0.0 {
                    if let Some(raw) =
                      info.get("amount").and_then(|v| v.as_str())
                    {
                      if let Ok(lamports) = raw.parse::<u64>() {
                        ui_amount = (lamports as f64) / 1_000_000_000.0;
                      }
                    } else if let Some(raw_num) =
                      info.get("amount").and_then(|v| v.as_u64())
                    {
                      ui_amount = (raw_num as f64) / 1_000_000_000.0;
                    }
                  }
                  if ui_amount > 0.0 {
                    total_ui += ui_amount;
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  total_ui
}

impl TransactionProcessor {
  /// Apply DEX-specific corrections based on instruction analysis and program patterns
  fn apply_dex_specific_corrections(
    &self,
    router: &str,
    calculated_amount: u64,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
    direction: &crate::transactions::analyzer::classify::SwapDirection,
  ) -> Option<u64> {
    match router {
 "jupiter"=> self.apply_jupiter_corrections(
        calculated_amount,
        tx_data,
        balance_analysis,
        direction,
      ),
 "pumpfun"=> self.apply_pumpfun_corrections(
        calculated_amount,
        tx_data,
        balance_analysis,
        direction,
      ),
 "raydium"=> {
        self.apply_raydium_corrections(calculated_amount, tx_data, balance_analysis)
      }
      _ => None,
    }
  }

  /// Apply Jupiter-specific corrections based on instruction analysis
  fn apply_jupiter_corrections(
    &self,
    calculated_amount: u64,
    tx_data: &crate::rpc::TransactionDetails,
    _balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
    direction: &crate::transactions::analyzer::classify::SwapDirection,
  ) -> Option<u64> {
    // Direction-aware correction policy for Jupiter:
    // - Buy (SOL -> Token): prefer authoritative deposit amount from wallet to WSOL ATA(s).
 // If detected, set calculated amount to that deposit (instruction truth). Do not add fee legs.
    // - Sell (Token -> SOL): do not adjust; output is derived from WSOL credits/unwrapped SOL and should be net of fees.
    match direction {
      crate::transactions::analyzer::classify::SwapDirection::SolToToken => {
        // Find the authoritative deposit (largest non-tip system transfer from wallet)
        if let Some(deposit_raw) = find_largest_system_transfer_from_wallet_excluding_tips(
          tx_data,
          &self.wallet_pubkey.to_string(),
        ) {
          if deposit_raw > 0 {
            // Only replace when it differs meaningfully (>0.05%) to avoid churn
            let rel = if calculated_amount > 0 {
              (((calculated_amount as i128) - (deposit_raw as i128)).abs() as f64)
                / (calculated_amount as f64)
            } else {
              1.0
            };
            if rel > 0.0005 {
              return Some(deposit_raw);
            }
          }
        }
        None
      }
      _ => None,
    }
  }

  /// Apply Raydium-specific corrections
  fn apply_raydium_corrections(
    &self,
    calculated_amount: u64,
    tx_data: &crate::rpc::TransactionDetails,
    _balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
  ) -> Option<u64> {
    let raydium_program_ids = [
      "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM
      "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // Raydium CPMM
    ];

    // Check if this transaction involves Raydium by analyzing instructions
    let has_raydium =
      if let Some(instructions) = tx_data.transaction.message.get("instructions") {
        if let Some(instructions_array) = instructions.as_array() {
          instructions_array.iter().any(|ix| {
            if let Some(program_id) = ix.get("programId").and_then(|p| p.as_str()) {
              raydium_program_ids.contains(&program_id)
            } else {
              false
            }
          })
        } else {
          false
        }
      } else {
        false
      };

    if !has_raydium {
      return None;
    }

    // Raydium corrections would go here
    // For now, no specific patterns identified
    None
  }

  /// Apply Pumpfun-specific corrections (placeholder: keep micro-adjustments strictly <0.5%)
  fn apply_pumpfun_corrections(
    &self,
    calculated_amount: u64,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
    direction: &crate::transactions::analyzer::classify::SwapDirection,
  ) -> Option<u64> {
    // Detect PumpFun by presence of legacy or AMM program IDs among outer instructions
    let pumpfun_programs = [
      crate::constants::PUMP_FUN_LEGACY_PROGRAM_ID,
      crate::constants::PUMP_FUN_AMM_PROGRAM_ID,
    ];

    let has_pumpfun = if let Some(ixs) = tx_data
      .transaction
      .message
      .get("instructions")
      .and_then(|v| v.as_array())
    {
      ixs.iter().any(|ix| {
        ix.get("programId")
          .and_then(|v| v.as_str())
          .map(|pid| pumpfun_programs.contains(&pid))
          .unwrap_or(false)
      })
    } else {
      false
    };
    if !has_pumpfun {
      return None;
    }

    // Direction-aware selection of instruction-truth candidates
    match direction {
      crate::transactions::analyzer::classify::SwapDirection::TokenToSol => {
        // SELL: prefer exact WSOL inner credits to wallet ATAs
        let candidate_sell_ui =
          sum_inner_wsol_transfers_ui_to_wallet(tx_data, &self.wallet_pubkey.to_string());
        let candidate_sell_raw = (candidate_sell_ui * 1_000_000_000.0)
          .round()
          .clamp(0.0, u64::MAX as f64) as u64;
        if candidate_sell_raw > 0 {
          return Some(candidate_sell_raw);
        }
        // Fallback: keep original
        None
      }
      crate::transactions::analyzer::classify::SwapDirection::SolToToken => {
        // BUY: prefer largest non-tip system transfer from wallet (authoritative deposit)
        if let Some(deposit_raw) = find_largest_system_transfer_from_wallet_excluding_tips(
          tx_data,
          &self.wallet_pubkey.to_string(),
        ) {
          if deposit_raw > 0 {
            return Some(deposit_raw);
          }
        }
        None
      }
      _ => None,
    }
  }

  /// Check if adjustment pattern is likely for Pumpfun
  fn is_likely_pumpfun_pattern(
    &self,
    calculated_amount: u64,
    adjusted_amount: u64,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
  ) -> bool {
    // Look for Pumpfun-specific instruction patterns
    let pumpfun_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

    // Count Pumpfun instructions by analyzing transaction structure
    let pumpfun_instruction_count =
      if let Some(instructions) = tx_data.transaction.message.get("instructions") {
        if let Some(instructions_array) = instructions.as_array() {
          instructions_array
            .iter()
            .filter(|ix| {
              if let Some(program_id) = ix.get("programId").and_then(|p| p.as_str()) {
                program_id == pumpfun_program_id
              } else {
                false
              }
            })
            .count()
        } else {
          0
        }
      } else {
        0
      };

    // Check for intermediary account patterns in balance changes
    let has_intermediary_pattern =
      balance_analysis
        .sol_changes
        .iter()
        .any(|(account, change)| {
          // Look for accounts that aren't the main wallet but have SOL changes
          change.change.abs() > 0.0 && !change.change.is_nan()
        });

    // Pumpfun typically has 1-2 instructions and intermediary accounts
    pumpfun_instruction_count >= 1 && pumpfun_instruction_count <= 3 && has_intermediary_pattern
  }
}
