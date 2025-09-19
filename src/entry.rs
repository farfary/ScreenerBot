/// Conservative Drop Detector (balanced and stable)
///
/// OPTIMIZED FOR STABLE TRADING with 15-35% profit targets:
/// - Conservative 30s-10min detection windows (balanced approach)
/// - ATH prevention using multi-timeframe analysis
/// - Database-driven confidence scoring with stability weighting
/// - Higher confidence thresholds for quality entries

use crate::global::is_debug_entry_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::{ get_pool_price, get_price_history, PriceResult };
use crate::tokens::security::get_security_analyzer; // for optional cached holder count
use crate::learner::get_learning_integration;
use chrono::{ DateTime, Utc };
use once_cell::sync::Lazy;
use tokio::sync::RwLock as AsyncRwLock; // switched from StdRwLock
use std::time::{ Instant, Duration };

// Lightweight TTL cache for recent exit prices to reduce DB pressure.
struct ExitPriceCacheEntry {
    prices: Vec<f64>,
    fetched_at: Instant,
}
static RECENT_EXIT_PRICE_CACHE: Lazy<
    AsyncRwLock<std::collections::HashMap<String, ExitPriceCacheEntry>>
> = Lazy::new(|| AsyncRwLock::new(std::collections::HashMap::new()));
const EXIT_PRICE_CACHE_TTL: Duration = Duration::from_secs(30);
const EXIT_PRICE_CACHE_MAX_ENTRIES: usize = 1024; // prune safeguard

// Optimized: Check cache only, no database calls during token checking
async fn get_cached_recent_exit_prices_fast(mint: &str) -> Vec<f64> {
    let map = RECENT_EXIT_PRICE_CACHE.read().await;
    if let Some(entry) = map.get(mint) {
        if entry.fetched_at.elapsed() < EXIT_PRICE_CACHE_TTL {
            return entry.prices.clone();
        }
    }
    Vec::new() // Return empty if not cached, avoid DB during token checking
}

// Batch preload exit prices for multiple tokens (call before token processing)
pub async fn preload_exit_prices_batch(mints: &[String]) -> Result<(), String> {
    use crate::positions::get_positions_database;

    // Only load for mints not in cache or expired
    let mut mints_to_load = Vec::new();
    {
        let map = RECENT_EXIT_PRICE_CACHE.read().await;
        for mint in mints {
            if let Some(entry) = map.get(mint) {
                if entry.fetched_at.elapsed() >= EXIT_PRICE_CACHE_TTL {
                    mints_to_load.push(mint.clone());
                }
            } else {
                mints_to_load.push(mint.clone());
            }
        }
    }

    if mints_to_load.is_empty() {
        return Ok(());
    }

    // Single database call for all mints
    if let Ok(db_lock) = get_positions_database().await {
        let guard = db_lock.lock().await;
        if let Some(ref db) = *guard {
            // Batch load all exit prices in one query (implement in positions_db if needed)
            for mint in &mints_to_load {
                if
                    let Ok(rows) = db.get_recent_closed_exit_prices_for_mint(
                        mint,
                        REENTRY_LOOKBACK_MAX
                    ).await
                {
                    let mut prices = Vec::new();
                    for (exit_p, eff_p) in rows.into_iter() {
                        if let Some(p) = eff_p.or(exit_p) {
                            if p.is_finite() && p > 0.0 {
                                prices.push(p);
                            }
                        }
                    }

                    // Store in cache
                    let mut mapw = RECENT_EXIT_PRICE_CACHE.write().await;
                    mapw.insert(mint.clone(), ExitPriceCacheEntry {
                        prices,
                        fetched_at: Instant::now(),
                    });

                    // Simple pruning: remove excess entries
                    if mapw.len() > EXIT_PRICE_CACHE_MAX_ENTRIES {
                        let keys_to_remove: Vec<String> = mapw
                            .keys()
                            .take(mapw.len() - EXIT_PRICE_CACHE_MAX_ENTRIES)
                            .cloned()
                            .collect();
                        for key in keys_to_remove {
                            mapw.remove(&key);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// CONSERVATIVE TRADING CONFIGURATION PARAMETERS
// =============================================================================

// Balanced windows for stable entries (30s to 10min)
const MIN_PRICE_POINTS: usize = 3; // Increased from 3 for better analysis

// CONSERVATIVE entry windows - more balanced approach
const WINDOWS_SEC: [i64; 6] = [10, 20, 40, 80, 160, 320]; // 30s to 10min windows
const MIN_DROP_PERCENT: f64 = 1.0; // Higher minimum for quality entries
const MAX_DROP_PERCENT: f64 = 90.0; // Allow larger drops for volatile tokens

// ============================= TREND ENTRY (Conservative) =============================
// Lightweight, safe detectors to capture strong trends while avoiding noise.
// These are designed to work with existing pool price history (no extra sources).
const TREND_MIN_HISTORY_POINTS: usize = 6;
const TREND_RANGE_SEC: i64 = 240; // 4m range for breakout context
const RETEST_HOLD_SEC: i64 = 25; // time to hold reclaimed level
const SMA_RECLAIM_SEC: i64 = 90; // ~EMA20 proxy over short horizon
const HL_LOOKBACK_SEC: i64 = 180; // 3m lookback for HL structure (not yet used)
const MOMENTUM_LOOKBACK_SEC: i64 = 120; // momentum check window
const BREAKOUT_BUFFER_PCT: f64 = 1.8; // breakout must exceed range high by this buffer
const RETEST_TOLERANCE_PCT: f64 = 1.2; // acceptable retest depth around breakout line
const MICRO_PULLBACK_MAX_PCT: f64 = 3.5; // max pullback for momentum continuation validation

// ============================= MICRO-LIQUIDITY CAPITULATION =============================
// Goal: Auto-approve high-upside entries when a token has near-zero real SOL liquidity and
// has experienced an extreme (>95-99%) capitulation from its recent local high while having
// a reasonable holder distribution (>= MICRO_LIQ_MIN_HOLDERS) to reduce pure honeypot risk.
// We rely ONLY on cached security data for holder counts (no fresh RPC in the hot loop).
// If holder count is unavailable we proceed (optimistic) but tag the reason accordingly.
const MICRO_LIQ_SOL_RESERVE_MAX: f64 = 0.01; // < 0.01 SOL reserves considered micro-liq
const MICRO_LIQ_MIN_DROP_PERCENT: f64 = 95.0; // At least 95% drop from recent high
const MICRO_LIQ_PREFERRED_DROP_PERCENT: f64 = 97.0; // Stronger confidence above this
const MICRO_LIQ_MIN_HOLDERS: u32 = 50; // Require >= 50 holders when info cached
const MICRO_LIQ_LOOKBACK_SECS: i64 = 900; // 15 min lookback window to find recent high
const MICRO_LIQ_CONF_BASE: f64 = 82.0; // Base confidence when triggered
const MICRO_LIQ_CONF_BONUS: f64 = 8.0; // Bonus if drop >= preferred threshold

fn get_cached_holder_count_fast(mint: &str) -> Option<u32> {
    // Uses security analyzer in-memory cache only; never triggers fresh analysis.
    // Safe (no panics) – returns None if analyzer not yet initialized or holder info missing.
    let analyzer = get_security_analyzer();
    if let Some(info) = analyzer.cache.get(mint) {
        if let Some(holder) = info.holder_info.as_ref() {
            return Some(holder.total_holders);
        }
    }
    None
}

fn detect_micro_liquidity_capitulation(
    price_info: &PriceResult,
    history: &[(DateTime<Utc>, f64)]
) -> Option<(f64, f64, Option<u32>)> {
    if price_info.sol_reserves > 0.0 && price_info.sol_reserves <= MICRO_LIQ_SOL_RESERVE_MAX {
        // Determine recent high within lookback
        let now = Utc::now();
        let mut recent_high = 0.0f64;
        for (ts, p) in history.iter() {
            if (now - *ts).num_seconds() <= MICRO_LIQ_LOOKBACK_SECS && *p > 0.0 && p.is_finite() {
                if *p > recent_high {
                    recent_high = *p;
                }
            }
        }
        if recent_high > 0.0 && price_info.price_sol > 0.0 && recent_high > price_info.price_sol {
            let drop_percent = ((recent_high - price_info.price_sol) / recent_high) * 100.0;
            if drop_percent >= MICRO_LIQ_MIN_DROP_PERCENT {
                let holder_count = get_cached_holder_count_fast(&price_info.mint);
                // If holder count known, enforce threshold; else allow optimistic proceed.
                if holder_count.map(|c| c >= MICRO_LIQ_MIN_HOLDERS).unwrap_or(true) {
                    return Some((drop_percent, recent_high, holder_count));
                }
            }
        }
    }
    None
}

// ATH Prevention parameters for scalping
const ATH_LOOKBACK_15MIN: i64 = 900; // 15 minutes
const ATH_LOOKBACK_1HR: i64 = 3600; // 1 hour
const ATH_LOOKBACK_6HR: i64 = 21600; // 6 hours
const ATH_THRESHOLD_15MIN: f64 = 0.75; // 95% of 15min high
const ATH_THRESHOLD_1HR: f64 = 0.6; // 90% of 1hr high
const ATH_THRESHOLD_6HR: f64 = 0.55; // 85% of 6hr high

// Conservative activity thresholds for stable entries
const HIGH_ACTIVITY_ENTRY: f64 = 20.0; // Reduced from 25.0 for high activity
const MED_ACTIVITY_ENTRY: f64 = 8.0; // Reduced from 12.0 for medium activity
const MIN_ACTIVITY_ENTRY: f64 = 3.0; // Reduced from 5.0 for minimum activity

// ============================= Re-Entry Adaptive Thresholds =============================
// Each additional prior closed verified position for same token (recent) demands deeper fresh drop
// to avoid catching a decaying downtrend too early.
pub const REENTRY_LOOKBACK_MAX: usize = 6; // how many prior exits to inspect
pub const REENTRY_DROP_EXTRA_PER_ENTRY_PCT: f64 = 2.2; // each prior entry adds this % to required min drop
pub const REENTRY_DROP_EXTRA_MAX_PCT: f64 = 14.0; // cap additional drop requirement
pub const REENTRY_MIN_DISCOUNT_TO_LAST_EXIT_PCT: f64 = 4.0; // require current price at least this % below most recent verified exit
pub const REENTRY_LOCAL_STABILITY_SECS: i64 = 45; // need some sideways stabilization length
pub const REENTRY_LOCAL_MAX_VOLATILITY_PCT: f64 = 9.0; // if intrarange > this then treat as unstable (need deeper drop)

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

/// Analyze recent micro-structure for stabilization after a drop.
/// Returns (local_low, local_high, stabilization_score[0..1], intrarange_pct)
fn analyze_local_structure(
    price_history: &[(DateTime<Utc>, f64)],
    seconds: i64
) -> (f64, f64, f64, f64) {
    if seconds <= 5 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let now = Utc::now();
    let mut prices: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= seconds)
        .map(|(_, p)| *p)
        .filter(|p| *p > 0.0 && p.is_finite())
        .collect();
    if prices.len() < 3 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut local_low = f64::INFINITY;
    let mut local_high: f64 = 0.0;
    for p in &prices {
        local_low = local_low.min(*p);
        local_high = local_high.max(*p);
    }
    if !local_low.is_finite() || local_low <= 0.0 || local_high <= 0.0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let intrarange_pct = ((local_high - local_low) / local_high) * 100.0;
    // Stabilization: measure proportion of prices within upper half after first third timeframe.
    let third = (prices.len() / 3).max(1);
    let latter_slice = &prices[third..];
    let median_after: f64 = {
        let mut s = latter_slice.to_vec();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        s[s.len() / 2]
    };
    // Score favors tight range & median recovery in upper half.
    let tight_factor = (
        1.0 - (intrarange_pct / REENTRY_LOCAL_MAX_VOLATILITY_PCT).clamp(0.0, 1.0)
    ).max(0.0);
    let recovery_factor = if local_high > local_low {
        ((median_after - local_low) / (local_high - local_low)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let stabilization_score = (0.55 * tight_factor + 0.45 * recovery_factor).clamp(0.0, 1.0);
    (local_low, local_high, stabilization_score, intrarange_pct)
}

// =============================================================================
// MAIN ENTRY FUNCTION
// =============================================================================

// Simple average over last N seconds (SMA proxy for very short EMA)
fn sma_over_window(price_history: &[(DateTime<Utc>, f64)], seconds: i64) -> Option<f64> {
    let now = Utc::now();
    let mut vals: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= seconds)
        .map(|(_, p)| *p)
        .filter(|p| *p > 0.0 && p.is_finite())
        .collect();
    if vals.len() < 3 {
        return None;
    }
    let sum: f64 = vals.iter().sum();
    let avg = sum / (vals.len() as f64);
    if avg.is_finite() && avg > 0.0 {
        Some(avg)
    } else {
        None
    }
}

fn window_high(price_history: &[(DateTime<Utc>, f64)], seconds: i64) -> Option<f64> {
    let now = Utc::now();
    let mut high = 0.0f64;
    let mut count = 0;
    for (ts, p) in price_history.iter() {
        if (now - *ts).num_seconds() <= seconds && *p > 0.0 && p.is_finite() {
            if *p > high {
                high = *p;
            }
            count += 1;
        }
    }
    if count >= 3 && high.is_finite() && high > 0.0 {
        Some(high)
    } else {
        None
    }
}

// Detect breakout above recent range with a clean retest that holds.
// Returns (approved, confidence_bonus, reason, ath_reclaim_ok)
fn detect_breakout_retest(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    stabilization_score: f64,
    intrarange_pct: f64
) -> (bool, f64, &'static str, bool) {
    if price_history.len() < TREND_MIN_HISTORY_POINTS || current_price <= 0.0 {
        return (false, 0.0, "", false);
    }
    if intrarange_pct > REENTRY_LOCAL_MAX_VOLATILITY_PCT {
        return (false, 0.0, "", false);
    }

    let Some(r_high) = window_high(price_history, TREND_RANGE_SEC) else {
        return (false, 0.0, "", false);
    };
    if r_high <= 0.0 {
        return (false, 0.0, "", false);
    }

    // Breakout must clear range high by buffer
    if current_price < r_high * (1.0 + BREAKOUT_BUFFER_PCT / 100.0) {
        return (false, 0.0, "", false);
    }

    // Retest: look for prices within tolerance below/around r_high in last RETEST_HOLD_SEC
    let now = Utc::now();
    let mut touched = false;
    let mut held_count = 0usize;
    for (ts, p) in price_history.iter() {
        let age = (now - *ts).num_seconds();
        if age <= RETEST_HOLD_SEC {
            if *p > 0.0 && p.is_finite() {
                let diff_pct = ((*p - r_high) / r_high) * 100.0;
                if diff_pct.abs() <= RETEST_TOLERANCE_PCT {
                    touched = true;
                }
                if *p >= r_high {
                    held_count += 1;
                }
            }
        }
    }
    if !touched || held_count < 3 {
        return (false, 0.0, "", false);
    }

    // Stabilization minimum
    if stabilization_score < 0.3 {
        return (false, 0.0, "", false);
    }

    (true, 18.0, "TREND_BREAKOUT_RETEST", true)
}

// Detect reclaim above short SMA after a controlled pullback; favors continuation.
fn detect_sma_reclaim(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
    stabilization_score: f64
) -> (bool, f64, &'static str) {
    if price_history.len() < TREND_MIN_HISTORY_POINTS || current_price <= 0.0 {
        return (false, 0.0, "");
    }
    let Some(sma) = sma_over_window(price_history, SMA_RECLAIM_SEC) else {
        return (false, 0.0, "");
    };
    if sma <= 0.0 {
        return (false, 0.0, "");
    }

    // Require momentum alignment on 120s window
    let now = Utc::now();
    let prices_120: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= MOMENTUM_LOOKBACK_SEC)
        .map(|(_, p)| *p)
        .collect();
    if prices_120.len() < 3 {
        return (false, 0.0, "");
    }
    let v120 = calculate_velocity(&prices_120, MOMENTUM_LOOKBACK_SEC);
    if v120 < 8.0 {
        return (false, 0.0, "");
    }

    // Reclaim condition and minimum stabilization
    if current_price >= sma * 1.002 && stabilization_score >= 0.3 {
        return (true, 14.0, "TREND_SMA_RECLAIM");
    }
    (false, 0.0, "")
}

/// Main entry point for determining if a token should be bought
/// Returns (approved_for_entry, confidence_score, reason)
pub async fn should_buy(price_info: &PriceResult) -> (bool, f64, String) {
    let price_history = get_price_history(&price_info.mint);

    if is_debug_entry_enabled() {
        log(LogTag::Entry, "DEBUG", &format!("� Using pool price for {}", price_info.mint));
    }

    let prior_exit_prices = get_cached_recent_exit_prices_fast(&price_info.mint).await;
    let prior_count = prior_exit_prices.len();
    let last_exit_price = prior_exit_prices.first().cloned();

    let current_price = if price_info.price_sol > 0.0 && price_info.price_sol.is_finite() {
        price_info.price_sol
    } else {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "WARN",
                &format!("❌ {} invalid price: {:.9}", price_info.mint, price_info.price_sol)
            );
        }
        return (false, 0.0, "Invalid price".to_string());
    };

    let activity_score = 0.5;

    // Get recent pool price history
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "HISTORY_REQUEST",
            &format!("📈 Using price history for {}", price_info.mint)
        );
    }

    let mut converted_history: Vec<(DateTime<Utc>, f64)> = price_history
        .iter()
        .map(|p| (
            Utc::now() - chrono::Duration::seconds(p.timestamp.elapsed().as_secs() as i64),
            p.price_sol,
        ))
        .collect();
    converted_history.retain(|(_, p)| *p > 0.0 && p.is_finite());

    // Micro-liquidity capitulation fast-path (before standard insufficient/history gates)
    if converted_history.len() >= 5 {
        // need minimal structure
        if
            let Some((drop_percent, recent_high, holder_opt)) = detect_micro_liquidity_capitulation(
                price_info,
                &converted_history
            )
        {
            let mut confidence = MICRO_LIQ_CONF_BASE;
            if drop_percent >= MICRO_LIQ_PREFERRED_DROP_PERCENT {
                confidence += MICRO_LIQ_CONF_BONUS;
            }
            // Slight penalty if holder count unknown (riskier) – keeps below max cap
            if holder_opt.is_none() {
                confidence -= 6.0;
            }
            confidence = confidence.clamp(60.0, 95.0);

            // Apply learner confidence boost for micro-liquidity scenarios
            let learning = get_learning_integration();
            let original_confidence = confidence;
            let adjustment = learning.get_entry_confidence_adjustment(
                &price_info.mint,
                current_price,
                drop_percent,
                0.0 // ath_proximity not available in this context
            ).await;
            confidence = (confidence * adjustment).clamp(60.0, 95.0);

            if is_debug_entry_enabled() && (adjustment - 1.0).abs() > 0.05 {
                log(
                    LogTag::Entry,
                    "LEARNER_MICRO_LIQ_BOOST",
                    &format!(
                        "🧠 {} micro-liq learner adjustment: {:.1}% → {:.1}% (multiplier: {:.2}x)",
                        price_info.mint,
                        original_confidence,
                        confidence,
                        adjustment
                    )
                );
            }

            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "MICRO_LIQ_CAPITULATION",
                    &format!(
                        "🚀 {} micro-liq capitulation detected: reserves {:.5} SOL, drop -{:.2}%, recent_high {:.9} holders {:?} → conf {:.1}% AUTO-ENTER",
                        price_info.mint,
                        price_info.sol_reserves,
                        drop_percent,
                        recent_high,
                        holder_opt,
                        confidence
                    )
                );
            }
            let reason = if let Some(hc) = holder_opt {
                format!(
                    "Micro-liq capitulation entry: -{:.1}% from high, {:.5} SOL reserves, holders {} (conf {:.1}%)",
                    drop_percent,
                    price_info.sol_reserves,
                    hc,
                    confidence
                )
            } else {
                format!(
                    "Micro-liq capitulation entry: -{:.1}% from high, {:.5} SOL reserves (holders unknown, conf {:.1}%)",
                    drop_percent,
                    price_info.sol_reserves,
                    confidence
                )
            };
            return (true, confidence, reason);
        }
    }

    // Quick insufficient history check - avoid expensive refresh for performance
    if converted_history.len() < MIN_PRICE_POINTS {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "INSUFFICIENT_HISTORY",
                &format!(
                    "❌ {} insufficient price history: {} < {} points",
                    price_info.mint,
                    converted_history.len(),
                    MIN_PRICE_POINTS
                )
            );
        }

        if converted_history.len() >= 1 && current_price > 0.0 {
            let recent_price = converted_history[0].1;
            if recent_price > 0.0 && recent_price.is_finite() {
                let instant_drop = ((recent_price - current_price) / recent_price) * 100.0;
                if instant_drop >= 15.0 && instant_drop <= 75.0 {
                    // Higher minimum drop requirement for insufficient data
                    let confidence = (25.0 + instant_drop * 0.5).min(45.0_f64); // Conservative scaling
                    let mut final_confidence = confidence;

                    // Apply learner confidence boost for instant drop scenarios
                    let learning = get_learning_integration();
                    let adjustment = learning.get_entry_confidence_adjustment(
                        &price_info.mint,
                        current_price,
                        instant_drop,
                        0.0 // ath_proximity not available in this context
                    ).await;
                    final_confidence = (final_confidence * adjustment).clamp(0.0, 95.0);

                    if is_debug_entry_enabled() && (adjustment - 1.0).abs() > 0.05 {
                        log(
                            LogTag::Entry,
                            "LEARNER_INSTANT_DROP_BOOST",
                            &format!(
                                "🧠 {} instant drop learner adjustment: {:.1}% → {:.1}% (multiplier: {:.2}x)",
                                price_info.mint,
                                confidence,
                                final_confidence,
                                adjustment
                            )
                        );
                    }

                    if final_confidence >= 35.0 {
                        if is_debug_entry_enabled() {
                            log(
                                LogTag::Entry,
                                "INSTANT_DROP_FALLBACK",
                                &format!(
                                    "🎯 {} instant drop -{:.1}% → conf {:.0}% → APPROVE",
                                    price_info.mint,
                                    instant_drop,
                                    final_confidence
                                )
                            );
                        }
                        return (
                            true,
                            final_confidence,
                            format!("Quick entry on {:.1}% instant drop", instant_drop),
                        );
                    }
                }
            }
        }

        return (
            false,
            12.0,
            format!(
                "Insufficient price history: {} < {}",
                converted_history.len(),
                MIN_PRICE_POINTS
            ),
        );
    }

    // Local structure before selecting best drop (short horizon for stabilization)
    let (_ll, _lh, stabilization_score, intrarange_pct) = analyze_local_structure(
        &converted_history,
        REENTRY_LOCAL_STABILITY_SECS
    );

    // ============================= Trend entries (conservative) =============================
    // Try to catch high-quality trend setups before standard drop-based logic.
    // 1) Breakout + Retest (enables ATH reclaim on success)
    let (brk_ok, brk_conf, brk_tag, brk_ath_bypass) = detect_breakout_retest(
        &converted_history,
        current_price,
        stabilization_score,
        intrarange_pct
    );
    if brk_ok {
        let mut confidence = 30.0 + brk_conf; // strong base for quality trend
        // Activity boost (single use): conservative scaling
        confidence += activity_score * 12.0;
        confidence = confidence.clamp(0.0, 95.0);

        // Apply learner confidence boost for trend entries
        let learning = get_learning_integration();
        let original_confidence = confidence;
        let adjustment = learning.get_entry_confidence_adjustment(
            &price_info.mint,
            current_price,
            0.0, // drop_percent not directly available in trend context
            0.0 // ath_proximity not available in this context
        ).await;
        confidence = (confidence * adjustment).clamp(0.0, 95.0);

        if is_debug_entry_enabled() && (adjustment - 1.0).abs() > 0.05 {
            log(
                LogTag::Entry,
                "LEARNER_TREND_BOOST",
                &format!(
                    "🧠 {} trend learner adjustment: {:.1}% → {:.1}% (multiplier: {:.2}x)",
                    price_info.mint,
                    original_confidence,
                    confidence,
                    adjustment
                )
            );
        }

        // ATH risk check with optional bypass
        let (ath_safe, max_ath_pct) = if brk_ath_bypass {
            (true, 99.9)
        } else {
            check_ath_risk(&converted_history, current_price).await
        };
        if !ath_safe {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "TREND_ATH_BLOCK",
                    &format!(
                        "❌ {} {} blocked by ATH: {:.1}%",
                        price_info.mint,
                        brk_tag,
                        max_ath_pct
                    )
                );
            }
        } else {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    brk_tag,
                    &format!(
                        "🎯 {} trend entry → conf {:.1}% (stab:{:.2} intrarange:{:.1}% ath_ok:{})",
                        price_info.mint,
                        confidence,
                        stabilization_score,
                        intrarange_pct,
                        ath_safe
                    )
                );
            }
            let approved = confidence >= 43.0; // slightly lower than conservative dip threshold
            return (
                approved,
                confidence,
                if approved {
                    "Trend breakout + retest entry".to_string()
                } else {
                    format!("{}: confidence {:.1}% < 43%", brk_tag, confidence)
                },
            );
        }
    }

    // 2) SMA reclaim continuation
    let (rec_ok, rec_conf, rec_tag) = detect_sma_reclaim(
        &converted_history,
        current_price,
        stabilization_score
    );
    if rec_ok {
        let mut confidence = 26.0 + rec_conf;
        confidence += activity_score * 10.0;
        confidence = confidence.clamp(0.0, 95.0);

        // Apply learner confidence boost for SMA reclaim entries
        let learning = get_learning_integration();
        let original_confidence = confidence;
        let adjustment = learning.get_entry_confidence_adjustment(
            &price_info.mint,
            current_price,
            0.0, // drop_percent not directly available in SMA reclaim context
            0.0 // ath_proximity not available in this context
        ).await;
        confidence = (confidence * adjustment).clamp(0.0, 95.0);

        if is_debug_entry_enabled() && (adjustment - 1.0).abs() > 0.05 {
            log(
                LogTag::Entry,
                "LEARNER_SMA_RECLAIM_BOOST",
                &format!(
                    "🧠 {} SMA reclaim learner adjustment: {:.1}% → {:.1}% (multiplier: {:.2}x)",
                    price_info.mint,
                    original_confidence,
                    confidence,
                    adjustment
                )
            );
        }

        let (ath_safe, max_ath_pct) = check_ath_risk(&converted_history, current_price).await;
        if !ath_safe {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "SMA_RECLAIM_ATH_BLOCK",
                    &format!(
                        "❌ {} {} blocked by ATH: {:.1}%",
                        price_info.mint,
                        rec_tag,
                        max_ath_pct
                    )
                );
            }
        } else {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    rec_tag,
                    &format!(
                        "🎯 {} trend SMA reclaim → conf {:.1}% (stab:{:.2})",
                        price_info.mint,
                        confidence,
                        stabilization_score
                    )
                );
            }
            let approved = confidence >= 43.0;
            return (
                approved,
                confidence,
                if approved {
                    "Trend SMA reclaim entry".to_string()
                } else {
                    format!("{}: confidence {:.1}% < 43%", rec_tag, confidence)
                },
            );
        }
    }

    // Adaptive extra drop requirement for re-entry scenarios
    let mut adaptive_min_drop = MIN_DROP_PERCENT;
    if prior_count > 0 {
        let extra = ((prior_count as f64) * REENTRY_DROP_EXTRA_PER_ENTRY_PCT).min(
            REENTRY_DROP_EXTRA_MAX_PCT
        );
        adaptive_min_drop += extra; // require deeper fresh drop
        if let Some(last_exit) = last_exit_price {
            // Ensure current price meaningfully below last exit to avoid chasing
            if last_exit > 0.0 && current_price > 0.0 {
                let discount_pct = ((last_exit - current_price) / last_exit) * 100.0;
                if discount_pct < REENTRY_MIN_DISCOUNT_TO_LAST_EXIT_PCT {
                    if is_debug_entry_enabled() {
                        log(
                            LogTag::Entry,
                            "REENTRY_DISCOUNT_INSUFFICIENT",
                            &format!(
                                "{} reentry discount {:.2}% < {:.2}% last_exit={:.9} cur={:.9} prior_count={}",
                                price_info.mint,
                                discount_pct,
                                REENTRY_MIN_DISCOUNT_TO_LAST_EXIT_PCT,
                                last_exit,
                                current_price,
                                prior_count
                            )
                        );
                    }
                    return (
                        false,
                        18.0,
                        format!(
                            "Re-entry discount too small {:.2}% < {:.2}%",
                            discount_pct,
                            REENTRY_MIN_DISCOUNT_TO_LAST_EXIT_PCT
                        ),
                    );
                }
            }
        }
    }

    // Before detecting best drop, log unreachable adaptive min drop
    if adaptive_min_drop > MAX_DROP_PERCENT {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "ADAPT_MIN_DROP_TOO_HIGH",
                &format!(
                    "{} adaptive_min_drop {:.1}% exceeds MAX {:.1}% prior_exits:{}",
                    price_info.mint,
                    adaptive_min_drop,
                    MAX_DROP_PERCENT,
                    prior_count
                )
            );
        }
    }

    // Detect best drop across windows with adaptive minimum
    let best = {
        let raw = detect_best_drop(&converted_history, current_price);
        match raw {
            Some(sig) if sig.drop_percent >= adaptive_min_drop => Some(sig),
            _ => None,
        }
    };

    if let Some(sig) = best {
        let mut confidence = 20.0;
        let drop_score = calculate_drop_magnitude_score(sig.drop_percent);
        confidence += drop_score * 25.0;
        // Transaction activity (single application)
        confidence += activity_score * 15.0; // single application

        // ATH Prevention Analysis
        let (ath_safe, max_ath_pct) = check_ath_risk(&converted_history, current_price).await;
        if !ath_safe {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "ATH_PREVENTION_SCALP",
                    &format!(
                        "❌ {} ATH prevention: {:.1}% of recent high - blocking entry",
                        price_info.mint,
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

        // Transaction activity already applied above - removed duplicate

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

        // Conservative entry conditions (adaptive)
        let is_good_entry =
            sig.drop_percent >= adaptive_min_drop &&
            sig.drop_percent <= 20.0 + ((prior_count as f64) * 3.0).min(15.0) &&
            sig.window_sec >= 60 &&
            activity_score >= 0.6 &&
            ath_safe &&
            stabilization_score >= 0.25; // require some stabilization
        if is_good_entry {
            confidence *= 1.25; // 25% boost for good conservative conditions
        }

        // Stabilization influence: boost if stabilized, penalize if volatile
        if stabilization_score >= 0.55 {
            confidence += 6.0;
        } else if stabilization_score < 0.2 {
            confidence -= 5.0;
        }
        if intrarange_pct > REENTRY_LOCAL_MAX_VOLATILITY_PCT {
            confidence -= 4.0;
        }

        confidence = confidence.clamp(0.0, 95.0);

        // Apply learner confidence boost
        let learning = get_learning_integration();
        let original_confidence = confidence;
        let adjustment = learning.get_entry_confidence_adjustment(
            &price_info.mint,
            current_price,
            sig.drop_percent,
            max_ath_pct
        ).await;
        confidence = (confidence * adjustment).clamp(0.0, 95.0);

        if is_debug_entry_enabled() && (adjustment - 1.0).abs() > 0.05 {
            log(
                LogTag::Entry,
                "LEARNER_CONFIDENCE_BOOST",
                &format!(
                    "🧠 {} learner adjustment: {:.1}% → {:.1}% (multiplier: {:.2}x)",
                    price_info.mint,
                    original_confidence,
                    confidence,
                    adjustment
                )
            );
        }

        // Conservative entry confidence threshold
        let approved = confidence >= 45.0; // Increased from 35.0% for higher quality entries

        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "ENTRY_ANALYSIS_COMPLETE",
                &format!(
                    "🎯 {} entry: -{:.1}%/{}s → conf {:.1}% → {} [activity:{:.1} ath_safe:{} ath_pct:{:.1}% good_entry:{} prior_exits:{} adapt_min_drop:{:.1}% stab:{:.2} intrarange:{:.1}%]",
                    price_info.mint,
                    sig.drop_percent,
                    sig.window_sec,
                    confidence,
                    if approved {
                        "ENTER"
                    } else {
                        "REJECT"
                    },
                    activity_score,
                    ath_safe,
                    max_ath_pct,
                    is_good_entry,
                    prior_count,
                    adaptive_min_drop,
                    stabilization_score,
                    intrarange_pct
                )
            );
        }

        return (
            approved,
            confidence,
            if approved {
                let reason = if is_good_entry {
                    format!(
                        "Conservative entry: -{:.1}%/{}s, ATH-safe, good activity",
                        sig.drop_percent,
                        sig.window_sec
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
                    "Entry analysis: -{:.1}%/{}s, conf {:.1}% < 45%, ATH: {:.1}%, activity: {:.1}, prior_exits:{}, adapt_min_drop:{:.1}%, stab:{:.2} intrarange:{:.1}%",
                    sig.drop_percent,
                    sig.window_sec,
                    confidence,
                    max_ath_pct,
                    activity_score,
                    prior_count,
                    adaptive_min_drop,
                    stabilization_score,
                    intrarange_pct
                )
            },
        );
    } else {
        if is_debug_entry_enabled() {
            // Log last few prices to understand zeros
            let recent_prices: Vec<String> = converted_history
                .iter()
                .take(5)
                .map(|(ts, price)| format!("{:.9}@{}", price, ts.format("%H:%M:%S")))
                .collect();
            log(
                LogTag::Entry,
                "NO_DROP_DETECTED",
                &format!(
                    "❌ {} no entry drops >= adapt_min_drop {:.1}% (base {:.1}%) detected in windows [{}] prior_exits:{}",
                    price_info.mint,
                    adaptive_min_drop,
                    MIN_DROP_PERCENT,
                    recent_prices.join(", "),
                    prior_count
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
/// OPTIMIZED: Single pass through price history with pre-computed time boundaries
async fn check_ath_risk(price_history: &[(DateTime<Utc>, f64)], current_price: f64) -> (bool, f64) {
    let now = Utc::now();
    let mut max_ath_percentage: f64 = 0.0;
    let mut near_ath = false;

    // Pre-compute time boundaries
    let cutoff_15min = now - chrono::Duration::seconds(ATH_LOOKBACK_15MIN);
    let cutoff_1hr = now - chrono::Duration::seconds(ATH_LOOKBACK_1HR);
    let cutoff_6hr = now - chrono::Duration::seconds(ATH_LOOKBACK_6HR);

    // Single pass through price history to collect all timeframe data
    let mut high_15min = 0.0f64;
    let mut high_1hr = 0.0f64;
    let mut high_6hr = 0.0f64;
    let mut count_15min = 0;
    let mut count_1hr = 0;
    let mut count_6hr = 0;

    for (ts, price) in price_history.iter() {
        if *price <= 0.0 || !price.is_finite() {
            continue;
        }

        if *ts >= cutoff_15min {
            high_15min = high_15min.max(*price);
            count_15min += 1;
        }
        if *ts >= cutoff_1hr {
            high_1hr = high_1hr.max(*price);
            count_1hr += 1;
        }
        if *ts >= cutoff_6hr {
            high_6hr = high_6hr.max(*price);
            count_6hr += 1;
        }
    }

    // Check 15min ATH
    if count_15min >= 3 && high_15min > 0.0 {
        let ath_pct = current_price / high_15min;
        max_ath_percentage = max_ath_percentage.max(ath_pct);
        if ath_pct >= ATH_THRESHOLD_15MIN {
            near_ath = true;
        }
    }

    // Check 1hr ATH
    if count_1hr >= 5 && high_1hr > 0.0 {
        let ath_pct = current_price / high_1hr;
        max_ath_percentage = max_ath_percentage.max(ath_pct);
        if ath_pct >= ATH_THRESHOLD_1HR {
            near_ath = true;
        }
    }

    // Check 6hr ATH
    if count_6hr >= 10 && high_6hr > 0.0 {
        let ath_pct = current_price / high_6hr;
        max_ath_percentage = max_ath_percentage.max(ath_pct);
        if ath_pct >= ATH_THRESHOLD_6HR {
            near_ath = true;
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

// =============================================================================
// PROFIT TARGET CALCULATION
// =============================================================================

/// Calculate profit targets optimized for fast scalping (5-10% minimum focus)
/// OPTIMIZED: Accept price_history to avoid redundant fetches
pub async fn get_profit_target(
    price_info: &PriceResult,
    price_history_opt: Option<&[(DateTime<Utc>, f64)]>
) -> (f64, f64) {
    let current_price_opt = Some(price_info.price_sol);

    let activity_score = 0.5;

    // Base profit targets with conservative approach for better success rates
    let (mut min_profit, mut max_profit): (f64, f64) = (18.0, 45.0); // Conservative default targets

    // Activity multiplier (reduced impact for conservative approach)
    let activity_multiplier = 1.0 + (activity_score - 0.5) * 0.25; // Reduced from 0.4 to 0.25
    min_profit *= activity_multiplier;
    max_profit *= activity_multiplier;

    // Volatility-based adjustment using provided price history
    if current_price_opt.is_some() && price_history_opt.is_some() {
        let price_history = price_history_opt.unwrap();

        if price_history.len() >= 5 {
            let now = Utc::now();
            let prices_5min: Vec<f64> = price_history
                .iter()
                .filter(|(ts, _)| (now - *ts).num_seconds() <= 300) // 5min window for more stability
                .map(|(_, p)| *p)
                .collect();
            if prices_5min.len() >= 3 {
                let high_5min = prices_5min.iter().fold(0.0f64, |a, b| a.max(*b));
                let low_5min = prices_5min.iter().fold(f64::INFINITY, |a, b| a.min(*b));
                if
                    high_5min.is_finite() &&
                    low_5min.is_finite() &&
                    high_5min > 0.0 &&
                    low_5min > 0.0
                {
                    let hl_range_5min = ((high_5min - low_5min) / high_5min) * 100.0;
                    let scale = (hl_range_5min / 50.0).clamp(0.0, 0.4); // Reduced scaling
                    min_profit *= 1.0 + scale * 0.3; // Reduced adjustment
                    max_profit *= 1.0 + scale * 0.4; // Reduced adjustment
                }
            }
        }
    }

    // Micro-liquidity capitulation profit expansion (re-detect cheaply)
    let mut micro_mode = false;
    if let Some(hist) = price_history_opt {
        if
            let Some((drop_percent, _recent_high, _holders)) = detect_micro_liquidity_capitulation(
                price_info,
                hist
            )
        {
            micro_mode = true;
            // Elevate targets aggressively for high-upside rebound plays
            // Keep base min profit at least 40%, max 120%+ (subject to clamps below)
            min_profit = min_profit.max(40.0).min(80.0);
            // Scale max by severity
            let severity_factor = ((drop_percent - MICRO_LIQ_MIN_DROP_PERCENT) / 5.0).clamp(
                0.0,
                1.0
            );
            let desired_max = 110.0 + severity_factor * 40.0; // 110% .. 150%
            max_profit = max_profit.max(desired_max).min(200.0);
        }
    }

    // Ensure conservative (or expanded) profit thresholds
    if micro_mode {
        min_profit = min_profit.clamp(35.0, 85.0);
        max_profit = max_profit.clamp(60.0, 200.0);
    } else {
        min_profit = min_profit.clamp(12.0, 40.0); // 12-40% minimum range (increased from 5-30%)
        max_profit = max_profit.clamp(25.0, 100.0); // 25-100% maximum range (increased from 15-80%)
    }

    // Ensure proper spread for conservative trading
    if max_profit - min_profit < 10.0 {
        // Maintain adequate spread (wider ceiling for micro mode)
        let ceiling = if micro_mode { 200.0 } else { 100.0 };
        max_profit = (min_profit + 10.0).min(ceiling);
    }

    if micro_mode && is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "MICRO_LIQ_TARGETS",
            &format!(
                "🎯 Micro-liq profit targets set: min {:.1}% max {:.1}% (sol_reserves {:.5})",
                min_profit,
                max_profit,
                price_info.sol_reserves
            )
        );
    }

    (min_profit, max_profit)
}
