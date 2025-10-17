//! learner/types.rs
//!
//! Core data structures for the learning system.
//!
//! This module defines all the types used throughout the learning system:
//! * TradeRecord: Completed trade data for analysis
//! * FeatureVector: Numeric features extracted from trades
//! * ModelWeights: Serializable model parameters
//! * Patterns: Recognized trading patterns
//! * Predictions: Model outputs for entry/exit decisions

use crate::positions::{get_recent_closed_positions_for_mint, get_token_snapshot, Position};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// TRADE RECORDING TYPES
// =============================================================================

/// Complete trade record for learning analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: i64,        // Database ID
    pub mint: String,   // Token mint address
    pub symbol: String, // Token symbol
    pub name: String,   // Token name

    // Entry/Exit Information
    pub entry_time: DateTime<Utc>, // When position was opened
    pub exit_time: DateTime<Utc>,  // When position was closed
    pub entry_price: f64,          // Entry price
    pub exit_price: f64,           // Exit price
    pub hold_duration_sec: i64,    // How long position was held

    // Trade Outcomes
    pub pnl_pct: f64,                  // Final P&L percentage
    pub max_up_pct: f64,               // Maximum profit during trade
    pub max_down_pct: f64,             // Maximum drawdown during trade
    pub peak_reached_sec: Option<i64>, // Time to reach peak (from entry)
    pub dd_reached_sec: Option<i64>,   // Time to reach max drawdown

    // Position Details
    pub entry_size_sol: f64,                // SOL amount invested
    pub token_amount: Option<u64>,          // Token amount received
    pub liquidity_at_entry: Option<f64>,    // Pool liquidity at entry
    pub sol_reserves_at_entry: Option<f64>, // SOL reserves at entry

    // Market Context
    pub tx_activity_5m: Option<i64>, // Transaction count in 5min before entry
    pub tx_activity_1h: Option<i64>, // Transaction count in 1h before entry
    pub risk_score: Option<u8>,      // Risk score at entry (raw rugcheck score)
    pub holder_count: Option<i64>,   // Holder count at entry

    // Entry Conditions
    pub drop_10s_pct: Option<f64>,  // 10s drop that triggered entry
    pub drop_30s_pct: Option<f64>,  // 30s drop at entry
    pub drop_60s_pct: Option<f64>,  // 60s drop at entry
    pub drop_120s_pct: Option<f64>, // 120s drop at entry
    pub drop_320s_pct: Option<f64>, // 320s drop at entry

    // ATH Context at Entry
    pub ath_dist_15m_pct: Option<f64>, // Distance from 15m high
    pub ath_dist_1h_pct: Option<f64>,  // Distance from 1h high
    pub ath_dist_6h_pct: Option<f64>,  // Distance from 6h high

    // Timing Information
    pub hour_of_day: i32, // Hour when trade was entered (0-23)
    pub day_of_week: i32, // Day of week (0=Sunday)

    // Flags
    pub was_re_entry: bool, // Was this a re-entry after previous exit
    pub phantom_exit: bool, // Was exit due to phantom detection
    pub forced_exit: bool,  // Was exit forced by time limit

    // Processing Status
    pub features_extracted: bool,  // Have features been extracted
    pub created_at: DateTime<Utc>, // When record was created
}

impl TradeRecord {
    /// Create trade record from completed position using token snapshots
    pub async fn from_position(
        position: &Position,
        max_up_pct: f64,
        max_down_pct: f64,
    ) -> Result<Self, String> {
        let exit_time = position.exit_time.ok_or("Position must have exit time")?;
        let exit_price = position.exit_price.ok_or("Position must have exit price")?;
        let position_id = position.id.ok_or("Position must have database ID")?;

        let hold_duration_sec = (exit_time - position.entry_time).num_seconds();
        let pnl_pct = ((exit_price - position.entry_price) / position.entry_price) * 100.0;

        // Get token snapshots for entry and exit data
        let opening_snapshot = get_token_snapshot(position_id, "opening")
            .await
            .map_err(|e| format!("Failed to get opening snapshot: {}", e))?;
        let closing_snapshot = get_token_snapshot(position_id, "closing")
            .await
            .map_err(|e| format!("Failed to get closing snapshot: {}", e))?;

        // Check for re-entry by looking at recent closed positions for this mint
        let was_re_entry = match get_recent_closed_positions_for_mint(&position.mint, 5).await {
            Ok(recent_positions) => !recent_positions.is_empty(),
            Err(_) => false, // Default to false if query fails
        };

        // Extract data from opening snapshot (entry time data)
        let (
            liquidity_at_entry,
            sol_reserves_at_entry,
            tx_activity_5m,
            tx_activity_1h,
            risk_score,
            holder_count,
        ) = if let Some(ref snapshot) = opening_snapshot {
            (
                snapshot.liquidity_base, // SOL liquidity base
                snapshot.liquidity_base, // Same as above for reserves
                snapshot
                    .txns_m5_buys
                    .and_then(|buys| snapshot.txns_m5_sells.map(|sells| buys + sells)),
                snapshot
                    .txns_h1_buys
                    .and_then(|buys| snapshot.txns_h1_sells.map(|sells| buys + sells)),
                None, // Security score not in snapshots - would need separate Rugcheck call
                None, // Holder count not in snapshots - would need separate Rugcheck call
            )
        } else {
            (None, None, None, None, None, None)
        };

        // Calculate drop percentages using real pools price history if available
        let (drop_10s_pct, drop_30s_pct, drop_60s_pct, drop_120s_pct, drop_320s_pct) = {
            // Try to get pools price history for this token around entry time
            let price_history = crate::pools::get_price_history(&position.mint);

            if !price_history.is_empty() {
                // Calculate drops using same logic as learner analyzer
                use chrono::Utc;

                let entry_time = position.entry_time;
                let entry_price = position.entry_price;
                let windows_sec = [10, 30, 60, 120, 320];
                let mut drops = [0.0; 5];

                for (i, &window_sec) in windows_sec.iter().enumerate() {
                    let mut window_high = entry_price;

                    // Find highest price in the window before entry
                    for price_result in price_history.iter().rev() {
                        let price_time = price_result.get_utc_timestamp();
                        let time_diff = (entry_time - price_time).num_seconds();

                        // Only consider prices within the window and before entry
                        if time_diff >= 0 && time_diff <= (window_sec as i64) {
                            if price_result.price_sol > window_high {
                                window_high = price_result.price_sol;
                            }
                        }
                    }

                    // Calculate drop percentage
                    if window_high > 0.0 && window_high >= entry_price {
                        drops[i] = ((window_high - entry_price) / window_high) * 100.0;
                    }
                }

                (
                    Some(drops[0]),
                    Some(drops[1]),
                    Some(drops[2]),
                    Some(drops[3]),
                    Some(drops[4]),
                )
            } else {
                // Fallback to approximation from snapshot if pools data unavailable
                if let Some(ref snapshot) = opening_snapshot {
                    let m5_drop = snapshot
                        .price_change_m5
                        .map(|pc| (if pc < 0.0 { -pc } else { 0.0 }));
                    let h1_drop = snapshot
                        .price_change_h1
                        .map(|pc| (if pc < 0.0 { -pc } else { 0.0 }));

                    // Approximate different timeframes from available data
                    (
                        m5_drop.map(|d| d * 0.1), // 10s approximation from 5m
                        m5_drop.map(|d| d * 0.2), // 30s approximation from 5m
                        m5_drop.map(|d| d * 0.5), // 60s approximation from 5m
                        m5_drop.map(|d| d * 0.8), // 120s approximation from 5m
                        m5_drop,                  // 5m drop for 320s
                    )
                } else {
                    (None, None, None, None, None)
                }
            }
        };

        // Calculate ATH distances using real pools price history if available
        let (ath_dist_15m_pct, ath_dist_1h_pct, ath_dist_6h_pct) = {
            let price_history = crate::pools::get_price_history(&position.mint);

            if !price_history.is_empty() {
                let entry_time = position.entry_time;
                let entry_price = position.entry_price;
                let windows_sec = [15 * 60, 60 * 60, 6 * 60 * 60]; // 15m, 1h, 6h
                let mut ath_distances = [0.0; 3];

                for (i, &window_sec) in windows_sec.iter().enumerate() {
                    let mut window_high = entry_price;

                    for price_result in price_history.iter().rev() {
                        let price_time = price_result.get_utc_timestamp();
                        let time_diff = (entry_time - price_time).num_seconds();

                        if time_diff >= 0 && time_diff <= (window_sec as i64) {
                            if price_result.price_sol > window_high {
                                window_high = price_result.price_sol;
                            }
                        }
                    }

                    if window_high > 0.0 && window_high >= entry_price {
                        ath_distances[i] = ((window_high - entry_price) / window_high) * 100.0;
                    }
                }

                (
                    Some(ath_distances[0]),
                    Some(ath_distances[1]),
                    Some(ath_distances[2]),
                )
            } else {
                // Fallback to approximation from snapshot
                if let Some(ref snapshot) = opening_snapshot {
                    (
                        snapshot
                            .price_change_m5
                            .map(|pc| if pc < 0.0 { -pc } else { 0.0 }),
                        snapshot
                            .price_change_h1
                            .map(|pc| if pc < 0.0 { -pc } else { 0.0 }),
                        snapshot
                            .price_change_h6
                            .map(|pc| if pc < 0.0 { -pc } else { 0.0 }),
                    )
                } else {
                    (None, None, None)
                }
            }
        };

        // Calculate peak and drawdown timing - these would need position tracking data
        // For now, estimate based on hold duration and performance
        let peak_reached_sec = if max_up_pct > 5.0 {
            Some(hold_duration_sec / 3) // Assume peak reached in first third if profitable
        } else {
            None
        };

        let dd_reached_sec = if max_down_pct < -5.0 {
            Some(hold_duration_sec / 2) // Assume max drawdown reached mid-way if significant
        } else {
            None
        };

        Ok(TradeRecord {
            id: 0, // Will be set by database
            mint: position.mint.clone(),
            symbol: position.symbol.clone(),
            name: position.name.clone(),

            entry_time: position.entry_time,
            exit_time,
            entry_price: position.entry_price,
            exit_price,
            hold_duration_sec,

            pnl_pct,
            max_up_pct,
            max_down_pct,
            peak_reached_sec,
            dd_reached_sec,

            entry_size_sol: position.entry_size_sol,
            token_amount: position.token_amount,
            liquidity_at_entry,
            sol_reserves_at_entry,

            tx_activity_5m,
            tx_activity_1h,
            risk_score,
            holder_count,

            drop_10s_pct,
            drop_30s_pct,
            drop_60s_pct,
            drop_120s_pct,
            drop_320s_pct,

            ath_dist_15m_pct,
            ath_dist_1h_pct,
            ath_dist_6h_pct,

            hour_of_day: position.entry_time.hour() as i32,
            day_of_week: position.entry_time.weekday().num_days_from_sunday() as i32,

            was_re_entry,
            phantom_exit: position.phantom_remove,
            forced_exit: position
                .closed_reason
                .as_ref()
                .map(|r| (r.contains("time") || r.contains("force")))
                .unwrap_or(false),

            features_extracted: false,
            created_at: Utc::now(),
        })
    }
}

// =============================================================================
// FEATURE EXTRACTION TYPES
// =============================================================================

/// Numeric feature vector for machine learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub trade_id: i64, // Reference to trade record

    // Drop Pattern Features (normalized 0-1)
    pub drop_10s_norm: f64,     // 10s drop normalized by volatility
    pub drop_30s_norm: f64,     // 30s drop normalized
    pub drop_60s_norm: f64,     // 60s drop normalized
    pub drop_120s_norm: f64,    // 120s drop normalized
    pub drop_320s_norm: f64,    // 320s drop normalized
    pub drop_velocity_30s: f64, // Drop velocity (pct/min)
    pub drop_acceleration: f64, // Change in drop velocity

    // Market Context Features
    pub liquidity_tier: f64,    // Liquidity tier (0-1)
    pub tx_activity_score: f64, // Transaction activity score (0-1)
    pub risk_score_norm: f64,   // Risk score normalized (0-1, higher = riskier)
    pub holder_count_log: f64,  // Log of holder count
    pub market_cap_tier: f64,   // Market cap tier (0-1)

    // ATH Proximity Features
    pub ath_prox_15m: f64,   // Proximity to 15m high (0-1)
    pub ath_prox_1h: f64,    // Proximity to 1h high (0-1)
    pub ath_prox_6h: f64,    // Proximity to 6h high (0-1)
    pub ath_risk_score: f64, // Combined ATH risk (0-1)

    // Temporal Features
    pub hour_sin: f64, // sin(hour * 2π / 24)
    pub hour_cos: f64, // cos(hour * 2π / 24)
    pub day_sin: f64,  // sin(day * 2π / 7)
    pub day_cos: f64,  // cos(day * 2π / 7)

    // Historical Features
    pub re_entry_flag: f64,     // 1.0 if re-entry, 0.0 otherwise
    pub token_trade_count: f64, // Number of previous trades for this token
    pub recent_exit_count: f64, // Exits in last 24h for this token
    pub avg_hold_duration: f64, // Average hold duration for this token

    // Labels for Training
    pub success_label: Option<f64>, // 1.0 if profitable exit, 0.0 otherwise
    pub quick_success_label: Option<f64>, // 1.0 if >25% profit in <20min
    pub risk_label: Option<f64>,    // 1.0 if >18% drawdown in first 8min
    pub peak_time_label: Option<f64>, // Time to peak (normalized)

    pub created_at: DateTime<Utc>,
}

impl FeatureVector {
    /// Create feature vector with default values
    pub fn new(trade_id: i64) -> Self {
        Self {
            trade_id,
            drop_10s_norm: 0.0,
            drop_30s_norm: 0.0,
            drop_60s_norm: 0.0,
            drop_120s_norm: 0.0,
            drop_320s_norm: 0.0,
            drop_velocity_30s: 0.0,
            drop_acceleration: 0.0,
            liquidity_tier: 0.0,
            tx_activity_score: 0.0,
            risk_score_norm: 0.0,
            holder_count_log: 0.0,
            market_cap_tier: 0.0,
            ath_prox_15m: 0.0,
            ath_prox_1h: 0.0,
            ath_prox_6h: 0.0,
            ath_risk_score: 0.0,
            hour_sin: 0.0,
            hour_cos: 0.0,
            day_sin: 0.0,
            day_cos: 0.0,
            re_entry_flag: 0.0,
            token_trade_count: 0.0,
            recent_exit_count: 0.0,
            avg_hold_duration: 0.0,
            success_label: None,
            quick_success_label: None,
            risk_label: None,
            peak_time_label: None,
            created_at: Utc::now(),
        }
    }

    /// Get feature vector as array for ML algorithms
    pub fn to_array(&self) -> Vec<f64> {
        vec![
            self.drop_10s_norm,
            self.drop_30s_norm,
            self.drop_60s_norm,
            self.drop_120s_norm,
            self.drop_320s_norm,
            self.drop_velocity_30s,
            self.drop_acceleration,
            self.liquidity_tier,
            self.tx_activity_score,
            self.risk_score_norm,
            self.holder_count_log,
            self.market_cap_tier,
            self.ath_prox_15m,
            self.ath_prox_1h,
            self.ath_prox_6h,
            self.ath_risk_score,
            self.hour_sin,
            self.hour_cos,
            self.day_sin,
            self.day_cos,
            self.re_entry_flag,
            self.token_trade_count,
            self.recent_exit_count,
            self.avg_hold_duration,
        ]
    }

    /// Number of features in the vector
    pub const FEATURE_COUNT: usize = 24;
}

// =============================================================================
// MODEL TYPES
// =============================================================================

/// Model weights for serialization and hot-swapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWeights {
    pub version: i64,       // Model version number
    pub model_type: String, // "logistic_regression", "random_forest", etc.

    // Success Prediction Model
    pub success_weights: Vec<f64>, // Feature weights for success prediction
    pub success_intercept: f64,    // Intercept for success model
    pub success_threshold: f64,    // Classification threshold

    // Risk Prediction Model
    pub risk_weights: Vec<f64>, // Feature weights for risk prediction
    pub risk_intercept: f64,    // Intercept for risk model
    pub risk_threshold: f64,    // Classification threshold

    // Model Metadata
    pub training_samples: usize,  // Number of samples used for training
    pub validation_accuracy: f64, // Validation accuracy
    pub feature_importance: Vec<f64>, // Feature importance scores

    pub created_at: DateTime<Utc>,
    pub trained_on_trades: i64, // Number of trades in training set
}

impl ModelWeights {
    /// Create new model weights with default values
    pub fn new(model_type: String) -> Self {
        Self {
            version: 1,
            model_type,
            success_weights: vec![0.0; FeatureVector::FEATURE_COUNT],
            success_intercept: 0.0,
            success_threshold: 0.5,
            risk_weights: vec![0.0; FeatureVector::FEATURE_COUNT],
            risk_intercept: 0.0,
            risk_threshold: 0.5,
            training_samples: 0,
            validation_accuracy: 0.0,
            feature_importance: vec![0.0; FeatureVector::FEATURE_COUNT],
            created_at: Utc::now(),
            trained_on_trades: 0,
        }
    }
}

// =============================================================================
// PATTERN RECOGNITION TYPES
// =============================================================================

/// Recognized trading pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPattern {
    pub pattern_id: String,        // Unique pattern identifier
    pub pattern_type: PatternType, // Type of pattern
    pub confidence: f64,           // Pattern confidence (0-1)

    // Pattern Characteristics
    pub drop_sequence: Vec<f64>, // Sequence of drops that define pattern
    pub duration_range: (i64, i64), // Duration range for pattern (min, max seconds)
    pub success_rate: f64,       // Historical success rate
    pub avg_profit: f64,         // Average profit percentage
    pub avg_duration: i64,       // Average hold duration
    pub sample_count: usize,     // Number of trades matching pattern

    // Context Requirements
    pub liquidity_min: Option<f64>,    // Minimum liquidity requirement
    pub tx_activity_min: Option<i64>,  // Minimum transaction activity
    pub ath_distance_max: Option<f64>, // Maximum distance from ATH

    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    SharpDrop,    // Quick sharp drop followed by recovery
    GradualDrop,  // Gradual drop over longer period
    DoubleBottom, // Drop, partial recovery, second drop
    Breakout,     // Drop below support then quick recovery
    Momentum,     // Small drop with momentum continuation
    Reversal,     // Large drop with reversal signal
}

// =============================================================================
// PREDICTION TYPES
// =============================================================================

/// Prediction result for entry decisions
#[derive(Debug, Clone)]
pub struct EntryPrediction {
    pub mint: String,    // Token being predicted
    pub confidence: f64, // Overall confidence (0-1)

    // Success Predictions
    pub success_probability: f64,      // Probability of profitable exit
    pub quick_profit_probability: f64, // Probability of quick profit (>25% in <20min)
    pub expected_profit: f64,          // Expected profit percentage
    pub expected_duration: i64,        // Expected hold duration (seconds)

    // Risk Predictions
    pub risk_probability: f64,      // Probability of significant drawdown
    pub expected_max_drawdown: f64, // Expected maximum drawdown
    pub risk_score: f64,            // Overall risk score (0-1)

    // Pattern Matching
    pub matching_patterns: Vec<String>, // IDs of matching patterns
    pub pattern_confidence: f64,        // Combined pattern confidence

    // Recommendations
    pub entry_recommendation: EntryRecommendation,
    pub confidence_adjustment: f64, // Adjustment to entry confidence (-1 to 1)

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum EntryRecommendation {
    StrongBuy,   // High confidence, good risk/reward
    Buy,         // Good probability, acceptable risk
    Neutral,     // Unclear signal, rely on heuristics
    Avoid,       // Poor probability or high risk
    StrongAvoid, // Very poor outlook
}

/// Prediction result for exit decisions
#[derive(Debug, Clone)]
pub struct ExitPrediction {
    pub mint: String,        // Token being predicted
    pub current_profit: f64, // Current P&L percentage
    pub hold_duration: i64,  // Current hold duration (seconds)

    // Exit Timing Predictions
    pub peak_reached_probability: f64, // Probability that peak has been reached
    pub further_upside_probability: f64, // Probability of further gains
    pub reversal_probability: f64,     // Probability of reversal

    // Target Adjustments
    pub recommended_trailing_stop: f64, // Recommended trailing stop percentage
    pub recommended_profit_target: f64, // Recommended profit target
    pub urgency_score: f64,             // Exit urgency (0-1)

    // Risk Assessment
    pub drawdown_risk: f64, // Risk of significant drawdown
    pub time_pressure: f64, // Pressure from holding duration

    // Recommendations
    pub exit_recommendation: ExitRecommendation,
    pub exit_score_adjustment: f64, // Adjustment to exit score (-1 to 1)

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum ExitRecommendation {
    HoldStrong, // Strong conviction to hold
    Hold,       // Lean toward holding
    Neutral,    // No strong opinion
    Sell,       // Lean toward selling
    SellStrong, // Strong conviction to sell
}

// =============================================================================
// SIMILARITY AND ANALYSIS TYPES
// =============================================================================

/// Token similarity analysis for pattern matching
#[derive(Debug, Clone)]
pub struct TokenSimilarity {
    pub target_mint: String,              // Token being analyzed
    pub similar_mints: Vec<SimilarToken>, // Similar tokens found
    pub similarity_score: f64,            // Overall similarity score
    pub confidence: f64,                  // Confidence in similarity
}

#[derive(Debug, Clone)]
pub struct SimilarToken {
    pub mint: String,              // Similar token mint
    pub symbol: String,            // Similar token symbol
    pub similarity_score: f64,     // Similarity score (0-1)
    pub trade_count: usize,        // Number of trades available
    pub avg_profit: f64,           // Average profit for this token
    pub success_rate: f64,         // Success rate for this token
    pub last_trade: DateTime<Utc>, // When last traded
}

/// Market context at time of analysis
#[derive(Debug, Clone)]
pub struct MarketContext {
    pub timestamp: DateTime<Utc>,         // When context was captured
    pub sol_price_usd: f64,               // SOL price in USD
    pub market_sentiment: f64,            // Overall market sentiment (-1 to 1)
    pub volume_trend: f64,                // Volume trend (-1 to 1)
    pub volatility_index: f64,            // Market volatility (0-1)
    pub active_trader_count: Option<i64>, // Number of active traders
}
