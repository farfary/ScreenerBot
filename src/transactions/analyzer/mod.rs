// Transaction analyzer module - Main coordination and interface
//
// This module coordinates all transaction analysis components following industry standards:
// - Balance change extraction (DexScreener methodology)
// - DEX/router detection (program ID mapping)
// - Transaction classification (graph-based flow analysis)
// - ATA operations tracking
// - P&L calculation with fee adjustments
// - Pattern detection and risk assessment
//
// Architecture: Each analyzer is focused and <400 LOC, with clear interfaces

pub mod balance;
pub mod dex;
pub mod classify;
pub mod ata;
pub mod pnl;
pub mod patterns;

use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::logger::{ log, LogTag };
use crate::transactions::types::*;

use self::{
    balance::BalanceAnalysis,
    dex::DexAnalysis,
    classify::{ TransactionClass, classify_transaction },
    ata::AtaAnalysis,
    pnl::PnlAnalysis,
    patterns::PatternAnalysis,
};

// =============================================================================
// CORE ANALYZER RESULT TYPES
// =============================================================================

/// Complete transaction analysis result
#[derive(Debug, Clone)]
pub struct CompleteAnalysis {
    /// Balance change analysis
    pub balance: BalanceAnalysis,
    /// DEX/router detection
    pub dex: DexAnalysis,
    /// Transaction classification
    pub classification: TransactionClass,
    /// ATA operations analysis
    pub ata: AtaAnalysis,
    /// Profit/loss calculation
    pub pnl: PnlAnalysis,
    /// Pattern detection and risk assessment
    pub patterns: PatternAnalysis,
    /// Overall analysis confidence
    pub confidence: AnalysisConfidence,
    /// Analysis timestamp
    pub analyzed_at: i64,
}

/// Analysis confidence levels
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisConfidence {
    High, // ≥0.8: Strong program ID match + balance validation
    Medium, // ≥0.6: Program ID or balance match with supporting evidence
    Low, // ≥0.4: Partial matches or fallback detection
    Unknown, // <0.4: Insufficient data for reliable classification
}

// =============================================================================
// MAIN ANALYZER INTERFACE
// =============================================================================

/// Core transaction analyzer that coordinates all sub-analyzers
pub struct TransactionAnalyzer {
    debug_enabled: bool,
}

impl TransactionAnalyzer {
    /// Create new transaction analyzer
    pub fn new(debug_enabled: bool) -> Self {
        Self { debug_enabled }
    }

    /// Perform complete transaction analysis
    pub async fn analyze_transaction(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<CompleteAnalysis, String> {
        log(
            LogTag::TransactionAnalyzer,
            &format!("Starting complete analysis for tx: {}", transaction.signature)
        );

        // Step 1: Extract balance changes
        let balance_analysis = balance::extract_balance_changes(transaction, tx_data).await?;

        // Step 2: Detect DEX interactions
        let dex_analysis = dex::detect_dex_interactions(
            transaction,
            tx_data,
            &balance_analysis
        ).await?;

        // Step 3: Classify transaction
        let classification = classify_transaction(
            transaction,
            &balance_analysis,
            &dex_analysis
        ).await?;

        // Step 4: Analyze ATA operations
        let ata_analysis = ata::analyze_ata_operations(
            transaction,
            tx_data,
            &balance_analysis
        ).await?;

        // Step 5: Calculate P&L
        let pnl_analysis = pnl::calculate_pnl(
            transaction,
            tx_data,
            &balance_analysis,
            &dex_analysis,
            &classification
        ).await?;

        // Step 6: Detect patterns and assess risk
        let pattern_analysis = patterns::detect_patterns(
            transaction,
            tx_data,
            &balance_analysis,
            &dex_analysis,
            &classification
        ).await?;

        // Calculate overall confidence
        let confidence = self.calculate_overall_confidence(
            &balance_analysis,
            &dex_analysis,
            &classification,
            &ata_analysis,
            &pnl_analysis,
            &pattern_analysis
        );

        let analyzed_at = chrono::Utc::now().timestamp();

        log(
            LogTag::TransactionAnalyzer,
            &format!(
                "Analysis complete for {}: confidence={:?}, patterns={}, classification={:?}",
                transaction.signature,
                confidence,
                pattern_analysis.detected_patterns.len(),
                classification.transaction_type
            )
        );

        Ok(CompleteAnalysis {
            balance: balance_analysis,
            dex: dex_analysis,
            classification,
            ata: ata_analysis,
            pnl: pnl_analysis,
            patterns: pattern_analysis,
            confidence,
            analyzed_at,
        })
    }

    /// Calculate overall analysis confidence
    fn calculate_overall_confidence(
        &self,
        balance_analysis: &BalanceAnalysis,
        dex_analysis: &DexAnalysis,
        classification: &TransactionClass,
        ata_analysis: &AtaAnalysis,
        pnl_analysis: &PnlAnalysis,
        pattern_analysis: &PatternAnalysis
    ) -> AnalysisConfidence {
        // Weight each component confidence
        let weights = [
            (balance_analysis.confidence, 0.25),
            (dex_analysis.confidence, 0.2),
            (classification.confidence, 0.2),
            (ata_analysis.confidence, 0.1),
            (pnl_analysis.confidence, 0.15),
            (pattern_analysis.confidence, 0.1),
        ];

        let weighted_sum: f64 = weights
            .iter()
            .map(|(conf, weight)| conf * weight)
            .sum();

        if weighted_sum >= 0.85 {
            AnalysisConfidence::High
        } else if weighted_sum >= 0.65 {
            AnalysisConfidence::Medium
        } else {
            AnalysisConfidence::Low
        }
    }

    /// Quick classification for performance-critical paths
    pub async fn quick_classify(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails
    ) -> Result<classify::TransactionClass, String> {
        // Lightweight analysis for basic classification only
        let balance_changes = balance::extract_balance_changes(transaction, tx_data).await?;
        let dex_detection = dex::detect_dex_interactions(
            transaction,
            tx_data,
            &balance_changes
        ).await?;

        classify::classify_transaction(transaction, &balance_changes, &dex_detection).await
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if analysis confidence meets minimum threshold for reliable results
pub fn is_analysis_reliable(confidence: &AnalysisConfidence) -> bool {
    matches!(confidence, AnalysisConfidence::High | AnalysisConfidence::Medium)
}

/// Convert confidence to numeric score for comparison
pub fn confidence_to_score(confidence: &AnalysisConfidence) -> f64 {
    match confidence {
        AnalysisConfidence::High => 0.9,
        AnalysisConfidence::Medium => 0.7,
        AnalysisConfidence::Low => 0.5,
        AnalysisConfidence::Unknown => 0.2,
    }
}

/// Combine multiple confidence scores into overall confidence
pub fn combine_confidence_scores(scores: &[f64]) -> AnalysisConfidence {
    if scores.is_empty() {
        return AnalysisConfidence::Unknown;
    }

    let average = scores.iter().sum::<f64>() / (scores.len() as f64);

    if average >= 0.8 {
        AnalysisConfidence::High
    } else if average >= 0.6 {
        AnalysisConfidence::Medium
    } else if average >= 0.4 {
        AnalysisConfidence::Low
    } else {
        AnalysisConfidence::Unknown
    }
}
