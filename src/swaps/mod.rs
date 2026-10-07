// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Swap routing, quoting and fallback orchestration — chain-neutral. The concrete
//! DEX adapters implementing `SwapRouter` (Jupiter, Raydium) live under
//! `crate::chains::solana::swaps::routers`; this module owns only the
//! `SwapRouter` contract, the registry that wires adapters up, and the
//! router-agnostic quote/execute-with-fallback policy.
pub mod error;
pub mod operations;
mod operations_wallet;
pub mod progress;
pub mod registry;
pub mod router;
pub mod types;

// Re-export router system
pub use error::{
    NotOfferedReason, NotSubmittedReason, QuoteError, QuoteResult, SwapExecutionError,
};
pub use operations::{
    execute_swap_with_fallback, get_best_quote, get_best_quote_for_opening, try_get_best_quote,
    unconfirmed_swap_signature,
};
pub use operations_wallet::quote_and_execute_for_wallet;
pub use progress::{with_swap_stage_listener, SwapStage, SwapStageListener};
pub use registry::{get_registry, try_get_registry, RouterRegistry};
pub use router::SwapRouter;
pub use types::{
    ExitType, Quote, QuoteRequest, RouterChoice, SwapAmountLimit, SwapMode, SwapResult,
};

use crate::chains::RawAmount;

/// The share of `total_amount` for `percentage`, truncated toward zero and never above the
/// total. Zero for an empty total or a NaN or nonpositive percentage; the whole total at 100%
/// or more.
pub fn calculate_partial_amount(total_amount: RawAmount, percentage: f64) -> RawAmount {
    if total_amount == RawAmount::ZERO || percentage.is_nan() || percentage <= 0.0 {
        return RawAmount::ZERO;
    }
    if percentage >= 100.0 {
        return total_amount;
    }
    let partial = (total_amount.raw() as f64 * percentage / 100.0).trunc();
    RawAmount::from_integral_f64(partial).map_or(total_amount, |partial| partial.min(total_amount))
}
