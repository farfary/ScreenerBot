// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The tokens no eviction may remove: everything a position, the wallet, the trader, the filters, a tool or an action in progress holds or watches right now.

use std::collections::{HashMap, HashSet};

use crate::storage::types::Protection;

/// The tokens in use when an eviction is planned, each with the strongest reason.
#[derive(Debug, Default, Clone)]
pub struct ProtectedMints {
    mints: HashMap<String, Protection>,
}

impl ProtectedMints {
    /// Record `mint` as protected for `protection`, keeping a stronger reason already
    /// recorded.
    pub fn insert(&mut self, mint: String, protection: Protection) {
        let rank = |protection: Protection| match protection {
            Protection::OpenPosition => 0,
            Protection::Held => 1,
            Protection::Watched => 2,
            Protection::RecentlyViewed => 3,
        };
        self.mints
            .entry(mint)
            .and_modify(|current| {
                if rank(protection) < rank(*current) {
                    *current = protection;
                }
            })
            .or_insert(protection);
    }

    pub fn protection(&self, mint: &str) -> Option<Protection> {
        self.mints.get(mint).copied()
    }

    pub fn mints(&self) -> HashSet<String> {
        self.mints.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.mints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mints.is_empty()
    }
}

/// Why the tokens in use could not be listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedMintsUnavailable {
    /// Open positions are not loaded yet, so an open position's token could be missed.
    PositionsNotReady,
    /// The watched tokens of the tools could not be read.
    WatchedTokens,
}

/// Every token that is in use right now on any chain: open positions and paper
/// holdings, wallet holdings, tokens that passed the filters or have a pool price,
/// tokens watched by a tool and the subjects of actions in progress. The chart
/// monitor's tokens are read from the chart store by the caller. Refuses while open
/// positions are not loaded, so nothing is evicted on a partial view.
pub async fn protected_mints() -> Result<ProtectedMints, ProtectedMintsUnavailable> {
    use std::sync::atomic::Ordering;
    if !crate::global::POSITIONS_SYSTEM_READY.load(Ordering::SeqCst) {
        return Err(ProtectedMintsUnavailable::PositionsNotReady);
    }
    let mut protected = ProtectedMints::default();
    for mint in crate::positions::get_open_mints().await {
        protected.insert(mint, Protection::OpenPosition);
    }
    for mint in crate::trader::copy::held_paper_mints() {
        protected.insert(mint, Protection::OpenPosition);
    }
    for mint in crate::wallet::get_held_mints() {
        protected.insert(mint, Protection::Held);
    }
    for &chain in crate::chains::ChainId::ALL {
        for mint in super::filtered::get_passed_tokens(chain)
            .into_iter()
            .chain(super::filtered::get_tokens_with_pool_price(chain))
        {
            protected.insert(mint, Protection::Watched);
        }
    }
    for action in crate::actions::state::get_active_actions().await {
        protected.insert(action.entity_id, Protection::Watched);
    }
    let watched = tokio::task::spawn_blocking(crate::tools::get_watched_tokens)
        .await
        .map_err(|_| ProtectedMintsUnavailable::WatchedTokens)?
        .map_err(|_| ProtectedMintsUnavailable::WatchedTokens)?;
    for token in watched {
        protected.insert(token.mint, Protection::Watched);
    }
    Ok(protected)
}
