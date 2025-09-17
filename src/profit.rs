//! profit.rs
//!
//! Streamlined profit-taking & exit decision module.
//!
//! Goals & changes:
//! * Keep surface area small: single public async `should_sell(position, current_price)`.
//! * Add "time pressure" behaviour: as a position nears MAX_HOLD_MINUTES the system
//!   becomes more aggressive about exiting (tightens trailing, lowers odds threshold).
//! * Improve numeric stability and logging clarity.
//! * Keep primitives simple and well-documented.
//!
//! External expectations:
//! * `Position` type and `calculate_position_pnl(position, Some(price)).await` exist in `crate::positions`.
//! * `crate::logger::log` and `crate::global::is_debug_profit_enabled()` are available.

use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::positions::{ calculate_position_pnl, Position };
use chrono::Utc;
use once_cell::sync::Lazy;
use tokio::sync::RwLock as AsyncRwLock; // replaced StdRwLock
use std::time::{ Instant, Duration };

// ============================= ATH (Recent High Proximity) Adaptation =============================
// We incorporate 1m OHLCV recent highs to adapt exits when price is very near short / mid / long
// lookback highs. Near recent highs the probability of sharp rejection increases; we respond by:
// * Tightening trailing gaps further.
// * Slightly lowering max profit target (bank sooner) while keeping min intact.
// * Adding a small nudge to exit scoring under extreme proximity.
// Design is lightweight: single cached fetch per mint every ATH_CACHE_TTL_SEC seconds.

const ATH_WINDOW_15M_SECS: i64 = 15 * 60; // 15 minutes
const ATH_WINDOW_1H_SECS: i64 = 60 * 60; // 1 hour
const ATH_WINDOW_6H_SECS: i64 = 6 * 60 * 60; // 6 hours

// Proximity classification thresholds (distance from high as % of high)
const ATH_DIST_EXTREME: f64 = 1.5; // <=1.5% from recent 6h high -> extreme
const ATH_DIST_HIGH: f64 = 3.0; // <=3%  -> high
const ATH_DIST_ELEVATED: f64 = 5.0; // <=5%  -> elevated

// Effects (multipliers / reductions)
const ATH_TRAIL_TIGHTEN_EXTREME: f64 = 0.7; // 30% tighter
const ATH_TRAIL_TIGHTEN_HIGH: f64 = 0.8; // 20% tighter
const ATH_TRAIL_TIGHTEN_ELEV: f64 = 0.9; // 10% tighter
const ATH_TARGET_MAX_REDUCTION_EXTREME: f64 = 0.75; // reduce target_max by 25%
const ATH_TARGET_MAX_REDUCTION_HIGH: f64 = 0.85; // reduce by 15%
const ATH_TARGET_MAX_REDUCTION_ELEV: f64 = 0.92; // reduce by 8%
const ATH_SCORE_NUDGE_EXTREME: f64 = 0.25; // add to exit_score
const ATH_SCORE_NUDGE_HIGH: f64 = 0.15;
const ATH_SCORE_NUDGE_ELEV: f64 = 0.08;

const ATH_CACHE_TTL_SEC: u64 = 20; // refresh at most every 20s per mint
const ATH_MAX_OHLCV_POINTS: u32 = 400; // ~6h of 1m candles

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AthLevel {
    None,
    Elevated,
    High,
    Extreme,
}

#[derive(Clone, Debug)]
struct AthCached {
    last_fetch: Instant,
    high_15m: f64,
    high_1h: f64,
    high_6h: f64,
}

static ATH_CACHE: Lazy<AsyncRwLock<std::collections::HashMap<String, AthCached>>> = Lazy::new(||
    AsyncRwLock::new(std::collections::HashMap::new())
);

struct AthContext {
    level: AthLevel,
    distance_pct: f64, // distance from 6h high (percent)
    high_6h: f64,
    trail_factor: f64,
    target_max_factor: f64,
    score_nudge: f64,
}

impl Default for AthContext {
    fn default() -> Self {
        Self {
            level: AthLevel::None,
            distance_pct: f64::INFINITY,
            high_6h: 0.0,
            trail_factor: 1.0,
            target_max_factor: 1.0,
            score_nudge: 0.0,
        }
    }
}

async fn fetch_ath_context(mint: &str, current_price: f64) -> AthContext {
    if current_price <= 0.0 || !current_price.is_finite() {
        return AthContext::default();
    }
    {
        let map = ATH_CACHE.read().await;
        if let Some(c) = map.get(mint) {
            if c.last_fetch.elapsed() < Duration::from_secs(ATH_CACHE_TTL_SEC) {
                return build_ath_context_from_cached(c, current_price);
            }
        }
    }

    // 2. Refresh
    let ohlcv = match crate::tokens::ohlcvs::get_latest_ohlcv(mint, ATH_MAX_OHLCV_POINTS).await {
        Ok(v) => v,
        Err(_) => {
            return AthContext::default();
        }
    };
    if ohlcv.is_empty() {
        return AthContext::default();
    }
    let now_ts = chrono::Utc::now().timestamp();
    let mut high_15m = 0.0;
    let mut high_1h = 0.0;
    let mut high_6h = 0.0;
    for p in &ohlcv {
        let age = now_ts - p.timestamp;
        if age < 0 {
            continue;
        }
        if (age as i64) <= ATH_WINDOW_6H_SECS {
            if p.high > high_6h {
                high_6h = p.high;
            }
        }
        if (age as i64) <= ATH_WINDOW_1H_SECS {
            if p.high > high_1h {
                high_1h = p.high;
            }
        }
        if (age as i64) <= ATH_WINDOW_15M_SECS {
            if p.high > high_15m {
                high_15m = p.high;
            }
        }
    }
    // Fallback chaining (ensure non-zero if any window produced values)
    if high_15m == 0.0 {
        high_15m = high_1h.max(high_6h);
    }
    if high_1h == 0.0 {
        high_1h = high_6h.max(high_15m);
    }
    if high_6h == 0.0 {
        high_6h = high_1h.max(high_15m);
    }
    if high_6h <= 0.0 {
        return AthContext::default();
    }

    // Store cache
    {
        let mut mapw = ATH_CACHE.write().await;
        mapw.insert(mint.to_string(), AthCached {
            last_fetch: Instant::now(),
            high_15m,
            high_1h,
            high_6h,
        });
    }
    build_ath_context_from_cached(
        &(AthCached { last_fetch: Instant::now(), high_15m, high_1h, high_6h }),
        current_price
    )
}

fn build_ath_context_from_cached(c: &AthCached, current_price: f64) -> AthContext {
    let high = c.high_6h.max(c.high_1h).max(c.high_15m);
    if high <= 0.0 || !high.is_finite() || !current_price.is_finite() {
        return AthContext::default();
    }
    let distance_pct = if current_price >= high {
        0.0
    } else {
        ((high - current_price) / high) * 100.0
    };
    let (level, trail_factor, target_max_factor, score_nudge) = if distance_pct <= ATH_DIST_EXTREME {
        (
            AthLevel::Extreme,
            ATH_TRAIL_TIGHTEN_EXTREME,
            ATH_TARGET_MAX_REDUCTION_EXTREME,
            ATH_SCORE_NUDGE_EXTREME,
        )
    } else if distance_pct <= ATH_DIST_HIGH {
        (
            AthLevel::High,
            ATH_TRAIL_TIGHTEN_HIGH,
            ATH_TARGET_MAX_REDUCTION_HIGH,
            ATH_SCORE_NUDGE_HIGH,
        )
    } else if distance_pct <= ATH_DIST_ELEVATED {
        (
            AthLevel::Elevated,
            ATH_TRAIL_TIGHTEN_ELEV,
            ATH_TARGET_MAX_REDUCTION_ELEV,
            ATH_SCORE_NUDGE_ELEV,
        )
    } else {
        (AthLevel::None, 1.0, 1.0, 0.0)
    };
    AthContext { level, distance_pct, high_6h: high, trail_factor, target_max_factor, score_nudge }
}

// Re-entry adaptive exit cache: key = mint + entry_time_unix -> capped profit percent
static REENTRY_CAP_CACHE: Lazy<AsyncRwLock<std::collections::HashMap<String, f64>>> = Lazy::new(||
    AsyncRwLock::new(std::collections::HashMap::new())
);

/// Re-entry optimization tunables
/// Goal: On re-entering a token that previously had higher exit prices, avoid waiting
/// for unrealistic prior highs (downtrend bias). We cap expectations near the most
/// recent verified exits that are ABOVE our current entry price, with a discount.
pub const REENTRY_LOOKBACK_POSITIONS: usize = 5; // how many prior exits to inspect
pub const REENTRY_CLOSE_PRICE_PROXIMITY_PCT: f64 = 3.5; // within X% of prior exit => allow early bank
pub const REENTRY_CAP_DISCOUNT_PCT: f64 = 12.0; // reduce prior exit-based target by this percent
pub const REENTRY_MIN_PROFIT_OVERRIDE_PCT: f64 = 4.0; // ensure at least modest profit before triggering
pub const REENTRY_MAX_CAP_PCT: f64 = 45.0; // never cap above this (protect runners)
pub const REENTRY_ADDITIONAL_DISCOUNT_PER_PRIOR_PCT: f64 = 2.5; // extra discount per additional qualifying higher prior exit

/// ============================= Tunables =============================

// Loss & risk (percent) - Tighter for scalping

pub const STOP_LOSS_PERCENT: f64 = -30.0; // Tighter stop loss for scalping (was -40.0)
pub const EXTREME_LOSS_PERCENT: f64 = -45.0; // Emergency exit (was -55.0)

// Time (minutes) - Shorter hold times for scalping
pub const MAX_HOLD_MINUTES: f64 = 90.0; // Reduced from 120.0 for faster scalping

// Profit ladders (percent) - Aggressive but allow runners
pub const BASE_MIN_PROFIT_PERCENT: f64 = 5.0; // base min profit gate
pub const INSTANT_EXIT_LEVEL_1: f64 = 20.0; // level 1 now conditional (needs context)
pub const INSTANT_EXIT_LEVEL_2: f64 = 35.0; // still relatively low but gives room for 20-30% runners

// Trailing stop dynamics (tighter for scalping)
pub const TRAIL_MIN_GAP: f64 = 3.0; // Reduced from 5.0 for tighter trailing
pub const TRAIL_MAX_GAP: f64 = 20.0; // Reduced from 35.0 for scalping

// Trailing tighten schedule (faster for scalping)
pub const TRAIL_TIGHTEN_START: f64 = 30.0; // Reduced from 45.0
pub const TRAIL_TIGHTEN_FULL: f64 = 60.0; // Reduced from 90.0

// Odds model threshold (higher for scalping quality)
pub const EXIT_ODDS_THRESHOLD: f64 = 0.7; // Increased from 0.65

// Early-hold protection (seconds)
// Prevents non-emergency exits in the first moments after entry to avoid churn on noise.
// Quick-capture profit exits and extreme/stop-loss still apply.
pub const EARLY_HOLD_GRACE_SECS: f64 = 8.0; // Reduced to 8s for faster spike capture

// Quick capture windows (post initial fast tier phase): (minutes, required profit %)
// Only include mid/late windows not already covered by FAST_TIERS logic.
const QUICK_WINDOWS: &[(f64, f64)] = &[
    (3.0, 50.0), // 3 minutes: 50% profit
    (5.0, 60.0), // 5 minutes: 60% profit
    (8.0, 70.0), // 8 minutes: 70% profit
    (12.0, 80.0), // 12 minutes: 80% profit
];

/// Fast tier specification for unified early exit evaluation.
/// (max_minutes, min_profit_percent, tag, require_retrace_percent)
/// `require_retrace_percent` if >0 means we only exit if we've retraced at least that percent
/// from the local peak (peak_profit - pnl_percent >= retrace_threshold).
const FAST_TIERS: &[(f64, f64, &str, f64)] = &[
    // Allow very fast banking of extreme spikes, unconditional.
    (0.25, 40.0, "FAST_40PCT_SUB15S", 0.0),
    (0.5, 55.0, "FAST_55PCT_SUB30S", 0.0),
    // Moderate spikes: allow partial letting runners continue unless retrace occurs.
    (0.5, 25.0, "FAST_25PCT_COND", 4.0),
    (0.75, 35.0, "FAST_35PCT_COND", 5.0),
    (1.0, 45.0, "FAST_45PCT_COND", 6.0),
    // Extended early window up to 2m with conditional exits to still capture decays.
    (1.5, 55.0, "FAST_55PCT_COND_EXT", 8.0),
    (2.0, 65.0, "FAST_65PCT_COND_EXT", 10.0),
];

/// ============================= Helpers =============================

#[inline]
fn clamp01(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0).min(1.0) } else { 0.0 }
}

/// Compute (and cache) a re-entry adaptive profit cap for a position.
/// Logic:
/// 1. Fetch up to REENTRY_LOOKBACK_POSITIONS prior closed + verified positions for same mint.
/// 2. Filter those whose exit_price is > current entry_price (indicates we re-entered lower).
/// 3. Take the minimum of those qualifying exit prices (most conservative prior achieved level).
/// 4. Convert to percent profit relative to current entry.
/// 5. Apply discount REENTRY_CAP_DISCOUNT_PCT to reflect downtrend decay.
/// 6. Clamp to [REENTRY_MIN_PROFIT_OVERRIDE_PCT, REENTRY_MAX_CAP_PCT].
/// 7. Cache per (mint + entry_time) to avoid repeated DB hits.
async fn get_reentry_cap_percent(position: &Position) -> Option<f64> {
    use crate::positions::get_positions_database;
    // Build cache key
    let key = format!("{}:{}", position.mint, position.entry_time.timestamp());
    {
        let map = REENTRY_CAP_CACHE.read().await;
        if let Some(cached) = map.get(&key).cloned() {
            return Some(cached);
        }
    }

    // Need prior exits
    let db_lock = match get_positions_database().await {
        Ok(db) => db,
        Err(_) => {
            return None;
        }
    };
    let db_guard = db_lock.lock().await;
    let db = if let Some(ref db) = *db_guard {
        db
    } else {
        return None;
    };
    // Use lightweight price-only fetch to minimize DB parsing & lock time
    let recent_prices = match
        db.get_recent_closed_exit_prices_for_mint(&position.mint, REENTRY_LOOKBACK_POSITIONS).await
    {
        Ok(v) => v,
        Err(_) => {
            return None;
        }
    };
    if recent_prices.is_empty() {
        return None;
    }

    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return None;
    }

    // Collect exit prices above our entry (we re-entered lower than past exit)
    let mut higher_exits: Vec<f64> = recent_prices
        .into_iter()
        .filter_map(|(exit_p, eff_p)| eff_p.or(exit_p))
        .filter(|&ep| ep.is_finite() && ep > entry_price * 1.005)
        .collect();
    if higher_exits.is_empty() {
        return None;
    }

    // Use conservative minimum of higher exits
    higher_exits.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let reference_exit = higher_exits[0];
    let raw_profit_pct = ((reference_exit - entry_price) / entry_price) * 100.0;
    if raw_profit_pct <= 0.0 {
        return None;
    }
    // Add extra discount proportional to number of additional higher exits (excluding reference)
    let extra_discount =
        (higher_exits.len().saturating_sub(1) as f64) * REENTRY_ADDITIONAL_DISCOUNT_PER_PRIOR_PCT;
    let total_discount = (REENTRY_CAP_DISCOUNT_PCT + extra_discount).min(40.0); // cap total discount at 40%
    let discounted = (raw_profit_pct * (1.0 - total_discount / 100.0)).max(
        REENTRY_MIN_PROFIT_OVERRIDE_PCT
    );
    let capped = discounted.min(REENTRY_MAX_CAP_PCT);

    // Store (async lock)
    {
        let mut m = REENTRY_CAP_CACHE.write().await;
        m.insert(key, capped);
    }
    Some(capped)
}

/// Compute a dynamic trailing gap (in percentage points) from peak profit and time held.
///
/// - For non-positive peak profit we return TRAIL_MIN_GAP to keep protection tight.
/// - Gap grows with profit but is clamped to TRAIL_MIN_GAP..TRAIL_MAX_GAP.
/// - As time passes beyond TRAIL_TIGHTEN_START the gap reduces up to 30% by TRAIL_TIGHTEN_FULL.
/// - Returned value rounded to 2 decimals to reduce log churn.
pub fn trailing_gap(peak_profit: f64, minutes_held: f64) -> f64 {
    // Sanitize inputs
    let minutes = if minutes_held.is_finite() && minutes_held > 0.0 { minutes_held } else { 0.0 };

    if !peak_profit.is_finite() || peak_profit <= 0.0 {
        return TRAIL_MIN_GAP;
    }

    // Base: higher profits allow wider absolute gaps (but proportionally smaller).
    // We use piecewise factors to avoid wild swings in the small profit region.
    let mut gap = if peak_profit < 20.0 {
        // Slightly tighter for small-to-moderate peaks to reduce giveback
        peak_profit * 0.35
    } else if peak_profit < 50.0 {
        peak_profit * 0.25
    } else if peak_profit < 100.0 {
        peak_profit * 0.2
    } else {
        peak_profit * 0.16
    };

    // Clamp
    gap = gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP);

    // Time tightening: from TRAIL_TIGHTEN_START -> TRAIL_TIGHTEN_FULL reduce gap by up to 30%
    if minutes >= TRAIL_TIGHTEN_START && TRAIL_TIGHTEN_FULL > TRAIL_TIGHTEN_START {
        let progress = (
            (minutes - TRAIL_TIGHTEN_START) /
            (TRAIL_TIGHTEN_FULL - TRAIL_TIGHTEN_START)
        ).clamp(0.0, 1.0);
        let shrink = 0.3 * progress;
        gap *= 1.0 - shrink;
    }

    // Safety clamp + rounding to 2 decimals
    (gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP) * 100.0).round() / 100.0
}

/// A simple continuation odds estimator (0.0..1.0).
///
/// - Odds decay with holding time and current profit (diminishing returns).
/// - Very quick, large moves get a temporary early_boost.
pub fn continuation_odds(profit_percent: f64, minutes_held: f64) -> f64 {
    let minutes = if minutes_held.is_finite() && minutes_held >= 0.0 { minutes_held } else { 0.0 };

    // Minimal floor - a slight edge to holding small profiting positions
    if profit_percent <= 0.0 {
        return 0.55;
    }

    // Time decay: longer held -> exponentially smaller chance of another big leg
    let time_decay = (-minutes / 50.0).exp();

    // Profit decay: higher profit reduces odds of another big leg (non-linear)
    let profit_decay = (-(profit_percent / 120.0).powf(1.1)).exp();

    // Early boost for very fast moves
    let early_boost = if minutes < 5.0 && profit_percent > 40.0 { 0.1 } else { 0.0 };

    (time_decay * profit_decay + early_boost).clamp(0.0, 1.0)
}

/// ============================= Main Decision Function =============================

/// Decide whether to sell (`true`) or continue holding (`false`).
///
/// The function is intentionally conservative about selling below `BASE_MIN_PROFIT_PERCENT`.
/// Time pressure makes the system more aggressive as we approach `MAX_HOLD_MINUTES`.
pub async fn should_sell(position: &Position, current_price: f64) -> bool {
    // Basic validation
    if !current_price.is_finite() || current_price <= 0.0 {
        return false;
    }

    let entry = position.effective_entry_price.unwrap_or(position.entry_price);
    if !entry.is_finite() || entry <= 0.0 {
        return false;
    }

    // PnL calculation (support SIM/test tokens)
    let (pnl_sol, pnl_percent) = if position.symbol == "SIM" || position.mint == "SIM" {
        let pnl_percent = ((current_price - entry) / entry) * 100.0;
        let pnl_sol = if entry != 0.0 {
            ((current_price - entry) * position.entry_size_sol) / entry
        } else {
            0.0
        };
        (pnl_sol, pnl_percent)
    } else {
        // Note: calculate_position_pnl is expected to be async and return (sol_value, percent)
        calculate_position_pnl(position, Some(current_price)).await
    };

    if !pnl_percent.is_finite() || !pnl_sol.is_finite() {
        return false;
    }

    // Time held in minutes/seconds (clamp >= 0)
    let (minutes_held, seconds_held) = {
        let secs_i = (Utc::now() - position.entry_time).num_seconds();
        let secs = secs_i.max(0) as f64;
        (secs / 60.0, secs)
    };

    // Early-hold grace flag
    let early_hold_active = seconds_held < EARLY_HOLD_GRACE_SECS;

    // Peak profit (percentage)
    let highest = position.price_highest.max(current_price);
    let peak_profit = if entry > 0.0 { ((highest - entry) / entry) * 100.0 } else { 0.0 };

    // Drawdown from peak (how far we've come off the peak)
    let drawdown = peak_profit - pnl_percent;

    // Time pressure factor [0..1], increases as we approach the absolute MAX_HOLD_MINUTES
    let time_pressure = clamp01(minutes_held / MAX_HOLD_MINUTES);

    // If nearing the end, increase aggressiveness: reduce allowed gaps and reduce odds threshold.
    // We compute an adaptive odds threshold and a time-pressure multiplier for trailing gaps.
    let adaptive_odds_threshold = {
        // start reducing threshold moderately when time_pressure > 0.6
        if time_pressure <= 0.6 {
            EXIT_ODDS_THRESHOLD
        } else {
            // Move linearly down to 0.50 at full pressure (forces more exits)
            let factor = 1.0 - (time_pressure - 0.6) / (1.0 - 0.6); // 1 -> 0 as pressure increases
            let lowered = 0.5 + 0.15 * factor; // ranges roughly 0.50 .. 0.65
            lowered.clamp(0.45, EXIT_ODDS_THRESHOLD)
        }
    };

    let trailing_time_pressure_multiplier = 1.0 - time_pressure * 0.35; // up to 35% tighter trailing gaps

    // ============================= Profit Targets Integration (ATH Adaptive) =============================
    // Use position-specific targets when available. Keep it simple and local.
    let target_min = position.profit_target_min
        .unwrap_or(BASE_MIN_PROFIT_PERCENT)
        .clamp(1.0, 300.0);
    let mut target_max = position.profit_target_max
        .unwrap_or(INSTANT_EXIT_LEVEL_1)
        .clamp(1.0, 500.0);
    if target_max < target_min {
        target_max = (target_min + 1.0).min(500.0);
    }

    // Fetch ATH proximity context (cheap cached). We ONLY adapt if no custom user-provided max target.
    let mut ath_ctx: AthContext = AthContext::default();
    let ath_adapt_allowed = position.profit_target_max.is_none();
    if ath_adapt_allowed {
        ath_ctx = fetch_ath_context(&position.mint, current_price).await;
        if ath_ctx.level != AthLevel::None {
            // Apply target max reduction
            let reduced = (target_max * ath_ctx.target_max_factor).max(target_min + 0.5);
            if reduced < target_max {
                target_max = reduced;
            }
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "ATH_ADAPT",
                    &format!(
                        "{} ath_level={:?} dist={:.2}% tmax_adj={:.2}% trail_fac={:.2} score_nudge={:.2}",
                        position.symbol,
                        ath_ctx.level,
                        ath_ctx.distance_pct,
                        target_max,
                        ath_ctx.trail_factor,
                        ath_ctx.score_nudge
                    )
                );
            }
        }
    }

    // 1) Extreme loss immediate kill (no questions)
    if pnl_percent <= EXTREME_LOSS_PERCENT {
        return true;
    }

    // 2) Regular stop loss after short grace (avoid spurious micro-exits right at open)
    if pnl_percent <= STOP_LOSS_PERCENT && minutes_held >= 1.0 {
        return true;
    }

    // 2b) Time-scaled soft stop for positions that never showed a meaningful peak.
    // Rationale: If we haven't reached even a small positive peak, accept smaller losses sooner
    // to avoid being trapped for long periods waiting for the hard stop. This is deliberately
    // simple and avoids new functions/complexity.
    if peak_profit < 5.0 {
        let trigger =
            (minutes_held >= 6.0 && pnl_percent <= -18.0) ||
            (minutes_held >= 15.0 && pnl_percent <= -25.0) ||
            (minutes_held >= 30.0 && pnl_percent <= -32.0);
        if trigger {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "LOSS_SOFT_STOP",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% t={:.1}m (soft stop)",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 3) Absolute time cap: must close
    if minutes_held >= MAX_HOLD_MINUTES {
        return true;
    }

    // 3a) Unified fast tier evaluation (early spike handling with optional retrace requirement)
    if minutes_held <= 2.0 {
        // only evaluate during early phase
        for (max_min, min_profit, tag, retrace_req) in FAST_TIERS.iter() {
            if minutes_held <= *max_min && pnl_percent >= *min_profit {
                let retrace_ok = if *retrace_req <= 0.0 {
                    true
                } else {
                    peak_profit - pnl_percent >= *retrace_req
                };
                if retrace_ok {
                    if is_debug_profit_enabled() {
                        log(
                            LogTag::Profit,
                            *tag,
                            &format!(
                                "{} pnl={:.2}% peak={:.2}% dd={:.2}% t={:.3}m req={:.1}% retrace_req={:.1}%",
                                position.symbol,
                                pnl_percent,
                                peak_profit,
                                peak_profit - pnl_percent,
                                minutes_held,
                                min_profit,
                                retrace_req
                            )
                        );
                    }
                    return true;
                }
            }
        }
    }

    // 3c) Round-trip neutral exit: after a meaningful peak, if we round-trip back to ~flat,
    // prefer to exit instead of waiting for another full cycle.
    if peak_profit >= target_min && minutes_held >= 8.0 && pnl_percent <= 1.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "ROUND_TRIP_NEUTRAL_EXIT",
                &format!(
                    "{} pnl={:.2}% peak={:.2}% t={:.1}m",
                    position.symbol,
                    pnl_percent,
                    peak_profit,
                    minutes_held
                )
            );
        }
        return true;
    }

    // Re-entry adaptive cap (only if no custom profit targets provided on position)
    if position.profit_target_min.is_none() && position.profit_target_max.is_none() {
        if let Some(cap_pct) = get_reentry_cap_percent(position).await {
            if
                pnl_percent >= cap_pct &&
                !early_hold_active &&
                cap_pct >= REENTRY_MIN_PROFIT_OVERRIDE_PCT
            {
                if is_debug_profit_enabled() {
                    log(
                        LogTag::Profit,
                        "REENTRY_CAP_EXIT",
                        &format!(
                            "{} pnl={:.2}% >= reentry_cap={:.2}% (entry={:.6} peak={:.2}%)",
                            position.symbol,
                            pnl_percent,
                            cap_pct,
                            entry,
                            peak_profit
                        )
                    );
                }
                return true;
            }
        }
    }

    // 4)A Target-based take-profit: if we hit the configured maximum target, exit cleanly.
    if pnl_percent >= target_max {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "TARGET_MAX_HIT",
                &format!(
                    "{} pnl={:.2}% target_max={:.2}% t={:.1}m",
                    position.symbol,
                    pnl_percent,
                    target_max,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 4b) Exceptional profit immediate capture (overrides most other logic)
    // For truly exceptional moves, capture immediately regardless of timing
    if pnl_percent >= 200.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "EXCEPTIONAL_PROFIT_200PCT",
                &format!(
                    "{} pnl={:.2}% t={:.1}m (exceptional 200%+ capture)",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 4c) Very high profit timed capture
    if pnl_percent >= 150.0 && minutes_held <= 5.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "VERY_HIGH_PROFIT_150PCT",
                &format!(
                    "{} pnl={:.2}% t={:.1}m (150%+ timed capture)",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 4d) Quick capture windows (fast large moves)
    for (window_minutes, required_profit) in QUICK_WINDOWS {
        if minutes_held <= *window_minutes && pnl_percent >= *required_profit {
            return true;
        }
    }

    // 4e) Early moderate peak retention (handles 15-40% spikes better):
    // For peaks that are meaningful but below the "strong peak" threshold,
    // still protect against major givebacks to prevent round-trips to losses
    {
        let moderate_peak = peak_profit >= 15.0 && peak_profit < 40.0;
        let very_early = minutes_held <= 5.0; // very early in position lifecycle
        let retain_frac = 0.4; // keep at least 40% of peak (less strict than strong peaks)
        let must_retain = peak_profit * retain_frac;
        let significant_giveback = pnl_percent < must_retain;
        let meaningful_dd = peak_profit - pnl_percent >= 6.0; // smaller threshold for moderate peaks

        if moderate_peak && very_early && significant_giveback && meaningful_dd {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "MODERATE_PEAK_RETAIN_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% retain_req={:.2}% t={:.1}m (moderate peak protection)",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        must_retain,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 4f) Big-peak retention rule (anti-giveback):
    // If we printed a strong early peak, require we retain a reasonable fraction of it.
    // Rationale: after big impulse moves, reversals are violent; we prefer to bank a
    // chunk rather than round-trip. Tunables chosen conservatively.
    {
        let strong_peak = peak_profit >= 40.0; // "big first leg" heuristic
        let early_enough = minutes_held <= 15.0; // focus on the opening impulse
        let retain_frac = if peak_profit >= 80.0 { 0.55 } else { 0.5 }; // keep at least 50-55%
        let must_retain = peak_profit * retain_frac;
        let big_giveback = pnl_percent < must_retain;
        let decent_abs_dd = peak_profit - pnl_percent >= 10.0; // avoid triggering on tiny wobbles

        if strong_peak && early_enough && big_giveback && decent_abs_dd {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "BIG_PEAK_RETAIN_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% retain_req={:.2}% t={:.1}m",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        must_retain,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 4g) General peak retention rule (time-agnostic):
    // If we've printed a meaningful peak at any time, enforce retaining a reasonable fraction
    // to avoid multi-hour round trips. Less strict than the early rule above but always active.
    {
        let meaningful_peak = peak_profit >= 30.0;
        if meaningful_peak {
            let retain_frac = if minutes_held <= 20.0 {
                if peak_profit >= 80.0 { 0.6 } else { 0.5 }
            } else {
                if peak_profit >= 80.0 { 0.5 } else { 0.45 }
            };
            let must_retain = peak_profit * retain_frac;
            let big_giveback = pnl_percent < must_retain;
            let decent_abs_dd = peak_profit - pnl_percent >= 8.0;
            if big_giveback && decent_abs_dd {
                if is_debug_profit_enabled() {
                    log(
                        LogTag::Profit,
                        "PEAK_RETAIN_EXIT",
                        &format!(
                            "{} pnl={:.2}% peak={:.2}% retain_req={:.2}% t={:.1}m",
                            position.symbol,
                            pnl_percent,
                            peak_profit,
                            must_retain,
                            minutes_held
                        )
                    );
                }
                return true;
            }
        }
    }

    // 5) Instant large profits (level 2 unconditional; level 1 conditional to let strong trends continue)
    if pnl_percent >= INSTANT_EXIT_LEVEL_2 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "INSTANT_EXIT_L2",
                &format!("{} pnl={:.2}% t={:.1}m", position.symbol, pnl_percent, minutes_held)
            );
        }
        return true;
    }
    if pnl_percent >= INSTANT_EXIT_LEVEL_1 {
        // Require either some holding time OR a drawdown off the peak to avoid killing fresh momentum.
        let drawdown_from_peak = peak_profit - pnl_percent;
        if minutes_held >= 1.0 || drawdown_from_peak >= 4.0 {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "INSTANT_EXIT_L1",
                    &format!(
                        "{} pnl={:.2}% dd={:.2}% t={:.2}m",
                        position.symbol,
                        pnl_percent,
                        drawdown_from_peak,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 6) Adaptive minimum profit gate
    // We reduce the min threshold over time (so we can exit smaller wins later instead of round-tripping)
    // and also if volatility already produced a decent peak but we gave a lot back.
    let dynamic_min_profit = {
        // Base decays from target_min toward 0.5 * target_min over time pressure
        let decay_component = target_min * (1.0 - 0.5 * time_pressure);
        // After 25 minutes allow a further soft decay to encourage freeing capital
        let long_hold_bonus = if minutes_held > 25.0 { target_min * 0.2 } else { 0.0 };
        (decay_component - long_hold_bonus).max(3.0)
    };

    // Early round-trip protection (re-tuned):
    // Original logic was triggering too aggressively on modest early peaks (e.g. 12-18%) causing
    // exits at tiny retained profits after a quick spike + pullback. We now:
    // * Require a more meaningful peak (>= ~1.8x BASE) OR enough elapsed time before considering.
    // * Impose a warm-up (>= 2.5m) so very early noise doesn't force an exit.
    // * Demand a substantial absolute drawdown (>= 8%) AND large relative give-back (current < 25% of peak).
    // * Still cap window to first 10 minutes (after that trailing / odds logic govern).
    // * Ensure we aren't still sitting on a solid base profit (skip if pnl >= 0.8 * BASE_MIN_PROFIT_PERCENT).
    {
        let meaningful_peak = peak_profit >= target_min * 1.8; // scale with target
        let warmup_passed = minutes_held >= 2.5;
        let within_window = minutes_held <= 10.0; // tighter than prior 12m
        let large_absolute_drawdown = peak_profit - pnl_percent >= 8.0; // avoid micro noise
        let large_relative_giveback = pnl_percent < peak_profit * 0.25; // retain <25% of peak
        let not_still_ok_profit = pnl_percent < target_min * 0.8; // scale with target

        if
            meaningful_peak &&
            warmup_passed &&
            within_window &&
            large_absolute_drawdown &&
            large_relative_giveback &&
            not_still_ok_profit
        {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "EARLY_RETRACE_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% t={:.1}m retrace={:.2}% cond=mp:{} wa:{} win:{} abs_dd:{} rel:{} ok:{}",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        minutes_held,
                        peak_profit - pnl_percent,
                        meaningful_peak,
                        warmup_passed,
                        within_window,
                        large_absolute_drawdown,
                        large_relative_giveback,
                        not_still_ok_profit
                    )
                );
            }
            return true;
        }
    }

    if pnl_percent < dynamic_min_profit && peak_profit < target_min {
        // However, if time pressure is very high and we're above a small profit, close to avoid forced cap
        if time_pressure > 0.95 && pnl_percent > 2.0 {
            return true;
        }
        // Do not return here; allow trailing/retention/scoring to handle exits once we have a peak.
    }

    // 7) Trailing stop logic (dynamic)
    // Suppress trailing exits during the early-hold grace to avoid churn on tiny spikes.
    if peak_profit >= target_min && !early_hold_active {
        // baseline gap derived from peak profit and age
        let mut gap = trailing_gap(peak_profit, minutes_held);

        // Micro trailing for early profits:
        // - For modest peaks (10% - 25%): keep very tight to avoid full givebacks.
        // - For medium peaks (25% - 60%): still tighten a bit vs baseline.
        if peak_profit < 25.0 {
            let micro_gap = (peak_profit * 0.35).clamp(3.0, 8.0);
            if micro_gap < gap {
                gap = micro_gap;
            }
        } else if peak_profit < 60.0 {
            let micro_gap_mid = (peak_profit * 0.3).clamp(6.0, 12.0);
            if micro_gap_mid < gap {
                gap = micro_gap_mid;
            }
        }

        // Apply time-pressure multiplier (tighten gap as approaching max hold)
        gap *= trailing_time_pressure_multiplier;
        // Apply ATH tightening factor (if active)
        if ath_ctx.level != AthLevel::None {
            gap *= ath_ctx.trail_factor;
        }

        // Additional adaptive tightening if we've been showing large drawdown relative to gap
        if minutes_held > 30.0 && drawdown > gap * 0.6 {
            gap = (gap * 0.85).max(TRAIL_MIN_GAP);
        }

        // If drawdown exceeds dynamic gap -> sell
        if drawdown >= gap {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "TRAIL_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% drawdown={:.2}% gap={:.2}% t={:.1}m tp={:.2}",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        drawdown,
                        gap,
                        minutes_held,
                        time_pressure
                    )
                );
            }
            return true;
        }
    }

    // 8) Scoring-based exit decision (lightweight, reuses existing signals)
    // Suppress scoring exits during the early-hold grace; rely on quick-capture or hard stops.
    if early_hold_active {
        // Skip SCORE_EXIT during grace
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "GRACE_HOLD",
                &format!(
                    "{} pnl={:.2}% peak={:.2}% t={:.1}s — suppressing scoring/trailing exits",
                    position.symbol,
                    pnl_percent,
                    peak_profit,
                    seconds_held
                )
            );
        }
        return false;
    }
    // Score components: negative adds to exit_score; positive subtracts (hold bias).
    let odds = continuation_odds(pnl_percent, minutes_held);
    let future_gap =
        trailing_gap(pnl_percent.max(peak_profit), minutes_held) *
        trailing_time_pressure_multiplier;
    let potential_gain_ceiling = 200.0;
    let potential_gain = (potential_gain_ceiling - pnl_percent).max(0.0).min(100.0);
    let expected_edge = odds * potential_gain - (1.0 - odds) * future_gap;

    let mut exit_score = 0.0f64;

    // Drawdown vs dynamic gap: heavy weight
    if peak_profit >= target_min {
        let gap_now = trailing_gap(peak_profit, minutes_held) * trailing_time_pressure_multiplier;
        let dd_ratio = if gap_now > 0.0 { drawdown / gap_now } else { 0.0 };
        if dd_ratio >= 1.0 {
            exit_score += 2.0;
        } else if
            // would trigger trail
            dd_ratio >= 0.7
        {
            exit_score += 1.2;
        } else if dd_ratio >= 0.4 {
            exit_score += 0.6;
        }
    }

    // Odds and EV
    if odds < adaptive_odds_threshold {
        exit_score += (adaptive_odds_threshold - odds) * 1.5;
    }
    if expected_edge <= 0.0 {
        exit_score += (expected_edge.abs() / (future_gap + 1.0)).min(1.5);
    }

    // Time pressure
    exit_score += time_pressure * 0.8; // up to +0.8

    // Profit quality
    if pnl_percent < dynamic_min_profit {
        exit_score += 0.8;
    } // below our dynamic gate
    if pnl_percent >= INSTANT_EXIT_LEVEL_1 {
        exit_score += 1.0;
    }

    // Early retrace already handled above; add a small nudge if peak was meaningful and we gave back a lot
    if peak_profit >= 20.0 && drawdown >= peak_profit * 0.4 {
        exit_score += 0.7;
    }

    // Very small profits after long time -> nudge to close
    if minutes_held > 45.0 && pnl_percent < 5.0 {
        exit_score += 0.6;
    }

    // Flat after big peak for a while -> additional nudge
    if peak_profit >= target_min && minutes_held >= 12.0 && pnl_percent <= 1.0 {
        exit_score += 1.0;
    }

    // Holding bias when trend looks healthy (odds good and EV positive)
    if odds >= adaptive_odds_threshold && expected_edge > 0.0 {
        exit_score -= 0.6;
    }

    // Normalize and threshold
    let mut score_threshold = 1.2; // calibrated to act after 1-3 weak signals or one strong
    // ATH proximity adds a nudge to exit_score (risk of rejection) if adaptation allowed
    if ath_ctx.level != AthLevel::None {
        exit_score += ath_ctx.score_nudge;
    }
    // Lower threshold under high time pressure to favor exits as cap approaches
    score_threshold -= 0.2 * time_pressure; // down to ~1.0 at full pressure
    if exit_score >= score_threshold {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "SCORE_EXIT",
                &format!(
                    "{} score={:.2} thr={:.2} pnl={:.2}% peak={:.2}% dd={:.2}% odds={:.2} edge={:.2} gapF={:.2} t={:.1}m",
                    position.symbol,
                    exit_score,
                    score_threshold,
                    pnl_percent,
                    peak_profit,
                    drawdown,
                    odds,
                    expected_edge,
                    future_gap,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 9) Time pressure final nudge — if we are very close to MAX_HOLD_MINUTES and have any decent profit,
    // prefer to exit rather than risk forced closure on the cap.
    if time_pressure > 0.92 && pnl_percent > target_min {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "TIME_PRESSURE_EXIT",
                &format!(
                    "{} pnl={:.2}% t={:.1}m pressure={:.2}",
                    position.symbol,
                    pnl_percent,
                    minutes_held,
                    time_pressure
                )
            );
        }
        return true;
    }

    // Final debug logging for transparency
    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "DECISION",
            &format!(
                "{} pnl={:.2}% ({:.6} SOL) peak={:.2}% dd={:.2}% t={:.1}m tp={:.2} odds={:.2} edge={:.2} thr={:.2} dyn_min={:.2} tgt_min={:.2} tgt_max={:.2}",
                position.symbol,
                pnl_percent,
                pnl_sol,
                peak_profit,
                drawdown,
                minutes_held,
                time_pressure,
                odds,
                expected_edge,
                adaptive_odds_threshold,
                dynamic_min_profit,
                target_min,
                target_max
            )
        );
    }

    // Default: continue holding
    false
}
