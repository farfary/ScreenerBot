// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Metadata filter source — validates token name, symbol, age, and description fields.

use chrono::Utc;

use crate::chains::ChainId;
use crate::config::FilteringConfig;
use crate::filtering::sources::FilterRejectionReason;
use crate::filtering::stage::{FilterStage, StageEval, StageOutcome};
use crate::positions;
use crate::tokens::types::Token;
use crate::tokens::{self, get_decimals};

/// Meta-level filters that apply regardless of external data sources.
///
/// Only the two lookups that await return a future: the decimals resolver, for a token
/// whose decimals were never resolved, and the position cooldown. Every other check
/// answers in place, so a token pays for no allocation it does not need.
pub struct MetaStage;

impl FilterStage for MetaStage {
    fn name(&self) -> &'static str {
        "meta"
    }

    fn supports(&self, _chain: ChainId) -> bool {
        true
    }

    fn evaluate<'a>(
        &'a self,
        chain: ChainId,
        token: &'a Token,
        config: &'a FilteringConfig,
    ) -> StageEval<'a> {
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
        if !has_decimals_in_hand(chain, token) {
            return StageEval::Pending(Box::pin(async move {
                if !resolve_decimals(chain, token).await {
                    return StageOutcome::Reject(FilterRejectionReason::NoDecimalsInDatabase);
                }
                match evaluate_with_decimals(token, config) {
                    StageEval::Ready(outcome) => outcome,
                    StageEval::Pending(future) => future.await,
                }
            }));
        }
        evaluate_with_decimals(token, config)
    }
}

/// The age and cooldown checks, for a token whose decimals are known.
fn evaluate_with_decimals<'a>(token: &'a Token, config: &'a FilteringConfig) -> StageEval<'a> {
    if config.age_enabled && is_too_new(token, config) {
        return StageEval::Ready(StageOutcome::Reject(FilterRejectionReason::TokenTooNew));
    }

    if config.cooldown_enabled && config.check_cooldown {
        return StageEval::Pending(Box::pin(async move {
            if positions::is_token_in_cooldown(&token.mint).await {
                StageOutcome::Reject(FilterRejectionReason::CooldownFiltered)
            } else {
                StageOutcome::Pass
            }
        }));
    }

    StageEval::Ready(StageOutcome::Pass)
}

fn is_too_new(token: &Token, config: &FilteringConfig) -> bool {
    let age_minutes = Utc::now()
        .signed_duration_since(token.first_discovered_at)
        .num_minutes()
        .max(0);

    age_minutes < config.min_token_age_minutes
}

fn has_decimals_in_hand(chain: ChainId, token: &Token) -> bool {
    crate::chains::adapter_for(chain).is_native_asset(&token.mint)
        || token.decimals.is_some_and(tokens::decimals_are_valid)
}

async fn resolve_decimals(chain: ChainId, token: &Token) -> bool {
    // Unresolved: fall back to the full chain (cache → DB → data server → RPC), which also
    // persists what it finds, so a token only ever pays this once.
    // Single-flight dedup prevents duplicate chain fetches; failures cached for 24h.
    get_decimals(chain, &token.mint).await.is_some()
}
