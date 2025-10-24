// P&L calculation module - Profit/loss analysis with fee adjustments
//
// This module implements the DexScreener standard for P&L calculations,
// properly accounting for fees, tips, rent, and DEX-specific adjustments.
//
// P&L calculation methodology:
// - For buys: subtract fees/tips from SOL amount spent
// - For sells: add back fees/tips to SOL amount received
// - Account for rent costs in ATA operations
// - Apply DEX-specific fee structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{
    ata::AtaAnalysis,
    balance::BalanceAnalysis,
    classify::{ClassifiedType, SwapDirection, TransactionClass},
};
use crate::logger::{self, LogTag};
use crate::transactions::types::*;

// =============================================================================
// P&L ANALYSIS TYPES
// =============================================================================

/// Comprehensive P&L analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLAnalysis {
    /// Main P&L calculation for the transaction
    pub main_pnl: Option<SwapPnL>,
    /// Individual swap components (for multi-hop)
    pub swap_components: Vec<SwapComponent>,
    /// Fee breakdown
    pub fee_breakdown: FeeBreakdown,
    /// Net transaction cost
    pub net_cost: NetTransactionCost,
    /// Analysis confidence
    pub confidence: f64,
}

/// P&L for a swap operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPnL {
    /// Token being bought/sold
    pub token_mint: String,
    /// Token amount
    pub token_amount: f64,
    /// Token decimals
    pub token_decimals: u8,
    /// SOL amount (adjusted for fees)
    pub sol_amount_adjusted: f64,
    /// Raw SOL amount (before adjustments)
    pub sol_amount_raw: f64,
    /// Price per token in SOL
    pub price_per_token: f64,
    /// Swap direction
    pub direction: SwapDirection,
    /// DEX used
    pub dex: Option<String>,
}

/// Individual swap component (for complex transactions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapComponent {
    /// Input token
    pub input_token: String,
    /// Output token
    pub output_token: String,
    /// Input amount
    pub input_amount: f64,
    /// Output amount
    pub output_amount: f64,
    /// DEX/router used
    pub dex: Option<String>,
}

/// Detailed fee breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    /// Base transaction fee
    pub base_fee: f64,
    /// Priority fee (tip to validators)
    pub priority_fee: f64,
    /// MEV/Jito tips
    pub mev_tips: f64,
    /// DEX swap fees
    pub swap_fees: f64,
    /// ATA rent costs
    pub rent_costs: f64,
    /// Total fees
    pub total_fees: f64,
}

/// Net cost of the entire transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetTransactionCost {
    /// SOL spent on fees and rent
    pub sol_fees_and_rent: f64,
    /// SOL spent/received in swaps (net)
    pub sol_swap_net: f64,
    /// Total SOL impact
    pub total_sol_impact: f64,
    /// Gas efficiency (transaction value / fees)
    pub gas_efficiency: Option<f64>,
}

// =============================================================================
// MAIN P&L CALCULATION
// =============================================================================

/// Calculate comprehensive P&L with fee adjustments
pub async fn calculate_pnl(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
    classification: &TransactionClass,
    ata_analysis: &AtaAnalysis,
) -> Result<PnLAnalysis, String> {
    logger::debug(
        LogTag::Transactions,
            &format!("Calculating P&L for tx: {}", transaction.signature),
        );

    // Step 1: Calculate fee breakdown
    let fee_breakdown = calculate_fee_breakdown(tx_data, balance_analysis, ata_analysis).await?;

    // Step 2: Calculate main P&L based on transaction type
    let main_pnl =
        calculate_main_swap_pnl(balance_analysis, classification, &fee_breakdown).await?;

    // Step 3: Extract swap components for complex transactions
    let swap_components = extract_swap_components(balance_analysis, classification).await?;

    // Step 4: Calculate net transaction cost
    let net_cost = calculate_net_cost(&fee_breakdown, &main_pnl, balance_analysis);

    // Step 5: Calculate confidence score
    let confidence = calculate_pnl_confidence(&main_pnl, &fee_breakdown, balance_analysis);

    Ok(PnLAnalysis {
        main_pnl,
        swap_components,
        fee_breakdown,
        net_cost,
        confidence,
    })
}

// =============================================================================
// FEE BREAKDOWN CALCULATION
// =============================================================================

/// Calculate detailed fee breakdown
async fn calculate_fee_breakdown(
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
    ata_analysis: &AtaAnalysis,
) -> Result<FeeBreakdown, String> {
    // Base signature fee and priority fee split from meta.fee, with override from ComputeBudget parsing
    let (mut base_fee, mut priority_fee) = if let Some(meta) = &tx_data.meta {
        const SIGNATURE_FEE_LAMPORTS: u64 = 5_000; // per-signature base fee
        let sig_count = tx_data.transaction.signatures.len() as u64;
        let base_sig_lamports = SIGNATURE_FEE_LAMPORTS.saturating_mul(sig_count);
        if meta.fee >= base_sig_lamports {
            let priority_lamports = meta.fee - base_sig_lamports;
            (
                (base_sig_lamports as f64) / 1_000_000_000.0,
                (priority_lamports as f64) / 1_000_000_000.0,
            )
        } else {
            ((meta.fee as f64) / 1_000_000_000.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    // Try to parse ComputeBudget instructions (both parsed and raw) for precise priority fee
    let mut cu_limit: Option<u64> = None;
    let mut cu_price_micro_lamports: Option<u64> = None;

    let mut consider_ix = |ix: &serde_json::Value| {
        // Prefer parsed form if available
        if let Some(parsed) = ix.get("parsed") {
            if let Some(ix_type) = parsed.get("type").and_then(|v| v.as_str()) {
                if let Some(info) = parsed.get("info") {
                    match ix_type {
                        "setComputeUnitLimit" => {
                            if let Some(units) = info.get("units").and_then(|v| v.as_u64()) {
                                cu_limit = Some(units);
                            }
                        }
                        "setComputeUnitPrice" => {
                            if let Some(price) = info.get("microLamports").and_then(|v| v.as_u64())
                            {
                                cu_price_micro_lamports = Some(price);
                            }
                        }
                        _ => {}
                    }
                }
            }
            return;
        }

        // Fall back to raw data decoding when parsed is absent
        let program_id = ix.get("programId").and_then(|v| v.as_str()).unwrap_or("");
        if program_id == "ComputeBudget111111111111111111111111111111" {
            if let Some(data_b58) = ix.get("data").and_then(|v| v.as_str()) {
                if let Ok(bytes) = bs58::decode(data_b58).into_vec() {
                    // Tags based on Solana ComputeBudget program:
                    // 2 = SetComputeUnitLimit { units: u32 }
                    // 3 = SetComputeUnitPrice { micro_lamports: u64 }
                    if let Some((&tag, rest)) = bytes.split_first() {
                        match tag {
                            2 => {
                                if rest.len() >= 4 {
                                    let units =
                                        u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]])
                                            as u64;
                                    cu_limit = Some(units);
                                }
                            }
                            3 => {
                                if rest.len() >= 8 {
                                    let price = u64::from_le_bytes([
                                        rest[0], rest[1], rest[2], rest[3], rest[4], rest[5],
                                        rest[6], rest[7],
                                    ]);
                                    cu_price_micro_lamports = Some(price);
                                }
                            }
                            _ => {}
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

    if let Some(price_micro) = cu_price_micro_lamports {
        // Use computeUnitsConsumed if available; else fall back to set limit
        let units = tx_data
            .meta
            .as_ref()
            .and_then(|m| m.compute_units_consumed)
            .or(cu_limit)
            .unwrap_or(0);
        let prio_lamports = price_micro.saturating_mul(units) / 1_000_000; // micro-lamports -> lamports
        priority_fee = (prio_lamports as f64) / 1_000_000_000.0;
        // Recompute base fee from total meta.fee if available
        if let Some(meta) = &tx_data.meta {
            let total = (meta.fee as f64) / 1_000_000_000.0;
            // Ensure non-negative base
            base_fee = (total - priority_fee).max(0.0);
        }
    }
    // MEV tips detected from explicit system transfers to known tip accounts
    // Prefer balance analysis value; if zero, fall back to instruction scan
    let mut mev_tips = balance_analysis.total_tips;
    if mev_tips <= f64::EPSILON {
        let scanned = detect_mev_tips_from_instructions(tx_data);
        if scanned > 0.0 {
            mev_tips = scanned;
        }
    }

    // DEX swap fees (estimated based on platform)
    let swap_fees = estimate_swap_fees(balance_analysis, tx_data).await?;

    // ATA rent costs
    let rent_costs = ata_analysis.rent_summary.net_rent_cost;

    let total_fees = base_fee + priority_fee + mev_tips + rent_costs + swap_fees;

    Ok(FeeBreakdown {
        base_fee,
        priority_fee,
        mev_tips,
        swap_fees,
        rent_costs,
        total_fees,
    })
}

/// Detect total MEV/Jito tips by scanning parsed outer and inner instructions (dup from balance)
fn detect_mev_tips_from_instructions(tx_data: &crate::rpc::TransactionDetails) -> f64 {
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

/// Estimate swap fees based on DEX and transaction patterns
async fn estimate_swap_fees(
    balance_analysis: &BalanceAnalysis,
    tx_data: &crate::rpc::TransactionDetails,
) -> Result<f64, String> {
    // This would implement DEX-specific fee calculation
    // For now, return a reasonable estimate based on transfer amounts

    let total_sol_transfers: f64 = balance_analysis
        .sol_changes
        .values()
        .map(|change| change.change.abs())
        .sum();

    // Estimate 0.1% fee for most DEXes
    Ok(total_sol_transfers * 0.001)
}

// =============================================================================
// MAIN SWAP P&L CALCULATION
// =============================================================================

/// Calculate main swap P&L with fee adjustments
async fn calculate_main_swap_pnl(
    balance_analysis: &BalanceAnalysis,
    classification: &TransactionClass,
    fee_breakdown: &FeeBreakdown,
) -> Result<Option<SwapPnL>, String> {
    // Only calculate P&L for swap-type transactions
    if !matches!(
        classification.transaction_type,
        ClassifiedType::Buy | ClassifiedType::Sell | ClassifiedType::Swap
    ) {
        return Ok(None);
    }

    let direction = classification
        .direction
        .as_ref()
        .ok_or("Missing swap direction")?;
    let token_mint = classification
        .primary_token
        .as_ref()
        .ok_or("Missing primary token")?;

    // Find the largest token change for this mint
    let token_change = find_largest_token_change(balance_analysis, token_mint)?;

    // Find the corresponding SOL change
    let sol_change = find_corresponding_sol_change(balance_analysis, &token_change)?;

    // Apply fee adjustments based on direction
    let sol_amount_adjusted = match direction {
        SwapDirection::SolToToken => {
            // Buy: subtract fees from SOL spent
            sol_change.abs() - fee_breakdown.total_fees
        }
        SwapDirection::TokenToSol => {
            // Sell: add back fees to SOL received
            sol_change.abs() + fee_breakdown.total_fees
        }
        SwapDirection::TokenToToken => {
            // Token-to-token: use raw amount
            sol_change.abs()
        }
    };

    let price_per_token = if token_change.change.abs() > 0.0 {
        sol_amount_adjusted / token_change.change.abs()
    } else {
        0.0
    };

    Ok(Some(SwapPnL {
        token_mint: token_mint.clone(),
        token_amount: token_change.change.abs(),
        token_decimals: token_change.decimals,
        sol_amount_adjusted,
        sol_amount_raw: sol_change.abs(),
        price_per_token,
        direction: direction.clone(),
        dex: None, // Would be filled from DEX detection
    }))
}

/// Find the largest token balance change for a specific mint
fn find_largest_token_change(
    balance_analysis: &BalanceAnalysis,
    target_mint: &str,
) -> Result<TokenBalanceChange, String> {
    let mut largest_change: Option<TokenBalanceChange> = None;
    let mut largest_amount = 0.0;

    for changes in balance_analysis.token_changes.values() {
        for change in changes {
            if change.mint == target_mint && change.change.abs() > largest_amount {
                largest_amount = change.change.abs();
                largest_change = Some(change.clone());
            }
        }
    }

    largest_change.ok_or_else(|| format!("No token changes found for mint: {}", target_mint))
}

/// Find the SOL change that corresponds to a token swap
fn find_corresponding_sol_change(
    balance_analysis: &BalanceAnalysis,
    token_change: &TokenBalanceChange,
) -> Result<f64, String> {
    // TODO: Implement proper SOL-token change correlation
    // For now, use the largest SOL change (heuristic)
    if let Some(largest_change) = balance_analysis.sol_changes.values().max_by(|a, b| {
        a.change
            .abs()
            .partial_cmp(&b.change.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        return Ok(largest_change.change);
    } else {
        return Err("No SOL changes found".to_string());
    }
    let largest_sol_change = balance_analysis
        .sol_changes
        .values()
        .max_by(|a, b| {
            a.change
                .abs()
                .partial_cmp(&b.change.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|change| change.change)
        .unwrap_or(0.0);

    Ok(largest_sol_change)
}

// =============================================================================
// SWAP COMPONENTS EXTRACTION
// =============================================================================

/// Extract individual swap components for complex transactions
async fn extract_swap_components(
    balance_analysis: &BalanceAnalysis,
    classification: &TransactionClass,
) -> Result<Vec<SwapComponent>, String> {
    let mut components = Vec::new();

    // For simple swaps, create a single component
    if let (Some(primary_token), Some(direction)) =
        (&classification.primary_token, &classification.direction)
    {
        let sol_mint = "So11111111111111111111111111111111111111112";

        match direction {
            SwapDirection::SolToToken => {
                components.push(SwapComponent {
                    input_token: sol_mint.to_string(),
                    output_token: primary_token.clone(),
                    input_amount: 0.0, // Would be calculated from balance changes
                    output_amount: 0.0,
                    dex: None,
                });
            }
            SwapDirection::TokenToSol => {
                components.push(SwapComponent {
                    input_token: primary_token.clone(),
                    output_token: sol_mint.to_string(),
                    input_amount: 0.0,
                    output_amount: 0.0,
                    dex: None,
                });
            }
            SwapDirection::TokenToToken => {
                if let Some(secondary_token) = &classification.secondary_token {
                    components.push(SwapComponent {
                        input_token: primary_token.clone(),
                        output_token: secondary_token.clone(),
                        input_amount: 0.0,
                        output_amount: 0.0,
                        dex: None,
                    });
                }
            }
        }
    }

    Ok(components)
}

// =============================================================================
// NET COST CALCULATION
// =============================================================================

/// Calculate net cost of the entire transaction
fn calculate_net_cost(
    fee_breakdown: &FeeBreakdown,
    main_pnl: &Option<SwapPnL>,
    balance_analysis: &BalanceAnalysis,
) -> NetTransactionCost {
    let sol_fees_and_rent = fee_breakdown.total_fees;

    // Calculate net SOL impact from swaps
    let sol_swap_net = if let Some(pnl) = main_pnl {
        match pnl.direction {
            SwapDirection::SolToToken => -pnl.sol_amount_adjusted, // SOL spent
            SwapDirection::TokenToSol => pnl.sol_amount_adjusted,  // SOL received
            SwapDirection::TokenToToken => 0.0,                    // No direct SOL impact
        }
    } else {
        // Use raw balance changes if no P&L
        balance_analysis
            .sol_changes
            .values()
            .map(|change| change.change)
            .sum()
    };

    let total_sol_impact = sol_fees_and_rent + sol_swap_net;

    // Calculate gas efficiency (transaction value / fees)
    let gas_efficiency = if fee_breakdown.total_fees > 0.0 {
        Some(sol_swap_net.abs() / fee_breakdown.total_fees)
    } else {
        None
    };

    NetTransactionCost {
        sol_fees_and_rent,
        sol_swap_net,
        total_sol_impact,
        gas_efficiency,
    }
}

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

/// Calculate P&L analysis confidence
fn calculate_pnl_confidence(
    main_pnl: &Option<SwapPnL>,
    fee_breakdown: &FeeBreakdown,
    balance_analysis: &BalanceAnalysis,
) -> f64 {
    let mut score = 0.0;
    let mut factors = 0;

    // Factor 1: P&L calculation success
    if main_pnl.is_some() {
        score += 0.4;
    }
    factors += 1;

    // Factor 2: Fee breakdown completeness
    if fee_breakdown.total_fees > 0.0 {
        score += 0.3;
    }
    factors += 1;

    // Factor 3: Balance analysis quality
    score += 0.3 * balance_analysis.confidence;
    factors += 1;

    if factors > 0 {
        score / (factors as f64)
    } else {
        0.0
    }
}
