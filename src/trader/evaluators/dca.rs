// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The DCA ladder: decides when an open position takes its next automatic add, and how
//! much each add buys.
//!
//! Add `k` (the position's `dca_count`) follows step `k` of the configured ladder. It
//! fires only when the ladder has that step, the price dropped below the step's level
//! from the configured basis (the average entry or the first buy's fill), the wait after
//! entry (first add only) and the wait after the last add passed, no add of the mint is
//! in flight, the position is the auto trader's, the pool price is fresh, and the loss
//! limit does not block entries. The add buys the step's multiple of the first buy.
//!
//! [`DcaEvaluation::evaluate`] is pure: the live inputs it needs (an add in flight, the
//! loss-limit gate, the clock) arrive in [`DcaContext`]. The evaluator reads pool prices
//! only, never OHLCV candles.

use crate::config::{DcaLevel, DcaMeasureFrom};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::positions::{Book, Position};
use crate::trader::constants::{DCA_PRICE_MAX_AGE_SECS, MIN_TRADE_SIZE_NATIVE};
use crate::trader::policy::ExitPolicy;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority};
use crate::trader::TradeReason;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration snapshot for DCA evaluation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DcaConfigSnapshot {
    pub enabled: bool,
    /// One step per automatic add, in firing order.
    pub levels: Vec<DcaLevel>,
    pub measure_from: DcaMeasureFrom,
    pub cooldown_minutes: i64,
    /// Minutes after the entry before the first add may fire.
    pub min_hold_minutes: i64,
}

/// The live inputs of one evaluation, read by the caller so the decision stays pure.
#[derive(Debug, Clone, Copy)]
pub struct DcaContext {
    /// An add of the position's mint is sent and not yet settled: the count and the
    /// cooldown it would move are not booked yet, so no further add is decided on them.
    pub add_in_flight: bool,
    /// The loss limit blocks new entries; automatic adds stop with them.
    pub entries_blocked: bool,
    pub now: DateTime<Utc>,
}

/// Calculated metrics for DCA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaCalculations {
    pub current_dca_count: u32,
    /// Index of the ladder step the next add follows; `None` once every step is used.
    pub step: Option<usize>,
    pub measure_from: DcaMeasureFrom,
    /// The entry price the drop is measured from; `None` when the position has none.
    pub basis_price: Option<f64>,
    pub current_price: Option<f64>,
    /// Percent move of the current price from the basis (negative below it).
    pub drop_pct: Option<f64>,
    /// The step's level the drop must pass, in percent (negative).
    pub required_drop_pct: Option<f64>,
    /// What the next add buys, in the native coin.
    pub dca_amount_native: Option<f64>,
    pub price_age_secs: Option<i64>,
    pub minutes_since_last: Option<i64>,
    pub minutes_held: i64,
    /// Every gate passed except the minimum trade size.
    pub below_minimum_trade: bool,
}

/// Structured DCA evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaEvaluation {
    pub should_trigger: bool,
    pub reasons: Vec<String>,
    pub config: DcaConfigSnapshot,
    pub calculations: DcaCalculations,
}

/// The first buy's size in the native coin, which every add is a multiple of. `None` for
/// a wallet-derived round, whose recorded entry size is the round's whole investment
/// rather than one buy, and for a size that is not a positive finite number.
fn first_buy_native(position: &Position) -> Option<f64> {
    if position.is_wallet_derived() {
        return None;
    }
    let size = position.entry_size_native;
    (size.is_finite() && size > 0.0).then_some(size)
}

/// The size of a manual add to `position` that names none, the amount the dashboard
/// presets: the next ladder step's multiple of the first buy, or the first buy itself
/// once every step is used. A wallet-derived round has no first buy, so it takes
/// `trade_size_native`, the configured trade size; the automatic add never uses that
/// stand-in and refuses such a round instead. `None` when the chosen size is not a
/// positive finite number.
pub fn default_add_size_native(
    position: &Position,
    levels: &[DcaLevel],
    trade_size_native: f64,
) -> Option<f64> {
    let size = match first_buy_native(position) {
        Some(first_buy) => {
            let step = usize::try_from(position.dca_count)
                .ok()
                .and_then(|k| levels.get(k));
            step.map_or(first_buy, |level| level.add_size_native(first_buy))
        }
        None if position.is_wallet_derived() => trade_size_native,
        None => return None,
    };
    (size.is_finite() && size > 0.0).then_some(size)
}

/// The entry price `basis` names: the average entry, or the verified fill of the first
/// buy. `None` when that price is missing or not a positive finite number; the first
/// buy's fill never falls back to another price.
fn basis_price(position: &Position, basis: DcaMeasureFrom) -> Option<f64> {
    let price = match basis {
        DcaMeasureFrom::Average => Some(position.average_entry_price),
        DcaMeasureFrom::FirstEntry => position.effective_entry_price,
    };
    price.filter(|price| price.is_finite() && *price > 0.0)
}

impl DcaEvaluation {
    /// Evaluate whether the next automatic add of `position` fires.
    pub fn evaluate(
        position: &Position,
        config: DcaConfigSnapshot,
        context: DcaContext,
    ) -> crate::trader::Result<Self> {
        let mut reasons = Vec::new();

        let step = usize::try_from(position.dca_count)
            .ok()
            .filter(|k| *k < config.levels.len());
        let level = step.and_then(|k| config.levels.get(k)).copied();
        let current_price = position
            .current_price
            .filter(|price| price.is_finite() && *price > 0.0);
        let basis_price = basis_price(position, config.measure_from);
        let drop_pct = basis_price
            .zip(current_price)
            .map(|(basis, current)| ((current - basis) / basis) * 100.0);
        let dca_amount_native = level
            .zip(first_buy_native(position))
            .map(|(level, first_buy)| level.add_size_native(first_buy));
        let price_age_secs = position
            .current_price_updated
            .map(|updated| (context.now - updated).num_seconds());
        let minutes_since_last = position
            .last_dca_time
            .map(|last| (context.now - last).num_minutes());
        // An entry time ahead of the evaluator's clock counts as no time held.
        let minutes_held = (context.now - position.entry_time).num_minutes().max(0);

        // Auto-DCA belongs exclusively to AutoTrader ownership. Copy/Hybrid positions
        // follow their source task and UserOnly positions remain under the user.
        if !position.management.allows_auto_dca() {
            reasons.push(format!(
                "Position management '{}' does not allow auto-DCA",
                position.management.as_str()
            ));
        }
        if !config.enabled {
            reasons.push("DCA disabled in config".to_owned());
        }
        if context.add_in_flight {
            reasons.push("A DCA add of this token is still in flight".to_owned());
        }
        if context.entries_blocked {
            reasons.push("The loss limit blocks entries".to_owned());
        }
        match level {
            None => reasons.push(format!(
                "Every DCA step is used ({}/{})",
                position.dca_count,
                config.levels.len()
            )),
            Some(level) => match drop_pct {
                // A drop strictly below the step's level fires the add.
                Some(drop) if drop < level.drop_pct => {}
                Some(drop) => reasons.push(format!(
                    "Price drop insufficient: {drop:.2}% from the {} (need < {:.2}%)",
                    config.measure_from.as_str(),
                    level.drop_pct
                )),
                None if current_price.is_none() => {
                    reasons.push("No valid current price".to_owned());
                }
                None => reasons.push(format!(
                    "No valid {} price to measure the drop from",
                    config.measure_from.as_str()
                )),
            },
        }
        if position.dca_count == 0
            && config.min_hold_minutes > 0
            && minutes_held < config.min_hold_minutes
        {
            reasons.push(format!(
                "Held {minutes_held}/{} minutes before the first add",
                config.min_hold_minutes
            ));
        }
        if let Some(minutes) = minutes_since_last {
            if minutes < config.cooldown_minutes {
                reasons.push(format!(
                    "DCA cooldown active ({minutes}/{} minutes)",
                    config.cooldown_minutes
                ));
            }
        }
        match price_age_secs {
            Some(age) if age <= DCA_PRICE_MAX_AGE_SECS => {}
            Some(age) => reasons.push(format!(
                "Price is {age}s old (fresh within {DCA_PRICE_MAX_AGE_SECS}s)"
            )),
            None => reasons.push("Price has no update time".to_owned()),
        }

        let mut below_minimum_trade = false;
        match dca_amount_native {
            Some(amount) if amount >= MIN_TRADE_SIZE_NATIVE => {}
            Some(amount) => {
                below_minimum_trade = reasons.is_empty();
                reasons.push(format!(
                    "DCA amount {amount:.6} SOL below minimum {MIN_TRADE_SIZE_NATIVE:.6} SOL"
                ));
            }
            None if level.is_some() => {
                reasons.push("No first buy size to size the add from".to_owned());
            }
            None => {}
        }

        let should_trigger = reasons.is_empty();
        if let (true, Some(level), Some(drop)) = (should_trigger, level, drop_pct) {
            reasons.push(format!(
                "DCA step {} triggered: {drop:.2}% from the {} passes {:.2}%",
                position.dca_count + 1,
                config.measure_from.as_str(),
                level.drop_pct
            ));
        }

        let calculations = DcaCalculations {
            current_dca_count: position.dca_count,
            step,
            measure_from: config.measure_from,
            basis_price,
            current_price,
            drop_pct,
            required_drop_pct: level.map(|level| level.drop_pct),
            dca_amount_native,
            price_age_secs,
            minutes_since_last,
            minutes_held,
            below_minimum_trade,
        };

        Ok(Self {
            should_trigger,
            reasons,
            config,
            calculations,
        })
    }

    /// Get a human-readable summary of the evaluation
    pub fn summary(&self) -> String {
        match (
            self.should_trigger,
            self.calculations.drop_pct,
            self.calculations.dca_amount_native,
        ) {
            (true, Some(drop), Some(amount)) => format!(
                "DCA #{}: {drop:.2}% from the {}, amount: {amount:.4} SOL",
                self.calculations.current_dca_count + 1,
                self.calculations.measure_from.as_str()
            ),
            _ => self.reasons.join(", "),
        }
    }
}

/// Process DCA opportunities for the eligible open positions of `book`
pub async fn process_dca_opportunities(book: Book) -> crate::trader::Result<Vec<TradeDecision>> {
    // Build config snapshot (batch read)
    let dca_config = ExitPolicy::from_config().dca;

    // Early exit if DCA is disabled
    if !dca_config.enabled {
        return Ok(Vec::new());
    }

    if positions::books::awaiting_reconciliation(book) {
        logger::debug(
            LogTag::Trader,
            "DCA evaluation skipped: the positions store is not reconciled",
        );
        return Ok(Vec::new());
    }

    // Get all open positions of the book
    let open_positions = positions::books::open_positions(book).await;
    if open_positions.is_empty() {
        return Ok(Vec::new());
    }

    // The same gate entry admission applies, read once for the whole cycle. The loss
    // limiter protects real funds and never pauses paper.
    let entries_blocked =
        book == Book::Live && crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit();
    let mut dca_decisions = Vec::new();

    for position in open_positions {
        if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
            continue;
        }

        // Skip if position doesn't have ID
        let position_id = match position.id {
            Some(id) => id,
            None => continue,
        };

        let context = DcaContext {
            add_in_flight: positions::books::dca_pending(book, &position.mint).await,
            entries_blocked,
            now: Utc::now(),
        };
        let evaluation = match DcaEvaluation::evaluate(&position, dca_config.clone(), context) {
            Ok(eval) => eval,
            Err(e) => {
                logger::error(
                    LogTag::Trader,
                    &format!("DCA evaluation failed for {}: {}", position.symbol, e),
                );
                continue;
            }
        };

        let calculations = &evaluation.calculations;
        match (
            evaluation.should_trigger,
            calculations.current_price,
            calculations.dca_amount_native,
        ) {
            (true, Some(price), Some(amount)) => {
                logger::info(
                    LogTag::Trader,
                    &format!(
                        "DCA opportunity: {} | {}",
                        position.symbol,
                        evaluation.summary()
                    ),
                );

                dca_decisions.push(TradeDecision {
                    position_id: Some(position_id.to_string()),
                    mint: position.mint.clone(),
                    action: TradeAction::DCA,
                    reason: TradeReason::DCAScheduled,
                    strategy_id: None,
                    book: position.book,
                    timestamp: Utc::now(),
                    priority: TradePriority::Normal,
                    price_native: Some(price),
                    size_native: Some(amount),
                    exit_percentage: None,
                    // Auto-trader slippage always follows config.
                    slippage_pct: None,
                    take_profit_level: None,
                    paper_leg_key: None,
                });
            }
            _ if calculations.below_minimum_trade => logger::warning(
                LogTag::Trader,
                &format!(
                    "DCA add refused for {}: {}",
                    position.symbol,
                    evaluation.summary()
                ),
            ),
            _ => logger::debug(
                LogTag::Trader,
                &format!(
                    "DCA not triggered for {}: {}",
                    position.symbol,
                    evaluation.summary()
                ),
            ),
        }
    }

    if !dca_decisions.is_empty() {
        logger::info(
            LogTag::Trader,
            &format!("Found {} DCA opportunities", dca_decisions.len()),
        );
    }

    Ok(dca_decisions)
}
