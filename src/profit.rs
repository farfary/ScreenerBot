use crate::global::*;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use crate::tokens::{
    get_token_rugcheck_data_safe,
    is_token_safe_for_trading_safe,
    get_high_risk_issues,
    TokenDatabase,
    pool::get_pool_service,
};
use chrono::Utc;
use serde::{ Serialize, Deserialize };

// ================================================================================================
// 🎯 NEXT-GENERATION INTELLIGENT PROFIT SYSTEM
// ================================================================================================
// Risk-based profit scaling with real-time token analysis
// Combines Rugcheck security data + Token API data + Market momentum
// Dynamic profit targets: 10% (safe) to 10,000% (dangerous)
// Time pressure: 10-45 minutes based on safety level
// Smart exit strategies based on liquidity, volume, and social proof
//
// ⚡ FAST PROFIT-TAKING OPTIMIZATION:
// - >3% profit in <1 minute = immediate exit (captures quick momentum)
// - >5% profit in <30 seconds = ultra-fast exit (exceptional momentum)
// - Prevents profit reversal on fast-moving tokens
// ================================================================================================

// 🔒 STOP LOSS PROTECTION - INTELLIGENT LOSS MANAGEMENT
pub const STOP_LOSS_PERCENT: f64 = -55.0; // Intelligent stop loss at -55%

// ⏰ OPTIMIZED HOLD TIMES BY SAFETY LEVEL (MINUTES)
// EXTENDED FOR BETTER PROFITS: 1 minute to 2 hours based on liquidity
const ULTRA_SAFE_MAX_TIME: f64 = 120.0; // Ultra safe tokens - 2 hours max
const SAFE_MAX_TIME: f64 = 90.0; // Safe tokens - 1.5 hours
const MEDIUM_MAX_TIME: f64 = 60.0; // Medium risk tokens - 1 hour
const RISKY_MAX_TIME: f64 = 45.0; // Risky tokens - 45 minutes
const DANGEROUS_MAX_TIME: f64 = 30.0; // Dangerous tokens - 30 minutes
const MIN_HOLD_TIME: f64 = 1.0; // Minimum hold time for all positions

// 🎯 OPTIMIZED PROFIT TARGETS - CORRECTED RISK-BASED STRATEGY
// LOWER RISK = HIGHER TARGETS + LONGER TIME (more patience for safer tokens)
// HIGHER RISK = LOWER TARGETS + SHORTER TIME (quick exits for dangerous tokens)
const ULTRA_SAFE_PROFIT_MIN: f64 = 8.0; // 8-500% for ultra safe tokens (can afford to wait)
const ULTRA_SAFE_PROFIT_MAX: f64 = 500.0;
const SAFE_PROFIT_MIN: f64 = 6.0; // 6-300% for safe tokens
const SAFE_PROFIT_MAX: f64 = 300.0;
const MEDIUM_PROFIT_MIN: f64 = 5.0; // 5-200% for medium risk tokens
const MEDIUM_PROFIT_MAX: f64 = 200.0;
const RISKY_PROFIT_MIN: f64 = 3.0; // 3-100% for risky tokens (faster exits)
const RISKY_PROFIT_MAX: f64 = 100.0;
const DANGEROUS_PROFIT_MIN: f64 = 2.0; // 2-50% for dangerous tokens (very fast exits)
const DANGEROUS_PROFIT_MAX: f64 = 50.0;

// 📈 TRAILING STOP CONFIGURATION - RISK-ADJUSTED
const USE_TRAILING_STOP: bool = true;
// Different trailing stops based on safety level
const TRAILING_STOP_ULTRA_SAFE: f64 = 12.0; // 12% for ultra safe (more tolerance)
const TRAILING_STOP_SAFE: f64 = 10.0; // 10% for safe
const TRAILING_STOP_MEDIUM: f64 = 8.0; // 8% for medium
const TRAILING_STOP_RISKY: f64 = 6.0; // 6% for risky (tighter stops)
const TRAILING_STOP_DANGEROUS: f64 = 4.0; // 4% for dangerous (very tight)
const TIME_DECAY_FACTOR: f64 = 0.15; // More aggressive time decay (15% vs 5%)

// 🚀 INSTANT SELL THRESHOLDS - CAPTURE MOONSHOTS
const INSTANT_SELL_PROFIT: f64 = 2000.0; // 2000%+ = instant sell
const MEGA_PROFIT_THRESHOLD: f64 = 1000.0; // 1000%+ = very urgent

// ⚡ FAST PROFIT-TAKING THRESHOLDS - RESPECTS MINIMUM HOLD TIME
// FIXED: No override of MIN_HOLD_TIME - fast profits only apply AFTER minimum hold
const FAST_PROFIT_THRESHOLD: f64 = 3.0; // 3%+ profit at 1+ minute = fast exit
const FAST_PROFIT_TIME_LIMIT: f64 = 1.0; // Must be held for 1+ minute minimum
const SPEED_PROFIT_THRESHOLD: f64 = 5.0; // 5%+ profit at 1+ minute = speed exit
const SPEED_PROFIT_TIME_LIMIT: f64 = 1.0; // Changed from 0.5 to respect MIN_HOLD_TIME
const MOMENTUM_MIN_TIME_SECONDS: f64 = 5.0; // Minimum 5 seconds before momentum calculation

// 📊 LIQUIDITY THRESHOLDS FOR PROFIT CALCULATIONS AND SAFETY CLASSIFICATION
const PROFIT_HIGH_LIQUIDITY_THRESHOLD: f64 = 200_000.0; // For profit calculations
const PROFIT_MEDIUM_HIGH_LIQUIDITY_THRESHOLD: f64 = 100_000.0; // For profit calculations
const PROFIT_MEDIUM_LIQUIDITY_THRESHOLD: f64 = 50_000.0; // For profit calculations
const PROFIT_LOW_LIQUIDITY_THRESHOLD: f64 = 10_000.0; // For profit calculations

// 🔍 ATH DANGER DETECTION - MORE TOLERANT
const ATH_DANGER_THRESHOLD: f64 = 85.0; // >85% of ATH = dangerous (was 75%)

// ================================================================================================
// 📊 COMPREHENSIVE TOKEN ANALYSIS DATA
// ================================================================================================

/// Complete token analysis combining all available data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAnalysis {
    // Core identification
    pub mint: String,
    pub symbol: String,
    pub current_price: f64,

    // Safety & Security Analysis
    pub safety_score: f64, // 0-100 comprehensive safety score
    pub rugcheck_score: Option<i32>, // Raw rugcheck score
    pub rugcheck_normalized: Option<i32>, // 0-100 normalized score
    pub is_rugged: bool,
    pub freeze_authority_safe: bool,
    pub lp_unlocked_risk: bool,
    pub risk_reasons: Vec<String>,

    // Market Data Analysis
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub volume_trend: f64, // Current vs average volume
    pub buy_pressure: f64, // 0-1, higher = more buying
    pub price_momentum: f64, // Recent price acceleration

    // Legitimacy Indicators
    pub has_website: bool,
    pub has_socials: bool,
    pub has_image: bool,
    pub verified_labels: usize, // Number of verified labels
    pub legitimacy_score: f64, // 0-1 legitimacy factor

    // Market Context
    pub token_age_hours: f64,
    pub is_near_ath: bool,
    pub ath_proximity_percent: f64, // How close to ATH (0-100%)

    // Analysis Results
    pub volatility_factor: f64, // Expected volatility multiplier
    pub momentum_score: f64, // Momentum-based urgency
    pub time_pressure_max: f64, // Maximum recommended hold time
}

/// Risk classification levels
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyLevel {
    UltraSafe, // 90-100 safety score
    Safe, // 70-89 safety score
    Medium, // 50-69 safety score
    Risky, // 30-49 safety score
    Dangerous, // 0-29 safety score
}

impl SafetyLevel {
    fn from_score(score: f64) -> Self {
        match score {
            s if s >= 90.0 => SafetyLevel::UltraSafe,
            s if s >= 70.0 => SafetyLevel::Safe,
            s if s >= 50.0 => SafetyLevel::Medium,
            s if s >= 30.0 => SafetyLevel::Risky,
            _ => SafetyLevel::Dangerous,
        }
    }

    fn get_base_profit_range(&self) -> (f64, f64) {
        match self {
            SafetyLevel::UltraSafe => (ULTRA_SAFE_PROFIT_MIN, ULTRA_SAFE_PROFIT_MAX),
            SafetyLevel::Safe => (SAFE_PROFIT_MIN, SAFE_PROFIT_MAX),
            SafetyLevel::Medium => (MEDIUM_PROFIT_MIN, MEDIUM_PROFIT_MAX),
            SafetyLevel::Risky => (RISKY_PROFIT_MIN, RISKY_PROFIT_MAX),
            SafetyLevel::Dangerous => (DANGEROUS_PROFIT_MIN, DANGEROUS_PROFIT_MAX),
        }
    }

    fn get_max_hold_time(&self) -> f64 {
        match self {
            SafetyLevel::UltraSafe => ULTRA_SAFE_MAX_TIME,
            SafetyLevel::Safe => SAFE_MAX_TIME,
            SafetyLevel::Medium => MEDIUM_MAX_TIME,
            SafetyLevel::Risky => RISKY_MAX_TIME,
            SafetyLevel::Dangerous => DANGEROUS_MAX_TIME,
        }
    }

    fn get_trailing_stop_percent(&self) -> f64 {
        match self {
            SafetyLevel::UltraSafe => TRAILING_STOP_ULTRA_SAFE,
            SafetyLevel::Safe => TRAILING_STOP_SAFE,
            SafetyLevel::Medium => TRAILING_STOP_MEDIUM,
            SafetyLevel::Risky => TRAILING_STOP_RISKY,
            SafetyLevel::Dangerous => TRAILING_STOP_DANGEROUS,
        }
    }
}

// ================================================================================================
// 🧠 INTELLIGENT TOKEN ANALYSIS ENGINE
// ================================================================================================

/// Analyze token comprehensively using all available data sources
pub async fn analyze_token_comprehensive(mint: &str) -> Result<TokenAnalysis, String> {
    // Get token price using pool service for real-time accuracy
    let pool_service = get_pool_service();
    let current_price = if let Some(pool_result) = pool_service.get_pool_price(mint, None).await {
        pool_result.price_sol.ok_or_else(||
            format!("Pool price calculation failed for token: {}", mint)
        )?
    } else {
        return Err(format!("Failed to get pool price for token: {}", mint));
    };

    if current_price <= 0.0 || !current_price.is_finite() {
        return Err(format!("Invalid current price for token: {}: {}", mint, current_price));
    }

    // Get token data from database
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let token_data = database
        .get_token_by_mint(mint)
        .map_err(|e| format!("Failed to get token data: {}", e))?
        .ok_or_else(|| format!("Token not found in database: {}", mint))?;

    // Get rugcheck security analysis
    let rugcheck_data = get_token_rugcheck_data_safe(mint).await.map_err(|e|
        format!("Failed to get rugcheck data: {}", e)
    )?;

    // Extract core security data
    let (
        rugcheck_score,
        rugcheck_normalized,
        is_rugged,
        freeze_authority_safe,
        lp_unlocked_risk,
        risk_reasons,
    ) = if let Some(data) = &rugcheck_data {
        let high_risk_issues = get_high_risk_issues(data);
        let is_safe = is_token_safe_for_trading_safe(mint).await;

        (
            data.score,
            data.score_normalised,
            data.rugged.unwrap_or(false),
            data.freeze_authority.is_none() && data.mint_authority.is_none(),
            false, // Will be determined from market data if available
            if high_risk_issues.is_empty() {
                vec!["Token appears safe based on rugcheck analysis".to_string()]
            } else {
                high_risk_issues
            },
        )
    } else {
        (None, None, false, true, false, vec!["No rugcheck data available".to_string()])
    };

    // Calculate safety score (0-100)
    let safety_score = calculate_comprehensive_safety_score(
        &token_data,
        &rugcheck_data,
        current_price
    );

    // Extract market data
    let liquidity_usd = token_data.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    let volume_24h = token_data.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(0.0);

    // Calculate volume trend (current vs historical average)
    let volume_trend = calculate_volume_trend(&token_data);

    // Calculate buy pressure from transaction data
    let buy_pressure = calculate_buy_pressure(&token_data);

    // Calculate price momentum
    let price_momentum = calculate_price_momentum(&token_data);

    // Analyze legitimacy indicators
    let (has_website, has_socials, has_image, verified_labels) = analyze_legitimacy_indicators(
        &token_data
    );
    let legitimacy_score = calculate_legitimacy_score(
        has_website,
        has_socials,
        has_image,
        verified_labels
    );

    // Calculate token age
    let token_age_hours = calculate_token_age_hours(&token_data);

    // Check ATH proximity (simplified - would need historical data for exact ATH)
    let (is_near_ath, ath_proximity_percent) = estimate_ath_proximity(&token_data, current_price);

    // Calculate volatility factor based on liquidity
    let volatility_factor = calculate_volatility_factor(liquidity_usd);

    // Calculate momentum score
    let momentum_score = calculate_momentum_score(volume_trend, buy_pressure, price_momentum);

    // Determine maximum hold time
    let safety_level = SafetyLevel::from_score(safety_score);
    let time_pressure_max = safety_level.get_max_hold_time();

    Ok(TokenAnalysis {
        mint: mint.to_string(),
        symbol: token_data.symbol.clone(),
        current_price,
        safety_score,
        rugcheck_score,
        rugcheck_normalized,
        is_rugged,
        freeze_authority_safe,
        lp_unlocked_risk,
        risk_reasons,
        liquidity_usd,
        volume_24h,
        volume_trend,
        buy_pressure,
        price_momentum,
        has_website,
        has_socials,
        has_image,
        verified_labels,
        legitimacy_score,
        token_age_hours,
        is_near_ath,
        ath_proximity_percent,
        volatility_factor,
        momentum_score,
        time_pressure_max,
    })
}

/// Calculate comprehensive safety score (0-100)
fn calculate_comprehensive_safety_score(
    token_data: &crate::tokens::types::ApiToken,
    rugcheck_data: &Option<crate::tokens::rugcheck::RugcheckResponse>,
    _current_price: f64
) -> f64 {
    let mut safety_score: f64 = 50.0; // Start with neutral score

    // Rugcheck contribution (40% of total score)
    if let Some(rugcheck) = rugcheck_data {
        // Check if token is detected as rugged
        if rugcheck.rugged.unwrap_or(false) {
            safety_score = 0.0; // Rugged token = 0 safety
        } else {
            // CORRECTED: Rugcheck score is a RISK score - higher means MORE risk!
            let rugcheck_risk_score = rugcheck.score_normalised
                .or(rugcheck.score)
                .unwrap_or(50) as f64;

            // Convert risk score (0-100) to safety contribution (40-0)
            // Higher risk score = lower safety contribution
            let rugcheck_contribution = 40.0 - (rugcheck_risk_score / 100.0) * 40.0;

            // Additional penalty for high-risk items
            let risk_penalty = if let Some(risks) = &rugcheck.risks {
                let high_risk_count = risks
                    .iter()
                    .filter(|r| {
                        r.level
                            .as_ref()
                            .map(|l| (l.to_lowercase() == "high" || l.to_lowercase() == "critical"))
                            .unwrap_or(false)
                    })
                    .count();
                (high_risk_count as f64) * 5.0 // -5 points per high/critical risk
            } else {
                0.0
            };

            safety_score = (rugcheck_contribution - risk_penalty).max(0.0);
        }
    } else {
        safety_score = 20.0; // No rugcheck data = lower safety
    }

    // Liquidity contribution (25% of total score)
    let liquidity_usd = token_data.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);
    let liquidity_contribution = match liquidity_usd {
        l if l >= PROFIT_HIGH_LIQUIDITY_THRESHOLD => 25.0,
        l if l >= PROFIT_MEDIUM_HIGH_LIQUIDITY_THRESHOLD => 20.0,
        l if l >= PROFIT_MEDIUM_LIQUIDITY_THRESHOLD => 15.0,
        l if l >= PROFIT_LOW_LIQUIDITY_THRESHOLD => 10.0,
        _ => 5.0,
    };
    safety_score += liquidity_contribution;

    // Legitimacy contribution (20% of total score)
    let has_website = token_data.info
        .as_ref()
        .and_then(|info| info.websites.as_ref())
        .map(|w| !w.is_empty())
        .unwrap_or(false);
    let has_socials = token_data.info
        .as_ref()
        .and_then(|info| info.socials.as_ref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_image = token_data.info
        .as_ref()
        .and_then(|info| info.image_url.as_ref())
        .is_some();

    let legitimacy_contribution =
        (if has_website { 7.0 } else { 0.0 }) +
        (if has_socials { 7.0 } else { 0.0 }) +
        (if has_image { 6.0 } else { 0.0 });
    safety_score += legitimacy_contribution;

    // Age contribution (10% of total score)
    let age_hours = token_data.pair_created_at
        .map(|timestamp| {
            let now = Utc::now().timestamp();
            ((now - timestamp) / 3600) as f64
        })
        .unwrap_or(0.0);

    let age_contribution = match age_hours {
        a if a >= 168.0 => 10.0, // 1 week+
        a if a >= 72.0 => 8.0, // 3 days+
        a if a >= 24.0 => 6.0, // 1 day+
        a if a >= 6.0 => 4.0, // 6 hours+
        _ => 2.0, // Very new
    };
    safety_score += age_contribution;

    // Volume/activity contribution (5% of total score)
    let volume_24h = token_data.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(0.0);
    let volume_contribution = if volume_24h > 10000.0 { 5.0 } else { 2.0 };
    safety_score += volume_contribution;

    safety_score.min(100.0).max(0.0)
}

/// Calculate volume trend factor
fn calculate_volume_trend(token_data: &crate::tokens::types::ApiToken) -> f64 {
    if let Some(volume) = &token_data.volume {
        let vol_1h = volume.h1.unwrap_or(0.0);
        let vol_6h = volume.h6.unwrap_or(0.0);
        let vol_24h = volume.h24.unwrap_or(0.0);

        if vol_24h > 0.0 && vol_6h > 0.0 {
            // Compare recent volume to average
            let avg_hourly = vol_24h / 24.0;
            let recent_hourly = vol_1h;

            if avg_hourly > 0.0 {
                return (recent_hourly / avg_hourly).min(3.0); // Cap at 3x
            }
        }
    }
    1.0 // Neutral if no data
}

/// Calculate buy pressure from transaction data
fn calculate_buy_pressure(token_data: &crate::tokens::types::ApiToken) -> f64 {
    if let Some(txns) = &token_data.txns {
        if let Some(h1) = &txns.h1 {
            let buys = h1.buys.unwrap_or(0) as f64;
            let sells = h1.sells.unwrap_or(0) as f64;
            let total = buys + sells;

            if total > 0.0 {
                return buys / total; // 0-1 ratio
            }
        }
    }
    0.5 // Neutral if no data
}

/// Calculate price momentum
fn calculate_price_momentum(token_data: &crate::tokens::types::ApiToken) -> f64 {
    if let Some(price_change) = &token_data.price_change {
        let change_1h = price_change.h1.unwrap_or(0.0);
        let change_6h = price_change.h6.unwrap_or(0.0);

        // Acceleration = short term change vs longer term
        if change_6h != 0.0 {
            return (change_1h / change_6h).abs().min(3.0);
        }

        return change_1h.abs() / 100.0; // Direct momentum
    }
    0.0 // No momentum if no data
}

/// Analyze legitimacy indicators
fn analyze_legitimacy_indicators(
    token_data: &crate::tokens::types::ApiToken
) -> (bool, bool, bool, usize) {
    let has_website = token_data.info
        .as_ref()
        .and_then(|info| info.websites.as_ref())
        .map(|w| !w.is_empty())
        .unwrap_or(false);

    let has_socials = token_data.info
        .as_ref()
        .and_then(|info| info.socials.as_ref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let has_image = token_data.info
        .as_ref()
        .and_then(|info| info.image_url.as_ref())
        .is_some();

    let verified_labels = token_data.labels
        .as_ref()
        .map(|labels| labels.len())
        .unwrap_or(0);

    (has_website, has_socials, has_image, verified_labels)
}

/// Calculate legitimacy score
fn calculate_legitimacy_score(
    has_website: bool,
    has_socials: bool,
    has_image: bool,
    verified_labels: usize
) -> f64 {
    let mut score = 0.0;

    if has_website {
        score += 0.3;
    }
    if has_socials {
        score += 0.3;
    }
    if has_image {
        score += 0.2;
    }
    score += ((verified_labels as f64) * 0.05).min(0.2); // Up to 0.2 for labels

    score.min(1.0)
}

/// Calculate token age in hours
fn calculate_token_age_hours(token_data: &crate::tokens::types::ApiToken) -> f64 {
    token_data.pair_created_at
        .map(|timestamp| {
            let now = Utc::now().timestamp();
            ((now - timestamp) / 3600) as f64
        })
        .unwrap_or(0.0)
}

/// Estimate ATH proximity (simplified without historical data)
fn estimate_ath_proximity(
    token_data: &crate::tokens::types::ApiToken,
    _current_price: f64
) -> (bool, f64) {
    // Use price change data to estimate if we're near recent highs
    if let Some(price_change) = &token_data.price_change {
        let change_24h = price_change.h24.unwrap_or(0.0);

        // If we're up significantly in 24h, we might be near highs
        if change_24h > 100.0 {
            // >100% gain in 24h
            let proximity = ((change_24h / 200.0) * 100.0).min(95.0); // Estimate proximity
            return (proximity > ATH_DANGER_THRESHOLD, proximity);
        }
    }

    (false, 0.0)
}

/// Calculate volatility factor based on liquidity
fn calculate_volatility_factor(liquidity_usd: f64) -> f64 {
    match liquidity_usd {
        l if l >= PROFIT_HIGH_LIQUIDITY_THRESHOLD => 0.5, // Low volatility
        l if l >= PROFIT_MEDIUM_HIGH_LIQUIDITY_THRESHOLD => 0.7, // Medium-low volatility
        l if l >= PROFIT_MEDIUM_LIQUIDITY_THRESHOLD => 1.0, // Normal volatility
        l if l >= PROFIT_LOW_LIQUIDITY_THRESHOLD => 1.5, // High volatility
        _ => 2.0, // Very high volatility
    }
}

/// Calculate momentum score for urgency
fn calculate_momentum_score(volume_trend: f64, buy_pressure: f64, price_momentum: f64) -> f64 {
    let volume_component = (volume_trend - 1.0).max(0.0).min(1.0); // 0-1
    let pressure_component = (buy_pressure - 0.5) * 2.0; // -1 to 1, then scale
    let momentum_component = price_momentum.min(1.0); // 0-1

    // Weighted average
    (volume_component * 0.4 + pressure_component.abs() * 0.3 + momentum_component * 0.3)
        .max(0.0)
        .min(2.0)
}

// ================================================================================================
// 🎯 MASTER SHOULD_SELL FUNCTION - THE ONE AND ONLY
// ================================================================================================

/// THE ULTIMATE SHOULD_SELL FUNCTION
///
/// Combines all available data sources for intelligent profit decisions:
/// - Real-time P&L calculation
/// - Comprehensive token safety analysis
/// - Market momentum detection
/// - Risk-adjusted profit targets
/// - Time pressure scaling
/// - ATH proximity warnings
/// - Minimum profit threshold in SOL (from trader.rs PROFIT_EXTRA_NEEDED_SOL)
///
/// Returns: (urgency_score: 0.0-1.0, detailed_reason: String)
pub async fn should_sell(position: &Position, current_price: f64) -> (f64, String) {
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🔍 CRITICAL SAFETY CHECKS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Validate inputs
    if current_price <= 0.0 || !current_price.is_finite() {
        log(LogTag::Profit, "ERROR", &format!("Invalid current price: {}", current_price));
        return (0.0, "Invalid price data - holding position".to_string());
    }

    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        log(LogTag::Profit, "ERROR", &format!("Invalid entry price: {}", entry_price));
        return (0.0, "Invalid entry price - holding position".to_string());
    }

    // Calculate current P&L
    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(current_price));

    // Calculate position duration
    let now = Utc::now();
    let duration = now - position.entry_time;
    let minutes_held = (duration.num_seconds() as f64) / 60.0;

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 💰 MINIMUM PROFIT THRESHOLD CHECK (NEW FEATURE)
    // ═══════════════════════════════════════════════════════════════════════════════════════
    
    // Import the minimum profit threshold from trader.rs
    use crate::trader::PROFIT_EXTRA_NEEDED_SOL;
    
    // For profitable positions, ensure minimum SOL profit before selling
    if pnl_percent > 0.0 && pnl_sol < PROFIT_EXTRA_NEEDED_SOL {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "MIN_PROFIT_CHECK",
                &format!(
                    "Profit below minimum threshold: {:.8} SOL < {:.8} SOL required ({}% profit) - holding position",
                    pnl_sol,
                    PROFIT_EXTRA_NEEDED_SOL,
                    pnl_percent
                )
            );
        }
        return (
            0.0, 
            format!(
                "Profit too small: {:.8} SOL < {:.8} SOL minimum ({:.2}% profit)",
                pnl_sol,
                PROFIT_EXTRA_NEEDED_SOL,
                pnl_percent
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🤖 RL-ENHANCED EXIT ANALYSIS FOR 30+ MINUTE POSITIONS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // For positions older than 30 minutes, use RL-based exit analysis
    if minutes_held >= 30.0 {
        use crate::rl_learning::get_rl_exit_recommendation;

        // Get comprehensive token data for RL analysis
        let (liquidity_usd, volume_24h, market_cap, rugcheck_score) = match
            analyze_token_for_rl(&position.mint).await
        {
            Ok((liq, vol, mc, rs)) => (liq, vol, mc, rs),
            Err(_) => (50000.0, 200000.0, None, Some(50.0)), // Safe defaults
        };

        // Get RL exit recommendation
        if
            let Ok(exit_prediction) = get_rl_exit_recommendation(
                &position.mint,
                entry_price,
                current_price,
                pnl_percent,
                minutes_held / 60.0, // Convert to hours
                liquidity_usd,
                volume_24h,
                market_cap,
                rugcheck_score
            ).await
        {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "RL_EXIT_ANALYSIS",
                    &format!(
                        "🤖 RL analysis for {} (age: {:.1}min, PnL: {:.2}%): Exit: {}, Urgency: {:.1}%, Recovery: {:.1}%, Support: {:?}",
                        position.symbol,
                        minutes_held,
                        pnl_percent,
                        if exit_prediction.should_exit_now {
                            "✅ YES"
                        } else {
                            "❌ NO"
                        },
                        exit_prediction.exit_urgency_score * 100.0,
                        exit_prediction.predicted_recovery_probability * 100.0,
                        exit_prediction.support_level
                    )
                );
            }

            // Smart exit decision based on RL analysis
            if exit_prediction.should_exit_now {
                if pnl_percent > 0.0 {
                    // Profitable position - exit based on RL urgency
                    log(
                        LogTag::Profit,
                        "RL_PROFIT_EXIT",
                        &format!(
                            "RL PROFIT EXIT: {:.2}% profit, {:.1}min held, urgency: {:.1}%, opportunity cost: {:.1}%",
                            pnl_percent,
                            minutes_held,
                            exit_prediction.exit_urgency_score * 100.0,
                            exit_prediction.opportunity_cost_score * 100.0
                        )
                    );
                    return (
                        exit_prediction.exit_urgency_score,
                        format!(
                            "RL PROFIT EXIT: {:.2}% profit - RL recommends exit for better opportunities",
                            pnl_percent
                        ),
                    );
                } else if pnl_percent > -55.0 {
                    // Loss position but not at stop loss - smart loss minimization
                    let min_loss_target = exit_prediction.min_loss_exit_price;
                    let current_distance = (
                        ((current_price - min_loss_target) / min_loss_target) *
                        100.0
                    ).abs();

                    if
                        current_distance <= 2.0 ||
                        exit_prediction.predicted_recovery_probability < 0.3
                    {
                        log(
                            LogTag::Profit,
                            "RL_LOSS_EXIT",
                            &format!(
                                "RL LOSS MINIMIZATION: {:.2}% loss, low recovery chance ({:.1}%), taking controlled exit",
                                pnl_percent,
                                exit_prediction.predicted_recovery_probability * 100.0
                            )
                        );
                        return (
                            exit_prediction.exit_urgency_score,
                            format!(
                                "RL SMART EXIT: {:.2}% loss - minimizing loss with low recovery chance",
                                pnl_percent
                            ),
                        );
                    }
                }
            } else if pnl_percent < 0.0 && exit_prediction.predicted_recovery_probability >= 0.6 {
                // Loss position but good recovery chance - hold for recovery
                if is_debug_profit_enabled() {
                    log(
                        LogTag::Profit,
                        "RL_RECOVERY_HOLD",
                        &format!(
                            "RL RECOVERY HOLD: {:.2}% loss, but {:.1}% recovery chance in {:.1}h, holding for support: {:?}",
                            pnl_percent,
                            exit_prediction.predicted_recovery_probability * 100.0,
                            exit_prediction.predicted_recovery_time_hours,
                            exit_prediction.support_level
                        )
                    );
                }
                return (
                    0.0,
                    format!(
                        "RL RECOVERY HOLD: {:.2}% loss - {:.0}% recovery chance",
                        pnl_percent,
                        exit_prediction.predicted_recovery_probability * 100.0
                    ),
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ STOP LOSS PROTECTION - ABSOLUTE PRIORITY
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ HARD STOP LOSS PROTECTION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if pnl_percent <= STOP_LOSS_PERCENT {
        log(
            LogTag::Profit,
            "STOP_LOSS",
            &format!(
                "Stop loss triggered: {:.2}% loss (threshold: {:.2}%)",
                pnl_percent,
                STOP_LOSS_PERCENT
            )
        );
        return (1.0, format!("STOP LOSS: {:.2}% loss reached", pnl_percent));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🧠 INTELLIGENT LOSS MANAGEMENT - 30+ MINUTE RULE & RL INTEGRATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // For positions at loss, apply intelligent loss management
    if pnl_percent < 0.0 {
        let loss_severity = pnl_percent.abs();

        // 🧠 GET RL ANALYSIS FOR INTELLIGENT EXIT DECISIONS (available for all loss scenarios)
        let token_analysis = {
            use crate::rl_learning::get_rl_exit_recommendation;
            get_rl_exit_recommendation(
                &position.mint,
                position.entry_price,
                current_price,
                pnl_percent,
                minutes_held / 60.0, // Convert to hours
                5000.0, // Default liquidity (will be improved later)
                50000.0, // Default volume (will be improved later)
                None, // market_cap
                None // rugcheck_score
            ).await.ok()
        };

        // 🕐 30+ MINUTE LOSS MANAGEMENT RULE
        if minutes_held >= 30.0 {
            let time_pressure = (minutes_held - 30.0) / 60.0; // Hours over 30 minutes

            // 📈 GET PRICE HISTORY FOR TREND ANALYSIS
            let price_trend_factor = {
                use crate::tokens::pool::get_price_history_for_rl_learning;
                let price_history = get_price_history_for_rl_learning(&position.mint).await;
                if price_history.len() >= 10 {
                    // Analyze recent price trend (last 30 minutes)
                    let recent_cutoff = Utc::now() - chrono::Duration::minutes(30);
                    let recent_prices: Vec<f64> = price_history
                        .iter()
                        .filter(|(timestamp, _)| *timestamp >= recent_cutoff)
                        .map(|(_, price)| *price)
                        .collect();

                    if recent_prices.len() >= 5 {
                        let recent_trend = if recent_prices.len() >= 2 {
                            let start_price = recent_prices.first().unwrap();
                            let end_price = recent_prices.last().unwrap();
                            (end_price - start_price) / start_price
                        } else {
                            0.0
                        };

                        // Downward trend increases exit urgency
                        if recent_trend < -0.05 {
                            0.3
                        } else if
                            // Strong downtrend
                            recent_trend < -0.02
                        {
                            0.15
                        } else if
                            // Moderate downtrend
                            recent_trend > 0.02
                        {
                            -0.1
                        } else {
                            // Uptrend reduces urgency
                            0.0
                        } // Sideways
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            };

            // Calculate intelligent loss exit urgency based on:
            // 1. Loss severity (higher loss = higher urgency)
            // 2. Time held (longer = higher urgency)
            // 3. RL recovery probability (lower recovery = higher urgency)
            // 4. Price trend (downtrend = higher urgency)
            let loss_urgency = if let Some(analysis) = &token_analysis {
                // Use RL-enhanced loss management
                let recovery_factor = 1.0 - analysis.predicted_recovery_probability;
                let severity_factor = (loss_severity / 55.0).min(1.0); // 0-1 scale
                let time_factor = (time_pressure * 0.3).min(0.5); // Up to 50% from time

                (
                    recovery_factor * 0.4 +
                    severity_factor * 0.3 +
                    time_factor * 0.2 +
                    price_trend_factor * 0.1
                ).min(1.0)
            } else {
                // Fallback: time + severity + trend based urgency
                let severity_factor = (loss_severity / 55.0).min(1.0);
                let time_factor = (time_pressure * 0.4).min(0.6);
                (severity_factor * 0.5 + time_factor * 0.4 + price_trend_factor * 0.1).min(1.0)
            };

            // Apply intelligent thresholds
            let should_exit_loss = if let Some(analysis) = &token_analysis {
                // RL-based decision with enhanced criteria
                loss_severity >= 12.0 && // Lowered threshold for early intervention
                    (analysis.predicted_recovery_probability < 0.4 || // Low recovery chance
                        minutes_held >= 120.0 || // 2+ hours held
                        loss_severity >= 30.0 || // Significant loss
                        price_trend_factor >= 0.2) // Strong downtrend
            } else {
                // Fallback decision with trend consideration
                loss_severity >= 15.0 && // Only for losses >= 15%
                    (minutes_held >= 90.0 || // 1.5+ hours held
                        loss_severity >= 35.0 || // Very significant loss
                        price_trend_factor >= 0.25) // Very strong downtrend
            };

            if should_exit_loss {
                log(
                    LogTag::Profit,
                    "SMART_LOSS_EXIT",
                    &format!(
                        "Intelligent loss exit: {:.2}% loss, {:.1}min held, urgency: {:.1}%, trend: {:.1}%{}",
                        pnl_percent,
                        minutes_held,
                        loss_urgency * 100.0,
                        price_trend_factor * 100.0,
                        if let Some(analysis) = &token_analysis {
                            format!(
                                ", recovery chance: {:.1}%",
                                analysis.predicted_recovery_probability * 100.0
                            )
                        } else {
                            String::new()
                        }
                    )
                );
                return (
                    loss_urgency,
                    format!(
                        "SMART LOSS: {:.2}% loss, {:.1}min held - cutting losses intelligently",
                        pnl_percent,
                        minutes_held
                    ),
                );
            }
        }

        // For losses under 30 minutes or not meeting exit criteria, hold position
        // BUT apply emergency exit for severe losses with bad trends
        if minutes_held < 30.0 && loss_severity >= 25.0 {
            // Emergency exit for severe early losses
            let emergency_urgency = if let Some(analysis) = &token_analysis {
                if analysis.predicted_recovery_probability < 0.2 {
                    0.7 // High urgency for low recovery chance
                } else {
                    0.0
                }
            } else {
                // Check price trend for emergency decision
                use crate::tokens::pool::get_price_history_for_rl_learning;
                let price_history = get_price_history_for_rl_learning(&position.mint).await;
                if price_history.len() >= 5 {
                    let recent_prices: Vec<f64> = price_history
                        .iter()
                        .rev()
                        .take(10)
                        .map(|(_, price)| *price)
                        .collect();

                    if recent_prices.len() >= 3 {
                        let start = recent_prices.last().unwrap();
                        let end = recent_prices.first().unwrap();
                        let trend = (end - start) / start;

                        if trend < -0.15 {
                            0.8
                        } else {
                            // Very bad trend
                            0.0
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            };

            if emergency_urgency > 0.0 {
                log(
                    LogTag::Profit,
                    "EMERGENCY_LOSS_EXIT",
                    &format!(
                        "Emergency loss exit: {:.2}% loss in {:.1}min, severe decline detected",
                        pnl_percent,
                        minutes_held
                    )
                );
                return (
                    emergency_urgency,
                    format!(
                        "EMERGENCY LOSS: {:.2}% loss in {:.1}min - severe decline",
                        pnl_percent,
                        minutes_held
                    ),
                );
            }
        }

        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "HOLD_LOSS",
                &format!(
                    "Holding position with {:.2}% loss ({:.1}min held, above stop loss)",
                    pnl_percent,
                    minutes_held
                )
            );
        }
        return (0.0, format!("Holding at {:.2}% loss (above stop loss)", pnl_percent));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🚀 FAST PROFIT-TAKING OPTIMIZATION - RESPECTS MINIMUM HOLD TIME
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // FIXED: Fast profits now only apply AFTER minimum hold time is respected

    // ⏰ MINIMUM HOLD TIME PROTECTION - NO EXITS BEFORE 1 MINUTE
    if minutes_held < MIN_HOLD_TIME && pnl_percent > 0.0 {
        // Still calculate momentum for logging but don't exit
        if pnl_percent >= SPEED_PROFIT_THRESHOLD {
            log(
                LogTag::Profit,
                "FAST_PROFIT_BLOCKED",
                &format!(
                    "Fast profit blocked by MIN_HOLD_TIME: {:.2}% profit in {:.1}s (need {:.0}s minimum)",
                    pnl_percent,
                    minutes_held * 60.0,
                    MIN_HOLD_TIME * 60.0
                )
            );
        }
        return (
            0.0,
            format!(
                "Holding for minimum time: {:.1}s of {:.0}s required",
                minutes_held * 60.0,
                MIN_HOLD_TIME * 60.0
            ),
        );
    }

    // � SPEED PROFIT EXIT: >5% profit in <30 seconds = mega urgent (MOST SPECIFIC CHECK FIRST)
    if minutes_held >= MIN_HOLD_TIME && pnl_percent >= SPEED_PROFIT_THRESHOLD {
        log(
            LogTag::Profit,
            "SPEED_PROFIT_EXIT",
            &format!(
                "Speed profit exit triggered: {:.2}% profit in {:.1} seconds - exceptional momentum",
                pnl_percent,
                minutes_held * 60.0
            )
        );
        return (
            1.0,
            format!(
                "SPEED PROFIT: {:.2}% in {:.0}s - ultra-fast momentum!",
                pnl_percent,
                minutes_held * 60.0
            ),
        );
    }

    // � ULTRA-FAST PROFIT EXIT: >3% profit in <1 minute = immediate sell (BROADER CHECK SECOND)
    if minutes_held >= MIN_HOLD_TIME && pnl_percent >= FAST_PROFIT_THRESHOLD {
        log(
            LogTag::Profit,
            "FAST_PROFIT_EXIT",
            &format!(
                "Fast profit exit triggered: {:.2}% profit in {:.1} seconds - capturing quick momentum",
                pnl_percent,
                minutes_held * 60.0
            )
        );
        return (
            1.0,
            format!(
                "FAST PROFIT: {:.2}% in {:.0}s - immediate exit!",
                pnl_percent,
                minutes_held * 60.0
            ),
        );
    }

    // 🧠 ADAPTIVE FAST PROFIT: Adjusts thresholds based on momentum and time
    // Lower thresholds for very fast gains, higher thresholds for sustained gains
    if minutes_held < 2.0 && pnl_percent > 0.0 {
        // Prevent division by zero and ensure minimum time for meaningful momentum calculation
        let time_seconds = (minutes_held * 60.0).max(MOMENTUM_MIN_TIME_SECONDS);

        // Calculate momentum factor: faster gains = higher urgency
        let momentum_factor = pnl_percent / time_seconds; // % per second

        // Dynamic threshold based on momentum
        let dynamic_threshold = if momentum_factor > 0.1 {
            // Very high momentum (>0.1% per second) = lower threshold
            1.5
        } else if momentum_factor > 0.05 {
            // High momentum (>0.05% per second) = medium threshold
            2.0
        } else {
            // Normal momentum = standard threshold
            2.5
        };

        if pnl_percent >= dynamic_threshold {
            log(
                LogTag::Profit,
                "ADAPTIVE_FAST_PROFIT",
                &format!(
                    "Adaptive fast profit: {:.2}% in {:.1}s (momentum: {:.4}%/s, threshold: {:.1}%)",
                    pnl_percent,
                    time_seconds,
                    momentum_factor,
                    dynamic_threshold
                )
            );
            return (
                0.9, // High urgency but not maximum to allow for slight delays
                format!(
                    "ADAPTIVE FAST: {:.2}% in {:.0}s (momentum: {:.4}%/s)",
                    pnl_percent,
                    time_seconds,
                    momentum_factor
                ),
            );
        }
    } // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🚀 INSTANT MEGA-PROFIT EXITS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if pnl_percent >= INSTANT_SELL_PROFIT {
        log(
            LogTag::Profit,
            "MEGA_PROFIT",
            &format!("Instant sell triggered: {:.2}% profit", pnl_percent)
        );
        return (1.0, format!("MEGA PROFIT: {:.2}% - instant sell!", pnl_percent));
    }

    if pnl_percent >= MEGA_PROFIT_THRESHOLD {
        log(LogTag::Profit, "LARGE_PROFIT", &format!("Large profit detected: {:.2}%", pnl_percent));
        return (0.9, format!("LARGE PROFIT: {:.2}% - sell very soon!", pnl_percent));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🧠 COMPREHENSIVE TOKEN ANALYSIS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let token_analysis = match analyze_token_comprehensive(&position.mint).await {
        Ok(analysis) => analysis,
        Err(e) => {
            log(
                LogTag::Profit,
                "WARN",
                &format!(
                    "Failed to analyze token {}: {} - using fallback logic",
                    position.symbol,
                    e
                )
            );

            // Fallback to simple profit logic when analysis fails
            return fallback_profit_logic(pnl_percent, minutes_held);
        }
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🎯 DYNAMIC PROFIT TARGET CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let safety_level = SafetyLevel::from_score(token_analysis.safety_score);
    let (base_min_profit, base_max_profit) = safety_level.get_base_profit_range();

    // Adjust profit targets based on momentum and volatility
    let momentum_multiplier = 1.0 + token_analysis.momentum_score * 0.5; // Up to 50% increase
    let volatility_multiplier = token_analysis.volatility_factor; // 0.5x to 2.0x

    let target_min_profit = (base_min_profit / momentum_multiplier) * volatility_multiplier;
    let target_max_profit = base_max_profit * momentum_multiplier * volatility_multiplier;

    // Calculate profit progression for reference (now using adjusted version)
    let _profit_progression = if pnl_percent >= target_max_profit {
        1.0
    } else if pnl_percent >= target_min_profit {
        (pnl_percent - target_min_profit) / (target_max_profit - target_min_profit)
    } else {
        0.0
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ TIME PRESSURE CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let max_hold_time = token_analysis.time_pressure_max;
    let time_pressure = (minutes_held / max_hold_time).min(1.0);

    // Additional time pressure for risky tokens
    let risk_time_pressure = match safety_level {
        SafetyLevel::Dangerous => time_pressure * 1.5, // 50% more time pressure
        SafetyLevel::Risky => time_pressure * 1.2, // 20% more time pressure
        _ => time_pressure,
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ SAFETY-BASED TIME EXIT LOGIC - RESPECTS TOKEN SAFETY LEVELS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Get safety-based time thresholds (don't override safer tokens with arbitrary 30-min rule)
    let safety_level = SafetyLevel::from_score(token_analysis.safety_score);
    let max_hold_time = safety_level.get_max_hold_time();
    let warning_time = max_hold_time * 0.7; // 70% of max hold time = warning
    let urgent_time = max_hold_time * 0.9; // 90% of max hold time = urgent

    // 🚨 SAFETY-BASED TIME EXIT: Approaching max hold time with profit
    if minutes_held >= warning_time && pnl_percent > 0.0 {
        let time_progress = (minutes_held - warning_time) / (max_hold_time - warning_time);
        let time_exit_urgency = (0.6 + time_progress * 0.4).min(1.0); // 60-100% urgency

        // Higher urgency for riskier tokens
        let risk_multiplier = match safety_level {
            SafetyLevel::Dangerous => 1.3,
            SafetyLevel::Risky => 1.2,
            _ => 1.0,
        };
        let final_urgency = (time_exit_urgency * risk_multiplier).min(1.0);

        log(
            LogTag::Profit,
            "SAFETY_TIME_EXIT",
            &format!(
                "SAFETY TIME EXIT: {:.1}min held ({:.1}% of {:.1}min max) with {:.2}% profit - urgency: {:.2}",
                minutes_held,
                (minutes_held / max_hold_time) * 100.0,
                max_hold_time,
                pnl_percent,
                final_urgency
            )
        );

        return (
            final_urgency,
            format!(
                "SAFETY EXIT: {:.1}min held ({:.0}% of max {:.0}min) with {:.2}% profit",
                minutes_held,
                (minutes_held / max_hold_time) * 100.0,
                max_hold_time,
                pnl_percent
            ),
        );
    }

    // � URGENT SAFETY EXIT: At or past max hold time with any profit
    if minutes_held >= urgent_time && pnl_percent > 0.0 {
        log(
            LogTag::Profit,
            "URGENT_SAFETY_EXIT",
            &format!(
                "URGENT SAFETY EXIT: {:.1}min held (>{:.1}min urgent threshold) with {:.2}% profit - immediate sell!",
                minutes_held,
                urgent_time,
                pnl_percent
            )
        );

        return (
            1.0,
            format!(
                "URGENT SAFETY EXIT: {:.1}min held (>{:.0}min limit) with {:.2}% profit!",
                minutes_held,
                urgent_time,
                pnl_percent
            ),
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ SAFETY-BASED TRAILING STOP LOGIC - RISK-ADJUSTED PROTECTION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let safety_level = SafetyLevel::from_score(token_analysis.safety_score);
    let trailing_stop_threshold = safety_level.get_trailing_stop_percent();

    if USE_TRAILING_STOP && pnl_percent > trailing_stop_threshold {
        // Get highest price reached (from position tracking)
        let highest_price = position.price_highest;

        if highest_price > 0.0 {
            let highest_profit_percent = ((highest_price - entry_price) / entry_price) * 100.0;
            let current_drop_from_peak = highest_profit_percent - pnl_percent;

            if current_drop_from_peak >= trailing_stop_threshold {
                log(
                    LogTag::Profit,
                    "TRAILING_STOP",
                    &format!(
                        "Safety-based trailing stop triggered: Dropped {:.2}% from peak of {:.2}% (safety={:?}, threshold: {:.2}%)",
                        current_drop_from_peak,
                        highest_profit_percent,
                        safety_level,
                        trailing_stop_threshold
                    )
                );
                return (
                    1.0,
                    format!(
                        "TRAILING STOP: Dropped {:.2}% from {:.2}% peak (safety {:?})",
                        current_drop_from_peak,
                        highest_profit_percent,
                        safety_level
                    ),
                );
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ OPTIMIZED MINIMUM HOLD TIME - NEW FEATURE
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if minutes_held < MIN_HOLD_TIME {
        log(
            LogTag::Profit,
            "MIN_HOLD",
            &format!(
                "Minimum hold time not reached: {:.2}min < {:.2}min threshold",
                minutes_held,
                MIN_HOLD_TIME
            )
        );
        return (
            0.0,
            format!("Minimum hold time: {:.2}min < {:.2}min required", minutes_held, MIN_HOLD_TIME),
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🎯 TIME DECAY PROFIT TARGET ADJUSTMENT - NEW FEATURE
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let time_decay_multiplier = 1.0 - (minutes_held / max_hold_time) * TIME_DECAY_FACTOR;
    let adjusted_min_profit = target_min_profit * time_decay_multiplier.max(0.7); // Never go below 70%
    let adjusted_max_profit = target_max_profit * time_decay_multiplier.max(0.8); // Never go below 80%

    // Recalculate profit progression with time decay
    let adjusted_profit_progression = if pnl_percent >= adjusted_max_profit {
        1.0
    } else if pnl_percent >= adjusted_min_profit {
        (pnl_percent - adjusted_min_profit) / (adjusted_max_profit - adjusted_min_profit)
    } else {
        0.0
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // �🚨 SPECIAL RISK FACTORS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let mut risk_urgency: f64 = 0.0;
    let mut risk_reasons = Vec::new();

    // Critical security risks
    if token_analysis.is_rugged {
        risk_urgency = 1.0;
        risk_reasons.push("TOKEN MARKED AS RUGGED".to_string());
    }

    if !token_analysis.freeze_authority_safe {
        risk_urgency = risk_urgency.max(0.7);
        risk_reasons.push("FREEZE AUTHORITY RISK".to_string());
    }

    if token_analysis.lp_unlocked_risk {
        risk_urgency = risk_urgency.max(0.6);
        risk_reasons.push("LP UNLOCK RISK".to_string());
    }

    // ATH proximity danger
    if token_analysis.is_near_ath {
        let ath_urgency = (token_analysis.ath_proximity_percent - ATH_DANGER_THRESHOLD) / 25.0;
        risk_urgency = risk_urgency.max(ath_urgency * 0.5); // Up to 50% urgency
        risk_reasons.push(format!("NEAR ATH ({:.1}%)", token_analysis.ath_proximity_percent));
    }

    // Low liquidity warning
    if token_analysis.liquidity_usd < PROFIT_LOW_LIQUIDITY_THRESHOLD {
        risk_urgency = risk_urgency.max(0.3);
        risk_reasons.push(format!("LOW LIQUIDITY (${:.0})", token_analysis.liquidity_usd));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🧮 FINAL URGENCY CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Enhanced time pressure for profitable positions
    let enhanced_time_pressure = if pnl_percent > 0.0 {
        match minutes_held {
            m if m >= 25.0 => risk_time_pressure * 1.8, // 80% boost for 25+ minutes
            m if m >= 20.0 => risk_time_pressure * 1.5, // 50% boost for 20+ minutes
            m if m >= 15.0 => risk_time_pressure * 1.3, // 30% boost for 15+ minutes
            _ => risk_time_pressure,
        }
    } else {
        risk_time_pressure
    };

    // Combine all factors with optimized weights
    let profit_urgency = adjusted_profit_progression * 0.3; // Use adjusted progression
    let time_urgency = enhanced_time_pressure * 0.4; // Increased from 30% to 40%
    let momentum_urgency = (token_analysis.momentum_score / 2.0) * 0.2; // 20% weight on momentum
    let safety_urgency = (1.0 - token_analysis.safety_score / 100.0) * 0.1; // 10% weight on safety

    let base_urgency = profit_urgency + time_urgency + momentum_urgency + safety_urgency;

    // Apply risk multipliers
    let final_urgency = (base_urgency + risk_urgency).min(1.0).max(0.0);

    // Additional urgency boost for profitable positions held too long
    let final_urgency_with_time_boost = if pnl_percent > 0.0 && minutes_held >= 25.0 {
        let time_boost = ((minutes_held - 25.0) / 20.0) * 0.3; // Up to 30% boost
        (final_urgency + time_boost).min(1.0)
    } else {
        final_urgency
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 📝 DETAILED REASON GENERATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let reason = if final_urgency_with_time_boost >= 0.8 {
        format!(
            "URGENT SELL: {:.1}% profit, {:.1}min held, safety={:.0}/100{}{}",
            pnl_percent,
            minutes_held,
            token_analysis.safety_score,
            if risk_reasons.is_empty() {
                ""
            } else {
                ", RISKS: "
            },
            risk_reasons.join(", ")
        )
    } else if final_urgency_with_time_boost >= 0.6 {
        format!(
            "CONSIDER SELL: {:.1}% profit, {:.1}min held, safety={:.0}/100, targets={:.1}%-{:.1}%",
            pnl_percent,
            minutes_held,
            token_analysis.safety_score,
            target_min_profit,
            target_max_profit
        )
    } else if final_urgency_with_time_boost >= 0.3 {
        format!(
            "WATCH CLOSELY: {:.1}% profit, {:.1}min held, target={:.1}%-{:.1}%, safety={:.0}/100",
            pnl_percent,
            minutes_held,
            target_min_profit,
            target_max_profit,
            token_analysis.safety_score
        )
    } else {
        format!(
            "HOLD: {:.1}% profit, {:.1}min held, target={:.1}%-{:.1}%, safety={:.0}/100",
            pnl_percent,
            minutes_held,
            target_min_profit,
            target_max_profit,
            token_analysis.safety_score
        )
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 📊 DEBUG LOGGING (if enabled)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "ANALYSIS",
            &format!(
                "Token: {} | Safety: {:.0}/100 | Liquidity: ${:.0} | Momentum: {:.2} | Time: {:.1}/{:.1}min",
                position.symbol,
                token_analysis.safety_score,
                token_analysis.liquidity_usd,
                token_analysis.momentum_score,
                minutes_held,
                max_hold_time
            )
        );

        log(
            LogTag::Profit,
            "OPTIMIZED",
            &format!(
                "OPTIMIZED SYSTEM: Profit: {:.1}% | Adjusted Targets: {:.1}%-{:.1}% | Time Decay: {:.3} | Trailing Stop: {} | Min Hold: {:.1}min",
                pnl_percent,
                adjusted_min_profit,
                adjusted_max_profit,
                time_decay_multiplier,
                if USE_TRAILING_STOP {
                    "ENABLED"
                } else {
                    "DISABLED"
                },
                MIN_HOLD_TIME
            )
        );

        log(
            LogTag::Profit,
            "TARGETS",
            &format!(
                "Decision: Urgency={:.3} | Original Targets: {:.1}%-{:.1}% | Adjusted: {:.1}%-{:.1}% | Action: {}",
                final_urgency_with_time_boost,
                target_min_profit,
                target_max_profit,
                adjusted_min_profit,
                adjusted_max_profit,
                if final_urgency_with_time_boost >= 0.6 {
                    "SELL"
                } else {
                    "HOLD"
                }
            )
        );
    }

    (final_urgency_with_time_boost, reason)
}

// ================================================================================================
// 🔧 FALLBACK PROFIT LOGIC (when token analysis fails)
// ================================================================================================

/// Simple fallback profit logic when comprehensive analysis fails
fn fallback_profit_logic(pnl_percent: f64, minutes_held: f64) -> (f64, String) {
    // 🚨 MANDATORY TIME-BASED EXITS (even in fallback mode)
    if minutes_held >= 45.0 && pnl_percent > 0.0 {
        return (
            1.0,
            format!(
                "FALLBACK URGENT EXIT: {:.1}min held (>45min) with {:.1}% profit - immediate sell!",
                minutes_held,
                pnl_percent
            ),
        );
    }

    if minutes_held >= 30.0 && pnl_percent > 0.0 {
        let overtime_urgency = 0.7 + ((minutes_held - 30.0) / 15.0) * 0.3; // 70-100% urgency
        return (
            overtime_urgency.min(1.0),
            format!(
                "FALLBACK FAST EXIT: {:.1}min held (>30min) with {:.1}% profit - taking profits!",
                minutes_held,
                pnl_percent
            ),
        );
    }

    // Conservative profit targets when we can't analyze the token
    let target_profit = match minutes_held {
        m if m < 5.0 => 50.0, // 50% in first 5 minutes
        m if m < 10.0 => 30.0, // 30% in 5-10 minutes
        m if m < 20.0 => 20.0, // 20% in 10-20 minutes
        _ => 15.0, // 15% after 20 minutes
    };

    // Enhanced time pressure for fallback mode
    let time_pressure = ((minutes_held / 25.0).min(1.0) * 1.2).min(1.0); // More aggressive, max 25 minutes
    let profit_factor = (pnl_percent / target_profit).min(1.0);

    // Add extra urgency for positions held 20+ minutes
    let time_boost = if minutes_held >= 20.0 && pnl_percent > 0.0 {
        ((minutes_held - 20.0) / 10.0) * 0.3 // +30% urgency for every 10 minutes over 20
    } else {
        0.0
    };

    let urgency = (profit_factor * 0.5 + time_pressure * 0.5 + time_boost).min(1.0);

    let reason = format!(
        "FALLBACK: {:.1}% profit, {:.1}min held, target {:.1}% (no token data){}",
        pnl_percent,
        minutes_held,
        target_profit,
        if time_boost > 0.0 {
            format!(" +{:.0}% time urgency", time_boost * 100.0)
        } else {
            "".to_string()
        }
    );

    (urgency, reason)
}

// ================================================================================================
// 🤖 RL INTEGRATION HELPER FUNCTIONS
// ================================================================================================

/// Helper function to extract token data for RL analysis
pub async fn analyze_token_for_rl(
    mint: &str
) -> Result<(f64, f64, Option<f64>, Option<f64>), String> {
    // Get token data from database
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let token_data = database
        .get_token_by_mint(mint)
        .map_err(|e| format!("Failed to get token data: {}", e))?
        .ok_or_else(|| format!("Token not found in database: {}", mint))?;

    // Extract basic market data
    let liquidity_usd = token_data.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(50000.0); // Default to safe value

    let volume_24h = token_data.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(200000.0); // Default to safe value

    let market_cap = token_data.market_cap;

    // Get rugcheck risk score (remember: higher = more risk)
    let rugcheck_score = match get_token_rugcheck_data_safe(mint).await {
        Ok(Some(data)) => data.score_normalised.or(data.score).map(|s| s as f64),
        _ => Some(50.0), // Default medium risk
    };

    Ok((liquidity_usd, volume_24h, market_cap, rugcheck_score))
}
