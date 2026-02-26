//! Metadata filter source — validates token name, symbol, age, and description fields.

use chrono::Utc;

use crate::config::FilteringConfig;
use crate::filtering::sources::FilterRejectionReason;
use crate::positions;
use crate::tokens::types::Token;
use crate::tokens::{self, get_decimals};

/// Evaluate meta-level filters that apply regardless of external data sources.
pub async fn evaluate(
    token: &Token,
    config: &FilteringConfig,
) -> Result<(), FilterRejectionReason> {
    // Resolve decimals via cache → DB → chain fallback.
    // Single-flight dedup prevents duplicate chain fetches; failures cached for 24h.
    if !has_decimals(&token.mint).await {
        return Err(FilterRejectionReason::NoDecimalsInDatabase);
    }

    if config.age_enabled && is_too_new(token, config) {
        return Err(FilterRejectionReason::TokenTooNew);
    }

    if config.cooldown_enabled
        && config.check_cooldown
        && positions::is_token_in_cooldown(&token.mint).await
    {
        return Err(FilterRejectionReason::CooldownFiltered);
    }

    Ok(())
}

fn is_too_new(token: &Token, config: &FilteringConfig) -> bool {
    let age_minutes = Utc::now()
        .signed_duration_since(token.first_discovered_at)
        .num_minutes()
        .max(0);

    age_minutes < config.min_token_age_minutes
}

async fn has_decimals(mint: &str) -> bool {
    if mint == tokens::SOL_MINT {
        return true;
    }

    get_decimals(mint).await.is_some()
}
