// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper account: the epochs of a paper run and the cash of the current one, derived
//! from the positions of the epoch and never stored as a running figure.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::positions::db::PaperEpoch;
use crate::positions::pnl::calculate_position_total_fees;
use crate::positions::types::Position;
use crate::positions::{Error, Result};

use super::book::{PaperBook, PaperBooked};

/// The paper account of a wallet in its current epoch.
#[derive(Debug, Clone, Serialize)]
pub struct PaperAccount {
    pub epoch: i64,
    pub starting_balance_native: f64,
    pub started_at: DateTime<Utc>,
    /// The starting balance with every buy, sale and fee of the epoch's positions.
    pub cash_native: f64,
}

/// The cash of a paper run that started with `starting_balance_native`: every position
/// of the run has paid its buys and fees and received its sales, the same terms its
/// realized P&L is taken on.
pub fn paper_cash(starting_balance_native: f64, positions: &[Position]) -> f64 {
    positions
        .iter()
        .fold(starting_balance_native, |cash, position| {
            cash + position.native_received.unwrap_or_default()
                - position.total_size_native
                - calculate_position_total_fees(position)
        })
}

impl PaperBook {
    /// The account of `wallet_address`, starting its first epoch with
    /// `starting_balance_native` when it has none.
    pub async fn account(
        &self,
        wallet_address: &str,
        starting_balance_native: f64,
    ) -> Result<PaperAccount> {
        let epoch = match self.db.current_paper_epoch(wallet_address)? {
            Some(epoch) => epoch,
            None => {
                self.db
                    .start_paper_epoch(wallet_address, starting_balance_native, Utc::now())?
            }
        };
        self.account_in(wallet_address, epoch)
    }

    /// Starts the next epoch of `wallet_address` with `starting_balance_native`. Rounds
    /// still open stay in the epoch they were entered in.
    pub async fn start_epoch(
        &self,
        wallet_address: &str,
        starting_balance_native: f64,
        now: DateTime<Utc>,
    ) -> Result<PaperAccount> {
        let epoch = self
            .db
            .start_paper_epoch(wallet_address, starting_balance_native, now)?;
        self.account_in(wallet_address, epoch)
    }

    /// Starts a new paper run of `wallet_address` with `starting_balance_native`. Every
    /// round still open is first closed at its last mark for `reason`
    /// ([`PaperBook::settle_at_mark`]), so the run that ends is settled and the new one
    /// starts with all its cash. A round whose decimals are unknown stops the reset before
    /// the new epoch starts. The whole reset holds the capacity lock, so no round opens or
    /// adds between the settled run and the next one. The new run starts at `now`, or just
    /// after the latest round the ending run booked when that is later.
    pub async fn reset_run(
        &self,
        wallet_address: &str,
        starting_balance_native: f64,
        reason: &str,
        decimals_of: impl Fn(&str) -> Option<u8>,
        now: DateTime<Utc>,
    ) -> Result<PaperAccount> {
        let _capacity = self.capacity.lock().await;
        let open = self.open_positions().await;
        self.settle_locked(wallet_address, &open, reason, &decimals_of, now)
            .await?;
        // `now` is read before the capacity lock is taken, so a round booked while the reset
        // waited for it can carry the same or a later entry time. That round belongs to the
        // run that ends; counting it in the new run would charge its fees to that run.
        let started_at = self
            .db
            .positions_entered_since(wallet_address, now)?
            .iter()
            .map(|round| round.entry_time + Duration::nanoseconds(1))
            .fold(now, std::cmp::max);
        self.start_epoch(wallet_address, starting_balance_native, started_at)
            .await
    }

    /// Closes each of `rounds` still open at its last mark (its entry price before the
    /// first mark) for `reason`, with no quote and no fee; their value returns to the run's
    /// cash. `decimals_of` gives the token decimals of a mint; a round whose decimals are
    /// unknown stops the settlement there. Rounds already closed are left as they are.
    /// Returns how many rounds this call closed. Holds the capacity lock, so no round opens
    /// or adds while the rounds settle.
    pub async fn settle_at_mark(
        &self,
        wallet_address: &str,
        rounds: &[Position],
        reason: &str,
        decimals_of: impl Fn(&str) -> Option<u8>,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let _capacity = self.capacity.lock().await;
        self.settle_locked(wallet_address, rounds, reason, &decimals_of, now)
            .await
    }

    async fn settle_locked(
        &self,
        wallet_address: &str,
        rounds: &[Position],
        reason: &str,
        decimals_of: &impl Fn(&str) -> Option<u8>,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let mut closed = 0;
        for position in rounds {
            let Some(decimals) = decimals_of(&position.mint) else {
                return Err(Error::PaperFillUnbookable {
                    mint: position.mint.clone(),
                    detail: "the token decimals are unknown".to_owned(),
                });
            };
            let mark = position
                .current_price
                .filter(|price| price.is_finite() && *price > 0.0)
                .unwrap_or(position.average_entry_price);
            if let PaperBooked::Booked(_) = self
                .close_at_mark(wallet_address, position, mark, decimals, reason, now)
                .await?
            {
                closed += 1;
            }
        }
        Ok(closed)
    }

    /// The rounds of the current epoch of `wallet_address` that strategy `strategy_id`
    /// took part in, open or closed; none before the first epoch starts.
    pub async fn strategy_rounds(
        &self,
        wallet_address: &str,
        strategy_id: &str,
    ) -> Result<Vec<Position>> {
        match self.db.current_paper_epoch(wallet_address)? {
            Some(epoch) => self.db.strategy_positions_entered_since(
                wallet_address,
                strategy_id,
                epoch.started_at,
            ),
            None => Ok(Vec::new()),
        }
    }

    /// The rounds of `wallet_address` that copy task `task_id` opened at or after `since`,
    /// open or closed.
    pub async fn copy_task_rounds(
        &self,
        wallet_address: &str,
        task_id: i64,
        since: DateTime<Utc>,
    ) -> Result<Vec<Position>> {
        self.db
            .copy_task_positions_entered_since(wallet_address, task_id, since)
    }

    /// The current paper epoch of `wallet_address`, without reading its cash; none before
    /// the first epoch starts.
    pub async fn current_epoch(&self, wallet_address: &str) -> Result<Option<PaperEpoch>> {
        self.db.current_paper_epoch(wallet_address)
    }

    /// The account of the current paper run of `wallet_address` when its cash covers a
    /// buy of `mint` costing `needed_native`. Refused with `PaperRunEnded` before any run
    /// started or when the current run is not `priced_in`, the run the buy was priced in,
    /// and with `PaperCashShort` when the cash falls short. Called under the capacity
    /// lock, so the cash it reads is the cash the booking spends.
    pub(super) fn covering_account(
        &self,
        wallet_address: &str,
        mint: &str,
        needed_native: f64,
        priced_in: Option<i64>,
    ) -> Result<PaperAccount> {
        let run_ended = || Error::PaperRunEnded {
            mint: mint.to_owned(),
        };
        let epoch = self
            .db
            .current_paper_epoch(wallet_address)?
            .ok_or_else(run_ended)?;
        if priced_in.is_some_and(|priced_in| priced_in != epoch.epoch) {
            return Err(run_ended());
        }
        let account = self.account_in(wallet_address, epoch)?;
        if account.cash_native + f64::EPSILON < needed_native {
            return Err(Error::PaperCashShort {
                mint: mint.to_owned(),
                cash_native: account.cash_native,
                needed_native,
            });
        }
        Ok(account)
    }

    /// A fee of `fee_raw` raw units of the book's native asset, in the native asset.
    pub(super) fn fee_native(&self, fee_raw: u64) -> f64 {
        crate::chains::adapter_for(self.db.chain()).raw_to_native(fee_raw)
    }

    fn account_in(&self, wallet_address: &str, epoch: PaperEpoch) -> Result<PaperAccount> {
        let positions = self
            .db
            .positions_entered_since(wallet_address, epoch.started_at)?;
        Ok(PaperAccount {
            epoch: epoch.epoch,
            starting_balance_native: epoch.starting_balance_native,
            started_at: epoch.started_at,
            cash_native: paper_cash(epoch.starting_balance_native, &positions),
        })
    }
}
