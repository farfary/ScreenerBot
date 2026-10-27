// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Production inputs of the paper executor and the seam its tests replace.

use std::time::Duration;

use crate::positions::paper::PaperBook;
use crate::positions::PositionOrigin;
use crate::swaps::{Quote, QuoteError, QuoteRequest};
use crate::trader::paper::TradingMode;
use crate::trader::types::{TradeAction, TradeDecision};

/// The paper settings one execution reads.
#[derive(Debug, Clone, Copy)]
pub(super) struct PaperSettings {
    pub landing_delay_ms: u64,
    pub starting_balance_native: f64,
    pub max_open: usize,
    /// Seconds after the latest paper open before the next one, the live open cooldown.
    pub open_cooldown_secs: u64,
}

/// What a paper round records of its token.
#[derive(Debug, Clone)]
pub(super) struct PaperToken {
    pub symbol: String,
    pub name: String,
    pub decimals: Option<u8>,
}

/// The market price a paper entry is made at and the pool reserves it was read from.
#[derive(Debug, Clone)]
pub(super) struct PaperMarket {
    pub price_native: f64,
    pub liquidity: Option<(f64, String)>,
}

pub(super) trait PaperTradeIo: Sync {
    fn book(&self) -> Option<&PaperBook>;
    fn wallet_address(&self) -> Option<String>;
    fn settings(&self) -> PaperSettings;
    /// The size of an entry, from the sizing owner the live buy uses.
    fn entry_size(&self, decision: &TradeDecision) -> f64;
    /// Whether the decision may still fill: nothing force-stopped, an automatic trade
    /// with the auto-trader running, a strategy buy from a strategy still enabled in
    /// Paper mode, and a trade of a copy round its copy task still wants
    /// (`trader::copy::paper_trade_wanted`).
    async fn still_wanted(&self, decision: &TradeDecision, origin: &PositionOrigin) -> bool;
    /// The token of the trade, fetched on demand on the lane a real trade of `origin` uses.
    async fn token(&self, mint: &str, origin: &PositionOrigin) -> Option<PaperToken>;
    async fn market_price(&self, mint: &str) -> Option<PaperMarket>;
    async fn quote(&self, request: QuoteRequest) -> Result<Quote, QuoteError>;
    async fn wait(&self, duration: Duration);
}

pub(super) struct ProductionPaperTradeIo;

impl PaperTradeIo for ProductionPaperTradeIo {
    fn book(&self) -> Option<&PaperBook> {
        crate::positions::paper::book()
    }

    fn wallet_address(&self) -> Option<String> {
        crate::utils::get_wallet_address().ok()
    }

    fn settings(&self) -> PaperSettings {
        crate::config::with_config(|config| PaperSettings {
            landing_delay_ms: config.trader.paper.landing_delay_ms,
            starting_balance_native: config.trader.paper.starting_balance_sol,
            max_open: config.trader.max_open_positions,
            open_cooldown_secs: config.positions.position_open_cooldown_secs.max(0) as u64,
        })
    }

    fn entry_size(&self, decision: &TradeDecision) -> f64 {
        crate::trader::executors::entry_size_native(decision)
    }

    async fn still_wanted(&self, decision: &TradeDecision, origin: &PositionOrigin) -> bool {
        if crate::global::is_force_stopped() {
            return false;
        }
        if super::is_manual(&decision.reason) {
            return true;
        }
        match origin {
            PositionOrigin::Auto { .. } => {}
            PositionOrigin::Copy { task_id, .. } => {
                return crate::trader::copy::paper_trade_wanted(*task_id, decision).await;
            }
            PositionOrigin::Manual | PositionOrigin::External => return true,
        }
        if !crate::trader::config::is_trader_enabled() {
            return false;
        }
        match (&decision.action, decision.strategy_id.as_deref()) {
            (TradeAction::Buy | TradeAction::DCA, Some(strategy_id)) => {
                matches!(
                    crate::strategies::db::get_strategy_async(strategy_id).await,
                    Ok(Some(strategy)) if strategy.enabled && strategy.mode == TradingMode::Paper
                )
            }
            _ => true,
        }
    }

    async fn token(&self, mint: &str, origin: &PositionOrigin) -> Option<PaperToken> {
        crate::tokens::ensure_token_available(
            crate::positions::paper::book()?.chain(),
            mint,
            crate::positions::operations::token_fetch_lane(origin),
        )
        .await
        .ok()
        .map(|token| PaperToken {
            symbol: token.symbol,
            name: token.name,
            decimals: token.decimals,
        })
    }

    async fn market_price(&self, mint: &str) -> Option<PaperMarket> {
        let (price, _) =
            crate::positions::price_resolution::get_price_with_api_fallback(mint).await?;
        if !price.price_native.is_finite() || price.price_native <= 0.0 {
            return None;
        }
        let liquidity = (price.native_reserves.is_finite()
            && price.native_reserves > 0.0
            && !price.pool_address.is_empty())
        .then(|| (price.native_reserves, price.pool_address.clone()));
        Some(PaperMarket {
            price_native: price.price_native,
            liquidity,
        })
    }

    async fn quote(&self, request: QuoteRequest) -> Result<Quote, QuoteError> {
        crate::swaps::try_get_best_quote(request).await
    }

    async fn wait(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
