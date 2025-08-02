/// ScreenerBot Trading Engine
///
/// ======================================
/// STOP LOSS PROTECTION SYSTEM
/// ======================================
///
/// This trading bot implements a stop loss at -55% with multiple layers of protection:
///
/// 1. **Stop Loss Threshold**: Set at -55% for controlled risk management
/// 2. **Profit-First Exit Logic**: All sell decisions prioritize profit taking
/// 3. **Smart System Validation**: Advanced profit system enforces stop loss rule
/// 4. **Final Safeguard**: Last-minute validation before any sell execution
/// 5. **Controlled Loss Strategy**: Positions at loss below -55% are sold to limit damage
///
/// This ensures controlled risk management while allowing reasonable loss limits
/// for positions that don't recover within acceptable timeframes.
///
/// MULTI-STRATEGY DIP DETECTION SYSTEM
/// ===================================
///
/// This system implements 5 sophisticated strategies to detect profitable dip entry points:
///
/// 1. **Immediate Drop Detection** (-5% to -30%)
///    - Detects sudden price drops in the immediate timeframe
///    - Adapts thresholds based on token's historical volatility
///    - Higher urgency for larger drops within the range
///
/// 2. **Moving Average Deviation** (-8% to -25%)
///    - Analyzes deviation from 5, 10, and 20-period moving averages
///    - Longer period deviations get higher weight
///    - Identifies oversold conditions relative to recent trends
///
/// 3. **Support Level Bounce** (-10% to -30%)
///    - Detects when price approaches historical support levels
///    - Uses technical analysis to find key price levels
///    - Bonus urgency for proximity to strong support
///
/// 4. **Volume-Weighted Price Dip** (-7% to -20%)
///    - Combines price drops with volume analysis
///    - Higher volume during dips indicates buying interest
///    - Validates dips with market participation data
///
/// 5. **Multi-Timeframe Convergence** (-6% to -18%)
///    - Requires multiple timeframes showing dip signals
///    - Short, medium, and long-term trend analysis
///    - Higher confidence when timeframes converge
///
/// The system calculates urgency scores from 0.0 to 2.0, with multiple strategies
/// providing consensus-based scoring and increased confidence.

// =============================================================================
// TRADING SYSTEM CONFIGURATION CONSTANTS
// =============================================================================

// -----------------------------------------------------------------------------
// Core Trading Parameters
// -----------------------------------------------------------------------------

/// Maximum number of concurrent open positions
pub const MAX_OPEN_POSITIONS: usize = 10;

/// Trade size in SOL for each position
pub const TRADE_SIZE_SOL: f64 = 0.001;

/// Default transaction fee for buy/sell operations
pub const TRANSACTION_FEE_SOL: f64 = 0.000005;

/// Default swap fee (set to 0 for GMGN routing)
pub const SWAP_FEE_PERCENT: f64 = 0.0;

/// Default slippage tolerance for swaps
pub const SLIPPAGE_TOLERANCE_PERCENT: f64 = 5.0;

// -----------------------------------------------------------------------------
// Position Timing Configuration - Improved for longer holding
// -----------------------------------------------------------------------------

/// Minimum hold time before considering sell (seconds) - reduced for flexibility
pub const MIN_POSITION_HOLD_TIME_SECS: f64 = 30.0;

/// Maximum hold time extended for longer-term profit taking (48 hours)
pub const MAX_POSITION_HOLD_TIME_SECS: f64 = 1.0 * 60.0 * 60.0; // 48 hours

/// Time after which time decay pressure starts - now 2 hours for better patience
pub const TIME_DECAY_START_SECS: f64 = 7200.0; // 2 hours

// -----------------------------------------------------------------------------
// Trading Logic Configuration
// -----------------------------------------------------------------------------
// Monitoring & Display Configuration
// -----------------------------------------------------------------------------

/// Summary display refresh interval (seconds)
pub const SUMMARY_DISPLAY_INTERVAL_SECS: u64 = 5;

/// New entry signals check interval (seconds)
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 5;

/// Open positions monitoring interval (seconds)
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

/// Price history tracking duration (hours)
pub const PRICE_HISTORY_DURATION_HOURS: i64 = 2;

// -----------------------------------------------------------------------------
// Task Timeout Configuration
// -----------------------------------------------------------------------------

/// Semaphore acquire timeout for token processing tasks (seconds)
pub const SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 120;

/// Individual token check task timeout (seconds)
pub const TOKEN_CHECK_TASK_TIMEOUT_SECS: u64 = 60;

/// Price cache lock acquire timeout (milliseconds)
pub const PRICE_CACHE_LOCK_TIMEOUT_MS: u64 = 2000;

/// Task collection timeout for concurrent operations (seconds)
pub const TASK_COLLECTION_TIMEOUT_SECS: u64 = 120;

// -----------------------------------------------------------------------------
// Wallet Management Configuration
// -----------------------------------------------------------------------------

/// Automatically close Associated Token Accounts after selling tokens
pub const AUTO_CLOSE_ATA_AFTER_SELL: bool = true;

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_trader_enabled;
use crate::tokens::{
    Token,
    update_open_positions_safe,
    get_all_tokens_by_liquidity,
    discover_tokens_once,
    monitor_tokens_once,
    get_token_price_blocking_safe,
    sync_watch_list_with_trader,
};
use crate::positions::{
    Position,
    calculate_position_pnl,
    update_position_tracking,
    get_open_positions_count,
    open_position,
    close_position,
    SAVED_POSITIONS,
};
use crate::summary::*;
use crate::utils::*;

use crate::filtering::{ should_buy_token, log_filtering_summary };
use crate::tokens::get_token_rugcheck_data_safe;
use crate::tokens::rugcheck::{ is_token_safe_for_trading, get_high_risk_issues };
use crate::smart_entry::{
    is_token_safe_for_smart_entry,
    is_token_safe_for_smart_entry_enhanced,
    is_valid_dip_for_liquidity,
    is_deepest_dip_moment,
    EntryAction,
};

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use colored::Colorize;

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: price history for each token (mint), stores Vec<(timestamp, price)>
pub static PRICE_HISTORY_24H: Lazy<
    StdArc<StdMutex<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>
> = Lazy::new(|| StdArc::new(StdMutex::new(HashMap::new())));

/// Static global: last known prices for each token
pub static LAST_PRICES: Lazy<StdArc<StdMutex<HashMap<String, f64>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashMap::new()))
});

/// Static global: tracks critical trading operations in progress to prevent force shutdown
pub static CRITICAL_OPERATIONS_IN_PROGRESS: Lazy<Arc<std::sync::atomic::AtomicUsize>> = Lazy::new(||
    Arc::new(std::sync::atomic::AtomicUsize::new(0))
);

// =============================================================================
// CRITICAL OPERATION PROTECTION
// =============================================================================

/// RAII guard that increments critical operations counter on creation and decrements on drop
/// Prevents shutdown while critical trading operations (buy/sell) are in progress
pub struct CriticalOperationGuard {
    _phantom: std::marker::PhantomData<()>,
}

impl CriticalOperationGuard {
    /// Create a new critical operation guard
    /// This should be created before any buy/sell operation
    pub fn new(operation_name: &str) -> Self {
        let count = CRITICAL_OPERATIONS_IN_PROGRESS.fetch_add(
            1,
            std::sync::atomic::Ordering::SeqCst
        );
        log(
            LogTag::Trader,
            "CRITICAL_OP_START",
            &format!(
                "🔒 PROTECTED: {} operation started (active operations: {})",
                operation_name,
                count + 1
            )
        );

        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the current number of critical operations in progress
    pub fn get_active_count() -> usize {
        CRITICAL_OPERATIONS_IN_PROGRESS.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for CriticalOperationGuard {
    fn drop(&mut self) {
        let count = CRITICAL_OPERATIONS_IN_PROGRESS.fetch_sub(
            1,
            std::sync::atomic::Ordering::SeqCst
        );
        log(
            LogTag::Trader,
            "CRITICAL_OP_END",
            &format!(
                "🔓 UNPROTECTED: Critical operation finished (remaining operations: {})",
                count - 1
            )
        );
    }
}

// =============================================================================
// DEBUG LOGGING CONFIGURATION
// =============================================================================

/// Helper function for conditional debug trader logs
pub fn debug_trader_log(log_type: &str, message: &str) {
    if is_debug_trader_enabled() {
        log(LogTag::Trader, log_type, message);
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR TOKENS MODULE INTEGRATION
// =============================================================================

/// Get tokens using the safe tokens module system
pub async fn get_tokens_from_safe_system() -> Vec<Token> {
    match get_all_tokens_by_liquidity().await {
        Ok(api_tokens) => {
            // Convert ApiToken to Token for compatibility with existing code
            api_tokens
                .into_iter()
                .map(|api_token| api_token.into())
                .collect()
        }
        Err(e) => {
            log(LogTag::Trader, "WARN", &format!("Failed to get tokens from safe system: {}", e));
            Vec::new()
        }
    }
}

/// Update open positions tracking in price service
async fn update_position_tracking_in_service() {
    let open_mints = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .filter(|pos| pos.exit_price.is_none())
                .map(|pos| pos.mint.clone())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        }
    };

    if !open_mints.is_empty() {
        update_open_positions_safe(open_mints).await;
        log(LogTag::Trader, "TRACK", "Updated open positions in price service");
    }
}

/// Try to populate tokens database with discovery data if it's empty
async fn ensure_tokens_populated() {
    // Check if we have tokens in the database
    match get_all_tokens_by_liquidity().await {
        Ok(tokens) if tokens.is_empty() => {
            log(LogTag::Trader, "INFO", "Token database is empty, running discovery...");

            // Run manual discovery to populate the database
            if let Err(e) = discover_tokens_once().await {
                log(LogTag::Trader, "WARN", &format!("Failed to run token discovery: {}", e));
            }

            // Run manual monitoring to update prices
            if let Err(e) = monitor_tokens_once().await {
                log(LogTag::Trader, "WARN", &format!("Failed to run token monitoring: {}", e));
            }
        }
        Ok(tokens) => {
            log(LogTag::Trader, "INFO", &format!("Token database has {} tokens", tokens.len()));
        }
        Err(e) => {
            log(LogTag::Trader, "WARN", &format!("Failed to check token database: {}", e));
        }
    }
}



/// Enhanced Multi-Strategy Buy Signal Detection System with OHLCV Analysis
/// Combines smart entry analysis with advanced OHLCV technical indicators
/// Returns urgency score from 0.0 (don't buy) to 2.0 (buy immediately)
pub async fn should_buy_enhanced(token: &Token, current_price: f64, prev_price: f64) -> f64 {
    use crate::global::is_debug_entry_enabled;
    use crate::ohlcv_analysis::perform_comprehensive_ohlcv_analysis;

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "ENHANCED_BUY_START",
            &format!(
                "🔬 ENHANCED SHOULD_BUY Analysis for {}: Current={:.10} SOL | Previous={:.10} SOL",
                token.symbol.as_str(),
                current_price,
                prev_price
            )
        );
    }

    // EMERGENCY BUY LOGIC - NEVER MISS BIG OPPORTUNITIES
    let price_change_percent = ((current_price - prev_price) / prev_price) * 100.0;
    let price_drop_percent = -price_change_percent;

    // Emergency thresholds for massive dips (bypass all other checks)
    let emergency_dip_threshold = match
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    {
        liq if liq >= 100_000.0 => 5.0, // High liquidity: 5%+ = emergency
        liq if liq >= 50_000.0 => 8.0, // Medium liquidity: 8%+ = emergency
        liq if liq >= 10_000.0 => 12.0, // Low liquidity: 12%+ = emergency
        _ => 15.0, // Very low: 15%+ = emergency
    };

    // EMERGENCY BUY - Skip all other checks for major dips
    if price_drop_percent >= emergency_dip_threshold {
        // Quick basic safety checks only
        if current_price > 0.0 && current_price.is_finite() && prev_price > 0.0 {
            let emergency_urgency = (price_drop_percent / emergency_dip_threshold).min(2.0);

            if is_debug_entry_enabled() {
                log(
                    LogTag::Trader,
                    "EMERGENCY_BUY_SIGNAL",
                    &format!(
                        "🚨 EMERGENCY BUY SIGNAL for {}: {:.2}% drop >= {:.1}% threshold | Urgency={:.2}",
                        token.symbol.as_str(),
                        price_drop_percent,
                        emergency_dip_threshold,
                        emergency_urgency
                    )
                );
            }
            return emergency_urgency;
        }
    }

    // Step 1: Use centralized filtering system (basic validation only)
    if !should_buy_token(token) {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENHANCED_BUY_FILTER_REJECT",
                &format!("❌ {} rejected by pre-filtering system", token.symbol.as_str())
            );
        }
        return 0.0;
    }

    // Step 2: Enhanced Smart entry analysis with OHLCV integration
    let (is_smart_safe, smart_analysis) = is_token_safe_for_smart_entry_enhanced(token).await;

    // RELAXED SMART ENTRY CHECK - Don't be too restrictive
    if !is_smart_safe && smart_analysis.entry_confidence < 0.3 {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENHANCED_BUY_SMART_REJECT",
                &format!(
                    "❌ {} rejected by smart entry analysis: ATH={:?} | Confidence={:.2}",
                    token.symbol.as_str(),
                    smart_analysis.ath_analysis.ath_danger_level,
                    smart_analysis.entry_confidence
                )
            );
        }
        return 0.0;
    }

    // Step 3: OHLCV Technical Analysis (NEW ENHANCED FEATURE)
    let ohlcv_analysis = perform_comprehensive_ohlcv_analysis(token).await;

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "ENHANCED_BUY_OHLCV",
            &format!(
                "📊 {} OHLCV Analysis: {} signals, urgency={:.2}, confidence={:.2}, safe={}",
                token.symbol.as_str(),
                ohlcv_analysis.dip_signals.len(),
                ohlcv_analysis.overall_buy_urgency,
                ohlcv_analysis.overall_confidence,
                ohlcv_analysis.is_safe_for_entry
            )
        );
    }

    // Step 4: Combine Traditional and OHLCV Analysis + MICRO-DIP DETECTION
    let price_change_percent = ((current_price - prev_price) / prev_price) * 100.0;
    let price_drop_percent = -price_change_percent;

    // MICRO-DIP LOGIC: Handle tiny price movements and static prices
    let micro_dip_threshold = smart_analysis.dynamic_dip_threshold * 0.3; // 30% of normal threshold
    let has_micro_dip = price_drop_percent >= micro_dip_threshold && price_drop_percent > 0.1;

    // Traditional dip check with micro-dip support
    let has_traditional_dip =
        price_drop_percent >= smart_analysis.dynamic_dip_threshold || has_micro_dip;

    // OHLCV signals check (relaxed confidence threshold)
    let has_ohlcv_signals =
        !ohlcv_analysis.dip_signals.is_empty() && ohlcv_analysis.overall_confidence > 0.2; // Lowered from 0.3

    // STATIC PRICE DETECTION: Buy tokens with good fundamentals even if price static
    let is_static_price = price_change_percent.abs() < 0.05; // Less than 0.05% change
    let has_static_opportunity =
        is_static_price &&
        smart_analysis.entry_confidence > 0.6 &&
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0) > 50_000.0; // Only for good liquidity

    if !has_traditional_dip && !has_ohlcv_signals && !has_static_opportunity {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENHANCED_BUY_NO_SIGNALS",
                &format!(
                    "❌ {} no buy signals: traditional_dip={} ({:.2}% < {:.1}%), micro_dip={} ({:.2}% < {:.1}%), ohlcv_signals={}, static_opportunity={}",
                    token.symbol.as_str(),
                    has_traditional_dip,
                    price_drop_percent,
                    smart_analysis.dynamic_dip_threshold,
                    has_micro_dip,
                    price_drop_percent,
                    micro_dip_threshold,
                    has_ohlcv_signals,
                    has_static_opportunity
                )
            );
        }
        return 0.0;
    }

    // Step 5: RELAXED OHLCV ATH Safety Check (less restrictive for more trades)
    let is_ath_safe = if let Some(ohlcv_ath) = &ohlcv_analysis.ath_analysis {
        // RELAXED: Allow moderate ATH proximity for good opportunities
        ohlcv_ath.is_safe_for_entry ||
            (smart_analysis.entry_confidence > 0.7 &&
                price_drop_percent > smart_analysis.dynamic_dip_threshold * 1.5)
    } else {
        // RELAXED: Less strict traditional ATH analysis
        smart_analysis.ath_analysis.is_safe_for_entry() || smart_analysis.entry_confidence > 0.6
    };

    // EMERGENCY OVERRIDE: Always allow trades for massive dips
    let emergency_ath_override = price_drop_percent >= emergency_dip_threshold * 0.7;

    if !is_ath_safe && !emergency_ath_override {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENHANCED_BUY_ATH_REJECT",
                &format!(
                    "❌ {} rejected by ATH analysis: OHLCV_safe={}, traditional_safe={}, emergency_override={}",
                    token.symbol.as_str(),
                    ohlcv_analysis.ath_analysis
                        .as_ref()
                        .map(|a| a.is_safe_for_entry)
                        .unwrap_or(false),
                    smart_analysis.ath_analysis.is_safe_for_entry(),
                    emergency_ath_override
                )
            );
        }
        return 0.0;
    }

    // Step 6: Calculate Enhanced Urgency Score with MULTIPLE OPPORTUNITY PATHS
    let traditional_urgency = if has_traditional_dip || has_micro_dip {
        let base_urgency = match smart_analysis.recommended_action {
            EntryAction::BuyNow => 1.4, // Increased from 1.2
            EntryAction::BuyOnDip => 1.0, // Increased from 0.8
            EntryAction::Monitor => 0.6, // Increased from 0.4
            EntryAction::Avoid => 0.2, // Changed from 0.0 - give some chance
        };

        // Micro-dip bonus for detecting small opportunities
        let micro_bonus = if has_micro_dip && !has_traditional_dip { 0.3 } else { 0.0 };

        (base_urgency + micro_bonus) * smart_analysis.entry_confidence
    } else {
        0.0
    };

    let ohlcv_urgency = if has_ohlcv_signals {
        ohlcv_analysis.overall_buy_urgency * 0.9 // Increased weight from 0.8
    } else {
        0.0
    };

    // Static price opportunity urgency
    let static_urgency = if has_static_opportunity {
        smart_analysis.entry_confidence * 0.8 // Moderate urgency for static prices
    } else {
        0.0
    };

    // Step 7: Combine Urgency Scores with INTELLIGENT MULTI-PATH WEIGHTING
    let combined_urgency = if has_traditional_dip && has_ohlcv_signals {
        // Both systems agree - highest confidence
        let max_urgency = traditional_urgency.max(ohlcv_urgency);
        let consensus_bonus = 0.4; // Increased from 0.3
        (max_urgency + consensus_bonus).min(2.0)
    } else if has_ohlcv_signals {
        // OHLCV-only signal - strong confidence
        (ohlcv_urgency * 1.2).min(2.0) // Increased from 1.1
    } else if has_static_opportunity {
        // Static price opportunity - patient entry
        static_urgency
    } else {
        // Traditional-only signal (including micro-dips)
        traditional_urgency
    };

    // Step 8: Apply ENHANCED BONUS SYSTEM for more opportunities
    let is_deepest = is_deepest_dip_moment(token);
    let deepest_bonus = if is_deepest { 0.25 } else { 0.0 }; // Increased from 0.2

    // Volume confirmation bonus from OHLCV
    let volume_bonus = if ohlcv_analysis.dip_signals.iter().any(|s| s.volume_confirmation) {
        0.2 // Increased from 0.15
    } else {
        0.0
    };

    // LIQUIDITY BONUS SYSTEM - Reward high liquidity tokens
    let liquidity_bonus = match
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    {
        liq if liq >= 500_000.0 => 0.25, // Ultra high liquidity bonus
        liq if liq >= 200_000.0 => 0.2, // High liquidity bonus
        liq if liq >= 100_000.0 => 0.15, // Good liquidity bonus
        liq if liq >= 50_000.0 => 0.1, // Medium liquidity bonus
        _ => 0.0,
    };

    // CONFIDENCE BONUS - Reward high confidence tokens
    let confidence_bonus = if smart_analysis.entry_confidence > 0.8 {
        0.2
    } else if smart_analysis.entry_confidence > 0.6 {
        0.1
    } else {
        0.0
    };

    let total_bonuses = deepest_bonus + volume_bonus + liquidity_bonus + confidence_bonus;
    let final_urgency = (combined_urgency + total_bonuses).min(2.0);

    // Step 9: RELAXED Final Safety Check - Allow more opportunities
    if
        final_urgency > 0.0 &&
        !ohlcv_analysis.is_safe_for_entry &&
        smart_analysis.entry_confidence < 0.5
    {
        // Only reject if both OHLCV and smart analysis are very negative
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "ENHANCED_BUY_FINAL_SAFETY",
                &format!(
                    "⚠️ {} urgency reduced due to combined safety concerns: {:.2} -> 0.0",
                    token.symbol.as_str(),
                    final_urgency
                )
            );
        }
        return 0.0;
    }

    if is_debug_entry_enabled() && final_urgency > 0.0 {
        log(
            LogTag::Trader,
            "ENHANCED_BUY_SUCCESS",
            &format!(
                "🚀 ENHANCED BUY SIGNAL for {}: Final={:.2} (Traditional={:.2}, OHLCV={:.2}, Static={:.2}, Bonuses={:.2})",
                token.symbol.as_str(),
                final_urgency,
                traditional_urgency,
                ohlcv_urgency,
                static_urgency,
                total_bonuses
            )
        );

        // Log detailed breakdown
        log(
            LogTag::Trader,
            "ENHANCED_BUY_BREAKDOWN",
            &format!(
                "   💎 {} Breakdown: Drop={:.2}% (threshold={:.1}%), Micro={}, Liquidity=${:.0}, Confidence={:.2}",
                token.symbol.as_str(),
                price_drop_percent,
                smart_analysis.dynamic_dip_threshold,
                has_micro_dip,
                token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0),
                smart_analysis.entry_confidence
            )
        );

        // Log OHLCV signal details
        for signal in &ohlcv_analysis.dip_signals {
            log(
                LogTag::Trader,
                "ENHANCED_BUY_OHLCV_DETAIL",
                &format!(
                    "   📈 OHLCV Signal: {} | Urgency={:.2} | Confidence={:.2} | {}",
                    signal.strategy_name,
                    signal.urgency,
                    signal.confidence,
                    signal.analysis_details
                )
            );
        }
    } else if is_debug_entry_enabled() && final_urgency == 0.0 {
        log(
            LogTag::Trader,
            "ENHANCED_BUY_NO_SIGNAL",
            &format!(
                "❌ {} NO BUY: Drop={:.2}% < {:.1}%, OHLCV_signals={}, Static={}, Confidence={:.2}",
                token.symbol.as_str(),
                price_drop_percent,
                smart_analysis.dynamic_dip_threshold,
                has_ohlcv_signals,
                has_static_opportunity,
                smart_analysis.entry_confidence
            )
        );
    }

    final_urgency
}

/// Smart Multi-Strategy Buy Signal Detection System (Original)
/// Uses intelligent ATH analysis, trend analysis, and liquidity-based dip detection
/// Returns urgency score from 0.0 (don't buy) to 2.0 (buy immediately)
pub fn should_buy(token: &Token, current_price: f64, prev_price: f64) -> f64 {
    use crate::global::is_debug_entry_enabled;

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_START",
            &format!(
                "🎯 SHOULD_BUY Analysis for {}: Current={:.10} SOL | Previous={:.10} SOL",
                token.symbol,
                current_price,
                prev_price
            )
        );
    }

    debug_trader_log(
        "SHOULD_BUY_START",
        &format!(
            "🧠 Smart buy analysis for {} ({}): current={:.10}, prev={:.10}",
            token.symbol,
            token.mint,
            current_price,
            prev_price
        )
    );

    // Step 1: Use centralized filtering system (basic validation only)
    if !should_buy_token(token) {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "BUY_SIGNAL_FILTER_REJECT",
                &format!("❌ {} rejected by pre-filtering system", token.symbol)
            );
        }
        debug_trader_log(
            "SHOULD_BUY_FILTER_REJECT",
            &format!("Token {} ({}) rejected by filtering system", token.symbol, token.mint)
        );
        return 0.0;
    }

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_FILTER_PASS",
            &format!("✅ {} passed pre-filtering system", token.symbol)
        );
    }

    debug_trader_log(
        "SHOULD_BUY_FILTER_OK",
        &format!("Token {} ({}) passed filtering", token.symbol, token.mint)
    );

    // Step 2: Smart entry analysis (ATH, trends, liquidity-based thresholds)
    let (is_smart_safe, smart_analysis) = is_token_safe_for_smart_entry(token);

    if !is_smart_safe {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "BUY_SIGNAL_SMART_REJECT",
                &format!(
                    "❌ {} rejected by smart entry analysis: ATH={:?} | Trend Safe={} | Confidence={:.2}",
                    token.symbol,
                    smart_analysis.ath_analysis.ath_danger_level,
                    smart_analysis.trend_analysis.is_safe_for_entry,
                    smart_analysis.entry_confidence
                )
            );
        }
        debug_trader_log(
            "SHOULD_BUY_SMART_REJECT",
            &format!(
                "Token {} ({}) rejected by smart entry analysis: ATH danger={:?}, Trend safe={}, Confidence={:.2}",
                token.symbol,
                token.mint,
                smart_analysis.ath_analysis.ath_danger_level,
                smart_analysis.trend_analysis.is_safe_for_entry,
                smart_analysis.entry_confidence
            )
        );
        return 0.0;
    }

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_SMART_PASS",
            &format!(
                "✅ {} passed smart entry analysis: Action={:?} | Confidence={:.2} | Dip Threshold={:.1}%",
                token.symbol,
                smart_analysis.recommended_action,
                smart_analysis.entry_confidence,
                smart_analysis.dynamic_dip_threshold
            )
        );
    }

    debug_trader_log(
        "SHOULD_BUY_SMART_OK",
        &format!(
            "Token {} ({}) passed smart analysis: Action={:?}, Confidence={:.2}, Dip threshold={:.1}%",
            token.symbol,
            token.mint,
            smart_analysis.recommended_action,
            smart_analysis.entry_confidence,
            smart_analysis.dynamic_dip_threshold
        )
    );

    // Step 3: Price validation
    if current_price <= 0.0 || prev_price <= 0.0 {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "BUY_SIGNAL_PRICE_INVALID",
                &format!(
                    "❌ {} has invalid prices: current={:.10} | previous={:.10}",
                    token.symbol,
                    current_price,
                    prev_price
                )
            );
        }
        debug_trader_log(
            "SHOULD_BUY_PRICE_INVALID",
            &format!(
                "Token {} ({}) has invalid prices: current={:.10}, prev={:.10}",
                token.symbol,
                token.mint,
                current_price,
                prev_price
            )
        );
        return 0.0;
    }

    // Step 4: Calculate price drop with liquidity-adjusted threshold
    let price_change_percent = ((current_price - prev_price) / prev_price) * 100.0;
    let price_drop_percent = -price_change_percent; // Convert to positive for drops

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_PRICE_ANALYSIS",
            &format!(
                "📊 {} Price Movement: Drop={:.2}% | Threshold={:.1}% | Sufficient={}",
                token.symbol,
                price_drop_percent,
                smart_analysis.dynamic_dip_threshold,
                price_drop_percent >= smart_analysis.dynamic_dip_threshold
            )
        );
    }

    debug_trader_log(
        "SHOULD_BUY_PRICE_ANALYSIS",
        &format!(
            "Token {} ({}) price drop: {:.2}% (threshold: {:.1}%)",
            token.symbol,
            token.mint,
            price_drop_percent,
            smart_analysis.dynamic_dip_threshold
        )
    );

    // Step 5: Check if this is a valid dip based on liquidity tier
    if !is_valid_dip_for_liquidity(token, price_drop_percent) {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "BUY_SIGNAL_DIP_INSUFFICIENT",
                &format!(
                    "❌ {} dip insufficient: {:.2}% < {:.1}% threshold (Liquidity Tier: {:?})",
                    token.symbol,
                    price_drop_percent,
                    smart_analysis.dynamic_dip_threshold,
                    smart_analysis.liquidity_tier
                )
            );
        }
        debug_trader_log(
            "SHOULD_BUY_INSUFFICIENT_DIP",
            &format!(
                "Token {} ({}) insufficient dip: {:.2}% < {:.1}% (liquidity tier: {:?})",
                token.symbol,
                token.mint,
                price_drop_percent,
                smart_analysis.dynamic_dip_threshold,
                smart_analysis.liquidity_tier
            )
        );
        return 0.0;
    }

    // Step 6: Check if we're in the deepest moment of the dip (5min trend analysis)
    let is_deepest = is_deepest_dip_moment(token);
    let deepest_bonus = if is_deepest { 0.3 } else { 0.0 };

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_DEEPEST_CHECK",
            &format!(
                "🕳️ {} Deepest Dip Check: Is Deepest={} | Bonus={:.2}",
                token.symbol,
                is_deepest,
                deepest_bonus
            )
        );
    }

    // Step 7: Calculate urgency based on smart analysis
    let base_urgency = match smart_analysis.recommended_action {
        EntryAction::BuyNow => 1.5,
        EntryAction::BuyOnDip => 1.0,
        EntryAction::Monitor => 0.5,
        EntryAction::Avoid => 0.0,
    };

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_BASE_URGENCY",
            &format!(
                "⚡ {} Base Urgency: {:.2} (from action: {:?})",
                token.symbol,
                base_urgency,
                smart_analysis.recommended_action
            )
        );
    }

    // Step 8: Apply confidence and trend momentum multipliers
    let confidence_multiplier = smart_analysis.entry_confidence;
    let momentum_multiplier = 1.0 + smart_analysis.trend_analysis.momentum_score * 0.3;

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_MULTIPLIERS",
            &format!(
                "🔢 {} Multipliers: Confidence={:.2} | Momentum={:.2} (from score: {:.2})",
                token.symbol,
                confidence_multiplier,
                momentum_multiplier,
                smart_analysis.trend_analysis.momentum_score
            )
        );
    }

    // Step 9: Apply dip intensity bonus (larger dips = higher urgency, capped)
    let dip_intensity_bonus = (price_drop_percent / smart_analysis.dynamic_dip_threshold - 1.0)
        .max(0.0)
        .min(0.5); // Max 0.5 bonus for very large dips

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_DIP_BONUS",
            &format!(
                "🎁 {} Dip Intensity Bonus: {:.2} (drop={:.2}% / threshold={:.1}% = ratio {:.2})",
                token.symbol,
                dip_intensity_bonus,
                price_drop_percent,
                smart_analysis.dynamic_dip_threshold,
                price_drop_percent / smart_analysis.dynamic_dip_threshold
            )
        );
    }

    let final_urgency = (
        base_urgency * confidence_multiplier * momentum_multiplier +
        deepest_bonus +
        dip_intensity_bonus
    ).min(2.0);

    if is_debug_entry_enabled() {
        log(
            LogTag::Trader,
            "BUY_SIGNAL_FINAL_RESULT",
            &format!(
                "🏁 {} FINAL RESULT: Urgency={:.3} | Will Buy={} (threshold=0.0)",
                token.symbol,
                final_urgency,
                if final_urgency > 0.0 {
                    "✅ YES"
                } else {
                    "❌ NO"
                }
            )
        );

        if final_urgency > 0.0 {
            log(
                LogTag::Trader,
                "BUY_SIGNAL_BREAKDOWN",
                &format!(
                    "   📋 Calculation: ({:.2} × {:.2} × {:.2}) + {:.2} + {:.2} = {:.3}",
                    base_urgency,
                    confidence_multiplier,
                    momentum_multiplier,
                    deepest_bonus,
                    dip_intensity_bonus,
                    final_urgency
                )
            );
        }
    }

    debug_trader_log(
        "SHOULD_BUY_FINAL_CALCULATION",
        &format!(
            "Token {} ({}): base={:.2}, confidence={:.2}, momentum={:.2}, deepest={:.2}, dip_bonus={:.2}, final={:.3}",
            token.symbol,
            token.mint,
            base_urgency,
            confidence_multiplier,
            momentum_multiplier,
            deepest_bonus,
            dip_intensity_bonus,
            final_urgency
        )
    );

    if final_urgency > 0.0 {
        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "BUY_SIGNAL_SUCCESS",
                &format!(
                    "🎯 BUY SIGNAL GENERATED for {}: Urgency={:.2} | Action={:?} | Dip={:.1}% | Profit Target={:.1}%-{:.1}%",
                    token.symbol,
                    final_urgency,
                    smart_analysis.recommended_action,
                    price_drop_percent,
                    smart_analysis.profit_target_range.0,
                    smart_analysis.profit_target_range.1
                )
            );
        }

        log(
            LogTag::Trader,
            "SMART_BUY_SIGNAL",
            &format!(
                "🎯 Smart buy signal for {} ({}): urgency={:.2}, action={:?}, dip={:.1}%, profit target={:.1}%-{:.1}%",
                token.symbol,
                token.mint,
                final_urgency,
                smart_analysis.recommended_action,
                price_drop_percent,
                smart_analysis.profit_target_range.0,
                smart_analysis.profit_target_range.1
            )
        );
    }

    final_urgency
}

/// Dip detection strategy result
#[derive(Debug, Clone)]
struct DipSignal {
    strategy_name: String,
    urgency: f64,
    drop_percent: f64,
    confidence: f64,
    details: String,
}























/// Checks if entry is allowed based on historical position data for this token
/// Returns true only if current price is below both:
/// 1. Average entry price from past closed positions
/// 2. Maximum price this token has ever reached
pub fn is_entry_allowed_by_historical_data(mint: &str, current_price: f64) -> bool {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        // Find all closed positions for this token
        let token_positions: Vec<&Position> = positions
            .iter()
            .filter(|p| p.mint == mint && p.exit_price.is_some())
            .collect();

        // If no historical positions, allow entry (first time seeing this token)
        if token_positions.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "No historical positions found for token {}, allowing entry at {:.12}",
                    mint,
                    current_price
                )
            );
            return true;
        }

        // Calculate average entry price from past positions
        let total_entry_prices: f64 = token_positions
            .iter()
            .map(|p| p.effective_entry_price.unwrap_or(p.entry_price))
            .sum();
        let average_entry_price = total_entry_prices / (token_positions.len() as f64);

        // Find maximum price this token has ever reached
        let max_historical_price = token_positions
            .iter()
            .map(|p| p.price_highest)
            .fold(0.0, f64::max);

        // Log the analysis
        log(
            LogTag::Trader,
            "ANALYSIS",
            &format!(
                "Historical analysis for {}: Current: {:.12}, Avg Entry: {:.12}, Max Ever: {:.12}, Positions: {}",
                mint,
                current_price,
                average_entry_price,
                max_historical_price,
                token_positions.len()
            )
        );

        // Allow entry only if current price is below both thresholds
        let below_avg_entry = current_price < average_entry_price;
        let below_max_price = current_price < max_historical_price;

        if !below_avg_entry {
            log(
                LogTag::Trader,
                "BLOCK",
                &format!(
                    "Entry blocked: Current price {:.12} >= average entry price {:.12}",
                    current_price,
                    average_entry_price
                )
            );
        }

        if !below_max_price {
            log(
                LogTag::Trader,
                "BLOCK",
                &format!(
                    "Entry blocked: Current price {:.12} >= maximum historical price {:.12}",
                    current_price,
                    max_historical_price
                )
            );
        }

        if below_avg_entry && below_max_price {
            log(
                LogTag::Trader,
                "ALLOW",
                &format!(
                    "Entry allowed: Current price {:.12} < avg entry {:.12} and < max price {:.12}",
                    current_price,
                    average_entry_price,
                    max_historical_price
                )
            );
        }

        return below_avg_entry && below_max_price;
    } else {
        log(
            LogTag::Trader,
            "ERROR",
            "Could not acquire lock on SAVED_POSITIONS for historical analysis"
        );
        return false; // Conservative: don't allow entry if we can't analyze
    }
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    log(LogTag::Trader, "STARTUP", "🚀 Starting monitor_new_entries task");

    'outer: loop {
        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        // Update position tracking in price service
        let position_update_start = std::time::Instant::now();
        update_position_tracking_in_service().await;
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "✅ Position tracking updated in {:.1}ms",
                    position_update_start.elapsed().as_millis()
                )
            );
        }

        // Ensure we have tokens to work with
        if is_debug_trader_enabled() {
            log(LogTag::Trader, "DEBUG", "🪙 Ensuring tokens are populated...");
        }
        let token_populate_start = std::time::Instant::now();
        ensure_tokens_populated().await;
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "✅ Tokens populated in {:.1}ms",
                    token_populate_start.elapsed().as_millis()
                )
            );
        }

        let mut tokens: Vec<_> = {
            // Get tokens from safe system
            if is_debug_trader_enabled() {
                log(LogTag::Trader, "DEBUG", "📡 Getting tokens from safe system...");
            }
            let token_fetch_start = std::time::Instant::now();
            let tokens_from_module = get_tokens_from_safe_system().await;
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "✅ Got {} tokens from safe system in {:.1}ms",
                        tokens_from_module.len(),
                        token_fetch_start.elapsed().as_millis()
                    )
                );
            }

            // Log total tokens available
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Total tokens from safe system: {}", tokens_from_module.len())
                        .dimmed()
                        .to_string()
                );
            }

            // Add debug info about token prices and update times
            if is_debug_trader_enabled() {
                log(LogTag::Trader, "DEBUG", "🏷️ Checking price service for sample tokens...");
            }
            let price_check_start = std::time::Instant::now();
            debug_trader_log(
                "TOKEN_PRICE_DEBUG",
                &format!(
                    "DEBUG: First 3 tokens price info - {} tokens total",
                    tokens_from_module.len()
                )
            );

            for (i, token) in tokens_from_module.iter().take(3).enumerate() {
                let price_lookup_start = std::time::Instant::now();
                let price_from_service = get_token_price_blocking_safe(&token.mint).await;
                debug_trader_log(
                    "TOKEN_PRICE_SAMPLE",
                    &format!(
                        "Token {}: {} ({}) - price_service={:?} (lookup took {:.1}ms)",
                        i + 1,
                        token.symbol,
                        token.mint,
                        price_from_service,
                        price_lookup_start.elapsed().as_millis()
                    )
                );
            }
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "✅ Price service check completed in {:.1}ms",
                        price_check_start.elapsed().as_millis()
                    )
                );
            } // Include all tokens - we want to trade on existing tokens with updated info
            // The discovery system ensures tokens are updated with fresh data before trading
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "Using all {} tokens for trading (startup filter removed)",
                        tokens_from_module.len()
                    )
                        .dimmed()
                        .to_string()
                );
            }

            // Count tokens with liquidity data
            let with_liquidity = tokens_from_module
                .iter()
                .filter(|token| {
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
                })
                .count();

            if with_liquidity > 0 {
                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Processing {} tokens with liquidity", with_liquidity)
                );
            }

            tokens_from_module
        };

        // Sort tokens by liquidity in descending order (highest liquidity first)
        tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Safety check - if processing is taking too long, log it
        if cycle_start.elapsed() > Duration::from_secs(5) {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token sorting took too long: {:?}", cycle_start.elapsed())
            );
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!(
                "Checking {} tokens for entry opportunities (sorted by liquidity)",
                tokens.len()
            )
                .dimmed()
                .to_string()
        );

        // Use centralized filtering system to get eligible tokens
        use crate::filtering::{ filter_tokens_with_reasons, get_filtering_stats };

        let (eligible_tokens, rejected_tokens) = filter_tokens_with_reasons(&tokens);

        // Log filtering statistics
        let (total, passed, pass_rate) = get_filtering_stats(&tokens);
        log(
            LogTag::Trader,
            "FILTER_STATS",
            &format!("Token filtering: {}/{} passed ({:.1}% pass rate)", passed, total, pass_rate)
        );

        // Use eligible tokens for trading
        tokens = eligible_tokens;

        // Early return if no tokens to process
        if tokens.is_empty() {
            log(LogTag::Trader, "INFO", "No tokens to process, skipping token checking cycle");

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break;
            }
            continue;
        }

        log(
            LogTag::Trader,
            "DEBUG",
            &format!("🔍 Starting to process {} eligible tokens", tokens.len())
        );

        // Use a semaphore to limit the number of concurrent token checks
        // This balances between parallelism and not overwhelming external APIs
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(5)); // Reduced to 5 concurrent checks to avoid overwhelming

        // Log filtering summary
        log(LogTag::Trader, "DEBUG", "📊 Logging filtering summary...");
        let summary_start = std::time::Instant::now();
        log_filtering_summary(&tokens);
        log(
            LogTag::Trader,
            "DEBUG",
            &format!("✅ Filtering summary logged in {:.1}ms", summary_start.elapsed().as_millis())
        );

        // Sync OHLCV watch list with trader tokens (run async to not block trading)
        if is_debug_trader_enabled() {
            log(LogTag::Trader, "DEBUG", "📈 Syncing OHLCV watch list with filtered tokens...");
        }
        let ohlcv_sync_start = std::time::Instant::now();
        tokio::spawn(async move {
            if let Err(e) = sync_watch_list_with_trader().await {
                log(LogTag::Trader, "WARN", &format!("OHLCV sync failed: {}", e));
            }
        });
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "✅ OHLCV sync task spawned in {:.1}ms",
                    ohlcv_sync_start.elapsed().as_millis()
                )
            );
        }

        // Process all tokens in parallel with concurrent tasks
        let mut handles = Vec::new();

        // Get the total token count before starting the loop
        let total_tokens = tokens.len();
        log(
            LogTag::Trader,
            "DEBUG",
            &format!("🚀 Starting parallel processing of {} tokens", total_tokens)
        );

        let token_processing_start = std::time::Instant::now();
        // Note: tokens are still sorted by liquidity from highest to lowest
        for (index, token) in tokens.iter().enumerate() {
            // Check for shutdown before spawning tasks
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break 'outer;
            }

            // Get permit from semaphore to limit concurrency with timeout
            let permit = match
                tokio::time::timeout(
                    Duration::from_secs(SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                    semaphore.clone().acquire_owned()
                ).await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(e)) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to acquire semaphore permit: {}", e)
                    );
                    continue;
                }
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!("Semaphore acquire timed out after {} seconds", SEMAPHORE_ACQUIRE_TIMEOUT_SECS)
                    );
                    continue;
                }
            };

            // Clone necessary variables for the task
            let token = token.clone();
            let shutdown_clone = shutdown.clone();

            // Spawn a new task for this token with overall timeout
            let handle = tokio::spawn(async move {
                // Keep the permit alive for the duration of this task
                let _permit = permit; // This will be automatically dropped when the task completes

                // Check for shutdown before starting task
                if check_shutdown_or_delay(&shutdown_clone, Duration::from_millis(1)).await {
                    return None;
                }

                // Clone the symbol for error logging before moving token into timeout
                let token_symbol = token.symbol.clone();

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(TOKEN_CHECK_TASK_TIMEOUT_SECS), async {
                        let task_start = std::time::Instant::now();

                        // Use centralized price service instead of direct token field access
                        debug_trader_log(
                            "TASK_TIMING",
                            &format!("Token {} starting price fetch", token.symbol)
                        );
                        let price_start = std::time::Instant::now();

                        if
                            let Some(current_price) = get_token_price_blocking_safe(
                                &token.mint
                            ).await
                        {
                            debug_trader_log(
                                "TASK_TIMING",
                                &format!(
                                    "Token {} price fetch took {:.2}ms",
                                    token.symbol,
                                    price_start.elapsed().as_millis()
                                )
                            );
                            // CRITICAL: Validate price is actually loaded and valid before any trading
                            if current_price <= 0.0 || !current_price.is_finite() {
                                debug_trader_log(
                                    "PRICE_INVALID",
                                    &format!(
                                        "Token {} ({}): INVALID PRICE DETECTED - not trading. Price = {:.10}",
                                        token.symbol,
                                        token.mint,
                                        current_price
                                    )
                                );
                                return None;
                            }

                            debug_trader_log(
                                "PRICE_SOURCE",
                                &format!(
                                    "Token {} ({}): current price from price service = {:.10}",
                                    token.symbol,
                                    token.mint,
                                    current_price
                                )
                            );

                            // Use centralized filtering system
                            debug_trader_log(
                                "TASK_TIMING",
                                &format!("Token {} starting filtering", token.symbol)
                            );
                            let filter_start = std::time::Instant::now();

                            if !should_buy_token(&token) {
                                // Token was filtered out, skip processing
                                return None;
                            }

                            // Check for shutdown after filtering
                            if
                                check_shutdown_or_delay(
                                    &shutdown_clone,
                                    Duration::from_millis(1)
                                ).await
                            {
                                return None;
                            }

                            debug_trader_log(
                                "TASK_TIMING",
                                &format!(
                                    "Token {} filtering took {:.2}ms",
                                    token.symbol,
                                    filter_start.elapsed().as_millis()
                                )
                            );

                            let liquidity_usd = token.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);

                            // Update price history with proper error handling and timeout
                            let now = Utc::now();
                            match
                                tokio::time::timeout(
                                    Duration::from_millis(PRICE_CACHE_LOCK_TIMEOUT_MS),
                                    async {
                                        PRICE_HISTORY_24H.try_lock()
                                    }
                                ).await
                            {
                                Ok(Ok(mut hist)) => {
                                    let entry = hist
                                        .entry(token.mint.clone())
                                        .or_insert_with(Vec::new);
                                    entry.push((now, current_price));

                                    // Retain only last 24h
                                    let cutoff =
                                        now - ChronoDuration::hours(PRICE_HISTORY_DURATION_HOURS);
                                    entry.retain(|(ts, _)| *ts >= cutoff);
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire price history lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Check for shutdown after price history update
                            if
                                check_shutdown_or_delay(
                                    &shutdown_clone,
                                    Duration::from_millis(1)
                                ).await
                            {
                                return None;
                            }

                            // Check for entry opportunity using should_buy function
                            let mut buy_urgency = 0.0;

                            // First, get the previous price and release the lock
                            let prev_price = {
                                match
                                    tokio::time::timeout(
                                        Duration::from_millis(PRICE_CACHE_LOCK_TIMEOUT_MS),
                                        async {
                                            LAST_PRICES.try_lock()
                                        }
                                    ).await
                                {
                                    Ok(Ok(last_prices)) => { last_prices.get(&token.mint).copied() }
                                    _ => None,
                                }
                            };

                            if let Some(prev_price) = prev_price {
                                if prev_price > 0.0 {
                                    debug_trader_log(
                                        "TOKEN_CHECK_PRICES",
                                        &format!(
                                            "Token {} ({}): checking buy signal with current={:.10}, prev={:.10}",
                                            token.symbol,
                                            token.mint,
                                            current_price,
                                            prev_price
                                        )
                                    );

                                    // Check if prices are identical (indicating no price update)
                                    if (current_price - prev_price).abs() < 0.000000000001 {
                                        debug_trader_log(
                                            "PRICE_UPDATE_ISSUE",
                                            &format!(
                                                "Token {} ({}): IDENTICAL PRICES detected! Current={:.10} == Prev={:.10} - No price movement detected, this indicates price cache not updating",
                                                token.symbol,
                                                token.mint,
                                                current_price,
                                                prev_price
                                            )
                                        );
                                    }

                                    // Rugcheck safety validation - block rugged or high-risk tokens
                                    debug_trader_log(
                                        "TASK_TIMING",
                                        &format!("Token {} starting rugcheck", token.symbol)
                                    );
                                    let rugcheck_start = std::time::Instant::now();

                                    match get_token_rugcheck_data_safe(&token.mint).await {
                                        Ok(Some(rugcheck_data)) => {
                                            // Block tokens that are detected as rugged
                                            if rugcheck_data.rugged.unwrap_or(false) {
                                                debug_trader_log(
                                                    "RUGCHECK_REJECT_RUGGED",
                                                    &format!(
                                                        "Token {} ({}) rejected - detected as rugged",
                                                        token.symbol,
                                                        token.mint
                                                    )
                                                );
                                                buy_urgency = 0.0;
                                            } else {
                                                // Use the proper rugcheck safety validation
                                                let rugcheck_score = rugcheck_data.score_normalised
                                                    .or(rugcheck_data.score)
                                                    .unwrap_or(50);

                                                if !is_token_safe_for_trading(&rugcheck_data) {
                                                    let high_risk_issues = get_high_risk_issues(
                                                        &rugcheck_data
                                                    );
                                                    debug_trader_log(
                                                        "RUGCHECK_REJECT_UNSAFE",
                                                        &format!(
                                                            "Token {} ({}) rejected - failed safety check (risk score: {}, issues: {:?})",
                                                            token.symbol,
                                                            token.mint,
                                                            rugcheck_score,
                                                            high_risk_issues
                                                        )
                                                    );
                                                    buy_urgency = 0.0;
                                                } else if
                                                    // Block tokens with critical risks (additional check)
                                                    let Some(risks) = &rugcheck_data.risks
                                                {
                                                    let critical_risks = risks
                                                        .iter()
                                                        .filter(|r| {
                                                            r.level
                                                                .as_ref()
                                                                .map(
                                                                    |l|
                                                                        l.to_lowercase() ==
                                                                        "critical"
                                                                )
                                                                .unwrap_or(false)
                                                        })
                                                        .count();

                                                    if critical_risks > 0 {
                                                        debug_trader_log(
                                                            "RUGCHECK_REJECT_CRITICAL",
                                                            &format!(
                                                                "Token {} ({}) rejected - {} critical risks detected",
                                                                token.symbol,
                                                                token.mint,
                                                                critical_risks
                                                            )
                                                        );
                                                        buy_urgency = 0.0;
                                                    } else {
                                                        // Use the enhanced OHLCV-based should_buy function
                                                        buy_urgency = should_buy_enhanced(
                                                            &token,
                                                            current_price,
                                                            prev_price
                                                        ).await;

                                                        debug_trader_log(
                                                            "RUGCHECK_OK",
                                                            &format!(
                                                                "Token {} ({}) passed rugcheck validation (risk score: {})",
                                                                token.symbol,
                                                                token.mint,
                                                                rugcheck_score
                                                            )
                                                        );
                                                    }
                                                } else {
                                                    // Use the enhanced OHLCV-based should_buy function
                                                    buy_urgency = should_buy_enhanced(
                                                        &token,
                                                        current_price,
                                                        prev_price
                                                    ).await;

                                                    debug_trader_log(
                                                        "RUGCHECK_OK",
                                                        &format!(
                                                            "Token {} ({}) passed rugcheck validation (risk score: {})",
                                                            token.symbol,
                                                            token.mint,
                                                            rugcheck_score
                                                        )
                                                    );
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            debug_trader_log(
                                                "RUGCHECK_MISSING",
                                                &format!(
                                                    "Token {} ({}) has no rugcheck data - proceeding with caution",
                                                    token.symbol,
                                                    token.mint
                                                )
                                            );

                                            // Use the enhanced OHLCV-based should_buy function
                                            buy_urgency = should_buy_enhanced(
                                                &token,
                                                current_price,
                                                prev_price
                                            ).await;
                                        }
                                        Err(e) => {
                                            debug_trader_log(
                                                "RUGCHECK_ERROR",
                                                &format!(
                                                    "Token {} ({}) rugcheck error: {} - proceeding with caution",
                                                    token.symbol,
                                                    token.mint,
                                                    e
                                                )
                                            );

                                            // Use the enhanced OHLCV-based should_buy function
                                            buy_urgency = should_buy_enhanced(
                                                &token,
                                                current_price,
                                                prev_price
                                            ).await;
                                        }
                                    }

                                    debug_trader_log(
                                        "TASK_TIMING",
                                        &format!(
                                            "Token {} rugcheck took {:.2}ms",
                                            token.symbol,
                                            rugcheck_start.elapsed().as_millis()
                                        )
                                    );

                                    // Check for shutdown after rugcheck processing
                                    if
                                        check_shutdown_or_delay(
                                            &shutdown_clone,
                                            Duration::from_millis(1)
                                        ).await
                                    {
                                        return None;
                                    }

                                    debug_trader_log(
                                        "TOKEN_CHECK_RESULT",
                                        &format!(
                                            "Token {} ({}): buy urgency result: {:.3}",
                                            token.symbol,
                                            token.mint,
                                            buy_urgency
                                        )
                                    );
                                } else {
                                    debug_trader_log(
                                        "TOKEN_CHECK_INVALID_PREV",
                                        &format!(
                                            "Token {} ({}): invalid prev_price: {:.10}",
                                            token.symbol,
                                            token.mint,
                                            prev_price
                                        )
                                    );
                                }
                            } else {
                                debug_trader_log(
                                    "TOKEN_CHECK_NO_PREV",
                                    &format!(
                                        "Token {} ({}): no previous price available",
                                        token.symbol,
                                        token.mint
                                    )
                                );
                            }

                            // Update price in cache
                            match
                                tokio::time::timeout(
                                    Duration::from_millis(PRICE_CACHE_LOCK_TIMEOUT_MS),
                                    async {
                                        LAST_PRICES.try_lock()
                                    }
                                ).await
                            {
                                Ok(Ok(mut last_prices)) => {
                                    // Add debug info before updating price
                                    debug_trader_log(
                                        "PRICE_UPDATE_BEFORE",
                                        &format!(
                                            "Token {} ({}): BEFORE UPDATE - old_price_in_cache={:.10}, new_current_price={:.10}",
                                            token.symbol,
                                            token.mint,
                                            last_prices.get(&token.mint).copied().unwrap_or(0.0),
                                            current_price
                                        )
                                    );

                                    last_prices.insert(token.mint.clone(), current_price);

                                    debug_trader_log(
                                        "PRICE_UPDATE_AFTER",
                                        &format!(
                                            "Token {} ({}): AFTER UPDATE - cache now contains: {:.10}",
                                            token.symbol,
                                            token.mint,
                                            current_price
                                        )
                                    );
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire last_prices lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Return the token and price if buy signal detected
                            if buy_urgency > 0.0 {
                                use crate::global::is_debug_entry_enabled;

                                if is_debug_entry_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "BUY_DECISION_POSITIVE",
                                        &format!(
                                            "🚀 BUY DECISION: {} has urgency {:.3} > 0.0 → Will attempt to buy!",
                                            token.symbol,
                                            buy_urgency
                                        )
                                    );
                                }

                                // Check for shutdown before attempting to buy
                                if
                                    check_shutdown_or_delay(
                                        &shutdown_clone,
                                        Duration::from_millis(1)
                                    ).await
                                {
                                    if is_debug_entry_enabled() {
                                        log(
                                            LogTag::Trader,
                                            "BUY_DECISION_SHUTDOWN",
                                            &format!(
                                                "❌ {} buy cancelled due to shutdown",
                                                token.symbol
                                            )
                                        );
                                    }
                                    return None;
                                }

                                let change = if let Ok(last_prices) = LAST_PRICES.try_lock() {
                                    if let Some(&prev_price) = last_prices.get(&token.mint) {
                                        if prev_price > 0.0 {
                                            ((current_price - prev_price) / prev_price) * 100.0
                                        } else {
                                            0.0
                                        }
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                };

                                if is_debug_entry_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "BUY_DECISION_FINAL",
                                        &format!(
                                            "✅ {} PASSED ALL CHECKS → Returning for buy execution (price: {:.10}, change: {:.2}%)",
                                            token.symbol,
                                            current_price,
                                            change
                                        )
                                    );
                                }

                                return Some((token, current_price, change));
                            } else {
                                use crate::global::is_debug_entry_enabled;

                                if is_debug_entry_enabled() {
                                    log(
                                        LogTag::Trader,
                                        "BUY_DECISION_NEGATIVE",
                                        &format!(
                                            "❌ {} has urgency {:.3} ≤ 0.0 → No buy signal generated",
                                            token.symbol,
                                            buy_urgency
                                        )
                                    );
                                }
                            }
                        } else {
                            // Price is not loaded - do not attempt any trading
                            debug_trader_log(
                                "PRICE_NOT_LOADED",
                                &format!(
                                    "Token {} ({}): PRICE NOT LOADED - skipping trading. Price service returned None",
                                    token.symbol,
                                    token.mint
                                )
                            );
                        }

                        debug_trader_log(
                            "TASK_TIMING",
                            &format!(
                                "Token {} task completed in {:.2}ms",
                                token.symbol,
                                task_start.elapsed().as_millis()
                            )
                        );

                        None
                    }).await
                {
                    Ok(result) => result,
                    Err(_) => {
                        // Task timed out
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "Token check task for {} timed out after {} seconds",
                                token_symbol,
                                TOKEN_CHECK_TASK_TIMEOUT_SECS
                            )
                        );
                        None
                    }
                }
            });

            handles.push(handle);
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Successfully spawned {} token checking tasks", handles.len())
                .dimmed()
                .to_string()
        );

        // Process the results of all tasks with overall timeout
        let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
            // This maintains the priority of processing high-liquidity tokens first
            log(
                LogTag::Trader,
                "INFO",
                &format!("Waiting for {} token checks to complete", handles.len())
                    .dimmed()
                    .to_string()
            );

            let mut opportunities = Vec::new();

            for handle in handles {
                // Skip any tasks that failed or if shutdown signal received
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "new entries monitor shutting down during result collection..."
                    );
                    return opportunities; // Return what we have so far
                }

                // Add timeout for each handle to prevent getting stuck on a single task
                match tokio::time::timeout(Duration::from_secs(120), handle).await {
                    Ok(task_result) => {
                        match task_result {
                            Ok(Some((token, price, percent_change))) => {
                                opportunities.push((token, price, percent_change));
                            }
                            Ok(None) => {
                                // No opportunity found for this token, continue
                            }
                            Err(e) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Token check task failed: {}", e)
                                );
                            }
                        }
                    }
                    Err(_) => {
                        // Task timed out after 5 seconds
                        log(LogTag::Trader, "WARN", "Token check task timed out after 5 seconds");
                    }
                }
            }

            opportunities
        }).await;

        let mut opportunities = match collection_result {
            Ok(opportunities) => opportunities,
            Err(_) => {
                log(LogTag::Trader, "ERROR", "Token check collection timed out after 60 seconds");
                Vec::new() // Return empty if timeout
            }
        };

        // Sort opportunities by liquidity again to ensure priority
        opportunities.sort_by(|(a, _, _), (b, _, _)| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Enhanced debug logging for opportunities
        use crate::global::is_debug_entry_enabled;

        if is_debug_entry_enabled() {
            log(
                LogTag::Trader,
                "OPPORTUNITIES_SUMMARY",
                &format!(
                    "🎯 Entry Opportunities Found: {} tokens passed all checks",
                    opportunities.len()
                )
            );

            if opportunities.is_empty() {
                log(
                    LogTag::Trader,
                    "OPPORTUNITIES_NONE",
                    "❌ No tokens generated buy signals this cycle"
                );
            } else {
                log(
                    LogTag::Trader,
                    "OPPORTUNITIES_LIST",
                    &format!("📋 Tokens ready for purchase:")
                );

                for (i, (token, price, change)) in opportunities.iter().enumerate() {
                    let liquidity = token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0);

                    log(
                        LogTag::Trader,
                        "OPPORTUNITY_DETAIL",
                        &format!(
                            "   {}. {} - Price: {:.10} SOL | Change: {:.2}% | Liquidity: ${:.0}",
                            i + 1,
                            token.symbol,
                            price,
                            change,
                            liquidity
                        )
                    );
                }
            }
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Found {} potential entry opportunities", opportunities.len())
        );

        // Log the total time taken for the token checking cycle
        log(
            LogTag::Trader,
            "INFO",
            &format!("Token checking cycle completed in {:?}", cycle_start.elapsed())
                .dimmed()
                .to_string()
        );

        // Process opportunities concurrently while respecting position limits
        if !opportunities.is_empty() {
            let current_open_count = get_open_positions_count();
            let available_slots = MAX_OPEN_POSITIONS.saturating_sub(current_open_count);

            if is_debug_entry_enabled() {
                log(
                    LogTag::Trader,
                    "POSITION_CHECK",
                    &format!(
                        "💼 Position Limits: Current Open: {} | Max: {} | Available Slots: {}",
                        current_open_count,
                        MAX_OPEN_POSITIONS,
                        available_slots
                    )
                );
            }

            if available_slots == 0 {
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Trader,
                        "POSITION_LIMIT_REACHED",
                        &format!(
                            "🚫 POSITION LIMIT REACHED: Cannot buy any tokens - {} opportunities skipped",
                            opportunities.len()
                        )
                    );
                }

                log(
                    LogTag::Trader,
                    "LIMIT",
                    &format!(
                        "Maximum open positions already reached ({}/{}). Skipping all opportunities.",
                        current_open_count,
                        MAX_OPEN_POSITIONS
                    )
                );
            } else {
                // Capture the original count before consuming the vector
                let total_opportunities_count = opportunities.len();

                // Limit opportunities to available slots
                let opportunities_to_process = opportunities
                    .into_iter()
                    .take(available_slots)
                    .collect::<Vec<_>>();

                if is_debug_entry_enabled() {
                    log(
                        LogTag::Trader,
                        "PROCESSING_OPPORTUNITIES",
                        &format!(
                            "🚀 PROCESSING {} opportunities for purchase (limited by available slots)",
                            opportunities_to_process.len()
                        )
                    );

                    if opportunities_to_process.len() < total_opportunities_count {
                        log(
                            LogTag::Trader,
                            "OPPORTUNITIES_LIMITED",
                            &format!(
                                "⚠️ Limited by position slots: Processing {} out of {} opportunities",
                                opportunities_to_process.len(),
                                total_opportunities_count
                            )
                        );
                    }
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Processing {} opportunities concurrently (available slots: {}, current open: {})",
                        opportunities_to_process.len(),
                        available_slots,
                        current_open_count
                    )
                );

                // Use a semaphore to limit concurrent buy transactions
                use tokio::sync::Semaphore;
                let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent buys

                let mut handles = Vec::new();

                // Process all buy orders concurrently
                for (token, price, percent_change) in opportunities_to_process {
                    // Check for shutdown before spawning tasks
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "new entries monitor shutting down during buy processing..."
                        );
                        break;
                    }

                    // Get permit from semaphore to limit concurrency with timeout
                    let permit = match
                        tokio::time::timeout(
                            Duration::from_secs(120),
                            semaphore.clone().acquire_owned()
                        ).await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to acquire semaphore permit for buy: {}", e)
                            );
                            continue;
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                "Semaphore acquire timed out for buy operation"
                            );
                            continue;
                        }
                    };

                    // Clone shutdown for use in the spawned task
                    let shutdown_for_task = shutdown.clone();
                    let handle = tokio::spawn(async move {
                        let _permit = permit; // Keep permit alive for duration of task

                        let token_symbol = token.symbol.clone();

                        // Check for shutdown before starting buy operation (non-blocking check)
                        let shutdown_check = tokio::time::timeout(
                            Duration::from_millis(1),
                            shutdown_for_task.notified()
                        ).await;
                        if shutdown_check.is_ok() {
                            log(
                                LogTag::Trader,
                                "SHUTDOWN",
                                &format!("Skipping buy operation for {} - shutdown in progress", token_symbol)
                            );
                            return false;
                        }

                        // Wrap the buy operation in a timeout
                        match
                            tokio::time::timeout(Duration::from_secs(120), async {
                                open_position(&token, price, percent_change).await
                            }).await
                        {
                            Ok(_) => {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Completed buy operation for {} in concurrent task", token_symbol)
                                );
                                true
                            }
                            Err(_) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Buy operation for {} timed out after 20 seconds", token_symbol)
                                );
                                false
                            }
                        }
                    });

                    handles.push(handle);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Spawned {} concurrent buy tasks", handles.len()).dimmed().to_string()
                );

                // Collect results from all concurrent buy operations with overall timeout
                let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                    let mut completed = 0;
                    let mut successful = 0;
                    let total_handles = handles.len();

                    for handle in handles {
                        // Skip if shutdown signal received
                        if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                            log(
                                LogTag::Trader,
                                "INFO",
                                "new entries monitor shutting down during buy result collection..."
                            );
                            break;
                        }

                        // Add timeout for each handle to prevent getting stuck
                        match tokio::time::timeout(Duration::from_secs(120), handle).await {
                            Ok(task_result) => {
                                match task_result {
                                    Ok(success) => {
                                        if success {
                                            successful += 1;
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Trader,
                                            "ERROR",
                                            &format!("Buy task failed: {}", e)
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                log(LogTag::Trader, "WARN", "Buy task timed out after 5 seconds");
                            }
                        }

                        completed += 1;
                        if completed % 2 == 0 || completed == total_handles {
                            log(
                                LogTag::Trader,
                                "INFO",
                                &format!("Completed {}/{} buy operations", completed, total_handles)
                                    .dimmed()
                                    .to_string()
                            );
                        }
                    }

                    (completed, successful)
                }).await;

                match collection_result {
                    Ok((completed, successful)) => {
                        let new_open_count = get_open_positions_count();
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!(
                                "Concurrent buy operations completed: {}/{} successful, new open positions: {}",
                                successful,
                                completed,
                                new_open_count
                            )
                        );
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            "Buy operations collection timed out after 30 seconds"
                        );
                    }
                }
            }
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token checking cycle took longer than interval: {:?}", cycle_duration)
            );
            Duration::from_millis(100)
        } else {
            // Otherwise wait for the remaining interval time
            Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        if check_shutdown_or_delay(&shutdown, wait_time).await {
            log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
            break;
        }
    }
}

/// Background task to monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: Arc<Notify>) {
    // Clone shutdown once at the start to avoid borrow checker issues
    let shutdown = shutdown.clone();

    loop {
        // First, collect all open position mints to fetch pool prices in parallel
        let open_position_mints: Vec<String> = {
            if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions
                    .iter()
                    .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
                    .map(|p| p.mint.clone())
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Request immediate pool price checks for open positions (non-blocking)
        if !open_position_mints.is_empty() {
            for mint in &open_position_mints {
                let mint_clone = mint.clone();
                // Temporarily disabled tokio::spawn due to Send/Sync issues with database
                // TODO: Re-enable once database is made thread-safe
                // tokio::spawn(async move {
                //     // Use new pool price system for immediate price check
                //     let _price = crate::tokens::get_token_price(&mint_clone).await;
                // });
            }
        }

        let mut positions_to_close = Vec::new();

        // First, collect open positions data (without holding mutex across await)
        let open_positions_data: Vec<(usize, Position)> = {
            if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, position)| {
                        if position.position_type == "buy" && position.exit_price.is_none() {
                            Some((index, position.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Now process each position with async calls (mutex is released)
        for (index, mut position) in open_positions_data {
            // Get current price from safe price service
            if let Some(current_price) = get_token_price_blocking_safe(&position.mint).await {
                if current_price > 0.0 && current_price.is_finite() {
                    // Update position tracking (extremes) on the local copy
                    update_position_tracking(&mut position, current_price);

                    // Calculate P&L using unified function with pool price
                    let (pnl_sol, pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(current_price)
                    );

                    let now = Utc::now();

                    // Calculate sell urgency using the unified profit system
                    let (sell_urgency, sell_reason) = crate::profit::should_sell(
                        &position,
                        current_price
                    ).await;

                    // Use profit system decision only - no duplicate emergency checks
                    let should_exit = sell_urgency > 0.7;

                    if is_debug_trader_enabled() {
                        debug_trader_log(
                            "SELL_ANALYSIS",
                            &format!(
                                "{} | Urgency: {:.2} | Reason: {} | Exit: {}",
                                position.symbol,
                                sell_urgency,
                                sell_reason,
                                should_exit
                            )
                        );
                    }

                    if should_exit {
                        let minimal_token = Token {
                            mint: position.mint.clone(),
                            symbol: position.symbol.clone(),
                            name: position.symbol.clone(),
                            chain: "solana".to_string(),
                            // All other fields as None/defaults
                            logo_url: None,
                            coingecko_id: None,
                            website: None,
                            description: None,
                            tags: vec![],
                            is_verified: false,
                            created_at: None,
                            price_dexscreener_sol: None,
                            price_dexscreener_usd: None,
                            price_pool_sol: None,
                            price_pool_usd: None,
                            dex_id: None,
                            pair_address: None,
                            pair_url: None,
                            labels: vec![],
                            fdv: None,
                            market_cap: None,
                            txns: None,
                            volume: None,
                            price_change: None,
                            liquidity: None,
                            info: None,
                            boosts: None,
                        };

                        log(
                            LogTag::Trader,
                            "SELL",
                            &format!(
                                "Sell signal for {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Emergency: {}",
                                position.symbol,
                                position.mint,
                                sell_urgency,
                                pnl_percent,
                                sell_urgency >= 1.0 // Emergency is when urgency is 1.0 (from profit.rs)
                            )
                        );

                        positions_to_close.push((
                            index,
                            position.clone(), // Include the full position data
                            minimal_token,
                            current_price,
                            now,
                        ));
                    } else {
                        // log(
                        //     LogTag::Trader,
                        //     "HOLD",
                        //     &format!(
                        //         "Holding {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Price: {:.12}",
                        //         position.symbol,
                        //         position.mint,
                        //         sell_urgency,
                        //         pnl_percent,
                        //         current_price
                        //     )
                        // );
                    }

                    // Update the position in the global list with tracking data
                    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                        if let Some(saved_position) = positions.get_mut(index) {
                            saved_position.price_highest = position.price_highest;
                            saved_position.price_lowest = position.price_lowest;
                        }
                    }
                } else {
                    // Price found but invalid (0, negative, or NaN)
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!(
                            "Invalid price for position monitoring: {} ({}) - Price = {:.10}",
                            position.symbol,
                            position.mint,
                            current_price
                        )
                    );
                }
            } else {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!(
                        "No price found for open position: {} ({})",
                        position.symbol,
                        position.mint
                    )
                );
            }
        }

        // Close positions that need to be closed concurrently (outside of lock to avoid deadlock)
        if !positions_to_close.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!("Processing {} positions for concurrent closing", positions_to_close.len())
            );

            // Use a semaphore to limit concurrent sell transactions to avoid overwhelming the network
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent sells

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (index, position, token, exit_price, exit_time) in positions_to_close {
                // Check for shutdown before spawning tasks
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "open positions monitor shutting down during sell processing..."
                    );
                    break;
                }

                // Get permit from semaphore to limit concurrency with timeout
                let permit = match
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        semaphore.clone().acquire_owned()
                    ).await
                {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(e)) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to acquire semaphore permit for sell: {}", e)
                        );
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            "Semaphore acquire timed out for sell operation"
                        );
                        continue;
                    }
                };

                // Clone shutdown for use in the spawned sell task
                let shutdown_for_task = shutdown.clone();
                // We already have the position from the analysis phase, no need to look it up
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    let mut position = position;
                    let token_symbol = token.symbol.clone();

                    // FINAL SAFEGUARD: Only allow sales at profit (>0%) or significant loss (<=-55%)
                    let (_, final_pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(exit_price)
                    );

                    // Block sales only if it's a small loss (between 0% and -55%)
                    if
                        final_pnl_percent <= 0.0 &&
                        final_pnl_percent > crate::profit::STOP_LOSS_PERCENT
                    {
                        log(
                            LogTag::Trader,
                            "SAFEGUARD",
                            &format!(
                                "Blocking sale of {} - P&L {:.2}% is a small loss (between 0% and {:.2}%)",
                                token_symbol,
                                final_pnl_percent,
                                crate::profit::STOP_LOSS_PERCENT
                            )
                        );
                        return None; // Abort the sale - small loss, hold until bigger loss or profit
                    }

                    // Check for shutdown before starting sell operation (non-blocking check)
                    let shutdown_check = tokio::time::timeout(
                        Duration::from_millis(1),
                        shutdown_for_task.notified()
                    ).await;
                    if shutdown_check.is_ok() {
                        log(
                            LogTag::Trader,
                            "SHUTDOWN",
                            &format!("Skipping sell operation for {} - shutdown in progress", token_symbol)
                        );
                        return None;
                    }

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(Duration::from_secs(120), async {
                            close_position(&mut position, &token, exit_price, exit_time).await
                        }).await
                    {
                        Ok(success) => {
                            if success {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Successfully closed position for {} in concurrent task", token_symbol)
                                );
                                Some((index, position))
                            } else {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Failed to close position for {} in concurrent task", token_symbol)
                                );
                                None
                            }
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Sell operation for {} timed out after 15 seconds", token_symbol)
                            );
                            None
                        }
                    }
                });

                handles.push(handle);
            }

            log(
                LogTag::Trader,
                "INFO",
                &format!("Spawned {} concurrent sell tasks", handles.len()).dimmed().to_string()
            );

            // Collect results from all concurrent sell operations with overall timeout
            // Increased timeout to 60 seconds to accommodate multiple 15-second sell operations
            let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                let mut completed_positions = Vec::new();

                for handle in handles {
                    // Skip if shutdown signal received
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "open positions monitor shutting down during sell result collection..."
                        );
                        break;
                    }

                    // Add timeout for each handle to prevent getting stuck
                    // Increased timeout to 15 seconds to allow for transaction verification and ATA closing
                    match tokio::time::timeout(Duration::from_secs(120), handle).await {
                        Ok(task_result) => {
                            match task_result {
                                Ok(Some((index, updated_position))) => {
                                    completed_positions.push((index, updated_position));
                                }
                                Ok(None) => {
                                    // Position failed to close, continue
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Trader,
                                        "ERROR",
                                        &format!("Sell task failed: {}", e)
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            log(LogTag::Trader, "WARN", "Sell task timed out after 60 seconds");
                        }
                    }
                }

                completed_positions
            }).await;

            let completed_positions = match collection_result {
                Ok(positions) => positions,
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        "Sell operations collection timed out after 60 seconds"
                    );
                    Vec::new()
                }
            };

            // Update all successfully closed positions in the saved positions
            if !completed_positions.is_empty() {
                if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                    for (index, updated_position) in &completed_positions {
                        if let Some(saved_position) = positions.get_mut(*index) {
                            *saved_position = updated_position.clone();
                        }
                    }
                    save_positions_to_file(&positions);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Updated {} positions after concurrent sell operations",
                        completed_positions.len()
                    )
                );
            }
        }

        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "open positions monitor shutting down...");
            break;
        }
    }
}

/// Main trader function that spawns both monitoring tasks
pub async fn trader(shutdown: Arc<Notify>) {
    log(LogTag::Trader, "INFO", "Starting trader with background tasks...");

    let shutdown_clone = shutdown.clone();
    let entries_task = tokio::spawn(async move {
        monitor_new_entries(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let positions_task = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let display_task = tokio::spawn(async move {
        monitor_positions_display(shutdown_clone).await;
    });

    // Wait for shutdown signal
    shutdown.notified().await;

    log(LogTag::Trader, "INFO", "Trader shutting down...");

    // Give tasks a chance to shutdown gracefully
    let graceful_timeout = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = tokio::try_join!(entries_task, positions_task, display_task);
    });

    match graceful_timeout.await {
        Ok(_) => {
            log(LogTag::Trader, "INFO", "Trader tasks finished gracefully");
        }
        Err(_) => {
            log(LogTag::Trader, "WARN", "Trader tasks did not finish gracefully, aborting");
            // Force abort if graceful shutdown fails
            // entries_task.abort(); // These might already be finished
            // positions_task.abort();
            // display_task.abort();
        }
    }
}
