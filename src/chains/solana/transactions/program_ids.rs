// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Program IDs for transaction analysis and router detection
//!
//! This module contains all program IDs needed for analyzing transactions
//! and detecting which platform/router was used for swaps. This is separate
//! from the pools module to maintain clean separation of concerns.
//!
//! The DEX/aggregator program IDs shared with pool decoding are re-exported
//! from `crate::chains::solana::constants` (the single source of truth) so
//! this module and `crate::chains::solana::pools::types` never drift.

pub use crate::chains::solana::constants::{
    FLUXBEAM_AMM_PROGRAM_ID, JUPITER_V4_PROGRAM_ID, JUPITER_V6_PROGRAM_ID, METEORA_DAMM_PROGRAM_ID,
    METEORA_DBC_PROGRAM_ID, METEORA_DLMM_PROGRAM_ID, MOONIT_AMM_PROGRAM_ID,
    ORCA_WHIRLPOOL_PROGRAM_ID, PUMP_FUN_AMM_PROGRAM_ID, PUMP_FUN_LEGACY_PROGRAM_ID,
    RAYDIUM_CLMM_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID, RAYDIUM_LEGACY_AMM_PROGRAM_ID,
};

// =============================================================================
// DEX AGGREGATOR PROGRAM IDS
// =============================================================================

pub const JUPITER_V3_PROGRAM_ID: &str = "JUP3c2Uh3WA4Ng34tw6kPd2G4C5BB21Xo36Je1s32Ph";

/// GMGN - Gaming and social trading platform
pub const GMGN_PROGRAM_ID: &str = "GMGNjvGr7ddxt2u1XSf8Zo6LLnDjDm9mJahGfhq7j6gk";

/// Raptor (Solana Tracker) - the aggregator program our `raptor` router routes
/// through. Without this, our OWN Raptor swaps analyse as an unknown program.
pub const RAPTOR_PROGRAM_ID: &str = "RaptorD5ojtsqDDtJeRsunPLg6GvLYNnwKJWxYE4m87";

// =============================================================================
// DIRECT DEX PROGRAM IDS
// =============================================================================

/// Meteora constant-product and stable pools (legacy)
const METEORA_POOLS_PROGRAM_ID: &str = "Dooar9JkhdZ7J3LHN3A7YCuoGRUggXhQaG4kijfLGU2j";
const LIFINITY_PROGRAM_ID: &str = "EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S";
const ALDRIN_PROGRAM_ID: &str = "AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6";
const SERUM_V3_PROGRAM_ID: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";
const SERUM_V2_PROGRAM_ID: &str = "srmqPiDkJokFGBWxH3qzowH4NhGFaKjR5Ek8TRnq6PZ";
const OPENBOOK_PROGRAM_ID: &str = "opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb";
const PHOENIX_PROGRAM_ID: &str = "PhoeNiX7BPQtuPBGYWf5KhxZVsXBMNzC9mHvgSe3kfE";

/// A market-maker fill program: it settles the swap itself with token transfers and
/// calls no pool program (instruction log `Fill`). Its operator is not established,
/// so the router id is `unknown`.
const MARKET_MAKER_FILL_PROGRAM_ID: &str = "61DFfeTKM7trxYcPQCM78bJ794ddZprZpAwAnLiwTpYH";

/// Raydium LaunchLab, the bonding-curve launchpad; trades are `BuyExactIn` and
/// `SellExactIn`. It holds no pool decoder, so it is listed here for analysis only.
const RAYDIUM_LAUNCHLAB_PROGRAM_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";

/// Boop, a bonding-curve launchpad that trades through `BuyToken` and `SellToken`. Its
/// operator is not established, so the router id is `unknown`.
const BOOP_PROGRAM_ID: &str = "boop8hVGQGqehUK2iVEMEnMrL5RbjywRzHKBmBE7ry4";

/// A routing program that trades pump.fun bonding curves and PumpSwap pools, directly or
/// behind Jupiter. Its upgrade authority is not pump.fun's, so the router id is `unknown`;
/// it sits after every named venue, so the program it calls names the swap.
const PUMP_ROUTING_PROGRAM_ID: &str = "6Vo3245eszAb5wuqEMw8mGdbfRUdKbHhDHP5LcaGuTAB";

/// Orca DEX (legacy variant not shared with pool decoding)
pub const ORCA_V1_PROGRAM_ID: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";

// =============================================================================
// KNOWN PROGRAM TABLE
// =============================================================================

/// A program the analyzer recognises as a swap venue or aggregator.
pub struct KnownProgram {
    pub id: &'static str,
    /// Stable router id stored on a transaction; every id is a `DetectedDex::router_id`
    /// except `unknown`.
    pub router: &'static str,
    /// An aggregator routes through other venues' programs, so it names the swap
    /// whenever it is present.
    pub aggregator: bool,
}

const fn aggregator(id: &'static str, router: &'static str) -> KnownProgram {
    KnownProgram {
        id,
        router,
        aggregator: true,
    }
}

const fn venue(id: &'static str, router: &'static str) -> KnownProgram {
    KnownProgram {
        id,
        router,
        aggregator: false,
    }
}

/// Every swap program the analyzer knows, aggregators first. The order is the tie-break
/// when a transaction holds several known programs, so `unknown` entries stay last.
pub(crate) const KNOWN_PROGRAMS: &[KnownProgram] = &[
    aggregator(RAPTOR_PROGRAM_ID, "raptor"),
    aggregator(JUPITER_V6_PROGRAM_ID, "jupiter"),
    aggregator(JUPITER_V4_PROGRAM_ID, "jupiter"),
    aggregator(JUPITER_V3_PROGRAM_ID, "jupiter"),
    aggregator(GMGN_PROGRAM_ID, "gmgn"),
    venue(RAYDIUM_CPMM_PROGRAM_ID, "raydium"),
    venue(RAYDIUM_LEGACY_AMM_PROGRAM_ID, "raydium"),
    venue(RAYDIUM_CLMM_PROGRAM_ID, "raydiumclmm"),
    venue(RAYDIUM_LAUNCHLAB_PROGRAM_ID, "raydium"),
    venue(ORCA_WHIRLPOOL_PROGRAM_ID, "orcawhirlpool"),
    venue(ORCA_V1_PROGRAM_ID, "orca"),
    venue(METEORA_DAMM_PROGRAM_ID, "meteora"),
    venue(METEORA_DLMM_PROGRAM_ID, "meteora"),
    venue(METEORA_DBC_PROGRAM_ID, "meteora"),
    venue(METEORA_POOLS_PROGRAM_ID, "meteora"),
    venue(PUMP_FUN_AMM_PROGRAM_ID, "pumpfun"),
    venue(PUMP_FUN_LEGACY_PROGRAM_ID, "pumpfun"),
    venue(MOONIT_AMM_PROGRAM_ID, "moonshot"),
    venue(FLUXBEAM_AMM_PROGRAM_ID, "fluxbeam"),
    venue(LIFINITY_PROGRAM_ID, "lifinity"),
    venue(ALDRIN_PROGRAM_ID, "aldrin"),
    venue(SERUM_V3_PROGRAM_ID, "serum"),
    venue(SERUM_V2_PROGRAM_ID, "serum"),
    venue(OPENBOOK_PROGRAM_ID, "openbook"),
    venue(PHOENIX_PROGRAM_ID, "phoenix"),
    venue(MARKET_MAKER_FILL_PROGRAM_ID, "unknown"),
    venue(BOOP_PROGRAM_ID, "unknown"),
    venue(PUMP_ROUTING_PROGRAM_ID, "unknown"),
];

// =============================================================================
// ROUTER DETECTION FUNCTIONS
// =============================================================================

/// The known program that names a transaction's swap: the first aggregator present, else
/// the first venue. The result depends on the table order, never on the order of
/// `program_ids`.
pub fn detect_known_program<S: AsRef<str>>(program_ids: &[S]) -> Option<&'static KnownProgram> {
    let present = |known: &&KnownProgram| program_ids.iter().any(|id| id.as_ref() == known.id);
    KNOWN_PROGRAMS
        .iter()
        .filter(|known| known.aggregator)
        .find(present)
        .or_else(|| KNOWN_PROGRAMS.iter().find(present))
}

/// Router id of the program that names a transaction's swap (see [`detect_known_program`]).
pub fn detect_router<S: AsRef<str>>(program_ids: &[S]) -> Option<&'static str> {
    detect_known_program(program_ids).map(|known| known.router)
}

/// Detect router/platform from a single program ID
pub fn detect_router_from_program_id(program_id: &str) -> Option<&'static str> {
    KNOWN_PROGRAMS
        .iter()
        .find(|known| known.id == program_id)
        .map(|known| known.router)
}

/// Get all known Jupiter program IDs for validation
pub fn get_jupiter_program_ids() -> &'static [&'static str] {
    &[
        JUPITER_V6_PROGRAM_ID,
        JUPITER_V4_PROGRAM_ID,
        JUPITER_V3_PROGRAM_ID,
    ]
}

/// Check if a program ID belongs to Jupiter
pub fn is_jupiter_program_id(program_id: &str) -> bool {
    matches!(
        program_id,
        JUPITER_V6_PROGRAM_ID | JUPITER_V4_PROGRAM_ID | JUPITER_V3_PROGRAM_ID
    )
}

/// Check if a program ID belongs to any known DEX aggregator
pub fn is_dex_aggregator_program_id(program_id: &str) -> bool {
    KNOWN_PROGRAMS
        .iter()
        .any(|known| known.aggregator && known.id == program_id)
}

// =============================================================================
// MEV AND PRIORITY FEE ADDRESSES
// =============================================================================

/// Known MEV/Jito tip addresses that should be excluded from swap calculations
pub const KNOWN_MEV_TIP_ADDRESSES: &[&str] = &[
    "BB5dnY55FXS1e1NXqZDwCzgdYJdMCj3B92PU6Q5Fb6DT", // Jito tip address
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5", // Jito tip address
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe", // Jito tip address
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY", // Jito tip address
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49", // Jito tip address
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh", // Jito tip address
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt", // Jito tip address
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL", // Jito tip address
];

/// Check if an address is a known MEV/Jito tip address
pub fn is_mev_tip_address(address: &str) -> bool {
    KNOWN_MEV_TIP_ADDRESSES.contains(&address)
}
