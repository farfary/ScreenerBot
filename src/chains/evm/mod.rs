// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! EVM chain family: the vendor dependency boundary shared by every EVM chain.
//!
//! Application code reaches the alloy crates only through this façade, so the
//! EVM dependency surface has one enforceable owner. Integration tests reach
//! the vendor through the `alloy` re-export below instead of a second
//! dependency declaration. Chain-neutral modules never name an alloy type:
//! values cross the boundary as neutral types (`RawAmount`, `AssetId`,
//! `AccountId`). Per-chain facts are data in [`spec::EvmChainSpec`], with
//! pinned contract addresses in [`constants`].

pub use alloy;

pub mod constants;
pub mod spec;
