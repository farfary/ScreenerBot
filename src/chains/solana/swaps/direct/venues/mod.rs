// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! One module per DEX program the direct engine can swap in.
//!
//! Each venue owns the exact curve its program's accounts describe and the exact
//! instruction that program expects. A pool account's byte layout is decoded in
//! `crate::chains::solana::pools::layouts` where one exists (Raydium AMM v4,
//! Meteora DBC), shared with the PRICE decoders in
//! `crate::chains::solana::pools::decoders`; the other venues still decode their
//! own layout. The price decoders answer "what is this token worth" and depend on
//! a decimals cache. A swap venue reads decimals out of the pool state itself and
//! fails rather than guess, because its numbers become a `min_out` that real money
//! is settled against.

pub mod clmm_ticks;
pub mod fluxbeam;
pub mod math;
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
pub mod token2022;
