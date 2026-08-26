//! Wallet-scoped swap orchestration.
//!
//! The router that produced a quote owns execution of that quote. Selection
//! uses the registry primary-router policy (lowest `priority()` among enabled
//! routers), never enabled-vector position.

use crate::swaps::registry::{get_registry, RouterRegistry};
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::{Error, Result};

/// Quote through the registry primary router and execute that same router
/// instance's quote for `wallet_id`. Missing registry initialization becomes
/// a structured service-init error; a router that cannot execute for a
/// wallet returns [`Error::unsupported_capability`] without submitting.
pub async fn quote_and_execute_for_wallet(
    request: QuoteRequest,
    wallet_id: i64,
) -> Result<(Quote, SwapResult)> {
    let registry = get_registry()?;
    quote_and_execute_for_wallet_on(registry, request, wallet_id).await
}

/// Same as [`quote_and_execute_for_wallet`], against an explicit registry.
/// Used by tests with stub routers; production goes through the global
/// accessor so boot still owns factory registration.
pub(crate) async fn quote_and_execute_for_wallet_on(
    registry: &RouterRegistry,
    request: QuoteRequest,
    wallet_id: i64,
) -> Result<(Quote, SwapResult)> {
    let router = registry
        .get_primary_router_for(request.chain)
        .ok_or_else(|| Error::configuration_error("No swap routers enabled in config"))?;
    router.accept_own_chain(&request)?;
    let quote = router.get_quote(&request).await.map_err(Error::from)?;
    let result = router.execute_swap_for_wallet(&quote, wallet_id).await?;
    Ok((quote, result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::InternalError;
    use crate::swaps::router::SwapRouter;
    use crate::swaps::types::SwapMode;
    use crate::tokens::Token;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct CallLog {
        quotes: Mutex<Vec<&'static str>>,
        wallet_execs: Mutex<Vec<(&'static str, String, Vec<u8>, i64)>>,
    }

    impl CallLog {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                quotes: Mutex::new(Vec::new()),
                wallet_execs: Mutex::new(Vec::new()),
            })
        }
    }

    struct StubRouter {
        id: &'static str,
        enabled: bool,
        priority: u8,
        supports_wallet: bool,
        log: Arc<CallLog>,
    }

    fn request() -> QuoteRequest {
        QuoteRequest {
            chain: crate::chains::active_chain(),
            input_mint: "So11111111111111111111111111111111111111112".to_owned(),
            output_mint: "TokenMint111111111111111111111111111111111".to_owned(),
            input_amount: 1_000_000,
            wallet_address: "Wallet1111111111111111111111111111111111111".to_owned(),
            slippage_pct: 1.0,
            swap_mode: SwapMode::ExactIn,
            exclude_dexes: None,
        }
    }

    #[async_trait]
    impl SwapRouter for StubRouter {
        fn id(&self) -> &'static str {
            self.id
        }
        fn name(&self) -> &'static str {
            self.id
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn priority(&self) -> u8 {
            self.priority
        }
        fn chain(&self) -> crate::chains::ChainId {
            crate::chains::ChainId::Solana
        }

        async fn get_quote(&self, request: &QuoteRequest) -> crate::swaps::QuoteResult<Quote> {
            self.log.quotes.lock().expect("quotes").push(self.id);
            Ok(Quote {
                chain: request.chain,
                router_id: self.id.to_owned(),
                router_name: self.id.to_owned(),
                input_mint: request.input_mint.clone(),
                output_mint: request.output_mint.clone(),
                input_amount: request.input_amount,
                output_amount: 42,
                price_impact_pct: 0.1,
                fee_lamports: 0,
                slippage_bps: 100,
                route_plan: self.id.to_owned(),
                swap_mode: request.swap_mode,
                wallet_address: request.wallet_address.clone(),
                execution_data: self.id.as_bytes().to_vec(),
            })
        }

        async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> Result<SwapResult> {
            Err(Error::api_error("ordinary execute_swap is not used"))
        }

        async fn execute_swap_for_wallet(
            &self,
            quote: &Quote,
            wallet_id: i64,
        ) -> Result<SwapResult> {
            self.accept_own_quote(quote)?;
            self.log.wallet_execs.lock().expect("execs").push((
                self.id,
                quote.router_id.clone(),
                quote.execution_data.clone(),
                wallet_id,
            ));
            if !self.supports_wallet {
                return Err(Error::unsupported_capability(
                    "wallet_scoped_execution",
                    self.id(),
                ));
            }
            Ok(SwapResult {
                success: true,
                router_id: self.id.to_owned(),
                router_name: self.id.to_owned(),
                transaction_signature: format!("sig-{}", self.id),
                input_amount: quote.input_amount,
                output_amount: quote.output_amount,
                price_impact_pct: quote.price_impact_pct,
                fee_lamports: quote.fee_lamports,
                execution_time_ms: 1,
                effective_price_sol: None,
            })
        }
    }

    fn registry(routers: Vec<StubRouter>) -> RouterRegistry {
        RouterRegistry::new(
            routers
                .into_iter()
                .map(|r| Arc::new(r) as Arc<dyn SwapRouter>)
                .collect(),
        )
    }

    #[tokio::test]
    async fn disabled_first_registered_router_does_not_execute_a_later_quote() {
        let log = CallLog::new();
        let registry = registry(vec![
            StubRouter {
                id: "jupiter",
                enabled: false,
                priority: 0,
                supports_wallet: true,
                log: Arc::clone(&log),
            },
            StubRouter {
                id: "gmgn",
                enabled: true,
                priority: 1,
                supports_wallet: true,
                log: Arc::clone(&log),
            },
        ]);

        let (quote, result) = quote_and_execute_for_wallet_on(&registry, request(), 7)
            .await
            .expect("gmgn should quote and execute");

        assert_eq!(quote.router_id, "gmgn");
        assert_eq!(result.router_id, "gmgn");
        assert_eq!(result.transaction_signature, "sig-gmgn");
        assert_eq!(*log.quotes.lock().expect("quotes"), vec!["gmgn"]);
        let execs = log.wallet_execs.lock().expect("execs");
        assert_eq!(execs.len(), 1);
        assert_eq!(execs[0].0, "gmgn");
        assert_eq!(execs[0].1, "gmgn");
        assert_eq!(execs[0].2, b"gmgn");
        assert_eq!(execs[0].3, 7);
    }

    #[tokio::test]
    async fn a_non_jupiter_quote_is_never_passed_to_jupiter_execution() {
        let log = CallLog::new();
        let registry = registry(vec![
            StubRouter {
                id: "jupiter",
                enabled: false,
                priority: 0,
                supports_wallet: true,
                log: Arc::clone(&log),
            },
            StubRouter {
                id: "gmgn",
                enabled: true,
                priority: 1,
                supports_wallet: true,
                log: Arc::clone(&log),
            },
        ]);

        quote_and_execute_for_wallet_on(&registry, request(), 3)
            .await
            .expect("gmgn path");

        let execs = log.wallet_execs.lock().expect("execs");
        assert!(
            execs.iter().all(|e| e.0 != "jupiter"),
            "Jupiter wallet execution must not run for a GMGN quote: {execs:?}"
        );
        assert_eq!(execs[0].2, b"gmgn");
    }

    #[tokio::test]
    async fn primary_priority_wins_over_registration_order() {
        let log = CallLog::new();
        let registry = registry(vec![
            StubRouter {
                id: "gmgn",
                enabled: true,
                priority: 1,
                supports_wallet: true,
                log: Arc::clone(&log),
            },
            StubRouter {
                id: "jupiter",
                enabled: true,
                priority: 0,
                supports_wallet: true,
                log: Arc::clone(&log),
            },
        ]);

        let (quote, result) = quote_and_execute_for_wallet_on(&registry, request(), 1)
            .await
            .expect("jupiter is primary");

        assert_eq!(quote.router_id, "jupiter");
        assert_eq!(result.router_id, "jupiter");
        assert_eq!(*log.quotes.lock().expect("quotes"), vec!["jupiter"]);
        assert_eq!(log.wallet_execs.lock().expect("execs")[0].0, "jupiter");
    }

    #[tokio::test]
    async fn unsupported_wallet_execution_is_a_structured_error() {
        let log = CallLog::new();
        let registry = registry(vec![StubRouter {
            id: "raydium",
            enabled: true,
            priority: 2,
            supports_wallet: false,
            log: Arc::clone(&log),
        }]);

        let err = quote_and_execute_for_wallet_on(&registry, request(), 9)
            .await
            .expect_err("raydium cannot execute for a wallet");

        match err {
            Error::Internal(InternalError::UnsupportedCapability { capability, owner }) => {
                assert_eq!(capability, "wallet_scoped_execution");
                assert_eq!(owner, "raydium");
            }
            other => panic!("expected UnsupportedCapability, got {other}"),
        }
        // Quote happened; execution was refused after recording the attempt,
        // and nothing was submitted as a success.
        assert_eq!(*log.quotes.lock().expect("quotes"), vec!["raydium"]);
    }

    #[tokio::test]
    async fn default_trait_wallet_execution_is_unsupported() {
        struct DefaultRouter;
        #[async_trait]
        impl SwapRouter for DefaultRouter {
            fn id(&self) -> &'static str {
                "raydium"
            }
            fn name(&self) -> &'static str {
                "Raydium"
            }
            fn is_enabled(&self) -> bool {
                true
            }
            fn priority(&self) -> u8 {
                2
            }
            fn chain(&self) -> crate::chains::ChainId {
                crate::chains::ChainId::Solana
            }
            async fn get_quote(&self, request: &QuoteRequest) -> crate::swaps::QuoteResult<Quote> {
                Ok(Quote {
                    chain: request.chain,
                    router_id: self.id().to_owned(),
                    router_name: self.name().to_owned(),
                    input_mint: request.input_mint.clone(),
                    output_mint: request.output_mint.clone(),
                    input_amount: request.input_amount,
                    output_amount: 1,
                    price_impact_pct: 0.0,
                    fee_lamports: 0,
                    slippage_bps: 100,
                    route_plan: "none".to_owned(),
                    swap_mode: request.swap_mode,
                    wallet_address: request.wallet_address.clone(),
                    execution_data: b"raydium".to_vec(),
                })
            }
            async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> Result<SwapResult> {
                Err(Error::internal_error("not implemented"))
            }
        }

        let registry = RouterRegistry::new(vec![Arc::new(DefaultRouter)]);
        let err = quote_and_execute_for_wallet_on(&registry, request(), 1)
            .await
            .expect_err("default wallet execution");
        match err {
            Error::Internal(InternalError::UnsupportedCapability { owner, .. }) => {
                assert_eq!(owner, "raydium");
            }
            other => panic!("expected UnsupportedCapability, got {other}"),
        }
    }
}
