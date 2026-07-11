//! Swap execution — routing, quoting, and transaction building for token trades.
pub mod operations;
pub mod registry;
pub mod router;
pub mod routers;
pub mod types;

// Re-export router system
pub use operations::{
    execute_swap_with_fallback, get_best_quote, get_best_quote_for_opening,
    unconfirmed_swap_signature,
};
pub use registry::{get_registry, RouterRegistry};
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
