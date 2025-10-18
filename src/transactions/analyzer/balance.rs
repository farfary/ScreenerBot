// Balance analysis module - SOL and SPL token balance change extraction
//
// This module implements the industry-standard approach used by DexScreener,
// GMGN, and Birdeye for extracting clean balance changes from Solana transactions.
//
// Key features:
// - Precise SOL balance change calculation using meta.preBalances/postBalances
// - SPL token balance tracking with proper decimal handling
// - Rent transfer filtering (~0.00204 SOL for ATA creation)
// - MEV/Jito tip exclusion for clean swap amount detection
// - Account-to-mint mapping for token identification

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::global::is_debug_transactions_enabled;
use crate::logger::{log, LogTag};
use crate::tokens::get_cached_decimals;
use crate::transactions::{program_ids, types::*, utils::*};
use crate::utils::{lamports_to_sol, sol_to_lamports};

// =============================================================================
// BALANCE ANALYSIS TYPES
// =============================================================================

/// Comprehensive balance analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceAnalysis {
    /// SOL balance changes by account
    pub sol_changes: HashMap<String, SolBalanceChange>,
    /// Token balance changes by account and mint
    pub token_changes: HashMap<String, Vec<TokenBalanceChange>>,
    /// Filtered transfer summary (excluding rent/tips)
    pub clean_transfers: Vec<CleanTransfer>,
    /// Total tips detected and excluded
    pub total_tips: f64,
    /// Total rent detected and excluded
    pub total_rent: f64,
    /// Analysis confidence score
    pub confidence: f64,
}

/// Clean transfer after filtering noise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanTransfer {
    pub from_account: String,
    pub to_account: String,
    pub mint: String, // SOL mint for SOL transfers
    pub amount: f64,
    pub transfer_type: TransferType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferType {
    SolTransfer,
    TokenTransfer,
    SwapLeg,     // Part of a DEX swap
    LiquidityOp, // Liquidity provision/removal
}

// =============================================================================
// KNOWN CONSTANTS (industry standard)
// =============================================================================

/// Common rent amounts for ATA creation (exclude from swap analysis)
const COMMON_RENT_AMOUNTS: &[u64] = &[
    2039280, // Standard ATA rent
    1461600, // Token account rent
    890880,  // Mint account rent
];

// MEV/Jito tip addresses are centralized in program_ids; reuse that list via helper.

/// Maximum tip amount to consider valid (larger amounts are likely swaps)
const MAX_TIP_AMOUNT: f64 = 0.01; // 0.01 SOL

// =============================================================================
// MAIN ANALYSIS FUNCTIONS
// =============================================================================

/// Comprehensive balance analysis following industry standards
pub async fn analyze_balance_changes(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<BalanceAnalysis, String> {
    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "BALANCE_ANALYZE",
            &format!(
                "Analyzing balance changes for tx: {}",
                transaction.signature
            ),
        );
    }

    // Extract raw balance changes
    let sol_changes = extract_sol_balance_changes(transaction, tx_data).await?;
    let token_changes = extract_token_balance_changes(transaction, tx_data).await?;

    // Filter out noise (tips, rent, etc.)
    let (clean_transfers, total_tips_filtered, total_rent) =
        filter_noise_transfers(&sol_changes, &token_changes).await?;

    // Extra safety: detect MEV/Jito tips directly from parsed instructions as well.
    // Take the max of the two methods to avoid double counting while ensuring we don't miss tips
    // when balance arrays don't capture destination keys properly.
    let instr_tips_sol = detect_mev_tips_from_instructions(tx_data).await;
    let total_tips = total_tips_filtered.max(instr_tips_sol);

    // Calculate confidence based on data quality
    let confidence = calculate_balance_confidence(&sol_changes, &token_changes, &clean_transfers);

    Ok(BalanceAnalysis {
        sol_changes,
        token_changes,
        clean_transfers,
        total_tips,
        total_rent,
        confidence,
    })
}

/// Main public function for extracting balance changes
pub async fn extract_balance_changes(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<BalanceAnalysis, String> {
    log(
        LogTag::Transactions,
        "BALANCE_ANALYZE",
        &format!(
            "Analyzing balance changes for tx: {}",
            transaction.signature
        ),
    );

    // Extract all balance changes
    let sol_changes = extract_sol_balance_changes(transaction, tx_data).await?;
    let token_changes = extract_token_balance_changes(transaction, tx_data).await?;

    // Simple implementations for now - TODO: Implement proper logic
    let clean_transfers = Vec::new(); // TODO: Implement transfer extraction
    let total_tips = 0.0; // TODO: Implement MEV tip calculation
    let total_rent = 0.0; // TODO: Implement rent calculation

    // Calculate confidence based on data quality
    let confidence = calculate_balance_confidence(&sol_changes, &token_changes, &clean_transfers);

    Ok(BalanceAnalysis {
        sol_changes,
        token_changes,
        clean_transfers,
        total_tips,
        total_rent,
        confidence,
    })
}

/// Quick balance extraction for performance-critical paths
pub async fn extract_basic_changes(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<BalanceAnalysis, String> {
    // Lightweight version - just raw changes without deep filtering
    let sol_changes = extract_sol_balance_changes(transaction, tx_data).await?;
    let token_changes = extract_token_balance_changes(transaction, tx_data).await?;

    Ok(BalanceAnalysis {
        sol_changes,
        token_changes,
        clean_transfers: Vec::new(),
        total_tips: 0.0,
        total_rent: 0.0,
        confidence: 0.8, // Assume good quality for quick analysis
    })
}

// =============================================================================
// SOL BALANCE EXTRACTION
// =============================================================================

/// Extract SOL balance changes using meta.preBalances/postBalances
async fn extract_sol_balance_changes(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<HashMap<String, SolBalanceChange>, String> {
    let mut sol_changes = HashMap::new();

    let message = &tx_data.transaction.message;
    let account_keys = account_keys_from_message(message);

    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "BALANCE_DEBUG",
            &format!("Extracted {} account keys from message", account_keys.len()),
        );
    }

    // Get balance arrays
    let pre_balances: &Vec<u64> = tx_data
        .meta
        .as_ref()
        .and_then(|m| Some(m.pre_balances.as_ref()))
        .ok_or("Missing pre_balances in transaction meta")?;

    let post_balances: &Vec<u64> = tx_data
        .meta
        .as_ref()
        .and_then(|m| Some(m.post_balances.as_ref()))
        .ok_or("Missing post_balances in transaction meta")?;

    if account_keys.len() != pre_balances.len() || account_keys.len() != post_balances.len() {
        log(
            LogTag::Transactions,
            "BALANCE_WARN",
            &format!(
                "Length mismatch - account_keys: {}, pre_balances: {}, post_balances: {} (will align)",
                account_keys.len(),
                pre_balances.len(),
                post_balances.len()
            )
        );
    }

    // Align account_keys length to balances length (jsonParsed often includes additional LUT keys)
    let min_len = std::cmp::min(
        account_keys.len(),
        std::cmp::min(pre_balances.len(), post_balances.len()),
    );
    let account_keys = account_keys.into_iter().take(min_len).collect::<Vec<_>>();

    // Calculate changes for each account
    for (i, account_key) in account_keys.iter().enumerate() {
        let pre_balance = pre_balances[i];
        let post_balance = post_balances[i];

        if pre_balance != post_balance {
            let change_lamports = (post_balance as i64) - (pre_balance as i64);
            let change_sol = lamports_to_sol(change_lamports.abs() as u64);

            sol_changes.insert(
                account_key.clone(),
                SolBalanceChange {
                    account: account_key.clone(),
                    pre_balance: lamports_to_sol(pre_balance),
                    post_balance: lamports_to_sol(post_balance),
                    change: change_sol * (if change_lamports < 0 { -1.0 } else { 1.0 }),
                },
            );
        }
    }

    Ok(sol_changes)
}

// =============================================================================
// TOKEN BALANCE EXTRACTION
// =============================================================================

/// Extract token balance changes using meta.preTokenBalances/postTokenBalances
async fn extract_token_balance_changes(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<HashMap<String, Vec<TokenBalanceChange>>, String> {
    let mut token_changes: HashMap<String, Vec<TokenBalanceChange>> = HashMap::new();

    let meta = tx_data.meta.as_ref().ok_or("Missing transaction meta")?;

    // Process pre/post token balances
    let empty_pre_balances = Vec::new();
    let empty_post_balances = Vec::new();
    let pre_token_balances = meta
        .pre_token_balances
        .as_ref()
        .unwrap_or(&empty_pre_balances);
    let post_token_balances = meta
        .post_token_balances
        .as_ref()
        .unwrap_or(&empty_post_balances);

    // Create lookup maps for efficient matching
    let mut pre_map: HashMap<(u32, String), &crate::rpc::UiTokenAmount> = HashMap::new();
    let mut post_map: HashMap<(u32, String), &crate::rpc::UiTokenAmount> = HashMap::new();
    let mut pre_owner_map: HashMap<(u32, String), Option<String>> = HashMap::new();
    let mut post_owner_map: HashMap<(u32, String), Option<String>> = HashMap::new();

    for balance in pre_token_balances {
        let key_for_amount = (balance.account_index, balance.mint.clone());
        let key_for_owner = (balance.account_index, balance.mint.clone());
        pre_map.insert(key_for_amount, &balance.ui_token_amount);
        pre_owner_map.insert(key_for_owner, balance.owner.clone());
    }

    for balance in post_token_balances {
        let key_for_amount = (balance.account_index, balance.mint.clone());
        let key_for_owner = (balance.account_index, balance.mint.clone());
        post_map.insert(key_for_amount, &balance.ui_token_amount);
        post_owner_map.insert(key_for_owner, balance.owner.clone());
    }

    // Find all unique (account_index, mint) combinations
    let mut all_keys: std::collections::HashSet<(u32, String)> = std::collections::HashSet::new();
    all_keys.extend(pre_map.keys().cloned());
    all_keys.extend(post_map.keys().cloned());

    let message = &tx_data.transaction.message;
    let account_keys = account_keys_from_message(message);

    for (account_index, mint) in all_keys {
        // Prefer attributing token changes to the owner (wallet) rather than the token account
        let owner_opt = post_owner_map
            .get(&(account_index, mint.clone()))
            .and_then(|o| o.clone())
            .or_else(|| {
                pre_owner_map
                    .get(&(account_index, mint.clone()))
                    .and_then(|o| o.clone())
            });

        let account_key_owned = if let Some(owner) = owner_opt {
            owner
        } else if (account_index as usize) < account_keys.len() {
            account_keys[account_index as usize].clone()
        } else {
            // Skip invalid indices with no resolvable owner
            continue;
        };

        let pre_amount = pre_map.get(&(account_index, mint.clone()));
        let post_amount = post_map.get(&(account_index, mint.clone()));

        let pre_ui = pre_amount.and_then(|a| a.ui_amount).unwrap_or(0.0);
        let post_ui = post_amount.and_then(|a| a.ui_amount).unwrap_or(0.0);

        if (pre_ui - post_ui).abs() > f64::EPSILON {
            let change = post_ui - pre_ui;
            // CRITICAL FIX: Use RPC-provided decimals as primary source (authoritative)
            // Fallback to DB lookup only if RPC doesn't provide it
            let decimals = post_amount
                .map(|a| a.decimals)
                .or_else(|| pre_amount.map(|a| a.decimals))
                .unwrap_or_else(|| get_cached_decimals(&mint).unwrap_or(9));

            let token_change = TokenBalanceChange {
                mint: mint.clone(),
                decimals,
                pre_balance: Some(pre_ui),
                post_balance: Some(post_ui),
                change,
                usd_value: None, // Will be calculated later if needed
            };

            token_changes
                .entry(account_key_owned)
                .or_insert_with(Vec::new)
                .push(token_change);
        }
    }

    // Diagnostic: summarize aggregation distribution
    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "BALANCE_DEBUG",
            &format!(
                "Token changes aggregated across {} owners/accounts (pre={} post={})",
                token_changes.len(),
                pre_token_balances.len(),
                post_token_balances.len()
            ),
        );
    }

    Ok(token_changes)
}

// =============================================================================
// NOISE FILTERING
// =============================================================================

/// Filter out rent payments, tips, and other noise from transfers
async fn filter_noise_transfers(
    sol_changes: &HashMap<String, SolBalanceChange>,
    token_changes: &HashMap<String, Vec<TokenBalanceChange>>,
) -> Result<(Vec<CleanTransfer>, f64, f64), String> {
    let mut clean_transfers = Vec::new();
    let mut total_tips = 0.0;
    let mut total_rent = 0.0;

    // Process SOL changes
    for change in sol_changes.values() {
        // Convert SOL amount back to lamports for rent pattern matching
        let change_lamports = sol_to_lamports(change.change.abs());
        if is_rent_amount(change_lamports) {
            total_rent += change.change.abs();
            continue;
        }

        if is_tip_transfer(&change.account, change.change.abs()) {
            total_tips += change.change.abs();
            continue;
        }

        // This is a clean SOL transfer
        clean_transfers.push(CleanTransfer {
            from_account: if change.change < 0.0 {
                change.account.clone()
            } else {
                "unknown".to_string()
            },
            to_account: if change.change > 0.0 {
                change.account.clone()
            } else {
                "unknown".to_string()
            },
            mint: WSOL_MINT.to_string(), // SOL mint
            amount: change.change.abs(),
            transfer_type: TransferType::SolTransfer,
        });
    }

    // Process token changes (typically these are clean)
    for (account, changes) in token_changes {
        for change in changes {
            clean_transfers.push(CleanTransfer {
                from_account: if change.change < 0.0 {
                    account.clone()
                } else {
                    "unknown".to_string()
                },
                to_account: if change.change > 0.0 {
                    account.clone()
                } else {
                    "unknown".to_string()
                },
                mint: change.mint.clone(),
                amount: change.change.abs(),
                transfer_type: TransferType::TokenTransfer,
            });
        }
    }

    Ok((clean_transfers, total_tips, total_rent))
}

/// Check if amount matches known rent patterns
pub(crate) fn is_rent_amount(lamports: u64) -> bool {
    // Consider close matches around known rent amounts to account for rent param variance and residuals
    const TOLERANCE: i64 = 150_000; // ~0.00015 SOL
    for known in COMMON_RENT_AMOUNTS {
        if ((lamports as i64) - (*known as i64)).abs() <= TOLERANCE {
            return true;
        }
    }
    false
}

/// Check if transfer is likely a tip to MEV/Jito
fn is_tip_transfer(account: &str, amount: f64) -> bool {
    program_ids::is_mev_tip_address(account) && amount <= MAX_TIP_AMOUNT
}

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

/// Calculate confidence score for balance analysis quality
fn calculate_balance_confidence(
    sol_changes: &HashMap<String, SolBalanceChange>,
    token_changes: &HashMap<String, Vec<TokenBalanceChange>>,
    clean_transfers: &[CleanTransfer],
) -> f64 {
    let mut score = 0.0;
    let mut factors = 0;

    // Factor 1: Number of balance changes detected
    if !sol_changes.is_empty() || !token_changes.is_empty() {
        score += 0.3;
    }
    factors += 1;

    // Factor 2: Clean transfers detected
    if !clean_transfers.is_empty() {
        score += 0.4;
    }
    factors += 1;

    // Factor 3: Reasonable number of changes (not too many = complex, not too few = incomplete)
    let total_changes = sol_changes.len() + token_changes.values().map(|v| v.len()).sum::<usize>();
    if total_changes >= 2 && total_changes <= 10 {
        score += 0.3;
    }
    factors += 1;

    if factors > 0 {
        score / (factors as f64)
    } else {
        0.0
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get account keys from transaction message (supports both legacy and v0)
fn account_keys_from_message(message: &Value) -> Vec<String> {
    // jsonParsed v0 can be:
    // - array of strings
    // - array of objects { pubkey, signer, writable, source }
    // - object { staticAccountKeys, loadedAddresses { writable, readonly } }
    if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
        // Try strings first
        let mut keys: Vec<String> = array
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !keys.is_empty() {
            return keys;
        }
        // Fallback: array of objects with pubkey field
        keys = array
            .iter()
            .filter_map(|v| {
                v.get("pubkey")
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
        return keys;
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
                    .map(|s| s.to_string()),
            );
        }

        // Loaded addresses
        if let Some(loaded) = obj.get("loadedAddresses").and_then(|v| v.as_object()) {
            if let Some(writable) = loaded.get("writable").and_then(|v| v.as_array()) {
                keys.extend(
                    writable
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string()),
                );
            }
            if let Some(readonly) = loaded.get("readonly").and_then(|v| v.as_array()) {
                keys.extend(
                    readonly
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string()),
                );
            }
        }

        return keys;
    }

    Vec::new()
}

// =============================================================================
// INSTRUCTION-BASED TIP DETECTION (robust)
// =============================================================================

/// Detect total MEV/Jito tips by scanning parsed outer and inner instructions
async fn detect_mev_tips_from_instructions(tx_data: &crate::rpc::TransactionDetails) -> f64 {
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
