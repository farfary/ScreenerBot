// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The initialised-tick entry both concentrated-liquidity tick-array decoders
//! produce: Raydium CLMM's `TickArrayState` (`super::raydium_clmm`) and Orca
//! Whirlpool's `TickArray`/`DynamicTickArray` (`super::orca_whirlpool`).

/// One initialised tick out of a decoded `TickArrayState`: the entries whose
/// `liquidity_gross` is non-zero, which is what an active position actually
/// requires -- a slot with `tick == 0, liquidity_net == 0, liquidity_gross ==
/// 0` is simply an unused array slot, not a real tick at index 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitializedTick {
    pub tick: i32,
    pub liquidity_net: i128,
}
