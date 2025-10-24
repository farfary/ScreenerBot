// Pattern detection module - Suspicious activity and anomaly detection
//
// This module implements pattern recognition for detecting unusual transaction
// patterns, potential security issues, and trading behavior analysis.
//
// Detection patterns:
// - MEV/sandwich attacks
// - Wash trading detection
// - Unusual fee patterns
// - Failed transaction patterns
// - High-frequency trading signatures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{balance::BalanceAnalysis, classify::TransactionClass, dex::DexAnalysis};
use crate::logger::{self, LogTag};
use crate::transactions::types::*;

// =============================================================================
// PATTERN ANALYSIS TYPES
// =============================================================================

/// Comprehensive pattern detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    /// Detected patterns
    pub detected_patterns: Vec<DetectedPattern>,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
    /// Trading behavior indicators
    pub trading_behavior: TradingBehavior,
    /// Analysis confidence
    pub confidence: f64,
}

/// Individual detected pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    /// Type of pattern
    pub pattern_type: PatternType,
    /// Pattern description
    pub description: String,
    /// Confidence in detection
    pub confidence: f64,
    /// Severity level
    pub severity: PatternSeverity,
    /// Supporting evidence
    pub evidence: Vec<String>,
}

/// Types of patterns that can be detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    /// MEV/sandwich attack signatures
    MevActivity,
    /// Wash trading patterns
    WashTrading,
    /// Unusual fee patterns
    FeeAnomaly,
    /// Failed transaction patterns
    FailurePattern,
    /// High-frequency trading
    HighFrequencyTrading,
    /// Large transaction (whale activity)
    WhaleActivity,
    /// Arbitrage patterns
    Arbitrage,
    /// Liquidation activity
    Liquidation,
    /// Suspicious timing
    SuspiciousTiming,
    /// Normal activity
    Normal,
}

/// Pattern severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternSeverity {
    Low,      // Informational
    Medium,   // Worth monitoring
    High,     // Potentially concerning
    Critical, // Likely malicious
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk score (0.0 - 1.0)
    pub risk_score: f64,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Main risk factors
    pub risk_factors: Vec<String>,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Trading behavior indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingBehavior {
    /// Estimated trader type
    pub trader_type: TraderType,
    /// Transaction size category
    pub size_category: SizeCategory,
    /// Trading sophistication level
    pub sophistication: SophisticationLevel,
    /// Frequency indicators
    pub frequency_indicators: FrequencyIndicators,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraderType {
    Retail,      // Small individual trader
    Whale,       // Large holder
    Bot,         // Automated trading
    Arbitrageur, // Arbitrage trader
    Liquidator,  // Liquidation bot
    MevBot,      // MEV bot
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SizeCategory {
    Micro,  // < 0.1 SOL
    Small,  // 0.1 - 1 SOL
    Medium, // 1 - 10 SOL
    Large,  // 10 - 100 SOL
    Whale,  // > 100 SOL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SophisticationLevel {
    Basic,        // Simple swaps
    Intermediate, // Multi-step operations
    Advanced,     // Complex DeFi interactions
    Expert,       // MEV/arbitrage strategies
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyIndicators {
    /// Estimated transactions per hour (if detectable)
    pub estimated_tx_per_hour: Option<f64>,
    /// Timing patterns
    pub timing_patterns: Vec<TimingPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimingPattern {
    RegularIntervals,
    BurstActivity,
    MarketHours,
    OffHours,
    Random,
}

// =============================================================================
// MAIN PATTERN DETECTION
// =============================================================================

/// Detect patterns and analyze trading behavior
pub async fn detect_patterns(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
    classification: &TransactionClass,
) -> Result<PatternAnalysis, String> {
    logger::info(
        LogTag::Transactions,
            &format!("Detecting patterns for tx: {}", transaction.signature),
        );

    // Step 1: Detect specific patterns
    let detected_patterns = detect_specific_patterns(
        transaction,
        tx_data,
        balance_analysis,
        dex_analysis,
        classification,
    )
    .await?;

    // Step 2: Assess risk level
    let risk_assessment = assess_risk(&detected_patterns, balance_analysis).await?;

    // Step 3: Analyze trading behavior
    let trading_behavior =
        analyze_trading_behavior(transaction, balance_analysis, dex_analysis, classification)
            .await?;

    // Step 4: Calculate overall confidence
    let confidence = calculate_pattern_confidence(&detected_patterns, &trading_behavior);

    Ok(PatternAnalysis {
        detected_patterns,
        risk_assessment,
        trading_behavior,
        confidence,
    })
}

// =============================================================================
// SPECIFIC PATTERN DETECTION
// =============================================================================

/// Detect specific patterns from transaction data
async fn detect_specific_patterns(
    transaction: &Transaction,
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
    classification: &TransactionClass,
) -> Result<Vec<DetectedPattern>, String> {
    let mut patterns = Vec::new();

    // Check for MEV activity
    patterns.extend(detect_mev_patterns(transaction, balance_analysis, dex_analysis).await?);

    // Check for wash trading
    patterns.extend(detect_wash_trading(transaction, balance_analysis).await?);

    // Check for fee anomalies
    patterns.extend(detect_fee_anomalies(tx_data, balance_analysis).await?);

    // Check for whale activity
    patterns.extend(detect_whale_activity(balance_analysis).await?);

    // Check for arbitrage patterns
    patterns.extend(detect_arbitrage_patterns(balance_analysis, dex_analysis).await?);

    // If no specific patterns found, mark as normal
    if patterns.is_empty() {
        patterns.push(DetectedPattern {
            pattern_type: PatternType::Normal,
            description: "Normal trading activity".to_string(),
            confidence: 0.8,
            severity: PatternSeverity::Low,
            evidence: vec!["No suspicious patterns detected".to_string()],
        });
    }

    Ok(patterns)
}

/// Detect MEV activity patterns
async fn detect_mev_patterns(
    transaction: &Transaction,
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
) -> Result<Vec<DetectedPattern>, String> {
    let mut patterns = Vec::new();

    // Check for high MEV tips
    if balance_analysis.total_tips > 0.001 {
        patterns.push(DetectedPattern {
            pattern_type: PatternType::MevActivity,
            description: format!(
                "High MEV tip detected: {:.6} SOL",
                balance_analysis.total_tips
            ),
            confidence: 0.8,
            severity: PatternSeverity::Medium,
            evidence: vec![
                format!("MEV tip amount: {:.6} SOL", balance_analysis.total_tips),
                "Above normal tip threshold".to_string(),
            ],
        });
    }

    // Check for multiple DEX interactions (potential arbitrage/MEV)
    if dex_analysis.program_ids.len() > 2 {
        patterns.push(DetectedPattern {
            pattern_type: PatternType::MevActivity,
            description: "Multiple DEX interactions detected".to_string(),
            confidence: 0.6,
            severity: PatternSeverity::Medium,
            evidence: vec![
                format!("DEX programs involved: {}", dex_analysis.program_ids.len()),
                format!("Programs: {:?}", dex_analysis.program_ids),
            ],
        });
    }

    Ok(patterns)
}

/// Detect wash trading patterns
async fn detect_wash_trading(
    transaction: &Transaction,
    balance_analysis: &BalanceAnalysis,
) -> Result<Vec<DetectedPattern>, String> {
    let mut patterns = Vec::new();

    // Look for circular token flows (simplified heuristic)
    let token_accounts: Vec<_> = balance_analysis.token_changes.keys().collect();

    if token_accounts.len() >= 3 {
        // Check if the same account appears in multiple token changes
        for (account, changes) in &balance_analysis.token_changes {
            if changes.len() > 2 {
                patterns.push(DetectedPattern {
                    pattern_type: PatternType::WashTrading,
                    description: "Multiple token changes in single account".to_string(),
                    confidence: 0.4,
                    severity: PatternSeverity::Low,
                    evidence: vec![
                        format!("Account: {}", account),
                        format!("Token changes: {}", changes.len()),
                    ],
                });
            }
        }
    }

    Ok(patterns)
}

/// Detect fee anomalies
async fn detect_fee_anomalies(
    tx_data: &crate::rpc::TransactionDetails,
    balance_analysis: &BalanceAnalysis,
) -> Result<Vec<DetectedPattern>, String> {
    let mut patterns = Vec::new();

    let base_fee = tx_data
        .meta
        .as_ref()
        .map(|m| (m.fee as f64) / 1_000_000_000.0)
        .unwrap_or(0.0);

    // Check for unusually high fees
    if base_fee > 0.01 {
        patterns.push(DetectedPattern {
            pattern_type: PatternType::FeeAnomaly,
            description: format!("Unusually high transaction fee: {:.6} SOL", base_fee),
            confidence: 0.7,
            severity: PatternSeverity::Medium,
            evidence: vec![
                format!("Base fee: {:.6} SOL", base_fee),
                "Above normal fee threshold".to_string(),
            ],
        });
    }

    // Check total tips vs transaction value
    let total_transfer_value: f64 = balance_analysis
        .clean_transfers
        .iter()
        .map(|t| t.amount)
        .sum();

    if balance_analysis.total_tips > 0.0 && total_transfer_value > 0.0 {
        let tip_ratio = balance_analysis.total_tips / total_transfer_value;
        if tip_ratio > 0.05 {
            patterns.push(DetectedPattern {
                pattern_type: PatternType::FeeAnomaly,
                description: "High tip-to-value ratio".to_string(),
                confidence: 0.6,
                severity: PatternSeverity::Low,
                evidence: vec![
                    format!("Tip ratio: {:.2}%", tip_ratio * 100.0),
                    format!("Tips: {:.6} SOL", balance_analysis.total_tips),
                    format!("Transfer value: {:.6} SOL", total_transfer_value),
                ],
            });
        }
    }

    Ok(patterns)
}

/// Detect whale activity
async fn detect_whale_activity(
    balance_analysis: &BalanceAnalysis,
) -> Result<Vec<DetectedPattern>, String> {
    let mut patterns = Vec::new();

    // Check for large SOL amounts
    for change in balance_analysis.sol_changes.values() {
        if change.change.abs() > 100.0 {
            patterns.push(DetectedPattern {
                pattern_type: PatternType::WhaleActivity,
                description: format!("Large SOL movement: {:.2} SOL", change.change.abs()),
                confidence: 0.9,
                severity: PatternSeverity::Low,
                evidence: vec![
                    format!("SOL amount: {:.2}", change.change.abs()),
                    format!("Account: {}", change.account),
                ],
            });
        }
    }

    Ok(patterns)
}

/// Detect arbitrage patterns
async fn detect_arbitrage_patterns(
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
) -> Result<Vec<DetectedPattern>, String> {
    let mut patterns = Vec::new();

    // Arbitrage often involves multiple DEXes
    if dex_analysis.program_ids.len() >= 2 && balance_analysis.clean_transfers.len() >= 4 {
        patterns.push(DetectedPattern {
            pattern_type: PatternType::Arbitrage,
            description: "Potential arbitrage activity".to_string(),
            confidence: 0.6,
            severity: PatternSeverity::Low,
            evidence: vec![
                format!("Multiple DEXes: {}", dex_analysis.program_ids.len()),
                format!(
                    "Multiple transfers: {}",
                    balance_analysis.clean_transfers.len()
                ),
            ],
        });
    }

    Ok(patterns)
}

// =============================================================================
// RISK ASSESSMENT
// =============================================================================

/// Assess overall risk level
async fn assess_risk(
    patterns: &[DetectedPattern],
    balance_analysis: &BalanceAnalysis,
) -> Result<RiskAssessment, String> {
    let mut risk_score = 0.0;
    let mut risk_factors = Vec::new();
    let mut recommendations = Vec::new();

    // Calculate risk score from patterns
    for pattern in patterns {
        let pattern_risk = match pattern.severity {
            PatternSeverity::Low => 0.1,
            PatternSeverity::Medium => 0.3,
            PatternSeverity::High => 0.6,
            PatternSeverity::Critical => 0.9,
        };

        risk_score += pattern_risk * pattern.confidence;

        if pattern_risk > 0.2 {
            risk_factors.push(pattern.description.clone());
        }
    }

    // Add volume-based risk
    let total_volume: f64 = balance_analysis
        .sol_changes
        .values()
        .map(|c| c.change.abs())
        .sum();

    if total_volume > 1000.0 {
        risk_score += 0.2;
        risk_factors.push("High volume transaction".to_string());
    }

    // Normalize risk score
    risk_score = risk_score.min(1.0);

    // Determine risk level
    let risk_level = if risk_score >= 0.8 {
        RiskLevel::VeryHigh
    } else if risk_score >= 0.6 {
        RiskLevel::High
    } else if risk_score >= 0.4 {
        RiskLevel::Medium
    } else if risk_score >= 0.2 {
        RiskLevel::Low
    } else {
        RiskLevel::VeryLow
    };

    // Generate recommendations
    if risk_score > 0.5 {
        recommendations.push("Monitor for additional suspicious activity".to_string());
    }
    if balance_analysis.total_tips > 0.01 {
        recommendations.push("Review MEV tip patterns".to_string());
    }

    Ok(RiskAssessment {
        risk_score,
        risk_level,
        risk_factors,
        recommendations,
    })
}

// =============================================================================
// TRADING BEHAVIOR ANALYSIS
// =============================================================================

/// Analyze trading behavior patterns
async fn analyze_trading_behavior(
    transaction: &Transaction,
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
    classification: &TransactionClass,
) -> Result<TradingBehavior, String> {
    // Determine trader type
    let trader_type = determine_trader_type(balance_analysis, dex_analysis).await?;

    // Categorize transaction size
    let size_category = categorize_transaction_size(balance_analysis);

    // Assess sophistication level
    let sophistication = assess_sophistication(dex_analysis, classification).await?;

    // Analyze frequency indicators (limited without historical data)
    let frequency_indicators = FrequencyIndicators {
        estimated_tx_per_hour: None, // Would require historical analysis
        timing_patterns: vec![TimingPattern::Random], // Default assumption
    };

    Ok(TradingBehavior {
        trader_type,
        size_category,
        sophistication,
        frequency_indicators,
    })
}

/// Determine trader type from patterns
async fn determine_trader_type(
    balance_analysis: &BalanceAnalysis,
    dex_analysis: &DexAnalysis,
) -> Result<TraderType, String> {
    let total_sol_volume: f64 = balance_analysis
        .sol_changes
        .values()
        .map(|c| c.change.abs())
        .sum();

    // High tips + multiple DEXes = likely MEV bot
    if balance_analysis.total_tips > 0.001 && dex_analysis.program_ids.len() > 1 {
        return Ok(TraderType::MevBot);
    }

    // Large volume = whale
    if total_sol_volume > 100.0 {
        return Ok(TraderType::Whale);
    }

    // Multiple DEXes = arbitrageur
    if dex_analysis.program_ids.len() > 2 {
        return Ok(TraderType::Arbitrageur);
    }

    // Default to retail
    Ok(TraderType::Retail)
}

/// Categorize transaction size
fn categorize_transaction_size(balance_analysis: &BalanceAnalysis) -> SizeCategory {
    let total_sol_volume: f64 = balance_analysis
        .sol_changes
        .values()
        .map(|c| c.change.abs())
        .sum();

    if total_sol_volume > 100.0 {
        SizeCategory::Whale
    } else if total_sol_volume > 10.0 {
        SizeCategory::Large
    } else if total_sol_volume > 1.0 {
        SizeCategory::Medium
    } else if total_sol_volume > 0.1 {
        SizeCategory::Small
    } else {
        SizeCategory::Micro
    }
}

/// Assess sophistication level
async fn assess_sophistication(
    dex_analysis: &DexAnalysis,
    classification: &TransactionClass,
) -> Result<SophisticationLevel, String> {
    // Multiple DEXes + complex classification = expert
    if dex_analysis.program_ids.len() > 2 {
        return Ok(SophisticationLevel::Expert);
    }

    // Multiple steps = advanced
    if dex_analysis.program_ids.len() > 1 {
        return Ok(SophisticationLevel::Advanced);
    }

    // Complex transaction types = intermediate
    if matches!(
        classification.transaction_type,
        super::classify::ClassifiedType::AddLiquidity
            | super::classify::ClassifiedType::RemoveLiquidity
    ) {
        return Ok(SophisticationLevel::Intermediate);
    }

    Ok(SophisticationLevel::Basic)
}

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

/// Calculate pattern analysis confidence
fn calculate_pattern_confidence(
    patterns: &[DetectedPattern],
    trading_behavior: &TradingBehavior,
) -> f64 {
    if patterns.is_empty() {
        return 0.5; // Medium confidence for no patterns
    }

    // Average confidence of detected patterns
    let pattern_confidence: f64 =
        patterns.iter().map(|p| p.confidence).sum::<f64>() / (patterns.len() as f64);

    // Boost confidence for clear trader type identification
    let behavior_boost = match trading_behavior.trader_type {
        TraderType::Unknown => 0.0,
        _ => 0.1,
    };

    (pattern_confidence + behavior_boost).min(1.0)
}
