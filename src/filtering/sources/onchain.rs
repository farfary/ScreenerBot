//! On-chain filter source — validates token accounts, authorities, and program ownership.

use crate::config::schemas::OnChainFilters;
use crate::filtering::sources::FilterRejectionReason;
use crate::tokens::types::Token;

/// On-chain core filtering — detects scam tokens using data already available
/// in the Token struct (metadata, authorities, supply) without any external API calls.
///
/// Pipeline position: AFTER meta, BEFORE dexscreener/geckoterminal/rugcheck.
/// This is a fast, zero-RPC-cost filter that catches obvious scams early,
/// preventing wasted API calls to external sources.
pub fn evaluate(token: &Token, config: &OnChainFilters) -> Result<(), FilterRejectionReason> {
    if !config.enabled {
        return Ok(());
    }

    // H1: Numeric-only symbol detection
    if config.reject_numeric_symbols {
        if is_numeric_only_symbol(&token.symbol) {
            return Err(FilterRejectionReason::OnChainNumericSymbol);
        }
    }

    // H2: Empty or whitespace-only symbol
    if config.reject_empty_symbols {
        if is_empty_or_whitespace(&token.symbol) {
            return Err(FilterRejectionReason::OnChainEmptySymbol);
        }
    }

    // H3: Suspicious single-char symbols (often spam)
    if config.reject_single_char_symbols {
        let trimmed = meaningful_symbol(&token.symbol);
        if trimmed.chars().count() == 1
            && !trimmed
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
        {
            return Err(FilterRejectionReason::OnChainSuspiciousSymbol);
        }
    }

    // H4: Known scam authority detection (auto-discovered, no hardcoding)
    if config.reject_known_scam_authorities {
        if let Some(ref freeze_auth) = token.freeze_authority {
            if crate::tokens::authority_cache::is_blocked_authority(
                crate::chains::active_chain(),
                freeze_auth,
            ) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
        if let Some(ref update_auth) = token.update_authority {
            if crate::tokens::authority_cache::is_blocked_authority(
                crate::chains::active_chain(),
                update_auth,
            ) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
        if let Some(ref mint_auth) = token.mint_authority {
            if crate::tokens::authority_cache::is_blocked_authority(
                crate::chains::active_chain(),
                mint_auth,
            ) {
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

/// A symbol with the padding a scam mint uses removed: ASCII whitespace plus the control
/// characters (NUL above all) that fixed-width on-chain metadata fields are packed with.
///
/// `str::trim` only strips Unicode whitespace, and NUL is not whitespace, so trimming alone
/// left `"\0"` looking like a real symbol. Trimming NUL from the ends first is not enough
/// either — `" \0 "` survived both passes, because the spaces protected the NUL from
/// `trim_matches` and the NUL kept the string non-empty for `trim`.
fn meaningful_symbol(symbol: &str) -> &str {
    symbol.trim_matches(|c: char| c.is_whitespace() || c.is_control())
}

/// Check if symbol contains only ASCII digits (e.g. "00", "123", "0000")
fn is_numeric_only_symbol(symbol: &str) -> bool {
    let trimmed = meaningful_symbol(symbol);
    !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit())
}

/// Check if symbol is empty or whitespace/null-padded
fn is_empty_or_whitespace(symbol: &str) -> bool {
    meaningful_symbol(symbol).is_empty()
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
