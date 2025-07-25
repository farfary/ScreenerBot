/// Centralized token filtering system for ScreenerBot
/// All token filtering logic consolidated into a single function
/// No structs or models - pure functional approach

use crate::global::*;
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
pub const MIN_TOKEN_AGE_HOURS: i64 = 12;

/// Maximum token age in hours (effectively unlimited)
pub const MAX_TOKEN_AGE_HOURS: i64 = 30 * 24;

/// Cooldown period after closing position before re-entering same token (minutes)
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 30;

/// Enable debug filtering logs
pub const ENABLE_DEBUG_FILTERING_LOGS: bool = true;

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

    debug_filtering_log(
        "APPROVED",
        &format!("Token {} ({}) passed all filters", token.symbol, token.mint)
    );

    FilterResult::Approved
}

// =============================================================================
// INDIVIDUAL FILTER FUNCTIONS
// =============================================================================

/// Validate basic token metadata
fn validate_basic_token_info(token: &Token) -> Option<FilterReason> {
    if token.symbol.is_empty() {
        debug_filtering_log("INFO_BLOCK", &format!("Token {} has empty symbol", token.mint));
        return Some(FilterReason::EmptySymbol);
    }

    if token.mint.is_empty() {
        debug_filtering_log("INFO_BLOCK", &format!("Token {} has empty mint", token.symbol));
        return Some(FilterReason::EmptyMint);
    }

    debug_filtering_log(
        "INFO_OK",
        &format!("Token {} ({}) basic info validation passed", token.symbol, token.mint)
    );

    None
}

/// Validate token age constraints
fn validate_token_age(token: &Token) -> Option<FilterReason> {
    let Some(created_at) = token.created_at else {
        debug_filtering_log(
            "AGE_BLOCK",
            &format!("Token {} ({}) has no creation date", token.symbol, token.mint)
        );
        return Some(FilterReason::NoCreationDate);
    };

    let now = Utc::now();
    let token_age = now - created_at;
    let age_hours = token_age.num_hours();

    if age_hours < MIN_TOKEN_AGE_HOURS {
        debug_filtering_log(
            "AGE_BLOCK",
            &format!(
                "Token {} ({}) too young: {} hours old (minimum {} hours required)",
                token.symbol,
                token.mint,
                age_hours,
                MIN_TOKEN_AGE_HOURS
            )
        );
        return Some(FilterReason::TooYoung {
            age_hours,
            min_required: MIN_TOKEN_AGE_HOURS,
        });
    }

    if age_hours > MAX_TOKEN_AGE_HOURS {
        debug_filtering_log(
            "AGE_BLOCK",
            &format!(
                "Token {} ({}) too old: {} hours old (maximum {} hours allowed)",
                token.symbol,
                token.mint,
                age_hours,
                MAX_TOKEN_AGE_HOURS
            )
        );
        return Some(FilterReason::TooOld {
            age_hours,
            max_allowed: MAX_TOKEN_AGE_HOURS,
        });
    }

    debug_filtering_log(
        "AGE_OK",
        &format!("Token {} ({}) age acceptable: {} hours old", token.symbol, token.mint, age_hours)
    );

    None
}

/// Validate liquidity requirements
fn validate_liquidity(token: &Token) -> Option<FilterReason> {
    let Some(liquidity) = &token.liquidity else {
        debug_filtering_log(
            "LIQUIDITY_BLOCK",
            &format!("Token {} ({}) has no liquidity data", token.symbol, token.mint)
        );
        return Some(FilterReason::MissingLiquidityData);
    };

    let liquidity_usd = liquidity.usd.unwrap_or(0.0);

    if liquidity_usd <= 0.0 {
        debug_filtering_log(
            "LIQUIDITY_BLOCK",
            &format!(
                "Token {} ({}) has zero or negative liquidity: ${:.2}",
                token.symbol,
                token.mint,
                liquidity_usd
            )
        );
        return Some(FilterReason::ZeroLiquidity);
    }

    debug_filtering_log(
        "LIQUIDITY_OK",
        &format!(
            "Token {} ({}) liquidity acceptable: ${:.2}",
            token.symbol,
            token.mint,
            liquidity_usd
        )
    );

    None
}

/// Validate price data availability
fn validate_price_data(token: &Token) -> Option<FilterReason> {
    let current_price = token.price_dexscreener_sol.unwrap_or(0.0);

    if current_price <= 0.0 {
        debug_filtering_log(
            "PRICE_BLOCK",
            &format!(
                "Token {} ({}) has invalid price: {:.12}",
                token.symbol,
                token.mint,
                current_price
            )
        );
        return Some(FilterReason::InvalidPrice);
    }

    if token.price_dexscreener_sol.is_none() {
        debug_filtering_log(
            "PRICE_BLOCK",
            &format!("Token {} ({}) has no DexScreener SOL price data", token.symbol, token.mint)
        );
        return Some(FilterReason::MissingPriceData);
    }

    debug_filtering_log(
        "PRICE_OK",
        &format!(
            "Token {} ({}) price validation passed: {:.12} SOL",
            token.symbol,
            token.mint,
            current_price
        )
    );

    None
}

/// Validate position-related constraints
fn validate_position_constraints(token: &Token) -> Option<FilterReason> {
    let Ok(positions) = SAVED_POSITIONS.lock() else {
        debug_filtering_log(
            "LOCK_ERROR",
            &format!("Could not acquire lock on SAVED_POSITIONS for {}", token.symbol)
        );
        return Some(FilterReason::LockAcquisitionFailed);
    };

    // Check for existing open position
    let has_open_position = positions
        .iter()
        .any(|p| p.mint == token.mint && p.position_type == "buy" && p.exit_price.is_none());

    if has_open_position {
        debug_filtering_log(
            "POSITION_BLOCK",
            &format!("Token {} ({}) already has an open position", token.symbol, token.mint)
        );
        return Some(FilterReason::ExistingOpenPosition);
    }

    // Check maximum open positions limit
    let open_positions_count = positions
        .iter()
        .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
        .count();

    if open_positions_count >= MAX_OPEN_POSITIONS {
        debug_filtering_log(
            "LIMIT_BLOCK",
            &format!(
                "Maximum open positions reached ({}/{}) - blocking {} ({})",
                open_positions_count,
                MAX_OPEN_POSITIONS,
                token.symbol,
                token.mint
            )
        );
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
                    debug_filtering_log(
                        "COOLDOWN_BLOCK",
                        &format!(
                            "Blocking {} ({}) purchase - position closed {} minutes ago (cooldown: {} min)",
                            token.symbol,
                            if token.mint.len() >= 8 {
                                &token.mint[..8]
                            } else {
                                &token.mint
                            },
                            minutes_ago,
                            POSITION_CLOSE_COOLDOWN_MINUTES
                        )
                    );
                    return Some(FilterReason::RecentlyClosed {
                        minutes_ago,
                        cooldown_minutes: POSITION_CLOSE_COOLDOWN_MINUTES,
                    });
                }
            }
        }
    }

    debug_filtering_log(
        "POSITION_OK",
        &format!(
            "Token {} ({}) position constraints passed - no conflicts",
            token.symbol,
            token.mint
        )
    );

    None
}

/// Validate loss prevention constraints
fn validate_loss_prevention(token: &Token) -> Option<FilterReason> {
    if !should_allow_token_purchase(&token.mint, &token.symbol) {
        debug_filtering_log(
            "LOSS_PREVENTION_BLOCK",
            &format!("Token {} ({}) blocked by loss prevention system", token.symbol, token.mint)
        );
        // Note: The actual loss rate and avg loss values would need to be extracted
        // from the loss prevention module for more detailed reporting
        return Some(FilterReason::PoorHistoricalPerformance {
            loss_rate: 0.0, // Placeholder - would need actual values
            avg_loss: 0.0, // Placeholder - would need actual values
        });
    }

    debug_filtering_log(
        "LOSS_PREVENTION_OK",
        &format!("Token {} ({}) passed loss prevention check", token.symbol, token.mint)
    );

    None
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Helper function for conditional debug filtering logs
pub fn debug_filtering_log(log_type: &str, message: &str) {
    if ENABLE_DEBUG_FILTERING_LOGS || is_debug_filtering_enabled() {
        log(LogTag::Trader, log_type, message);
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
