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
use chrono::{ DateTime, Utc };

// =============================================================================
// CORE CONFIGURATION PARAMETERS
// =============================================================================

// Drop Detection Thresholds
const FLASH_DROP_MIN: f64 = 5.0; // Minimum flash drop %
const FLASH_DROP_MAX: f64 = 15.0; // Maximum flash drop %
const MODERATE_DROP_MIN: f64 = 15.0; // Minimum moderate drop %
const MODERATE_DROP_MAX: f64 = 35.0; // Maximum moderate drop %
const DEEP_DROP_MIN: f64 = 35.0; // Minimum deep drop %
const DEEP_DROP_MAX: f64 = 60.0; // Maximum deep drop %
const EXTREME_DROP_MIN: f64 = 60.0; // Minimum extreme drop %
const EXTREME_DROP_MAX: f64 = 100.0; // Maximum extreme drop %

// Time Windows for Analysis
const FLASH_WINDOW_SEC: i64 = 10; // Flash drop detection window
const MODERATE_WINDOW_SEC: i64 = 60; // Moderate drop detection window
const DEEP_WINDOW_SEC: i64 = 300; // Deep drop detection window (5 min)
const EXTREME_WINDOW_SEC: i64 = 900; // Extreme drop detection window (15 min)

// ATH Protection Settings
const ATH_MIN_DISTANCE_PERCENT: f64 = 5.0; // Minimum 5% below ATH

// Confidence Scoring Base Values
const FLASH_BASE_CONFIDENCE: f64 = 70.0; // Base confidence for flash drops
const MODERATE_BASE_CONFIDENCE: f64 = 75.0; // Base confidence for moderate drops
const DEEP_BASE_CONFIDENCE: f64 = 80.0; // Base confidence for deep drops
const EXTREME_BASE_CONFIDENCE: f64 = 85.0; // Base confidence for extreme drops

// Price History Requirements
const MIN_PRICE_POINTS: usize = 3; // Minimum price points needed
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
}

impl DropStyle {
    fn to_string(&self) -> &'static str {
        match self {
            DropStyle::Flash => "FLASH",
            DropStyle::Moderate => "MODERATE",
            DropStyle::Deep => "DEEP",
            DropStyle::Extreme => "EXTREME",
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

    // Get price history for drop analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    // Update confidence based on available data
    if price_history.len() < MIN_PRICE_POINTS {
        confidence = confidence.min(25.0); // Cap confidence if insufficient history
        return (
            false,
            confidence,
            format!("Insufficient price history: {} < {}", price_history.len(), MIN_PRICE_POINTS),
        );
    }

    // Check ATH protection using OHLCV data
    let ath_check = is_near_ath(&token.mint, current_price).await;
    if ath_check {
        confidence = confidence.min(15.0); // Very low confidence near ATH
        return (false, confidence, "Too close to ATH (2h protection)".to_string());
    }

    // Analyze drop patterns with confidence scoring
    let drop_analysis = analyze_drop_patterns(&price_history, current_price, liquidity_usd).await;

    if let Some(analysis) = drop_analysis {
        // Update confidence based on drop analysis
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

    // No significant drop detected - return base confidence
    let final_reason = if !liquidity_check.0 {
        format!("No significant drop pattern detected ({})", liquidity_check.1)
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

    // 1. Flash Drop Detection (10 seconds)
    if let Some(analysis) = detect_flash_drop(price_history, current_price, now) {
        return Some(enhance_confidence_with_context(analysis, liquidity_usd));
    }

    // 2. Moderate Drop Detection (1 minute)
    if let Some(analysis) = detect_moderate_drop(price_history, current_price, now) {
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

// =============================================================================
// CONFIDENCE CALCULATION
// =============================================================================

fn calculate_flash_confidence(drop_percent: f64, velocity: f64) -> f64 {
    let mut confidence = FLASH_BASE_CONFIDENCE;

    // Adjust based on drop magnitude
    let drop_factor = (drop_percent - FLASH_DROP_MIN) / (FLASH_DROP_MAX - FLASH_DROP_MIN);
    confidence += drop_factor * 10.0; // Up to +10 for larger drops

    // Adjust based on velocity (negative velocity = downward)
    if velocity < -20.0 {
        confidence += 10.0; // High downward velocity is good
    } else if velocity > 10.0 {
        confidence -= 15.0; // Upward velocity during drop is suspicious
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

    percent_change / minutes // Percent per minute
}

fn should_enter_based_on_analysis(analysis: &DropAnalysis, liquidity_usd: f64) -> bool {
    // Base confidence threshold varies by drop style
    let min_confidence = match analysis.drop_style {
        DropStyle::Flash => 60.0, // Need higher confidence for quick moves
        DropStyle::Moderate => 55.0, // Moderate confidence needed
        DropStyle::Deep => 50.0, // Lower threshold for deep drops
        DropStyle::Extreme => 45.0, // Lowest threshold for extreme opportunities
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

// =============================================================================
// PROFIT TARGET CALCULATION
// =============================================================================

/// Calculate profit targets based on drop analysis and liquidity
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    let liquidity_usd = match get_current_pool_data(token).await {
        Some((_, _, liquidity)) => liquidity,
        None => {
            token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(10_000.0) // Default fallback
        }
    };

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
