// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The chain-runtime seam: the only door from neutral code into chain
//! behaviour.
//!
//! The two injected factories (swap routers, the wallet-watch
//! runtime) behind it. Later units add methods as their domains thread the
//! chain through (discovery, pool pricing, OHLCV,
//! wallets, trading) — a method without a caller is forbidden until
//! then. Neutral code resolves an instance through
//! [`crate::chains::runtime_for`] and never names a concrete chain module.

use std::sync::Arc;

use crate::chains::ChainId;
use crate::swaps::router::SwapRouter;
use crate::wallets::watch::runtime::WalletWatchRuntime;

/// Chain behaviour installed once at boot for every enabled chain.
pub trait ChainRuntime: Send + Sync + 'static {
    /// The chain this runtime serves.
    fn id(&self) -> ChainId;
    /// The swap routers this chain contributes to the process registry.
    fn swap_routers(&self) -> Vec<Arc<dyn SwapRouter>>;
    /// The wallet-watch execution runtime for this chain.
    fn wallet_watch_runtime(&self) -> Arc<dyn WalletWatchRuntime>;
}
