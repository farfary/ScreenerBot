/// Conservative Drop Detector (balanced and stable)
///
/// OPTIMIZED FOR STABLE TRADING with 15-35% profit targets:
/// - Conservative 30s-10min detection windows (balanced approach)
/// - ATH prevention using multi-timeframe analysis
/// - Database-driven confidence scoring with stability weighting
/// - Higher confidence thresholds for quality entries
/// - Enhanced liquidity filters for successful execution

use crate::global::is_debug_entry_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::{ get_pool_service, Token, PriceOptions };
use chrono::{ DateTime, Utc };

// =============================================================================
// CONSERVATIVE TRADING CONFIGURATION PARAMETERS
// =============================================================================

// Balanced windows for stable entries (30s to 10min)
const MIN_PRICE_POINTS: usize = 8; // Increased from 3 for better analysis
const MAX_DATA_AGE_MIN: i64 = 5; // Keep tight data freshness requirement

// Conservative liquidity filter for more stable entries
const MIN_RESERVE_SOL: f64 = 20.0; // Higher minimum for stability
const MAX_RESERVE_SOL: f64 = 3000.0; // Higher maximum for less restrictive filtering

// CONSERVATIVE entry windows - more balanced approach
const WINDOWS_SEC: [i64; 6] = [30, 60, 120, 180, 300, 600]; // 30s to 10min windows
const MIN_DROP_PERCENT: f64 = 5.0; // Higher minimum for quality entries
const MAX_DROP_PERCENT: f64 = 75.0; // Allow larger drops for volatile tokens

// ATH Prevention parameters for scalping
const ATH_LOOKBACK_15MIN: i64 = 900; // 15 minutes
const ATH_LOOKBACK_1HR: i64 = 3600; // 1 hour
const ATH_LOOKBACK_6HR: i64 = 21600; // 6 hours
const ATH_THRESHOLD_15MIN: f64 = 0.95; // 95% of 15min high
const ATH_THRESHOLD_1HR: f64 = 0.9; // 90% of 1hr high
const ATH_THRESHOLD_6HR: f64 = 0.85; // 85% of 6hr high

// Conservative activity thresholds for stable entries
const HIGH_ACTIVITY_ENTRY: f64 = 20.0; // Reduced from 25.0 for high activity
const MED_ACTIVITY_ENTRY: f64 = 8.0; // Reduced from 12.0 for medium activity
const MIN_ACTIVITY_ENTRY: f64 = 3.0; // Reduced from 5.0 for minimum activity

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
                    calculate_scalp_activity_score(total_5m as f64)
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

        // Fallback: If we have at least 1 price point, still attempt basic evaluation with higher threshold
        if price_history.len() >= 1 && current_price > 0.0 {
            let recent_price = price_history[0].1;
            if recent_price > 0.0 && recent_price.is_finite() {
                let instant_drop = ((recent_price - current_price) / recent_price) * 100.0;
                if instant_drop >= 8.0 && instant_drop <= 75.0 {
                    // Higher minimum drop requirement
                    // Conservative confidence for single-point drops
                    let confidence = (20.0 + instant_drop * 0.6).min(50.0); // Reduced scaling
                    if confidence >= 35.0 {
                        // Higher confidence threshold
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
                        return (
                            true,
                            confidence,
                            format!("Conservative instant drop -{:.1}%", instant_drop),
                        );
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
        // Enhanced confidence calculation with conservative approach
        let mut confidence = 20.0; // Reduced base from 25.0

        // Drop magnitude (more conservative scoring)
        let drop_score = calculate_drop_magnitude_score(sig.drop_percent);
        confidence += drop_score * 25.0; // Reduced from 35.0 scaling

        // Transaction activity (reduced impact for conservative approach)
        let activity_score = calculate_scalp_activity_score(activity_score);
        confidence += activity_score * 15.0; // Reduced from 25.0

        // ATH Prevention Analysis
        let (ath_safe, max_ath_pct) = check_ath_risk(&price_history, current_price).await;
        if !ath_safe {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "ATH_PREVENTION_SCALP",
                    &format!(
                        "❌ {} ATH prevention: {:.1}% of recent high - blocking entry",
                        token.symbol,
                        max_ath_pct
                    )
                );
            }
            return (false, 15.0, format!("ATH prevention: {:.1}% of recent high", max_ath_pct));
        }

        // ATH safety bonus (conservative)
        confidence += 6.0; // Reduced from 8.0
        if max_ath_pct < 70.0 {
            confidence += 3.0; // Reduced from 5.0 for being well below highs
        }

        // Transaction activity (reduced impact for conservative approach)
        confidence += activity_score * 15.0; // Reduced from 25.0

        // Liquidity impact (moderate increase)
        let liquidity_score = calculate_liquidity_score(reserve_sol);
        confidence += liquidity_score * 10.0; // Reduced from 15.0

        // Window preference (favor longer-term drops for stability)
        confidence += match sig.window_sec {
            30 => 20.0, // Short-term but not too aggressive
            60 => 25.0, // 1-minute window (good balance)
            120 => 30.0, // 2-minute window (preferred)
            180 => 28.0, // 3-minute window (good)
            300 => 25.0, // 5-minute window (standard)
            600 => 20.0, // 10-minute window (longer term)
            _ => 10.0,
        };

        // Velocity adjustments (keep existing logic)
        if sig.velocity_per_minute < -20.0 {
            confidence += 8.0;
        }
        if sig.velocity_per_minute > 15.0 {
            confidence -= 6.0;
        }

        // Conservative entry conditions (less aggressive requirements)
        let is_good_entry =
            sig.drop_percent >= 6.0 &&
            sig.drop_percent <= 20.0 && // Balanced range for conservative entry
            sig.window_sec >= 60 && // Minimum 1-minute window for stability
            reserve_sol >= 50.0 && // Good liquidity requirement
            reserve_sol <= 800.0 && // Reasonable maximum
            activity_score >= 0.6 && // Moderate activity requirement
            ath_safe; // ATH safe requirement
        if is_good_entry {
            confidence *= 1.25; // 25% boost for good conservative conditions
        }

        confidence = confidence.clamp(0.0, 95.0);

        // Conservative entry confidence threshold
        let approved = confidence >= 45.0; // Increased from 35.0% for higher quality entries

        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "ENTRY_ANALYSIS_COMPLETE",
                &format!(
                    "🎯 {} entry: -{:.1}%/{}s → conf {:.1}% → {} [activity:{:.1} liquidity:{:.0} ath_safe:{} ath_pct:{:.1}% good_entry:{}]",
                    token.symbol,
                    sig.drop_percent,
                    sig.window_sec,
                    confidence,
                    if approved {
                        "ENTER"
                    } else {
                        "REJECT"
                    },
                    activity_score,
                    reserve_sol,
                    ath_safe,
                    max_ath_pct,
                    is_good_entry
                )
            );
        }

        return (
            approved,
            confidence,
            if approved {
                let reason = if is_good_entry {
                    format!(
                        "Conservative entry: -{:.1}%/{}s, {:.0} SOL, ATH-safe, good activity",
                        sig.drop_percent,
                        sig.window_sec,
                        reserve_sol
                    )
                } else {
                    format!(
                        "Standard entry: -{:.1}%/{}s, ATH-safe (conf: {:.1}%)",
                        sig.drop_percent,
                        sig.window_sec,
                        confidence
                    )
                };
                reason
            } else {
                format!(
                    "Entry analysis: -{:.1}%/{}s, conf {:.1}% < 45%, ATH: {:.1}%, activity: {:.1}",
                    sig.drop_percent,
                    sig.window_sec,
                    confidence,
                    max_ath_pct,
                    activity_score
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
                "NO_DROP_DETECTED",
                &format!(
                    "❌ {} no entry drops {:.0}-{:.0}% detected in conservative windows [{}]",
                    token.symbol,
                    MIN_DROP_PERCENT,
                    MAX_DROP_PERCENT,
                    recent_prices.join(", ")
                )
            );
        }
        return (false, 20.0, "No entry opportunity detected in conservative windows".to_string());
    }
}

// Removed complex multi-style detectors and confidence systems

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Conservative activity score for stable entries (0.0 to 1.0 scale)
/// Based on database analysis with more conservative thresholds
fn calculate_scalp_activity_score(txns_5min: f64) -> f64 {
    if txns_5min >= HIGH_ACTIVITY_ENTRY {
        1.0 // High activity (conservative threshold)
    } else if txns_5min >= MED_ACTIVITY_ENTRY {
        0.7 // Good activity (reduced from 0.8)
    } else if txns_5min >= MIN_ACTIVITY_ENTRY {
        0.4 // Minimum activity (reduced from 0.5)
    } else {
        0.1 // Below entry threshold (reduced from 0.2)
    }
}

/// ATH Prevention Analysis - checks if current price is too close to recent highs
async fn check_ath_risk(price_history: &[(DateTime<Utc>, f64)], current_price: f64) -> (bool, f64) {
    let now = Utc::now();
    let mut max_ath_percentage: f64 = 0.0;
    let mut near_ath = false;

    // Check 15min ATH
    let prices_15min: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= ATH_LOOKBACK_15MIN)
        .map(|(_, p)| *p)
        .collect();
    if prices_15min.len() >= 3 {
        let high_15min = prices_15min.iter().fold(0.0f64, |a, b| a.max(*b));
        if high_15min > 0.0 {
            let ath_pct = current_price / high_15min;
            max_ath_percentage = max_ath_percentage.max(ath_pct);
            if ath_pct >= ATH_THRESHOLD_15MIN {
                near_ath = true;
            }
        }
    }

    // Check 1hr ATH
    let prices_1hr: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= ATH_LOOKBACK_1HR)
        .map(|(_, p)| *p)
        .collect();
    if prices_1hr.len() >= 5 {
        let high_1hr = prices_1hr.iter().fold(0.0f64, |a, b| a.max(*b));
        if high_1hr > 0.0 {
            let ath_pct = current_price / high_1hr;
            max_ath_percentage = max_ath_percentage.max(ath_pct);
            if ath_pct >= ATH_THRESHOLD_1HR {
                near_ath = true;
            }
        }
    }

    // Check 6hr ATH
    let prices_6hr: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= ATH_LOOKBACK_6HR)
        .map(|(_, p)| *p)
        .collect();
    if prices_6hr.len() >= 10 {
        let high_6hr = prices_6hr.iter().fold(0.0f64, |a, b| a.max(*b));
        if high_6hr > 0.0 {
            let ath_pct = current_price / high_6hr;
            max_ath_percentage = max_ath_percentage.max(ath_pct);
            if ath_pct >= ATH_THRESHOLD_6HR {
                near_ath = true;
            }
        }
    }

    (!near_ath, max_ath_percentage * 100.0) // Return (ath_safe, max_ath_percentage)
}

/// Calculate enhanced drop magnitude score (balanced approach for various drop sizes)
/// Based on database analysis: 7-15% drops have best success rates, but allow larger drops with reduced scoring
fn calculate_drop_magnitude_score(drop_percent: f64) -> f64 {
    if drop_percent >= 8.0 && drop_percent <= 15.0 {
        // Sweet spot: enhanced scoring
        1.0
    } else if drop_percent >= 7.0 && drop_percent <= 25.0 {
        // Good range: standard scoring
        0.8
    } else if drop_percent >= 25.0 && drop_percent <= 45.0 {
        // Moderate range: reduced scoring
        0.6
    } else if drop_percent >= 45.0 && drop_percent <= 70.0 {
        // Large drops: lower scoring but still acceptable
        0.4
    } else if drop_percent > 70.0 {
        // Extreme drops: minimal scoring but not blocked
        0.2
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

/// Calculate profit targets optimized for fast scalping (5-10% minimum focus)
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
                    .unwrap_or(30.0), // Default to 30 SOL reserves for scalping
            ),
    };

    // Enhanced activity scoring for profit targets
    let txns_5min = token.txns
        .as_ref()
        .and_then(|txns| txns.m5.clone())
        .map(|t| t.buys.unwrap_or(0) + t.sells.unwrap_or(0))
        .unwrap_or(0) as f64;
    let activity_score = calculate_scalp_activity_score(txns_5min);

    // Base profit targets with conservative approach for better success rates
    let (mut min_profit, mut max_profit): (f64, f64) = if reserve_sol < 25.0 {
        // Below minimum liquidity threshold
        (35.0, 80.0) // Higher targets for risky low liquidity
    } else if reserve_sol < 100.0 {
        // Low liquidity tier
        (25.0, 60.0)
    } else if reserve_sol < 300.0 {
        // Medium liquidity tier (good for conservative trading)
        (18.0, 45.0)
    } else if reserve_sol < 800.0 {
        // High liquidity tier (optimal range)
        (15.0, 35.0) // Conservative 15-35% targets
    } else if reserve_sol < 1200.0 {
        // Very high liquidity tier
        (20.0, 40.0)
    } else {
        // Extremely high liquidity (higher targets for potential whale movements)
        (25.0, 50.0)
    };

    // Activity multiplier (reduced impact for conservative approach)
    let activity_multiplier = 1.0 + (activity_score - 0.5) * 0.25; // Reduced from 0.4 to 0.25
    min_profit *= activity_multiplier;
    max_profit *= activity_multiplier;

    // Volatility-based adjustment using longer window for stability (5min vs 1min)
    let pool_service = get_pool_service();
    let price_history = pool_service.get_price_history(&token.mint).await;
    if current_price_opt.is_some() && price_history.len() >= 5 {
        let now = Utc::now();
        let prices_5min: Vec<f64> = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= 300) // 5min window for more stability
            .map(|(_, p)| *p)
            .collect();
        if prices_5min.len() >= 3 {
            let high_5min = prices_5min.iter().fold(0.0f64, |a, b| a.max(*b));
            let low_5min = prices_5min.iter().fold(f64::INFINITY, |a, b| a.min(*b));
            if high_5min.is_finite() && low_5min.is_finite() && high_5min > 0.0 && low_5min > 0.0 {
                let hl_range_5min = ((high_5min - low_5min) / high_5min) * 100.0;
                let scale = (hl_range_5min / 50.0).clamp(0.0, 0.4); // Reduced scaling
                min_profit *= 1.0 + scale * 0.3; // Reduced adjustment
                max_profit *= 1.0 + scale * 0.4; // Reduced adjustment
            }
        }
    }

    // Ensure conservative profit thresholds
    min_profit = min_profit.clamp(12.0, 40.0); // 12-40% minimum range (increased from 5-30%)
    max_profit = max_profit.clamp(25.0, 100.0); // 25-100% maximum range (increased from 15-80%)

    // Ensure proper spread for conservative trading
    if max_profit - min_profit < 10.0 {
        // Increased minimum spread from 6.0 to 10.0
        max_profit = (min_profit + 10.0).min(100.0);
    }

    (min_profit, max_profit)
}
