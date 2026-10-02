// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana-only endpoint health monitors (moved out of `src/connectivity`).

pub mod jupiter;
pub mod raptor;

pub use jupiter::JupiterMonitor;
pub use raptor::RaptorMonitor;

/// The monitors the neutral connectivity checker cannot construct itself
/// (installed through `crate::connectivity::checker::set_chain_monitors`).
pub fn monitors() -> Vec<Box<dyn crate::connectivity::monitor::EndpointMonitor>> {
    vec![
        Box::new(JupiterMonitor::new()),
        Box::new(RaptorMonitor::new()),
    ]
}
