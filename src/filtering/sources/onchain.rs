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
        let trimmed = token.symbol.trim();
        if trimmed.chars().count() == 1 && !trimmed.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
            return Err(FilterRejectionReason::OnChainSuspiciousSymbol);
        }
    }

    // H4: Known scam authority detection (auto-discovered, no hardcoding)
    if config.reject_known_scam_authorities {
        if let Some(ref freeze_auth) = token.freeze_authority {
            if crate::tokens::authority_cache::is_blocked_authority(freeze_auth) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
        if let Some(ref update_auth) = token.update_authority {
            if crate::tokens::authority_cache::is_blocked_authority(update_auth) {
                return Err(FilterRejectionReason::OnChainKnownScamAuthority);
            }
        }
        if let Some(ref mint_auth) = token.mint_authority {
            if crate::tokens::authority_cache::is_blocked_authority(mint_auth) {
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

/// Check if symbol contains only ASCII digits (e.g. "00", "123", "0000")
fn is_numeric_only_symbol(symbol: &str) -> bool {
    let trimmed = symbol.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit())
}

/// Check if symbol is empty or whitespace/null-padded
fn is_empty_or_whitespace(symbol: &str) -> bool {
    symbol.is_empty() || symbol.trim().is_empty() || symbol.trim_matches('\0').trim().is_empty()
}

/// Compute a combined risk score from multiple weak signals.
/// Each signal contributes points; the total determines rejection.
/// Score range: 0–100 (capped).
fn compute_risk_score(token: &Token) -> u32 {
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

    // Immutable metadata with other signals (+10)
    if let Some(false) = token.is_mutable {
        if score > 0 {
            score += 10;
        }
    }

    // Name matches symbol exactly (lazy scam pattern) (+15)
    if !token.name.is_empty()
        && !token.symbol.is_empty()
        && token.name.trim().eq_ignore_ascii_case(token.symbol.trim())
    {
        score += 15;
    }

    score.min(100)
}
