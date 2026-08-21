//! Solana implementations of `crate::swaps::SwapRouter`: Jupiter, GMGN, Raydium.

mod gmgn;
mod jupiter;
mod raydium;

pub use gmgn::GmgnRouter;
pub use jupiter::JupiterRouter;
pub use raydium::RaydiumRouter;

/// Build the Solana swap router set for `crate::swaps::registry::RouterRegistry`.
/// This is the factory the application composition root registers via
/// `crate::swaps::registry::set_router_factory` — add new Solana routers here.
pub fn build_routers() -> Vec<std::sync::Arc<dyn crate::swaps::router::SwapRouter>> {
    vec![
        std::sync::Arc::new(JupiterRouter::new()),
        std::sync::Arc::new(GmgnRouter::new()),
        std::sync::Arc::new(RaydiumRouter::new()),
    ]
}

// Shared referral fee-account resolver, used by the multi-wallet tool executor
// so it collects the same 0.5% referral revenue as the main router.
pub(crate) use jupiter::referral_fee_account;

// Jupiter build/sign/submit with a caller-supplied keypair, shared by the
// multi-wallet tool executor (`crate::tools::swap_executor`).
pub(crate) use jupiter::execute_with_keypair;

/// Build, sign and submit a Jupiter swap transaction for a wallet identified
/// only by its database ID. Resolves and decrypts the keypair here, inside
/// `crate::chains::solana`, so `crate::tools::swap_executor` (and everything
/// it's called from) never holds a `Keypair`.
pub(crate) async fn execute_for_wallet(
    quote: &crate::swaps::types::Quote,
    wallet_id: i64,
) -> std::result::Result<String, String> {
    let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
    execute_with_keypair(quote, &keypair).await
}
