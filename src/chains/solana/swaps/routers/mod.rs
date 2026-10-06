// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana implementations of `crate::swaps::SwapRouter`.

mod direct_pool;
mod http;
mod jupiter;
mod raptor;

pub use direct_pool::DirectPoolRouter;
pub use jupiter::JupiterRouter;
pub use raptor::RaptorRouter;

pub(crate) use jupiter::venue_label_for_program;
pub(crate) use raptor::health_probe_url;

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
        std::sync::Arc::new(RaptorRouter::new()),
    ]
}

/// Refuse amounts outside Solana's instruction range before router I/O.
pub(super) fn checked_quote_amounts(quote: &crate::swaps::Quote) -> crate::Result<()> {
    match quote.amount_outside_u64() {
        Some((field, amount)) => Err(crate::Error::invalid_amount(
            amount.to_string(),
            format!("{field} exceeds the Solana u64 limit"),
        )),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::{ChainId, RawAmount};
    use crate::swaps::{Quote, SwapMode, SwapRouter};

    #[tokio::test]
    async fn wide_execution_quote_is_refused_before_wallet_or_provider_io() {
        for router in build_routers() {
            for field in ["input", "output", "minimum"] {
                let mut quote = Quote {
                    chain: ChainId::Solana,
                    router_id: router.id().to_owned(),
                    router_name: router.name().to_owned(),
                    input_mint: crate::chains::solana::constants::SOL_MINT.to_owned(),
                    output_mint: crate::chains::solana::constants::USDC_MINT.to_owned(),
                    input_amount: 1u64.into(),
                    output_amount: 1u64.into(),
                    minimum_output_amount: 1u64.into(),
                    price_impact_pct: 0.0,
                    platform_fee_lamports: None,
                    estimated_network_fee_lamports: None,
                    slippage_bps: 100,
                    route_plan: String::new(),
                    swap_mode: SwapMode::ExactIn,
                    wallet_address: String::new(),
                    exclude_dexes: None,
                    execution_data: Vec::new(),
                };
                let wide = RawAmount::new(u128::from(u64::MAX) + 1);
                match field {
                    "input" => quote.input_amount = wide,
                    "output" => quote.output_amount = wide,
                    _ => quote.minimum_output_amount = wide,
                }
                let error = router
                    .execute_swap_for_wallet(&quote, -1)
                    .await
                    .unwrap_err();
                assert!(
                    error.to_string().contains("Solana u64 limit"),
                    "{field}: {error}"
                );
            }
        }
    }
}
