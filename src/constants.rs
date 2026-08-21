//! Chain-neutral global constants used across ScreenerBot.
//!
//! Empty today: SOL/WSOL/USDC/USDT mint addresses, decimals and lamport
//! conversion constants are Solana-address-shaped and live in
//! `crate::chains::solana::constants` (the sole chain implemented). Shared
//! code that needs "the native asset" conceptually uses
//! `crate::chains::{SOLANA_NATIVE_ASSET, ChainMetadata}` instead of a raw
//! literal. This module is the home for a genuinely chain-neutral constant
//! when a second chain exists — do not re-export a Solana literal through it.
