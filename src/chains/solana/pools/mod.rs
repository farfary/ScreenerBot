// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana pool discovery, on-chain account decoding, and protocol recognition.
//!
//! This module owns everything Solana-specific about finding and reading pools:
//! RPC account fetching (`fetcher`), program account discovery (`discovery`),
//! protocol classification and reserve-account extraction (`analyzer`,
//! `analyzer_extractors`, `types::ProgramKind`), and per-DEX byte decoding
//! (`decoders`). The pricing driver (`driver`) is the only door into these
//! components from neutral code, reached through the chain runtime.
//! Chain-neutral pool persistence, caching, pricing policy and service
//! lifecycle stay in `crate::pools` and consume this module's output
//! (`PoolDescriptor`, `PriceResult`) — see `crate::pools` for that boundary.

pub mod analyzer;
mod analyzer_extractors;
pub mod calculator;
pub mod decode_utils;
pub mod decoders;
pub mod discovery;
pub mod driver;
pub mod fetcher;
mod fetcher_ops;
mod fetcher_types;
pub mod reserve_accounts;
pub mod selection;
pub mod service;
pub mod types;
