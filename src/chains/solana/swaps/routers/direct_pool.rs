//! `crate::swaps::SwapRouter` over the direct pool-swap engine.
//!
//! One router covers EVERY venue the engine supports, rather than one router per
//! DEX. Routing between pools is the engine's job, not the registry's: adding
//! Orca or Meteora must not change how many routers the comparison layer sees.
//!
//! # Pool resolution
//!
//! The pool comes from the live pool-price cache — the same pool whose price the
//! trader made its decision on. That is the point of a direct swap: the price a
//! decision was made on is the price the decision trades at. A resolved pool is
//! still checked against the requested pair before anything is built, because a
//! mint-keyed lookup only proves the pool holds ONE of the two mints.
//!
//! # Failure classification
//!
//! Every failure is classified HERE, where the typed [`DirectSwapError`] is still
//! in hand, into the `QuoteError` variant the routing layer acts on. The rule
//! that matters: only a pool-side verdict may become `NotTradable`/`NoRoute` and
//! count against a mint. Our own RPC or build faults are `Unavailable`, which
//! never does.

use crate::chains::solana::swaps::direct::{self, DirectSwapIntent, DirectSwapOutcome};
use crate::config::with_config;
use crate::errors::DataError;
use crate::logger::{self, LogTag};
use crate::swaps::error::{QuoteError, QuoteResult};
use crate::swaps::router::SwapRouter;
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Instant;

use crate::chains::solana::solana_sdk::{pubkey::Pubkey, signature::Keypair};

/// What a direct quote carries forward to execution. `execute_with_keypair`
/// re-quotes and re-builds against the CURRENT market rather than trusting a
/// cached instruction list, because the market can move between a quote being
/// accepted and execution running -- but it refuses to execute below
/// `accepted_min_net_out`, which is the floor the caller actually agreed to.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirectExecutionData {
    pool: String,
    /// Total input including the platform fee.
    amount_in: u64,
    slippage_bps: u16,
    /// The venue the quote was priced against, for the log line.
    venue: String,
    /// The guaranteed net output the caller accepted this quote for. Execution
    /// refuses to proceed if the fresh quote has fallen below this.
    accepted_min_net_out: u64,
}

/// The direct pool-swap router.
pub struct DirectPoolRouter;

impl DirectPoolRouter {
    /// Create the router.
    pub fn new() -> Self {
        Self
    }

    /// The pool to trade a pair in, from the live pool-price cache.
    ///
    /// Tries the non-reference mint first: a pool is keyed in the cache by the
    /// token it prices, and for a SOL or USDC pair that is the other leg.
    fn resolve_pool(input_mint: &str, output_mint: &str) -> Option<Pubkey> {
        let ordered = if is_reference_mint(input_mint) {
            [output_mint, input_mint]
        } else {
            [input_mint, output_mint]
        };
        for mint in ordered {
            if let Some(price) = crate::pools::get_pool_price(mint) {
                if let Ok(pool) = Pubkey::from_str(&price.pool_address) {
                    return Some(pool);
                }
            }
        }
        None
    }

    /// Build the intent a request describes.
    fn intent_for(request: &QuoteRequest, pool: Pubkey) -> QuoteResult<DirectSwapIntent> {
        let owner =
            Pubkey::from_str(&request.wallet_address).map_err(|e| QuoteError::RouterRejected {
                router: "Direct Pool".to_owned(),
                detail: format!("wallet address is not a pubkey: {e}"),
            })?;
        let input_mint = parse_mint(&request.input_mint)?;
        let output_mint = parse_mint(&request.output_mint)?;

        Ok(DirectSwapIntent {
            pool,
            owner,
            input_mint,
            output_mint,
            amount_in: request.input_amount,
            slippage_bps: slippage_bps_for(request.slippage_pct),
        })
    }

    /// Execute an accepted quote with a specific signer.
    ///
    /// Quotes and builds explicitly (`direct::quote` -> `direct::build_plan` ->
    /// `direct::execute_plan`) rather than calling the opaque `direct::swap`, so
    /// the FRESH quote is in hand and can be checked against what the caller
    /// actually accepted before anything is built or sent. The comparison layer
    /// may have chosen this router over Jupiter on a number that no longer
    /// exists by the time execution runs; refusing below the accepted floor is
    /// what keeps that choice honest.
    async fn execute_with_keypair(
        &self,
        quote: &Quote,
        keypair: &Keypair,
    ) -> Result<DirectSwapOutcome> {
        use crate::chains::solana::solana_sdk::signature::Signer;

        let data: DirectExecutionData =
            serde_json::from_slice(&quote.execution_data).map_err(|e| {
                Error::Data(DataError::ParseError {
                    data_type: "direct pool execution data".to_owned(),
                    error: e.to_string(),
                })
            })?;

        let intent = DirectSwapIntent {
            pool: Pubkey::from_str(&data.pool).map_err(|e| {
                Error::Data(DataError::ParseError {
                    data_type: format!("direct pool execution data pool ({})", data.pool),
                    error: e.to_string(),
                })
            })?,
            owner: keypair.pubkey(),
            input_mint: Pubkey::from_str(&quote.input_mint).map_err(|e| {
                Error::Data(DataError::ParseError {
                    data_type: format!("quote input mint ({})", quote.input_mint),
                    error: e.to_string(),
                })
            })?,
            output_mint: Pubkey::from_str(&quote.output_mint).map_err(|e| {
                Error::Data(DataError::ParseError {
                    data_type: format!("quote output mint ({})", quote.output_mint),
                    error: e.to_string(),
                })
            })?,
            amount_in: data.amount_in,
            slippage_bps: data.slippage_bps,
        };

        let (fresh_quote, market) = direct::quote(&intent).await?;

        if fresh_quote.expected_net_out < data.accepted_min_net_out {
            return Err(direct::DirectSwapError::MarketMoved {
                pool: intent.pool,
                accepted_min_net_out: data.accepted_min_net_out,
                fresh_expected_net_out: fresh_quote.expected_net_out,
            }
            .into());
        }

        if fresh_quote.expected_net_out != data.accepted_min_net_out {
            logger::info(
                LogTag::Swap,
                &format!(
                    "Direct pool market moved before execution: accepted floor {}, fresh \
                     expected net out {} (pool {})",
                    data.accepted_min_net_out, fresh_quote.expected_net_out, intent.pool
                ),
            );
        }

        let plan = direct::build_plan(&intent, market.as_ref(), &fresh_quote)?;
        Ok(direct::execute_plan(&plan, keypair).await?)
    }
}

impl Default for DirectPoolRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SwapRouter for DirectPoolRouter {
    fn id(&self) -> &'static str {
        "direct"
    }

    fn name(&self) -> &'static str {
        "Direct Pool"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.direct.enabled)
    }

    fn priority(&self) -> u8 {
        1 // Behind Jupiter, which can route through pools we have no venue for.
    }

    fn chain(&self) -> crate::chains::ChainId {
        crate::chains::ChainId::Solana
    }

    async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
        self.accept_own_chain(request)
            .map_err(|e| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: e.to_string(),
            })?;

        let pool =
            Self::resolve_pool(&request.input_mint, &request.output_mint).ok_or_else(|| {
                QuoteError::NoRoute {
                    router: self.name().to_owned(),
                    detail: "no live pool is known for either side of the pair".to_owned(),
                }
            })?;

        let intent = Self::intent_for(request, pool)?;
        let (quote, market) = direct::quote(&intent)
            .await
            .map_err(|e| e.into_quote_error(self.name()))?;

        let max_price_impact_pct = with_config(|cfg| cfg.swaps.direct.max_price_impact_pct);
        if quote.price_impact_pct > max_price_impact_pct {
            return Err(QuoteError::NoRoute {
                router: self.name().to_owned(),
                detail: format!(
                    "price impact {:.2}% exceeds the {max_price_impact_pct:.2}% ceiling -- an \
                     aggregator that can split the order is the safer choice at this size",
                    quote.price_impact_pct
                ),
            });
        }

        let execution_data = serde_json::to_vec(&DirectExecutionData {
            pool: pool.to_string(),
            amount_in: intent.amount_in,
            slippage_bps: intent.slippage_bps,
            venue: format!("{:?}", market.program()),
            accepted_min_net_out: quote.min_net_out,
        })
        .map_err(|e| QuoteError::RouterRejected {
            router: self.name().to_owned(),
            detail: format!("quote could not be serialised: {e}"),
        })?;

        Ok(Quote {
            chain: request.chain,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: quote.amount_in,
            // What the WALLET keeps. Reporting the pool's gross output here would
            // overstate every sell by the platform fee and make the comparison
            // against an aggregator quote dishonest.
            output_amount: quote.expected_net_out,
            price_impact_pct: quote.price_impact_pct,
            fee_lamports: 0,
            slippage_bps: quote.slippage_bps,
            route_plan: format!("{:?}", market.program()),
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            execution_data,
        })
    }

    async fn execute_swap(&self, _token: &Token, quote: &Quote) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        let start = Instant::now();
        let keypair = crate::chains::solana::accounts::configured_keypair()?;
        let outcome = self.execute_with_keypair(quote, &keypair).await?;

        logger::info(
            LogTag::Swap,
            &format!(
                "Direct pool swap executed: sig={}, in={}, received={}, fee={}, {}ms",
                outcome.signature,
                outcome.amount_in,
                outcome.receipt.received,
                outcome.platform_fee,
                outcome.duration_ms
            ),
        );

        Ok(SwapResult {
            success: true,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            fee_lamports: outcome.platform_fee_lamports(),
            transaction_signature: outcome.signature,
            input_amount: outcome.amount_in,
            output_amount: outcome.receipt.received,
            price_impact_pct: quote.price_impact_pct,
            execution_time_ms: start.elapsed().as_millis() as u64,
            effective_price_sol: None,
        })
    }

    async fn execute_swap_for_wallet(&self, quote: &Quote, wallet_id: i64) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        let start = Instant::now();
        let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
        let outcome = self.execute_with_keypair(quote, &keypair).await?;

        logger::info(
            LogTag::Swap,
            &format!(
                "Direct pool swap executed for wallet {wallet_id}: sig={}, in={}, received={}, fee={}, {}ms",
                outcome.signature,
                outcome.amount_in,
                outcome.receipt.received,
                outcome.platform_fee,
                outcome.duration_ms
            ),
        );

        Ok(SwapResult {
            success: true,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            fee_lamports: outcome.platform_fee_lamports(),
            transaction_signature: outcome.signature,
            input_amount: outcome.amount_in,
            output_amount: outcome.receipt.received,
            price_impact_pct: quote.price_impact_pct,
            execution_time_ms: start.elapsed().as_millis() as u64,
            effective_price_sol: None,
        })
    }
}

fn is_reference_mint(mint: &str) -> bool {
    mint == crate::chains::solana::constants::SOL_MINT
        || mint == crate::chains::solana::constants::USDC_MINT
}

fn parse_mint(mint: &str) -> QuoteResult<Pubkey> {
    Pubkey::from_str(mint).map_err(|e| QuoteError::RouterRejected {
        router: "Direct Pool".to_owned(),
        detail: format!("mint {mint} is not a pubkey: {e}"),
    })
}

/// Percentage slippage to basis points, floored at one bp so a rounding error
/// can never produce an unprotected zero, and ceilinged at the engine's own
/// [`MAX_SLIPPAGE_BPS`] rather than `u16::MAX`. Clamping to the raw integer
/// range let a high slippage SETTING silently disable this router entirely: a
/// value like 1_000% clamped to 65_535 bps, which `DirectSwapIntent::validate`
/// then rejects outright as `RouterRejected` -- a confusing way to find out a
/// slippage preference and a router are incompatible.
fn slippage_bps_for(slippage_pct: f64) -> u16 {
    ((slippage_pct * 100.0).round() as i64).clamp(1, direct::intent::MAX_SLIPPAGE_BPS as i64) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::swaps::direct::DirectSwapError;

    #[test]
    fn slippage_converts_percent_to_basis_points_and_never_reaches_zero() {
        assert_eq!(slippage_bps_for(1.0), 100);
        assert_eq!(slippage_bps_for(0.5), 50);
        assert_eq!(
            slippage_bps_for(0.0),
            1,
            "an unprotected swap is never built"
        );
        assert_eq!(slippage_bps_for(-5.0), 1);
    }

    #[test]
    fn an_extreme_slippage_setting_clamps_to_the_engines_own_ceiling_not_u16_max() {
        // Clamping to u16::MAX (655%) used to pass validation straight into
        // `DirectSwapIntent::validate`'s 50% ceiling, which then rejected the
        // swap as a confusing `RouterRejected` rather than a slippage clamp.
        assert_eq!(
            slippage_bps_for(1_000_000.0),
            crate::chains::solana::swaps::direct::intent::MAX_SLIPPAGE_BPS
        );
        assert!(
            DirectSwapIntent {
                pool: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                input_mint: Pubkey::new_unique(),
                output_mint: Pubkey::new_unique(),
                amount_in: 1,
                slippage_bps: slippage_bps_for(1_000_000.0),
            }
            .validate()
            .is_ok(),
            "a clamped value must still pass the intent's own validation"
        );
    }

    #[test]
    fn only_pool_side_failures_become_a_verdict_on_the_token() {
        let pool = Pubkey::new_unique();
        assert!(matches!(
            DirectSwapError::PoolNotTradable {
                pool,
                detail: String::new()
            }
            .into_quote_error("Direct Pool"),
            QuoteError::NotTradable { .. }
        ));
        assert!(matches!(
            DirectSwapError::InsufficientLiquidity {
                pool,
                amount_in: 1,
                detail: String::new()
            }
            .into_quote_error("Direct Pool"),
            QuoteError::NoRoute { .. }
        ));
    }

    #[test]
    fn an_rpc_or_build_fault_never_counts_against_the_token() {
        for error in [
            DirectSwapError::AccountUnavailable {
                address: Pubkey::new_unique(),
                detail: String::new(),
            },
            DirectSwapError::Build {
                detail: String::new(),
            },
            DirectSwapError::UnsupportedVenue {
                program: Pubkey::new_unique(),
            },
            DirectSwapError::SubmitFailed {
                detail: String::new(),
            },
        ] {
            assert!(
                matches!(
                    error.into_quote_error("Direct Pool"),
                    QuoteError::Unavailable { .. }
                ),
                "our own faults must stay router-level"
            );
        }
    }

    #[test]
    fn a_malformed_request_is_our_refusal_not_a_provider_verdict() {
        assert!(matches!(
            DirectSwapError::InvalidRequest {
                detail: String::new()
            }
            .into_quote_error("Direct Pool"),
            QuoteError::RouterRejected { .. }
        ));
    }

    #[test]
    fn the_router_identifies_itself_by_mechanism_not_by_dex() {
        let router = DirectPoolRouter::new();
        assert_eq!(router.id(), "direct");
        assert_eq!(router.chain(), crate::chains::ChainId::Solana);
        assert!(router.priority() > 0, "Jupiter stays the primary router");
    }
}
