// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper book: its open rounds in memory over `paper_positions.db`, each fill booked
//! with the live booking math in one transaction under the round's lock.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};

use crate::positions::booking::{CloseFill, DcaFill, EntryFill, PartialExitFill};
use crate::positions::db::{
    Booking, BookingRecord, Committed, PaperExitOwner, PaperLegKind, PaperLegRecord,
    PaperRoundInsert,
};
use crate::positions::types::{
    Book, EntryRecord, ExitRecord, Opening, Position, PositionManagement, PriceSource,
};
use crate::positions::{marks, Error, PositionsDatabase, Result};

use super::leg::PaperLeg;

/// What a booking onto a paper round did.
#[derive(Debug, Clone)]
pub enum PaperBooked {
    /// The fill was booked; the round as written.
    Booked(Position),
    /// The round was already closed; nothing was written.
    AlreadyClosed,
    /// This leg was already booked onto the round; nothing was written.
    AlreadyBooked,
    /// The take-profit step the sale takes is not the round's next one; nothing was
    /// written.
    StepTaken,
}

/// What opening a paper round did.
#[derive(Debug, Clone)]
pub enum PaperOpened {
    /// The round was opened; the round as written.
    Opened(Position),
    /// The entry leg was already booked; the round it opened, as stored, and nothing was
    /// written.
    AlreadyBooked(Position),
}

/// What a paper round booked: its entry and exit records and the leg evidence of each.
#[derive(Debug, Clone)]
pub struct PaperRoundHistory {
    pub entries: Vec<EntryRecord>,
    pub exits: Vec<ExitRecord>,
    pub legs: Vec<PaperLegRecord>,
}

/// What a new paper round must fit within, checked under the book's capacity lock.
#[derive(Debug, Clone, Copy)]
pub struct PaperOpenTerms {
    /// Open rounds at which a new one is refused.
    pub max_open: usize,
    /// Seconds after the latest paper open before the next one may open.
    pub open_cooldown_secs: u64,
    /// The paper run the buy was priced in; the round is not booked into a later one.
    pub epoch: i64,
}

/// Why a booking wrote nothing.
#[derive(Debug, Clone, Copy)]
enum Skipped {
    Closed,
    AlreadyBooked,
    StepTaken,
}

/// The paper book of one wallet on one chain.
pub struct PaperBook {
    pub(super) db: PositionsDatabase,
    open: RwLock<Vec<Position>>,
    round_locks: StdMutex<HashMap<i64, Arc<Mutex<()>>>>,
    /// Held across every check and write that spends or resets the run's cash: a new
    /// round, an add, and a run reset. Taken before a round's lock.
    pub(super) capacity: Mutex<()>,
    /// When the latest paper round opened; written under the capacity lock.
    last_open: StdMutex<Option<DateTime<Utc>>>,
}

impl PaperBook {
    /// The book over `db` with `open` as its open rounds.
    pub(crate) fn new(db: PositionsDatabase, open: Vec<Position>) -> Self {
        debug_assert_eq!(db.book(), Book::Paper);
        Self {
            db,
            open: RwLock::new(open),
            round_locks: StdMutex::new(HashMap::new()),
            capacity: Mutex::new(()),
            last_open: StdMutex::new(None),
        }
    }

    /// The chain whose paper rounds this book holds.
    pub fn chain(&self) -> crate::chains::ChainId {
        self.db.chain()
    }

    /// When the latest paper round opened in this process. The live open time is not
    /// read or written by the paper book.
    pub fn last_open(&self) -> Option<DateTime<Utc>> {
        *self
            .last_open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The open paper rounds.
    pub async fn open_positions(&self) -> Vec<Position> {
        self.open.read().await.clone()
    }

    /// True while `mint` has an open paper round.
    pub async fn holds_open_round(&self, mint: &str) -> bool {
        self.open.read().await.iter().any(|p| p.mint == mint)
    }

    /// The open paper round `position_id`.
    pub async fn position(&self, position_id: i64) -> Option<Position> {
        self.open
            .read()
            .await
            .iter()
            .find(|p| p.id == Some(position_id))
            .cloned()
    }

    /// The open paper round in `mint`.
    pub async fn open_round(&self, mint: &str) -> Option<Position> {
        self.open
            .read()
            .await
            .iter()
            .find(|p| p.mint == mint)
            .cloned()
    }

    /// The latest paper round of `mint`, open or closed.
    pub async fn latest_by_mint(&self, mint: &str) -> Result<Option<Position>> {
        self.db.get_latest_position_by_mint(mint).await
    }

    /// The paper rounds closed at or after `since`, newest first.
    pub async fn closed_since(&self, since: DateTime<Utc>) -> Result<Vec<Position>> {
        self.db.get_closed_positions_since(since).await
    }

    /// The closed paper rounds of the active wallet, newest exit first.
    pub async fn closed_positions(&self) -> Result<Vec<Position>> {
        self.db.get_closed_positions().await
    }

    /// The entries, exits and legs of paper round `position_id` of `wallet_address`.
    pub async fn round_history(
        &self,
        wallet_address: &str,
        position_id: i64,
    ) -> Result<PaperRoundHistory> {
        Ok(PaperRoundHistory {
            entries: self.db.entry_history(wallet_address, position_id)?,
            exits: self.db.exit_history(wallet_address, position_id)?,
            legs: self.db.paper_legs(position_id)?,
        })
    }

    /// The paper round `position_id`, open or closed.
    pub async fn stored(&self, position_id: i64) -> Result<Option<Position>> {
        self.db.get_position_by_id(position_id).await
    }

    /// Records a paper exit attempt of `position_id` decided by `strategy_id`, and
    /// whether any rung of the exit slippage ladder filled it.
    pub(crate) async fn record_exit_attempt(
        &self,
        position_id: i64,
        strategy_id: Option<&str>,
        filled: bool,
    ) -> Result<()> {
        self.db
            .insert_paper_exit_attempt(position_id, strategy_id, filled, Utc::now())
    }

    /// The latest paper exits of the rounds of `wallet_address` that `owner` took part in
    /// that found no fill in a row.
    pub async fn failed_exit_streak(
        &self,
        wallet_address: &str,
        owner: &PaperExitOwner,
    ) -> Result<u32> {
        self.db.failed_paper_exit_streak(wallet_address, owner)
    }

    /// The stored paper leg `leg_ref` and the round it was booked onto.
    pub async fn paper_leg(&self, leg_ref: &str) -> Result<Option<(i64, PaperLegRecord)>> {
        self.db.paper_leg(leg_ref)
    }

    /// Sets who manages the open round `position_id`, in the store and in memory, under
    /// the round's lock. Returns false when no open round has that id.
    pub async fn set_management(
        &self,
        position_id: i64,
        management: PositionManagement,
    ) -> Result<bool> {
        let _round = self.lock_round(position_id).await;
        if self.position(position_id).await.is_none() {
            return Ok(false);
        }
        if !self
            .db
            .set_position_management(position_id, management)
            .await?
        {
            return Ok(false);
        }
        if let Some(open) = self
            .open
            .write()
            .await
            .iter_mut()
            .find(|p| p.id == Some(position_id))
        {
            open.management = management;
        }
        Ok(true)
    }

    /// Opens a paper round of `wallet_address` from `opening` with its entry `fill`
    /// booked, within `terms`: refused with `SlotUnavailable` once `max_open` rounds are
    /// open, `PaperOpenCooldown` within the open cooldown of the latest open,
    /// `PaperRunEnded` once the run the buy was priced in has ended, `PaperCashShort` when
    /// the run's cash does not cover the fill, and `AlreadyOpen` while the mint has an open
    /// round. Every check and the insert hold the book's capacity lock, so concurrent opens
    /// never exceed `max_open`, the cooldown or the cash, and a run reset never interleaves.
    /// A leg already booked opens nothing and returns the round it opened, before any
    /// check, so a replayed buy is never refused for the capacity its own round takes.
    pub(crate) async fn open(
        &self,
        wallet_address: &str,
        opening: Opening,
        fill: &EntryFill,
        leg: &PaperLeg,
        terms: &PaperOpenTerms,
    ) -> Result<PaperOpened> {
        let _capacity = self.capacity.lock().await;
        if let Some((position_id, _)) = self.db.paper_leg(&leg.leg_ref)? {
            return self.booked_round(position_id).await;
        }
        if self.open.read().await.len() >= terms.max_open {
            return Err(Error::SlotUnavailable { remaining: 0 });
        }
        let now = Utc::now();
        if let Some(wait_secs) = crate::trader::admission::open_cooldown_wait_secs(
            self.last_open(),
            now,
            terms.open_cooldown_secs,
        ) {
            return Err(Error::PaperOpenCooldown { wait_secs });
        }
        self.covering_account(
            wallet_address,
            &opening.mint,
            fill.native_size + self.fee_native(fill.fee_raw),
            Some(terms.epoch),
        )?;

        let mut position = Position::opening(Opening {
            entry_signature: None,
            book: Book::Paper,
            ..opening
        });
        position.apply_entry_fill(fill);
        let entry = EntryRecord {
            id: None,
            position_id: 0,
            timestamp: leg.at,
            amount: fill.token_amount,
            price: fill.effective_entry_price,
            native_spent: fill.native_size,
            transaction_signature: leg.leg_ref.clone(),
            is_dca: false,
            fees_raw: Some(fill.fee_raw),
        };
        let row = leg.row(PaperLegKind::Entry);
        let position_id =
            match self
                .db
                .insert_paper_round(wallet_address, &position, entry, &row)?
            {
                PaperRoundInsert::Inserted(position_id) => position_id,
                PaperRoundInsert::AlreadyBooked(position_id) => {
                    return self.booked_round(position_id).await;
                }
            };
        position.id = Some(position_id);
        self.open.write().await.push(position.clone());
        *self
            .last_open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(now);
        Ok(PaperOpened::Opened(position))
    }

    /// The round `position_id` an already booked entry leg opened, as stored.
    async fn booked_round(&self, position_id: i64) -> Result<PaperOpened> {
        self.db
            .get_position_by_id(position_id)
            .await?
            .map(PaperOpened::AlreadyBooked)
            .ok_or(Error::NotFoundById { position_id })
    }

    /// Books a paper DCA buy filled at `fill_price` onto `position_id`, blending the
    /// average entry price with the held amount in a token of `decimals`. Refused with
    /// `PaperCashShort` when the run's cash does not cover the fill, checked under the
    /// capacity lock with every other spend of the run.
    pub(crate) async fn dca(
        &self,
        wallet_address: &str,
        position_id: i64,
        fill: &DcaFill,
        fill_price: f64,
        decimals: u8,
        leg: &PaperLeg,
    ) -> Result<PaperBooked> {
        let _capacity = self.capacity.lock().await;
        let _round = self.lock_round(position_id).await;
        if let Some(position) = self.position(position_id).await {
            self.covering_account(
                wallet_address,
                &position.mint,
                fill.native_spent + self.fee_native(fill.fee_raw),
                None,
            )?;
        }
        let row = leg.row(PaperLegKind::Dca);
        let leg_ref = leg.leg_ref.as_str();
        let committed = self.db.commit_paper_booking(
            position_id,
            wallet_address,
            &row,
            |position, reads| {
                if position.exit_time.is_some() {
                    return Ok(Booking::Skip(Some(Skipped::Closed)));
                }
                if reads.entry_record_exists(leg_ref)? {
                    return Ok(Booking::Skip(Some(Skipped::AlreadyBooked)));
                }
                position.book_dca(fill)?;
                position.blend_average_entry_price(fill.tokens_bought, fill.native_spent, decimals);
                Ok(Booking::Write {
                    record: Some(BookingRecord::Entry(EntryRecord {
                        id: None,
                        position_id,
                        timestamp: fill.dca_time,
                        amount: fill.tokens_bought,
                        price: fill_price,
                        native_spent: fill.native_spent,
                        transaction_signature: leg_ref.to_owned(),
                        is_dca: true,
                        fees_raw: Some(fill.fee_raw),
                    })),
                    outcome: None,
                })
            },
        )?;
        self.adopt(position_id, committed).await
    }

    /// Books a paper partial exit of `percentage` of the held amount onto `position_id`,
    /// taking take-profit step `take_profit_level` when the sale is one. A step that is not
    /// the round's next one is refused, as the live reservation refuses it, so the ladder
    /// never sells one step twice.
    pub(crate) async fn partial(
        &self,
        wallet_address: &str,
        position_id: i64,
        fill: &PartialExitFill,
        percentage: f64,
        take_profit_level: Option<u32>,
        leg: &PaperLeg,
    ) -> Result<PaperBooked> {
        let _round = self.lock_round(position_id).await;
        let row = leg.row(PaperLegKind::Partial);
        let leg_ref = leg.leg_ref.as_str();
        let committed = self.db.commit_paper_booking(
            position_id,
            wallet_address,
            &row,
            |position, reads| {
                if position.exit_time.is_some() {
                    return Ok(Booking::Skip(Some(Skipped::Closed)));
                }
                if reads.exit_record_exists(leg_ref)? {
                    return Ok(Booking::Skip(Some(Skipped::AlreadyBooked)));
                }
                if let Some(level) = take_profit_level {
                    if level.checked_sub(1) != Some(position.take_profit_level) {
                        return Ok(Booking::Skip(Some(Skipped::StepTaken)));
                    }
                    position.take_profit_level = level;
                }
                position.book_partial_exit(fill)?;
                Ok(Booking::Write {
                    record: Some(BookingRecord::Exit(ExitRecord {
                        id: None,
                        position_id,
                        timestamp: leg.at,
                        amount: fill.exit_amount,
                        price: fill.effective_exit_price,
                        native_received: fill.native_received,
                        transaction_signature: leg_ref.to_owned(),
                        is_partial: true,
                        percentage,
                        fees_raw: Some(fill.fee_raw),
                    })),
                    outcome: None,
                })
            },
        )?;
        self.adopt(position_id, committed).await
    }

    /// Closes `position_id` with the paper sale `fill` at the market price `exit_price`
    /// for `reason`. A round already closed is left as it is.
    pub(crate) async fn close(
        &self,
        wallet_address: &str,
        position_id: i64,
        fill: &CloseFill,
        exit_price: f64,
        reason: &str,
        leg: &PaperLeg,
    ) -> Result<PaperBooked> {
        let _round = self.lock_round(position_id).await;
        let row = leg.row(PaperLegKind::Exit);
        let leg_ref = leg.leg_ref.as_str();
        let committed = self.db.commit_paper_booking(
            position_id,
            wallet_address,
            &row,
            |position, reads| {
                if position.exit_time.is_some() {
                    return Ok(Booking::Skip(Some(Skipped::Closed)));
                }
                if reads.exit_record_exists(leg_ref)? {
                    return Ok(Booking::Skip(Some(Skipped::AlreadyBooked)));
                }
                position.exit_price = Some(exit_price);
                position.closed_reason = Some(reason.to_owned());
                let sold = position.book_close(fill)?;
                Ok(Booking::Write {
                    record: Some(BookingRecord::Exit(ExitRecord {
                        id: None,
                        position_id,
                        timestamp: fill.exit_time,
                        amount: sold,
                        price: fill.effective_exit_price,
                        native_received: fill.native_received,
                        transaction_signature: leg_ref.to_owned(),
                        is_partial: false,
                        percentage: 100.0,
                        fees_raw: Some(fill.fee_raw),
                    })),
                    outcome: None,
                })
            },
        )?;
        let booked = self.adopt(position_id, committed).await?;
        if let PaperBooked::Booked(_) | PaperBooked::AlreadyClosed = booked {
            self.round_locks
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&position_id);
        }
        Ok(booked)
    }

    /// Marks the open round `position_id` at `price` from `source`: the mark and the
    /// unrealized P&L in memory, then the price columns of its row.
    pub async fn mark(
        &self,
        position_id: i64,
        price: f64,
        source: PriceSource,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let _round = self.lock_round(position_id).await;
        let Some(mut position) = self.position(position_id).await else {
            return Err(Error::NotFoundById { position_id });
        };
        if !price.is_finite() || price <= 0.0 {
            return Err(Error::InvalidPrice {
                mint: position.mint,
                price,
            });
        }
        marks::apply_mark(&mut position, price, source, now);
        let (pnl, pnl_percent) =
            crate::positions::calculate_position_pnl(&position, Some(price)).await;
        position.unrealized_pnl = Some(pnl);
        position.unrealized_pnl_percent = Some(pnl_percent);
        self.db
            .update_position_prices_and_pnl(
                position_id,
                position.current_price,
                position.current_price_updated,
                position.price_highest,
                position.price_lowest,
                position.basis_peak_price,
                position.average_entry_price,
                position.unrealized_pnl,
                position.unrealized_pnl_percent,
            )
            .await?;
        if let Some(open) = self
            .open
            .write()
            .await
            .iter_mut()
            .find(|p| p.id == Some(position_id))
        {
            *open = position;
        }
        Ok(())
    }

    /// The lock that serializes every booking and mark of one round.
    async fn lock_round(&self, position_id: i64) -> OwnedMutexGuard<()> {
        let lock = self
            .round_locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(position_id)
            .or_default()
            .clone();
        lock.lock_owned().await
    }

    /// Brings memory to a committed booking: an open row replaces its round, keeping the
    /// mark source memory holds, and a closed row leaves the open rounds.
    async fn adopt(
        &self,
        position_id: i64,
        committed: Committed<Option<Skipped>>,
    ) -> Result<PaperBooked> {
        let mut row = match committed {
            Committed::Written { row, .. } => row,
            Committed::Skipped(Some(Skipped::Closed)) | Committed::Deleted { .. } => {
                self.open
                    .write()
                    .await
                    .retain(|p| p.id != Some(position_id));
                return Ok(PaperBooked::AlreadyClosed);
            }
            Committed::Skipped(Some(Skipped::AlreadyBooked) | None) => {
                return Ok(PaperBooked::AlreadyBooked)
            }
            Committed::Skipped(Some(Skipped::StepTaken)) => return Ok(PaperBooked::StepTaken),
        };
        row.basis_peak_restarted = false;
        let mut open = self.open.write().await;
        let held = open.iter().position(|p| p.id == row.id);
        if row.exit_time.is_some() {
            if let Some(index) = held {
                open.remove(index);
            }
        } else if let Some(index) = held {
            row.current_price_source = open[index].current_price_source;
            open[index] = row.clone();
        } else {
            open.push(row.clone());
        }
        Ok(PaperBooked::Booked(row))
    }
}
