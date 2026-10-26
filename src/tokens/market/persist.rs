// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Stores one market source's answer for a token: the token row first, which every market table references, then the market row.

use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;

/// The token metadata a market answer carries; `None` keeps the stored value.
pub(super) struct AnsweredMetadata<'a> {
    pub symbol: Option<&'a str>,
    pub name: Option<&'a str>,
    pub decimals: Option<u8>,
}

/// Write the token row of `mint` with `metadata`, then the market row through `write`.
///
/// The market tables reference the token row, so a token a source answers for before
/// anything else stored it (opened from Featured or search right after boot) is created
/// first instead of failing the market write. A failed market write is retried once,
/// token row included, and its error, with the underlying SQLite cause, is returned; the
/// caller must not cache the answer then, so the next fetch writes it again.
pub(super) fn store_market_answer(
    db: &TokenDatabase,
    source: &str,
    mint: &str,
    metadata: AnsweredMetadata<'_>,
    write: impl Fn() -> TokenResult<()>,
) -> TokenResult<()> {
    let mut retried = false;
    loop {
        if let Err(e) = db.upsert_token(mint, metadata.symbol, metadata.name, metadata.decimals) {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKENS][{source}] Token row for {mint} not written before its market data: {e}"
                ),
            );
        }
        match write() {
            Ok(()) => return Ok(()),
            Err(e) if !retried => {
                logger::warning(
                    LogTag::Tokens,
                    &format!("[TOKENS][{source}] Market data for {mint} not stored, retrying: {e}"),
                );
                retried = true;
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("[TOKENS][{source}] Failed to store market data for {mint}: {e}"),
                );
                return Err(e);
            }
        }
    }
}
