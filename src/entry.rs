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
const MIN_PRICE_POINTS: usize = 3; // Keep at 2 for safety
const MAX_DATA_AGE_MIN: i64 = 15; // Increased from 10 to 15 minutes

// Liquidity filter (optimized based on database analysis at $200/SOL)
const MIN_RESERVE_SOL: f64 = 10.0; // Minimum SOL reserves in pool (~$2,000, excludes bottom 5% of tokens)
const MAX_RESERVE_SOL: f64 = 5_000.0; // Maximum SOL reserves in pool (~$1M, focuses on liquid but not mega pools)

// Detection windows and thresholds - relaxed for more entries
const WINDOWS_SEC: [i64; 6] = [5, 10, 30, 60, 120, 300]; // Added 5-minute window
const MIN_DROP_PERCENT: f64 = 7.0; // Reduced from 10% to 5%
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

    let (current_price, reserve_sol, activity_score) = match
        crate::tokens::get_price(
            &token.mint,
            Some(PriceOptions { warm_on_miss: true, ..PriceOptions::default() }),
            false
        ).await
    {
        Some(result) => {
            let price = result.sol_price().unwrap_or(0.0);
            let reserve = result.reserve_sol.unwrap_or(0.0);

            // Calculate transaction activity score from token data
            let activity = token.txns
                .as_ref()
                .and_then(|txns| txns.m5.as_ref())
                .map(|m5| {
                    let total_5m = m5.buys.unwrap_or(0) + m5.sells.unwrap_or(0);
                    calculate_activity_score(total_5m as f64)
                })
                .unwrap_or(0.0);

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
            (price, reserve, activity)
        }
        None => {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "NO_POOL_DATA", &format!("❌ {} no pool data", token.symbol));
            }
            return (false, 5.0, "No valid pool data".to_string());
        }
    };
    // Basic liquidity filter using SOL reserves (lightweight)
    if reserve_sol < MIN_RESERVE_SOL || reserve_sol > MAX_RESERVE_SOL {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "LIQUIDITY_FILTER",
                &format!(
                    "❌ {} SOL reserves {:.2} outside bounds {:.1}-{:.0}",
                    token.symbol,
                    reserve_sol,
                    MIN_RESERVE_SOL,
                    MAX_RESERVE_SOL
                )
            );
        }
        return (
            false,
            10.0,
            format!(
                "SOL reserves out of bounds: {:.2} (allowed {:.1}..{:.0})",
                reserve_sol,
                MIN_RESERVE_SOL,
                MAX_RESERVE_SOL
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

    let mut price_history = pool_service.get_price_history(&token.mint).await;
    // Filter out invalid prices (0 or non-finite)
    price_history.retain(|(_, p)| *p > 0.0 && p.is_finite());

    // Proactively refresh once if history is insufficient, then re-fetch
    if price_history.len() < MIN_PRICE_POINTS {
        // Force a fresh pool-only price to seed history
        let _ = crate::tokens::get_price(&token.mint, Some(PriceOptions::default()), false).await;
        let mut refreshed = pool_service.get_price_history(&token.mint).await;
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
        // Enhanced confidence calculation based on database analysis
        let mut confidence = 25.0; // Reduced base from 30.0

        // Drop magnitude (non-linear curve favoring 8-15% sweet spot)
        let drop_score = calculate_drop_magnitude_score(sig.drop_percent);
        confidence += drop_score * 35.0; // Optimized from linear 50.0 scaling

        // Transaction activity (NEW - high impact factor)
        confidence += activity_score * 20.0; // Major addition based on 24.2% vs 3.5% success

        // Liquidity impact (significantly increased)
        let liquidity_score = calculate_liquidity_score(reserve_sol);
        confidence += liquidity_score * 15.0; // Increased from 5.0

        // Window preference (heavily favor quick drops)
        confidence += match sig.window_sec {
            5 => 25.0, // Increased from 18.0 (fast drops perform best)
            10 => 20.0, // Increased from 15.0
            30 => 12.0, // Keep existing
            60 => 6.0, // Reduced from 8.0
            120 => 3.0, // Reduced from 5.0
            300 => 1.0, // Reduced from 3.0
            _ => 1.0,
        };

        // Velocity adjustments (keep existing logic)
        if sig.velocity_per_minute < -20.0 {
            confidence += 8.0;
        }
        if sig.velocity_per_minute > 15.0 {
            confidence -= 6.0;
        }

        // Perfect storm multiplier (compound effect for ideal conditions)
        let is_perfect_storm =
            sig.drop_percent >= 7.0 &&
            sig.drop_percent <= 15.0 &&
            reserve_sol >= 250.0 &&
            reserve_sol <= 1000.0 &&
            activity_score >= 1.0; // >20 transactions
        if is_perfect_storm {
            confidence *= 1.3; // 30% boost for perfect conditions (50% success rate)
        }

        confidence = confidence.clamp(0.0, 95.0);

        // Entry decision with more permissive threshold
        let approved = confidence >= 28.0; // Reduced from 32.0%

        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "ENHANCED_DROP_ANALYSIS",
                &format!(
                    "🎯 {} drop -{:.1}% over {}s → conf {:.0}% → {} [drop_score:{:.1} activity:{:.1} liquidity:{:.1} perfect_storm:{}]",
                    token.symbol,
                    sig.drop_percent,
                    sig.window_sec,
                    confidence,
                    if approved {
                        "APPROVE"
                    } else {
                        "REJECT"
                    },
                    drop_score,
                    activity_score,
                    liquidity_score,
                    is_perfect_storm
                )
            );
        }

        return (
            approved,
            confidence,
            if approved {
                let reason = if is_perfect_storm {
                    format!(
                        "Perfect storm: -{:.1}% drop, {:.0} SOL liquidity, high activity",
                        sig.drop_percent,
                        reserve_sol
                    )
                } else {
                    format!(
                        "Enhanced scalp: -{:.1}% over {}s (conf: {:.0}%)",
                        sig.drop_percent,
                        sig.window_sec,
                        confidence
                    )
                };
                reason
            } else {
                format!(
                    "Enhanced analysis: -{:.1}% drop, conf {:.0}% < 28% [activity:{:.1} liquidity:{:.1}]",
                    sig.drop_percent,
                    confidence,
                    activity_score,
                    liquidity_score
                )
            },
        );
    } else {
        if is_debug_entry_enabled() {
            // Log last few prices to understand zeros
            let recent_prices: Vec<String> = price_history
                .iter()
                .take(5)
                .map(|(ts, price)| format!("{:.9}@{}", price, ts.format("%H:%M:%S")))
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

/// Calculate transaction activity score (0.0 to 1.0 scale)
/// Based on database analysis: >20 txns = 24.2% success, <5 txns = 3.5% success
fn calculate_activity_score(txns_5min: f64) -> f64 {
    if txns_5min >= 20.0 {
        1.0 // High activity
    } else if txns_5min >= 10.0 {
        0.7 // Medium activity
    } else if txns_5min >= 5.0 {
        0.4 // Low activity
    } else {
        0.1 // Very low activity
    }
}

/// Calculate enhanced drop magnitude score (non-linear curve favoring 8-15% sweet spot)
/// Based on database analysis: 7-15% drops have best success rates
fn calculate_drop_magnitude_score(drop_percent: f64) -> f64 {
    if drop_percent >= 8.0 && drop_percent <= 15.0 {
        // Sweet spot: enhanced scoring
        1.0
    } else if drop_percent >= 7.0 && drop_percent <= 20.0 {
        // Good range: standard scoring
        0.8
    } else if drop_percent >= 20.0 && drop_percent <= 30.0 {
        // Moderate range: reduced scoring
        0.6
    } else if drop_percent > 30.0 {
        // Extreme drops: heavily penalized (7-19% success rate)
        0.3
    } else {
        // Below minimum
        0.0
    }
}

/// Calculate liquidity tier score (0.0 to 1.0 scale)
/// Based on database analysis: 250-1000 SOL = 50%+ success rate
fn calculate_liquidity_score(reserve_sol: f64) -> f64 {
    if reserve_sol >= 250.0 && reserve_sol <= 1000.0 {
        1.0 // Sweet spot (50%+ success rate)
    } else if reserve_sol >= 100.0 && reserve_sol <= 500.0 {
        0.8 // Good range (35% success rate)
    } else if reserve_sol >= 50.0 {
        0.6 // Acceptable range
    } else if reserve_sol >= 10.0 {
        0.3 // Minimum viable
    } else {
        0.1 // Very low liquidity (1.6% success rate)
    }
}

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

/// Get current pool data: (price_sol, age_minutes, reserve_sol)
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

                    let reserve_sol = price_result.reserve_sol.unwrap_or_else(|| {
                        // Fallback: estimate SOL reserves from legacy liquidity data if available
                        token.liquidity
                            .as_ref()
                            .and_then(|l| l.usd)
                            .map(|usd_liq| usd_liq / 200.0) // Updated conversion at $200/SOL
                            .unwrap_or(0.0)
                    });

                    Some((price, data_age_minutes, reserve_sol))
                }
                _ => None,
            }
        }
        None => None,
    }
}

// =============================================================================
// PROFIT TARGET CALCULATION
// =============================================================================

/// Calculate profit targets based on drop analysis and SOL reserves
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    // Pull current pool data first (price + SOL reserves)
    let (current_price_opt, reserve_sol) = match get_current_pool_data(token).await {
        Some((price, _age_min, reserves)) => (Some(price), reserves),
        None =>
            (
                None,
                token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .map(|usd_liq| usd_liq / 200.0) // Updated conversion at $200/SOL
                    .unwrap_or(20.0), // Default to 20 SOL reserves
            ),
    };

    // Base profit targets by SOL reserves (updated for $200/SOL and aligned with entry tiers)
    let (mut min_profit, mut max_profit): (f64, f64) = if reserve_sol < 10.0 {
        // Below minimum viable liquidity
        (35.0, 140.0)
    } else if reserve_sol < 50.0 {
        // Low liquidity tier
        (28.0, 110.0)
    } else if reserve_sol < 250.0 {
        // Medium liquidity tier (good for scalping)
        (20.0, 85.0)
    } else if reserve_sol < 1000.0 {
        // High liquidity tier (sweet spot for entries)
        (15.0, 70.0)
    } else if reserve_sol < 2500.0 {
        // Very high liquidity tier
        (12.0, 55.0)
    } else {
        // Mega pools (limited entry focus)
        (10.0, 45.0)
    };

    // Volatility-based adjustment using recent window
    let pool_service = get_pool_service();
    let price_history = pool_service.get_price_history(&token.mint).await;
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
