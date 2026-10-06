// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Symbol filter stage — rejects numeric-only, empty and suspicious single-character
//! symbols. Chain-neutral; its rejections belong to the on-chain source.

use crate::chains::ChainId;
use crate::config::schemas::OnChainFilters;
use crate::config::FilteringConfig;
use crate::filtering::sources::FilterRejectionReason;
use crate::filtering::stage::{FilterStage, StageEval};
use crate::tokens::types::Token;

/// The symbol rules of the on-chain source, run before the chain's own on-chain stage.
pub struct SymbolStage;

impl FilterStage for SymbolStage {
    fn name(&self) -> &'static str {
        "symbols"
    }

    fn supports(&self, _chain: ChainId) -> bool {
        true
    }

    fn evaluate<'a>(
        &'a self,
        _chain: ChainId,
        token: &'a Token,
        config: &'a FilteringConfig,
    ) -> StageEval<'a> {
        StageEval::Ready(evaluate(token, &config.onchain).into())
    }
}

/// Symbol rules — data already in the Token struct, no external calls.
fn evaluate(token: &Token, config: &OnChainFilters) -> Result<(), FilterRejectionReason> {
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

    Ok(())
}

/// A symbol with the padding a scam mint uses removed: ASCII whitespace plus the control
/// characters (NUL above all) that fixed-width on-chain metadata fields are packed with.
///
/// `str::trim` only strips Unicode whitespace, and NUL is not whitespace, so trimming alone
/// left `"\0"` looking like a real symbol. Trimming NUL from the ends first is not enough
/// either — `" \0 "` survived both passes, because the spaces protected the NUL from
/// `trim_matches` and the NUL kept the string non-empty for `trim`.
pub(crate) fn meaningful_symbol(symbol: &str) -> &str {
    symbol.trim_matches(|c: char| c.is_whitespace() || c.is_control())
}

/// Check if symbol contains only ASCII digits (e.g. "00", "123", "0000")
pub(crate) fn is_numeric_only_symbol(symbol: &str) -> bool {
    let trimmed = meaningful_symbol(symbol);
    !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit())
}

/// Check if symbol is empty or whitespace/null-padded
pub(crate) fn is_empty_or_whitespace(symbol: &str) -> bool {
    meaningful_symbol(symbol).is_empty()
}
