// Transaction analyzer submodule - Main coordination and interface
//
// This module provides comprehensive transaction analysis following industry standards:
// - Balance change extraction (DexScreener methodology)
// - DEX/router detection (program ID mapping)
// - Transaction classification (graph-based flow analysis)
// - ATA operations tracking and rent calculation
// - P&L calculation with fee adjustments
// - Pattern detection and risk assessment
//
// Public API:
// - TransactionAnalyzer::analyze_transaction() -> CompleteAnalysis (full 6-step pipeline)
// - TransactionAnalyzer::quick_classify() -> TransactionClass (lightweight for high-frequency)
//
// All analysis returns Result<T, String> with structured logging via LogTag.
// Confidence scoring ranges from Unknown (<0.4) to High (≥0.8).

pub mod ata;
pub mod balance;
pub mod classify;
pub mod dex;
pub mod patterns;
pub mod pnl;

// Re-export public types for external use
pub use ata::AtaAnalysis;
pub use balance::BalanceAnalysis;
pub use classify::TransactionClass;
pub use dex::DexAnalysis;
pub use patterns::PatternAnalysis;
pub use pnl::PnLAnalysis;

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::logger::{log, LogTag};
use crate::transactions::types::*;

use self::classify::classify_transaction;

// =============================================================================
// PUBLIC API TYPES
// =============================================================================

/// Complete transaction analysis result containing all analysis components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteAnalysis {
    /// Balance change analysis (SOL/SPL transfers, MEV tips, rent filtering)
    pub balance: BalanceAnalysis,
    /// DEX/router detection (program ID mapping, confidence scoring)
    pub dex: DexAnalysis,
    /// Transaction classification (Buy/Sell/Transfer/AddLiquidity/etc)
    pub classification: TransactionClass,
    /// ATA operations analysis (creation/close, rent tracking)
    pub ata: AtaAnalysis,
    /// Profit/loss calculation (fee-adjusted, net cost analysis)
    pub pnl: PnLAnalysis,
    /// Pattern detection and risk assessment (MEV, wash trading, etc)
    pub patterns: PatternAnalysis,
    /// Overall analysis confidence (weighted across all components)
    pub confidence: AnalysisConfidence,
    /// Analysis timestamp (UTC unix timestamp)
    pub analyzed_at: i64,
}

/// Analysis confidence levels with clear thresholds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisConfidence {
    /// ≥85% weighted confidence: Strong program ID match + balance validation + clear patterns
    High,
    /// ≥65% weighted confidence: Program ID or balance match with supporting evidence
    Medium,
    /// ≥40% weighted confidence: Partial matches or fallback detection methods
    Low,
    /// <40% weighted confidence: Insufficient data for reliable classification
    Unknown,
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
        tx_data: &crate::rpc::TransactionDetails,
    ) -> Result<CompleteAnalysis, String> {
        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ANALYZE_START",
                &format!(
                    "Starting complete analysis for tx: {}",
                    transaction.signature
                ),
            );
        }

        // Step 1: Extract balance changes (full analyzer for tips/rent detection)
        let balance_analysis = balance::analyze_balance_changes(transaction, tx_data).await?;

        // Step 2: Detect DEX interactions
        let dex_analysis =
            dex::detect_dex_interactions(transaction, tx_data, &balance_analysis).await?;

        // Step 3: Classify transaction
        let classification =
            classify_transaction(transaction, tx_data, &balance_analysis, &dex_analysis).await?;

        // Step 4: Analyze ATA operations
        let ata_analysis = ata::analyze_ata_operations(transaction, tx_data).await?;

        // Step 5: Calculate P&L
        let pnl_analysis = pnl::calculate_pnl(
            transaction,
            tx_data,
            &balance_analysis,
            &classification,
            &ata_analysis,
        )
        .await?;

        // Step 6: Detect patterns and assess risk
        let pattern_analysis = patterns::detect_patterns(
            transaction,
            tx_data,
            &balance_analysis,
            &dex_analysis,
            &classification,
        )
        .await?;

        // Calculate overall confidence
        let confidence = self.calculate_overall_confidence(
            &balance_analysis,
            &dex_analysis,
            &classification,
            &ata_analysis,
            &pnl_analysis,
            &pattern_analysis,
        );

        let analyzed_at = chrono::Utc::now().timestamp();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "ANALYZE_COMPLETE",
                &format!(
                    "Analysis complete for {}: confidence={:?}, patterns={}, classification={:?}",
                    transaction.signature,
                    confidence,
                    pattern_analysis.detected_patterns.len(),
                    classification.transaction_type
                ),
            );
        }

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
        pnl_analysis: &PnLAnalysis,
        pattern_analysis: &PatternAnalysis,
    ) -> AnalysisConfidence {
        // Weight each component confidence
        let weights = [
            (balance_analysis.confidence, 0.25),
            (dex_analysis.confidence, 0.2),
            (confidence_to_score(&classification.confidence), 0.2),
            (ata_analysis.confidence, 0.1),
            (pnl_analysis.confidence, 0.15),
            (pattern_analysis.confidence, 0.1),
        ];

        let weighted_sum: f64 = weights.iter().map(|(conf, weight)| conf * weight).sum();

        score_to_confidence(weighted_sum)
    }

    /// Quick classification for performance-critical paths
    pub async fn quick_classify(
        &self,
        transaction: &Transaction,
        tx_data: &crate::rpc::TransactionDetails,
    ) -> Result<classify::TransactionClass, String> {
        // Lightweight analysis for basic classification only
        let balance_changes = balance::extract_balance_changes(transaction, tx_data).await?;
        let dex_detection =
            dex::detect_dex_interactions(transaction, tx_data, &balance_changes).await?;

        classify::classify_transaction(transaction, tx_data, &balance_changes, &dex_detection).await
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if analysis confidence meets minimum threshold for reliable results
pub fn is_analysis_reliable(confidence: &AnalysisConfidence) -> bool {
    matches!(
        confidence,
        AnalysisConfidence::High | AnalysisConfidence::Medium
    )
}

/// Convert confidence to numeric score for comparison and weighting
pub fn confidence_to_score(confidence: &AnalysisConfidence) -> f64 {
    match confidence {
        AnalysisConfidence::High => 0.9,
        AnalysisConfidence::Medium => 0.7,
        AnalysisConfidence::Low => 0.5,
        AnalysisConfidence::Unknown => 0.2,
    }
}

/// Convert numeric confidence score to AnalysisConfidence enum
pub fn score_to_confidence(score: f64) -> AnalysisConfidence {
    if score >= 0.85 {
        AnalysisConfidence::High
    } else if score >= 0.65 {
        AnalysisConfidence::Medium
    } else if score >= 0.4 {
        AnalysisConfidence::Low
    } else {
        AnalysisConfidence::Unknown
    }
}
