//! Router Implementations Module
//! Exports all swap router implementations

mod gmgn;
mod jupiter;
mod raydium;

pub use gmgn::GmgnRouter;
pub use jupiter::JupiterRouter;
pub use raydium::RaydiumRouter;

// Shared referral fee-account resolver, used by the multi-wallet tool executor
// so it collects the same 0.5% referral revenue as the main router.
pub(crate) use jupiter::referral_fee_account;
