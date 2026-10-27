// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! `crate::swaps::SwapRouter` over the direct pool-swap engine.
//!
//! One router covers EVERY venue the engine supports, rather than one router per
//! DEX. Routing between pools is the engine's job, not the registry's: adding
//! Orca or Meteora must not change how many routers the comparison layer sees.
//!
//! # Pool resolution
//!
//! The live pool-price cache comes first — the same pool whose price the trader
//! made its decision on, so the price a decision was made on is the price the
//! decision trades at. When the token has no live price, the pools the pool
//! service discovered for the pair are tried instead, canonical first and then
//! by liquidity, up to [`MAX_CANDIDATE_POOLS`]; every candidate is quoted and the
//! one that leaves the wallet the most wins, the earlier candidate on a tie.
//! Every candidate is still checked against the requested pair before anything
//! is built. A token neither source knows -- one launched seconds ago, which no
//! aggregator routes yet either -- is tried at its pump.fun bonding curve, whose
//! address derives from the mint.
//!
//! # Failure classification
//!
//! Every failure is classified HERE, where the typed [`DirectSwapError`] is still
//! in hand, into the `QuoteError` variant the routing layer acts on. The rule
//! that matters: only a pool-side verdict may become `NotTradable`/`NoRoute` and
//! count against a mint. Our own RPC or build faults are `Unavailable`, which
//! never does.

use crate::chains::solana::swaps::direct::{self, DirectSwapIntent, DirectSwapOutcome};
use crate::chains::solana::swaps::revenue::fee_reference_for_mint;
use crate::chains::{adapter_for, ChainId};
use crate::config::with_config;
use crate::errors::DataError;
use crate::logger::{self, LogTag};
use crate::swaps::error::{NotOfferedReason, QuoteError, QuoteResult};
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

/// Most pools one direct quote tries. Each costs an account batch, and a pair
/// whose best few pools cannot be traded is better left to an aggregator.
const MAX_CANDIDATE_POOLS: usize = 3;

/// The fresh-quote refusal: the market moved below what the caller accepted.
fn market_moved(
    fresh_quote: &direct::DirectQuote,
    data: &DirectExecutionData,
    fresh_expected_net_out: u64,
) -> direct::DirectSwapError {
    direct::DirectSwapError::MarketMoved {
        pool: fresh_quote.pool,
        accepted_min_net_out: data.accepted_min_net_out,
        fresh_expected_net_out,
    }
}

/// Raw units of slack when comparing a re-sized net floor to the accepted one.
/// The output fee is `floor(expected_out * rate)`, so an `expected_out` that
/// differs by a hair can move the fee by exactly one raw unit while `min_out`
/// stays flat. That single unit of rounding is not a market move.
const FEE_ROUNDING_TOLERANCE: u64 = 1;

/// Hold the floor the caller accepted against a fresh quote.
///
/// A fresh quote re-derives its own floor from the new mid price -- taking that
/// one would apply slippage a second time and execute below what was accepted.
/// So the accepted gross floor is carried into the instruction, and the
/// output-side fee is re-sized from the fresh expected output, capped at the
/// floor that will actually be enforced. Refuses with `MarketMoved` when the
/// fresh quote cannot honour the accepted floor.
fn hold_accepted_floor(
    fresh_quote: &mut direct::DirectQuote,
    data: &DirectExecutionData,
) -> direct::DirectSwapResult<()> {
    if fresh_quote.expected_out < data.accepted_min_out
        || fresh_quote.expected_net_out < data.accepted_min_net_out
    {
        return Err(market_moved(
            fresh_quote,
            data,
            fresh_quote.expected_net_out,
        ));
    }

    let enforced_min_out = fresh_quote.min_out.max(data.accepted_min_out);
    fresh_quote.min_out = enforced_min_out;
    if fresh_quote.fee.side == direct::FeeSide::Output {
        let mut fee = direct::PlatformFee::resolve(
            direct::FeeSide::Output,
            &fresh_quote.input_mint,
            &fresh_quote.output_mint,
            fresh_quote.expected_out,
        )?;
        fee.amount = fee.amount.min(enforced_min_out);
        fresh_quote.fee = fee;
        fresh_quote.expected_net_out = fresh_quote
            .expected_out
            .saturating_sub(fresh_quote.fee.amount);
        fresh_quote.min_net_out = enforced_min_out.saturating_sub(fresh_quote.fee.amount);
    } else {
        fresh_quote.min_net_out = enforced_min_out;
    }
    if fresh_quote
        .min_net_out
        .saturating_add(FEE_ROUNDING_TOLERANCE)
        < data.accepted_min_net_out
    {
        return Err(market_moved(fresh_quote, data, fresh_quote.min_net_out));
    }
    Ok(())
}

/// The direct pool-swap router.
pub struct DirectPoolRouter;

impl DirectPoolRouter {
    /// Create the router.
    pub fn new() -> Self {
        Self
    }

    /// The pools to try for a pair, best first, at most [`MAX_CANDIDATE_POOLS`].
    ///
    /// The live pool-price cache comes first: it is the pool the trader's
    /// decision was priced on. A token with no live pool price yet -- a first
    /// buy, or a holding priced from elsewhere -- falls back to the pools the
    /// pool service has discovered for it, canonical first and then by
    /// liquidity, so a direct swap is not limited to the tokens that happen to
    /// be priced right now. The pools are keyed by [`pair_token`], the same leg
    /// whichever way the pair is traded. [`best_quote`] picks among them in
    /// this order, so an earlier candidate wins a tie.
    fn candidate_pools(input_mint: &str, output_mint: &str) -> Vec<Pubkey> {
        let token = pair_token(input_mint, output_mint);
        let other = if token == input_mint {
            output_mint
        } else {
            input_mint
        };

        let mut candidates: Vec<Pubkey> = Vec::new();
        let mut push = |address: &str| {
            if let Ok(pool) = Pubkey::from_str(address) {
                if !candidates.contains(&pool) {
                    candidates.push(pool);
                }
            }
        };

        if let Some(price) = crate::pools::get_pool_price(ChainId::Solana, token) {
            push(&price.pool_address);
        }

        let mut discovered = crate::chains::solana::pools::service::get_token_pools(token);
        discovered.retain(|pool| {
            let (base, quote) = (pool.base_mint.address(), pool.quote_mint.address());
            (base == token && quote == other) || (base == other && quote == token)
        });
        // `get_token_pools` already puts the canonical pool first.
        if discovered.len() > 1 {
            discovered[1..].sort_by(|a, b| b.liquidity_usd.total_cmp(&a.liquidity_usd));
        }
        for pool in &discovered {
            push(pool.pool_id.address());
        }

        candidates.truncate(MAX_CANDIDATE_POOLS);
        candidates
    }

    /// Quote one named pool, with every refusal a single pool can produce. The
    /// router's own quotes run through here for each candidate pool; a caller
    /// that already knows the pool reaches the same quote without the pool
    /// service.
    pub async fn quote_pool(&self, request: &QuoteRequest, pool: Pubkey) -> QuoteResult<Quote> {
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

        let max_price_impact_pct =
            with_config(|cfg| cfg.chains.solana.swaps.direct.max_price_impact_pct);
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
            input_amount: quote.amount_in.into(),
            // What the WALLET keeps. Reporting the pool's gross output here would
            // overstate every sell by the platform fee and make the comparison
            // against an aggregator quote dishonest.
            output_amount: quote.expected_net_out.into(),
            minimum_output_amount: quote.min_net_out.into(),
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

    /// Build the intent a request describes.
    fn intent_for(request: &QuoteRequest, pool: Pubkey) -> QuoteResult<DirectSwapIntent> {
        let input_amount =
            u64::try_from(request.input_amount).map_err(|_| QuoteError::RouterRejected {
                router: "Direct Pool".to_owned(),
                detail: "input amount exceeds the Solana u64 limit".to_owned(),
            })?;
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
            amount_in: input_amount,
            slippage_bps: slippage_bps_for(request.slippage_pct),
        })
    }

    /// Execute an accepted quote with a specific signer: the plan
    /// [`Self::fresh_plan`] builds for the signer's wallet, executed.
    async fn execute_with_keypair(
        &self,
        quote: &Quote,
        keypair: &Keypair,
    ) -> Result<DirectSwapOutcome> {
        use crate::chains::solana::solana_sdk::signature::Signer;

        let plan = self.fresh_plan(quote, keypair.pubkey()).await?;
        Ok(direct::execute_plan(&plan, keypair).await?)
    }

    /// The plan an accepted quote executes as, for `owner`'s wallet.
    ///
    /// Quotes and builds explicitly (`direct::quote` -> `direct::build_plan`)
    /// rather than calling the opaque `direct::swap`, so the FRESH quote is in
    /// hand and can be checked against what the caller actually accepted before
    /// anything is built or sent. The comparison layer may have chosen this
    /// router over Jupiter on a number that no longer exists by the time
    /// execution runs; refusing below the accepted floor is what keeps that
    /// choice honest.
    async fn fresh_plan(&self, quote: &Quote, owner: Pubkey) -> Result<direct::SwapPlan> {
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
            owner,
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

        // The ceiling is a property of the CURRENT pool, so it is re-applied to
        // the fresh quote: an accepted quote is not a licence to move the pool
        // by more than the configured limit a block later.
        let max_impact = with_config(|cfg| cfg.chains.solana.swaps.direct.max_price_impact_pct);
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
            return Err(market_moved(&fresh_quote, &data, fresh_quote.expected_net_out).into());
        }

        let natural_min_out = fresh_quote.min_out;
        hold_accepted_floor(&mut fresh_quote, &data)?;
        let enforced_min_out = fresh_quote.min_out;

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

        Ok(direct::build_plan(&intent, market.as_ref(), &fresh_quote)?)
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
        with_config(|cfg| cfg.chains.solana.swaps.direct.enabled)
    }

    fn priority(&self) -> u8 {
        1 // Behind Jupiter, which can route through pools we have no venue for.
    }

    fn chain(&self) -> crate::chains::ChainId {
        ChainId::Solana
    }

    async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
        u64::try_from(request.input_amount).map_err(|_| QuoteError::RouterRejected {
            router: self.name().to_owned(),
            detail: "input amount exceeds the Solana u64 limit".to_owned(),
        })?;
        self.accept_own_chain(request)
            .map_err(|e| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: e.to_string(),
            })?;
        // A pool swap is priced forward from the input, and the engine has no
        // ExactOut path. This router abstains, which is never a verdict on the
        // pair and never suppresses the other routers' verdict.
        if request.swap_mode != crate::swaps::types::SwapMode::ExactIn {
            return Err(QuoteError::NotOffered {
                router: self.name().to_owned(),
                reason: NotOfferedReason::ExactOut,
            });
        }

        let candidates = Self::candidate_pools(&request.input_mint, &request.output_mint);
        let no_pool = QuoteError::NoRoute {
            router: self.name().to_owned(),
            detail: "the pool service knows no pool for this pair yet".to_owned(),
        };
        if candidates.is_empty() {
            // Most tokens are bought first on their launch curve, before any
            // discovery source has indexed them. Any refusal from the derived
            // curve (no such account, a migrated curve) keeps the no-pool answer.
            let Some(curve) = launch_curve_for(&request.input_mint, &request.output_mint) else {
                return Err(no_pool);
            };
            return self.quote_pool(request, curve).await.map_err(|error| {
                logger::debug(
                    LogTag::Swap,
                    &format!("Launch curve {curve} could not quote this trade: {error}"),
                );
                no_pool
            });
        }
        let results = futures::future::join_all(
            candidates
                .iter()
                .map(|pool| self.quote_pool(request, *pool)),
        )
        .await;
        for (pool, result) in candidates.iter().zip(&results) {
            if let Err(error) = result {
                logger::debug(
                    LogTag::Swap,
                    &format!("Direct pool {pool} could not quote this trade: {error}"),
                );
            }
        }
        best_quote(results, no_pool)
    }

    async fn execute_swap(&self, _token: &Token, quote: &Quote) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        super::checked_quote_amounts(quote)?;
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
            input_amount: outcome.amount_in.into(),
            output_amount: outcome.receipt.received.into(),
            price_impact_pct: quote.price_impact_pct,
            execution_time_ms: start.elapsed().as_millis() as u64,
            effective_price_sol: None,
        })
    }

    async fn build_for_simulation(&self, quote: &Quote, owner: &str) -> Result<Vec<u8>> {
        self.accept_own_quote(quote)?;
        super::checked_quote_amounts(quote)?;
        let owner = Pubkey::from_str(owner).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: format!("direct pool simulation owner ({owner})"),
                error: e.to_string(),
            })
        })?;
        let plan = self.fresh_plan(quote, owner).await?;
        let transaction = direct::unsigned_transaction(&plan, &owner).await?;
        bincode::serialize(&transaction).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "direct pool simulation transaction".to_owned(),
                error: e.to_string(),
            })
        })
    }
}

/// The leg a pair's pools are keyed by, whichever way the pair is traded.
///
/// Each leg is ranked by how much it is a quote asset: the native asset first,
/// then a fee reference, then any other quote asset of the chain, then a traded
/// token. The lower-ranked leg is the one whose pools list the pair; SOL<->USDC
/// keys USDC, SOL<->USDT keys USDT, USDC<->USDT keys USDT. Equal ranks are
/// broken by address order, so the key never depends on the direction.
fn pair_token<'a>(input_mint: &'a str, output_mint: &'a str) -> &'a str {
    let adapter = adapter_for(ChainId::Solana);
    let rank = |mint: &str| -> u8 {
        if adapter.is_native_asset(mint) {
            3
        } else if fee_reference_for_mint(mint).is_some() {
            2
        } else if adapter.is_quote_asset(mint) {
            1
        } else {
            0
        }
    };
    match rank(input_mint).cmp(&rank(output_mint)) {
        std::cmp::Ordering::Less => input_mint,
        std::cmp::Ordering::Greater => output_mint,
        std::cmp::Ordering::Equal => input_mint.min(output_mint),
    }
}

/// The pump.fun bonding curve of the token in a pair with the native asset,
/// derived from its mint. A quote asset has no launch curve.
fn launch_curve_for(input_mint: &str, output_mint: &str) -> Option<Pubkey> {
    let adapter = adapter_for(ChainId::Solana);
    let token = if adapter.is_native_asset(input_mint) {
        output_mint
    } else if adapter.is_native_asset(output_mint) {
        input_mint
    } else {
        return None;
    };
    if adapter.is_quote_asset(token) {
        return None;
    }
    let mint = Pubkey::from_str(token).ok()?;
    Some(direct::venues::pumpfun_legacy::bonding_curve_address(&mint))
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

/// Choose among the candidate pools' quotes, given in candidate order.
///
/// The `Ok` quote with the largest `output_amount` wins, because that is what
/// the wallet keeps; a tie goes to the earlier candidate, which is the pricing
/// pool. With no `Ok` the last error in candidate order is returned, and
/// `no_candidates` when there was nothing to quote.
fn best_quote(results: Vec<QuoteResult<Quote>>, no_candidates: QuoteError) -> QuoteResult<Quote> {
    let mut best: Option<Quote> = None;
    let mut last_error = no_candidates;
    for result in results {
        match result {
            Ok(quote) => {
                if best
                    .as_ref()
                    .is_none_or(|kept| quote.output_amount > kept.output_amount)
                {
                    best = Some(quote);
                }
            }
            Err(error) => last_error = error,
        }
    }
    best.ok_or(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::swaps::direct::intent::wsol_mint;

    #[tokio::test]
    async fn oversized_input_is_rejected_before_router_io() {
        let request = QuoteRequest {
            chain: ChainId::Solana,
            input_mint: crate::chains::solana::constants::SOL_MINT.to_owned(),
            output_mint: crate::chains::solana::constants::USDC_MINT.to_owned(),
            input_amount: crate::chains::RawAmount::new(u128::from(u64::MAX) + 1),
            wallet_address: String::new(),
            slippage_pct: 1.0,
            swap_mode: crate::swaps::types::SwapMode::ExactIn,
            exclude_dexes: None,
        };
        let error = DirectPoolRouter::new()
            .get_quote(&request)
            .await
            .unwrap_err();
        assert!(matches!(error, QuoteError::RouterRejected { .. }));
        assert!(error.to_string().contains("Solana u64 limit"));
    }

    use crate::chains::solana::swaps::direct::DirectSwapError;

    #[test]
    fn only_the_token_of_a_sol_pair_is_tried_at_its_launch_curve() {
        let sol = crate::chains::solana::constants::SOL_MINT;
        let usdc = crate::chains::solana::constants::USDC_MINT;
        let token = "AMmANDhpdoJiyqqzkHAC2c992WDR9DXbTw6AezTApump";
        let curve = direct::venues::pumpfun_legacy::bonding_curve_address(
            &Pubkey::from_str(token).expect("valid mint"),
        );

        assert_eq!(launch_curve_for(sol, token), Some(curve), "a buy");
        assert_eq!(launch_curve_for(token, sol), Some(curve), "a sell");
        assert_eq!(
            launch_curve_for(usdc, token),
            None,
            "the curve trades SOL only"
        );
        assert_eq!(
            launch_curve_for(sol, usdc),
            None,
            "a reference pair has no curve"
        );
        assert_eq!(launch_curve_for(sol, "not-a-mint"), None);
    }

    /// No quote asset of the chain is ever probed at a launch curve, in either
    /// direction, while a token's curve is unchanged.
    #[test]
    fn no_quote_asset_is_tried_at_a_launch_curve() {
        let adapter = adapter_for(ChainId::Solana);
        let sol = adapter.native_asset_address();
        for stable in adapter.stable_assets() {
            assert_eq!(launch_curve_for(sol, stable), None, "buying {stable}");
            assert_eq!(launch_curve_for(stable, sol), None, "selling {stable}");
        }
        assert_eq!(launch_curve_for(sol, sol), None);

        let token = "AMmANDhpdoJiyqqzkHAC2c992WDR9DXbTw6AezTApump";
        let curve = direct::venues::pumpfun_legacy::bonding_curve_address(
            &Pubkey::from_str(token).expect("valid mint"),
        );
        assert_eq!(launch_curve_for(sol, token), Some(curve));
        assert_eq!(launch_curve_for(token, sol), Some(curve));
    }

    /// Every pair over the native asset, the stablecoins and a token.
    fn pair_legs() -> Vec<&'static str> {
        let adapter = adapter_for(ChainId::Solana);
        let mut legs = vec![adapter.native_asset_address()];
        legs.extend(adapter.stable_assets().iter().copied());
        legs.push("AMmANDhpdoJiyqqzkHAC2c992WDR9DXbTw6AezTApump");
        legs
    }

    /// A sell must find the same pools as the buy it reverses: the key never
    /// depends on which leg is the input.
    #[test]
    fn a_pair_is_keyed_by_the_same_leg_in_both_directions() {
        let legs = pair_legs();
        for a in &legs {
            for b in &legs {
                assert_eq!(pair_token(a, b), pair_token(b, a), "{a} <-> {b}");
                let key = pair_token(a, b);
                assert!(key == *a || key == *b, "the key is one of the two legs");
            }
        }
    }

    #[test]
    fn the_key_is_the_leg_that_is_traded_against_the_quote_asset() {
        let adapter = adapter_for(ChainId::Solana);
        let sol = adapter.native_asset_address();
        let token = "AMmANDhpdoJiyqqzkHAC2c992WDR9DXbTw6AezTApump";

        for stable in adapter.stable_assets() {
            assert_eq!(pair_token(sol, stable), *stable, "SOL -> {stable}");
            assert_eq!(pair_token(stable, sol), *stable, "{stable} -> SOL");
            assert_eq!(pair_token(stable, token), token, "{stable} -> token");
            assert_eq!(pair_token(token, stable), token, "token -> {stable}");
        }
        assert_eq!(pair_token(sol, token), token);
        assert_eq!(pair_token(token, sol), token);

        // Between two stablecoins the fee-reference one is the quote, so the
        // other one keys the pools.
        let reference = adapter
            .stable_assets()
            .iter()
            .copied()
            .find(|mint| fee_reference_for_mint(mint).is_some())
            .expect("a stablecoin is a fee reference");
        for stable in adapter.stable_assets() {
            if *stable != reference {
                assert_eq!(pair_token(reference, stable), *stable);
                assert_eq!(pair_token(stable, reference), *stable);
            }
        }
    }

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

    /// A pool on a programme the engine cannot trade is a shape this router
    /// does not offer: it abstains rather than answering for the token or
    /// masking the other routers' answers.
    #[test]
    fn a_shape_the_engine_lacks_is_an_abstention() {
        assert!(matches!(
            DirectSwapError::UnsupportedVenue {
                program: Pubkey::new_unique(),
            }
            .into_quote_error("Direct Pool"),
            QuoteError::NotOffered {
                reason: NotOfferedReason::UnsupportedVenue,
                ..
            }
        ));
    }

    /// The engine prices forward from the input only, and says so before any
    /// pool is read.
    #[tokio::test]
    async fn an_exact_out_request_is_not_offered() {
        let request = QuoteRequest {
            chain: ChainId::Solana,
            input_mint: crate::chains::solana::constants::SOL_MINT.to_owned(),
            output_mint: crate::chains::solana::constants::USDC_MINT.to_owned(),
            input_amount: 1_000_000u64.into(),
            wallet_address: String::new(),
            slippage_pct: 1.0,
            swap_mode: crate::swaps::types::SwapMode::ExactOut,
            exclude_dexes: None,
        };
        assert!(matches!(
            DirectPoolRouter::new().get_quote(&request).await,
            Err(QuoteError::NotOffered {
                reason: NotOfferedReason::ExactOut,
                ..
            })
        ));
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
        assert_eq!(router.chain(), ChainId::Solana);
        assert!(router.priority() > 0, "Jupiter stays the primary router");
    }

    fn sell_quote(expected_out: u64, min_out: u64) -> direct::DirectQuote {
        let input_mint = Pubkey::new_unique();
        let fee = direct::PlatformFee::resolve(
            direct::FeeSide::Output,
            &input_mint,
            &wsol_mint(),
            expected_out,
        )
        .unwrap();
        direct::DirectQuote {
            pool: Pubkey::new_unique(),
            program: ProgramKind::RaydiumCpmm,
            input_mint,
            output_mint: wsol_mint(),
            amount_in: 1_000_000,
            swap_amount_in: 1_000_000,
            expected_out,
            min_out,
            expected_net_out: expected_out - fee.amount,
            min_net_out: min_out - fee.amount,
            fee,
            lp_fee: 0,
            price_impact_pct: 0.0,
            slippage_bps: 100,
        }
    }

    fn accepted(expected_out: u64, min_out: u64) -> DirectExecutionData {
        let accepted = sell_quote(expected_out, min_out);
        DirectExecutionData {
            pool: accepted.pool.to_string(),
            amount_in: accepted.amount_in,
            slippage_bps: accepted.slippage_bps,
            accepted_min_net_out: accepted.min_net_out,
            accepted_min_out: accepted.min_out,
        }
    }

    #[test]
    fn a_fresh_output_fee_follows_the_fresh_expected_output() {
        let data = accepted(1_000_000, 990_000);

        let mut above = sell_quote(1_000_199, 990_000);
        hold_accepted_floor(&mut above, &data).expect("a better fill is accepted");
        assert_eq!(above.fee.amount, 5_000, "0.5% of the fresh expected output");
        assert_eq!(above.min_net_out, 990_000 - 5_000);
        assert_eq!(above.expected_net_out, 1_000_199 - 5_000);
    }

    #[test]
    fn a_fresh_quote_below_the_accepted_floor_is_a_market_move() {
        let data = accepted(1_000_000, 990_000);

        let mut below = sell_quote(989_000, 979_110);
        assert!(matches!(
            hold_accepted_floor(&mut below, &data),
            Err(direct::DirectSwapError::MarketMoved { .. })
        ));
    }

    #[test]
    fn a_fee_that_steps_by_one_raw_unit_is_not_a_market_move() {
        // 1_000_000 -> fee 5_000; 1_000_200 -> fee 5_001 with the floor unchanged.
        let data = accepted(1_000_000, 990_000);
        let mut rounded = sell_quote(1_000_200, 990_000);
        assert_eq!(rounded.fee.amount, 5_001);
        hold_accepted_floor(&mut rounded, &data)
            .expect("one unit of fee rounding is within tolerance");
        assert_eq!(rounded.min_net_out, 990_000 - 5_001);

        let mut two_units = sell_quote(1_000_000, 990_000);
        let mut strict = data.clone();
        strict.accepted_min_net_out = two_units.min_net_out + 2;
        assert!(matches!(
            hold_accepted_floor(&mut two_units, &strict),
            Err(direct::DirectSwapError::MarketMoved { .. })
        ));
    }

    fn candidate_quote(output_amount: u64, pool_label: &str) -> Quote {
        Quote {
            chain: ChainId::Solana,
            router_id: "direct_pool".to_owned(),
            router_name: "Direct Pool".to_owned(),
            input_mint: crate::chains::solana::constants::SOL_MINT.to_owned(),
            output_mint: crate::chains::solana::constants::USDC_MINT.to_owned(),
            input_amount: 1_000_000u64.into(),
            output_amount: output_amount.into(),
            minimum_output_amount: output_amount.into(),
            price_impact_pct: 0.0,
            platform_fee_lamports: None,
            estimated_network_fee_lamports: None,
            slippage_bps: 100,
            route_plan: pool_label.to_owned(),
            swap_mode: crate::swaps::types::SwapMode::ExactIn,
            wallet_address: String::new(),
            exclude_dexes: None,
            execution_data: Vec::new(),
        }
    }

    fn no_route(detail: &str) -> QuoteError {
        QuoteError::NoRoute {
            router: "Direct Pool".to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn the_candidate_that_leaves_the_wallet_the_most_wins_over_an_earlier_one() {
        let chosen = best_quote(
            vec![
                Ok(candidate_quote(965, "first")),
                Ok(candidate_quote(1_000, "second")),
                Ok(candidate_quote(990, "third")),
            ],
            no_route("none"),
        )
        .unwrap();
        assert_eq!(chosen.route_plan, "second");
    }

    #[test]
    fn a_tie_keeps_the_earlier_candidate() {
        let chosen = best_quote(
            vec![
                Ok(candidate_quote(1_000, "pricing pool")),
                Ok(candidate_quote(1_000, "neighbour")),
            ],
            no_route("none"),
        )
        .unwrap();
        assert_eq!(chosen.route_plan, "pricing pool");
    }

    #[test]
    fn with_no_quote_the_last_error_in_candidate_order_is_returned() {
        let error = best_quote(
            vec![Err(no_route("first")), Err(no_route("second"))],
            no_route("none"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("second"), "{error}");
    }

    #[test]
    fn one_quote_among_errors_is_returned() {
        let chosen = best_quote(
            vec![
                Err(no_route("first")),
                Ok(candidate_quote(500, "only")),
                Err(no_route("third")),
            ],
            no_route("none"),
        )
        .unwrap();
        assert_eq!(chosen.route_plan, "only");
    }

    #[test]
    fn without_candidates_the_supplied_error_is_returned() {
        let error = best_quote(Vec::new(), no_route("no pool")).unwrap_err();
        assert!(error.to_string().contains("no pool"), "{error}");
    }
}
