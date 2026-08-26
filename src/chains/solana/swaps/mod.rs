//! Solana swap mechanics: the concrete Jupiter and direct-pool adapters behind
//! `crate::swaps::SwapRouter`, plus everything Solana-specific about turning a
//! swap intent into a landed transaction.
//!
//! Two execution mechanisms live here and they are deliberately different:
//!
//! * `routers::JupiterRouter` — an aggregator quotes and builds the transaction
//!   for us; we sign and send it.
//! * `direct` — we decode the pool, compute the curve, build the instruction and
//!   attach our own fee. No third party in the money path.
//!
//! `revenue` holds the fee rate and destinations BOTH use, so there is exactly
//! one definition of what a ScreenerBot swap charges.
//!
//! Chain-neutral swap intent, quoting policy, routing/fallback orchestration and
//! the `SwapRouter` contract stay in `crate::swaps` — this module implements that
//! contract, it does not own it.

pub mod direct;
pub mod revenue;
pub mod routers;

pub use direct::{DirectSwapIntent, DirectSwapOutcome, DirectSwapResult};
pub use routers::{DirectPoolRouter, JupiterRouter};
