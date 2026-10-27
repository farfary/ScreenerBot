// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Quote-based paper fills with an injected landing wait for deterministic tests,
//! shared by every paper book.

use std::future::Future;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::chains::{ChainAdapter, RawAmount};
use crate::swaps::{Quote, QuoteError, QuoteRequest, SwapMode};

use super::error::PaperFillError;
use super::types::{PaperFill, PaperModel, PaperQuoteMetadata, PaperSellFill};

/// Longest wait between the initial and the landing quote any paper book may configure.
pub const MAX_LANDING_DELAY_MS: u64 = 30_000;

/// Wait between the initial and the landing quote of a paper fill when none is configured.
pub const DEFAULT_LANDING_DELAY_MS: u64 = 1000;

/// Closed paper rounds a readiness threshold may require.
pub const READINESS_ROUNDS_RANGE: std::ops::RangeInclusive<usize> = 1..=500;

const EXIT_PERCENT_SCALE: u128 = 10_u128.pow(9);
const EXIT_PERCENT_DENOMINATOR: u128 = 100 * EXIT_PERCENT_SCALE;

/// Select the exact raw token input of a paper sale of `exit_percentage` of a holding
/// (`None` sells all of it).
pub fn raw_sell_amount(
    held: RawAmount,
    exit_percentage: Option<f64>,
) -> Result<RawAmount, PaperFillError> {
    if held == RawAmount::ZERO {
        return Err(PaperFillError::EmptySell);
    }
    let Some(percentage) = exit_percentage else {
        return Ok(held);
    };
    if !percentage.is_finite() {
        return Err(PaperFillError::InvalidSize);
    }
    if percentage >= 100.0 {
        return Ok(held);
    }
    if percentage <= 0.0 {
        return Err(PaperFillError::InvalidSize);
    }
    let parts = RawAmount::from_integral_f64((percentage * EXIT_PERCENT_SCALE as f64).floor())
        .filter(|parts| parts.raw() > 0 && parts.raw() < EXIT_PERCENT_DENOMINATOR)
        .ok_or(PaperFillError::InvalidSize)?;
    let amount = held
        .checked_mul_div(parts, RawAmount::new(EXIT_PERCENT_DENOMINATOR))
        .filter(|amount| *amount > RawAmount::ZERO && *amount <= held)
        .ok_or(PaperFillError::InvalidSize)?;
    Ok(amount)
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettledPaperQuote {
    Buy(PaperFill),
    Sell(PaperSellFill),
}

fn quote_matches(request: &QuoteRequest, quote: &Quote) -> bool {
    quote.chain == request.chain
        && quote.input_mint == request.input_mint
        && quote.output_mint == request.output_mint
        && quote.input_amount == request.input_amount
        && quote.swap_mode == request.swap_mode
        && quote.wallet_address == request.wallet_address
        && !quote.router_id.trim().is_empty()
        && quote.output_amount > RawAmount::ZERO
        && quote.minimum_output_amount > RawAmount::ZERO
        && quote.minimum_output_amount <= quote.output_amount
        && quote.price_impact_pct.is_finite()
        && quote.price_impact_pct >= 0.0
}

fn quote_metadata(
    request: &QuoteRequest,
    token_decimals: u8,
    initial: &Quote,
    landing: &Quote,
    initial_at: DateTime<Utc>,
    landing_at: DateTime<Utc>,
    requested_delay_ms: u64,
    queue_duration_ms: u64,
    epoch: u64,
) -> PaperQuoteMetadata {
    PaperQuoteMetadata {
        router_id: landing.router_id.clone(),
        initial_at,
        landing_at,
        requested_delay_ms,
        queue_duration_ms,
        minimum_output_amount: initial.minimum_output_amount,
        final_output_amount: landing.output_amount,
        input_amount: Some(request.input_amount),
        token_decimals: Some(token_decimals),
        epoch,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    }
}

fn checked_samples(
    request: &QuoteRequest,
    initial: Quote,
    landing: Quote,
) -> Result<(Quote, Quote), PaperFillError> {
    if !quote_matches(request, &initial) || !quote_matches(request, &landing) {
        return Err(PaperFillError::QuoteUnfillable);
    }
    if initial.estimated_network_fee_lamports.is_none()
        || landing.estimated_network_fee_lamports.is_none()
    {
        return Err(PaperFillError::QuoteUnfillable);
    }
    if landing.output_amount < initial.minimum_output_amount {
        return Err(PaperFillError::MinimumOutputNotMet);
    }
    Ok((initial, landing))
}

async fn quote_twice<E, Q, QFut, W, WFut>(
    request: &QuoteRequest,
    token_decimals: u8,
    delay_ms: u64,
    queued_at: DateTime<Utc>,
    epoch: u64,
    mut quote: Q,
    wait: W,
) -> Result<(Quote, Quote, PaperQuoteMetadata), E>
where
    E: From<PaperFillError>,
    Q: FnMut(QuoteRequest) -> QFut,
    QFut: Future<Output = Result<Quote, QuoteError>>,
    W: FnOnce(Duration) -> WFut,
    WFut: Future<Output = Result<(), E>>,
{
    let queued_duration_start = Utc::now();
    let initial = quote(request.clone())
        .await
        .map_err(|_| PaperFillError::QuoteUnavailable)?;
    let initial_at = Utc::now();
    if !quote_matches(request, &initial) || initial.estimated_network_fee_lamports.is_none() {
        return Err(PaperFillError::QuoteUnfillable.into());
    }
    wait(Duration::from_millis(delay_ms)).await?;
    let landing = quote(request.clone())
        .await
        .map_err(|_| PaperFillError::QuoteUnavailable)?;
    let landing_at = Utc::now();
    let (initial, landing) = checked_samples(request, initial, landing)?;
    let metadata = quote_metadata(
        request,
        token_decimals,
        &initial,
        &landing,
        initial_at,
        landing_at,
        delay_ms,
        queued_duration_start
            .signed_duration_since(queued_at)
            .num_milliseconds()
            .max(0) as u64,
        epoch,
    );
    Ok((initial, landing, metadata))
}

fn estimated_network_fee(total_lamports: u64, adapter: &dyn ChainAdapter) -> f64 {
    adapter.raw_to_native(total_lamports)
}

/// Settle validated samples from one exact-input request. This is also the
/// deterministic fixture boundary used by Promo Studio; it performs no I/O.
pub fn settle_quote_samples(
    request: &QuoteRequest,
    token_decimals: u8,
    initial: Quote,
    landing: Quote,
    initial_at: DateTime<Utc>,
    landing_at: DateTime<Utc>,
    requested_delay_ms: u64,
    queue_duration_ms: u64,
    epoch: u64,
) -> Result<SettledPaperQuote, PaperFillError> {
    if request.swap_mode != SwapMode::ExactIn || request.input_amount == RawAmount::ZERO {
        return Err(PaperFillError::InvalidSize);
    }
    let (initial, landing) = checked_samples(request, initial, landing)?;
    let adapter = crate::chains::adapter_for(request.chain);
    let fee_lamports = landing
        .estimated_network_fee_lamports
        .ok_or(PaperFillError::QuoteUnfillable)?;
    let network_fee_sol = estimated_network_fee(fee_lamports, adapter);
    let metadata = quote_metadata(
        request,
        token_decimals,
        &initial,
        &landing,
        initial_at,
        landing_at,
        requested_delay_ms,
        queue_duration_ms,
        epoch,
    );
    if adapter.is_native_asset(&request.input_mint) {
        if adapter.is_native_asset(&request.output_mint) {
            return Err(PaperFillError::QuoteUnfillable);
        }
        let input_sol = request
            .input_amount
            .to_whole_units(adapter.native_asset_decimals());
        let token_amount = landing.output_amount.to_whole_units(token_decimals);
        let fill_price_sol = input_sol / token_amount;
        let total_cost_sol = input_sol + network_fee_sol;
        if !input_sol.is_finite()
            || input_sol <= 0.0
            || !token_amount.is_finite()
            || token_amount <= 0.0
            || !fill_price_sol.is_finite()
            || fill_price_sol <= 0.0
            || !total_cost_sol.is_finite()
            || total_cost_sol <= 0.0
        {
            return Err(PaperFillError::QuoteUnfillable);
        }
        let fill = PaperFill {
            input_sol,
            market_price_sol: fill_price_sol,
            priced_from_pool: false,
            fill_price_sol,
            token_amount,
            referral_fee_sol: 0.0,
            network_fee_sol,
            priority_fee_sol: 0.0,
            total_cost_sol,
            model: PaperModel::Quote,
            quote: Some(metadata),
        };
        return Ok(SettledPaperQuote::Buy(fill));
    }
    if !adapter.is_native_asset(&request.output_mint) {
        return Err(PaperFillError::QuoteUnfillable);
    }
    let token_amount = request.input_amount.to_whole_units(token_decimals);
    let quoted_net_sol = landing
        .output_amount
        .to_whole_units(adapter.native_asset_decimals());
    let net_proceeds_sol = quoted_net_sol - network_fee_sol;
    let fill_price_sol = quoted_net_sol / token_amount;
    if !token_amount.is_finite()
        || token_amount <= 0.0
        || !quoted_net_sol.is_finite()
        || quoted_net_sol <= 0.0
        || !net_proceeds_sol.is_finite()
        || net_proceeds_sol <= 0.0
        || !fill_price_sol.is_finite()
        || fill_price_sol <= 0.0
    {
        return Err(PaperFillError::QuoteUnfillable);
    }
    let fill = PaperSellFill {
        token_amount,
        market_price_sol: fill_price_sol,
        priced_from_pool: false,
        fill_price_sol,
        gross_sol: quoted_net_sol,
        referral_fee_sol: 0.0,
        network_fee_sol,
        priority_fee_sol: 0.0,
        net_proceeds_sol,
        model: PaperModel::Quote,
        quote: Some(metadata),
    };
    Ok(SettledPaperQuote::Sell(fill))
}

/// Simulate a buy from the initial and landing quote for its exact raw input.
pub async fn simulate_buy<E, Q, QFut, W, WFut>(
    request: QuoteRequest,
    token_decimals: u8,
    delay_ms: u64,
    queued_at: DateTime<Utc>,
    epoch: u64,
    quote: Q,
    wait: W,
) -> Result<PaperFill, E>
where
    E: From<PaperFillError>,
    Q: FnMut(QuoteRequest) -> QFut,
    QFut: Future<Output = Result<Quote, QuoteError>>,
    W: FnOnce(Duration) -> WFut,
    WFut: Future<Output = Result<(), E>>,
{
    if request.swap_mode != SwapMode::ExactIn || request.input_amount == RawAmount::ZERO {
        return Err(PaperFillError::InvalidSize.into());
    }
    let adapter = crate::chains::adapter_for(request.chain);
    if !adapter.is_native_asset(&request.input_mint)
        || adapter.is_native_asset(&request.output_mint)
    {
        return Err(PaperFillError::QuoteUnfillable.into());
    }
    let (initial, landing, metadata) = quote_twice(
        &request,
        token_decimals,
        delay_ms,
        queued_at,
        epoch,
        quote,
        wait,
    )
    .await?;
    match settle_quote_samples(
        &request,
        token_decimals,
        initial,
        landing,
        metadata.initial_at,
        metadata.landing_at,
        metadata.requested_delay_ms,
        metadata.queue_duration_ms,
        metadata.epoch,
    )? {
        SettledPaperQuote::Buy(fill) => Ok(fill),
        SettledPaperQuote::Sell(_) => Err(PaperFillError::QuoteUnfillable.into()),
    }
}

/// Simulate a sell using the exact raw token amount accepted for quoting.
pub async fn simulate_sell<E, Q, QFut, W, WFut>(
    request: QuoteRequest,
    token_decimals: u8,
    delay_ms: u64,
    queued_at: DateTime<Utc>,
    epoch: u64,
    quote: Q,
    wait: W,
) -> Result<PaperSellFill, E>
where
    E: From<PaperFillError>,
    Q: FnMut(QuoteRequest) -> QFut,
    QFut: Future<Output = Result<Quote, QuoteError>>,
    W: FnOnce(Duration) -> WFut,
    WFut: Future<Output = Result<(), E>>,
{
    if request.swap_mode != SwapMode::ExactIn || request.input_amount == RawAmount::ZERO {
        return Err(PaperFillError::EmptySell.into());
    }
    let adapter = crate::chains::adapter_for(request.chain);
    if adapter.is_native_asset(&request.input_mint)
        || !adapter.is_native_asset(&request.output_mint)
    {
        return Err(PaperFillError::QuoteUnfillable.into());
    }
    let (initial, landing, metadata) = quote_twice(
        &request,
        token_decimals,
        delay_ms,
        queued_at,
        epoch,
        quote,
        wait,
    )
    .await?;
    match settle_quote_samples(
        &request,
        token_decimals,
        initial,
        landing,
        metadata.initial_at,
        metadata.landing_at,
        metadata.requested_delay_ms,
        metadata.queue_duration_ms,
        metadata.epoch,
    )? {
        SettledPaperQuote::Sell(fill) => Ok(fill),
        SettledPaperQuote::Buy(_) => Err(PaperFillError::QuoteUnfillable.into()),
    }
}

#[cfg(test)]
#[path = "fill/tests.rs"]
mod tests;
