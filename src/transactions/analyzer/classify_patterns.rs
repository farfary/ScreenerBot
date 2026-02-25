//! Pattern detection and classification helpers for transaction flow analysis.

use super::classify::{
    ClassifiedType, EdgeType, FlowAnalysis, FlowEdge, FlowNode, FlowPattern, NodeType,
    PatternType, SwapDirection,
};
use super::dex::DexAnalysis;
use super::AnalysisConfidence;
use crate::transactions::utils::WSOL_MINT;

// =============================================================================
// PATTERN DETECTION
// =============================================================================

/// Detect flow patterns from the constructed graph
pub(super) async fn detect_flow_patterns(
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
                        to_token: "LP_TOKEN".to_owned(),
                        amount: amount.abs(),
                        confidence: 0.7,
                    });
                }
            } else if negative_changes > 0 && positive_changes == 0 {
                // All negative = liquidity removal
                for (token, amount) in &node.token_changes {
                    patterns.push(FlowPattern {
                        pattern_type: PatternType::LiquidityRemove,
                        from_token: "LP_TOKEN".to_owned(),
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
pub(super) async fn classify_from_patterns(
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
pub(super) fn calculate_classification_confidence(
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
pub(super) fn calculate_flow_confidence(nodes: &[FlowNode], edges: &[FlowEdge]) -> f64 {
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
                        let mint = info
                            .get("mint")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
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
