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
    // Decimals the batch load already carries are authoritative — the same DB row the
    // resolver would consult. Only a token whose decimals we have never resolved pays for
    // the cache → DB → server → chain fallback below.
    //
    // PERF: this is the whole cost of a filtering pass. The snapshot evaluates every token
    // with market data (328k on a mature database) while the decimals cache is capped far
    // below that, so routing every token through the resolver meant a per-token DB
    // round-trip for the majority — ~217us each against ~4us for a cache hit, about 95
    // seconds of lookups per 30-second refresh, for data that was already in hand. Reading
    // it off the token instead: 0.7us, and no difference between a cold and a warm pass.
    if !has_decimals(token).await {
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

async fn has_decimals(token: &Token) -> bool {
    if crate::chains::adapter().is_native_asset(&token.mint)
        || token.decimals.is_some_and(tokens::decimals_are_valid)
    {
        return true;
    }

    // Unresolved: fall back to the full chain (cache → DB → data server → RPC), which also
    // persists what it finds, so a token only ever pays this once.
    // Single-flight dedup prevents duplicate chain fetches; failures cached for 24h.
    get_decimals(crate::chains::active_chain(), &token.mint)
        .await
        .is_some()
}
