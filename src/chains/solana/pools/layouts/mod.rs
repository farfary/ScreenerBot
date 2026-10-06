// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Byte layouts of Solana pool program accounts.
//!
//! One decoder per program account, read by every consumer of that account: the
//! direct-swap venues (`crate::chains::solana::swaps::direct::venues`), the price
//! decoders (`super::decoders`) and the analyzer's reserve-account derivation
//! (`super::analyzer_extractors`). Each decoder is pure — no RPC, no cache, no clock —
//! and returns `None` rather than a partial state when the bytes do not match.

pub mod clmm_ticks;
pub mod fluxbeam;
pub mod meteora_damm;
pub mod meteora_dbc;
pub mod meteora_dlmm;
pub mod moonit;
pub mod orca_whirlpool;
pub mod pumpfun_amm;
pub mod pumpfun_legacy;
pub mod raydium_amm_v4;
pub mod raydium_clmm;
pub mod raydium_cpmm;
