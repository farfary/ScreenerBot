// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper executor: fills a paper decision from the router quotes a live swap of the
//! same size would be offered, after the configured landing wait, and books the fill onto
//! the paper book. It runs inside the paper scope, which the signing, swap and send seams
//! refuse, and never spawns a task.

use chrono::Utc;

use crate::chains::RawAmount;
use crate::logger::{self, LogTag};
use crate::positions::operations::{check_auto_add, entry_slippage, exit_slippage_ladder};
use crate::positions::paper::{PaperBook, PaperBooked, PaperOpenTerms, PaperSale};
use crate::positions::{Book, Opening, Position, PositionOrigin};
use crate::swaps::{QuoteRequest, SwapMode};
use crate::trader::executors::{resolve_exit_size, ExecutionContext, ExitSize};
use crate::trader::types::{TradeAction, TradeDecision, TradeReason, TradeResult, TradeStep};
use crate::trader::{Error, Result};

use super::fill;
use super::{PaperFill, PaperSellFill};

mod io;
use io::{PaperTradeIo, ProductionPaperTradeIo};

#[cfg(test)]
mod tests;

/// Records in the paper store whether the sale of round `position_id` filled. For a sale
/// that found no fill, returns the longest run of unfilled exits after it among the
/// strategies the sale belongs to: the entry strategy of the round and the exit strategy
/// that decided the sale. A filled exit ends the run.
async fn record_exit_outcome(
    book: &PaperBook,
    wallet: &str,
    position: &Position,
    position_id: i64,
    decision: &TradeDecision,
    filled: bool,
) -> crate::positions::Result<u32> {
    book.record_exit_attempt(position_id, decision.strategy_id.as_deref(), filled)
        .await?;
    let mut longest = 0;
    if !filled {
        for strategy_id in [
            position.origin.entry_strategy_id(),
            decision.strategy_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            longest = longest.max(book.failed_exit_streak(wallet, strategy_id).await?);
        }
    }
    Ok(longest)
}

/// A trade the user asked for: it runs while the auto-trader is stopped and is not bound
/// by the auto-trader's DCA policy.
fn is_manual(reason: &TradeReason) -> bool {
    matches!(
        reason,
        TradeReason::ManualEntry
            | TradeReason::ForceBuy
            | TradeReason::ManualExit
            | TradeReason::ForceSell
    )
}

/// Fills `decision` on the paper book, opening its round with `context`.
pub async fn execute(decision: &TradeDecision, context: &ExecutionContext) -> Result<TradeResult> {
    execute_with(decision, context, &ProductionPaperTradeIo).await
}

async fn execute_with(
    decision: &TradeDecision,
    context: &ExecutionContext,
    io: &impl PaperTradeIo,
) -> Result<TradeResult> {
    if decision.book != Book::Paper {
        return Err(Error::PaperDecisionOnLivePath {
            mint: decision.mint.clone(),
        });
    }
    crate::global::paper_scope(async {
        let book = io.book().ok_or(Error::PaperBookUnavailable)?;
        let wallet = io.wallet_address().ok_or(Error::PaperBookUnavailable)?;
        match decision.action {
            TradeAction::Buy => buy(decision, context, io, book, &wallet).await,
            TradeAction::DCA => add(decision, io, book, &wallet).await,
            TradeAction::Sell => sell(decision, io, book, &wallet).await,
        }
    })
    .await
}

fn refused(decision: &TradeDecision, step: TradeStep, error: String) -> TradeResult {
    logger::info(
        LogTag::Trader,
        &format!(
            "Paper {:?} for {} not filled: {error}",
            decision.action, decision.mint
        ),
    );
    TradeResult::failure_at(decision.clone(), step, error, 0)
}

fn booking_failed(decision: &TradeDecision, error: &crate::positions::Error) -> TradeResult {
    let message = format!("Paper booking failed: {error}");
    logger::error(
        LogTag::Trader,
        &format!("{message} (mint {})", decision.mint),
    );
    TradeResult::failure_from(decision.clone(), error, message)
}

/// The open paper round a decision acts on: the round it names, else the open round of
/// its mint.
async fn target_round(book: &PaperBook, decision: &TradeDecision) -> Option<Position> {
    let named = decision
        .position_id
        .as_deref()
        .and_then(|id| id.parse::<i64>().ok());
    if let Some(id) = named {
        if let Some(position) = book.position(id).await {
            if position.mint == decision.mint {
                return Some(position);
            }
        }
    }
    book.open_round(&decision.mint).await
}

/// The cash of the current paper run, or the result refusing the trade when it cannot be
/// read or does not cover `needed`.
async fn cash_covering(
    decision: &TradeDecision,
    io: &impl PaperTradeIo,
    book: &PaperBook,
    wallet: &str,
    needed: f64,
) -> std::result::Result<i64, TradeResult> {
    let account = book
        .account(wallet, io.settings().starting_balance_native)
        .await
        .map_err(|error| booking_failed(decision, &error))?;
    if account.cash_native + f64::EPSILON < needed {
        return Err(refused(
            decision,
            TradeStep::Validation,
            format!(
                "paper balance exhausted: {:.6} SOL available, {needed:.6} SOL needed",
                account.cash_native
            ),
        ));
    }
    Ok(account.epoch)
}

/// The quote request of a buy of `size_native` of `decision.mint`.
fn buy_request(
    decision: &TradeDecision,
    wallet: &str,
    size_native: f64,
) -> std::result::Result<QuoteRequest, TradeResult> {
    let chain = crate::chains::active_chain();
    let adapter = crate::chains::adapter_for(chain);
    let scale = 10_f64.powi(i32::from(adapter.native_asset_decimals()));
    let Some(input_amount) = RawAmount::from_integral_f64((size_native * scale).floor())
        .filter(|amount| *amount > RawAmount::ZERO)
    else {
        return Err(refused(
            decision,
            TradeStep::Validation,
            format!("paper buy size {size_native} SOL is not a positive amount"),
        ));
    };
    Ok(QuoteRequest {
        chain,
        input_mint: adapter.native_asset_address().to_owned(),
        output_mint: decision.mint.clone(),
        input_amount,
        wallet_address: wallet.to_owned(),
        slippage_pct: entry_slippage(decision.slippage_pct),
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    })
}

/// A buy filled from the router quotes after the landing wait, refused when the decision
/// is no longer wanted once the wait ends.
async fn fill_buy(
    decision: &TradeDecision,
    origin: &PositionOrigin,
    io: &impl PaperTradeIo,
    request: QuoteRequest,
    decimals: u8,
    epoch: i64,
) -> std::result::Result<PaperFill, TradeResult> {
    let filled = fill::simulate_buy::<Error, _, _, _, _>(
        request,
        decimals,
        io.settings().landing_delay_ms,
        Utc::now(),
        u64::try_from(epoch).unwrap_or_default(),
        |request| io.quote(request),
        |delay| landing_wait(decision, origin, io, delay),
    )
    .await;
    filled.map_err(|error| refused(decision, fill_step(&error), error.to_string()))
}

async fn landing_wait(
    decision: &TradeDecision,
    origin: &PositionOrigin,
    io: &impl PaperTradeIo,
    delay: std::time::Duration,
) -> Result<()> {
    io.wait(delay).await;
    if io.still_wanted(decision, origin).await {
        Ok(())
    } else {
        Err(Error::PaperFillAborted {
            mint: decision.mint.clone(),
        })
    }
}

fn fill_step(error: &Error) -> TradeStep {
    match error {
        Error::PaperFillAborted { .. } => TradeStep::Validation,
        _ => TradeStep::Quote,
    }
}

async fn buy(
    decision: &TradeDecision,
    context: &ExecutionContext,
    io: &impl PaperTradeIo,
    book: &PaperBook,
    wallet: &str,
) -> Result<TradeResult> {
    if !io.still_wanted(decision, &context.origin).await {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "paper buy is no longer wanted".to_owned(),
        ));
    }
    let size_native = io.entry_size(decision);
    let epoch = match cash_covering(decision, io, book, wallet, size_native).await {
        Ok(epoch) => epoch,
        Err(result) => return Ok(result),
    };
    let Some(token) = io.token(&decision.mint).await else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "token metadata unavailable".to_owned(),
        ));
    };
    let Some(decimals) = token.decimals else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "token decimals unknown".to_owned(),
        ));
    };
    let request = match buy_request(decision, wallet, size_native) {
        Ok(request) => request,
        Err(result) => return Ok(result),
    };
    let market = io.market_price(&decision.mint).await;
    let fill = match fill_buy(decision, &context.origin, io, request, decimals, epoch).await {
        Ok(fill) => fill,
        Err(result) => return Ok(result),
    };
    if let Err(result) = cash_covering(decision, io, book, wallet, fill.total_cost_sol).await {
        return Ok(result);
    }

    let opening = Opening {
        mint: decision.mint.clone(),
        symbol: token.symbol,
        name: token.name,
        entry_price: market
            .as_ref()
            .map(|market| market.price_native)
            .unwrap_or(fill.market_price_sol),
        entry_time: Utc::now(),
        size_native: fill.input_sol,
        entry_signature: None,
        origin: context.origin.clone(),
        management: context.management,
        entry_liquidity: market.and_then(|market| market.liquidity),
        book: Book::Paper,
    };
    let settings = io.settings();
    let terms = PaperOpenTerms {
        max_open: settings.max_open,
        open_cooldown_secs: settings.open_cooldown_secs,
        epoch,
    };
    match book
        .book_buy(wallet, opening, &fill, decision.strategy_id.clone(), &terms)
        .await
    {
        Ok(position) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Paper buy filled: {} | {:.6} SOL at {:.12} | position {:?}",
                    decision.mint, fill.input_sol, fill.fill_price_sol, position.id
                ),
            );
            Ok(TradeResult::paper(
                decision.clone(),
                Some(fill.fill_price_sol),
                Some(fill.input_sol),
                position.id.map(|id| id.to_string()),
            ))
        }
        Err(crate::positions::Error::SlotUnavailable { remaining }) => {
            let mut result = refused(
                decision,
                TradeStep::Validation,
                format!("paper position limit reached (remaining permits: {remaining})"),
            );
            result.capacity_guard_remaining = Some(remaining);
            Ok(result)
        }
        Err(
            error @ (crate::positions::Error::PaperCashShort { .. }
            | crate::positions::Error::PaperOpenCooldown { .. }
            | crate::positions::Error::PaperRunEnded { .. }),
        ) => Ok(refused(decision, TradeStep::Validation, error.to_string())),
        Err(error) => Ok(booking_failed(decision, &error)),
    }
}

async fn add(
    decision: &TradeDecision,
    io: &impl PaperTradeIo,
    book: &PaperBook,
    wallet: &str,
) -> Result<TradeResult> {
    let Some(size_native) = decision.size_native else {
        return Err(Error::AddSizeRequired {
            mint: decision.mint.clone(),
        });
    };
    let Some(position) = target_round(book, decision).await else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "no open paper round to add to".to_owned(),
        ));
    };
    let Some(position_id) = position.id else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "the paper round has no id".to_owned(),
        ));
    };
    if !io.still_wanted(decision, &position.origin).await {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "paper add is no longer wanted".to_owned(),
        ));
    }
    if !is_manual(&decision.reason) {
        if let Err(error) = check_auto_add(&position, &decision.mint) {
            return Ok(booking_failed(decision, &error));
        }
    }
    let epoch = match cash_covering(decision, io, book, wallet, size_native).await {
        Ok(epoch) => epoch,
        Err(result) => return Ok(result),
    };
    let Some(decimals) = io
        .token(&decision.mint)
        .await
        .and_then(|token| token.decimals)
    else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "token decimals unknown".to_owned(),
        ));
    };
    let request = match buy_request(decision, wallet, size_native) {
        Ok(request) => request,
        Err(result) => return Ok(result),
    };
    let fill = match fill_buy(decision, &position.origin, io, request, decimals, epoch).await {
        Ok(fill) => fill,
        Err(result) => return Ok(result),
    };
    if let Err(result) = cash_covering(decision, io, book, wallet, fill.total_cost_sol).await {
        return Ok(result);
    }
    match book
        .book_add(
            wallet,
            position_id,
            &decision.mint,
            &fill,
            decision.strategy_id.clone(),
        )
        .await
    {
        Ok(PaperBooked::Booked(_)) => Ok(TradeResult::paper(
            decision.clone(),
            Some(fill.fill_price_sol),
            Some(fill.input_sol),
            Some(position_id.to_string()),
        )),
        Ok(other) => Ok(refused(
            decision,
            TradeStep::Validation,
            format!("paper add not booked: {other:?}"),
        )),
        Err(error @ crate::positions::Error::PaperCashShort { .. }) => {
            Ok(refused(decision, TradeStep::Validation, error.to_string()))
        }
        Err(error) => Ok(booking_failed(decision, &error)),
    }
}

/// A sale of `amount` filled from the router quotes, walking the exit slippage ladder as
/// the live exit does: the first rung that fills wins. `None` when no rung fills.
async fn fill_sale(
    decision: &TradeDecision,
    position: &Position,
    io: &impl PaperTradeIo,
    wallet: &str,
    amount: RawAmount,
    decimals: u8,
    epoch: i64,
) -> std::result::Result<PaperSellFill, Error> {
    let chain = crate::chains::active_chain();
    let adapter = crate::chains::adapter_for(chain);
    let mut last_error = Error::PaperFill(super::PaperFillError::QuoteUnfillable);
    for slippage_pct in exit_slippage_ladder(decision.slippage_pct) {
        let request = QuoteRequest {
            chain,
            input_mint: decision.mint.clone(),
            output_mint: adapter.native_asset_address().to_owned(),
            input_amount: amount,
            wallet_address: wallet.to_owned(),
            slippage_pct,
            swap_mode: SwapMode::ExactIn,
            exclude_dexes: None,
        };
        let filled = fill::simulate_sell::<Error, _, _, _, _>(
            request,
            decimals,
            io.settings().landing_delay_ms,
            Utc::now(),
            u64::try_from(epoch).unwrap_or_default(),
            |request| io.quote(request),
            |delay| landing_wait(decision, &position.origin, io, delay),
        )
        .await;
        match filled {
            Ok(fill) => return Ok(fill),
            Err(error @ Error::PaperFillAborted { .. }) => return Err(error),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

async fn sell(
    decision: &TradeDecision,
    io: &impl PaperTradeIo,
    book: &PaperBook,
    wallet: &str,
) -> Result<TradeResult> {
    let Some(position) = target_round(book, decision).await else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "no open paper round to sell".to_owned(),
        ));
    };
    let Some(position_id) = position.id else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "the paper round has no id".to_owned(),
        ));
    };
    if !io.still_wanted(decision, &position.origin).await {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "paper sale is no longer wanted".to_owned(),
        ));
    }
    let Some(decimals) = io
        .token(&decision.mint)
        .await
        .and_then(|token| token.decimals)
    else {
        return Ok(refused(
            decision,
            TradeStep::Validation,
            "token decimals unknown".to_owned(),
        ));
    };
    let exit_size = resolve_exit_size(&decision.reason, decision.exit_percentage);
    let percentage = match exit_size {
        ExitSize::Full => None,
        ExitSize::Partial(percentage) => Some(percentage),
    };
    let amount = match fill::raw_sell_amount(position.held_amount().unwrap_or_default(), percentage)
    {
        Ok(amount) => amount,
        Err(error) => return Ok(refused(decision, TradeStep::Validation, error.to_string())),
    };
    let epoch = match book
        .account(wallet, io.settings().starting_balance_native)
        .await
    {
        Ok(account) => account.epoch,
        Err(error) => return Ok(booking_failed(decision, &error)),
    };

    let fill = match fill_sale(decision, &position, io, wallet, amount, decimals, epoch).await {
        Ok(fill) => fill,
        Err(error @ Error::PaperFillAborted { .. }) => {
            return Ok(refused(decision, TradeStep::Validation, error.to_string()));
        }
        Err(error) => {
            let streak = match record_exit_outcome(
                book,
                wallet,
                &position,
                position_id,
                decision,
                false,
            )
            .await
            {
                Ok(streak) => Some(streak),
                Err(record_error) => {
                    logger::warning(
                            LogTag::Trader,
                            &format!(
                                "Paper exit attempt of position {position_id} not recorded: {record_error}"
                            ),
                        );
                    None
                }
            };
            crate::events::record_trader_event(
                "paper_exit_failed",
                crate::events::Severity::Warn,
                Some(&decision.mint),
                None,
                serde_json::json!({
                    "book": Book::Paper.as_str(),
                    "position_id": position_id,
                    "reason": format!("{:?}", decision.reason),
                    "error": error.to_string(),
                    "consecutive_failures": streak,
                }),
            )
            .await;
            return Ok(refused(decision, TradeStep::Quote, error.to_string()));
        }
    };

    let exit_reason = format!("{:?}", decision.reason);
    let sale = match exit_size {
        ExitSize::Partial(percentage) => PaperSale::Partial {
            percentage,
            take_profit_level: decision.take_profit_level,
        },
        ExitSize::Full => PaperSale::Full {
            exit_price: decision
                .price_native
                .or(position.current_price)
                .unwrap_or(fill.market_price_sol),
            reason: &exit_reason,
        },
    };
    match book
        .book_sale(
            wallet,
            position_id,
            &decision.mint,
            &fill,
            sale,
            decision.strategy_id.clone(),
        )
        .await
    {
        Ok(PaperBooked::Booked(_)) => {
            if let Err(record_error) =
                record_exit_outcome(book, wallet, &position, position_id, decision, true).await
            {
                logger::warning(
                    LogTag::Trader,
                    &format!(
                        "Paper exit attempt of position {position_id} not recorded: {record_error}"
                    ),
                );
            }
            logger::info(
                LogTag::Trader,
                &format!(
                    "Paper sale filled: {} | {:?} | {:.6} SOL at {:.12} | {exit_reason}",
                    decision.mint, exit_size, fill.net_proceeds_sol, fill.fill_price_sol
                ),
            );
            Ok(TradeResult::paper(
                decision.clone(),
                Some(fill.fill_price_sol),
                Some(fill.net_proceeds_sol),
                Some(position_id.to_string()),
            ))
        }
        Ok(other) => Ok(refused(
            decision,
            TradeStep::Validation,
            format!("paper sale not booked: {other:?}"),
        )),
        Err(error) => Ok(booking_failed(decision, &error)),
    }
}
