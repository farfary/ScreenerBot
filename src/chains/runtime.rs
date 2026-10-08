// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The chain-runtime seam: the only door from neutral code into chain
//! behaviour.
//!
//! The two injected factories (swap routers, the wallet-watch
//! runtime), the token-account read, the filter profile, the discovery
//! feeds, the candle feeds, the pool pricing driver and the settlement reader
//! behind it. Methods are added as further domains thread the chain through
//! (wallets, trading) — a method without a caller is forbidden until then.
//! Neutral code resolves an instance through [`crate::chains::runtime_for`]
//! and never names a concrete chain module.

use std::sync::Arc;

use crate::chains::ChainId;
use crate::swaps::router::SwapRouter;
use crate::wallets::watch::runtime::WalletWatchRuntime;

/// Decimals and authorities of a token's on-chain account, in the chain's own
/// address spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenAccountFacts {
    pub decimals: u8,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
}

/// Chain behaviour installed once at boot for every enabled chain.
#[async_trait::async_trait]
pub trait ChainRuntime: Send + Sync + 'static {
    /// The chain this runtime serves.
    fn id(&self) -> ChainId;
    /// The swap routers this chain contributes to the process registry.
    fn swap_routers(&self) -> Vec<Arc<dyn SwapRouter>>;
    /// The wallet-watch execution runtime for this chain.
    fn wallet_watch_runtime(&self) -> Arc<dyn WalletWatchRuntime>;
    /// Read a token's account from the chain: its decimals and authorities.
    async fn read_token_account(&self, address: &str) -> crate::chains::Result<TokenAccountFacts>;
    /// The filter stages this chain's tokens are evaluated through, in canonical order.
    fn filter_profile(&self) -> Arc<crate::filtering::FilterProfile>;
    /// The discovery feeds this chain contributes beyond the shared market-data
    /// providers, enabled per current config and in discovery order. Read on
    /// every discovery run.
    fn discovery_feeds(&self) -> Vec<crate::tokens::DiscoveryFeed>;
    /// The candle feeds this chain contributes beyond the Data Server and
    /// GeckoTerminal, enabled per current config and in fetch order. Read on
    /// every fetch.
    fn candle_feeds(&self) -> Vec<crate::ohlcvs::CandleFeed>;
    /// The pipeline that discovers, reads and prices this chain's pools.
    fn pricing_driver(&self) -> Arc<dyn crate::pools::PricingDriver>;
    /// The reader of signature verdicts, holdings and expiry bounds on this chain.
    fn settlement(&self) -> Arc<dyn crate::chains::SettlementReader>;
}
