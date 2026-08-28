//! One module per DEX program the direct engine can swap in.
//!
//! Each venue owns the exact byte layout of its program's accounts, the exact
//! curve those accounts describe, and the exact instruction that program expects.
//! Nothing here is shared with the PRICE decoders in
//! `crate::chains::solana::pools::decoders`: those answer "what is this token
//! worth", depend on a decimals cache, and may fall back to an approximation. A
//! swap venue may not — it reads decimals out of the pool state itself and fails
//! rather than guess, because its numbers become a `min_out` that real money is
//! settled against.

pub mod clmm_ticks;
pub mod layout;
pub mod math;
pub mod meteora_damm;
pub mod orca_whirlpool;
pub mod pumpfun_amm;
pub mod raydium_amm_v4;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod token2022;
