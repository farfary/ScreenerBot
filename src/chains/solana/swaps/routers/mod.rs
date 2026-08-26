//! Solana implementations of `crate::swaps::SwapRouter`.

mod direct_pool;
mod jupiter;

pub use direct_pool::DirectPoolRouter;
pub use jupiter::JupiterRouter;

/// Build the Solana swap router set for `crate::swaps::registry::RouterRegistry`.
/// This is the factory the application composition root registers via
/// `crate::swaps::registry::set_router_factory` — add new Solana routers here.
///
/// Note the shape: `DirectPoolRouter` is ONE router covering every DEX the direct
/// engine has a venue for. Adding a venue does not add a router.
pub fn build_routers() -> Vec<std::sync::Arc<dyn crate::swaps::router::SwapRouter>> {
    vec![
        std::sync::Arc::new(JupiterRouter::new()),
        std::sync::Arc::new(DirectPoolRouter::new()),
    ]
}
