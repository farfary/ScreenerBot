/// Simple Scalping Drop Detector (fast and focused)
///
/// Replaces the previous complex multi-style system with a lightweight detector
/// optimized for fast scalping entries on sharp drops.
///
/// Goals:
/// - Detect quick drops between 10% and 50%
/// - Use only pool price history (no OHLCV/ATH/pump.fun specifics)
/// - Keep logic simple, fast, and easy to reason about
/// - Provide concise debug logs

use crate::global::is_debug_entry_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::{ get_pool_service, Token, PriceOptions };
use chrono::{ DateTime, Utc };

// =============================================================================
// CORE CONFIGURATION PARAMETERS
// =============================================================================

// Simple scalping config - more permissive
const MIN_PRICE_POINTS: usize = 2; // Keep at 2 for safety
const MAX_DATA_AGE_MIN: i64 = 15; // Increased from 10 to 15 minutes

// Liquidity filter (very permissive for scalping)
const MIN_LIQUIDITY_USD: f64 = 50.0;
const MAX_LIQUIDITY_USD: f64 = 100_000_000.0;

// Detection windows and thresholds - relaxed for more entries
const WINDOWS_SEC: [i64; 6] = [5, 10, 30, 60, 120, 300]; // Added 5-minute window
const MIN_DROP_PERCENT: f64 = 5.0; // Reduced from 10% to 5%
const MAX_DROP_PERCENT: f64 = 70.0; // Increased from 50% to 70%

// =============================================================================
// SIMPLE DROP SIGNAL
// =============================================================================

#[derive(Debug, Clone)]
struct SimpleDropSignal {
    window_sec: i64,
    drop_percent: f64,
    window_high: f64,
    current_price: f64,
    samples: usize,
    velocity_per_minute: f64,
}

// =============================================================================
// MAIN ENTRY FUNCTION
// =============================================================================

/// Main entry point for determining if a token should be bought
/// Returns (approved_for_entry, confidence_score, reason)
pub async fn should_buy(token: &Token) -> (bool, f64, String) {
    // Immediate debug log to ensure we're getting called
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "SHOULD_BUY_START",
            &format!("🔍 Starting entry analysis for {}", token.symbol)
        );
    }

    let pool_service = get_pool_service();

    // Get current pool price and liquidity first
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "POOL_PRICE_REQUEST",
            &format!("📊 Getting pool price for {}", token.symbol)
        );
    }

    let (current_price, liquidity_usd) = match
        crate::tokens::get_price(&token.mint, Some(PriceOptions::default()), false).await
    {
        Some(result) => {
            let price = result.sol_price().unwrap_or(0.0);
            let liquidity = result.liquidity_usd.unwrap_or(0.0);
            if price <= 0.0 || !price.is_finite() {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        "INVALID_PRICE",
                        &format!("❌ {} invalid price: {}", token.symbol, price)
                    );
                }
                return (false, 10.0, "Invalid price data".to_string());
            }
            (price, liquidity)
        }
        None => {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "NO_POOL_DATA", &format!("❌ {} no pool data", token.symbol));
            }
            return (false, 5.0, "No valid pool data".to_string());
        }
    };
    // Basic liquidity filter (lightweight)
    if liquidity_usd < MIN_LIQUIDITY_USD || liquidity_usd > MAX_LIQUIDITY_USD {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "LIQUIDITY_FILTER",
                &format!(
                    "❌ {} liquidity ${:.0} outside bounds {}-{:.0}",
                    token.symbol,
                    liquidity_usd,
                    MIN_LIQUIDITY_USD as i64,
                    MAX_LIQUIDITY_USD
                )
            );
        }
        return (
            false,
            10.0,
            format!(
                "Liquidity out of bounds: ${:.0} (allowed {}..{:.0})",
                liquidity_usd,
                MIN_LIQUIDITY_USD as i64,
                MAX_LIQUIDITY_USD
            ),
        );
    }

    // Get recent pool price history
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "HISTORY_REQUEST",
            &format!("📈 Getting price history for {}", token.symbol)
        );
    }

    let mut price_history = pool_service.get_recent_price_history(&token.mint).await;
    // Filter out invalid prices (0 or non-finite)
    price_history.retain(|(_, p)| *p > 0.0 && p.is_finite());

    // Proactively refresh once if history is insufficient, then re-fetch
    if price_history.len() < MIN_PRICE_POINTS {
        // Force a fresh pool-only price to seed history
        let _ = crate::tokens::get_price(&token.mint, Some(PriceOptions::default()), false).await;
        let mut refreshed = pool_service.get_recent_price_history(&token.mint).await;
        refreshed.retain(|(_, p)| *p > 0.0 && p.is_finite());
        if refreshed.len() >= price_history.len() {
            price_history = refreshed;
        }
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "HISTORY_REFRESHED",
                &format!(
                    "{} refreshed history size={} (needed >= {})",
                    token.symbol,
                    price_history.len(),
                    MIN_PRICE_POINTS
                )
            );
        }
    }

    if price_history.len() < MIN_PRICE_POINTS {
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

        // Fallback: If we have at least 1 price point, still attempt basic evaluation
        if price_history.len() >= 1 && current_price > 0.0 {
            let recent_price = price_history[0].1;
            if recent_price > 0.0 && recent_price.is_finite() {
                let instant_drop = ((recent_price - current_price) / recent_price) * 100.0;
                if instant_drop >= 3.0 && instant_drop <= 70.0 {
                    // Basic confidence for single-point drops
                    let confidence = (25.0 + instant_drop * 0.8).min(60.0);
                    if confidence >= 28.0 {
                        if is_debug_entry_enabled() {
                            log(
                                LogTag::Entry,
                                "INSTANT_DROP_FALLBACK",
                                &format!(
                                    "🎯 {} instant drop -{:.1}% → conf {:.0}% → APPROVE",
                                    token.symbol,
                                    instant_drop,
                                    confidence
                                )
                            );
                        }
                        return (true, confidence, format!("Instant drop -{:.1}%", instant_drop));
                    }
                }
            }
        }

        return (
            false,
            12.0,
            format!("Insufficient price history: {} < {}", price_history.len(), MIN_PRICE_POINTS),
        );
    }

    // Detect best drop across windows
    let best = detect_best_drop(&price_history, current_price);

    if let Some(sig) = best {
        // Confidence: more aggressive base + magnitude + window bonus + velocity tweak + liquidity tweak
        let mut confidence = 30.0; // Reduced from 35.0
        confidence +=
            ((sig.drop_percent - MIN_DROP_PERCENT) / (MAX_DROP_PERCENT - MIN_DROP_PERCENT)).clamp(
                0.0,
                1.0
            ) * 50.0; // Increased from 45 to 50
        confidence += match sig.window_sec {
            5 => 18.0, // Increased bonuses
            10 => 15.0,
            30 => 12.0, // Increased from 8
            60 => 8.0, // Increased from 5
            120 => 5.0, // Increased from 2
            300 => 3.0, // New longer window
            _ => 2.0,
        };
        if sig.velocity_per_minute < -20.0 {
            confidence += 8.0; // Increased from 6
        }
        if sig.velocity_per_minute > 15.0 {
            confidence -= 6.0; // Reduced penalty from 8
        }
        if liquidity_usd < 5_000.0 {
            confidence -= 3.0; // Reduced penalty from 5
        } else if liquidity_usd > 100_000.0 {
            confidence += 5.0; // Increased bonus from 3
        }
        confidence = confidence.clamp(0.0, 95.0);

        // Entry decision with more permissive threshold
        let approved = confidence >= 28.0; // Reduced from 32.0%

        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "SIMPLE_DROP_DETECTED",
                &format!(
                    "🎯 {} drop -{:.1}% over {}s (samples: {}, vel: {:.1}%/min) → conf {:.0}% → {}",
                    token.symbol,
                    sig.drop_percent,
                    sig.window_sec,
                    sig.samples,
                    sig.velocity_per_minute,
                    confidence,
                    if approved {
                        "APPROVE"
                    } else {
                        "REJECT"
                    }
                )
            );
        }

        return (
            approved,
            confidence,
            if approved {
                format!("Scalp drop -{:.1}% over {}s", sig.drop_percent, sig.window_sec)
            } else {
                format!(
                    "Drop -{:.1}% over {}s but confidence {:.0}% < 28%",
                    sig.drop_percent,
                    sig.window_sec,
                    confidence
                )
            },
        );
    } else {
        if is_debug_entry_enabled() {
            // Log last few prices to understand zeros
            let recent_prices: Vec<String> = price_history
                .iter()
                .take(5)
                .map(|(ts, price)| format!("{:.6}@{}", price, ts.format("%H:%M:%S")))
                .collect();
            log(
                LogTag::Entry,
                "NO_DROP_PATTERN",
                &format!(
                    "❌ {} no drop 5-70% detected in {} points | recent: [{}]",
                    token.symbol,
                    price_history.len(),
                    recent_prices.join(", ")
                )
            );
        }
        return (false, 20.0, "No 5-70% drop detected".to_string());
    }
}

// Removed complex multi-style detectors and confidence systems

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

    let velocity_per_minute = percent_change / minutes;

    // Add debug logging to see what's happening - only for very large velocity changes
    if crate::global::is_debug_entry_enabled() && velocity_per_minute.abs() > 50.0 {
        crate::logger::log(
            crate::logger::LogTag::Entry,
            "VELOCITY_CALC",
            &format!(
                "Velocity calc: first={:.9}, last={:.9}, change={:.2}%/min over {:.1}min",
                first,
                last,
                velocity_per_minute,
                minutes
            )
        );
    }

    velocity_per_minute // Percent per minute
}

// Simple best-drop detector over predefined windows
fn detect_best_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64
) -> Option<SimpleDropSignal> {
    let now = Utc::now();
    let mut best: Option<SimpleDropSignal> = None;
    for &w in WINDOWS_SEC.iter() {
        // Prices in window
        let mut prices: Vec<f64> = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= w)
            .map(|(_, p)| *p)
            .collect();
        if prices.len() < 2 {
            continue;
        }
        // Use high in window to compute drop magnitude to current
        let window_high = prices.iter().fold(0.0f64, |a, b| a.max(*b));
        if window_high <= 0.0 || !window_high.is_finite() {
            continue;
        }
        let drop_percent = ((window_high - current_price) / window_high) * 100.0;
        if drop_percent < MIN_DROP_PERCENT || drop_percent > MAX_DROP_PERCENT {
            continue;
        }

        // Velocity based on first/last within window
        let velocity = calculate_velocity(&prices, w);
        let cand = SimpleDropSignal {
            window_sec: w,
            drop_percent,
            window_high,
            current_price,
            samples: prices.len(),
            velocity_per_minute: velocity,
        };
        // Prefer larger drops; tie-breaker: shorter window; then stronger negative velocity
        let better = match &best {
            None => true,
            Some(b) => {
                if cand.drop_percent > b.drop_percent + 1e-6 {
                    true
                } else if (cand.drop_percent - b.drop_percent).abs() <= 1e-6 {
                    if cand.window_sec < b.window_sec {
                        true
                    } else {
                        cand.velocity_per_minute < b.velocity_per_minute
                    }
                } else {
                    false
                }
            }
        };
        if better {
            best = Some(cand);
        }
    }
    best
}

async fn get_current_pool_data(token: &Token) -> Option<(f64, i64, f64)> {
    match crate::tokens::get_price(&token.mint, Some(PriceOptions::default()), false).await {
        Some(price_result) => {
            match price_result.sol_price() {
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

// Remove ATH/OHLCV dependency for fast scalping flow

// =============================================================================
// PUMP.FUN SPECIAL ENTRY DETECTION
// =============================================================================

// Remove pump.fun specific logic entirely

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
                    .unwrap_or(10_000.0),
            ),
    };

    // Base profit targets by liquidity only (simple tiers)
    let (mut min_profit, mut max_profit): (f64, f64) = if liquidity_usd < 2_500.0 {
        (30.0, 120.0)
    } else if liquidity_usd < 10_000.0 {
        (24.0, 100.0)
    } else if liquidity_usd < 50_000.0 {
        (18.0, 80.0)
    } else if liquidity_usd < 250_000.0 {
        (14.0, 65.0)
    } else if liquidity_usd < 1_000_000.0 {
        (10.0, 50.0)
    } else {
        (8.0, 38.0)
    };

    // Volatility-based adjustment using recent window
    let pool_service = get_pool_service();
    let price_history = pool_service.get_recent_price_history(&token.mint).await;
    if current_price_opt.is_some() && price_history.len() >= 3 {
        let now = Utc::now();
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
                let scale = (hl_range_60 / 60.0).clamp(0.0, 0.8);
                min_profit *= 1.0 + scale * 0.6; // up to +48%
                max_profit *= 1.0 + scale * 0.8; // up to +64%
            }
        }
    }

    // Clamp and ensure spread
    min_profit = min_profit.clamp(6.0, 45.0);
    max_profit = max_profit.clamp(24.0, 180.0);
    if max_profit - min_profit < 10.0 {
        max_profit = (min_profit + 10.0).min(180.0);
    }

    (min_profit, max_profit)
}
