/// Centralized token filtering system for ScreenerBot
/// All token filtering logic consolidated into a single function
/// No structs or models - pure functional approach

use crate::tokens::Token;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_filtering_enabled;
use crate::loss_prevention::should_allow_token_purchase;
use crate::positions::SAVED_POSITIONS;
use crate::trader::MAX_OPEN_POSITIONS;
use crate::rugcheck_filtering::enhanced_filter_token_rugcheck;
use chrono::{ Duration as ChronoDuration, Utc, DateTime };
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::collections::HashMap;

// =============================================================================
// FILTERING CONFIGURATION CONSTANTS
// =============================================================================

/// Minimum token age in hours before trading
pub const MIN_TOKEN_AGE_HOURS: i64 = 1;

/// Maximum token age in hours (effectively unlimited)
pub const MAX_TOKEN_AGE_HOURS: i64 = 30 * 24;

/// Cooldown period after closing position before re-entering same token (minutes)
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 24 * 60;

/// Maximum allowed percentage from all-time high to allow buying (e.g. 75% means
/// current price must be at least 25% below ATH)
pub const MAX_PRICE_TO_ATH_PERCENT: f64 = 75.0;

/// Cooldown period for token filter logs (minutes)
pub const LOG_COOLDOWN_MINUTES: i64 = 15;

/// Static global: log cooldown tracking by token mint and reason
static TOKEN_LOG_COOLDOWNS: Lazy<Mutex<HashMap<String, DateTime<Utc>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// =============================================================================
// FILTERING RESULT ENUM
// =============================================================================

/// Reasons why a token might be filtered out
#[derive(Debug, Clone)]
pub enum FilterReason {
    // Basic validation failures
    EmptySymbol,
    EmptyMint,
    InvalidPrice,
    ZeroLiquidity,
    MissingLiquidityData,
    MissingPriceData,

    // Age-related failures
    TooYoung {
        age_hours: i64,
        min_required: i64,
    },
    TooOld {
        age_hours: i64,
        max_allowed: i64,
    },
    NoCreationDate,

    // Position-related failures
    ExistingOpenPosition,
    RecentlyClosed {
        minutes_ago: i64,
        cooldown_minutes: i64,
    },
    MaxPositionsReached {
        current: usize,
        max: usize,
    },

    // Price action related failures
    TooCloseToATH {
        current_percent_of_ath: f64,
        max_allowed_percent: f64,
    },

    // Loss prevention
    PoorHistoricalPerformance {
        loss_rate: f64,
        avg_loss: f64,
    },

    // Account/Token status issues
    AccountFrozen,
    TokenAccountFrozen,

    // Rugcheck security risks
    RugcheckRisk {
        risk_level: String,
        reasons: Vec<String>,
    },

    // Trading requirements
    LockAcquisitionFailed,
}

/// Result of token filtering
#[derive(Debug, Clone)]
pub enum FilterResult {
    Approved,
    Rejected(FilterReason),
}

// =============================================================================
// MAIN FILTERING FUNCTION
// =============================================================================

/// Centralized token filtering function
/// Returns FilterResult::Approved if token passes all filters
/// Returns FilterResult::Rejected(reason) if token fails any filter
pub fn filter_token_for_trading(token: &Token) -> FilterResult {
    // 1. RUGCHECK SECURITY VALIDATION (FIRST - HIGHEST PRIORITY)
    if let Some(reason) = validate_rugcheck_risks(token) {
        if should_log_token_filter(token, "SecurityRisk") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Security risk - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 2. Basic metadata validation
    if let Some(reason) = validate_basic_token_info(token) {
        if should_log_token_filter(token, "Metadata") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Invalid metadata - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 3. Age validation
    if let Some(reason) = validate_token_age(token) {
        if should_log_token_filter(token, "Age") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Age constraint - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 4. Liquidity validation
    if let Some(reason) = validate_liquidity(token) {
        if should_log_token_filter(token, "Liquidity") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Liquidity issue - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 5. Price-to-ATH validation (NEW: Avoid buying near all-time highs)
    if let Some(reason) = validate_price_to_ath(token) {
        if should_log_token_filter(token, "ATH") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Too close to ATH - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 5. Price validation
    if let Some(reason) = validate_price_data(token) {
        if should_log_token_filter(token, "Price") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Price data issue - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 6. Position-related validation
    if let Some(reason) = validate_position_constraints(token) {
        if should_log_token_filter(token, "Position") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Position constraint - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // 7. Loss prevention check
    if let Some(reason) = validate_loss_prevention(token) {
        if should_log_token_filter(token, "LossPrevention") {
            log(
                LogTag::Filtering,
                "REJECT",
                &format!("{}: Loss prevention - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }

    // Token passed all filters
    if should_log_token_filter(token, "Approved") {
        log(LogTag::Filtering, "APPROVE", &format!("{}: Passed all filters", token.symbol));
    }

    FilterResult::Approved
}

// =============================================================================
// INDIVIDUAL FILTER FUNCTIONS
// =============================================================================

/// Validate if token price is too close to all-time high
fn validate_price_to_ath(token: &Token) -> Option<FilterReason> {
    // If we don't have price history or all-time high data, we can't validate
    let price_history = match crate::trader::PRICE_HISTORY_24H.try_lock() {
        Ok(history) => history,
        Err(_) => {
            // If we can't lock the price history, we'll be conservative and skip the token
            return Some(FilterReason::LockAcquisitionFailed);
        }
    };

    if let Some(token_history) = price_history.get(&token.mint) {
        if token_history.len() < 5 {
            // Not enough history to determine ATH
            return None;
        }

        // Find the all-time high price in the available history
        let mut all_time_high = 0.0;
        for (_, price) in token_history {
            if *price > all_time_high {
                all_time_high = *price;
            }
        }

        // Get current price
        let current_price = token.price_dexscreener_sol.unwrap_or(0.0);
        if current_price <= 0.0 || all_time_high <= 0.0 {
            return None; // Invalid price data
        }

        // Calculate what percentage of ATH is the current price
        let current_percent_of_ath = (current_price / all_time_high) * 100.0;

        // If current price is too close to ATH (e.g. above 75% of ATH), reject
        if current_percent_of_ath > MAX_PRICE_TO_ATH_PERCENT {
            if should_log_token_filter(token, "ATH_Detail") {
                log(
                    LogTag::Filtering,
                    "ATH_CHECK",
                    &format!(
                        "Token {} rejected: price is {:.1}% of ATH ({:.6} / {:.6})",
                        token.symbol,
                        current_percent_of_ath,
                        current_price,
                        all_time_high
                    )
                );
            }

            return Some(FilterReason::TooCloseToATH {
                current_percent_of_ath,
                max_allowed_percent: MAX_PRICE_TO_ATH_PERCENT,
            });
        }
    }

    None
}

/// Validate rugcheck security risks (HIGHEST PRIORITY - RUNS FIRST)
fn validate_rugcheck_risks(token: &Token) -> Option<FilterReason> {
    // Use enhanced filtering with cached rugcheck data
    if let Some(risk_message) = enhanced_filter_token_rugcheck(token) {
        // Parse risk level from message
        let risk_level = if risk_message.starts_with("RUGCHECK-CRITICAL") {
            "CRITICAL".to_string()
        } else if risk_message.starts_with("RUGCHECK-DANGEROUS") {
            "DANGEROUS".to_string()
        } else if risk_message.contains("FREEZE-AUTHORITY") {
            "HIGH".to_string()
        } else if risk_message.contains("SCAM-INDICATOR") {
            "CRITICAL".to_string()
        } else {
            "MEDIUM".to_string()
        };

        return Some(FilterReason::RugcheckRisk {
            risk_level,
            reasons: vec![risk_message],
        });
    }

    None
}

/// Validate basic token metadata
fn validate_basic_token_info(token: &Token) -> Option<FilterReason> {
    if token.symbol.is_empty() {
        return Some(FilterReason::EmptySymbol);
    }

    if token.mint.is_empty() {
        return Some(FilterReason::EmptyMint);
    }

    None
}

/// Validate token age constraints
fn validate_token_age(token: &Token) -> Option<FilterReason> {
    let Some(created_at) = token.created_at else {
        return Some(FilterReason::NoCreationDate);
    };

    let now = Utc::now();
    let token_age = now - created_at;
    let age_hours = token_age.num_hours();

    if age_hours < MIN_TOKEN_AGE_HOURS {
        return Some(FilterReason::TooYoung {
            age_hours,
            min_required: MIN_TOKEN_AGE_HOURS,
        });
    }

    if age_hours > MAX_TOKEN_AGE_HOURS {
        return Some(FilterReason::TooOld {
            age_hours,
            max_allowed: MAX_TOKEN_AGE_HOURS,
        });
    }

    None
}

/// Validate liquidity requirements
fn validate_liquidity(token: &Token) -> Option<FilterReason> {
    let Some(liquidity) = &token.liquidity else {
        return Some(FilterReason::MissingLiquidityData);
    };

    let liquidity_usd = liquidity.usd.unwrap_or(0.0);

    if liquidity_usd <= 0.0 {
        return Some(FilterReason::ZeroLiquidity);
    }

    None
}

/// Validate price data availability
fn validate_price_data(token: &Token) -> Option<FilterReason> {
    let current_price = token.price_dexscreener_sol.unwrap_or(0.0);

    if current_price <= 0.0 {
        return Some(FilterReason::InvalidPrice);
    }

    if token.price_dexscreener_sol.is_none() {
        return Some(FilterReason::MissingPriceData);
    }

    None
}

/// Validate position-related constraints
fn validate_position_constraints(token: &Token) -> Option<FilterReason> {
    let Ok(positions) = SAVED_POSITIONS.lock() else {
        log(
            LogTag::Filtering,
            "ERROR",
            &format!("Could not acquire lock on positions for {}", token.symbol)
        );
        return Some(FilterReason::LockAcquisitionFailed);
    };

    // Check for existing open position
    let has_open_position = positions
        .iter()
        .any(|p| p.mint == token.mint && p.position_type == "buy" && p.exit_price.is_none());

    if has_open_position {
        return Some(FilterReason::ExistingOpenPosition);
    }

    // Check maximum open positions limit
    let open_positions_count = positions
        .iter()
        .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
        .count();

    if open_positions_count >= MAX_OPEN_POSITIONS {
        return Some(FilterReason::MaxPositionsReached {
            current: open_positions_count,
            max: MAX_OPEN_POSITIONS,
        });
    }

    // Check for recently closed positions (cooldown period)
    let cooldown_duration = ChronoDuration::minutes(POSITION_CLOSE_COOLDOWN_MINUTES);
    let now = Utc::now();

    for position in positions.iter() {
        if position.mint == token.mint && position.exit_time.is_some() {
            if let Some(exit_time) = position.exit_time {
                let time_since_close = now - exit_time;
                if time_since_close <= cooldown_duration {
                    let minutes_ago = time_since_close.num_minutes();
                    return Some(FilterReason::RecentlyClosed {
                        minutes_ago,
                        cooldown_minutes: POSITION_CLOSE_COOLDOWN_MINUTES,
                    });
                }
            }
        }
    }

    None
}

/// Validate loss prevention constraints
fn validate_loss_prevention(token: &Token) -> Option<FilterReason> {
    if !should_allow_token_purchase(&token.mint, &token.symbol) {
        return Some(FilterReason::PoorHistoricalPerformance {
            loss_rate: 0.0, // Placeholder - would need actual values
            avg_loss: 0.0, // Placeholder - would need actual values
        });
    }

    None
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if we should log a filtering event for a token based on cooldown
/// Returns true if the token hasn't been logged recently (within cooldown period)
fn should_log_token_filter(token: &Token, reason_type: &str) -> bool {
    if !is_debug_filtering_enabled() {
        return false;
    }

    // Create a unique key combining token mint and reason type
    let key = format!("{}:{}", token.mint, reason_type);

    let now = Utc::now();
    let cooldown_duration = ChronoDuration::minutes(LOG_COOLDOWN_MINUTES);

    // Try to acquire lock on the cooldowns map
    if let Ok(mut cooldowns) = TOKEN_LOG_COOLDOWNS.lock() {
        // Check if this token+reason has a recent log entry
        if let Some(last_logged) = cooldowns.get(&key) {
            let elapsed = now - *last_logged;

            // If within cooldown period, don't log
            if elapsed < cooldown_duration {
                return false;
            }
        }

        // Update the last logged time
        cooldowns.insert(key, now);

        // Cleanup old entries periodically (every 100 inserts, approximately)
        if cooldowns.len() % 100 == 0 {
            cooldowns.retain(|_, timestamp| { now - *timestamp < ChronoDuration::hours(1) });
        }

        true
    } else {
        // If we can't get the lock, default to allowing the log
        // This should be rare and is better than silent failures
        true
    }
}

/// Check if a specific token passes all filters (convenience function)
pub fn is_token_eligible_for_trading(token: &Token) -> bool {
    matches!(filter_token_for_trading(token), FilterResult::Approved)
}

/// Filter a list of tokens and return only eligible ones
pub fn filter_eligible_tokens(tokens: &[Token]) -> Vec<Token> {
    tokens
        .iter()
        .filter(|token| is_token_eligible_for_trading(token))
        .cloned()
        .collect()
}

/// Filter a list of tokens and return both eligible and rejected with reasons
pub fn filter_tokens_with_reasons(tokens: &[Token]) -> (Vec<Token>, Vec<(Token, FilterReason)>) {
    let mut eligible = Vec::new();
    let mut rejected = Vec::new();

    for token in tokens {
        match filter_token_for_trading(token) {
            FilterResult::Approved => eligible.push(token.clone()),
            FilterResult::Rejected(reason) => rejected.push((token.clone(), reason)),
        }
    }

    // Log detailed breakdown only in debug mode
    if is_debug_filtering_enabled() && !tokens.is_empty() {
        log_filtering_breakdown(&rejected);
    }

    (eligible, rejected)
}

/// Count how many tokens would pass filtering without processing them all
pub fn count_eligible_tokens(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .filter(|token| is_token_eligible_for_trading(token))
        .count()
}

/// Get filtering statistics for a list of tokens
pub fn get_filtering_stats(tokens: &[Token]) -> (usize, usize, f64) {
    let total = tokens.len();
    let eligible = count_eligible_tokens(tokens);
    let pass_rate = if total > 0 { ((eligible as f64) / (total as f64)) * 100.0 } else { 0.0 };

    (total, eligible, pass_rate)
}

/// Log filtering summary statistics
pub fn log_filtering_summary(tokens: &[Token]) {
    // Only log summary if we have tokens and debug is enabled
    if tokens.is_empty() || !is_debug_filtering_enabled() {
        return;
    }

    // Create a simple key for summary logs that changes every LOG_COOLDOWN_MINUTES
    // This ensures we don't spam summaries too often
    let now = Utc::now();

    if let Ok(mut cooldowns) = TOKEN_LOG_COOLDOWNS.lock() {
        let last_summary = cooldowns.get("summary_log").cloned();

        // Only log summary if we haven't done so recently
        if
            last_summary.is_none() ||
            now - last_summary.unwrap() > ChronoDuration::minutes(LOG_COOLDOWN_MINUTES)
        {
            let (total, eligible, pass_rate) = get_filtering_stats(tokens);

            log(
                LogTag::Filtering,
                "SUMMARY",
                &format!(
                    "Processed {} tokens: {} eligible ({:.1}% pass rate)",
                    total,
                    eligible,
                    pass_rate
                )
            );

            if eligible == 0 && total > 0 {
                log(
                    LogTag::Filtering,
                    "WARN",
                    "No tokens passed filtering - check filter criteria"
                );
            }

            // Update last summary timestamp
            cooldowns.insert("summary_log".to_string(), now);
        }
    }
}

/// Log detailed breakdown of rejection reasons (debug mode only)
fn log_filtering_breakdown(rejected: &[(Token, FilterReason)]) {
    if rejected.is_empty() || !is_debug_filtering_enabled() {
        return;
    }

    // Create a simple key for breakdown logs that changes every LOG_COOLDOWN_MINUTES
    // This prevents spam in the logs
    let now = Utc::now();
    let time_bucket = now.timestamp() / (LOG_COOLDOWN_MINUTES * 60);

    if let Ok(mut cooldowns) = TOKEN_LOG_COOLDOWNS.lock() {
        let breakdown_key = format!("breakdown_{}", time_bucket);
        let last_breakdown = cooldowns.get(&breakdown_key).cloned();

        // Only log breakdown if we haven't done so recently for this time bucket
        if last_breakdown.is_none() {
            use std::collections::HashMap;
            let mut reason_counts: HashMap<String, usize> = HashMap::new();

            for (_, reason) in rejected {
                let reason_type = match reason {
                    FilterReason::EmptySymbol | FilterReason::EmptyMint => "Invalid Metadata",
                    FilterReason::InvalidPrice | FilterReason::MissingPriceData => "Price Issues",
                    FilterReason::ZeroLiquidity | FilterReason::MissingLiquidityData =>
                        "Liquidity Issues",
                    | FilterReason::TooYoung { .. }
                    | FilterReason::TooOld { .. }
                    | FilterReason::NoCreationDate => "Age Constraints",
                    | FilterReason::ExistingOpenPosition
                    | FilterReason::RecentlyClosed { .. }
                    | FilterReason::MaxPositionsReached { .. } => "Position Constraints",
                    FilterReason::TooCloseToATH { .. } => "Price Peak Protection",
                    FilterReason::PoorHistoricalPerformance { .. } => "Loss Prevention",
                    FilterReason::AccountFrozen | FilterReason::TokenAccountFrozen =>
                        "Account Issues",
                    FilterReason::RugcheckRisk { .. } => "Security Risks",
                    FilterReason::LockAcquisitionFailed => "System Errors",
                };

                *reason_counts.entry(reason_type.to_string()).or_insert(0) += 1;
            }

            log(LogTag::Filtering, "DEBUG", &format!("Rejection breakdown: {:?}", reason_counts));

            // Update last breakdown timestamp
            cooldowns.insert(breakdown_key, now);
        }
    }
}

/// Log specific filtering error for important cases
pub fn log_filtering_error(token: &Token, reason: &FilterReason) {
    let should_log = match reason {
        FilterReason::LockAcquisitionFailed => true,
        FilterReason::MaxPositionsReached { .. } => true,
        _ => false,
    };

    if should_log {
        let message = match reason {
            FilterReason::LockAcquisitionFailed =>
                format!("Lock acquisition failed for {}", token.symbol),
            FilterReason::MaxPositionsReached { current, max } =>
                format!("Max positions reached ({}/{})", current, max),
            _ => {
                return;
            }
        };

        log(LogTag::Filtering, "ERROR", &message);
    }
}

/// Main public interface function that combines filtering with logging
/// This should be used by the trader instead of filter_token_for_trading directly
pub fn should_buy_token(token: &Token) -> bool {
    match filter_token_for_trading(token) {
        FilterResult::Approved => true,
        FilterResult::Rejected(reason) => {
            log_filtering_error(token, &reason);
            false
        }
    }
}
