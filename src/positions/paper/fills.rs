// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper fills booked onto the paper book: a quote-settled buy, add or sale turned into
//! the fill the live verifier books, and the close of a round at its mark when a paper run
//! is reset.

use chrono::{DateTime, Utc};

use crate::chains::RawAmount;
use crate::positions::booking::{CloseFill, DcaFill, EntryFill, PartialExitFill};
use crate::positions::types::{Opening, Position};
use crate::positions::{Error, Result};
use crate::trader::paper::{PaperFill, PaperModel, PaperSellFill};

use super::book::{PaperBook, PaperBooked, PaperOpenTerms};
use super::leg::PaperLeg;

/// How a paper sale leaves its round.
#[derive(Debug, Clone, Copy)]
pub enum PaperSale<'a> {
    /// `percentage` of the held amount; `take_profit_level` is the ladder step it takes.
    Partial {
        percentage: f64,
        take_profit_level: Option<u32>,
    },
    /// The whole round, at the market price `exit_price`, for `reason`.
    Full { exit_price: f64, reason: &'a str },
}

/// The raw token amount a quote-settled fill moved and the token's decimals, from the
/// quote evidence the fill carries.
fn quoted_amount(
    mint: &str,
    quote: Option<&crate::trader::paper::PaperQuoteMetadata>,
    sold: bool,
) -> Result<(RawAmount, u8)> {
    let unbookable = |detail: &str| Error::PaperFillUnbookable {
        mint: mint.to_owned(),
        detail: detail.to_owned(),
    };
    let quote = quote.ok_or_else(|| unbookable("the fill carries no quote evidence"))?;
    let decimals = quote
        .token_decimals
        .ok_or_else(|| unbookable("the quote evidence carries no token decimals"))?;
    let amount = if sold {
        quote
            .input_amount
            .ok_or_else(|| unbookable("the quote evidence carries no sold amount"))?
    } else {
        quote.final_output_amount
    };
    if amount == RawAmount::ZERO {
        return Err(unbookable("the fill moved no tokens"));
    }
    Ok((amount, decimals))
}

impl PaperBook {
    /// The estimated network fee of a fill in raw units of the book's native asset.
    fn fee_raw(&self, network_fee_native: f64) -> u64 {
        crate::chains::adapter_for(self.db.chain()).native_to_raw(network_fee_native)
    }

    /// Opens a paper round from the quote-settled buy `fill` within `terms` (see
    /// [`PaperBook::open`]).
    pub(crate) async fn book_buy(
        &self,
        wallet_address: &str,
        opening: Opening,
        fill: &PaperFill,
        strategy_id: Option<String>,
        terms: &PaperOpenTerms,
    ) -> Result<Position> {
        let (token_amount, _) = quoted_amount(&opening.mint, fill.quote.as_ref(), false)?;
        let entry = EntryFill {
            effective_entry_price: fill.fill_price_sol,
            token_amount,
            fee_raw: self.fee_raw(fill.network_fee_sol),
            native_size: fill.input_sol,
        };
        let leg = PaperLeg::buy(fill, strategy_id, Utc::now())?;
        self.open(wallet_address, opening, &entry, &leg, terms)
            .await
    }

    /// Books the quote-settled add `fill` onto the open round `position_id`.
    pub(crate) async fn book_add(
        &self,
        wallet_address: &str,
        position_id: i64,
        mint: &str,
        fill: &PaperFill,
        strategy_id: Option<String>,
    ) -> Result<PaperBooked> {
        let (tokens_bought, decimals) = quoted_amount(mint, fill.quote.as_ref(), false)?;
        let now = Utc::now();
        let add = DcaFill {
            tokens_bought,
            native_spent: fill.input_sol,
            fee_raw: self.fee_raw(fill.network_fee_sol),
            dca_time: now,
        };
        let leg = PaperLeg::buy(fill, strategy_id, now)?;
        self.dca(
            wallet_address,
            position_id,
            &add,
            fill.fill_price_sol,
            decimals,
            &leg,
        )
        .await
    }

    /// Books the quote-settled sale `fill` onto the open round `position_id`, as a partial
    /// exit or as the close of the round.
    pub(crate) async fn book_sale(
        &self,
        wallet_address: &str,
        position_id: i64,
        mint: &str,
        fill: &PaperSellFill,
        sale: PaperSale<'_>,
        strategy_id: Option<String>,
    ) -> Result<PaperBooked> {
        let (sold, _) = quoted_amount(mint, fill.quote.as_ref(), true)?;
        let now = Utc::now();
        let fee_raw = self.fee_raw(fill.network_fee_sol);
        let leg = PaperLeg::sell(fill, strategy_id, now)?;
        match sale {
            PaperSale::Partial {
                percentage,
                take_profit_level,
            } => {
                let partial = PartialExitFill {
                    exit_amount: sold,
                    native_received: fill.net_proceeds_sol,
                    effective_exit_price: fill.fill_price_sol,
                    unrealized_pnl: None,
                    fee_raw,
                };
                self.partial(
                    wallet_address,
                    position_id,
                    &partial,
                    percentage,
                    take_profit_level,
                    &leg,
                )
                .await
            }
            PaperSale::Full { exit_price, reason } => {
                let close = CloseFill {
                    effective_exit_price: fill.fill_price_sol,
                    native_received: fill.net_proceeds_sol,
                    fee_raw,
                    exit_time: now,
                };
                self.close(
                    wallet_address,
                    position_id,
                    &close,
                    exit_price,
                    reason,
                    &leg,
                )
                .await
            }
        }
    }

    /// Closes the open round `position` at `mark`, the price it was last marked at, for
    /// `reason`, with no quote and no fee: the close a paper run reset books for rounds
    /// still open. `decimals` are the token's.
    pub(crate) async fn close_at_mark(
        &self,
        wallet_address: &str,
        position: &Position,
        mark: f64,
        decimals: u8,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<PaperBooked> {
        let unbookable = |detail: &str| Error::PaperFillUnbookable {
            mint: position.mint.clone(),
            detail: detail.to_owned(),
        };
        let position_id = position
            .id
            .ok_or_else(|| unbookable("the round has no id"))?;
        if !mark.is_finite() || mark <= 0.0 {
            return Err(Error::InvalidPrice {
                mint: position.mint.clone(),
                price: mark,
            });
        }
        let held = position.held_amount().unwrap_or_default();
        let tokens = held.to_whole_units(decimals);
        let value = tokens * mark;
        let sale = PaperSellFill {
            token_amount: tokens,
            market_price_sol: mark,
            priced_from_pool: true,
            fill_price_sol: mark,
            gross_sol: value,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            net_proceeds_sol: value,
            model: PaperModel::Mark,
            quote: None,
        };
        let leg = PaperLeg::sell(&sale, None, now)?;
        let close = CloseFill {
            effective_exit_price: mark,
            native_received: value,
            fee_raw: 0,
            exit_time: now,
        };
        self.close(wallet_address, position_id, &close, mark, reason, &leg)
            .await
    }
}
