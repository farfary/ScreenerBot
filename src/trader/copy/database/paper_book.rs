// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Mutations that apply quote-backed or historical outcomes to the paper book.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::chains::RawAmount;
use crate::trader::copy::types::{PaperDecision, PaperModel, PaperSellFill};
use crate::trader::error::Error;

/// Book a paper fill: the tokens and their full cost (input plus fees) join the
/// position, reopening it when an earlier round was sold out.
pub(super) fn apply_paper_buy(
    transaction: &Transaction<'_>,
    decision: &PaperDecision,
) -> crate::trader::Result<()> {
    let metadata = decision
        .fill
        .quote
        .as_ref()
        .filter(|_| decision.fill.model == PaperModel::Quote)
        .ok_or_else(|| Error::CopyValidation {
            detail: "paper buy lacks quote provenance".to_owned(),
        })?;
    let raw_output = metadata.final_output_amount;
    let decimals = metadata
        .token_decimals
        .ok_or_else(|| Error::CopyValidation {
            detail: "paper buy lacks token decimals".to_owned(),
        })?;
    if metadata
        .input_amount
        .is_none_or(|amount| amount == RawAmount::ZERO)
        || raw_output == RawAmount::ZERO
        || metadata.minimum_output_amount == RawAmount::ZERO
        || metadata.minimum_output_amount > raw_output
        || decision.fill.token_amount.to_bits() != raw_output.to_whole_units(decimals).to_bits()
    {
        return Err(Error::CopyValidation {
            detail: "paper buy raw amount does not match its display amount".to_owned(),
        });
    }
    if !decision.fill.total_cost_sol.is_finite()
        || decision.fill.total_cost_sol <= 0.0
        || !decision.fill.input_sol.is_finite()
        || decision.fill.input_sol <= 0.0
        || !decision.fill.market_price_sol.is_finite()
        || decision.fill.market_price_sol <= 0.0
        || metadata.router_id.trim().is_empty()
        || !metadata.network_fee_estimated
        || !metadata.platform_fee_included_in_output
    {
        return Err(Error::CopyValidation {
            detail: "paper buy cost is invalid".to_owned(),
        });
    }
    let at = decision.telemetry.decided_at.to_rfc3339();
    let current = transaction
        .query_row(
            "SELECT token_amount_raw, token_decimals, cost_basis_native, invested_native, \
             realized_proceeds_native, realized_cost_native, buys, sells, closed_at, \
             opened_at, peak_price_native FROM copy_paper_positions WHERE task_id=?1 AND mint=?2",
            params![decision.task_id, decision.mint],
            |row| {
                Ok((
                    row.get::<_, Option<RawAmount>>(0)?,
                    row.get::<_, Option<u8>>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, f64>(5)?,
                    row.get::<_, u64>(6)?,
                    row.get::<_, u64>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<f64>>(10)?,
                ))
            },
        )
        .optional()
        .map_err(crate::errors::DatabaseError::from)?;
    let (raw_held, cost, invested, proceeds, realized_cost, buys, sells, is_open, opened_at, peak) =
        match current {
            Some((
                Some(held),
                Some(stored_decimals),
                cost,
                invested,
                proceeds,
                realized_cost,
                buys,
                sells,
                closed,
                opened_at,
                peak,
            )) => {
                let is_open = closed.is_none();
                if is_open && stored_decimals != decimals {
                    return Err(Error::CopyValidation {
                        detail: "paper buy token decimals differ from the open holding".to_owned(),
                    });
                }
                let raw = if is_open {
                    held.checked_add(raw_output)
                        .ok_or_else(|| Error::CopyValidation {
                            detail: "paper token balance overflow".to_owned(),
                        })?
                } else {
                    raw_output
                };
                (
                    raw,
                    cost,
                    invested,
                    proceeds,
                    realized_cost,
                    buys,
                    sells,
                    is_open,
                    opened_at,
                    peak,
                )
            }
            Some(_) => {
                return Err(Error::CopyValidation {
                    detail: "active paper holding lacks canonical raw balance".to_owned(),
                })
            }
            None => (
                raw_output,
                0.0,
                0.0,
                0.0,
                0.0,
                0,
                0,
                false,
                at.clone(),
                None,
            ),
        };
    if [cost, invested, proceeds, realized_cost]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(Error::CopyValidation {
            detail: "active paper cost ledger is invalid".to_owned(),
        });
    }
    let next_cost = if is_open {
        cost + decision.fill.total_cost_sol
    } else {
        decision.fill.total_cost_sol
    };
    let next_invested = invested + decision.fill.total_cost_sol;
    if !next_cost.is_finite() || !next_invested.is_finite() {
        return Err(Error::CopyValidation {
            detail: "paper buy cost is not finite".to_owned(),
        });
    }
    transaction
        .execute(
            "INSERT INTO copy_paper_positions (task_id, mint, token_amount, token_amount_raw, token_decimals, \
             cost_basis_native, invested_native, realized_proceeds_native, realized_cost_native, buys, sells, \
             last_price_native, last_price_at, opened_at, updated_at, peak_price_native, closed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?13, ?15, NULL) \
             ON CONFLICT(task_id, mint) DO UPDATE SET token_amount=excluded.token_amount, \
             token_amount_raw=excluded.token_amount_raw, token_decimals=excluded.token_decimals, \
             cost_basis_native=excluded.cost_basis_native, invested_native=excluded.invested_native, \
             buys=excluded.buys, last_price_native=excluded.last_price_native, \
             last_price_at=excluded.last_price_at, opened_at=excluded.opened_at, \
             peak_price_native=excluded.peak_price_native, closed_at=NULL, updated_at=excluded.updated_at",
            params![
                decision.task_id,
                decision.mint,
                raw_held.to_whole_units(decimals),
                raw_held,
                decimals,
                next_cost,
                next_invested,
                proceeds,
                realized_cost,
                buys + 1,
                sells,
                decision.fill.market_price_sol,
                at,
                if is_open { opened_at } else { at.clone() },
                if is_open { peak.unwrap_or(decision.fill.market_price_sol).max(decision.fill.market_price_sol) } else { decision.fill.market_price_sol },
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}

/// Replay only pre-v8 float-based decisions before the active raw columns exist.
pub(super) fn apply_historical_paper_buy(
    transaction: &Transaction<'_>,
    decision: &PaperDecision,
) -> crate::trader::Result<()> {
    let at = decision.telemetry.decided_at.to_rfc3339();
    transaction.execute(
        "INSERT INTO copy_paper_positions (task_id, mint, token_amount, cost_basis_native, invested_native, \
         buys, last_price_native, last_price_at, opened_at, updated_at, peak_price_native) \
         VALUES (?1, ?2, ?3, ?4, ?4, 1, ?5, ?6, ?6, ?6, ?5) \
         ON CONFLICT(task_id, mint) DO UPDATE SET token_amount=token_amount+excluded.token_amount, \
         cost_basis_native=cost_basis_native+excluded.cost_basis_native, \
         invested_native=invested_native+excluded.invested_native, buys=buys+1, \
         last_price_native=excluded.last_price_native, last_price_at=excluded.last_price_at, \
         opened_at=CASE WHEN closed_at IS NULL THEN opened_at ELSE excluded.opened_at END, \
         peak_price_native=CASE WHEN closed_at IS NULL \
         THEN MAX(COALESCE(peak_price_native, excluded.peak_price_native), excluded.peak_price_native) \
         ELSE excluded.peak_price_native END, closed_at=NULL, updated_at=excluded.updated_at",
        params![decision.task_id, decision.mint, decision.fill.token_amount,
            decision.fill.total_cost_sol, decision.fill.market_price_sol, at],
    ).map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}

/// Book an exact raw sell; only a zero raw remainder closes the holding.
pub(super) fn apply_paper_sell(
    transaction: &Transaction<'_>,
    task_id: i64,
    mint: &str,
    fill: &PaperSellFill,
    at: DateTime<Utc>,
) -> crate::trader::Result<()> {
    let metadata = fill
        .quote
        .as_ref()
        .filter(|_| fill.model == PaperModel::Quote)
        .ok_or_else(|| Error::CopyValidation {
            detail: "paper sell lacks quote provenance".to_owned(),
        })?;
    let sold = metadata.input_amount.ok_or_else(|| Error::CopyValidation {
        detail: "paper sell lacks raw input amount".to_owned(),
    })?;
    let decimals = metadata
        .token_decimals
        .ok_or_else(|| Error::CopyValidation {
            detail: "paper sell lacks token decimals".to_owned(),
        })?;
    if sold == RawAmount::ZERO
        || metadata.final_output_amount == RawAmount::ZERO
        || metadata.minimum_output_amount == RawAmount::ZERO
        || metadata.minimum_output_amount > metadata.final_output_amount
        || fill.token_amount.to_bits() != sold.to_whole_units(decimals).to_bits()
        || metadata.router_id.trim().is_empty()
        || !metadata.network_fee_estimated
        || !metadata.platform_fee_included_in_output
    {
        return Err(Error::CopyValidation {
            detail: "paper sell raw amount does not match its display amount".to_owned(),
        });
    }
    let Some((Some(held), Some(stored_decimals), cost_basis)) = transaction
        .query_row(
            "SELECT token_amount_raw, token_decimals, cost_basis_native FROM copy_paper_positions \
             WHERE task_id = ?1 AND mint = ?2 AND closed_at IS NULL",
            params![task_id, mint],
            |row| {
                Ok((
                    row.get::<_, Option<RawAmount>>(0)?,
                    row.get::<_, Option<u8>>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(crate::errors::DatabaseError::from)?
    else {
        return Err(Error::CopyValidation {
            detail: "paper sell has no active raw holding".to_owned(),
        });
    };
    if stored_decimals != decimals || held == RawAmount::ZERO || sold > held {
        return Err(Error::CopyValidation {
            detail: "paper sell raw amount or decimals do not match the active holding".to_owned(),
        });
    }
    if !cost_basis.is_finite()
        || cost_basis < 0.0
        || !fill.market_price_sol.is_finite()
        || fill.market_price_sol <= 0.0
    {
        return Err(Error::CopyValidation {
            detail: "paper sell cost basis or price is invalid".to_owned(),
        });
    }
    let remaining = held
        .checked_sub(sold)
        .ok_or_else(|| Error::CopyValidation {
            detail: "paper sell exceeds raw holding".to_owned(),
        })?;
    let closes = remaining == RawAmount::ZERO;
    let cost_sold = if closes {
        cost_basis
    } else {
        let share = sold.raw() as f64 / held.raw() as f64;
        let allocated = cost_basis * share;
        if !share.is_finite()
            || !(0.0..=1.0).contains(&share)
            || !allocated.is_finite()
            || allocated < 0.0
            || allocated > cost_basis
        {
            return Err(Error::CopyValidation {
                detail: "paper sell cost allocation is invalid".to_owned(),
            });
        }
        allocated
    };
    if !fill.net_proceeds_sol.is_finite() || fill.net_proceeds_sol <= 0.0 {
        return Err(Error::CopyValidation {
            detail: "paper sell proceeds are invalid".to_owned(),
        });
    }
    let at = at.to_rfc3339();
    transaction
        .execute(
            "UPDATE copy_paper_positions SET token_amount=?3, token_amount_raw=?4, token_decimals=?5, \
             cost_basis_native=?6, realized_proceeds_native=realized_proceeds_native+?7, \
             realized_cost_native=realized_cost_native+?8, sells=sells+1, last_price_native=?9, \
             last_price_at=?10, closed_at=?11, updated_at=?10 \
             WHERE task_id = ?1 AND mint = ?2",
            params![
                task_id,
                mint,
                remaining.to_whole_units(decimals),
                remaining,
                decimals,
                if closes { 0.0 } else { cost_basis - cost_sold },
                fill.net_proceeds_sol,
                cost_sold,
                fill.market_price_sol,
                at,
                closes.then_some(at.as_str()),
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}
