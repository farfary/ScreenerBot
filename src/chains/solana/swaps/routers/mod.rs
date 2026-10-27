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

use crate::chains::solana::solana_sdk::signature::Keypair;
use crate::swaps::{NotSubmittedReason, SwapExecutionError};

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
        Some((field, amount)) => Err(not_submitted(
            &quote.router_name,
            NotSubmittedReason::QuoteUnusable {
                detail: format!("{field} {amount} exceeds the Solana u64 limit"),
            },
        )),
        None => Ok(()),
    }
}

/// The wallet's signing key for an execution by `router`. A key that does not
/// load is refused before anything is built or signed.
pub(super) fn signing_keypair(router: &str) -> crate::Result<Keypair> {
    crate::chains::solana::accounts::configured_keypair().map_err(|error| {
        not_submitted(
            router,
            NotSubmittedReason::SignerUnavailable {
                detail: error.to_string(),
            },
        )
    })
}

fn not_submitted(router: &str, reason: NotSubmittedReason) -> crate::Error {
    crate::Error::Swaps(SwapExecutionError::NotSubmitted {
        router: router.to_owned(),
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::{ChainId, RawAmount};
    use crate::swaps::{Quote, SwapMode};

    fn traded_token() -> crate::tokens::Token {
        serde_json::from_value(serde_json::json!({
            "mint": crate::chains::solana::constants::USDC_MINT,
            "symbol": "USDC",
            "name": "USD Coin",
            "data_source": "DexScreener",
            "first_discovered_at": "2026-01-01T00:00:00Z",
            "metadata_last_fetched_at": "2026-01-01T00:00:00Z",
            "decimals_last_fetched_at": "2026-01-01T00:00:00Z",
            "market_data_last_fetched_at": "2026-01-01T00:00:00Z",
            "pool_price_last_calculated_at": "2026-01-01T00:00:00Z",
            "price_usd": 0.0,
            "price_sol": 0.0,
            "price_native": "0",
            "websites": [],
            "socials": [],
            "is_rugged": false,
            "security_risks": [],
            "top_holders": [],
            "is_blacklisted": false,
            "priority": "PoolTracked",
        }))
        .expect("token fixture")
    }

    fn quote_for(router_id: &str, router_name: &str) -> Quote {
        Quote {
            chain: ChainId::Solana,
            router_id: router_id.to_owned(),
            router_name: router_name.to_owned(),
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
        }
    }

    fn quote_unusable_detail(error: &crate::Error) -> Option<&str> {
        match error {
            crate::Error::Swaps(SwapExecutionError::NotSubmitted {
                reason: NotSubmittedReason::QuoteUnusable { detail },
                ..
            }) => Some(detail),
            _ => None,
        }
    }

    #[tokio::test]
    async fn wide_execution_quote_is_refused_before_wallet_or_provider_io() {
        let token = traded_token();
        for router in build_routers() {
            for field in ["input", "output", "minimum"] {
                let mut quote = quote_for(router.id(), router.name());
                let wide = RawAmount::new(u128::from(u64::MAX) + 1);
                match field {
                    "input" => quote.input_amount = wide,
                    "output" => quote.output_amount = wide,
                    _ => quote.minimum_output_amount = wide,
                }
                let error = router.execute_swap(&token, &quote).await.unwrap_err();
                let detail = quote_unusable_detail(&error)
                    .unwrap_or_else(|| panic!("{field}: not a typed refusal: {error}"));
                assert!(detail.contains("Solana u64 limit"), "{field}: {detail}");
            }
        }
    }

    #[tokio::test]
    async fn a_quote_from_another_router_is_refused_before_wallet_or_provider_io() {
        let token = traded_token();
        let routers = build_routers();
        for router in &routers {
            let other = routers
                .iter()
                .find(|other| other.id() != router.id())
                .expect("a second router");
            let quote = quote_for(other.id(), other.name());
            let error = router.execute_swap(&token, &quote).await.unwrap_err();
            let detail = quote_unusable_detail(&error)
                .unwrap_or_else(|| panic!("{}: not a typed refusal: {error}", router.id()));
            assert!(detail.contains(other.id()), "{}: {detail}", router.id());
        }
    }
}
