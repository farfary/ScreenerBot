//! Swap routing, quoting and fallback orchestration — chain-neutral. The concrete
//! DEX adapters implementing `SwapRouter` (Jupiter, GMGN, Raydium) live under
//! `crate::chains::solana::swaps::routers`; this module owns only the
//! `SwapRouter` contract, the registry that wires adapters up, and the
//! router-agnostic quote/execute-with-fallback policy.
pub mod error;
pub mod operations;
mod operations_wallet;
pub mod registry;
pub mod router;
pub mod types;

// Re-export router system
pub use error::{QuoteError, QuoteResult};
pub use operations::{
    execute_swap_with_fallback, get_best_quote, get_best_quote_for_opening, try_get_best_quote,
    unconfirmed_swap_signature, unconfirmed_swap_signature_from_message,
};
pub use operations_wallet::quote_and_execute_for_wallet;
pub use registry::{get_registry, try_get_registry, RouterRegistry};
pub use router::SwapRouter;
pub use types::{ExitType, Quote, QuoteRequest, RouterType, SwapMode, SwapResult};

/// Calculate the token amount for a partial exit
/// Returns 0 if total_amount is 0 or percentage is <= 0
/// Returns total_amount if percentage is >= 100
pub fn calculate_partial_amount(total_amount: u64, percentage: f64) -> u64 {
    if total_amount == 0 || percentage <= 0.0 {
        return 0;
    }
    if percentage >= 100.0 {
        return total_amount;
    }

    let partial = (total_amount as f64 * percentage / 100.0) as u64;
    partial.min(total_amount)
}
