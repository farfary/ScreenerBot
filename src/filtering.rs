/// Centralized token filtering system for ScreenerBot
/// All token filtering logic consolidated into a single function
/// No structs or models - pure functional approach

use crate::tokens::Token;
use crate::logger::{ log, LogTag };
use crate::loss_prevention::should_allow_token_purchase;
use crate::positions::SAVED_POSITIONS;
use crate::trader::MAX_OPEN_POSITIONS;
use chrono::{ Duration as ChronoDuration, Utc };

// =============================================================================
// FILTERING CONFIGURATION CONSTANTS
// =============================================================================

/// Minimum token age in hours before trading
pub const MIN_TOKEN_AGE_HOURS: i64 = 4;

/// Maximum token age in hours (effectively unlimited)
pub const MAX_TOKEN_AGE_HOURS: i64 = 30 * 24;

/// Cooldown period after closing position before re-entering same token (minutes)
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 24 * 60;

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

    // Loss prevention
    PoorHistoricalPerformance {
        loss_rate: f64,
        avg_loss: f64,
    },

    // Account/Token status issues
    AccountFrozen,
    TokenAccountFrozen,

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
    // 1. Basic metadata validation
    if let Some(reason) = validate_basic_token_info(token) {
        return FilterResult::Rejected(reason);
    }

    // 2. Age validation
    if let Some(reason) = validate_token_age(token) {
        return FilterResult::Rejected(reason);
    }

    // 3. Liquidity validation
    if let Some(reason) = validate_liquidity(token) {
        return FilterResult::Rejected(reason);
    }

    // 4. Price validation
    if let Some(reason) = validate_price_data(token) {
        return FilterResult::Rejected(reason);
    }

    // 5. Position-related validation
    if let Some(reason) = validate_position_constraints(token) {
        return FilterResult::Rejected(reason);
    }

    // 6. Loss prevention check
    if let Some(reason) = validate_loss_prevention(token) {
        return FilterResult::Rejected(reason);
    }

    FilterResult::Approved
}

// =============================================================================
// INDIVIDUAL FILTER FUNCTIONS
// =============================================================================

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
    let (total, eligible, pass_rate) = get_filtering_stats(tokens);

    if total > 0 {
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
            log(LogTag::Filtering, "WARN", "No tokens passed filtering - check filter criteria");
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
