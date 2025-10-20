use chrono::Utc;

use crate::config::FilteringConfig;
use crate::filtering::sources::FilterRejectionReason;
use crate::positions;
use crate::tokens::types::Token;
use crate::tokens::{self, get_cached_decimals};

/// Evaluate meta-level filters that apply regardless of external data sources.
pub async fn evaluate(
    token: &Token,
    config: &FilteringConfig,
) -> Result<(), FilterRejectionReason> {
    if config.require_decimals_in_db && !has_cached_decimals(&token.mint) {
        return Err(FilterRejectionReason::NoDecimalsInDatabase);
    }

    if is_too_new(token, config) {
        return Err(FilterRejectionReason::TokenTooNew);
    }

    if config.check_cooldown && positions::is_token_in_cooldown(&token.mint).await {
        return Err(FilterRejectionReason::CooldownFiltered);
    }

    Ok(())
}

fn is_too_new(token: &Token, config: &FilteringConfig) -> bool {
    let age_minutes = Utc::now()
        .signed_duration_since(token.first_seen_at)
        .num_minutes()
        .max(0);

    age_minutes < config.min_token_age_minutes
}

fn has_cached_decimals(mint: &str) -> bool {
    if mint == tokens::SOL_MINT || mint == tokens::WSOL_MINT {
        return true;
    }

    get_cached_decimals(mint).is_some()
}
