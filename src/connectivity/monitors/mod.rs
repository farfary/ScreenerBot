// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Health monitors for external API endpoints (DEX, RPC, Jupiter, etc.).
pub mod dexscreener;
pub mod geckoterminal;
pub mod internet;
pub mod jupiter;
pub mod raptor;
pub mod rpc;
pub mod rugcheck;

pub use dexscreener::DexScreenerMonitor;
pub use geckoterminal::GeckoTerminalMonitor;
pub use internet::InternetMonitor;
pub use jupiter::JupiterMonitor;
pub use raptor::RaptorMonitor;
pub use rpc::RpcMonitor;
pub use rugcheck::RugcheckMonitor;
