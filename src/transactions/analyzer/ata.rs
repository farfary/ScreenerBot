// ATA operations analysis module - Associated Token Account lifecycle tracking
//
// This module analyzes ATA (Associated Token Account) operations including
// creation, initialization, and rent calculations following Solana standards.
//
// Key operations tracked:
// - ATA creation and rent funding (~0.00204 SOL)
// - Token account initialization
// - Rent recovery on account closure
// - Multi-ATA operations in complex transactions

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::transactions::{types::*, utils::*};
use crate::utils::lamports_to_sol;

// =============================================================================
// ATA ANALYSIS TYPES
// =============================================================================

/// Comprehensive ATA analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaAnalysis {
    /// ATA operations detected
    pub ata_operations: Vec<AtaOperation>,
    /// Summary of rent costs and recoveries
    pub rent_summary: RentSummary,
    /// Total accounts created/closed
    pub account_lifecycle: AccountLifecycle,
    /// Analysis confidence
    pub confidence: f64,
}

/// Individual ATA operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaOperation {
    /// Type of operation
    pub operation_type: AtaOperationType,
    /// Account address
    pub account_address: String,
    /// Token mint (if applicable)
    pub mint: Option<String>,
    /// Owner/authority
    pub owner: Option<String>,
    /// Rent amount involved
    pub rent_amount: f64,
    /// Operation success
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtaOperationType {
    Create,       // ATA creation
    Initialize,   // Token account initialization
    Close,        // Account closure with rent recovery
    Transfer,     // ATA ownership transfer
    SetAuthority, // Authority change
    CreateNative, // Native SOL account creation
}

/// Summary of rent-related operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentSummary {
    /// Total rent paid for new accounts
    pub total_rent_paid: f64,
    /// Total rent recovered from closed accounts
    pub total_rent_recovered: f64,
    /// Net rent cost (paid - recovered)
    pub net_rent_cost: f64,
    /// Number of accounts created
    pub accounts_created: u32,
    /// Number of accounts closed
    pub accounts_closed: u32,
}

/// Account lifecycle tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountLifecycle {
    /// Newly created accounts
    pub created_accounts: Vec<CreatedAccount>,
    /// Closed accounts
    pub closed_accounts: Vec<ClosedAccount>,
    /// Modified accounts
    pub modified_accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedAccount {
    pub account: String,
    pub account_type: AccountType,
    pub mint: Option<String>,
    pub owner: String,
    pub rent_paid: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedAccount {
    pub account: String,
    pub account_type: AccountType,
    pub rent_recovered: f64,
    pub final_balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountType {
    TokenAccount,
    AssociatedTokenAccount,
    NativeTokenAccount,
    ProgramAccount,
    Unknown,
}

// =============================================================================
// STANDARD RENT AMOUNTS
// =============================================================================

/// Standard rent amounts for different account types (in lamports)
const STANDARD_RENTS: &[(u64, &str)] = &[
    (2039280, "Standard ATA"),     // Most common ATA rent
    (1461600, "Token Account"),    // Basic token account
    (890880, "Mint Account"),      // Token mint account
    (1002240, "Multisig Account"), // Multisig account
    (5616720, "Metadata Account"), // NFT metadata account
];

/// Maximum expected rent for validation (10 SOL)
const MAX_EXPECTED_RENT: u64 = 10_000_000_000;

// =============================================================================
// MAIN ANALYSIS FUNCTION
// =============================================================================

/// Analyze ATA operations and rent calculations
pub async fn analyze_ata_operations(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<AtaAnalysis, String> {
    logger::debug(
        LogTag::Transactions,
        &format!("Analyzing ATA operations for tx: {}", transaction.signature),
    );

    // Step 1: Extract ATA operations from instructions
    let ata_operations = extract_ata_operations(transaction, tx_data).await?;

    // Step 2: Calculate rent summary
    let rent_summary = calculate_rent_summary(&ata_operations);

    // Step 3: Track account lifecycle
    let account_lifecycle = track_account_lifecycle(&ata_operations, tx_data).await?;

    // Step 4: Calculate analysis confidence
    let confidence = calculate_ata_confidence(&ata_operations, &rent_summary);

    Ok(AtaAnalysis {
        ata_operations,
        rent_summary,
        account_lifecycle,
        confidence,
    })
}

// =============================================================================
// ATA OPERATION EXTRACTION
// =============================================================================

/// Extract ATA operations from transaction instructions
async fn extract_ata_operations(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<Vec<AtaOperation>, String> {
    let mut operations = Vec::new();

    // Extract from balance changes (rent payments/recoveries)
    operations.extend(extract_from_balance_changes(transaction, tx_data).await?);

    // Extract from instruction analysis
    operations.extend(extract_from_instructions(tx_data).await?);

    // Remove duplicates and consolidate
    let consolidated_operations = consolidate_operations(operations);

    Ok(consolidated_operations)
}

/// Extract ATA operations from balance changes (rent detection)
async fn extract_from_balance_changes(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<Vec<AtaOperation>, String> {
    let mut operations = Vec::new();

    // Get balance changes
    let message = &tx_data.transaction.message;
    let account_keys = extract_account_keys(message);

    let empty_pre_balances = Vec::new();
    let empty_post_balances = Vec::new();
    let pre_balances = tx_data
        .meta
        .as_ref()
        .and_then(|m| Some(m.pre_balances.as_ref()))
        .unwrap_or(&empty_pre_balances);

    let post_balances = tx_data
        .meta
        .as_ref()
        .and_then(|m| Some(m.post_balances.as_ref()))
        .unwrap_or(&empty_post_balances);
    // Align to minimum length to handle LUT keys present in balances
    let min_len = std::cmp::min(
        account_keys.len(),
        std::cmp::min(pre_balances.len(), post_balances.len()),
    );
    let account_keys = account_keys.into_iter().take(min_len).collect::<Vec<_>>();
    let pre_balances = pre_balances
        .into_iter()
        .take(min_len)
        .cloned()
        .collect::<Vec<_>>();
    let post_balances = post_balances
        .into_iter()
        .take(min_len)
        .cloned()
        .collect::<Vec<_>>();

    for (i, account_key) in account_keys.iter().enumerate() {
        let pre_balance = pre_balances[i];
        let post_balance = post_balances[i];
        let change_lamports = (post_balance as i64) - (pre_balance as i64);

        // Check for rent patterns - restrict to ATA accounts by lifecycle heuristics
        if let Some(_rent_type) = identify_rent_pattern(change_lamports.abs() as u64) {
            // Classify as Create only when the account was zero before (new account funded with rent)
            if change_lamports > 0 && pre_balance == 0 {
                operations.push(AtaOperation {
                    operation_type: AtaOperationType::Create,
                    account_address: account_key.clone(),
                    mint: None,
                    owner: None,
                    rent_amount: lamports_to_sol(change_lamports.abs() as u64),
                    success: true,
                });
                logger::debug(
                    LogTag::Transactions,
                    &format!(
                        "account={} change_lamports={} type=Create",
                        account_key, change_lamports
                    ),
                );
                continue;
            }

            // Classify as Close only when the account goes to zero (rent reclaimed on close)
            if change_lamports < 0 && post_balance == 0 {
                operations.push(AtaOperation {
                    operation_type: AtaOperationType::Close,
                    account_address: account_key.clone(),
                    mint: None,
                    owner: None,
                    rent_amount: lamports_to_sol(change_lamports.abs() as u64),
                    success: true,
                });
                logger::debug(
                    LogTag::Transactions,
                    &format!(
                        "account={} change_lamports={} type=Close",
                        account_key, change_lamports
                    ),
                );
                continue;
            }
        }
    }

    Ok(operations)
}

/// Extract ATA operations from instruction analysis
async fn extract_from_instructions(
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<Vec<AtaOperation>, String> {
    let mut operations = Vec::new();

    let message = &tx_data.transaction.message;
    let account_keys = extract_account_keys(message);

    // Check outer instructions
    if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
        for instruction in instructions {
            if let Some(operation) = analyze_instruction_for_ata(instruction, &account_keys).await?
            {
                operations.push(operation);
            }
        }
    }

    // Check inner instructions
    if let Some(meta) = &tx_data.meta {
        if let Some(inner_instructions) = &meta.inner_instructions {
            for inner_ix_group in inner_instructions {
                if let Some(instructions) = inner_ix_group
                    .get("instructions")
                    .and_then(|v| v.as_array())
                {
                    for inner_ix in instructions {
                        if let Some(operation) =
                            analyze_instruction_for_ata(inner_ix, &account_keys).await?
                        {
                            operations.push(operation);
                        }
                    }
                }
            }
        }
    }

    Ok(operations)
}

/// Analyze individual instruction for ATA operations
async fn analyze_instruction_for_ata(
    instruction: &Value,
    account_keys: &[String],
) -> Result<Option<AtaOperation>, String> {
    let program_id_index = instruction
        .get("programIdIndex")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    if program_id_index >= account_keys.len() {
        return Ok(None);
    }

    let program_id = &account_keys[program_id_index];

    // Check for ATA program
    if program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
        // ATA operations are better detected from balance changes (more accurate account info)
        // Skip instruction-based detection to avoid duplicates
        return Ok(None);
    }

    // Check for Token program operations
    if program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" {
        // Parse instruction data for token operations
        return parse_token_instruction(instruction, account_keys).await;
    }

    Ok(None)
}

/// Parse Token program instruction for ATA-related operations
async fn parse_token_instruction(
    instruction: &Value,
    account_keys: &[String],
) -> Result<Option<AtaOperation>, String> {
    // This would implement detailed Token program instruction parsing
    // For now, return a placeholder
    Ok(None)
}

// =============================================================================
// OPERATION CONSOLIDATION
// =============================================================================

/// Consolidate and deduplicate ATA operations
fn consolidate_operations(operations: Vec<AtaOperation>) -> Vec<AtaOperation> {
    let mut consolidated = Vec::new();
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();

    for operation in operations {
        // Create a unique key from account + operation type + rent amount
        // This prevents duplicates from balance analysis + instruction parsing
        let key = format!(
            "{}::{:?}::{}",
            operation.account_address,
            operation.operation_type,
            (operation.rent_amount * 1_000_000_000.0).round() as u64
        );

        if !seen_keys.contains(&key) {
            seen_keys.insert(key);
            consolidated.push(operation);
        }
    }

    consolidated
}

// =============================================================================
// RENT ANALYSIS
// =============================================================================

/// Calculate comprehensive rent summary
fn calculate_rent_summary(operations: &[AtaOperation]) -> RentSummary {
    let mut total_rent_paid = 0.0;
    let mut total_rent_recovered = 0.0;
    let mut accounts_created = 0;
    let mut accounts_closed = 0;

    for operation in operations {
        match operation.operation_type {
            AtaOperationType::Create | AtaOperationType::Initialize => {
                total_rent_paid += operation.rent_amount;
                accounts_created += 1;
            }
            AtaOperationType::Close => {
                total_rent_recovered += operation.rent_amount;
                accounts_closed += 1;
            }
            _ => {}
        }
    }

    let net_rent_cost = total_rent_paid - total_rent_recovered;

    RentSummary {
        total_rent_paid,
        total_rent_recovered,
        net_rent_cost,
        accounts_created,
        accounts_closed,
    }
}

/// Identify rent pattern from lamport amount
fn identify_rent_pattern(lamports: u64) -> Option<&'static str> {
    if lamports > MAX_EXPECTED_RENT {
        return None; // Too large to be rent
    }

    for (rent_amount, description) in STANDARD_RENTS {
        if lamports == *rent_amount {
            return Some(description);
        }
    }

    // Check for close matches (within ~150k lamports ≈ 0.00015 SOL)
    for (rent_amount, description) in STANDARD_RENTS {
        if ((lamports as i64) - (*rent_amount as i64)).abs() <= 150_000 {
            return Some(description);
        }
    }

    None
}

// =============================================================================
// ACCOUNT LIFECYCLE TRACKING
// =============================================================================

/// Track account creation and closure lifecycle
async fn track_account_lifecycle(
    operations: &[AtaOperation],
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<AccountLifecycle, String> {
    let mut created_accounts = Vec::new();
    let mut closed_accounts = Vec::new();
    let mut modified_accounts = Vec::new();

    for operation in operations {
        match operation.operation_type {
            AtaOperationType::Create => {
                created_accounts.push(CreatedAccount {
                    account: operation.account_address.clone(),
                    account_type: AccountType::AssociatedTokenAccount,
                    mint: operation.mint.clone(),
                    owner: operation.owner.clone().unwrap_or_default(),
                    rent_paid: operation.rent_amount,
                });
            }
            AtaOperationType::Close => {
                closed_accounts.push(ClosedAccount {
                    account: operation.account_address.clone(),
                    account_type: AccountType::AssociatedTokenAccount,
                    rent_recovered: operation.rent_amount,
                    final_balance: 0.0, // Simplified
                });
            }
            _ => {
                modified_accounts.push(operation.account_address.clone());
            }
        }
    }

    Ok(AccountLifecycle {
        created_accounts,
        closed_accounts,
        modified_accounts,
    })
}

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

/// Calculate ATA analysis confidence
fn calculate_ata_confidence(operations: &[AtaOperation], rent_summary: &RentSummary) -> f64 {
    let mut score = 0.0;
    let mut factors = 0;

    // Factor 1: Operations detected
    if !operations.is_empty() {
        score += 0.4;
    }
    factors += 1;

    // Factor 2: Rent amounts match known patterns
    let valid_rent_operations = operations
        .iter()
        .filter(|op| identify_rent_pattern((op.rent_amount * 1_000_000_000.0) as u64).is_some())
        .count();

    if operations.is_empty() {
        score += 0.3;
    } else {
        let rent_accuracy = (valid_rent_operations as f64) / (operations.len() as f64);
        score += 0.3 * rent_accuracy;
    }
    factors += 1;

    // Factor 3: Reasonable rent amounts
    if rent_summary.net_rent_cost >= 0.0 && rent_summary.net_rent_cost <= 0.1 {
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

/// Extract account keys from transaction message
fn extract_account_keys(message: &Value) -> Vec<String> {
    // accountKeys can be:
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
