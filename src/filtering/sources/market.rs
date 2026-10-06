// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Market filter stage — applies the DexScreener or GeckoTerminal rules to the token's
//! market data, whichever provider the batch load resolved for it.

use crate::chains::ChainId;
use crate::config::FilteringConfig;
use crate::filtering::sources::{self, FilterRejectionReason};
use crate::filtering::stage::{FilterStage, StageEval};
use crate::tokens::types::{DataSource, Token};

/// Both market data providers cover every chain, so this stage applies everywhere.
pub struct MarketStage;

impl FilterStage for MarketStage {
    fn name(&self) -> &'static str {
        "market"
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
        StageEval::Ready(evaluate(token, config).into())
    }
}

fn evaluate(token: &Token, config: &FilteringConfig) -> Result<(), FilterRejectionReason> {
    // DexScreener and GeckoTerminal are two PROVIDERS OF THE SAME market data, and the
    // batch load resolves exactly one of them per token (`data_source`). So each enabled
    // source is applied only to the tokens it actually covers, and a token is rejected for
    // missing market data only when NONE of the enabled sources covers it.
    //
    // Demanding both — which is what running the two checks independently amounted to —
    // is unsatisfiable: with both enabled (the shipped default) every token failed
    // whichever gate did not match its single `data_source`, so the pipeline passed
    // literally nothing. Measured against the production database: 100% rejected.
    let mut market_data_seen = false;

    if config.dexscreener.enabled && token.data_source == DataSource::DexScreener {
        sources::dexscreener::evaluate(token, &config.dexscreener)?;
        market_data_seen = true;
    }

    if config.geckoterminal.enabled && token.data_source == DataSource::GeckoTerminal {
        sources::geckoterminal::evaluate(token, &config.geckoterminal)?;
        market_data_seen = true;
    }

    if !market_data_seen {
        // Attribute the rejection to an enabled source the token is genuinely missing.
        // With only one source enabled this is precise ("you asked for DexScreener rules
        // and this token has GeckoTerminal data"); with both enabled the token has no
        // market data at all and DexScreener is the primary source.
        if config.dexscreener.enabled {
            return Err(FilterRejectionReason::DexScreenerDataMissing);
        }
        if config.geckoterminal.enabled {
            return Err(FilterRejectionReason::GeckoTerminalDataMissing);
        }
    }

    Ok(())
}
