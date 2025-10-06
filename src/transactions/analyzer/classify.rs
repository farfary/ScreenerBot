// Transaction classification module - Graph-based flow analysis
//
// This module implements the sophisticated classification system used by
// SolScan and DexScreener to determine transaction types (Buy/Sell/Transfer/etc)
// using graph-based flow resolution and confidence scoring.
//
// Classification strategy:
// 1. Build directed flow graph from balance changes
// 2. Detect swap patterns: SOL->token (buy), token->SOL (sell)
// 3. Identify complex patterns: LP operations, multi-hop swaps
// 4. Apply confidence thresholds for reliable classification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{balance::BalanceAnalysis, dex::DexAnalysis, AnalysisConfidence};
use crate::global::is_debug_transactions_enabled;
use crate::logger::{log, LogTag};
use crate::transactions::types::*;
use crate::transactions::utils::WSOL_MINT;

// =============================================================================
// CLASSIFICATION TYPES
// =============================================================================

/// Transaction classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionClass {
    /// Classified transaction type
    pub transaction_type: ClassifiedType,
    /// Direction for swaps (buy/sell)
    pub direction: Option<SwapDirection>,
    /// Primary token involved (for swaps)
    pub primary_token: Option<String>,
    /// Secondary token involved (for complex operations)
    pub secondary_token: Option<String>,
    /// Classification confidence
    pub confidence: AnalysisConfidence,
    /// Detailed flow analysis
    pub flow_analysis: FlowAnalysis,
}

/// Classified transaction types following industry standards
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClassifiedType {
    /// Token purchase with SOL
    Buy,
    /// Token sale for SOL
    Sell,
    /// Direct token-to-token swap
    Swap,
    /// SOL or token transfer
    Transfer,
    /// Liquidity provision
    AddLiquidity,
    /// Liquidity removal
    RemoveLiquidity,
    /// NFT operations
    NftOperation,
    /// Program deployment/interaction
    ProgramInteraction,
    /// Failed transaction
    Failed,
    /// Cannot classify reliably
    Unknown,
}

/// Swap direction for buy/sell operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwapDirection {
    SolToToken,   // Buy: SOL -> Token
    TokenToSol,   // Sell: Token -> SOL
    TokenToToken, // Swap: Token A -> Token B
}

/// Flow analysis result showing transfer patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowAnalysis {
    /// Detected flow patterns
    pub flow_patterns: Vec<FlowPattern>,
    /// Graph nodes (accounts with changes)
    pub nodes: Vec<FlowNode>,
    /// Graph edges (transfers between accounts)
    pub edges: Vec<FlowEdge>,
    /// Flow confidence score
    pub flow_confidence: f64,
}

/// Individual flow pattern detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPattern {
    pub pattern_type: PatternType,
    pub from_token: String,
    pub to_token: String,
    pub amount: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    SimpleSwap,
    MultiHopSwap,
    LiquidityAdd,
    LiquidityRemove,
    TokenTransfer,
    SolTransfer,
}

/// Graph node representing an account with changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub account: String,
    pub sol_change: f64,
    pub token_changes: HashMap<String, f64>,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Wallet,   // User wallet
    Pool,     // Liquidity pool
    Router,   // DEX router
    Treasury, // Protocol treasury
    Unknown,
}

/// Graph edge representing a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub from_account: String,
    pub to_account: String,
    pub token: String,
    pub amount: f64,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Swap,
    Transfer,
    Fee,
    Rent,
}

// =============================================================================
// MAIN CLASSIFICATION FUNCTION
// =============================================================================

/// Classify transaction using graph-based flow analysis
pub async fn classify_transaction(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
) -> Result<TransactionClass, String> {
    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "CLASSIFY",
            &format!("Classifying transaction: {}", transaction.signature),
        );
    }

    // Step 1: Build flow graph from balance changes
    let flow_analysis = build_flow_graph(balance_analysis, dex_analysis).await?;

    // Step 2: Detect flow patterns
    let flow_patterns = detect_flow_patterns(&flow_analysis, tx_data, dex_analysis).await?;

    // Step 3: Classify based on dominant pattern
    let classification = classify_from_patterns(&flow_patterns, dex_analysis).await?;

    // Step 4: Calculate overall confidence
    let confidence =
        calculate_classification_confidence(&classification, &flow_analysis, dex_analysis);

    // Decision summary
    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "CLASSIFY_DECISION",
            &format!(
                "classification={:?} direction={:?} primary_token={:?} confidence={:?}",
                classification.0, classification.1, classification.2, confidence
            ),
        );

        // Debug: summarize nodes, edges, and patterns
        log(
            LogTag::Transactions,
            "CLASSIFY_DEBUG",
            &format!(
                "Flow graph: nodes={} edges={} patterns={} dex_conf={:.2}",
                flow_analysis.nodes.len(),
                flow_analysis.edges.len(),
                flow_patterns.len(),
                dex_analysis.confidence
            ),
        );
    }

    Ok(TransactionClass {
        transaction_type: classification.0,
        direction: classification.1,
        primary_token: classification.2,
        secondary_token: classification.3,
        confidence,
        flow_analysis: FlowAnalysis {
            flow_patterns,
            nodes: flow_analysis.nodes,
            edges: flow_analysis.edges,
            flow_confidence: flow_analysis.flow_confidence,
        },
    })
}

// =============================================================================
// FLOW GRAPH CONSTRUCTION
// =============================================================================

/// Build directed flow graph from balance changes
async fn build_flow_graph(
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
) -> Result<FlowAnalysis, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Step 1: Create nodes from accounts with balance changes
    for (account, sol_change) in &balance_analysis.sol_changes {
        let token_changes = balance_analysis
            .token_changes
            .get(account)
            .map(|changes| changes.iter().map(|c| (c.mint.clone(), c.change)).collect())
            .unwrap_or_default();

        let node_type = classify_node_type(account, dex_analysis);

        nodes.push(FlowNode {
            account: account.clone(),
            sol_change: sol_change.change,
            token_changes,
            node_type,
        });
    }

    // Step 2: Create edges from clean transfers
    for transfer in &balance_analysis.clean_transfers {
        let edge_type = match transfer.transfer_type {
            super::balance::TransferType::SolTransfer => EdgeType::Transfer,
            super::balance::TransferType::TokenTransfer => EdgeType::Transfer,
            super::balance::TransferType::SwapLeg => EdgeType::Swap,
            super::balance::TransferType::LiquidityOp => EdgeType::Transfer,
        };

        edges.push(FlowEdge {
            from_account: transfer.from_account.clone(),
            to_account: transfer.to_account.clone(),
            token: transfer.mint.clone(),
            amount: transfer.amount,
            edge_type,
        });
    }

    // Step 3: Calculate flow confidence
    let flow_confidence = calculate_flow_confidence(&nodes, &edges);

    Ok(FlowAnalysis {
        flow_patterns: Vec::new(), // Will be filled in next step
        nodes,
        edges,
        flow_confidence,
    })
}

/// Classify what type of account this is based on patterns
fn classify_node_type(account: &str, dex_analysis: &DexAnalysis) -> NodeType {
    // Check if it's a known DEX program
    if dex_analysis.program_ids.contains(&account.to_string()) {
        return NodeType::Router;
    }

    // Check if it's a pool address
    if let Some(pool_addr) = &dex_analysis.pool_address {
        if pool_addr == account {
            return NodeType::Pool;
        }
    }

    // Heuristics for other account types
    if account.len() == 44 {
        // Could be wallet or other account
        NodeType::Wallet
    } else {
        NodeType::Unknown
    }
}

// =============================================================================
// PATTERN DETECTION
// =============================================================================

/// Detect flow patterns from the constructed graph
async fn detect_flow_patterns(
    flow_analysis: &FlowAnalysis,
    tx_data: &crate::rpc::TransactionDetails,
    dex_analysis: &DexAnalysis,
) -> Result<Vec<FlowPattern>, String> {
    let mut patterns = Vec::new();

    // Look for swap patterns: SOL out + Token in (buy) or Token out + SOL in (sell)
    patterns.extend(detect_swap_patterns(flow_analysis).await?);

    // Look for transfer patterns: Single token movement
    patterns.extend(detect_transfer_patterns(flow_analysis).await?);

    // Look for liquidity patterns: Multiple tokens in/out to pool
    patterns.extend(detect_liquidity_patterns(flow_analysis).await?);

    // Instruction-aware fallback: if no clear swap patterns were found yet, but we
    // observe inner transferChecked credits of WSOL, infer a token->SOL sell by
    // pairing the largest negative non-WSOL token change with SOL out.
    if !patterns
        .iter()
        .any(|p| matches!(p.pattern_type, PatternType::SimpleSwap))
    {
        let wsol_ui = sum_inner_wsol_transferchecked_ui(tx_data);
        if wsol_ui > 0.0 {
            let sol_mint = WSOL_MINT;
            // Find the most likely sold token: largest-magnitude negative change (non-WSOL)
            let mut best_token: Option<(String, f64)> = None;
            for node in &flow_analysis.nodes {
                for (token_mint, token_change) in &node.token_changes {
                    if token_mint == sol_mint {
                        continue;
                    }
                    if *token_change < 0.0 {
                        let cand = (*token_mint).to_string();
                        let amt = token_change.abs();
                        if let Some((_, best_amt)) = &best_token {
                            if amt > *best_amt {
                                best_token = Some((cand, amt));
                            }
                        } else {
                            best_token = Some((cand, amt));
                        }
                    }
                }
            }
            if let Some((mint, amt)) = best_token {
                patterns.push(FlowPattern {
                    pattern_type: PatternType::SimpleSwap,
                    from_token: mint,
                    to_token: sol_mint.to_string(),
                    amount: amt, // amount field is informational; direction is what matters here
                    confidence: 0.75,
                });
            }
        }
    }

    // Aggregator-aware fallback: if still no SimpleSwap pattern and a known aggregator
    // (e.g., Jupiter) is present among program IDs, infer a token->SOL sell by pairing the
    // largest negative non-WSOL token change with SOL, even if no WSOL credits are visible.
    if !patterns
        .iter()
        .any(|p| matches!(p.pattern_type, PatternType::SimpleSwap))
    {
        let has_aggregator = {
            // Prioritize explicit detected Jupiter, otherwise check program IDs
            let jup_detected = matches!(
                dex_analysis.detected_dex,
                Some(super::dex::DetectedDex::Jupiter)
            );
            let jup_in_programs = dex_analysis
                .program_ids
                .iter()
                .any(|pid| pid == crate::transactions::program_ids::JUPITER_V6_PROGRAM_ID);
            jup_detected || jup_in_programs
        };

        if has_aggregator {
            let sol_mint = WSOL_MINT;
            // Select the dominant sold token by magnitude of negative change
            let mut best_token: Option<(String, f64)> = None;
            for node in &flow_analysis.nodes {
                for (token_mint, token_change) in &node.token_changes {
                    if token_mint == sol_mint {
                        continue;
                    }
                    if *token_change < 0.0 {
                        let cand = (*token_mint).to_string();
                        let amt = token_change.abs();
                        if let Some((_, best_amt)) = &best_token {
                            if amt > *best_amt {
                                best_token = Some((cand, amt));
                            }
                        } else {
                            best_token = Some((cand, amt));
                        }
                    }
                }
            }

            if let Some((mint, amt)) = best_token {
                patterns.push(FlowPattern {
                    pattern_type: PatternType::SimpleSwap,
                    from_token: mint,
                    to_token: sol_mint.to_string(),
                    amount: amt,
                    confidence: 0.65, // Lower than instruction-backed fallback
                });
            }
        }
    }

    // Aggregator-aware buy fallback: if still no SimpleSwap pattern and a known aggregator
    // is present among program IDs, infer a SOL->token buy by selecting the dominant positive
    // non-WSOL token change even when SOL leg signals are weak or ambiguous.
    if !patterns
        .iter()
        .any(|p| matches!(p.pattern_type, PatternType::SimpleSwap))
    {
        let has_aggregator = {
            let jup_detected = matches!(
                dex_analysis.detected_dex,
                Some(super::dex::DetectedDex::Jupiter)
            );
            let jup_in_programs = dex_analysis
                .program_ids
                .iter()
                .any(|pid| pid == crate::transactions::program_ids::JUPITER_V6_PROGRAM_ID);
            jup_detected || jup_in_programs
        };

        if has_aggregator {
            let sol_mint = WSOL_MINT;
            // Select the dominant bought token by magnitude of positive change
            let mut best_token_in: Option<(String, f64)> = None;
            for node in &flow_analysis.nodes {
                for (token_mint, token_change) in &node.token_changes {
                    if token_mint == sol_mint {
                        continue;
                    }
                    if *token_change > 0.0 {
                        let cand = (*token_mint).to_string();
                        let amt = (*token_change).abs();
                        if let Some((_, best_amt)) = &best_token_in {
                            if amt > *best_amt {
                                best_token_in = Some((cand, amt));
                            }
                        } else {
                            best_token_in = Some((cand, amt));
                        }
                    }
                }
            }

            if let Some((mint, amt)) = best_token_in {
                patterns.push(FlowPattern {
                    pattern_type: PatternType::SimpleSwap,
                    from_token: sol_mint.to_string(),
                    to_token: mint,
                    amount: amt,
                    confidence: 0.6, // Conservative confidence for aggregator-only buy inference
                });
            }
        }
    }

    Ok(patterns)
}

/// Detect swap patterns (buy/sell operations)
async fn detect_swap_patterns(flow_analysis: &FlowAnalysis) -> Result<Vec<FlowPattern>, String> {
    let mut patterns = Vec::new();
    let sol_mint = WSOL_MINT;

    // Find accounts with both SOL and token changes
    for node in &flow_analysis.nodes {
        if node.sol_change.abs() > f64::EPSILON && !node.token_changes.is_empty() {
            for (token_mint, token_change) in &node.token_changes {
                if token_mint == sol_mint {
                    continue; // Skip wrapped SOL
                }

                // Buy pattern: SOL decrease + Token increase
                if node.sol_change < 0.0 && *token_change > 0.0 {
                    patterns.push(FlowPattern {
                        pattern_type: PatternType::SimpleSwap,
                        from_token: sol_mint.to_string(),
                        to_token: token_mint.clone(),
                        amount: token_change.abs(),
                        confidence: 0.8,
                    });
                }

                // Sell pattern: Token decrease + SOL increase
                if *token_change < 0.0 && node.sol_change > 0.0 {
                    patterns.push(FlowPattern {
                        pattern_type: PatternType::SimpleSwap,
                        from_token: token_mint.clone(),
                        to_token: sol_mint.to_string(),
                        amount: token_change.abs(),
                        confidence: 0.8,
                    });
                }
            }
        }
    }

    Ok(patterns)
}

/// Detect transfer patterns (simple movements)
async fn detect_transfer_patterns(
    flow_analysis: &FlowAnalysis,
) -> Result<Vec<FlowPattern>, String> {
    let mut patterns = Vec::new();

    // Look for edges that represent pure transfers
    for edge in &flow_analysis.edges {
        if matches!(edge.edge_type, EdgeType::Transfer) {
            let pattern_type = if edge.token == WSOL_MINT {
                PatternType::SolTransfer
            } else {
                PatternType::TokenTransfer
            };

            patterns.push(FlowPattern {
                pattern_type,
                from_token: edge.token.clone(),
                to_token: edge.token.clone(),
                amount: edge.amount,
                confidence: 0.7,
            });
        }
    }

    Ok(patterns)
}

/// Detect liquidity provision/removal patterns
async fn detect_liquidity_patterns(
    flow_analysis: &FlowAnalysis,
) -> Result<Vec<FlowPattern>, String> {
    let mut patterns = Vec::new();

    // Look for nodes with multiple token changes (LP operations)
    for node in &flow_analysis.nodes {
        if node.token_changes.len() >= 2 && matches!(node.node_type, NodeType::Pool) {
            // Determine if adding or removing liquidity based on change direction
            let positive_changes = node.token_changes.values().filter(|&&v| v > 0.0).count();
            let negative_changes = node.token_changes.values().filter(|&&v| v < 0.0).count();

            if positive_changes > 0 && negative_changes == 0 {
                // All positive = liquidity addition
                for (token, amount) in &node.token_changes {
                    patterns.push(FlowPattern {
                        pattern_type: PatternType::LiquidityAdd,
                        from_token: token.clone(),
                        to_token: "LP_TOKEN".to_string(),
                        amount: amount.abs(),
                        confidence: 0.7,
                    });
                }
            } else if negative_changes > 0 && positive_changes == 0 {
                // All negative = liquidity removal
                for (token, amount) in &node.token_changes {
                    patterns.push(FlowPattern {
                        pattern_type: PatternType::LiquidityRemove,
                        from_token: "LP_TOKEN".to_string(),
                        to_token: token.clone(),
                        amount: amount.abs(),
                        confidence: 0.7,
                    });
                }
            }
        }
    }

    Ok(patterns)
}

// =============================================================================
// CLASSIFICATION FROM PATTERNS
// =============================================================================

/// Classify transaction based on detected patterns
async fn classify_from_patterns(
    patterns: &[FlowPattern],
    dex_analysis: &DexAnalysis,
) -> Result<
    (
        ClassifiedType,
        Option<SwapDirection>,
        Option<String>,
        Option<String>,
    ),
    String,
> {
    if patterns.is_empty() {
        return Ok((ClassifiedType::Unknown, None, None, None));
    }

    // Find the highest confidence pattern
    let dominant_pattern = patterns
        .iter()
        .max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();
    let sol_mint = WSOL_MINT;

    match dominant_pattern.pattern_type {
        PatternType::SimpleSwap => {
            if dominant_pattern.from_token == sol_mint {
                // SOL -> Token = Buy
                Ok((
                    ClassifiedType::Buy,
                    Some(SwapDirection::SolToToken),
                    Some(dominant_pattern.to_token.clone()),
                    None,
                ))
            } else if dominant_pattern.to_token == sol_mint {
                // Token -> SOL = Sell
                Ok((
                    ClassifiedType::Sell,
                    Some(SwapDirection::TokenToSol),
                    Some(dominant_pattern.from_token.clone()),
                    None,
                ))
            } else {
                // Token -> Token = Swap
                Ok((
                    ClassifiedType::Swap,
                    Some(SwapDirection::TokenToToken),
                    Some(dominant_pattern.from_token.clone()),
                    Some(dominant_pattern.to_token.clone()),
                ))
            }
        }
        PatternType::TokenTransfer | PatternType::SolTransfer => Ok((
            ClassifiedType::Transfer,
            None,
            Some(dominant_pattern.from_token.clone()),
            None,
        )),
        PatternType::LiquidityAdd => Ok((
            ClassifiedType::AddLiquidity,
            None,
            Some(dominant_pattern.from_token.clone()),
            None,
        )),
        PatternType::LiquidityRemove => Ok((
            ClassifiedType::RemoveLiquidity,
            None,
            Some(dominant_pattern.to_token.clone()),
            None,
        )),
        PatternType::MultiHopSwap => Ok((
            ClassifiedType::Swap,
            Some(SwapDirection::TokenToToken),
            Some(dominant_pattern.from_token.clone()),
            Some(dominant_pattern.to_token.clone()),
        )),
    }
}

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

/// Calculate overall classification confidence
fn calculate_classification_confidence(
    classification: &(
        ClassifiedType,
        Option<SwapDirection>,
        Option<String>,
        Option<String>,
    ),
    flow_analysis: &FlowAnalysis,
    dex_analysis: &DexAnalysis,
) -> AnalysisConfidence {
    let mut confidence_factors = Vec::new();

    // Factor 1: Flow analysis quality
    confidence_factors.push(flow_analysis.flow_confidence);

    // Factor 2: DEX detection confidence
    confidence_factors.push(dex_analysis.confidence);

    // Factor 3: Pattern clarity (how well-defined the pattern is)
    let pattern_clarity = if matches!(classification.0, ClassifiedType::Unknown) {
        0.2
    } else {
        0.8
    };
    confidence_factors.push(pattern_clarity);

    // Factor 4: Data completeness (do we have all the pieces?)
    let data_completeness = if classification.2.is_some() { 0.8 } else { 0.4 };
    confidence_factors.push(data_completeness);

    // Calculate weighted average
    let average_confidence =
        confidence_factors.iter().sum::<f64>() / (confidence_factors.len() as f64);

    if average_confidence >= 0.8 {
        AnalysisConfidence::High
    } else if average_confidence >= 0.6 {
        AnalysisConfidence::Medium
    } else if average_confidence >= 0.4 {
        AnalysisConfidence::Low
    } else {
        AnalysisConfidence::Unknown
    }
}

/// Calculate flow confidence based on graph quality
fn calculate_flow_confidence(nodes: &[FlowNode], edges: &[FlowEdge]) -> f64 {
    let mut score = 0.0;
    let mut factors = 0;

    // Factor 1: Number of nodes with changes
    if !nodes.is_empty() {
        score += 0.3;
    }
    factors += 1;

    // Factor 2: Number of edges (transfers)
    if !edges.is_empty() {
        score += 0.4;
    }
    factors += 1;

    // Factor 3: Balance between inflows and outflows
    let total_amount: f64 = edges.iter().map(|e| e.amount).sum();
    if total_amount > 0.0 {
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
// INSTRUCTION-AWARE HELPERS (local, lightweight)
// =============================================================================

/// Sum uiAmount across all inner instructions where the parsed info indicates a
/// transferChecked of WSOL (So1111...), returning SOL units.
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
                        if mint == WSOL_MINT {
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
