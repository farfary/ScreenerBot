//! Solana swap mechanics: DEX instruction building, transaction execution, and
//! the concrete Jupiter/GMGN/Raydium adapters behind `crate::swaps::SwapRouter`.
//!
//! Owns every Solana-specific detail of turning a swap intent into a landed
//! transaction: program IDs, instruction discriminators, account metas,
//! WSOL wrap/unwrap, ATA preparation (via `crate::chains::solana::assets`),
//! signing and submission (via `crate::chains::solana::rpc`), and the
//! endpoint-specific Jupiter/GMGN response decoding. Chain-neutral swap
//! intent, quoting policy, routing/fallback orchestration and the
//! `SwapRouter` contract stay in `crate::swaps` — this module implements
//! that contract, it does not own it.

pub mod builder;
pub mod executor;
pub mod programs;
pub mod routers;
pub mod types;

pub use builder::SwapBuilder;
pub use executor::SwapExecutor;
pub use programs::raydium_clmm::RaydiumClmmSwap;
pub use programs::raydium_cpmm::RaydiumCpmmSwap;
pub use routers::{GmgnRouter, JupiterRouter, RaydiumRouter};
pub use types::{SwapDirection, SwapError, SwapRequest, SwapResult};
