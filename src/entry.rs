/// Advanced Drop Detection and Entry Analysis System
///
/// This module provides sophisticated drop pattern detection with confidence scoring
/// for trading decisions. It focuses on detecting different styles of price drops
/// from -5% to -100% with appropriate entry timing and confidence assessment.
///
/// ## Key Features:
/// - **Multi-Style Drop Detection**: Detects flash crashes, gradual declines, capitulation wicks
/// - **Confidence Scoring**: Returns probability scores for drop likelihood and quality
/// - **Fast Pool Price Integration**: Uses real-time pool data for immediate drop detection
/// - **OHLCV ATH Protection**: Prevents entries near all-time highs using 1m OHLCV data
/// - **Progressive Entry Logic**: Different strategies for different drop magnitudes and styles
///
/// ## Drop Detection Categories:
/// - **Flash Drops (5-15%)**: Quick, sudden price movements with high velocity
/// - **Moderate Drops (15-35%)**: Sustained downward pressure with momentum analysis
/// - **Deep Drops (35-60%)**: Major corrections requiring careful timing
/// - **Extreme Drops (60-100%)**: Potential capitulation events with high risk/reward
///
/// ## Confidence Scoring (0-100):
/// - 0-30: Low confidence - drop may not be real or sustainable
/// - 30-60: Moderate confidence - decent entry opportunity with some risk
/// - 60-85: High confidence - strong entry signal with good risk/reward
/// - 85-100: Extreme confidence - exceptional entry opportunity

use crate::global::is_debug_entry_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::{ get_pool_service, Token, PriceOptions };
use crate::tokens::pool::{ PUMP_FUN_AMM_PROGRAM_ID, PoolPriceInfo };
use chrono::{ DateTime, Utc };

// =============================================================================
// CORE CONFIGURATION PARAMETERS
// =============================================================================

// Drop Detection Thresholds - LOWERED FOR MORE AGGRESSIVE ENTRY
const FLASH_DROP_MIN: f64 = 2.0; // Minimum flash drop % (reduced from 2.5%)
const FLASH_DROP_MAX: f64 = 15.0; // Maximum flash drop %
const MODERATE_DROP_MIN: f64 = 5.0; // Minimum moderate drop % (reduced from 8.0%)
const MODERATE_DROP_MAX: f64 = 35.0; // Maximum moderate drop %
const DEEP_DROP_MIN: f64 = 15.0; // Minimum deep drop % (reduced from 20.0%)
const DEEP_DROP_MAX: f64 = 60.0; // Maximum deep drop %
const EXTREME_DROP_MIN: f64 = 50.0; // Minimum extreme drop % (reduced from 60.0%)
const EXTREME_DROP_MAX: f64 = 100.0; // Maximum extreme drop %

// Pump.fun Token Optimal Entry Configuration
const PUMPFUN_SOL_RESERVE_MIN: f64 = 23.0; // Minimum SOL reserve for easy entry
const PUMPFUN_SOL_RESERVE_MAX: f64 = 26.0; // Maximum SOL reserve for easy entry
const PUMPFUN_EASY_ENTRY_CONFIDENCE_BOOST: f64 = 25.0; // Confidence boost for optimal range
const PUMPFUN_EASY_ENTRY_MIN_BASE_CONFIDENCE: f64 = 45.0; // Min confidence to apply boost

// Time Windows for Analysis
const FLASH_WINDOW_SEC: i64 = 20; // Flash drop detection window (doubled from 10s)
const MODERATE_WINDOW_SEC: i64 = 120; // Moderate drop detection window (doubled from 60s)
const DEEP_WINDOW_SEC: i64 = 300; // Deep drop detection window (5 min)
const EXTREME_WINDOW_SEC: i64 = 900; // Extreme drop detection window (15 min)
const ULTRA_FLASH_WINDOW_SEC: i64 = 6; // Very fast drop detection window (doubled from 3s)
const WICK_WINDOW_SEC: i64 = 30; // Wick detection window (drop + partial recovery)
const CASCADE_WINDOW_SEC: i64 = 120; // Stair-step (cascade) window (2 min)
const MEDIAN_DIP_WINDOW_SEC: i64 = 20; // Median-dip detection window

// ATH Protection Settings
const ATH_MIN_DISTANCE_PERCENT: f64 = 5.0; // Minimum 5% below ATH

// Confidence Scoring Base Values
const FLASH_BASE_CONFIDENCE: f64 = 70.0; // Base confidence for flash drops
const MODERATE_BASE_CONFIDENCE: f64 = 75.0; // Base confidence for moderate drops
const DEEP_BASE_CONFIDENCE: f64 = 80.0; // Base confidence for deep drops
const EXTREME_BASE_CONFIDENCE: f64 = 85.0; // Base confidence for extreme drops

// Price History Requirements
const MIN_PRICE_POINTS: usize = 2; // Minimum price points needed (reduced from 3)
const MAX_DATA_AGE_MIN: i64 = 10; // Maximum data age in minutes

// Liquidity Filters (Basic)
const MIN_LIQUIDITY_USD: f64 = 100.0; // Minimum liquidity requirement
const MAX_LIQUIDITY_USD: f64 = 50_000_000.0; // Maximum liquidity to avoid mega caps

// =============================================================================
// DROP DETECTION STRUCTURES
// =============================================================================

#[derive(Debug, Clone)]
pub struct DropAnalysis {
    pub drop_percent: f64,
    pub drop_style: DropStyle,
    pub confidence: f64,
    pub velocity_per_minute: f64,
    pub time_window_used: i64,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub enum DropStyle {
    Flash, // Quick sudden drop (5-15%)
    Moderate, // Sustained decline (15-35%)
    Deep, // Major correction (35-60%)
    Extreme, // Potential capitulation (60-100%)
    UltraFlash, // Very quick sudden drop (5-12%) over ~3s
    WickRebound, // Sharp wick down then partial recovery within short window
    Cascade, // Stair-step sequential declines across ~2min
    MedianDip, // Current price deviates significantly below short-term median
}

impl DropStyle {
    fn to_string(&self) -> &'static str {
        match self {
            DropStyle::Flash => "FLASH",
            DropStyle::Moderate => "MODERATE",
            DropStyle::Deep => "DEEP",
            DropStyle::Extreme => "EXTREME",
            DropStyle::UltraFlash => "ULTRA_FLASH",
            DropStyle::WickRebound => "WICK_REBOUND",
            DropStyle::Cascade => "CASCADE",
            DropStyle::MedianDip => "MEDIAN_DIP",
        }
    }
}

// =============================================================================
// MAIN ENTRY FUNCTION
// =============================================================================

/// Main entry point for determining if a token should be bought
/// Returns (approved_for_entry, confidence_score, reason)
pub async fn should_buy(token: &Token) -> (bool, f64, String) {
    let pool_service = get_pool_service();

    // Get current pool price and liquidity first
    let (current_price, liquidity_usd) = match
        pool_service.get_pool_price(&token.mint, None, &PriceOptions::default()).await
    {
        Some(result) => {
            let price = result.price_sol.unwrap_or(0.0);
            let liquidity = result.liquidity_usd;
            if price <= 0.0 || !price.is_finite() {
                return (false, 10.0, "Invalid price data".to_string());
            }
            (price, liquidity)
        }
        None => {
            return (false, 5.0, "No valid pool data".to_string());
        }
    };

    // Initialize base confidence based on basic metrics
    let mut confidence = calculate_base_confidence(liquidity_usd);

    // Basic liquidity filter (affects entry approval but not confidence calculation)
    let liquidity_check = if liquidity_usd < MIN_LIQUIDITY_USD {
        (false, format!("Liquidity too low: ${:.0} < ${:.0}", liquidity_usd, MIN_LIQUIDITY_USD))
    } else if liquidity_usd > MAX_LIQUIDITY_USD {
        (false, format!("Liquidity too high: ${:.0} > ${:.0}", liquidity_usd, MAX_LIQUIDITY_USD))
    } else {
        (true, "Liquidity OK".to_string())
    };

    // Check for pump.fun tokens in optimal SOL reserve range (23-26 SOL)
    let pumpfun_easy_entry = check_pumpfun_easy_entry(token).await;

    // Get price history for drop analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    // Update confidence based on available data
    if price_history.len() < MIN_PRICE_POINTS {
        confidence = confidence.min(25.0);
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "INSUFFICIENT_HISTORY",
                &format!(
                    "❌ {} insufficient price history: {} < {} points",
                    token.symbol,
                    price_history.len(),
                    MIN_PRICE_POINTS
                )
            );
        }
        return (
            false,
            confidence,
            format!("Insufficient price history: {} < {}", price_history.len(), MIN_PRICE_POINTS),
        );
    }

    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "PRICE_HISTORY_OK",
            &format!(
                "✅ {} price history available: {} points, current: {:.6} SOL, liquidity: ${:.0}",
                token.symbol,
                price_history.len(),
                current_price,
                liquidity_usd
            )
        );
    }

    // Soft ATH penalty using OHLCV data (don't hard-block pool-based signals)
    let ath_distance_opt = get_ath_distance_percent(&token.mint, current_price).await;

    // Analyze drop patterns with confidence scoring
    let drop_analysis = analyze_drop_patterns(&price_history, current_price, liquidity_usd).await;

    if is_debug_entry_enabled() {
        if drop_analysis.is_some() {
            log(
                LogTag::Entry,
                "DROP_DETECTED",
                &format!("✅ {} drop analysis found pattern", token.symbol)
            );
        } else {
            log(
                LogTag::Entry,
                "NO_DROP_PATTERN",
                &format!(
                    "❌ {} no drop pattern detected in {} price points",
                    token.symbol,
                    price_history.len()
                )
            );

            // Log recent price data for debugging
            if price_history.len() > 0 {
                let recent_prices: Vec<String> = price_history
                    .iter()
                    .take(5)
                    .map(|(ts, price)| format!("{:.6}@{}", price, ts.format("%H:%M:%S")))
                    .collect();
                log(
                    LogTag::Entry,
                    "RECENT_PRICES",
                    &format!("📊 {} recent prices: [{}]", token.symbol, recent_prices.join(", "))
                );
            }
        }
    }

    if let Some(analysis) = drop_analysis {
        // Update confidence based on drop analysis and apply soft ATH penalty
        let mut analysis = analysis;

        // Apply pump.fun easy entry boost if applicable
        if let Some((is_pumpfun, sol_reserves, in_range)) = pumpfun_easy_entry {
            if
                is_pumpfun &&
                in_range &&
                analysis.confidence >= PUMPFUN_EASY_ENTRY_MIN_BASE_CONFIDENCE
            {
                let old_confidence = analysis.confidence;
                analysis.confidence = (
                    analysis.confidence + PUMPFUN_EASY_ENTRY_CONFIDENCE_BOOST
                ).min(95.0);
                analysis.reasoning = format!(
                    "{} (pump.fun {:.1} SOL +{:.0}% confidence)",
                    analysis.reasoning,
                    sol_reserves,
                    analysis.confidence - old_confidence
                );

                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        "PUMPFUN_BOOST",
                        &format!(
                            "🚀 {} pump.fun easy entry boost: {:.1} SOL reserves → confidence {:.0}% → {:.0}%",
                            token.symbol,
                            sol_reserves,
                            old_confidence,
                            analysis.confidence
                        )
                    );
                }
            } else if is_pumpfun && is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "PUMPFUN_INFO",
                    &format!(
                        "📊 {} pump.fun token: {:.1} SOL reserves (optimal: {:.0}-{:.0}), confidence: {:.0}%",
                        token.symbol,
                        sol_reserves,
                        PUMPFUN_SOL_RESERVE_MIN,
                        PUMPFUN_SOL_RESERVE_MAX,
                        analysis.confidence
                    )
                );
            }
        }

        if let Some(distance) = ath_distance_opt {
            if distance < ATH_MIN_DISTANCE_PERCENT {
                let penalty = match analysis.drop_style {
                    DropStyle::UltraFlash | DropStyle::Flash | DropStyle::MedianDip => 15.0,
                    DropStyle::Moderate => 10.0,
                    DropStyle::WickRebound | DropStyle::Cascade => 6.0,
                    DropStyle::Deep => 4.0,
                    DropStyle::Extreme => 0.0,
                };
                analysis.confidence = (analysis.confidence - penalty).max(0.0);
                analysis.reasoning = format!(
                    "{} (near ATH penalty -{:.0}%)",
                    analysis.reasoning,
                    penalty
                );
            }
        }

        confidence = analysis.confidence;

        let approved =
            liquidity_check.0 && should_enter_based_on_analysis(&analysis, liquidity_usd);

        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "DROP_ANALYSIS",
                &format!(
                    "🎯 {} {}: -{:.1}% drop, confidence: {:.0}%, velocity: {:.1}%/min, approved: {}",
                    token.symbol,
                    analysis.drop_style.to_string(),
                    analysis.drop_percent,
                    analysis.confidence,
                    analysis.velocity_per_minute,
                    approved
                )
            );
        }

        return (
            approved,
            confidence,
            if approved {
                analysis.reasoning
            } else {
                format!("{} ({})", analysis.reasoning, liquidity_check.1)
            },
        );
    }

    // No significant drop detected - check for pump.fun easy entry opportunity
    let final_reason = if !liquidity_check.0 {
        format!("No significant drop pattern detected ({})", liquidity_check.1)
    } else if let Some((is_pumpfun, sol_reserves, in_range)) = pumpfun_easy_entry {
        if is_pumpfun && in_range {
            // Even without a significant drop, pump.fun tokens in optimal range can be good entries
            let pumpfun_confidence = (confidence + PUMPFUN_EASY_ENTRY_CONFIDENCE_BOOST * 0.6).min(
                85.0
            );

            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "PUMPFUN_NO_DROP",
                    &format!(
                        "🎯 {} pump.fun no-drop entry: {:.1} SOL reserves, confidence: {:.0}%",
                        token.symbol,
                        sol_reserves,
                        pumpfun_confidence
                    )
                );
            }

            // Consider entry if confidence is reasonable (lower threshold for pump.fun)
            if pumpfun_confidence >= 35.0 {
                return (
                    true,
                    pumpfun_confidence,
                    format!("Pump.fun easy entry ({:.1} SOL reserves)", sol_reserves),
                );
            } else {
                format!("Pump.fun token ({:.1} SOL) but confidence too low", sol_reserves)
            }
        } else if is_pumpfun {
            format!("Pump.fun token but SOL reserves not optimal ({:.1} SOL)", sol_reserves)
        } else {
            "No significant drop pattern detected".to_string()
        }
    } else {
        "No significant drop pattern detected".to_string()
    };

    (false, confidence, final_reason)
}

/// Calculate base confidence based on fundamental metrics
fn calculate_base_confidence(liquidity: f64) -> f64 {
    // Base confidence from liquidity
    let confidence: f64 = match liquidity {
        liq if liq >= 1000000.0 => 40.0, // $1M+ liquidity
        liq if liq >= 500000.0 => 35.0, // $500K+ liquidity
        liq if liq >= 100000.0 => 30.0, // $100K+ liquidity
        liq if liq >= 50000.0 => 25.0, // $50K+ liquidity
        liq if liq >= 10000.0 => 20.0, // $10K+ liquidity
        _ => 10.0, // Lower liquidity
    };
    confidence.min(40.0)
}

// =============================================================================
// DROP PATTERN ANALYSIS
// =============================================================================

async fn analyze_drop_patterns(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    liquidity_usd: f64
) -> Option<DropAnalysis> {
    let now = Utc::now();

    // Try different drop detection strategies in order of priority

    // 0. Ultra-Flash Drop Detection (~3 seconds)
    if let Some(analysis) = detect_ultra_flash_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 1. Flash Drop Detection (10 seconds)
    if let Some(analysis) = detect_flash_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 1.5 Wick-Rebound Detection (30 seconds)
    if let Some(analysis) = detect_wick_rebound(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 2. Moderate Drop Detection (1 minute)
    if let Some(analysis) = detect_moderate_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 2.5 Cascade (stair-step) Detection (2 minutes)
    if let Some(analysis) = detect_cascade_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 3. Deep Drop Detection (5 minutes)
    if let Some(analysis) = detect_deep_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 4. Extreme Drop Detection (15 minutes)
    if let Some(analysis) = detect_extreme_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 4.5 Median Dip Detection (current below short-term median)
    if let Some(analysis) = detect_median_dip(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    None
}

// =============================================================================
// INDIVIDUAL DROP DETECTORS
// =============================================================================

fn detect_flash_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= FLASH_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();

    if window_prices.len() < 2 {
        return None;
    }

    let window_high = window_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    if window_high <= 0.0 || !window_high.is_finite() {
        return None;
    }

    let drop_percent = ((window_high - current_price) / window_high) * 100.0;

    if drop_percent >= FLASH_DROP_MIN && drop_percent <= FLASH_DROP_MAX {
        let velocity = calculate_velocity(&window_prices, FLASH_WINDOW_SEC);
        let confidence = calculate_flash_confidence(drop_percent, velocity);

        return Some(DropAnalysis {
            drop_percent,
            drop_style: DropStyle::Flash,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: FLASH_WINDOW_SEC,
            reasoning: format!("Flash drop -{:.1}% in {}s", drop_percent, FLASH_WINDOW_SEC),
        });
    }

    None
}

fn detect_ultra_flash_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= ULTRA_FLASH_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();

    if window_prices.len() < 2 {
        return None;
    }

    let window_high = window_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    if window_high <= 0.0 || !window_high.is_finite() {
        return None;
    }

    let drop_percent = ((window_high - current_price) / window_high) * 100.0;

    // Slightly narrower band than flash to require very sudden drops
    if drop_percent >= 5.0 && drop_percent <= 12.0 {
        let velocity = calculate_velocity(&window_prices, ULTRA_FLASH_WINDOW_SEC);
        // Start from higher base due to immediacy
        let mut confidence = FLASH_BASE_CONFIDENCE + 5.0;
        let drop_factor = (drop_percent - 5.0) / (12.0 - 5.0);
        confidence += drop_factor * 8.0;
        if velocity < -40.0 {
            confidence += 12.0;
        }
        if velocity > 15.0 {
            confidence -= 18.0;
        }
        confidence = confidence.max(25.0).min(97.0);

        return Some(DropAnalysis {
            drop_percent,
            drop_style: DropStyle::UltraFlash,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: ULTRA_FLASH_WINDOW_SEC,
            reasoning: format!(
                "Ultra-flash drop -{:.1}% in {}s",
                drop_percent,
                ULTRA_FLASH_WINDOW_SEC
            ),
        });
    }

    None
}

fn detect_moderate_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= MODERATE_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();

    if window_prices.len() < 3 {
        return None;
    }

    let window_high = window_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    if window_high <= 0.0 || !window_high.is_finite() {
        return None;
    }

    let drop_percent = ((window_high - current_price) / window_high) * 100.0;

    if drop_percent >= MODERATE_DROP_MIN && drop_percent <= MODERATE_DROP_MAX {
        let velocity = calculate_velocity(&window_prices, MODERATE_WINDOW_SEC);
        let confidence = calculate_moderate_confidence(drop_percent, velocity);

        return Some(DropAnalysis {
            drop_percent,
            drop_style: DropStyle::Moderate,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: MODERATE_WINDOW_SEC,
            reasoning: format!("Moderate drop -{:.1}% in {}s", drop_percent, MODERATE_WINDOW_SEC),
        });
    }

    None
}

fn detect_deep_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= DEEP_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();

    if window_prices.len() < 5 {
        return None;
    }

    let window_high = window_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    if window_high <= 0.0 || !window_high.is_finite() {
        return None;
    }

    let drop_percent = ((window_high - current_price) / window_high) * 100.0;

    if drop_percent >= DEEP_DROP_MIN && drop_percent <= DEEP_DROP_MAX {
        let velocity = calculate_velocity(&window_prices, DEEP_WINDOW_SEC);
        let confidence = calculate_deep_confidence(drop_percent, velocity);

        return Some(DropAnalysis {
            drop_percent,
            drop_style: DropStyle::Deep,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: DEEP_WINDOW_SEC,
            reasoning: format!("Deep drop -{:.1}% in {}min", drop_percent, DEEP_WINDOW_SEC / 60),
        });
    }

    None
}

fn detect_extreme_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= EXTREME_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();

    if window_prices.len() < 8 {
        return None;
    }

    let window_high = window_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    if window_high <= 0.0 || !window_high.is_finite() {
        return None;
    }

    let drop_percent = ((window_high - current_price) / window_high) * 100.0;

    if drop_percent >= EXTREME_DROP_MIN && drop_percent <= EXTREME_DROP_MAX {
        let velocity = calculate_velocity(&window_prices, EXTREME_WINDOW_SEC);
        let confidence = calculate_extreme_confidence(drop_percent, velocity);

        return Some(DropAnalysis {
            drop_percent,
            drop_style: DropStyle::Extreme,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: EXTREME_WINDOW_SEC,
            reasoning: format!(
                "Extreme drop -{:.1}% in {}min",
                drop_percent,
                EXTREME_WINDOW_SEC / 60
            ),
        });
    }

    None
}

fn detect_wick_rebound(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window: Vec<(DateTime<Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= WICK_WINDOW_SEC)
        .cloned()
        .collect();

    if window.len() < 3 {
        return None;
    }

    let prices: Vec<f64> = window
        .iter()
        .map(|(_, p)| *p)
        .collect();
    let window_high = prices.iter().fold(0.0f64, |a, b| a.max(*b));
    let window_low = prices.iter().fold(f64::INFINITY, |a, b| a.min(*b));
    if window_high <= 0.0 || !window_high.is_finite() || !window_low.is_finite() {
        return None;
    }

    let total_drop = ((window_high - window_low) / window_high) * 100.0;
    if total_drop < 12.0 {
        return None;
    }

    // Require some rebound off the low
    let recovered = if current_price > window_low {
        ((current_price - window_low) / (window_high - window_low)).max(0.0)
    } else {
        0.0
    };

    if recovered >= 0.2 {
        // at least 20% of wick recovered
        let velocity = calculate_velocity(&prices, WICK_WINDOW_SEC);
        let mut confidence = MODERATE_BASE_CONFIDENCE + 4.0;
        // deeper wick and some stabilization increase confidence
        confidence += ((total_drop - 12.0) / 30.0).clamp(0.0, 1.0) * 8.0;
        if velocity.abs() < 6.0 {
            confidence += 6.0;
        }
        if velocity < -15.0 {
            confidence -= 8.0;
        }
        confidence = confidence.max(30.0).min(93.0);

        return Some(DropAnalysis {
            drop_percent: total_drop,
            drop_style: DropStyle::WickRebound,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: WICK_WINDOW_SEC,
            reasoning: format!(
                "Wick dip/rebound -{:.1}% in {}s with recovery",
                total_drop,
                WICK_WINDOW_SEC
            ),
        });
    }

    None
}

fn detect_cascade_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let window: Vec<(DateTime<Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= CASCADE_WINDOW_SEC)
        .cloned()
        .collect();
    if window.len() < 5 {
        return None;
    }
    let prices: Vec<f64> = window
        .iter()
        .map(|(_, p)| *p)
        .collect();
    let window_high = prices.iter().fold(0.0f64, |a, b| a.max(*b));
    if window_high <= 0.0 || !window_high.is_finite() {
        return None;
    }

    // Count stair-step lower-high/lower-low sequences
    let mut steps = 0usize;
    let mut last = prices[0];
    for p in prices.iter().skip(1) {
        if *p < last {
            steps += 1;
        }
        last = *p;
    }
    let total_drop = ((window_high - current_price) / window_high) * 100.0;
    if steps >= 4 && total_drop >= 15.0 && total_drop <= 45.0 {
        let velocity = calculate_velocity(&prices, CASCADE_WINDOW_SEC);
        let mut confidence = MODERATE_BASE_CONFIDENCE + 2.0;
        confidence += (((steps as f64) - 4.0) * 1.5).min(8.0);
        if velocity.abs() < 8.0 {
            confidence += 4.0;
        }
        confidence = confidence.max(28.0).min(90.0);
        return Some(DropAnalysis {
            drop_percent: total_drop,
            drop_style: DropStyle::Cascade,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: CASCADE_WINDOW_SEC,
            reasoning: format!(
                "Cascade drop -{:.1}% over {}s ({} steps)",
                total_drop,
                CASCADE_WINDOW_SEC,
                steps
            ),
        });
    }
    None
}

fn detect_median_dip(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    now: DateTime<Utc>
) -> Option<DropAnalysis> {
    let mut window_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= MEDIAN_DIP_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();
    if window_prices.len() < 3 {
        return None;
    }
    window_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if window_prices.len() % 2 == 1 {
        window_prices[window_prices.len() / 2]
    } else {
        let mid = window_prices.len() / 2;
        (window_prices[mid - 1] + window_prices[mid]) / 2.0
    };
    if median <= 0.0 || !median.is_finite() {
        return None;
    }
    let dip_percent = ((median - current_price) / median) * 100.0;
    if dip_percent >= 6.0 && dip_percent <= 25.0 {
        // short-term overshoot below typical
        let velocity = calculate_velocity(&window_prices, MEDIAN_DIP_WINDOW_SEC);
        let mut confidence = FLASH_BASE_CONFIDENCE - 5.0; // a bit lower base than flash
        confidence += ((dip_percent - 6.0) / 19.0).clamp(0.0, 1.0) * 6.0;
        if velocity.abs() < 8.0 {
            confidence += 5.0;
        }
        confidence = confidence.max(22.0).min(88.0);
        return Some(DropAnalysis {
            drop_percent: dip_percent,
            drop_style: DropStyle::MedianDip,
            confidence,
            velocity_per_minute: velocity,
            time_window_used: MEDIAN_DIP_WINDOW_SEC,
            reasoning: format!(
                "Median-dip -{:.1}% vs {}s median",
                dip_percent,
                MEDIAN_DIP_WINDOW_SEC
            ),
        });
    }
    None
}

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

fn calculate_flash_confidence(drop_percent: f64, velocity: f64) -> f64 {
    let mut confidence = FLASH_BASE_CONFIDENCE;

    // Adjust based on drop magnitude
    let drop_factor = (drop_percent - FLASH_DROP_MIN) / (FLASH_DROP_MAX - FLASH_DROP_MIN);
    confidence += drop_factor * 10.0; // Up to +10 for larger drops

    // Adjust based on velocity (negative velocity = downward) - MADE MORE FORGIVING
    if velocity < -20.0 {
        confidence += 10.0; // High downward velocity is good
    } else if velocity > 25.0 {
        // Changed from 10.0 to 25.0 - allow more recovery
        confidence -= 8.0; // Reduced penalty from 15.0 to 8.0
    }

    confidence.max(20.0).min(95.0)
}

fn calculate_moderate_confidence(drop_percent: f64, velocity: f64) -> f64 {
    let mut confidence = MODERATE_BASE_CONFIDENCE;

    // Adjust based on drop magnitude
    let drop_factor = (drop_percent - MODERATE_DROP_MIN) / (MODERATE_DROP_MAX - MODERATE_DROP_MIN);
    confidence += drop_factor * 8.0; // Up to +8 for larger drops

    // Adjust based on velocity
    if velocity < -10.0 {
        confidence += 8.0; // Strong downward trend
    } else if velocity > 5.0 {
        confidence -= 12.0; // Recovery during drop reduces confidence
    }

    confidence.max(25.0).min(92.0)
}

fn calculate_deep_confidence(drop_percent: f64, velocity: f64) -> f64 {
    let mut confidence = DEEP_BASE_CONFIDENCE;

    // Deep drops are inherently more confident opportunities
    let drop_factor = (drop_percent - DEEP_DROP_MIN) / (DEEP_DROP_MAX - DEEP_DROP_MIN);
    confidence += drop_factor * 6.0; // Up to +6 for deeper drops

    // For deep drops, we want stabilization (lower velocity)
    if velocity.abs() < 5.0 {
        confidence += 5.0; // Stabilizing price is good for deep drops
    } else if velocity < -15.0 {
        confidence -= 8.0; // Still falling fast might not be bottom
    }

    confidence.max(30.0).min(90.0)
}

fn calculate_extreme_confidence(drop_percent: f64, velocity: f64) -> f64 {
    let mut confidence = EXTREME_BASE_CONFIDENCE;

    // Extreme drops need careful analysis
    let drop_factor = (drop_percent - EXTREME_DROP_MIN) / (EXTREME_DROP_MAX - EXTREME_DROP_MIN);
    confidence += drop_factor * 5.0; // Up to +5 for more extreme drops

    // For extreme drops, we strongly prefer stabilization
    if velocity.abs() < 3.0 {
        confidence += 8.0; // Very stable price after extreme drop
    } else if velocity < -10.0 {
        confidence -= 15.0; // Still crashing heavily, dangerous
    } else if velocity > 8.0 {
        confidence += 3.0; // Some recovery can be good sign
    }

    confidence.max(35.0).min(95.0)
}

// =============================================================================
// CONFIDENCE ENHANCEMENT WITH CONTEXT
// =============================================================================

fn enhance_confidence_with_context(mut analysis: DropAnalysis, liquidity_usd: f64) -> DropAnalysis {
    // Adjust confidence based on liquidity context
    if liquidity_usd > 1_000_000.0 {
        // Higher liquidity tokens are more stable/predictable
        analysis.confidence += 5.0;
        analysis.reasoning += " (high liquidity)";
    } else if liquidity_usd < 10_000.0 {
        // Lower liquidity can be more volatile but riskier
        analysis.confidence -= 3.0;
        analysis.reasoning += " (low liquidity)";
    }

    // Ensure confidence stays in bounds
    analysis.confidence = analysis.confidence.max(0.0).min(100.0);

    analysis
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn calculate_velocity(prices: &[f64], window_seconds: i64) -> f64 {
    if prices.len() < 2 {
        return 0.0;
    }

    // Note: prices array comes from filtering by timestamp, so it should be chronologically ordered
    // But let's be safe and use first/last by design
    let first = prices[0];
    let last = prices[prices.len() - 1];

    if first <= 0.0 || !first.is_finite() || !last.is_finite() {
        return 0.0;
    }

    let percent_change = ((last - first) / first) * 100.0;
    let minutes = (window_seconds as f64) / 60.0;

    if minutes <= 0.0 {
        return 0.0;
    }

    // Add debug logging to see what's happening
    if crate::global::is_debug_entry_enabled() {
        crate::logger::log(
            crate::logger::LogTag::Entry,
            "VELOCITY_CALC",
            &format!(
                "Velocity calc: first={:.6}, last={:.6}, change={:.2}%/min over {:.1}min",
                first,
                last,
                percent_change / minutes,
                minutes
            )
        );
    }

    percent_change / minutes // Percent per minute
}

fn should_enter_based_on_analysis(analysis: &DropAnalysis, liquidity_usd: f64) -> bool {
    // Base confidence threshold varies by drop style - FURTHER LOWERED for more aggressive entry
    let min_confidence = match analysis.drop_style {
        DropStyle::UltraFlash => 30.0, // Very quick moves (reduced from 35.0)
        DropStyle::Flash => 28.0, // Flash drops (reduced from 32.0)
        DropStyle::WickRebound => 26.0, // Wick rebounds (reduced from 30.0)
        DropStyle::Moderate => 25.0, // Moderate drops (reduced from 30.0)
        DropStyle::Cascade => 24.0, // Cascade drops (reduced from 28.0)
        DropStyle::Deep => 22.0, // Deep drops (reduced from 25.0)
        DropStyle::Extreme => 20.0, // Extreme drops (reduced from 22.0)
        DropStyle::MedianDip => 25.0, // Median dips (reduced from 30.0)
    };

    // Adjust threshold based on liquidity
    let adjusted_threshold = if liquidity_usd > 100_000.0 {
        min_confidence - 5.0 // Lower threshold for higher liquidity (safer)
    } else if liquidity_usd < 5_000.0 {
        min_confidence + 10.0 // Higher threshold for low liquidity (riskier)
    } else {
        min_confidence
    };

    analysis.confidence >= adjusted_threshold
}

async fn get_current_pool_data(token: &Token) -> Option<(f64, i64, f64)> {
    match crate::tokens::get_price(&token.mint, Some(PriceOptions::pool_only()), false).await {
        Some(price_result) => {
            match price_result.best_sol_price() {
                Some(price) if price > 0.0 && price.is_finite() => {
                    let data_age_minutes =
                        (Utc::now() - price_result.calculated_at).num_seconds() / 60;

                    if data_age_minutes > MAX_DATA_AGE_MIN {
                        return None;
                    }

                    let liquidity = price_result.liquidity_usd.unwrap_or_else(|| {
                        token.liquidity
                            .as_ref()
                            .and_then(|l| l.usd)
                            .unwrap_or(0.0)
                    });

                    Some((price, data_age_minutes, liquidity))
                }
                _ => None,
            }
        }
        None => None,
    }
}

async fn is_near_ath(mint: &str, current_price: f64) -> bool {
    // Check OHLCV data for 2-hour ATH protection
    match crate::tokens::ohlcvs::get_latest_ohlcv(mint, 120).await {
        Ok(ohlcv_data) => {
            let recent_high = ohlcv_data
                .iter()
                .map(|point| point.high)
                .fold(0.0f64, |a, b| a.max(b));

            if recent_high > 0.0 && recent_high.is_finite() {
                let drop_from_high = ((recent_high - current_price) / recent_high) * 100.0;
                return drop_from_high < ATH_MIN_DISTANCE_PERCENT;
            }
        }
        _ => {}
    }

    false // If no OHLCV data, don't block the entry
}

// Returns percent below recent ATH over last 120 minutes if available
async fn get_ath_distance_percent(mint: &str, current_price: f64) -> Option<f64> {
    match crate::tokens::ohlcvs::get_latest_ohlcv(mint, 120).await {
        Ok(ohlcv_data) => {
            let recent_high = ohlcv_data
                .iter()
                .map(|point| point.high)
                .fold(0.0f64, |a, b| a.max(b));
            if recent_high > 0.0 && recent_high.is_finite() {
                let drop_from_high = ((recent_high - current_price) / recent_high) * 100.0;
                return Some(drop_from_high);
            }
            None
        }
        _ => None,
    }
}

// =============================================================================
// PUMP.FUN SPECIAL ENTRY DETECTION
// =============================================================================

/// Check if token is a pump.fun token in the optimal SOL reserve range (23-26 SOL)
/// Returns (is_pumpfun, sol_reserves, in_optimal_range)
async fn check_pumpfun_easy_entry(token: &Token) -> Option<(bool, f64, bool)> {
    let pool_service = get_pool_service();

    // Check if this is a pump.fun token by DEX ID or pool program
    let is_pumpfun_token = if let Some(ref dex_id) = token.dex_id {
        dex_id.to_lowercase().contains("pump") || dex_id == "pumpfun" || dex_id == "pumpswap"
    } else {
        false
    };

    if !is_pumpfun_token {
        // Also check if we have pool data indicating pump.fun
        if
            let Some(pool_result) = pool_service.get_pool_price(
                &token.mint,
                None,
                &PriceOptions::default()
            ).await
        {
            if let Some(pool_type) = &pool_result.pool_type {
                if pool_type.to_uppercase().contains("PUMP") {
                    return check_sol_reserves(&token.mint, pool_service).await;
                }
            }
        }
        return None;
    }

    // It's a pump.fun token, now check SOL reserves
    check_sol_reserves(&token.mint, pool_service).await
}

/// Check if a pool result represents a pump.fun token
fn is_pump_fun_pool(pool_result: &crate::tokens::pool::PoolPriceResult) -> bool {
    // Check if the pool type indicates pump.fun
    if let Some(ref pool_type) = pool_result.pool_type {
        return pool_type.to_lowercase().contains("pump");
    }

    // Check if dex_id indicates pump.fun
    pool_result.dex_id.to_lowercase().contains("pump")
}

/// Check SOL reserves for a pump.fun token
async fn check_sol_reserves(
    mint: &str,
    pool_service: &crate::tokens::pool::PoolPriceService
) -> Option<(bool, f64, bool)> {
    // Try to get direct pool price calculation which includes reserve information
    match pool_service.get_pool_price_direct(&format!("{}pool", mint), mint, None).await {
        Some(pool_result) => {
            // Check if we have direct SOL reserve information from the new fields
            if let Some(sol_reserve) = pool_result.sol_reserve {
                let is_pump_fun = is_pump_fun_pool(&pool_result);
                let in_optimal_range = sol_reserve >= 23.0 && sol_reserve <= 26.0;
                return Some((is_pump_fun, sol_reserve, in_optimal_range));
            }

            // Fallback: For pump.fun, we need to check if we have SOL liquidity data
            if pool_result.liquidity_usd > 0.0 {
                // Estimate SOL reserves from liquidity (rough approximation)
                // Pump.fun pools typically have SOL as quote token, so roughly half the liquidity is SOL
                let estimated_sol_reserves = if pool_result.liquidity_usd > 0.0 {
                    // Assume SOL price ~$150 and roughly half liquidity is SOL
                    (pool_result.liquidity_usd * 0.5) / 150.0
                } else {
                    0.0
                };

                let in_optimal_range =
                    estimated_sol_reserves >= PUMPFUN_SOL_RESERVE_MIN &&
                    estimated_sol_reserves <= PUMPFUN_SOL_RESERVE_MAX;
                return Some((true, estimated_sol_reserves, in_optimal_range));
            }
        }
        None => {}
    }

    // Fallback: Try to get pool info and extract reserves from there
    match pool_service.get_pool_price(mint, None, &PriceOptions::default()).await {
        Some(pool_result) => {
            // Use liquidity to estimate SOL reserves if available
            if pool_result.liquidity_usd > 0.0 {
                // Rough estimation: pump.fun pools typically hold 20-30 SOL at optimal trading times
                // We can estimate from liquidity USD
                let estimated_sol_reserves = (pool_result.liquidity_usd * 0.4) / 150.0; // Assume SOL ~$150
                let in_optimal_range =
                    estimated_sol_reserves >= PUMPFUN_SOL_RESERVE_MIN &&
                    estimated_sol_reserves <= PUMPFUN_SOL_RESERVE_MAX;

                Some((true, estimated_sol_reserves, in_optimal_range))
            } else {
                Some((true, 0.0, false))
            }
        }
        None => {
            // If we can't get pool data but we know it's pump.fun, still return that info
            Some((true, 0.0, false))
        }
    }
}

// =============================================================================
// PROFIT TARGET CALCULATION
// =============================================================================

/// Calculate profit targets based on drop analysis and liquidity
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    // Pull current pool data first (price + liquidity)
    let (current_price_opt, liquidity_usd) = match get_current_pool_data(token).await {
        Some((price, _age_min, liquidity)) => (Some(price), liquidity),
        None =>
            (
                None,
                token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(10_000.0), // Default fallback
            ),
    };

    // Check for pump.fun easy entry opportunity
    let pumpfun_easy_entry = check_pumpfun_easy_entry(token).await;

    // Base profit targets (refined buckets)
    // Keep simple: just a few more tiers tuned by liquidity.
    let (mut min_profit, mut max_profit): (f64, f64) = if liquidity_usd < 2_500.0 {
        (35.0, 140.0) // very micro caps
    } else if liquidity_usd < 10_000.0 {
        (28.0, 110.0) // micro/small caps
    } else if liquidity_usd < 50_000.0 {
        (20.0, 85.0) // small caps
    } else if liquidity_usd < 250_000.0 {
        (16.0, 70.0) // lower mid caps
    } else if liquidity_usd < 1_000_000.0 {
        (12.0, 55.0) // mid caps
    } else {
        (9.0, 40.0) // large caps
    };

    // Apply pump.fun easy entry adjustments
    if let Some((is_pumpfun, sol_reserves, in_range)) = pumpfun_easy_entry {
        if is_pumpfun && in_range {
            // Pump.fun tokens in optimal range tend to have quick, predictable bounces
            min_profit *= 1.15; // 15% higher min target for quicker exits
            max_profit *= 1.25; // 25% higher max target due to volatility

            // Add additional boost based on how close to center of range
            let range_center = (PUMPFUN_SOL_RESERVE_MIN + PUMPFUN_SOL_RESERVE_MAX) / 2.0;
            let distance_from_center = (sol_reserves - range_center).abs();
            let max_distance = (PUMPFUN_SOL_RESERVE_MAX - PUMPFUN_SOL_RESERVE_MIN) / 2.0;
            let proximity_factor = 1.0 - (distance_from_center / max_distance).min(1.0);

            // Closer to 24.5 SOL gets additional boost
            let proximity_boost = proximity_factor * 0.1; // Up to 10% boost
            min_profit *= 1.0 + proximity_boost;
            max_profit *= 1.0 + proximity_boost * 1.5;
        }
    }

    // Enhance with dynamic, pool-price-based signals if we have data
    let pool_service = get_pool_service();
    let price_history = pool_service.get_recent_price_history(&token.mint).await;
    if let Some(current_price) = current_price_opt {
        if price_history.len() >= 3 {
            let now = Utc::now();
            // 60s volatility via high-low range
            let prices_60: Vec<f64> = price_history
                .iter()
                .filter(|(ts, _)| (now - *ts).num_seconds() <= 60)
                .map(|(_, p)| *p)
                .collect();

            if prices_60.len() >= 3 {
                let high_60 = prices_60.iter().fold(0.0f64, |a, b| a.max(*b));
                let low_60 = prices_60.iter().fold(f64::INFINITY, |a, b| a.min(*b));
                if high_60.is_finite() && low_60.is_finite() && high_60 > 0.0 && low_60 > 0.0 {
                    let hl_range_60 = ((high_60 - low_60) / high_60) * 100.0;
                    // Scale targets with volatility (larger range => larger targets)
                    let min_scale = (hl_range_60 / 80.0).clamp(0.0, 0.6); // up to +60%
                    let max_scale = (hl_range_60 / 60.0).clamp(0.0, 0.8); // up to +80%
                    min_profit *= 1.0 + min_scale;
                    max_profit *= 1.0 + max_scale;
                }
            }

            // 10s velocity — quick momentum context
            let prices_10: Vec<f64> = price_history
                .iter()
                .filter(|(ts, _)| (now - *ts).num_seconds() <= 10)
                .map(|(_, p)| *p)
                .collect();
            if prices_10.len() >= 2 {
                let vel10 = calculate_velocity(&prices_10, 10);
                if vel10 < -20.0 {
                    min_profit += 3.0;
                    max_profit += 8.0;
                } else if vel10 < -8.0 {
                    min_profit += 1.5;
                    max_profit += 5.0;
                } else if vel10 > 18.0 {
                    min_profit -= 3.0;
                    max_profit -= 6.0;
                } else if vel10 > 8.0 {
                    min_profit -= 1.5;
                    max_profit -= 3.5;
                }
            }

            // Detected drop style => tailor targets
            if
                let Some(da) = analyze_drop_patterns(
                    &price_history,
                    current_price,
                    liquidity_usd
                ).await
            {
                let s = (da.confidence / 100.0).clamp(0.4, 1.0);
                match da.drop_style {
                    DropStyle::UltraFlash | DropStyle::Flash | DropStyle::WickRebound => {
                        // Expect fast bounce — raise near-term target
                        let bump = (da.drop_percent * 0.35 * s).clamp(2.0, 15.0);
                        min_profit += bump;
                        max_profit = max_profit.max(min_profit + bump * 1.8);
                    }
                    DropStyle::Moderate => {
                        let bump = (da.drop_percent * 0.25 * s).clamp(1.0, 10.0);
                        min_profit += bump;
                        max_profit = max_profit.max(min_profit + bump * 1.5);
                    }
                    DropStyle::Cascade => {
                        let bump = (da.drop_percent * 0.2 * s).clamp(1.0, 8.0);
                        min_profit += bump;
                        max_profit = max_profit.max(min_profit + bump * 1.4);
                    }
                    DropStyle::MedianDip => {
                        let bump = (da.drop_percent * 0.22 * s).clamp(1.0, 9.0);
                        min_profit += bump;
                        max_profit = max_profit.max(min_profit + bump * 1.5);
                    }
                    DropStyle::Deep => {
                        min_profit -= 2.0 * s; // allow tighter first scale-out
                        max_profit += (da.drop_percent * 0.5 * s).clamp(8.0, 40.0);
                    }
                    DropStyle::Extreme => {
                        min_profit -= 3.0 * s;
                        max_profit += (da.drop_percent * 0.7 * s).clamp(12.0, 60.0);
                    }
                }
            }
        }
    }

    // Liquidity risk adjustments and caps
    if liquidity_usd < 5_000.0 {
        min_profit += 2.0;
        max_profit = max_profit.min(120.0);
        if liquidity_usd < 2_500.0 {
            min_profit += 2.0;
            max_profit = max_profit.min(110.0);
        }
    } else if liquidity_usd > 1_000_000.0 {
        // Large caps: require more conservative expectations
        min_profit = (min_profit * 0.9).max(8.0);
        max_profit = (max_profit * 0.9).min(120.0);
    }

    // Ensure proportional spread: at least 60% of min or 12%, whichever larger
    let min_spread = (min_profit * 0.6).max(12.0);
    if max_profit - min_profit < min_spread {
        max_profit = min_profit + min_spread;
    }

    // Clamp bounds to sane global limits
    min_profit = min_profit.clamp(6.0, 45.0);
    max_profit = max_profit.clamp(24.0, 180.0);

    // Final ordering safety: keep at least 10% gap post-clamp
    if max_profit - min_profit < 10.0 {
        max_profit = (min_profit + 10.0).min(180.0);
    }

    (min_profit, max_profit)
}
