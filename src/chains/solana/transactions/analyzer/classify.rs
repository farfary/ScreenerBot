//! Transaction classification — categorizes transactions by type (swap, transfer, etc.).
//
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

use super::classify_patterns::{
    calculate_classification_confidence, calculate_flow_confidence, classify_from_patterns,
    detect_flow_patterns,
};
use super::{balance::BalanceAnalysis, dex::DexAnalysis, AnalysisConfidence};
use crate::logger::{self, LogTag};
use crate::transactions::types::*;

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
    tx_data: &crate::chains::solana::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
) -> Result<TransactionClass, String> {
    logger::debug(
        LogTag::Transactions,
        &format!("Classifying transaction: {}", transaction.signature),
    );

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
    logger::debug(
        LogTag::Transactions,
        &format!(
            "classification={:?} direction={:?} primary_token={:?} confidence={:?}",
            classification.0, classification.1, classification.2, confidence
        ),
    );

    // Debug: summarize nodes, edges, and patterns
    logger::debug(
        LogTag::Transactions,
        &format!(
            "Flow graph: nodes={} edges={} patterns={} dex_conf={:.2}",
            flow_analysis.nodes.len(),
            flow_analysis.edges.len(),
            flow_patterns.len(),
            dex_analysis.confidence
        ),
    );

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
