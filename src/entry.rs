/// Conservative Drop Detector (balanced and stable)
///
/// OPTIMIZED FOR STABLE TRADING with 15-35% profit targets:
/// - Conservative 30s-10min detection windows (balanced approach)
/// - ATH prevention using multi-timeframe analysis
/// - Database-driven confidence scoring with stability weighting
/// - Higher confidence thresholds for quality entries
use crate::global::is_debug_entry_enabled;
use crate::logger::{log, LogTag};
use crate::pools::{check_price_history_quality, get_pool_price, get_price_history, PriceResult};
// use token store accessors directly as needed
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};
use tokio::sync::RwLock as AsyncRwLock; // switched from StdRwLock

// Lightweight TTL cache for recent exit prices to reduce DB pressure.
struct ExitPriceCacheEntry {
    prices: Vec<f64>,
    fetched_at: Instant,
}
static RECENT_EXIT_PRICE_CACHE: Lazy<
    AsyncRwLock<std::collections::HashMap<String, ExitPriceCacheEntry>>,
> = Lazy::new(|| AsyncRwLock::new(std::collections::HashMap::new()));
const EXIT_PRICE_CACHE_TTL: Duration = Duration::from_secs(30);
const EXIT_PRICE_CACHE_MAX_ENTRIES: usize = 1024; // prune safeguard

// Lightweight recent liquidity snapshots to infer short-term trend without external calls
#[derive(Clone, Copy)]
struct LiqSnap {
    sol: f64,
    price: f64,
    t: Instant,
}
static RECENT_LIQ_CACHE: Lazy<
    AsyncRwLock<std::collections::HashMap<String, std::collections::VecDeque<LiqSnap>>>,
> = Lazy::new(|| AsyncRwLock::new(std::collections::HashMap::new()));
const LIQ_CACHE_MAX_SNAPS: usize = 5;
const LIQ_CACHE_MAX_AGE: Duration = Duration::from_secs(5 * 60); // keep ~5 minutes

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
                if let Ok(rows) = db
                    .get_recent_closed_exit_prices_for_mint(mint, REENTRY_LOOKBACK_MAX)
                    .await
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
                    mapw.insert(
                        mint.clone(),
                        ExitPriceCacheEntry {
                            prices,
                            fetched_at: Instant::now(),
                        },
                    );

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
// Extended with ultra-short and longer slice to catch quick dips and orderly step-downs
const WINDOWS_SEC: [i64; 9] = [5, 8, 10, 20, 40, 80, 160, 320, 1200]; // 5s..20m windows
const MIN_DROP_PERCENT: f64 = 1.0; // base fallback; dynamic function overrides in logic
const MAX_DROP_PERCENT: f64 = 90.0; // Allow larger drops for volatile tokens

// ============================= POST-PEAK CASCADE GUARD =============================
// Blocks entries during liquidity-drain cascades (like ASTER) until reset conditions appear.
const CASCADE_LOOKBACK_M15: i64 = 900; // 15m
const CASCADE_LOOKBACK_H1: i64 = 3600; // 60m
const CASCADE_DROP_THRESHOLD_M15: f64 = 60.0; // ≥60% drop over 15m
const CASCADE_DROP_THRESHOLD_H1: f64 = 60.0; // ≥60% drop over 1h
const CASCADE_MDD_WINDOW_SEC: i64 = 180; // analyze last 3m micro-structure
const CASCADE_MDD_MIN_PCT: f64 = 30.0; // big drawdown from 3m high
const CASCADE_MRU_MAX_PCT: f64 = 3.0; // tiny run-up from 3m low → no bounce
const CASCADE_MIN_QUOTE_SOL: f64 = 50.0; // if below, treat as dangerous unless rising
const CASCADE_LIQ_FALLING_MIN_SNAPS: usize = 3; // require 3 decreasing snaps to consider falling

// ============================= PUMP FUN NEAR-20 SOL GUARD =============================
// Goal: Avoid catching continuing dumps around Pump Fun migration band (~20 SOL quote liquidity).
// Trigger only inside a narrow SOL reserve band; use cheap long-lookback proxy (h24 change from
// tokens cache) and corroborate with sell-dominance on 1h txns plus micro-structure weakness.
// DB read is optional and occurs only when in band to keep perf acceptable.
const PF_BAND_MIN_SOL: f64 = 18.0;
const PF_BAND_MAX_SOL: f64 = 26.0;
const PF_H24_DROP_EXTREME: f64 = -90.0; // ≤ -90% on h24 change implies post-peak context
const PF_M15_DROP_MIN: f64 = 35.0; // alternative recent drop gate
const PF_H1_DROP_MIN: f64 = 50.0;
const PF_SELL_DOM_H1_MIN: f64 = 2.0; // sells >= 2x buys
const PF_TXNS_H1_MIN_TOTAL: i64 = 6; // ensure some activity
const PF_MDD3M_MIN: f64 = 12.0; // still sliding within last 3m
const PF_MRU3M_MAX: f64 = 4.0; // lack of bounce in last 3m

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

// ============================= VOLATILITY CONTRACTION PATTERN =============================
// Detect volatility compression followed by expansion - a powerful entry pattern
const VCP_BASE_WINDOW_SEC: i64 = 240; // 4 minutes base window
const VCP_LOOKBACK_WINDOW_SEC: i64 = 120; // 2 minutes lookback window
const VCP_MIN_SAMPLES: usize = 10; // Minimum price points needed for detection
const VCP_MAX_VOLATILITY_PCT: f64 = 6.0; // Maximum range during contraction phase
const VCP_BREAKOUT_MIN_PCT: f64 = 3.0; // Minimum breakout percentage
const VCP_MIN_VOLATILITY_REDUCTION: f64 = 0.5; // Contraction must be at least 50% of previous volatility
const VCP_MIN_CONFIDENCE_BONUS: f64 = 15.0; // Base confidence bonus

// ============================= MEAN-REVERSION OVERSOLD ENTRY =============================
// Detect extremely oversold conditions with reversal signs
const OVERSOLD_MIN_DROP_PCT: f64 = 25.0; // Minimum initial drop percentage
const OVERSOLD_MAX_TIME_SEC: i64 = 300; // Maximum time window for oversold condition (5m)
const OVERSOLD_MIN_RECOVERY_PCT: f64 = 5.0; // Minimum recovery from low
const OVERSOLD_MAX_FROM_LOW_PCT: f64 = 15.0; // Maximum allowed recovery (avoid late entry)
const OVERSOLD_MIN_SAMPLES: usize = 8; // Minimum samples for detection
const OVERSOLD_MIN_CONFIDENCE_BONUS: f64 = 12.0; // Base confidence bonus

// ============================= LIQUIDITY ACCUMULATION ENTRY =============================
// Detect when liquidity is being accumulated before price moves up
const LIQ_ACCUM_MIN_INCREASE_PCT: f64 = 20.0; // Minimum liquidity increase percentage
const LIQ_ACCUM_MIN_SNAPS: usize = 3; // Minimum snapshots needed
const LIQ_ACCUM_MIN_SOL_RESERVES: f64 = 1.0; // Minimum SOL reserves to consider
const LIQ_ACCUM_MIN_PRICE_UPTICK_PCT: f64 = 2.0; // Minimum price uptick after accumulation
const LIQ_ACCUM_MIN_CONFIDENCE_BONUS: f64 = 12.0; // Base confidence bonus

// ============================= MICRO-LIQUIDITY CAPITULATION =============================
// Goal: Auto-approve high-upside entries when a token has near-zero real SOL liquidity and
// has experienced an extreme (>95-99%) capitulation from its recent local high while having
// a reasonable holder distribution (>= MICRO_LIQ_MIN_HOLDERS) to reduce pure honeypot risk.
// We rely ONLY on cached security data for holder counts (no fresh RPC in the hot loop).
// If holder count is unavailable we proceed (optimistic) but tag the reason accordingly.
const MICRO_LIQ_SOL_RESERVE_MAX: f64 = 0.001; // < 0.01 SOL reserves considered micro-liq
const MICRO_LIQ_MIN_DROP_PERCENT: f64 = 95.0; // At least 95% drop from recent high
const MICRO_LIQ_PREFERRED_DROP_PERCENT: f64 = 97.0; // Stronger confidence above this
const MICRO_LIQ_MIN_HOLDERS: u32 = 50; // Require >= 50 holders when info cached
const MICRO_LIQ_LOOKBACK_SECS: i64 = 900; // 15 min lookback window to find recent high
const MICRO_LIQ_CONF_BASE: f64 = 82.0; // Base confidence when triggered
const MICRO_LIQ_CONF_BONUS: f64 = 8.0; // Bonus if drop >= preferred threshold

async fn get_cached_holder_count_fast(_mint: &str) -> Option<u32> {
    // Security analyzer removed; no cached holder count available.
    None
}

async fn detect_micro_liquidity_capitulation(
    price_info: &PriceResult,
    history: &[(DateTime<Utc>, f64)],
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
                let holder_count = get_cached_holder_count_fast(&price_info.mint).await;
                // If holder count known, enforce threshold; else allow optimistic proceed.
                if holder_count
                    .map(|c| c >= MICRO_LIQ_MIN_HOLDERS)
                    .unwrap_or(true)
                {
                    return Some((drop_percent, recent_high, holder_count));
                }
            }
        }
    }
    None
}

// ============================= DYNAMIC THRESHOLDS =============================
// Minimum drop required scales by SOL liquidity and short-term volatility.
// High-liquidity pools can qualify on smaller pullbacks; thin pools need deeper.
fn dynamic_min_drop_percent(sol_reserves: f64, price_history: &[(DateTime<Utc>, f64)]) -> f64 {
    // Liquidity band factor
    let liq_factor = if sol_reserves >= 200.0 {
        0.6
    } else if sol_reserves >= 100.0 {
        0.75
    } else if sol_reserves >= 50.0 {
        0.9
    } else if sol_reserves >= 10.0 {
        1.0
    } else {
        1.2
    };

    // Volatility factor based on 2m high-low range
    let now = Utc::now();
    let prices_2m: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= 120)
        .map(|(_, p)| *p)
        .filter(|p| *p > 0.0 && p.is_finite())
        .collect();
    let vol_factor = if prices_2m.len() >= 3 {
        let hi = prices_2m.iter().fold(0.0f64, |a, b| a.max(*b));
        let lo = prices_2m.iter().fold(f64::INFINITY, |a, b| a.min(*b));
        if hi > 0.0 && lo.is_finite() && lo > 0.0 {
            let hl = ((hi - lo) / hi) * 100.0; // % range
                                               // Map 0..20% → 1.0..0.8 (more volatile → slightly lower min drop)
            (1.0 - (hl / 100.0).clamp(0.0, 0.2) * 1.0).max(0.8)
        } else {
            1.0
        }
    } else {
        1.0
    };

    let base = MIN_DROP_PERCENT; // 1.0%
    let dyn_val = base * liq_factor * vol_factor;
    dyn_val.clamp(0.5, 2.0) // Bound minimal sanity 0.5%..2.0%
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
pub const REENTRY_DROP_EXTRA_MAX_PCT: f64 = 10.0; // cap additional drop requirement (reduced from 14%)
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
    seconds: i64,
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
    let tight_factor =
        (1.0 - (intrarange_pct / REENTRY_LOCAL_MAX_VOLATILITY_PCT).clamp(0.0, 1.0)).max(0.0);
    let recovery_factor = if local_high > local_low {
        ((median_after - local_low) / (local_high - local_low)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let stabilization_score = (0.65 * tight_factor + 0.35 * recovery_factor).clamp(0.0, 1.0);
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
    intrarange_pct: f64,
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

    // If just above range high with strong momentum, allow continuation entries
    if current_price >= r_high * 1.004 {
        // momentum proxy: require stabilization and small micro-pullback
        if stabilization_score >= 0.35 && intrarange_pct <= MICRO_PULLBACK_MAX_PCT {
            return (true, 10.0, "TREND_CONTINUATION", true);
        }
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
    stabilization_score: f64,
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

// Detect Volatility Contraction Pattern (VCP) followed by breakout
// Returns (detected, confidence_bonus, reason)
fn detect_vcp_breakout(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
) -> (bool, f64, &'static str) {
    if price_history.len() < VCP_MIN_SAMPLES || current_price <= 0.0 || !current_price.is_finite() {
        return (false, 0.0, "");
    }

    let now = Utc::now();

    // Contraction phase prices (earlier window)
    let contraction_prices: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| {
            let age_sec = (now - *ts).num_seconds();
            age_sec > VCP_LOOKBACK_WINDOW_SEC
                && age_sec <= VCP_BASE_WINDOW_SEC + VCP_LOOKBACK_WINDOW_SEC
        })
        .map(|(_, p)| *p)
        .filter(|p| *p > 0.0 && p.is_finite())
        .collect();

    // Breakout phase prices (recent window)
    let breakout_prices: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| (now - *ts).num_seconds() <= VCP_LOOKBACK_WINDOW_SEC)
        .map(|(_, p)| *p)
        .filter(|p| *p > 0.0 && p.is_finite())
        .collect();

    if contraction_prices.len() < 5 || breakout_prices.len() < 3 {
        return (false, 0.0, "");
    }

    // Contraction volatility
    let contraction_high = contraction_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    let contraction_low = contraction_prices
        .iter()
        .fold(f64::INFINITY, |a, b| a.min(*b));
    if !contraction_high.is_finite()
        || !contraction_low.is_finite()
        || contraction_high <= 0.0
        || contraction_low <= 0.0
    {
        return (false, 0.0, "");
    }
    let contraction_volatility = ((contraction_high - contraction_low) / contraction_high) * 100.0;
    if !contraction_volatility.is_finite() || contraction_volatility > VCP_MAX_VOLATILITY_PCT {
        return (false, 0.0, "");
    }

    // Prior volatility to confirm contraction
    let prior_prices: Vec<f64> = price_history
        .iter()
        .filter(|(ts, _)| {
            let age_sec = (now - *ts).num_seconds();
            age_sec > VCP_BASE_WINDOW_SEC + VCP_LOOKBACK_WINDOW_SEC
                && age_sec <= 2 * VCP_BASE_WINDOW_SEC + VCP_LOOKBACK_WINDOW_SEC
        })
        .map(|(_, p)| *p)
        .filter(|p| *p > 0.0 && p.is_finite())
        .collect();
    if prior_prices.len() >= 5 {
        let prior_high = prior_prices.iter().fold(0.0f64, |a, b| a.max(*b));
        let prior_low = prior_prices.iter().fold(f64::INFINITY, |a, b| a.min(*b));
        if prior_high.is_finite() && prior_low.is_finite() && prior_high > 0.0 && prior_low > 0.0 {
            let prior_volatility = ((prior_high - prior_low) / prior_high) * 100.0;
            if contraction_volatility > prior_volatility * VCP_MIN_VOLATILITY_REDUCTION {
                return (false, 0.0, "");
            }
        }
    }

    // Verify breakout from contraction
    let breakout_high = breakout_prices.iter().fold(0.0f64, |a, b| a.max(*b));
    let min_breakout_price = contraction_high * (1.0 + VCP_BREAKOUT_MIN_PCT / 100.0);
    if current_price >= min_breakout_price && breakout_high >= min_breakout_price {
        let volatility_quality =
            (1.0 - contraction_volatility / VCP_MAX_VOLATILITY_PCT).clamp(0.0, 1.0);
        let breakout_strength =
            ((current_price / contraction_high - 1.0) * 100.0).clamp(3.0, 20.0) / 17.0;
        let confidence_bonus =
            VCP_MIN_CONFIDENCE_BONUS + 10.0 * volatility_quality + 12.0 * breakout_strength;
        return (true, confidence_bonus, "VCP_BREAKOUT");
    }

    (false, 0.0, "")
}

// Detect mean-reversion oversold entry (extreme drop + initial recovery)
// Returns (detected, confidence_bonus, reason)
fn detect_oversold_reversal(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
) -> (bool, f64, &'static str) {
    if price_history.len() < OVERSOLD_MIN_SAMPLES
        || current_price <= 0.0
        || !current_price.is_finite()
    {
        return (false, 0.0, "");
    }
    let now = Utc::now();
    let window: Vec<(DateTime<Utc>, f64)> = price_history
        .iter()
        .filter(|(ts, p)| {
            (now - *ts).num_seconds() <= OVERSOLD_MAX_TIME_SEC && *p > 0.0 && p.is_finite()
        })
        .cloned()
        .collect();
    if window.len() < OVERSOLD_MIN_SAMPLES {
        return (false, 0.0, "");
    }

    // Find high, low and their timestamps
    let mut high = 0.0;
    let mut high_ts = now;
    let mut low = f64::INFINITY;
    let mut low_ts = now;
    for (ts, p) in &window {
        if *p > high {
            high = *p;
            high_ts = *ts;
        }
        if *p < low {
            low = *p;
            low_ts = *ts;
        }
    }
    if !high.is_finite() || !low.is_finite() || high <= 0.0 || low <= 0.0 {
        return (false, 0.0, "");
    }
    if high_ts >= low_ts {
        return (false, 0.0, "");
    }

    let drop_pct = ((high - low) / high) * 100.0;
    if drop_pct < OVERSOLD_MIN_DROP_PCT {
        return (false, 0.0, "");
    }

    let recovery_pct = ((current_price - low) / low) * 100.0;
    if recovery_pct < OVERSOLD_MIN_RECOVERY_PCT || recovery_pct > OVERSOLD_MAX_FROM_LOW_PCT {
        return (false, 0.0, "");
    }

    let still_down_pct = ((high - current_price) / high) * 100.0;
    if still_down_pct < 10.0 {
        return (false, 0.0, "");
    }

    let drop_quality = (drop_pct / 50.0).clamp(0.5, 1.5);
    let recovery_quality = (recovery_pct / OVERSOLD_MAX_FROM_LOW_PCT).clamp(0.3, 0.9);
    let time_quality = 1.0 - (((now - low_ts).num_seconds() as f64) / 120.0).clamp(0.0, 0.8);
    let confidence_bonus = OVERSOLD_MIN_CONFIDENCE_BONUS
        + 10.0 * drop_quality
        + 5.0 * recovery_quality
        + 8.0 * time_quality;
    (true, confidence_bonus, "OVERSOLD_REVERSAL")
}

/// Main entry point for determining if a token should be bought
/// Returns (approved_for_entry, confidence_score, reason)
pub async fn should_buy(price_info: &PriceResult) -> (bool, f64, String) {
    // 1. Check if price history is available and sufficient for analysis
    match check_price_history_quality(&price_info.mint, 10, 120) {
        Ok(true) => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "INFO",
                    &format!(
                        "✅ Price history quality check passed for {}",
                        price_info.mint
                    ),
                );
            }
        }
        Ok(false) => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "WARN",
                    &format!("❌ {} insufficient price history quality", price_info.mint),
                );
            }
            return (false, 0.0, "Insufficient price history quality".to_string());
        }
        Err(e) => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "ERROR",
                    &format!("❌ {} price history check error: {}", price_info.mint, e),
                );
            }
            return (false, 0.0, format!("Price history check error: {}", e));
        }
    }

    // 2. Verify current price is fresh (same threshold as quality check)
    if price_info.is_stale(120) {
        // 2 minutes max age - consistent with quality check
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "WARN",
                &format!("❌ {} price is stale", price_info.mint),
            );
        }
        return (false, 0.0, "Price data is stale".to_string());
    }

    let price_history = get_price_history(&price_info.mint);

    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "DEBUG",
            &format!("✅ Using pool price for {}", price_info.mint),
        );
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
                &format!(
                    "❌ {} invalid price: {:.9}",
                    price_info.mint, price_info.price_sol
                ),
            );
        }
        return (false, 0.0, "Invalid price".to_string());
    };

    // Dynamic activity score based on cached token txns (m5 buys)
    let activity_score: f64 = {
        let txns_m5_buys: f64 = match crate::tokens::get_full_token_async(&price_info.mint).await {
            Ok(Some(token)) => token.txns_m5_buys.unwrap_or(0) as f64,
            _ => 0.0,
        };
        calculate_scalp_activity_score(txns_m5_buys)
    };

    // Get recent pool price history
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "HISTORY_REQUEST",
            &format!("📈 Using price history for {}", price_info.mint),
        );
    }

    // Fixed timestamp conversion - use the proper UTC timestamp method
    let mut converted_history: Vec<(DateTime<Utc>, f64)> = price_history
        .iter()
        .map(|p| (p.get_utc_timestamp(), p.price_sol))
        .collect();
    converted_history.retain(|(_, p)| *p > 0.0 && p.is_finite());

    // Record a liquidity snapshot for trend assessment (store price too)
    record_liquidity_snapshot(&price_info.mint, price_info.sol_reserves, current_price).await;

    // Early cascade guard (post-peak liquidity drain) — block entries in dump phase
    if converted_history.len() >= 6 {
        // Compute m15 and h1 changes using first vs last in those windows
        let now = Utc::now();
        let (mut first_15, mut last_15) = (None, None);
        let (mut first_60, mut last_60) = (None, None);
        for (ts, p) in converted_history.iter() {
            let age = (now - *ts).num_seconds();
            if age <= CASCADE_LOOKBACK_M15 {
                if first_15.is_none() {
                    first_15 = Some(*p);
                }
                last_15 = Some(*p);
            }
            if age <= CASCADE_LOOKBACK_H1 {
                if first_60.is_none() {
                    first_60 = Some(*p);
                }
                last_60 = Some(*p);
            }
        }
        let pct_change = |f: f64, l: f64| if f > 0.0 { ((l - f) / f) * 100.0 } else { 0.0 };
        let m15_change = match (first_15, last_15) {
            (Some(f), Some(l)) => pct_change(f, l),
            _ => 0.0,
        };
        let h1_change = match (first_60, last_60) {
            (Some(f), Some(l)) => pct_change(f, l),
            _ => 0.0,
        };

        // Micro-structure MDD/MRU over last 3m
        let (mdd_3m, mru_3m) =
            compute_mdd_mru(&converted_history, CASCADE_MDD_WINDOW_SEC).unwrap_or((0.0, 0.0));

        // Liquidity checks
        let liq_falling = is_liquidity_falling(&price_info.mint).await;
        let low_liq =
            price_info.sol_reserves > 0.0 && price_info.sol_reserves < CASCADE_MIN_QUOTE_SOL;

        let drop_criteria =
            m15_change <= -CASCADE_DROP_THRESHOLD_M15 || h1_change <= -CASCADE_DROP_THRESHOLD_H1;
        let microstruct_bad = mdd_3m >= CASCADE_MDD_MIN_PCT && mru_3m <= CASCADE_MRU_MAX_PCT;
        let liq_criteria = low_liq || liq_falling;

        if drop_criteria && microstruct_bad && liq_criteria {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "CASCADE_GUARD",
                    &format!(
                        "⛔ {} blocked by cascade guard: m15={:.1}% h1={:.1}% mdd3m={:.1}% mru3m={:.1}% sol_res={:.3} falling={}",
                        price_info.mint,
                        m15_change,
                        h1_change,
                        mdd_3m,
                        mru_3m,
                        price_info.sol_reserves,
                        liq_falling
                    )
                );
            }
            return (
                false,
                10.0,
                format!(
                    "Cascade guard: m15={:.1}% h1={:.1}% mdd3m={:.1}% mru3m={:.1}% liq={:.1} SOL",
                    m15_change, h1_change, mdd_3m, mru_3m, price_info.sol_reserves
                ),
            );
        }
    }

    // Pump Fun near-20 SOL band guard — lighter drop conditions + sell dominance corroboration
    // Only engages inside a tight SOL-quote band to avoid broad DB lookups.
    if price_info.sol_reserves >= PF_BAND_MIN_SOL
        && price_info.sol_reserves <= PF_BAND_MAX_SOL
        && converted_history.len() >= 6
    {
        let now = Utc::now();
        let (mut first_15, mut last_15) = (None, None);
        let (mut first_60, mut last_60) = (None, None);
        for (ts, p) in converted_history.iter() {
            let age = (now - *ts).num_seconds();
            if age <= CASCADE_LOOKBACK_M15 {
                if first_15.is_none() {
                    first_15 = Some(*p);
                }
                last_15 = Some(*p);
            }
            if age <= CASCADE_LOOKBACK_H1 {
                if first_60.is_none() {
                    first_60 = Some(*p);
                }
                last_60 = Some(*p);
            }
        }
        let pct_change = |f: f64, l: f64| if f > 0.0 { ((l - f) / f) * 100.0 } else { 0.0 };
        let m15_change = match (first_15, last_15) {
            (Some(f), Some(l)) => pct_change(f, l),
            _ => 0.0,
        };
        let h1_change = match (first_60, last_60) {
            (Some(f), Some(l)) => pct_change(f, l),
            _ => 0.0,
        };
        let (mdd_3m, mru_3m) =
            compute_mdd_mru(&converted_history, CASCADE_MDD_WINDOW_SEC).unwrap_or((0.0, 0.0));

        // Optional token market snapshot (price_change_h24 + txns_h1)
        let mut h24_change_opt: Option<f64> = None;
        let mut h1_buys: i64 = 0;
        let mut h1_sells: i64 = 0;
        if let Some(snap) = get_token_market_snapshot(&price_info.mint).await {
            h24_change_opt = snap.price_change_h24;
            h1_buys = snap.txns_h1_buys.unwrap_or(0);
            h1_sells = snap.txns_h1_sells.unwrap_or(0);
        }
        let total_h1 = h1_buys.saturating_add(h1_sells);
        let sell_dom = if h1_buys <= 0 {
            if h1_sells > 0 {
                10.0
            } else {
                0.0
            }
        } else {
            (h1_sells as f64) / (h1_buys as f64)
        };

        let long_lookback_bad = h24_change_opt
            .map(|c| c <= PF_H24_DROP_EXTREME)
            .unwrap_or(false);
        let recent_drop_bad = m15_change <= -PF_M15_DROP_MIN || h1_change <= -PF_H1_DROP_MIN;
        let micro_weak = mdd_3m >= PF_MDD3M_MIN && mru_3m <= PF_MRU3M_MAX;
        let sell_pressure_ok = total_h1 >= PF_TXNS_H1_MIN_TOTAL && sell_dom >= PF_SELL_DOM_H1_MIN;

        if (long_lookback_bad || recent_drop_bad) && micro_weak && sell_pressure_ok {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "PF_GUARD",
                    &format!(
                        "⛔ {} blocked by PF band guard: sol_res={:.2} h24={:?}% m15={:.1}% h1={:.1}% mdd3m={:.1}% mru3m={:.1}% h1_buys={} h1_sells={} sell_dom={:.2}",
                        price_info.mint,
                        price_info.sol_reserves,
                        h24_change_opt,
                        m15_change,
                        h1_change,
                        mdd_3m,
                        mru_3m,
                        h1_buys,
                        h1_sells,
                        sell_dom
                    )
                );
            }
            return (
                false,
                12.0,
                format!(
                    "PF band guard: liq {:.1} SOL, h24 {:?}%, m15 {:.1}%, h1 {:.1}%, mdd3m {:.1}%, mru3m {:.1}%, h1 sell-dom {:.2} ({}/{})",
                    price_info.sol_reserves,
                    h24_change_opt,
                    m15_change,
                    h1_change,
                    mdd_3m,
                    mru_3m,
                    sell_dom,
                    h1_sells,
                    h1_buys
                ),
            );
        }
    }

    // Micro-liquidity capitulation fast-path (before standard insufficient/history gates)
    if converted_history.len() >= 5 {
        // need minimal structure
        if let Some((drop_percent, recent_high, holder_opt)) =
            detect_micro_liquidity_capitulation(price_info, &converted_history).await
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
                ),
            );
        }

        if converted_history.len() >= 1 && current_price > 0.0 {
            let recent_price = converted_history[0].1;
            if recent_price > 0.0 && recent_price.is_finite() {
                let instant_drop = ((recent_price - current_price) / recent_price) * 100.0;
                if instant_drop >= 15.0 && instant_drop <= 75.0 {
                    // Higher minimum drop requirement for insufficient data
                    let confidence = (25.0 + instant_drop * 0.5).min(45.0_f64); // Conservative scaling
                    let final_confidence = confidence;

                    if final_confidence >= 35.0 {
                        if is_debug_entry_enabled() {
                            log(
                                LogTag::Entry,
                                "INSTANT_DROP_FALLBACK",
                                &format!(
                                    "🎯 {} instant drop -{:.1}% → conf {:.0}% → APPROVE",
                                    price_info.mint, instant_drop, final_confidence
                                ),
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
    let (_ll, _lh, stabilization_score, intrarange_pct) =
        analyze_local_structure(&converted_history, REENTRY_LOCAL_STABILITY_SECS);

    // ============================= Trend entries (conservative) =============================
    // Try to catch high-quality trend setups before standard drop-based logic.
    // 1) Breakout + Retest (enables ATH reclaim on success)
    let (brk_ok, brk_conf, brk_tag, brk_ath_bypass) = detect_breakout_retest(
        &converted_history,
        current_price,
        stabilization_score,
        intrarange_pct,
    );
    if brk_ok {
        let mut confidence = 30.0 + brk_conf; // strong base for quality trend
                                              // Activity boost (single use): conservative scaling
        confidence += activity_score * 12.0;
        confidence = confidence.clamp(0.0, 95.0);

        // Calculate drop percentage (use simple recent high approach)
        let trend_drop_percent = if converted_history.len() >= 10 {
            let recent_prices: Vec<f64> = converted_history
                .iter()
                .rev()
                .take(60) // Look back up to 60 price points
                .map(|(_, p)| *p)
                .collect();

            if let Some(&recent_high) = recent_prices
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
            {
                if recent_high > current_price && recent_high > 0.0 {
                    ((recent_high - current_price) / recent_high) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

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
                        price_info.mint, brk_tag, max_ath_pct
                    ),
                );
            }
        } else {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    brk_tag,
                    &format!(
                        "🎯 {} trend entry → conf {:.1}% (stab:{:.2} intrarange:{:.1}% ath_ok:{})",
                        price_info.mint, confidence, stabilization_score, intrarange_pct, ath_safe
                    ),
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
    let (rec_ok, rec_conf, rec_tag) =
        detect_sma_reclaim(&converted_history, current_price, stabilization_score);
    if rec_ok {
        let mut confidence = 26.0 + rec_conf;
        confidence += activity_score * 10.0;
        confidence = confidence.clamp(0.0, 95.0);

        let (ath_safe, max_ath_pct) = check_ath_risk(&converted_history, current_price).await;
        if !ath_safe {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "SMA_RECLAIM_ATH_BLOCK",
                    &format!(
                        "❌ {} {} blocked by ATH: {:.1}%",
                        price_info.mint, rec_tag, max_ath_pct
                    ),
                );
            }
        } else {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    rec_tag,
                    &format!(
                        "🎯 {} trend SMA reclaim → conf {:.1}% (stab:{:.2})",
                        price_info.mint, confidence, stabilization_score
                    ),
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

    // ============================= NEW ENTRY STRATEGIES =============================
    // 1) Volatility Contraction Pattern (VCP) breakout
    {
        let (vcp_ok, vcp_conf, vcp_tag) = detect_vcp_breakout(&converted_history, current_price);
        if vcp_ok {
            let mut confidence = 28.0 + vcp_conf; // Base confidence for VCP
            confidence += activity_score * 10.0; // Activity bonus
            confidence = confidence.clamp(0.0, 95.0);

            // ATH check
            let (ath_safe, max_ath_pct) = check_ath_risk(&converted_history, current_price).await;
            if !ath_safe {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        "VCP_ATH_BLOCK",
                        &format!(
                            "❌ {} {} blocked by ATH: {:.1}%",
                            price_info.mint, vcp_tag, max_ath_pct
                        ),
                    );
                }
            } else {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        vcp_tag,
                        &format!("🎯 {} VCP entry → conf {:.1}%", price_info.mint, confidence),
                    );
                }
                let approved = confidence >= 42.0;
                return (
                    approved,
                    confidence,
                    if approved {
                        "Volatility Contraction Pattern breakout entry".to_string()
                    } else {
                        format!("{}: confidence {:.1}% < 42%", vcp_tag, confidence)
                    },
                );
            }
        }
    }

    // 2) Mean-reversion oversold entry
    {
        let (oversold_ok, oversold_conf, oversold_tag) =
            detect_oversold_reversal(&converted_history, current_price);
        if oversold_ok {
            let mut confidence = 25.0 + oversold_conf; // Base confidence
            confidence += activity_score * 8.0; // reduced weight
            confidence = confidence.clamp(0.0, 95.0);

            // ATH check (lenient)
            let (ath_safe, max_ath_pct) = check_ath_risk(&converted_history, current_price).await;
            if !ath_safe && max_ath_pct > 75.0 {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        "OVERSOLD_ATH_BLOCK",
                        &format!(
                            "❌ {} {} blocked by extreme ATH: {:.1}%",
                            price_info.mint, oversold_tag, max_ath_pct
                        ),
                    );
                }
            } else {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        oversold_tag,
                        &format!(
                            "🎯 {} Oversold reversal entry → conf {:.1} pct (lenient ATH)",
                            price_info.mint, confidence
                        ),
                    );
                }
                let approved = confidence >= 40.0;
                return (
                    approved,
                    confidence,
                    if approved {
                        "Oversold reversal entry".to_string()
                    } else {
                        format!("{}: confidence {:.1}% < 40%", oversold_tag, confidence)
                    },
                );
            }
        }
    }

    // 3) Liquidity accumulation entry
    {
        let (liq_ok, liq_conf, liq_tag, liq_accum_pct) =
            detect_liquidity_accumulation(&price_info.mint, current_price, price_info.sol_reserves)
                .await;
        if liq_ok {
            let mut confidence = 26.0 + liq_conf;
            confidence += activity_score * 6.0;
            confidence = confidence.clamp(0.0, 95.0);

            let (ath_safe, max_ath_pct) = check_ath_risk(&converted_history, current_price).await;
            if !ath_safe {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        "LIQ_ACCUM_ATH_BLOCK",
                        &format!(
                            "❌ {} {} blocked by ATH: {:.1}%",
                            price_info.mint, liq_tag, max_ath_pct
                        ),
                    );
                }
            } else {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        liq_tag,
                        &format!(
                            "🎯 {} Liquidity accumulation +{:.1}% → conf {:.1}% (sol:{:.3})",
                            price_info.mint, liq_accum_pct, confidence, price_info.sol_reserves
                        ),
                    );
                }
                let approved = confidence >= 42.0;
                return (
                    approved,
                    confidence,
                    if approved {
                        format!(
                            "Liquidity accumulation entry (+{:.1}% SOL reserves)",
                            liq_accum_pct
                        )
                    } else {
                        format!(
                            "{}: confidence {:.1}% < 42%, accum +{:.1}%",
                            liq_tag, confidence, liq_accum_pct
                        )
                    },
                );
            }
        }
    }

    // Adaptive extra drop requirement for re-entry scenarios
    // Base dynamic minimum drop based on liquidity and short-term volatility
    let dyn_min_drop = dynamic_min_drop_percent(price_info.sol_reserves, &converted_history);
    let mut adaptive_min_drop = dyn_min_drop;
    if prior_count > 0 {
        let extra = ((prior_count as f64) * REENTRY_DROP_EXTRA_PER_ENTRY_PCT)
            .min(REENTRY_DROP_EXTRA_MAX_PCT);
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
                            discount_pct, REENTRY_MIN_DISCOUNT_TO_LAST_EXIT_PCT
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
                    price_info.mint, adaptive_min_drop, MAX_DROP_PERCENT, prior_count
                ),
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
                        price_info.mint, max_ath_pct
                    ),
                );
            }
            return (
                false,
                15.0,
                format!("ATH prevention: {:.1}% of recent high", max_ath_pct),
            );
        }

        // ATH safety bonus (conservative)
        confidence += 6.0; // Reduced from 8.0
        if max_ath_pct < 70.0 {
            confidence += 3.0; // Reduced from 5.0 for being well below highs
        }

        // Transaction activity already applied above - removed duplicate

        // Window preference (favor longer-term drops for stability)
        confidence += match sig.window_sec {
            30 => 20.0,  // Short-term but not too aggressive
            60 => 25.0,  // 1-minute window (good balance)
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
        let is_good_entry = sig.drop_percent >= adaptive_min_drop
            && sig.drop_percent <= 20.0 + ((prior_count as f64) * 3.0).min(15.0)
            && sig.window_sec >= 60
            && activity_score >= 0.6
            && ath_safe
            && stabilization_score >= 0.25; // require some stabilization
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
                        sig.drop_percent, sig.window_sec
                    )
                } else {
                    format!(
                        "Standard entry: -{:.1}%/{}s, ATH-safe (conf: {:.1}%)",
                        sig.drop_percent, sig.window_sec, confidence
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
                    dyn_min_drop,
                    recent_prices.join(", "),
                    prior_count
                )
            );
        }
        return (
            false,
            20.0,
            "No entry opportunity detected in conservative windows".to_string(),
        );
    }
}

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

// Record a quick liquidity snapshot for the mint (uses current PriceResult.sol_reserves)
async fn record_liquidity_snapshot(mint: &str, sol_reserves: f64, price: f64) {
    let mut map = RECENT_LIQ_CACHE.write().await;
    let q = map
        .entry(mint.to_string())
        .or_insert_with(|| std::collections::VecDeque::with_capacity(LIQ_CACHE_MAX_SNAPS));
    let now = Instant::now();
    // prune old
    while let Some(front) = q.front() {
        if now.duration_since(front.t) > LIQ_CACHE_MAX_AGE {
            q.pop_front();
        } else {
            break;
        }
    }
    // push
    if q.len() == LIQ_CACHE_MAX_SNAPS {
        q.pop_front();
    }
    q.push_back(LiqSnap {
        sol: sol_reserves.max(0.0),
        price: price.max(0.0),
        t: now,
    });
}

// Determine if liquidity is falling across recent snapshots (strict: monotonic decrease at least N steps)
async fn is_liquidity_falling(mint: &str) -> bool {
    let map = RECENT_LIQ_CACHE.read().await;
    if let Some(q) = map.get(mint) {
        if q.len() < CASCADE_LIQ_FALLING_MIN_SNAPS {
            return false;
        }
        let mut falls = 0usize;
        for i in 1..q.len() {
            if q[i].sol + 1e-9 < q[i - 1].sol {
                falls += 1;
            }
        }
        return falls + 1 >= CASCADE_LIQ_FALLING_MIN_SNAPS; // at least N decreasing steps
    }
    false
}

/// Detect significant liquidity accumulation followed by initial price movement
/// Returns (detected, confidence_bonus, reason, accum_percent)
async fn detect_liquidity_accumulation(
    mint: &str,
    current_price: f64,
    current_sol_reserves: f64,
) -> (bool, f64, &'static str, f64) {
    if current_price <= 0.0
        || !current_price.is_finite()
        || current_sol_reserves < LIQ_ACCUM_MIN_SOL_RESERVES
    {
        return (false, 0.0, "", 0.0);
    }
    let map = RECENT_LIQ_CACHE.read().await;
    if let Some(q) = map.get(mint) {
        if q.len() < LIQ_ACCUM_MIN_SNAPS {
            return (false, 0.0, "", 0.0);
        }
        let oldest_snap = q.front().unwrap();
        let newest_snap = q.back().unwrap();
        if newest_snap.sol <= oldest_snap.sol || oldest_snap.sol <= 0.0 {
            return (false, 0.0, "", 0.0);
        }
        let accum_percent = ((newest_snap.sol - oldest_snap.sol) / oldest_snap.sol) * 100.0;
        if !accum_percent.is_finite() || accum_percent < LIQ_ACCUM_MIN_INCREASE_PCT {
            return (false, 0.0, "", 0.0);
        }
        if newest_snap.price <= oldest_snap.price || oldest_snap.price <= 0.0 {
            return (false, 0.0, "", 0.0);
        }
        let price_change_pct =
            ((newest_snap.price - oldest_snap.price) / oldest_snap.price) * 100.0;
        if !price_change_pct.is_finite() || price_change_pct < LIQ_ACCUM_MIN_PRICE_UPTICK_PCT {
            return (false, 0.0, "", 0.0);
        }
        // Confidence estimation
        let liq_quality = (accum_percent / 50.0).clamp(0.4, 1.2);
        let price_quality = (price_change_pct / 10.0).clamp(0.2, 1.0);
        let size_quality = (current_sol_reserves / 20.0).clamp(0.1, 1.0);
        let confidence_bonus = LIQ_ACCUM_MIN_CONFIDENCE_BONUS
            + 12.0 * liq_quality
            + 8.0 * price_quality
            + 6.0 * size_quality;
        return (
            true,
            confidence_bonus,
            "LIQUIDITY_ACCUMULATION",
            accum_percent,
        );
    }
    (false, 0.0, "", 0.0)
}

// Compute MDD/MRU over a recent seconds window from history
fn compute_mdd_mru(price_history: &[(DateTime<Utc>, f64)], seconds: i64) -> Option<(f64, f64)> {
    let now = Utc::now();
    let mut high = 0.0f64;
    let mut low = f64::INFINITY;
    let mut cur = 0.0f64;
    let mut have = false;
    for (ts, p) in price_history.iter() {
        if (now - *ts).num_seconds() <= seconds && *p > 0.0 && p.is_finite() {
            high = high.max(*p);
            low = low.min(*p);
            cur = *p;
            have = true;
        }
    }
    if !have || high <= 0.0 || !high.is_finite() || !low.is_finite() || low <= 0.0 {
        return None;
    }
    let mdd = ((high - cur) / high) * 100.0;
    let mru = ((cur - low) / low) * 100.0;
    Some((mdd, mru))
}

// Lightweight accessor for cached token market snapshot (optional DB read).
// Called only inside a tight SOL band to keep overhead low.
struct TokenMarketSnapshot {
    price_change_h24: Option<f64>,
    txns_h1_buys: Option<i64>,
    txns_h1_sells: Option<i64>,
}

async fn get_token_market_snapshot(mint: &str) -> Option<TokenMarketSnapshot> {
    let token = crate::tokens::get_full_token_async(mint).await.ok()??;
    let price_change_h24 = token.price_change_h24;
    let txns_h1_buys = token.txns_h1_buys;
    let txns_h1_sells = token.txns_h1_sells;
    Some(TokenMarketSnapshot {
        price_change_h24,
        txns_h1_buys,
        txns_h1_sells,
    })
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

    let mut velocity_per_minute = percent_change / minutes;

    // Volume weighting: scale by recent 5m activity vs a soft baseline
    // We don't have mint context here; caller prefers adjusted velocity using global recent token snapshot.
    // As a lightweight proxy, use number of samples and clamp to [0.8, 1.2]. More samples -> higher trust.
    if prices.len() >= 3 {
        let sample_factor = ((prices.len() as f64) / 12.0).clamp(0.8, 1.2);
        velocity_per_minute *= sample_factor;
    }

    // Add debug logging to see what's happening - only for very large velocity changes
    if crate::global::is_debug_entry_enabled() && velocity_per_minute.abs() > 50.0 {
        crate::logger::log(
            crate::logger::LogTag::Entry,
            "VELOCITY_CALC",
            &format!(
                "Velocity calc: first={:.9}, last={:.9}, change={:.2}%/min over {:.1}min",
                first, last, velocity_per_minute, minutes
            ),
        );
    }

    velocity_per_minute // Percent per minute (volume-weighted proxy)
}

// Simple best-drop detector over predefined windows
fn detect_best_drop(
    price_history: &[(DateTime<Utc>, f64)],
    current_price: f64,
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
        if drop_percent <= 0.0 || drop_percent > MAX_DROP_PERCENT {
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
    price_history_opt: Option<&[(DateTime<Utc>, f64)]>,
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
                if high_5min.is_finite()
                    && low_5min.is_finite()
                    && high_5min > 0.0
                    && low_5min > 0.0
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
        if let Some((drop_percent, _recent_high, _holders)) =
            detect_micro_liquidity_capitulation(price_info, hist).await
        {
            micro_mode = true;
            // Elevate targets aggressively for high-upside rebound plays
            // Keep base min profit at least 40%, max 120%+ (subject to clamps below)
            min_profit = min_profit.max(40.0).min(80.0);
            // Scale max by severity
            let severity_factor =
                ((drop_percent - MICRO_LIQ_MIN_DROP_PERCENT) / 5.0).clamp(0.0, 1.0);
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
                min_profit, max_profit, price_info.sol_reserves
            ),
        );
    }

    (min_profit, max_profit)
}
