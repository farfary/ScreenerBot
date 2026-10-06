// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana's filter profile and its on-chain stage — authority reputation, immutable
//! metadata with a freeze authority, and the combined risk score.

use std::sync::{Arc, OnceLock};

use crate::chains::ChainId;
use crate::config::schemas::OnChainFilters;
use crate::config::FilteringConfig;
use crate::filtering::sources::symbols::{
    is_empty_or_whitespace, is_numeric_only_symbol, meaningful_symbol,
};
use crate::filtering::sources::FilterRejectionReason;
use crate::filtering::{FilterProfile, FilterStage, StageEval};
use crate::tokens::types::Token;

/// Solana's filter profile, assembled once per process.
pub(crate) fn profile() -> Arc<FilterProfile> {
    static PROFILE: OnceLock<Arc<FilterProfile>> = OnceLock::new();
    PROFILE
        .get_or_init(|| {
            Arc::new(FilterProfile::assemble(
                ChainId::Solana,
                Arc::new(SolanaOnChainStage),
            ))
        })
        .clone()
}

/// On-chain scam detection from data already in the Token struct (metadata,
/// authorities) without any external API calls.
///
/// Pipeline position: AFTER meta and the symbol rules, BEFORE the market sources and
/// Rugcheck. This is a fast, zero-RPC-cost filter that catches obvious scams early,
/// preventing wasted API calls to external sources.
pub struct SolanaOnChainStage;

impl FilterStage for SolanaOnChainStage {
    fn name(&self) -> &'static str {
        "onchain"
    }

    fn supports(&self, chain: ChainId) -> bool {
        chain == ChainId::Solana
    }

    fn evaluate<'a>(
        &'a self,
        chain: ChainId,
        token: &'a Token,
        config: &'a FilteringConfig,
    ) -> StageEval<'a> {
        StageEval::Ready(evaluate(chain, token, &config.onchain).into())
    }
}

fn evaluate(
    chain: ChainId,
    token: &Token,
    config: &OnChainFilters,
) -> Result<(), FilterRejectionReason> {
    if !config.enabled {
        return Ok(());
    }

    // H4: Known scam authority detection (auto-discovered, no hardcoding)
    if config.reject_known_scam_authorities {
        if let Some(ref freeze_auth) = token.freeze_authority {
            if crate::tokens::authority_cache::is_blocked_authority(chain, freeze_auth) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
        if let Some(ref update_auth) = token.update_authority {
            if crate::tokens::authority_cache::is_blocked_authority(chain, update_auth) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
        if let Some(ref mint_auth) = token.mint_authority {
            if crate::tokens::authority_cache::is_blocked_authority(chain, mint_auth) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
    }

    // H5: Immutable metadata combined with freeze authority (strong scam signal)
    if config.reject_immutable_with_freeze {
        if let Some(false) = token.is_mutable {
            if token.freeze_authority.is_some() {
                return Err(FilterRejectionReason::OnChainImmutableWithFreeze);
            }
        }
    }

    // H6: Combined risk score — multiple weak signals add up
    if config.combined_risk_enabled {
        let score = compute_risk_score(token);
        if score >= config.max_combined_risk_score {
            return Err(FilterRejectionReason::OnChainHighRiskScore);
        }
    }

    Ok(())
}

/// Compute a combined risk score from multiple weak signals.
/// Each signal contributes points; the total determines rejection.
/// Score range: 0–100 (capped).
fn compute_risk_score(token: &Token) -> u32 {
    // Independent signals are summed first, so that the one CONDITIONAL signal below can
    // ask "did anything else fire?" and get the same answer no matter what order the
    // signals happen to be written in. Scoring immutability inline against a running total
    // made it depend on which signals had been added ABOVE it: the name-matches-symbol
    // signal, evaluated afterwards, could not trigger the bonus while the freeze-authority
    // signal could, purely because of source position.
    let mut score: u32 = 0;

    // Numeric symbol (+30)
    if is_numeric_only_symbol(&token.symbol) {
        score += 30;
    }

    // Empty symbol (+25)
    if is_empty_or_whitespace(&token.symbol) {
        score += 25;
    }

    // Freeze authority present (+10)
    if token.freeze_authority.is_some() {
        score += 10;
    }

    // Name matches symbol exactly (lazy scam pattern) (+15)
    let name = meaningful_symbol(&token.name);
    let symbol = meaningful_symbol(&token.symbol);
    if !name.is_empty() && !symbol.is_empty() && name.eq_ignore_ascii_case(symbol) {
        score += 15;
    }

    // Immutable metadata AMPLIFIES any other signal, but is not a scam signal on its own —
    // most legitimate tokens make their metadata immutable on purpose (+10)
    if token.is_mutable == Some(false) && score > 0 {
        score += 10;
    }

    score.min(100)
}
