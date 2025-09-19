//! learner/integration.rs
//!
//! Integration interface for entry and profit systems.
//!
//! This module provides clean, simple APIs that entry.rs and profit.rs can use
//! to get learning-based adjustments without understanding the complexity of the
//! underlying ML system.
//!
//! Key principles:
//! * Non-blocking: All calls complete in <5ms or return fallback values
//! * Fail-safe: Never cause entry/profit systems to fail
//! * Conservative: Prefer false negatives over false positives
//! * Transparent: Log decisions for debugging

use crate::learner::types::*;
use crate::learner::analyzer::PatternAnalyzer;
use crate::learner::model::ModelManager;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_learning_enabled;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::time::timeout;

/// Learning system integration interface
pub struct LearningIntegration {
    analyzer: Arc<PatternAnalyzer>,
    model_manager: Arc<ModelManager>,
}

impl LearningIntegration {
    /// Create new integration interface
    pub fn new(analyzer: Arc<PatternAnalyzer>, model_manager: Arc<ModelManager>) -> Self {
        Self {
            analyzer,
            model_manager,
        }
    }

    /// Get confidence adjustment for entry decision
    ///
    /// Returns a multiplier (0.5 to 2.0) to adjust entry confidence.
    /// * > 1.0: Increase confidence (good pattern match)
    /// * < 1.0: Decrease confidence (risky pattern match)
    /// * 1.0: No adjustment (neutral/unknown)
    ///
    /// Completes in <5ms or returns 1.0 as fallback.
    pub async fn get_entry_confidence_adjustment(
        &self,
        mint: &str,
        current_price: f64,
        drop_percent: f64,
        ath_proximity: f64
    ) -> f64 {
        let start_time = Instant::now();

        // Timeout protection
        let result = timeout(Duration::from_millis(5), async {
            self.compute_entry_adjustment(mint, current_price, drop_percent, ath_proximity).await
        }).await;

        match result {
            Ok(adjustment) => {
                if is_debug_learning_enabled() {
                    let elapsed = start_time.elapsed();
                    log(
                        LogTag::Learning,
                        "DEBUG",
                        &format!(
                            "Entry adjustment for {} computed in {:?}: {:.3}",
                            mint,
                            elapsed,
                            adjustment
                        )
                    );
                }
                adjustment
            }
            Err(_) => {
                log(
                    LogTag::Learning,
                    "WARN",
                    &format!("Entry adjustment timeout for {}, using fallback", mint)
                );
                1.0 // Fallback: no adjustment
            }
        }
    }

    /// Get score adjustment for exit decision
    ///
    /// Returns a multiplier (0.5 to 2.0) to adjust exit urgency score.
    /// * > 1.0: Increase exit urgency (pattern suggests selling)
    /// * < 1.0: Decrease exit urgency (pattern suggests holding)
    /// * 1.0: No adjustment (neutral/unknown)
    ///
    /// Completes in <5ms or returns 1.0 as fallback.
    pub async fn get_exit_score_adjustment(
        &self,
        mint: &str,
        current_price: f64,
        entry_price: f64,
        position_duration_mins: u32
    ) -> f64 {
        let start_time = Instant::now();

        // Timeout protection
        let result = timeout(Duration::from_millis(5), async {
            self.compute_exit_adjustment(
                mint,
                current_price,
                entry_price,
                position_duration_mins
            ).await
        }).await;

        match result {
            Ok(adjustment) => {
                if is_debug_learning_enabled() {
                    let elapsed = start_time.elapsed();
                    log(
                        LogTag::Learning,
                        "DEBUG",
                        &format!(
                            "Exit adjustment for {} computed in {:?}: {:.3}",
                            mint,
                            elapsed,
                            adjustment
                        )
                    );
                }
                adjustment
            }
            Err(_) => {
                log(
                    LogTag::Learning,
                    "WARN",
                    &format!("Exit adjustment timeout for {}, using fallback", mint)
                );
                1.0 // Fallback: no adjustment
            }
        }
    }

    /// Internal: Compute entry confidence adjustment
    async fn compute_entry_adjustment(
        &self,
        mint: &str,
        current_price: f64,
        drop_percent: f64,
        ath_proximity: f64
    ) -> f64 {
        // Extract features for this potential entry
        let features = match
            self.analyzer.extract_features_for_entry(
                mint,
                current_price,
                drop_percent,
                ath_proximity
            ).await
        {
            Ok(f) => f,
            Err(e) => {
                if is_debug_learning_enabled() {
                    log(
                        LogTag::Learning,
                        "DEBUG",
                        &format!("Feature extraction failed for {}: {}", mint, e)
                    );
                }
                return 1.0; // No adjustment if can't extract features
            }
        };

        // Get ML prediction
        let prediction = match self.model_manager.predict_entry_success(&features).await {
            Ok(p) => p,
            Err(e) => {
                if is_debug_learning_enabled() {
                    log(
                        LogTag::Learning,
                        "DEBUG",
                        &format!("Prediction failed for {}: {}", mint, e)
                    );
                }
                return 1.0; // No adjustment if prediction fails
            }
        };

        // Convert prediction to confidence adjustment
        let adjustment: f64 = if prediction.confidence < 0.3 {
            1.0 // Low confidence: no adjustment
        } else if prediction.success_probability > 0.7 {
            // High success probability: boost confidence
            1.0 + (prediction.success_probability - 0.5) * 0.6 // Max 1.3x
        } else if prediction.risk_probability > 0.6 {
            // High risk probability: reduce confidence
            1.0 - (prediction.risk_probability - 0.5) * 0.8 // Min 0.6x
        } else {
            1.0 // Neutral
        };

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Entry adjustment for {}: {:.3} (success: {:.2}, risk: {:.2}, conf: {:.2})",
                    mint,
                    adjustment,
                    prediction.success_probability,
                    prediction.risk_probability,
                    prediction.confidence
                )
            );
        }

        adjustment.clamp(0.5, 2.0)
    }

    /// Internal: Compute exit score adjustment
    async fn compute_exit_adjustment(
        &self,
        mint: &str,
        current_price: f64,
        entry_price: f64,
        position_duration_mins: u32
    ) -> f64 {
        // Extract features for this exit decision
        let features = match
            self.analyzer.extract_features_for_exit(
                mint,
                current_price,
                entry_price,
                position_duration_mins
            ).await
        {
            Ok(f) => f,
            Err(e) => {
                if is_debug_learning_enabled() {
                    log(
                        LogTag::Learning,
                        "DEBUG",
                        &format!("Exit feature extraction failed for {}: {}", mint, e)
                    );
                }
                return 1.0; // No adjustment if can't extract features
            }
        };

        // Get ML prediction
        let prediction = match self.model_manager.predict_exit_outcome(&features).await {
            Ok(p) => p,
            Err(e) => {
                if is_debug_learning_enabled() {
                    log(
                        LogTag::Learning,
                        "DEBUG",
                        &format!("Exit prediction failed for {}: {}", mint, e)
                    );
                }
                return 1.0; // No adjustment if prediction fails
            }
        };

        // Convert prediction to exit urgency adjustment
        let adjustment: f64 = if prediction.urgency_score < 0.3 {
            1.0 // Low urgency: no adjustment
        } else if prediction.reversal_probability > 0.7 {
            // High loss probability: increase exit urgency
            1.0 + (prediction.reversal_probability - 0.5) * 0.8 // Max 1.4x
        } else if prediction.further_upside_probability > 0.6 {
            // High profit probability: reduce exit urgency (hold longer)
            1.0 - (prediction.further_upside_probability - 0.5) * 0.6 // Min 0.7x
        } else {
            1.0 // Neutral
        };

        if is_debug_learning_enabled() {
            log(
                LogTag::Learning,
                "INFO",
                &format!(
                    "Exit adjustment for {}: {:.3} (further_upside: {:.2}, reversal: {:.2}, conf: {:.2})",
                    mint,
                    adjustment,
                    prediction.further_upside_probability,
                    prediction.reversal_probability,
                    prediction.urgency_score
                )
            );
        }

        adjustment.clamp(0.5, 2.0)
    }
}

// === Convenience Functions for Entry/Profit Integration ===

/// Quick entry confidence adjustment (for entry.rs)
///
/// Non-blocking convenience function that can be called directly from entry logic.
/// Returns 1.0 if learning system not available.
pub async fn get_entry_confidence_boost(
    learning_integration: Option<&LearningIntegration>,
    mint: &str,
    current_price: f64,
    drop_percent: f64,
    ath_proximity: f64
) -> f64 {
    match learning_integration {
        Some(integration) => {
            integration.get_entry_confidence_adjustment(
                mint,
                current_price,
                drop_percent,
                ath_proximity
            ).await
        }
        None => 1.0, // No learning system available
    }
}

/// Quick exit score adjustment (for profit.rs)
///
/// Non-blocking convenience function that can be called directly from profit logic.
/// Returns 1.0 if learning system not available.
pub async fn get_exit_urgency_multiplier(
    learning_integration: Option<&LearningIntegration>,
    mint: &str,
    current_price: f64,
    entry_price: f64,
    position_duration_mins: u32
) -> f64 {
    match learning_integration {
        Some(integration) => {
            integration.get_exit_score_adjustment(
                mint,
                current_price,
                entry_price,
                position_duration_mins
            ).await
        }
        None => 1.0, // No learning system available
    }
}

/// Synchronous fallback for entry decisions
///
/// For cases where async calls are not feasible, this provides basic heuristics
/// based on simple rules derived from historical patterns.
pub fn get_entry_confidence_sync_fallback(drop_percent: f64, ath_proximity: f64) -> f64 {
    let drop_score: f64 = if drop_percent > 30.0 {
        1.2
    } else if drop_percent > 20.0 {
        1.1
    } else {
        1.0
    };
    let ath_score: f64 = if ath_proximity > 0.8 { 1.1 } else { 1.0 };

    (drop_score * ath_score).clamp(0.8, 1.3)
}

/// Synchronous fallback for exit decisions
///
/// For cases where async calls are not feasible, this provides basic heuristics
/// based on simple rules derived from historical patterns.
pub fn get_exit_urgency_sync_fallback(
    current_profit_percent: f64,
    position_duration_mins: u32
) -> f64 {
    // Simple heuristic: quick profits good, long losses bad
    if current_profit_percent > 15.0 && position_duration_mins < 30 {
        0.8 // Reduce exit urgency for quick profits
    } else if current_profit_percent < -10.0 && position_duration_mins > 120 {
        1.3 // Increase exit urgency for prolonged losses
    } else {
        1.0 // Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_fallbacks() {
        // Test entry fallback
        assert!(get_entry_confidence_sync_fallback(35.0, 0.9) > 1.0);
        assert!(get_entry_confidence_sync_fallback(10.0, 0.5) == 1.0);

        // Test exit fallback
        assert!(get_exit_urgency_sync_fallback(20.0, 25) < 1.0);
        assert!(get_exit_urgency_sync_fallback(-15.0, 150) > 1.0);
        assert!(get_exit_urgency_sync_fallback(5.0, 60) == 1.0);
    }
}
