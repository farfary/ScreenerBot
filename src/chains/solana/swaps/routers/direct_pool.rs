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

use crate::chains::solana::pools::types::ProgramKind;
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
    /// The guaranteed net output the caller accepted this quote for. Execution
    /// refuses to proceed if the fresh quote has fallen below this.
    accepted_min_net_out: u64,
    /// The gross pool floor accepted by the caller.  Reusing a fresh quote's
    /// floor would apply slippage twice and can weaken the accepted trade.
    accepted_min_out: u64,
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

        let (mut fresh_quote, market) = direct::quote(&intent).await?;
        let moved = |fresh: u64| direct::DirectSwapError::MarketMoved {
            pool: intent.pool,
            accepted_min_net_out: data.accepted_min_net_out,
            fresh_expected_net_out: fresh,
        };

        // The ceiling is a property of the CURRENT pool, so it is re-applied to
        // the fresh quote: an accepted quote is not a licence to move the pool
        // by more than the configured limit a block later.
        let max_impact = with_config(|cfg| cfg.swaps.direct.max_price_impact_pct);
        if !fresh_quote.price_impact_pct.is_finite()
            || fresh_quote.price_impact_pct < 0.0
            || fresh_quote.price_impact_pct > max_impact
        {
            logger::warning(
                LogTag::Swap,
                &format!(
                    "Direct pool execution refused: price impact {:.2}% now exceeds the \
                     {max_impact:.2}% ceiling (pool {})",
                    fresh_quote.price_impact_pct, intent.pool
                ),
            );
            return Err(moved(fresh_quote.expected_net_out).into());
        }

        // The caller accepted a GUARANTEED floor, and a fresh quote re-derives
        // its own floor from the new mid price -- taking that one would apply
        // slippage a second time and execute below what was accepted. So the
        // accepted floor is carried into the instruction, and the output-side
        // fee is re-sized from the floor that will actually be enforced.
        if fresh_quote.expected_out < data.accepted_min_out
            || fresh_quote.expected_net_out < data.accepted_min_net_out
        {
            return Err(moved(fresh_quote.expected_net_out).into());
        }

        let natural_min_out = fresh_quote.min_out;
        let enforced_min_out = natural_min_out.max(data.accepted_min_out);
        fresh_quote.min_out = enforced_min_out;
        if fresh_quote.fee.side == direct::FeeSide::Output {
            fresh_quote.fee = direct::PlatformFee::resolve(
                direct::FeeSide::Output,
                &fresh_quote.input_mint,
                &fresh_quote.output_mint,
                enforced_min_out,
            )?;
            fresh_quote.expected_net_out = fresh_quote
                .expected_out
                .saturating_sub(fresh_quote.fee.amount);
            fresh_quote.min_net_out = enforced_min_out.saturating_sub(fresh_quote.fee.amount);
        } else {
            fresh_quote.min_net_out = enforced_min_out;
        }
        if fresh_quote.min_net_out < data.accepted_min_net_out {
            return Err(moved(fresh_quote.min_net_out).into());
        }

        if enforced_min_out > natural_min_out {
            logger::info(
                LogTag::Swap,
                &format!(
                    "Direct pool market moved before execution: holding the accepted gross \
                     floor {enforced_min_out} (a fresh quote would have guaranteed only \
                     {natural_min_out}), net floor {}, expected net out {} (pool {})",
                    fresh_quote.min_net_out, fresh_quote.expected_net_out, intent.pool
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
        // A pool swap is priced forward from the input, and the engine has no
        // ExactOut path. This is a refusal by THIS router, never a verdict on
        // the pair: `RouterRejected` keeps it out of the no-route strikes that
        // retire a token.
        if request.swap_mode != crate::swaps::types::SwapMode::ExactIn {
            return Err(QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: format!(
                    "{:?} is not supported; the direct engine prices ExactIn only",
                    request.swap_mode
                ),
            });
        }

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

        // A caller that excluded a DEX excluded it from EVERY router. The
        // aggregator applies the exclusion while routing; here the pool is
        // already chosen, so the only correct answer is to decline it.
        if let Some(label) = excluded_venue(request.exclude_dexes.as_deref(), market.program()) {
            return Err(QuoteError::NoRoute {
                router: self.name().to_owned(),
                detail: format!("this pool is a {label} pool, which the request excluded"),
            });
        }

        let max_price_impact_pct = with_config(|cfg| cfg.swaps.direct.max_price_impact_pct);
        if !quote.price_impact_pct.is_finite()
            || quote.price_impact_pct < 0.0
            || quote.price_impact_pct > max_price_impact_pct
        {
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
            accepted_min_net_out: quote.min_net_out,
            accepted_min_out: quote.min_out,
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
            minimum_output_amount: quote.min_net_out,
            price_impact_pct: quote.price_impact_pct,
            platform_fee_lamports: quote
                .fee
                .mint
                .filter(direct::intent::is_wsol)
                .map(|_| quote.fee.amount),
            estimated_network_fee_lamports: direct::build_plan(&intent, market.as_ref(), &quote)
                .ok()
                .map(|plan| direct::compute::network_fee_lamports(&plan.instructions)),
            slippage_bps: quote.slippage_bps,
            route_plan: market.program().display_name().to_owned(),
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            exclude_dexes: request.exclude_dexes.clone(),
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
                "Direct pool swap executed on {}: sig={}, in={}, received={}, fee={}, {}ms",
                quote.route_plan,
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
                "Direct pool swap executed for wallet {wallet_id} on {}: sig={}, in={}, received={}, fee={}, {}ms",
                quote.route_plan,
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

/// The exclusion label that rules this pool's program out, if any.
///
/// `exclude_dexes` speaks the aggregator's vocabulary (the labels Jupiter uses
/// in a route step), because that is what the callers already send -- today
/// `"Pump.fun Amm"` on the graduated-token retry. Matching is case-insensitive
/// and covers every program the label names: excluding Pump.fun must exclude the
/// bonding curve as well as the AMM, or the retry lands right back on the venue
/// it was told to avoid.
fn excluded_venue(exclusions: Option<&[String]>, program: ProgramKind) -> Option<&'static str> {
    let labels: &[&str] = match program {
        ProgramKind::PumpFunAmm | ProgramKind::PumpFunLegacy => &["Pump.fun", "Pump.fun Amm"],
        ProgramKind::RaydiumCpmm => &["Raydium CP"],
        ProgramKind::RaydiumLegacyAmm => &["Raydium"],
        ProgramKind::RaydiumClmm => &["Raydium CLMM"],
        ProgramKind::OrcaWhirlpool => &["Whirlpool", "Orca V2"],
        ProgramKind::MeteoraDamm => &["Meteora DAMM v2", "Meteora"],
        ProgramKind::MeteoraDlmm => &["Meteora DLMM"],
        ProgramKind::MeteoraDbc => &["Meteora DBC"],
        ProgramKind::Moonit => &["Moonshot", "Moonit"],
        ProgramKind::FluxbeamAmm => &["FluxBeam"],
        ProgramKind::Unknown => &[],
    };
    let exclusions = exclusions.unwrap_or_default();
    labels.iter().copied().find(|label| {
        exclusions
            .iter()
            .any(|excluded| excluded.trim().eq_ignore_ascii_case(label))
    })
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

    /// This engine only ever looks at ONE pool, so nothing it sees is a verdict
    /// on the mint: a pool it cannot trade is a fact about that pool, and
    /// `NotTradable` -- which retires a token permanently -- must never come
    /// from here. `NoRoute` is the strongest claim the direct path can make.
    #[test]
    fn a_pool_side_failure_is_at_most_a_no_route() {
        let pool = Pubkey::new_unique();
        for error in [
            DirectSwapError::PoolNotTradable {
                pool,
                detail: String::new(),
            },
            DirectSwapError::PairNotInPool {
                pool,
                input_mint: Pubkey::new_unique(),
                output_mint: Pubkey::new_unique(),
            },
            DirectSwapError::InsufficientLiquidity {
                pool,
                amount_in: 1,
                detail: String::new(),
            },
        ] {
            assert!(
                matches!(
                    error.into_quote_error("Direct Pool"),
                    QuoteError::NoRoute { .. }
                ),
                "a single pool's refusal must not retire the mint"
            );
        }
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

    /// The graduated-token retry excludes `"Pump.fun Amm"` so the sell stops
    /// being routed through the venue that just rejected it. That exclusion has
    /// to reach the bonding curve too, and must not quietly rule out an
    /// unrelated venue.
    #[test]
    fn an_excluded_dex_label_covers_its_whole_program_family() {
        let excluded = [String::from("Pump.fun Amm")];
        assert!(excluded_venue(Some(&excluded), ProgramKind::PumpFunAmm).is_some());
        assert!(excluded_venue(Some(&excluded), ProgramKind::PumpFunLegacy).is_some());
        assert!(excluded_venue(Some(&excluded), ProgramKind::RaydiumClmm).is_none());
        assert!(excluded_venue(None, ProgramKind::PumpFunAmm).is_none());

        // Labels come from a provider's vocabulary, so casing and padding are
        // not something a caller should have to get exactly right.
        let padded = [String::from("  pump.fun  ")];
        assert!(excluded_venue(Some(&padded), ProgramKind::PumpFunLegacy).is_some());
    }

    #[test]
    fn the_router_identifies_itself_by_mechanism_not_by_dex() {
        let router = DirectPoolRouter::new();
        assert_eq!(router.id(), "direct");
        assert_eq!(router.chain(), crate::chains::ChainId::Solana);
        assert!(router.priority() > 0, "Jupiter stays the primary router");
    }
}
