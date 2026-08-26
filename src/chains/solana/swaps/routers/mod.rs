//! Solana implementations of `crate::swaps::SwapRouter`: Jupiter, Raydium.

mod jupiter;
mod raydium;

pub use jupiter::JupiterRouter;
pub use raydium::RaydiumRouter;

/// Build the Solana swap router set for `crate::swaps::registry::RouterRegistry`.
/// This is the factory the application composition root registers via
/// `crate::swaps::registry::set_router_factory` — add new Solana routers here.
pub fn build_routers() -> Vec<std::sync::Arc<dyn crate::swaps::router::SwapRouter>> {
    vec![
        std::sync::Arc::new(JupiterRouter::new()),
        std::sync::Arc::new(RaydiumRouter::new()),
    ]
}
