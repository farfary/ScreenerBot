//! entry_new.rs - Optimized Fast Scalping Entry System
//!
//! Complete rewrite focused on:
//! - Database-driven parameter optimization for 5-10% minimum profit targets
//! - ATH prevention using comprehensive price history analysis
//! - Seconds-level entry decisions for fast scalping opportunities
//! - Jupiter profit optimization with high-frequency trading focus
//! - Enhanced liquidity and activity filtering for maximum success rates
//!
//! Key Optimizations:
//! - Aggressive timeframes (1-60 second windows vs 5-300 second windows)
//! - ATH detection prevents buying peaks using 15min/1hr/6hr history analysis
//! - Activity-weighted confidence scoring (high txn volume = higher success)
//! - Tighter liquidity bands optimized for scalping success rates
//! - Perfect storm detection for compound entry opportunities

use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::positions_types::Token;
use crate::tokens::get_price;
use crate::tokens::types::PriceOptions;
use chrono::{ DateTime, Utc };
use std::collections::VecDeque;

// ================================ FAST SCALPING PARAMETERS ================================

/// Aggressive time windows for fast scalping (seconds) - optimized for speed
const SCALP_WINDOWS_SEC: &[i64] = &[1, 3, 5, 10, 20, 30, 60];

/// Drop detection range optimized for scalping
const MIN_SCALP_DROP: f64 = 3.0; // Lower minimum for fast entries
const MAX_SCALP_DROP: f64 = 25.0; // Tighter maximum to avoid extreme volatility

/// Liquidity bounds optimized for scalping success (SOL reserves)
const MIN_SCALP_LIQUIDITY: f64 = 15.0; // Higher minimum for faster fills
const MAX_SCALP_LIQUIDITY: f64 = 800.0; // Upper bound for optimal slippage

/// Activity thresholds for scalping opportunities
const HIGH_ACTIVITY_THRESHOLD: f64 = 30.0; // Txns per 5min for premium entries
const MED_ACTIVITY_THRESHOLD: f64 = 15.0; // Medium activity threshold
const MIN_ACTIVITY_THRESHOLD: f64 = 8.0; // Minimum viable activity

/// ATH Prevention parameters
const ATH_LOOKBACK_PERIODS: &[(i64, f64)] = &[
    (900, 0.95), // 15min: prevent if within 5% of recent high
    (3600, 0.9), // 1hr: prevent if within 10% of hourly high
    (21600, 0.85), // 6hr: prevent if within 15% of 6hr high
];

/// Confidence thresholds optimized for scalping
const SCALP_CONFIDENCE_THRESHOLD: f64 = 35.0; // Higher threshold for quality
const PERFECT_SCALP_THRESHOLD: f64 = 55.0; // Premium scalp opportunity

/// Data freshness requirements (seconds)
const MAX_DATA_AGE_SEC: i64 = 15; // Tighter data freshness for scalping
const MIN_HISTORY_POINTS: usize = 8; // Minimum price points for analysis

// ================================ ENTRY ANALYSIS TYPES ================================

#[derive(Debug, Clone)]
pub struct ScalpingSignal {
    /// Time window for the drop (seconds)
    pub window_sec: i64,
    /// Drop percentage from window high to current
    pub drop_percent: f64,
    /// Window high price for reference
    pub window_high: f64,
    /// Current price
    pub current_price: f64,
    /// Number of price samples in window
    pub samples: usize,
    /// Price velocity (percent per minute)
    pub velocity_per_minute: f64,
    /// Activity score during the window
    pub activity_factor: f64,
}

#[derive(Debug, Clone)]
pub struct ATHAnalysis {
    /// Is current price near ATH in any timeframe?
    pub near_ath: bool,
    /// ATH percentage for each period (15min, 1hr, 6hr)
    pub ath_percentages: Vec<f64>,
    /// Worst ATH percentage (highest)
    pub worst_ath_pct: f64,
    /// Safe for entry (not near ATH in any period)
    pub ath_safe: bool,
}

// ================================ MAIN ENTRY FUNCTION ================================

/// Fast scalping entry decision optimized for 5-10% minimum profit targets
/// Returns (should_enter, confidence_score, analysis_reason)
pub async fn should_enter_scalp(token: &Token) -> (bool, f64, String) {
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "SCALP_ENTRY_START",
            &format!("🚀 Fast scalp analysis: {}", token.symbol)
        );
    }

    // Step 1: Get fresh pool data with tight age requirements
    let (current_price, data_age_sec, reserve_sol) = match get_fresh_pool_data(token).await {
        Some(data) => data,
        None => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "NO_FRESH_DATA",
                    &format!("❌ {} no fresh pool data", token.symbol)
                );
            }
            return (false, 5.0, "No fresh pool data for scalping".to_string());
        }
    };

    // Step 2: Scalping liquidity filter (tighter bounds)
    if reserve_sol < MIN_SCALP_LIQUIDITY || reserve_sol > MAX_SCALP_LIQUIDITY {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "SCALP_LIQUIDITY_FILTER",
                &format!(
                    "❌ {} liquidity {:.1} SOL outside scalp range {:.0}-{:.0}",
                    token.symbol,
                    reserve_sol,
                    MIN_SCALP_LIQUIDITY,
                    MAX_SCALP_LIQUIDITY
                )
            );
        }
        return (false, 8.0, format!("Liquidity {:.1} SOL outside scalp range", reserve_sol));
    }

    // Step 3: Get comprehensive price history for analysis
    let pool_service = get_pool_service();
    let mut price_history = pool_service.get_price_history(&token.mint).await;
    price_history.retain(|(_, p)| *p > 0.0 && p.is_finite());

    // Refresh history if insufficient (critical for scalping)
    if price_history.len() < MIN_HISTORY_POINTS {
        let _ = get_price(&token.mint, Some(PriceOptions::default()), false).await;
        let mut refreshed = pool_service.get_price_history(&token.mint).await;
        refreshed.retain(|(_, p)| *p > 0.0 && p.is_finite());
        if refreshed.len() > price_history.len() {
            price_history = refreshed;
        }

        if price_history.len() < MIN_HISTORY_POINTS {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "INSUFFICIENT_SCALP_HISTORY",
                    &format!(
                        "❌ {} insufficient history: {} < {}",
                        token.symbol,
                        price_history.len(),
                        MIN_HISTORY_POINTS
                    )
                );
            }
            return (false, 12.0, "Insufficient price history for scalping".to_string());
        }
    }

    // Step 4: ATH Prevention Analysis
    let ath_analysis = analyze_ath_risk(&price_history, current_price).await;
    if !ath_analysis.ath_safe {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "ATH_PREVENTION",
                &format!(
                    "❌ {} near ATH: worst {:.1}% (periods: {:.1}%, {:.1}%, {:.1}%)",
                    token.symbol,
                    ath_analysis.worst_ath_pct,
                    ath_analysis.ath_percentages.get(0).unwrap_or(&0.0),
                    ath_analysis.ath_percentages.get(1).unwrap_or(&0.0),
                    ath_analysis.ath_percentages.get(2).unwrap_or(&0.0)
                )
            );
        }
        return (
            false,
            15.0,
            format!("ATH prevention: {:.1}% of recent high", ath_analysis.worst_ath_pct),
        );
    }

    // Step 5: Activity scoring for scalping opportunities
    let activity_score = calculate_scalp_activity_score(token);
    if activity_score < 0.3 {
        // Minimum activity for scalping
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "LOW_SCALP_ACTIVITY",
                &format!(
                    "❌ {} insufficient activity for scalping: {:.1}",
                    token.symbol,
                    activity_score
                )
            );
        }
        return (false, 18.0, "Insufficient activity for scalping".to_string());
    }

    // Step 6: Fast drop detection across aggressive windows
    let scalp_signal = detect_scalping_opportunity(
        &price_history,
        current_price,
        activity_score
    ).await;

    match scalp_signal {
        Some(signal) => {
            // Step 7: Calculate scalping confidence score
            let confidence = calculate_scalping_confidence(
                &signal,
                reserve_sol,
                &ath_analysis,
                token
            );

            let approved = confidence >= SCALP_CONFIDENCE_THRESHOLD;
            let is_perfect_scalp = confidence >= PERFECT_SCALP_THRESHOLD;

            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "SCALP_ANALYSIS_COMPLETE",
                    &format!(
                        "🎯 {} scalp: -{:.1}%/{}s → conf {:.1}% → {} [activity:{:.1} liquidity:{:.0} ath_safe:{} perfect:{}]",
                        token.symbol,
                        signal.drop_percent,
                        signal.window_sec,
                        confidence,
                        if approved {
                            "ENTER"
                        } else {
                            "REJECT"
                        },
                        activity_score,
                        reserve_sol,
                        ath_analysis.ath_safe,
                        is_perfect_scalp
                    )
                );
            }

            let reason = if approved {
                if is_perfect_scalp {
                    format!(
                        "Perfect scalp: -{:.1}%/{}s, {:.0} SOL, high activity",
                        signal.drop_percent,
                        signal.window_sec,
                        reserve_sol
                    )
                } else {
                    format!(
                        "Fast scalp: -{:.1}%/{}s (conf: {:.1}%)",
                        signal.drop_percent,
                        signal.window_sec,
                        confidence
                    )
                }
            } else {
                format!(
                    "Scalp analysis: -{:.1}%/{}s, conf {:.1}% < {:.0}%",
                    signal.drop_percent,
                    signal.window_sec,
                    confidence,
                    SCALP_CONFIDENCE_THRESHOLD
                )
            };

            (approved, confidence, reason)
        }
        None => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "NO_SCALP_OPPORTUNITY",
                    &format!(
                        "❌ {} no scalp drops {:.0}-{:.0}% in fast windows",
                        token.symbol,
                        MIN_SCALP_DROP,
                        MAX_SCALP_DROP
                    )
                );
            }
            (false, 25.0, "No scalping opportunity detected".to_string())
        }
    }
}

// ================================ HELPER FUNCTIONS ================================

/// Get fresh pool data with tight age requirements for scalping
async fn get_fresh_pool_data(token: &Token) -> Option<(f64, i64, f64)> {
    match get_price(&token.mint, Some(PriceOptions::default()), false).await {
        Some(price_result) => {
            match price_result.sol_price() {
                Some(price) if price > 0.0 && price.is_finite() => {
                    let data_age_sec = (Utc::now() - price_result.calculated_at).num_seconds();

                    // Tighter data freshness for scalping
                    if data_age_sec > MAX_DATA_AGE_SEC {
                        return None;
                    }

                    let reserve_sol = price_result.reserve_sol.unwrap_or_else(|| {
                        token.liquidity
                            .as_ref()
                            .and_then(|l| l.usd)
                            .map(|usd_liq| usd_liq / 200.0) // Updated conversion at $200/SOL
                            .unwrap_or(0.0)
                    });

                    Some((price, data_age_sec, reserve_sol))
                }
                _ => None,
            }
        }
        None => None,
    }
}

/// Analyze ATH risk across multiple timeframes
async fn analyze_ath_risk(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64
) -> ATHAnalysis {
    let now = Utc::now();
    let mut ath_percentages = Vec::new();
    let mut near_ath = false;

    for &(lookback_sec, threshold) in ATH_LOOKBACK_PERIODS.iter() {
        // Get prices in the lookback period
        let period_prices: Vec<f64> = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= lookback_sec)
            .map(|(_, p)| *p)
            .collect();

        if period_prices.len() >= 3 {
            let period_high = period_prices.iter().fold(0.0f64, |a, b| a.max(*b));
            if period_high > 0.0 && period_high.is_finite() {
                let ath_percentage = current_price / period_high;
                ath_percentages.push(ath_percentage * 100.0);

                // Check if we're too close to ATH
                if ath_percentage >= threshold {
                    near_ath = true;
                }
            } else {
                ath_percentages.push(0.0);
            }
        } else {
            ath_percentages.push(0.0);
        }
    }

    let worst_ath_pct = ath_percentages.iter().fold(0.0f64, |a, b| a.max(*b));
    let ath_safe = !near_ath;

    ATHAnalysis {
        near_ath,
        ath_percentages,
        worst_ath_pct,
        ath_safe,
    }
}

/// Calculate activity score optimized for scalping (0.0 to 1.0 scale)
fn calculate_scalp_activity_score(token: &Token) -> f64 {
    // Extract 5-minute transaction count for activity analysis
    let txns_5min = token.txns
        .as_ref()
        .and_then(|txns| txns.m5)
        .map(|t| t.buys + t.sells)
        .unwrap_or(0.0);

    // Scalping-optimized activity scoring
    if txns_5min >= HIGH_ACTIVITY_THRESHOLD {
        1.0 // Premium scalping activity
    } else if txns_5min >= MED_ACTIVITY_THRESHOLD {
        0.8 // Good scalping activity
    } else if txns_5min >= MIN_ACTIVITY_THRESHOLD {
        0.5 // Minimum viable activity
    } else {
        0.2 // Low activity (reduced but not eliminated)
    }
}

/// Detect scalping opportunities across aggressive time windows
async fn detect_scalping_opportunity(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    activity_score: f64
) -> Option<ScalpingSignal> {
    let now = Utc::now();
    let mut best_signal: Option<ScalpingSignal> = None;

    for &window_sec in SCALP_WINDOWS_SEC.iter() {
        // Get prices within the aggressive time window
        let window_prices: Vec<f64> = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= window_sec)
            .map(|(_, p)| *p)
            .collect();

        if window_prices.len() < 2 {
            continue;
        }

        // Find the high within this window
        let window_high = window_prices.iter().fold(0.0f64, |a, b| a.max(*b));
        if window_high <= 0.0 || !window_high.is_finite() {
            continue;
        }

        // Calculate drop percentage
        let drop_percent = ((window_high - current_price) / window_high) * 100.0;
        if drop_percent < MIN_SCALP_DROP || drop_percent > MAX_SCALP_DROP {
            continue;
        }

        // Calculate velocity for this window
        let velocity = calculate_scalp_velocity(&window_prices, window_sec);

        let signal = ScalpingSignal {
            window_sec,
            drop_percent,
            window_high,
            current_price,
            samples: window_prices.len(),
            velocity_per_minute: velocity,
            activity_factor: activity_score,
        };

        // Select best signal (prefer larger drops, then shorter windows, then better velocity)
        let is_better = match &best_signal {
            None => true,
            Some(best) => {
                if signal.drop_percent > best.drop_percent + 0.5 {
                    true // Significantly larger drop
                } else if (signal.drop_percent - best.drop_percent).abs() <= 0.5 {
                    if signal.window_sec < best.window_sec {
                        true // Faster drop
                    } else if signal.window_sec == best.window_sec {
                        signal.velocity_per_minute < best.velocity_per_minute // Better velocity
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };

        if is_better {
            best_signal = Some(signal);
        }
    }

    best_signal
}

/// Calculate velocity for scalping windows (percent per minute)
fn calculate_scalp_velocity(prices: &[f64], window_sec: i64) -> f64 {
    if prices.len() < 2 || window_sec <= 0 {
        return 0.0;
    }

    let first = prices[0];
    let last = prices[prices.len() - 1];

    if first <= 0.0 || !first.is_finite() || !last.is_finite() {
        return 0.0;
    }

    let percent_change = ((last - first) / first) * 100.0;
    let minutes = (window_sec as f64) / 60.0;

    if minutes <= 0.0 {
        return 0.0;
    }

    percent_change / minutes
}

/// Calculate comprehensive scalping confidence score
fn calculate_scalping_confidence(
    signal: &ScalpingSignal,
    reserve_sol: f64,
    ath_analysis: &ATHAnalysis,
    token: &Token
) -> f64 {
    let mut confidence = 30.0; // Base confidence for scalping

    // Drop magnitude scoring (optimized for scalping range)
    let drop_score = if signal.drop_percent >= 5.0 && signal.drop_percent <= 12.0 {
        1.0 // Sweet spot for scalping
    } else if signal.drop_percent >= 3.0 && signal.drop_percent <= 18.0 {
        0.9 // Good scalping range
    } else if signal.drop_percent <= 25.0 {
        0.7 // Acceptable range
    } else {
        0.4 // Higher volatility risk
    };
    confidence += drop_score * 25.0;

    // Activity factor (high weight for scalping)
    confidence += signal.activity_factor * 20.0;

    // Liquidity scoring optimized for scalping fills
    let liquidity_score = if reserve_sol >= 100.0 && reserve_sol <= 500.0 {
        1.0 // Optimal for scalping fills
    } else if reserve_sol >= 50.0 && reserve_sol <= 700.0 {
        0.9 // Good scalping liquidity
    } else if reserve_sol >= 25.0 {
        0.7 // Adequate liquidity
    } else {
        0.4 // Limited liquidity
    };
    confidence += liquidity_score * 15.0;

    // Window speed bonus (aggressive scalping preference)
    confidence += match signal.window_sec {
        1 => 20.0, // Ultra-fast scalp
        3 => 18.0, // Very fast scalp
        5 => 15.0, // Fast scalp
        10 => 12.0, // Quick scalp
        20 => 8.0, // Medium scalp
        30 => 5.0, // Slower scalp
        60 => 2.0, // Slowest acceptable
        _ => 0.0,
    };

    // ATH safety bonus
    if ath_analysis.ath_safe {
        confidence += 8.0;

        // Extra bonus for being well below recent highs
        if ath_analysis.worst_ath_pct < 70.0 {
            confidence += 5.0;
        }
    }

    // Velocity adjustments
    if signal.velocity_per_minute < -30.0 {
        confidence += 6.0; // Strong downward momentum
    } else if signal.velocity_per_minute > 20.0 {
        confidence -= 4.0; // Upward momentum (risk)
    }

    // Perfect scalp multiplier (compound effect)
    let is_perfect_scalp =
        signal.drop_percent >= 5.0 &&
        signal.drop_percent <= 10.0 &&
        signal.window_sec <= 10 &&
        reserve_sol >= 100.0 &&
        reserve_sol <= 400.0 &&
        signal.activity_factor >= 0.8 &&
        ath_analysis.worst_ath_pct < 75.0;

    if is_perfect_scalp {
        confidence *= 1.25; // 25% boost for perfect conditions
    }

    confidence.clamp(0.0, 95.0)
}

// ================================ PROFIT TARGET CALCULATION ================================

/// Calculate profit targets optimized for fast scalping (5-10% minimum focus)
pub async fn get_scalp_profit_targets(token: &Token) -> (f64, f64) {
    // Get fresh pool data for analysis
    let reserve_sol = match get_fresh_pool_data(token).await {
        Some((_, _, reserves)) => reserves,
        None =>
            token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .map(|usd_liq| usd_liq / 200.0)
                .unwrap_or(50.0), // Default reserves
    };

    // Activity-adjusted base targets
    let activity_score = calculate_scalp_activity_score(token);
    let activity_multiplier = 1.0 + (activity_score - 0.5) * 0.3; // Higher activity = higher targets

    // Base profit targets optimized for scalping success
    let (mut min_profit, mut max_profit) = if reserve_sol < 25.0 {
        (18.0, 45.0) // Lower liquidity = higher targets
    } else if reserve_sol < 100.0 {
        (12.0, 35.0) // Medium liquidity
    } else if reserve_sol < 400.0 {
        (8.0, 28.0) // Optimal scalping liquidity (sweet spot)
    } else if reserve_sol < 700.0 {
        (10.0, 32.0) // High liquidity
    } else {
        (15.0, 40.0) // Very high liquidity
    };

    // Apply activity multiplier
    min_profit *= activity_multiplier;
    max_profit *= activity_multiplier;

    // Volatility adjustment using recent price action
    let pool_service = get_pool_service();
    let price_history = pool_service.get_price_history(&token.mint).await;
    if price_history.len() >= 5 {
        let now = Utc::now();
        let recent_prices: Vec<f64> = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= 300) // 5min window
            .map(|(_, p)| *p)
            .collect();

        if recent_prices.len() >= 3 {
            let high = recent_prices.iter().fold(0.0f64, |a, b| a.max(*b));
            let low = recent_prices.iter().fold(f64::INFINITY, |a, b| a.min(*b));

            if high.is_finite() && low.is_finite() && high > 0.0 && low > 0.0 {
                let volatility_range = ((high - low) / high) * 100.0;
                let vol_multiplier = (1.0 + volatility_range / 100.0).clamp(0.8, 1.4);

                min_profit *= vol_multiplier;
                max_profit *= vol_multiplier;
            }
        }
    }

    // Ensure minimum scalping thresholds and reasonable spread
    min_profit = min_profit.clamp(5.0, 25.0); // 5-25% minimum range
    max_profit = max_profit.clamp(20.0, 60.0); // 20-60% maximum range

    // Ensure proper spread
    if max_profit - min_profit < 8.0 {
        max_profit = min_profit + 8.0;
    }

    (min_profit, max_profit)
}

// ================================ PUBLIC API COMPATIBILITY ================================

/// Main entry function (maintains compatibility with existing interface)
/// Redirects to the optimized scalping system
pub async fn should_enter(token: &Token) -> (bool, f64, String) {
    should_enter_scalp(token).await
}

/// Profit target calculation (maintains compatibility)
/// Redirects to optimized scalping profit targets
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    get_scalp_profit_targets(token).await
}
