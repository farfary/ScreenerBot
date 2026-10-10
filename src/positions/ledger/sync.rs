// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Materialise reduced wallet-history rounds as `positions` rows.
//!
//! [`reduce_rounds`](super::reduce_rounds) answers "what rounds does this wallet's
//! history contain"; this module answers "which position rows must exist so the
//! dashboard shows them". It deliberately splits into a PURE planner
//! ([`plan_position_writes`]) and a thin impure applier ([`sync_wallet_history`]), so
//! every rule about what a wallet-derived row may claim is testable without a database,
//! a wallet or a clock.
//!
//! # One row per round
//!
//! A round is a fact about the wallet, not about who executed it, so exactly ONE
//! `positions` row may represent it — otherwise a bot-executed buy shows up twice, once
//! as the trader's row and once as an imported round, and every portfolio total counts
//! it twice. A round therefore claims an existing row before it ever inserts one:
//!
//! 1. by `round_key`, the identity stamped on the row by an earlier sync;
//! 2. otherwise by the `entry_transaction_signature` of a row that has no round key yet
//!    and holds the same mint — the trader's own row for the very swap this round was
//!    reduced from. Claiming it is called ADOPTION, and it happens at most once per row.
//!
//! A mint has one open position, and every buy of its round is booked onto it. An open bot
//! row belongs to its home round, the latest round holding one of its own legs, and no
//! other round claims it. A round either step matched to a CLOSED row therefore belongs to
//! the open row homed there instead: a position written off while its entry was in flight,
//! whose late entry was handed to the open row, shares the open row's round and must not
//! hold it. The closed row stays as it is settled, and gives up the round key so the open
//! row can carry it.
//!
//! # Ownership boundary
//!
//! What the ledger may write depends on who owns the row.
//!
//! An [`PositionOrigin::External`] row is the ledger's own: it owns the *history* fields
//! (amounts, basis, realized P&L, signatures, open/closed) and nothing else. Live-price
//! fields (`current_price`, `price_highest`, `price_lowest`, unrealized P&L) belong to
//! the price updater, and `archived` belongs to the user — a resync must never move a
//! row the user archived back into the open list, and must never archive one on its own.
//!
//! A row the bot executed (Auto/Manual/Copy) keeps its origin, management and strategy
//! targets forever, and keeps the fee-exact SOL it booked for the legs IT executed. The
//! ledger RECONCILES it against the chain: it stamps the round key, flags a frozen
//! account, follows the holding up as well as down, and closes a position whose token
//! was sold somewhere else. Following it UP is what makes a buy the user made in another
//! wallet app show up — it is the same round, so it is the same row, and the row's basis
//! becomes the trader's booked SOL plus the chain's SOL for every leg the trader did not
//! book (see [`TraderLegs`]). A round that another position of the mint booked part of
//! charges the row only its own part, and a closed row is never resized. Nothing else the
//! trader owns is rewritten, and a row with work in flight (an unverified entry, a pending
//! DCA or partial exit) is left completely alone until that work lands — including not
//! being duplicated.
//!
//! Freezing is a FLAG, never an action: a frozen token account sets
//! `holding_state = "frozen"` so the user can see the holding cannot be sold and archive
//! it if they choose. Nothing here archives, hides or closes a frozen position.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::{
    reconcile_with_wallet, reduce_rounds, LedgerEvent, LedgerEventKind, LedgerRound, QuoteAsset,
    WalletHolding, DUST,
};
use crate::chains::RawAmount;
use crate::logger::{self, LogTag};
use crate::positions::db::{Booking, Committed, MintRow, TraderSwapLeg};
use crate::positions::pnl::realized_pnl;
use crate::positions::round_state::attributable_held;
use crate::positions::types::{Position, PositionManagement, PositionOrigin, HOLDING_STATE_FROZEN};
use crate::positions::{EXIT_RETRY_PENDING, PENDING_VERIFICATION_SUFFIX};

/// `closed_reason` for a bot-executed position whose token left the wallet through a
/// sale or transfer the bot did not make.
pub const CLOSED_EXTERNALLY: &str = "closed_externally";

/// Whether `reason` is one a wallet-history close leaves on the row it closes: its own
/// label on a row that had none, or a transient trader label it keeps
/// ([`EXIT_RETRY_PENDING`], any [`PENDING_VERIFICATION_SUFFIX`] label). A close the trader
/// books replaces each transient label, so on a closed, verified, non-synthetic row these
/// mean the wallet history closed it.
pub fn is_wallet_history_close_reason(reason: Option<&str>) -> bool {
    reason.is_some_and(|reason| {
        reason == CLOSED_EXTERNALLY
            || reason == EXIT_RETRY_PENDING
            || reason.ends_with(PENDING_VERIFICATION_SUFFIX)
    })
}

/// The swap legs the TRADER itself executed for one position.
///
/// A bot-owned round can also contain acquisitions the user made in another wallet app.
/// Telling the two apart is what lets the row absorb the outside buy — the trader's own
/// legs keep their fee-exact SOL, and only the rest is taken from the chain.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraderLegs {
    /// Signatures of acquisitions the trader booked.
    pub entry_signatures: HashSet<String>,
    /// Signatures of disposals the trader booked.
    pub exit_signatures: HashSet<String>,
    /// SOL those acquisitions cost, fee-exact as the trader recorded it.
    pub booked_invested_native: f64,
    /// Tokens those acquisitions booked, raw units.
    pub booked_acquired: RawAmount,
    /// SOL those disposals received, as the trader recorded it.
    pub booked_received_native: f64,
}

impl TraderLegs {
    /// Build the per-position map from the booked legs, counting each signature once per
    /// position.
    pub fn from_rows(rows: impl IntoIterator<Item = TraderSwapLeg>) -> HashMap<i64, Self> {
        let mut map: HashMap<i64, Self> = HashMap::new();
        for leg in rows {
            let legs = map.entry(leg.position_id).or_default();
            if leg.is_exit {
                if legs.exit_signatures.insert(leg.signature) {
                    legs.booked_received_native += leg.native;
                }
            } else if legs.entry_signatures.insert(leg.signature) {
                legs.booked_invested_native += leg.native;
                legs.booked_acquired = legs
                    .booked_acquired
                    .checked_add(leg.amount)
                    .unwrap_or(RawAmount::MAX);
            }
        }
        map
    }

    /// True when this position booked a leg under `signature`.
    fn booked(&self, signature: &str) -> bool {
        self.entry_signatures.contains(signature) || self.exit_signatures.contains(signature)
    }
}

/// What a bot row's reconciliation may charge it from a round: its own booked legs, and
/// which of the round's signatures other positions of the mint booked.
///
/// A round is a wallet fact, so it can hold the legs of more than one position: a closed
/// position whose round never returned to zero (dust, a write-off whose tokens stayed) and
/// the open position bought into the same holding. Each is charged only its own part.
#[derive(Debug, Clone)]
struct RoundAttribution {
    /// The legs this row booked.
    own: TraderLegs,
    /// This row's entry signature, which identifies its part even without records.
    own_entry_signature: Option<String>,
    /// Every record, entry and exit signature of the mint's other positions.
    booked_elsewhere: HashSet<String>,
    /// What the mint's other open positions hold.
    held_elsewhere: RawAmount,
    /// The row's records cover its entry, every DCA and every partial exit it booked.
    records_complete: bool,
}

impl RoundAttribution {
    /// The attribution of `row` among `mint_rows`, every position of its mint, from the
    /// booked `legs` of each and what the other open positions hold.
    fn new(
        row: &Position,
        mint_rows: &[MintRow],
        legs: &HashMap<i64, TraderLegs>,
        held_elsewhere: RawAmount,
    ) -> Self {
        let mut booked_elsewhere = HashSet::new();
        for other in mint_rows.iter().filter(|other| Some(other.id) != row.id) {
            booked_elsewhere.extend(other.entry_signature.iter().cloned());
            booked_elsewhere.extend(other.exit_signature.iter().cloned());
            if let Some(other_legs) = legs.get(&other.id) {
                booked_elsewhere.extend(other_legs.entry_signatures.iter().cloned());
                booked_elsewhere.extend(other_legs.exit_signatures.iter().cloned());
            }
        }
        let own: TraderLegs = row
            .id
            .and_then(|id| legs.get(&id))
            .cloned()
            .unwrap_or_default();
        let records_complete = row
            .entry_transaction_signature
            .as_ref()
            .is_none_or(|signature| own.entry_signatures.contains(signature))
            && own.entry_signatures.len() >= 1 + row.dca_count as usize
            && own.exit_signatures.len() >= row.partial_exit_count as usize;
        Self {
            own,
            own_entry_signature: row.entry_transaction_signature.clone(),
            booked_elsewhere,
            held_elsewhere,
            records_complete,
        }
    }

    /// True when another position of the mint booked one of the round's signatures.
    fn shares(&self, round: &LedgerRound) -> bool {
        round
            .events
            .iter()
            .any(|event| self.booked_elsewhere.contains(&event.signature))
    }

    /// True when the row is charged only a part of `round`: earlier rounds hold some of its
    /// legs, or another position of the mint booked one of the round's signatures.
    fn charges_part(&self, round: &LedgerRound, earlier: &[LedgerRound]) -> bool {
        !earlier.is_empty() || self.shares(round)
    }

    /// True when `signature` is one of this row's own: a booked leg or its entry.
    fn is_own(&self, signature: &str) -> bool {
        self.own.booked(signature) || self.own_entry_signature.as_deref() == Some(signature)
    }

    /// Where the row's part of `round` starts: its first own event when another position of
    /// the mint booked part of the round, else the round's first event. `None` when such a
    /// round holds none of the row's own events.
    fn part_start(&self, round: &LedgerRound) -> Option<usize> {
        if self.shares(round) {
            round
                .events
                .iter()
                .position(|event| self.is_own(&event.signature))
        } else {
            Some(0)
        }
    }

    /// True when neither this row nor another position of the mint booked `signature`: the
    /// event happened outside the bot.
    fn is_outside(&self, signature: &str) -> bool {
        !self.is_own(signature) && !self.booked_elsewhere.contains(signature)
    }
}

/// A row's part of the rounds holding its legs, when it is charged only a part.
#[derive(Debug, Clone)]
struct RoundShare {
    /// The row's booked acquisitions plus every outside acquisition of its part, raw.
    acquired: RawAmount,
    /// The row's booked SOL plus the SOL of every outside traded acquisition of its part.
    invested_native: f64,
    /// The row's booked entries plus the outside traded acquisitions of its part.
    entries: u32,
    /// The wallet's holding the row can own.
    held: RawAmount,
    has_outside_acquisition: bool,
    /// Every outside acquisition of the part is a trade with a SOL quote, in a round whose
    /// basis is complete.
    basis_known: bool,
    /// Every round of the part reconciles with the balances observed.
    history_complete: bool,
}

/// SOL received for whole tokens sold outside the bot.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OutsideProceeds {
    native: f64,
    tokens: f64,
    /// Raw tokens the part acquired outside the bot.
    acquired: RawAmount,
}

/// The row's part of `round`, its home round, and of the `earlier` rounds holding its legs.
/// In a round another position of the mint booked part of, the part runs from the row's
/// first own event to the end of that round; in any other round of the span it is the whole
/// round. `None` when the row is not charged a part, the row's records do not cover its
/// entry, every DCA and every partial exit, or a shared round holds none of the row's own
/// events.
///
/// The row's own legs come from its records and are counted once, whatever the number of
/// rounds they lie in. An event the row booked is skipped, and an event another position
/// booked belongs to that position; every other event of the part happened outside the bot
/// and is the row's. A leg the row booked without a record would be counted as outside, so
/// a row missing one has no part that can be told apart. What the row holds is the home
/// round's balance: the earlier rounds ended at zero.
fn round_share(
    round: &LedgerRound,
    earlier: &[LedgerRound],
    attribution: &RoundAttribution,
) -> Option<RoundShare> {
    if !attribution.charges_part(round, earlier) || !attribution.records_complete {
        return None;
    }

    let own = &attribution.own;
    let mut acquired = own.booked_acquired;
    let mut invested_native = own.booked_invested_native;
    let mut entries = u32::try_from(own.entry_signatures.len()).unwrap_or(u32::MAX);
    let mut has_outside_acquisition = false;
    let mut outside_acquisitions_priced = true;
    let mut basis_complete = true;
    let mut history_complete = true;

    for part_round in earlier.iter().chain(std::iter::once(round)) {
        let start = attribution.part_start(part_round)?;
        basis_complete &= part_round.basis_complete;
        history_complete &= part_round.history_complete;

        let outside = part_round.events[start..]
            .iter()
            .filter(|event| attribution.is_outside(&event.signature));
        for event in outside {
            match event.kind {
                LedgerEventKind::Entry | LedgerEventKind::Add | LedgerEventKind::Receive => {
                    has_outside_acquisition = true;
                    acquired = acquired
                        .checked_add(event.amount_raw)
                        .unwrap_or(RawAmount::MAX);
                    if event.kind == LedgerEventKind::Receive {
                        outside_acquisitions_priced = false;
                        continue;
                    }
                    entries = entries.saturating_add(1);
                    match sol_quote(event) {
                        Some(sol) => invested_native += sol,
                        None => outside_acquisitions_priced = false,
                    }
                }
                LedgerEventKind::PartialExit | LedgerEventKind::Exit | LedgerEventKind::Send => {}
            }
        }
    }

    Some(RoundShare {
        acquired,
        invested_native,
        entries,
        held: attributable_held(
            RawAmount::new(round.balance_raw),
            attribution.held_elsewhere,
            acquired,
        ),
        has_outside_acquisition,
        basis_known: basis_complete && outside_acquisitions_priced,
        history_complete,
    })
}

/// What the row's part of `round` and of the `earlier` rounds holding its legs sold outside
/// the bot received. `None` when the row's records do not cover its legs, a round of the
/// part does not reconcile with the balances observed or holds an unpriced disposal, an
/// outside disposal has no SOL price or was a transfer, or nothing was sold outside.
///
/// The row's own sales are counted through its records, and a sale another position of the
/// mint booked is that position's; only the events no position booked are counted here.
fn outside_proceeds(
    round: &LedgerRound,
    earlier: &[LedgerRound],
    attribution: &RoundAttribution,
) -> Option<OutsideProceeds> {
    if !attribution.records_complete {
        return None;
    }

    let mut disposed = RawAmount::ZERO;
    let mut acquired = RawAmount::ZERO;
    let mut native = 0.0;
    for part_round in earlier.iter().chain(std::iter::once(round)) {
        if !part_round.history_complete {
            return None;
        }
        // The reducer leaves a round's exit price unset when it refused to price its
        // proceeds, for example SOL legs in a round quoted in USD.
        if part_round.exit_count > 0 && part_round.average_exit_price_native.is_none() {
            return None;
        }
        let start = attribution.part_start(part_round)?;
        let outside = part_round.events[start..]
            .iter()
            .filter(|event| attribution.is_outside(&event.signature));
        for event in outside {
            match event.kind {
                LedgerEventKind::PartialExit | LedgerEventKind::Exit => {
                    disposed = disposed
                        .checked_add(event.amount_raw)
                        .unwrap_or(RawAmount::MAX);
                    native += sol_quote(event)?;
                }
                LedgerEventKind::Send => return None,
                LedgerEventKind::Entry | LedgerEventKind::Add | LedgerEventKind::Receive => {
                    acquired = acquired
                        .checked_add(event.amount_raw)
                        .unwrap_or(RawAmount::MAX);
                }
            }
        }
    }

    let tokens = disposed.to_whole_units(round.decimals);
    (tokens > DUST).then_some(OutsideProceeds {
        native,
        tokens,
        acquired,
    })
}

/// The SOL side of a traded event, when it was quoted in SOL.
fn sol_quote(event: &LedgerEvent) -> Option<f64> {
    event
        .quote
        .filter(|quote| quote.asset == QuoteAsset::Sol)
        .map(|quote| quote.amount)
}

/// Display metadata for a mint, resolved once per sync from the tokens database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoundMetadata {
    pub symbol: Option<String>,
    pub name: Option<String>,
    /// The wallet's token account for this mint is frozen and cannot be sold.
    pub frozen: bool,
}

/// What a sync must write. Nothing else in `positions` is touched.
#[derive(Debug, Default)]
pub struct SyncPlan {
    pub inserts: Vec<Position>,
    pub updates: Vec<PlannedUpdate>,
    /// Closed rows that give up a round key to the open row of their mint, written before
    /// the updates so the open row can take the key.
    pub round_key_releases: Vec<RoundKeyRelease>,
    /// The planning clock, reused when an update is re-derived at write time.
    pub now: DateTime<Utc>,
    /// The signatures of every swap leg the trader had booked before the wallet history
    /// was read, when known. A bot row of a mint with a leg booked later, on any of its
    /// positions, is not reconciled: its round may predate that fill.
    booked_before: Option<HashSet<String>>,
}

/// A rewrite of an existing row, with the rounds and metadata it was derived from.
///
/// `position` is the rewrite as planned against the rows read for the plan. The write
/// re-derives it from the same rounds on the row read inside its own transaction, so a
/// booking committed after the plan was made is kept instead of overwritten.
#[derive(Debug, Clone)]
pub struct PlannedUpdate {
    pub position: Position,
    round: LedgerRound,
    /// The earlier rounds holding this row's own legs, whose outside events are part of
    /// what it is charged. Empty for a wallet-derived row, a closed row, and a row whose
    /// legs lie in no earlier round.
    earlier: Vec<LedgerRound>,
    meta: RoundMetadata,
}

/// A closed row whose round key moves to the open row of its mint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundKeyRelease {
    pub position_id: i64,
    pub round_key: String,
}

impl SyncPlan {
    pub fn is_empty(&self) -> bool {
        self.inserts.is_empty() && self.updates.is_empty() && self.round_key_releases.is_empty()
    }

    /// The plan, told which swap legs the trader had booked before the wallet history it
    /// was planned from was read.
    pub fn booked_before(mut self, signatures: HashSet<String>) -> Self {
        self.booked_before = Some(signatures);
        self
    }
}

/// Outcome of one wallet-history sync, for logging and the debug API.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyncSummary {
    pub rounds: usize,
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
}

/// The rows [`apply_plan`] wrote. A planned insert or update found busy, or an update
/// already current, at write time is `skipped`; a row that failed to write is logged and
/// counted in neither.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AppliedPlan {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Decide which position rows must be created, rewritten or reconciled.
///
/// Pure: the same rounds, existing rows, metadata, busy mints and `now` always produce
/// the same plan. `now` is used for exactly two things, both of them a last resort: the
/// entry timestamp of a round whose history contains no block time at all (and only for
/// a row being created for the first time — an existing row keeps the timestamp it
/// already has, so repeated syncs never make a position appear to drift forward in
/// time), and the close time of a holding that vanished from the wallet without any
/// disposal we could observe.
///
/// `busy_mints` are mints with bot work in flight (a pending DCA or partial exit, or a bot
/// swap whose row or pending state is not recorded yet). A matched row for one of them is
/// neither reconciled nor duplicated, and an unmatched round of one creates no row — the
/// trader's own bookkeeping lands first and the next sync sees a settled position.
///
/// An open bot row is reconciled on its home round: the latest round, an open one first,
/// holding one of its own legs (see [`row_homes`]). A row whose holding spans rounds (the
/// wallet emptied between its first buy and a later DCA) is never matched by an earlier
/// round, by key or by entry signature; the earlier rounds holding its legs add their
/// outside events to its part instead, and its key moves to the home round with its own
/// write. A closed round whose events include a bot row's own leg imports no row: those
/// legs are booked on their row, and a wallet-derived copy would be its twin.
pub fn plan_position_writes(
    rounds: &[LedgerRound],
    existing: &[Position],
    metadata: &HashMap<String, RoundMetadata>,
    trader_legs: &HashMap<i64, TraderLegs>,
    busy_mints: &HashSet<String>,
    now: DateTime<Utc>,
) -> SyncPlan {
    let by_round_key: HashMap<&str, &Position> = existing
        .iter()
        .filter_map(|position| {
            position
                .round_key
                .as_deref()
                .map(|round_key| (round_key, position))
        })
        .collect();
    let mut by_mint: HashMap<&str, Vec<&Position>> = HashMap::new();
    for position in existing {
        by_mint
            .entry(position.mint.as_str())
            .or_default()
            .push(position);
    }

    // Rows that predate the ledger, and rows the trader opened before their transaction
    // reached the delta table, carry no round key yet. They are adopted by
    // (mint, entry signature) so the round they belong to reconciles them in place
    // instead of materialising a second row for the same tokens.
    let mut adoptable: HashMap<(&str, &str), Vec<&Position>> = HashMap::new();
    for position in existing
        .iter()
        .filter(|position| position.round_key.is_none() && position.id.is_some())
    {
        if let Some(signature) = position.entry_transaction_signature.as_deref() {
            adoptable
                .entry((position.mint.as_str(), signature))
                .or_default()
                .push(position);
        }
    }
    for candidates in adoptable.values_mut() {
        candidates.sort_by_key(|position| (position.entry_time, position.id));
    }

    let homes = row_homes(rounds, existing, trader_legs);
    let mut claimed: HashSet<i64> = HashSet::new();
    let mut plan = SyncPlan {
        now,
        ..SyncPlan::default()
    };

    for (index, round) in rounds.iter().enumerate() {
        let meta = metadata.get(&round.mint).cloned().unwrap_or_default();
        let belongs_here = |row: &Position| !homes.elsewhere(row, index);
        let matched = by_round_key
            .get(round.round_key.as_str())
            .copied()
            .filter(|row| belongs_here(row))
            .or_else(|| adopt_row(round, &adoptable, &mut claimed, belongs_here));
        if let Some(id) = matched.and_then(|row| row.id) {
            claimed.insert(id);
        }
        let home_row = homes.home_row(index, &claimed);
        let mut key_release = None;
        let current = match (matched, home_row) {
            (Some(row), home_row) if row.exit_time.is_none() => {
                if let Some(home_row) = home_row {
                    let id = |row: &Position| row.id.map_or("?".to_owned(), |id| id.to_string());
                    logger::warning(
                        LogTag::Positions,
                        &format!(
                            "Ledger reconcile for {}: round {} matched open position {} while open position {} holds its latest legs; position {} is not reconciled this sync",
                            short_mint(&round.mint),
                            round.round_key,
                            id(row),
                            id(home_row),
                            id(home_row)
                        ),
                    );
                }
                row
            }
            (Some(closed), Some(home_row)) => {
                if let Some(id) = home_row.id {
                    claimed.insert(id);
                }
                if closed.round_key.as_deref() == Some(round.round_key.as_str()) {
                    key_release = closed.id.map(|position_id| RoundKeyRelease {
                        position_id,
                        round_key: round.round_key.clone(),
                    });
                }
                home_row
            }
            (Some(closed), None) => closed,
            (None, Some(home_row)) => {
                if let Some(id) = home_row.id {
                    claimed.insert(id);
                }
                home_row
            }
            // A closed round whose bot legs are booked on their row: a row imported now
            // would be its twin.
            (None, None) if !round.is_open && homes.holds_bot_leg(index) => continue,
            // A bot swap of the mint in flight may be this round's own buy, whose row is
            // saved once the swap returns; a row imported now would be its twin.
            (None, None) if busy_mints.contains(&round.mint) => continue,
            (None, None) => {
                plan.inserts.push(build_position(round, &meta, None, now));
                continue;
            }
        };

        let earlier = homes.earlier(current, index, rounds);
        let attribution = (!current.is_wallet_derived()).then(|| {
            let mint_positions = by_mint
                .get(current.mint.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            planned_attribution(current, mint_positions, trader_legs)
        });
        let Some(position) = rewrite_row(
            current,
            round,
            &earlier,
            &meta,
            attribution.as_ref(),
            busy_mints,
            now,
        ) else {
            continue;
        };
        plan.round_key_releases.extend(key_release);
        plan.updates.push(PlannedUpdate {
            position,
            round: round.clone(),
            earlier,
            meta,
        });
    }

    plan
}

/// Where the own legs of each bot row lie among the rounds of one plan.
#[derive(Debug, Default)]
struct RowHomes<'a> {
    /// The home round of each open bot row whose legs lie in a round of the plan.
    home: HashMap<i64, usize>,
    /// The rounds holding one of each bot row's own legs, ascending, each once.
    span: HashMap<i64, Vec<usize>>,
    /// The open bot rows homed at each round.
    homed: HashMap<usize, Vec<&'a Position>>,
    /// The rounds holding an own leg of any bot row.
    with_bot_leg: HashSet<usize>,
}

impl<'a> RowHomes<'a> {
    /// True when `row` is an open bot row whose home is a round other than `index`.
    fn elsewhere(&self, row: &Position, index: usize) -> bool {
        row.id
            .and_then(|id| self.home.get(&id))
            .is_some_and(|home| *home != index)
    }

    /// The open row homed at round `index` that no round claimed yet. Where several are,
    /// the choice is the one every open-row lookup makes
    /// (`positions::state::get_open_round_by_mint`).
    fn home_row(&self, index: usize, claimed: &HashSet<i64>) -> Option<&'a Position> {
        crate::positions::state::choose_open_round(
            self.homed
                .get(&index)
                .into_iter()
                .flatten()
                .copied()
                .filter(|row| row.id.is_some_and(|id| !claimed.contains(&id))),
        )
    }

    /// True when round `index` holds an own leg of a bot row.
    fn holds_bot_leg(&self, index: usize) -> bool {
        self.with_bot_leg.contains(&index)
    }

    /// The rounds before `index` holding the own legs of `row`, an open bot row with a
    /// home; empty for any other row.
    fn earlier(&self, row: &Position, index: usize, rounds: &[LedgerRound]) -> Vec<LedgerRound> {
        let Some(id) = row.id.filter(|id| self.home.contains_key(id)) else {
            return Vec::new();
        };
        self.span
            .get(&id)
            .into_iter()
            .flatten()
            .filter(|span_index| **span_index < index)
            .map(|span_index| rounds[*span_index].clone())
            .collect()
    }
}

/// Locate every bot row's own legs among `rounds`, and the home round of each open one.
///
/// A row's own legs are its booked entries and exits and its entry signature; never its
/// exit signature, which on a row the ledger closed is a sale made outside the bot. Only
/// round events count, and only in rounds of the row's mint. The home of an open row is
/// the round of its span that is open, else the one seen last, else the later one: the
/// round its holding lives in now. An open row whose legs lie in no round has no home and
/// is matched by key or entry signature.
fn row_homes<'a>(
    rounds: &[LedgerRound],
    existing: &'a [Position],
    trader_legs: &HashMap<i64, TraderLegs>,
) -> RowHomes<'a> {
    let mut by_signature: HashMap<&str, Vec<&'a Position>> = HashMap::new();
    for row in existing.iter().filter(|row| !row.is_wallet_derived()) {
        let Some(id) = row.id else { continue };
        let legs = trader_legs.get(&id);
        let mut own: HashSet<&str> = legs
            .into_iter()
            .flat_map(|legs| legs.entry_signatures.iter().chain(&legs.exit_signatures))
            .map(String::as_str)
            .collect();
        own.extend(row.entry_transaction_signature.as_deref());
        for signature in own {
            by_signature.entry(signature).or_default().push(row);
        }
    }

    let mut homes = RowHomes::default();
    for (index, round) in rounds.iter().enumerate() {
        for event in &round.events {
            let Some(rows) = by_signature.get(event.signature.as_str()) else {
                continue;
            };
            for row in rows.iter().filter(|row| row.mint == round.mint) {
                let Some(id) = row.id else { continue };
                homes.with_bot_leg.insert(index);
                let span = homes.span.entry(id).or_default();
                if span.last() != Some(&index) {
                    span.push(index);
                }
            }
        }
    }

    for row in existing
        .iter()
        .filter(|row| !row.is_wallet_derived() && row.exit_time.is_none())
    {
        let Some(id) = row.id else { continue };
        let Some(home) = homes.span.get(&id).and_then(|span| {
            span.iter().copied().max_by_key(|index| {
                let round = &rounds[*index];
                (round.is_open, round.last_seen_at(), *index)
            })
        }) else {
            continue;
        };
        homes.home.insert(id, home);
        homes.homed.entry(home).or_default().push(row);
    }
    homes
}

/// The attribution of bot row `row` against `mint_positions`, every row of its mint read for
/// the plan: the other open rows of the mint hold what they claim.
fn planned_attribution(
    row: &Position,
    mint_positions: &[&Position],
    trader_legs: &HashMap<i64, TraderLegs>,
) -> RoundAttribution {
    let mint_rows: Vec<MintRow> = mint_positions
        .iter()
        .copied()
        .filter_map(MintRow::of)
        .collect();
    let held_elsewhere = mint_positions
        .iter()
        .copied()
        .filter(|other| other.id != row.id && other.exit_time.is_none())
        .filter_map(Position::held_amount)
        .fold(RawAmount::ZERO, |held, amount| {
            held.checked_add(amount).unwrap_or(RawAmount::MAX)
        });
    RoundAttribution::new(row, &mint_rows, trader_legs, held_elsewhere)
}

/// The round's acquisition signatures, its entry first, each once.
fn acquisition_signatures(round: &LedgerRound) -> Vec<&str> {
    let mut signatures: Vec<&str> = Vec::new();
    if let Some(signature) = round.entry_signature.as_deref() {
        signatures.push(signature);
    }
    for event in &round.events {
        if event.kind.is_acquisition() && !signatures.contains(&event.signature.as_str()) {
            signatures.push(event.signature.as_str());
        }
    }
    signatures
}

/// The rewrite a round implies for a row it claimed, or `None` when the row must be left
/// as it is.
///
/// A row the bot executed is reconciled against the chain, never rewritten, and left
/// alone while the trader has work in flight on it; it needs its `attribution`, and is left
/// as it is without one, and is charged the outside events of the `earlier` rounds holding
/// its legs. A wallet-derived row is rebuilt from the round. Either way, a rewrite that
/// changes nothing is not written.
fn rewrite_row(
    current: &Position,
    round: &LedgerRound,
    earlier: &[LedgerRound],
    meta: &RoundMetadata,
    attribution: Option<&RoundAttribution>,
    busy_mints: &HashSet<String>,
    now: DateTime<Utc>,
) -> Option<Position> {
    if current.is_wallet_derived() {
        let fresh = build_position(round, meta, Some(current), now);
        return differs(current, &fresh).then_some(fresh);
    }
    if is_busy(current, busy_mints) {
        return None;
    }
    let fresh = reconcile_owned_position(current, round, earlier, meta, attribution?, now);
    differs_owned(current, &fresh).then_some(fresh)
}

/// True when the trader still has work in flight on this position, so the ledger must
/// not touch it.
///
/// Three states qualify, and each is a race the reconciliation would lose: an entry that
/// has not been verified yet (the chain may not even show the buy), a submitted exit
/// awaiting verification (the verifier writes the fee-exact close moments later, and
/// closing it here first would make that write land on an already-closed row), and a
/// pending DCA or partial exit on the mint (the balance is about to move again).
fn is_busy(position: &Position, busy_mints: &HashSet<String>) -> bool {
    let exit_in_flight =
        position.exit_transaction_signature.is_some() && !position.transaction_exit_verified;

    !position.transaction_entry_verified || exit_in_flight || busy_mints.contains(&position.mint)
}

/// Claim the unkeyed row this round was executed through, if there is one.
///
/// Only an ACQUISITION signature can claim a row: a position's identity is the buy that
/// opened it, and letting a sale match would attach a round to a row it never funded.
/// The mint must match too — one signature can move several mints, and a token -> token
/// swap's signature belongs to both sides.
///
/// A row is claimed at most once per plan, so two rounds of the same mint (the user
/// bought, sold, and bought again) can never collapse onto the same row. A candidate
/// `accepts` refuses is skipped and stays unclaimed for the round it belongs to.
fn adopt_row<'a>(
    round: &LedgerRound,
    adoptable: &HashMap<(&str, &str), Vec<&'a Position>>,
    claimed: &mut HashSet<i64>,
    accepts: impl Fn(&Position) -> bool,
) -> Option<&'a Position> {
    for signature in acquisition_signatures(round) {
        let Some(candidates) = adoptable.get(&(round.mint.as_str(), signature)) else {
            continue;
        };
        for candidate in candidates {
            let Some(id) = candidate.id else { continue };
            if !claimed.contains(&id) && accepts(candidate) {
                claimed.insert(id);
                return Some(candidate);
            }
        }
    }

    None
}

/// Build the position row a round implies, carrying over everything the ledger does not
/// own from the row that already exists.
fn build_position(
    round: &LedgerRound,
    meta: &RoundMetadata,
    existing: Option<&Position>,
    now: DateTime<Utc>,
) -> Position {
    let entry_time = round
        .opened_at
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .or_else(|| existing.map(|p| p.entry_time))
        .unwrap_or(now);
    let exit_time = round
        .closed_at
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        // A round the wallet no longer holds but whose closing transaction we never saw
        // is still closed; date it by the last moment we saw the holding rather than
        // inventing one, and fall back to the entry time so ordering stays sane.
        .or_else(|| {
            if round.is_open {
                None
            } else {
                round
                    .last_seen_at()
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .or_else(|| existing.and_then(|p| p.exit_time))
                    .or(Some(entry_time))
            }
        });

    // Only a complete basis may be presented as money. Without one, invested SOL is
    // zero (the reducer already zeroes it) and there is no P&L to show at all.
    let realized_pnl = if round.basis_complete && round.history_complete {
        round.realized_pnl_native
    } else {
        None
    };
    let realized_pnl_percent = realized_pnl.and_then(|pnl| {
        (round.realized_cost_native > super::DUST).then(|| pnl / round.realized_cost_native * 100.0)
    });

    let entry_price = round.average_entry_price_native.unwrap_or(0.0);

    Position {
        id: existing.and_then(|p| p.id),
        mint: round.mint.clone(),
        symbol: meta
            .symbol
            .clone()
            .or_else(|| existing.map(|p| p.symbol.clone()))
            .unwrap_or_else(|| short_mint(&round.mint)),
        name: meta
            .name
            .clone()
            .or_else(|| existing.map(|p| p.name.clone()))
            .unwrap_or_else(|| short_mint(&round.mint)),
        entry_price,
        entry_time,
        exit_price: round.average_exit_price_native,
        exit_time,
        position_type: "buy".to_owned(),
        entry_size_native: round.invested_native,
        total_size_native: round.invested_native,
        // Price extremes belong to the price updater; seed them from the entry price so
        // a brand-new row is not stuck at zero.
        price_highest: existing.map(|p| p.price_highest).unwrap_or(entry_price),
        price_lowest: existing.map(|p| p.price_lowest).unwrap_or(entry_price),
        entry_transaction_signature: round.entry_signature.clone(),
        exit_transaction_signature: round.exit_signature.clone(),
        token_amount: Some(RawAmount::new(round.total_acquired_raw)),
        effective_entry_price: round.average_entry_price_native,
        effective_exit_price: round.average_exit_price_native,
        native_received: (round.exit_count > 0).then_some(round.realized_proceeds_native),
        profit_target_min: None,
        profit_target_max: None,
        liquidity_tier: existing.and_then(|p| p.liquidity_tier.clone()),
        // The chain is the verification: these rounds are reduced from confirmed,
        // fully-processed transactions, so there is nothing left to verify. A closed
        // round MUST be marked verified or it would never appear in the Closed tab.
        transaction_entry_verified: true,
        transaction_exit_verified: !round.is_open,
        entry_fee_raw: None,
        exit_fee_raw: None,
        current_price: existing.and_then(|p| p.current_price),
        current_price_updated: existing.and_then(|p| p.current_price_updated),
        current_price_source: existing.and_then(|p| p.current_price_source),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: (!round.is_open).then(|| "wallet_history".to_owned()),
        pnl: realized_pnl,
        pnl_percent: realized_pnl_percent,
        unrealized_pnl: existing.and_then(|p| p.unrealized_pnl),
        unrealized_pnl_percent: existing.and_then(|p| p.unrealized_pnl_percent),
        remaining_token_amount: Some(RawAmount::new(round.balance_raw)),
        total_exited_amount: RawAmount::new(round.total_disposed_raw),
        average_exit_price: round.average_exit_price_native,
        // The FIRST traded acquisition is the entry; the rest are adds. Likewise the
        // disposal that closed the round is the exit and the rest are partials.
        partial_exit_count: round
            .exit_count
            .saturating_sub(if round.is_open { 0 } else { 1 }),
        dca_count: round.entry_count.saturating_sub(1),
        average_entry_price: entry_price,
        last_dca_time: existing.and_then(|p| p.last_dca_time),
        // Archival is the user's decision alone — a resync never sets or clears it.
        archived: existing.is_some_and(|p| p.archived),
        archived_at: existing.and_then(|p| p.archived_at),
        origin: PositionOrigin::External,
        management: PositionManagement::UserOnly,
        round_key: Some(round.round_key.clone()),
        basis_complete: round.basis_complete,
        history_complete: round.history_complete,
        holding_state: (meta.frozen && round.is_open).then(|| HOLDING_STATE_FROZEN.to_owned()),
    }
}

/// Reconcile a position the BOT executed against what the chain says the wallet holds.
///
/// The trader owns this row: its origin, management, profit targets, strategy state and
/// entry records are never touched. What the CHAIN owns is the size of the holding and
/// what it cost, and this is where the two meet.
///
/// A closed row is settled: the ledger stamps its round key and holding state and changes
/// nothing else, whatever its round did after the close.
///
/// A round is one wallet fact and one row, so an acquisition the user made in another
/// wallet app belongs to the same round as the bot's own buy, and the row follows the
/// holding up as well as down (see [`adopt_external_growth`]).
///
/// `round` is the row's home round: the latest round holding one of its own legs. The key
/// it stamps moves the row onto it, so a row whose holding spans rounds follows the round of
/// its latest leg.
///
/// The row is charged only its own part (see [`round_share`]) when another position of the
/// mint booked part of the round (a closed position whose round never returned to zero,
/// with the open position bought into the same holding), or when `earlier` rounds hold some
/// of its legs (the wallet emptied between its first buy and a later DCA): its booked legs,
/// plus the outside events of every round of the part. A row whose part cannot be located,
/// whose records miss its entry, a DCA or a partial exit, or whose records do not cover
/// what it holds, keeps its sizes and basis as booked, and a close of its round books no
/// proceeds.
///
/// While the round is open, the proceeds of the part's outside sales join the row's own
/// recorded proceeds (see [`outside_proceeds`]); an unpriced or transferred disposal leaves
/// the proceeds as booked, and so does an outside acquisition of the part that the row's
/// books do not carry, by amount or by a complete basis (see [`book_outside_proceeds`]).
///
/// A round closed by a sale outside the bot closes the row. Its proceeds are the round's
/// when the row owns the whole round alone, and the row's own recorded proceeds plus its
/// part's outside sales when it is charged a part; P&L is the realized P&L of the row as
/// closed.
fn reconcile_owned_position(
    existing: &Position,
    round: &LedgerRound,
    earlier: &[LedgerRound],
    meta: &RoundMetadata,
    attribution: &RoundAttribution,
    now: DateTime<Utc>,
) -> Position {
    let mut position = existing.clone();
    position.round_key = Some(round.round_key.clone());
    position.holding_state =
        (meta.frozen && round.is_open).then(|| HOLDING_STATE_FROZEN.to_owned());

    if existing.exit_time.is_some() {
        return position;
    }

    let outside = outside_proceeds(round, earlier, attribution);
    let claimed_remaining = existing.remaining_token_amount.unwrap_or_default();
    let close_proceeds = if attribution.charges_part(round, earlier) {
        let Some(share) = round_share(round, earlier, attribution)
            .filter(|share| share.acquired >= claimed_remaining)
        else {
            // The row's legs are not all recorded, or its records do not cover what it
            // holds: what it owns of the round is unknown.
            if round.is_open {
                return position;
            }
            return close_owned_position(position, existing, round, CloseProceeds::Unknown, now);
        };
        if share.has_outside_acquisition && share.held > claimed_remaining && share.history_complete
        {
            adopt_external_growth(
                &mut position,
                Growth {
                    held: share.held,
                    acquired: share.acquired,
                    exited: share
                        .acquired
                        .checked_sub(share.held)
                        .unwrap_or(RawAmount::ZERO),
                    entries: share.entries,
                    invested_native: share.basis_known.then_some(share.invested_native),
                },
                round.decimals,
            );
        }
        if round.is_open {
            shrink_to(&mut position, existing, claimed_remaining, share.held);
            book_outside_proceeds(&mut position, &attribution.own, outside);
            return position;
        }
        if share.history_complete {
            CloseProceeds::Outside {
                own_native: attribution.own.booked_received_native,
                sold: outside,
            }
        } else {
            CloseProceeds::Unknown
        }
    } else {
        let observed_remaining = RawAmount::new(round.balance_raw);
        if observed_remaining > claimed_remaining && round.history_complete {
            adopt_external_growth(
                &mut position,
                Growth {
                    held: observed_remaining,
                    acquired: RawAmount::new(round.total_acquired_raw),
                    exited: RawAmount::new(round.total_disposed_raw),
                    entries: round.entry_count,
                    invested_native: round
                        .basis_complete
                        .then(|| whole_round_invested(existing, round, &attribution.own)),
                },
                round.decimals,
            );
        }
        if round.is_open {
            shrink_to(
                &mut position,
                existing,
                claimed_remaining,
                observed_remaining,
            );
            book_outside_proceeds(&mut position, &attribution.own, outside);
            return position;
        }
        CloseProceeds::Round
    };

    close_owned_position(position, existing, round, close_proceeds, now)
}

/// Lowers an open row's holding from `claimed` to `observed` when tokens left the wallet.
fn shrink_to(
    position: &mut Position,
    existing: &Position,
    claimed: RawAmount,
    observed: RawAmount,
) {
    let Some(sold) = claimed
        .checked_sub(observed)
        .filter(|sold| *sold > RawAmount::ZERO)
    else {
        return;
    };
    if let Err(error) = position.book_exit(sold) {
        position.remaining_token_amount = Some(observed);
        position.history_complete = false;
        logger::warning(
            LogTag::Positions,
            &format!(
                "Ledger reconcile for {}: exited amount not updated: {error}",
                short_mint(&existing.mint)
            ),
        );
    }
}

/// Books on an open row what its part sold outside the bot: the row's own recorded proceeds
/// plus those sales. Recomputed whole on every pass, so a resync writes the same figure;
/// without a priced outside sale the booked proceeds stand.
///
/// An outside acquisition of the part that the row's books do not carry, by amount or by a
/// complete basis, leaves the proceeds as booked: a sale of tokens whose cost the row does
/// not carry would read as profit. The books are what the row ever acquired, held plus
/// exited, since a bot row's entry amount does not grow on a DCA.
fn book_outside_proceeds(
    position: &mut Position,
    own: &TraderLegs,
    outside: Option<OutsideProceeds>,
) {
    let Some(outside) = outside else {
        return;
    };
    let covered = outside.acquired == RawAmount::ZERO
        || (position.basis_complete
            && own
                .booked_acquired
                .checked_add(outside.acquired)
                .zip(position.acquired_amount())
                .is_some_and(|(needed, acquired)| acquired >= needed));
    if covered {
        position.native_received = Some(own.booked_received_native + outside.native);
    }
}

/// Which proceeds a round closed outside the bot books on the row it closes.
enum CloseProceeds {
    /// The row owns the whole round: the round's proceeds.
    Round,
    /// The row is charged a part: its own recorded proceeds plus its part's outside sales,
    /// when they were all priced.
    Outside {
        own_native: f64,
        sold: Option<OutsideProceeds>,
    },
    /// What the row owns of the round is unknown: no proceeds.
    Unknown,
}

/// Close an open bot row whose round closed on chain without the bot selling it.
fn close_owned_position(
    mut position: Position,
    existing: &Position,
    round: &LedgerRound,
    proceeds: CloseProceeds,
    now: DateTime<Utc>,
) -> Position {
    if let Err(error) = position.book_remaining_as_exited() {
        position.history_complete = false;
        logger::warning(
            LogTag::Positions,
            &format!(
                "Ledger reconcile for {}: exited amount not updated: {error}",
                short_mint(&existing.mint)
            ),
        );
    }
    position.remaining_token_amount = Some(RawAmount::ZERO);
    position.exit_time = Some(
        round
            .closed_at
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            // Gone from the wallet with no disposal we could observe: the close is real
            // and only its moment is unknown, so date it by the last time we saw the
            // holding. Stamping "now" would rank a long-abandoned position above the
            // wallet's most recent exit every time the ledger re-read it.
            .or_else(|| {
                round
                    .last_seen_at()
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
            })
            .unwrap_or(now),
    );
    if position.exit_transaction_signature.is_none() {
        position.exit_transaction_signature = round.exit_signature.clone();
    }
    // Reduced from confirmed, fully-processed transactions: there is nothing left for
    // the verifier to confirm, and an unverified exit would keep the row out of the
    // Closed tab forever.
    position.transaction_exit_verified = true;
    if position.closed_reason.is_none() {
        position.closed_reason = Some(CLOSED_EXTERNALLY.to_owned());
    }
    position.unrealized_pnl = None;
    position.unrealized_pnl_percent = None;

    // Proceeds only when every disposal counted was priced in SOL AND the history
    // reconciles. A round whose tokens left the wallet without a disposal we could
    // observe still carries the average price of the disposals we DID see, so pricing
    // the close from it would book the proceeds of part of the position against the cost
    // of all of it — a plausible, permanently wrong loss. Flag it instead.
    if !round.history_complete {
        position.history_complete = false;
        return position;
    }

    let (exit_price, native_received) = match proceeds {
        CloseProceeds::Round => match round.average_exit_price_native {
            Some(exit_price) => (exit_price, round.realized_proceeds_native),
            None => return position,
        },
        CloseProceeds::Outside {
            own_native,
            sold: Some(sold),
        } => (sold.native / sold.tokens, own_native + sold.native),
        CloseProceeds::Outside { sold: None, .. } => return position,
        CloseProceeds::Unknown => {
            position.history_complete = false;
            return position;
        }
    };
    position.exit_price = Some(exit_price);
    position.effective_exit_price = Some(exit_price);
    position.average_exit_price = Some(exit_price);
    position.native_received = Some(native_received);
    // The basis is the position's own cumulative `total_size_native` (entry + every DCA),
    // fee-exact because the trader booked it; the proceeds are the chain's.
    if position.total_size_native > DUST {
        let (pnl, pnl_percent) = realized_pnl(&position);
        position.pnl = Some(pnl);
        position.pnl_percent = Some(pnl_percent);
    }

    position
}

/// The holding a bot row takes on when acquisitions made outside the bot grew it.
struct Growth {
    held: RawAmount,
    acquired: RawAmount,
    exited: RawAmount,
    /// Traded acquisitions, the entry included.
    entries: u32,
    /// What everything acquired cost, `None` when part of it has no SOL price.
    invested_native: Option<f64>,
}

/// Fold acquisitions the user made outside the bot into a bot-owned row.
///
/// The sizes are the chain's. The basis is the trader's own legs at the SOL it recorded
/// (which includes the fee it actually paid) plus the SOL the chain shows for every other
/// traded acquisition the row is charged with. Recomputing the whole basis from those two
/// parts on each pass is what makes a resync idempotent — adding the difference would
/// inflate the position a little more every time the wallet moved. An outside buy with no
/// SOL price (an airdrop, a USD fill, a token -> token swap) leaves the trader's basis alone
/// and clears `basis_complete`: the holding is real, the cost of part of it is not.
fn adopt_external_growth(position: &mut Position, growth: Growth, decimals: u8) {
    position.remaining_token_amount = Some(growth.held);
    position.token_amount = Some(growth.acquired);
    position.total_exited_amount = growth.exited;
    position.dca_count = growth.entries.saturating_sub(1);

    let Some(invested_native) = growth.invested_native else {
        position.basis_complete = false;
        return;
    };
    position.total_size_native = invested_native;
    let acquired = growth.acquired.to_whole_units(decimals);
    if acquired > DUST {
        position.average_entry_price = invested_native / acquired;
    }
}

/// What a round the row owns alone cost: the trader's booked SOL for its own legs, plus the
/// chain's SOL for every other traded acquisition. Without any records the row's own total
/// is the best fee-exact number there is, and its entry signature the only leg it can
/// attribute.
fn whole_round_invested(existing: &Position, round: &LedgerRound, legs: &TraderLegs) -> f64 {
    let (booked_signatures, booked_native) = if legs.entry_signatures.is_empty() {
        (
            existing
                .entry_transaction_signature
                .iter()
                .cloned()
                .collect(),
            existing.total_size_native,
        )
    } else {
        (legs.entry_signatures.clone(), legs.booked_invested_native)
    };

    let external_native: f64 = round
        .events
        .iter()
        .filter(|event| {
            matches!(event.kind, LedgerEventKind::Entry | LedgerEventKind::Add)
                && !booked_signatures.contains(&event.signature)
        })
        .filter_map(sol_quote)
        .sum();

    booked_native + external_native
}

/// True when the reconciliation of a bot-owned row changes anything. Every other field
/// is carried over untouched, so an unchanged wallet plans no write.
fn differs_owned(existing: &Position, fresh: &Position) -> bool {
    existing.round_key != fresh.round_key
        || existing.holding_state != fresh.holding_state
        || existing.remaining_token_amount != fresh.remaining_token_amount
        || existing.token_amount != fresh.token_amount
        || existing.total_exited_amount != fresh.total_exited_amount
        || existing.dca_count != fresh.dca_count
        || existing.basis_complete != fresh.basis_complete
        || !same_money(existing.total_size_native, fresh.total_size_native)
        || !same_money(existing.average_entry_price, fresh.average_entry_price)
        || existing.exit_time != fresh.exit_time
        || existing.exit_transaction_signature != fresh.exit_transaction_signature
        || existing.transaction_exit_verified != fresh.transaction_exit_verified
        || existing.closed_reason != fresh.closed_reason
        || existing.unrealized_pnl != fresh.unrealized_pnl
        || !same_opt_money(existing.exit_price, fresh.exit_price)
        || !same_opt_money(existing.native_received, fresh.native_received)
        || !same_opt_money(existing.pnl, fresh.pnl)
}

/// True when the ledger-owned fields of `fresh` say something different from `existing`.
///
/// Deliberately ignores the price/archival fields `build_position` carries over, so an
/// idle wallet produces an EMPTY plan and the sync writes nothing at all.
fn differs(existing: &Position, fresh: &Position) -> bool {
    existing.entry_time != fresh.entry_time
        || existing.exit_time != fresh.exit_time
        || existing.transaction_exit_verified != fresh.transaction_exit_verified
        || existing.remaining_token_amount != fresh.remaining_token_amount
        || existing.total_exited_amount != fresh.total_exited_amount
        || existing.token_amount != fresh.token_amount
        || existing.dca_count != fresh.dca_count
        || existing.partial_exit_count != fresh.partial_exit_count
        || existing.basis_complete != fresh.basis_complete
        || existing.history_complete != fresh.history_complete
        || existing.holding_state != fresh.holding_state
        || existing.entry_transaction_signature != fresh.entry_transaction_signature
        || existing.exit_transaction_signature != fresh.exit_transaction_signature
        || existing.symbol != fresh.symbol
        || existing.name != fresh.name
        || !same_money(existing.total_size_native, fresh.total_size_native)
        || !same_money(existing.entry_price, fresh.entry_price)
        || !same_opt_money(existing.exit_price, fresh.exit_price)
        || !same_opt_money(existing.native_received, fresh.native_received)
        || !same_opt_money(existing.pnl, fresh.pnl)
}

fn same_money(a: f64, b: f64) -> bool {
    (a - b).abs() <= super::DUST
}

fn same_opt_money(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => same_money(a, b),
        _ => false,
    }
}

fn short_mint(mint: &str) -> String {
    if mint.len() <= 8 {
        return mint.to_owned();
    }
    format!("{}…{}", &mint[..4], &mint[mint.len() - 4..])
}

// =============================================================================
// APPLICATION
// =============================================================================

/// Reduce the wallet's processed history into rounds and write the missing/changed
/// external position rows.
///
/// Never fails the caller in a way that matters: every step degrades to "sync nothing"
/// rather than corrupting an existing position. Safe to call repeatedly — a wallet whose
/// history has not moved produces an empty plan and writes nothing.
pub async fn sync_wallet_history() -> super::super::error::Result<SyncSummary> {
    use crate::positions::Error;

    let wallet_address =
        crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;

    let Some(transactions_db) = crate::transactions::database::get_transaction_database().await
    else {
        return Err(Error::NotInitialised);
    };

    // Which legs the trader booked itself, read BEFORE the history: a bot-owned round can
    // then absorb an outside buy without double-counting the bot's own, and a leg booked
    // after this read marks a round that may predate it. A failure here is not fatal: with
    // no legs known, every bot row of a mint that has a record is left unreconciled by this
    // sync, and wallet-derived rows still sync.
    let booked_legs = match crate::positions::db::get_trader_swap_legs().await {
        Ok(legs) => legs,
        Err(e) => {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Wallet-history sync could not read the booked swap legs; bot positions with records are not reconciled: {e}"
                ),
            );
            Vec::new()
        }
    };
    let booked_before: HashSet<String> = booked_legs
        .iter()
        .map(|leg| leg.signature.clone())
        .collect();
    let trader_legs = TraderLegs::from_rows(booked_legs);

    let deltas = transactions_db
        .get_subject_deltas(&wallet_address)
        .await
        .map_err(|e| Error::WalletHistorySync {
            detail: e.to_string(),
        })?;
    if deltas.is_empty() {
        return Ok(SyncSummary::default());
    }

    let mut rounds = reduce_rounds(&deltas);

    // On-chain balances win over anything we inferred. A failure here is not fatal: the
    // reduced rounds are still the best truth we have, they simply keep their observed
    // balances (and `reconcile_with_wallet` is what would have flagged a mismatch).
    let holdings =
        match crate::chains::solana::assets::ata::get_all_token_accounts(&wallet_address).await {
            Ok(accounts) => accounts,
            Err(e) => {
                logger::warning(
                    LogTag::Positions,
                    &format!("Wallet-history sync could not read token accounts: {e}"),
                );
                Vec::new()
            }
        };

    let frozen_mints: HashSet<String> = holdings
        .iter()
        .filter(|account| account.is_frozen)
        .map(|account| account.mint.clone())
        .collect();

    if !holdings.is_empty() {
        let wallet_holdings: Vec<WalletHolding> = holdings
            .iter()
            .filter(|account| !account.is_nft && account.balance > 0)
            .map(|account| WalletHolding {
                mint: account.mint.clone(),
                amount_raw: u128::from(account.balance),
                decimals: account.decimals,
            })
            .collect();
        reconcile_with_wallet(&mut rounds, &wallet_holdings);
    }

    if rounds.is_empty() {
        return Ok(SyncSummary::default());
    }

    let metadata = resolve_metadata(&rounds, &frozen_mints).await;

    // Existing rows come from the DATABASE, not in-memory state, so this sync is
    // independent of whether the positions service has finished loading yet. Both
    // orderings converge: a row inserted here is picked up by the later load, and a row
    // already in memory is updated in place below.
    let existing = crate::positions::db::load_all_positions().await?;
    let busy_mints = crate::positions::state::mints_with_pending_swaps().await;
    let plan = plan_position_writes(
        &rounds,
        &existing,
        &metadata,
        &trader_legs,
        &busy_mints,
        Utc::now(),
    )
    .booked_before(booked_before);

    let planned_unchanged = rounds.len() - plan.inserts.len() - plan.updates.len();
    let applied = apply_plan(plan).await;
    let summary = SyncSummary {
        rounds: rounds.len(),
        inserted: applied.inserted,
        updated: applied.updated,
        unchanged: planned_unchanged + applied.skipped,
    };

    if summary.inserted > 0 || summary.updated > 0 {
        logger::info(
            LogTag::Positions,
            &format!(
                "Wallet-history sync: {} rounds ({} new, {} updated, {} unchanged)",
                summary.rounds, summary.inserted, summary.updated, summary.unchanged
            ),
        );
    }

    Ok(summary)
}

/// Resolve symbol/name/frozen for every mint in the plan in ONE query, not one per round.
async fn resolve_metadata(
    rounds: &[LedgerRound],
    frozen_mints: &HashSet<String>,
) -> HashMap<String, RoundMetadata> {
    let mints: Vec<String> = rounds
        .iter()
        .map(|round| round.mint.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // One batch per chain; a mint no enabled chain accepts has no stored metadata.
    let mut mints_by_chain: HashMap<crate::chains::ChainId, Vec<String>> = HashMap::new();
    for mint in &mints {
        if let Ok(chain) = crate::chains::chain_for_address(mint) {
            mints_by_chain.entry(chain).or_default().push(mint.clone());
        }
    }
    let mut info = HashMap::new();
    for (chain, chain_mints) in mints_by_chain {
        info.extend(
            crate::tokens::database::get_token_info_batch_async(chain, chain_mints)
                .await
                .unwrap_or_default(),
        );
    }

    mints
        .into_iter()
        .map(|mint| {
            let (symbol, name) = info
                .get(&mint)
                .map(|(symbol, name, _image)| (symbol.clone(), name.clone()))
                .unwrap_or((None, None));
            let frozen = frozen_mints.contains(&mint);
            (
                mint,
                RoundMetadata {
                    symbol,
                    name,
                    frozen,
                },
            )
        })
        .collect()
}

/// Write the plan to the database and mirror it into in-memory state.
///
/// An update is re-derived from its round on the row read inside its own booking
/// transaction (see [`rewrite_row`]), so a booking committed between planning and writing
/// is kept, and memory adopts the committed row. A bot row is charged its part of the round
/// from the legs, rows and holdings of its mint read in that same transaction. A row that
/// became busy, no longer differs, or whose mint carries a leg booked after the history was
/// read is left untouched.
///
/// A single failed row is logged and skipped: one unwritable position must not abort the
/// import of the rest of the wallet's history. Returns the rows actually written.
pub async fn apply_plan(plan: SyncPlan) -> AppliedPlan {
    let mut applied = AppliedPlan::default();
    let mut wrote_a_bot_row = false;
    for release in &plan.round_key_releases {
        release_round_key(release).await;
    }
    for update in plan.updates {
        let Some(id) = update.position.id else {
            continue;
        };
        let busy_mints = crate::positions::state::mints_with_pending_swaps().await;
        let booked_before = plan.booked_before.as_ref();
        let committed = crate::positions::apply::book_position(id, |row, reads| {
            let attribution = if row.is_wallet_derived() {
                None
            } else {
                let legs = reads.mint_swap_legs(&row.mint)?;
                // A fill the trader booked on any position of the mint after the history
                // was read is not in the round yet: reconciling against it would book the
                // fill again as an outside leg, or shrink the row by it.
                if booked_before
                    .is_some_and(|seen| legs.iter().any(|leg| !seen.contains(&leg.signature)))
                {
                    return Ok(Booking::Skip(false));
                }
                let mut attribution = RoundAttribution::new(
                    row,
                    &reads.mint_rows(&row.mint)?,
                    &TraderLegs::from_rows(legs),
                    RawAmount::ZERO,
                );
                // Only a row charged a part is charged against what the mint's other open
                // positions hold, and that read fails while one of them is unverified.
                if attribution.charges_part(&update.round, &update.earlier) {
                    attribution.held_elsewhere = reads.other_open_held(&row.mint)?;
                }
                Some(attribution)
            };
            Ok(
                match rewrite_row(
                    row,
                    &update.round,
                    &update.earlier,
                    &update.meta,
                    attribution.as_ref(),
                    &busy_mints,
                    plan.now,
                ) {
                    Some(fresh) => {
                        // Decided on the row read in this transaction: a row the trader
                        // closed after the plan was made is not closed by this write.
                        let closed_now = row.exit_time.is_none() && fresh.exit_time.is_some();
                        *row = fresh;
                        Booking::Write {
                            record: None,
                            outcome: closed_now,
                        }
                    }
                    None => Booking::Skip(false),
                },
            )
        })
        .await;
        let (position, closed_now) = match committed {
            Ok(Committed::Written { row, outcome }) => (row, outcome),
            Ok(Committed::Skipped(_) | Committed::Deleted { .. }) => {
                applied.skipped += 1;
                continue;
            }
            Err(e) => {
                logger::warning(
                    LogTag::Positions,
                    &format!("Wallet-history sync failed to update position {id}: {e}"),
                );
                continue;
            }
        };
        applied.updated += 1;
        wrote_a_bot_row |= !position.is_wallet_derived();

        // A bot position this write closed still holds the trading slot it took when it
        // opened, so the slot is released here. An archived row holds none, and a row that
        // was already closed is not announced again.
        if closed_now && !position.is_wallet_derived() {
            crate::positions::state::release_position_slot(id).await;
            logger::info(
                LogTag::Positions,
                &format!(
                    "Closed position {id} ({}) from wallet history: the token left the wallet without us selling it",
                    position.symbol
                ),
            );
            // The user's own action closed this, so nothing else would announce it. It
            // is the same shape as every other close, with its own action so the feed
            // never claims the bot sold.
            crate::events::record_position_event(
                &id.to_string(),
                &position.mint,
                "closed_externally",
                position.entry_transaction_signature.as_deref(),
                position.exit_transaction_signature.as_deref(),
                position.total_size_native,
                position.token_amount.unwrap_or_default(),
                position.pnl,
                position.pnl_percent,
            )
            .await;
        }
    }
    // Inserts follow the updates: a mint has one open position, so a round the history
    // closed must be closed before the next round of its mint can open a row. The mints
    // with bot work in flight are read after the updates, which may have run for a while.
    let busy_mints = crate::positions::state::mints_with_pending_swaps().await;
    for mut position in plan.inserts {
        if busy_mints.contains(&position.mint) {
            applied.skipped += 1;
            continue;
        }
        match crate::positions::db::save_position(&position).await {
            Ok(id) => {
                applied.inserted += 1;
                position.id = Some(id);
                // Mirror into memory. If the positions service has not loaded yet, its
                // load replaces the whole vector from the database (which now contains
                // this row), so neither ordering can duplicate it.
                crate::positions::state::add_position(position).await;
            }
            Err(e) => logger::warning(
                LogTag::Positions,
                &format!(
                    "Wallet-history sync failed to insert round {}: {e}",
                    position.round_key.as_deref().unwrap_or("?")
                ),
            ),
        }
    }

    // A bot row the ledger closed, or whose P&L it rewrote, changes the realized losses
    // the loss limiter counts from the books.
    if wrote_a_bot_row {
        crate::trader::safety::loss_limit::sync_from_books().await;
    }
    applied
}

/// Clears the round key of a closed row that gives it up to the open row of its mint (see
/// [`plan_position_writes`]), in its own booking transaction. A row that reopened or took
/// another key since the plan was made is left alone. A failure is logged: the open row's
/// update then fails on the key and is planned again by the next sync.
async fn release_round_key(release: &RoundKeyRelease) {
    let RoundKeyRelease {
        position_id,
        round_key,
    } = release;
    let released = crate::positions::apply::book_position(*position_id, |row, _| {
        if row.exit_time.is_none() || row.round_key.as_deref() != Some(round_key.as_str()) {
            return Ok(Booking::Skip(()));
        }
        row.round_key = None;
        Ok(Booking::Write {
            record: None,
            outcome: (),
        })
    })
    .await;
    match released {
        Ok(Committed::Written { .. }) => logger::info(
            LogTag::Positions,
            &format!(
                "Closed position {position_id} gave round {round_key} up to the open position of its mint"
            ),
        ),
        Ok(Committed::Skipped(()) | Committed::Deleted { .. }) => {}
        Err(e) => logger::warning(
            LogTag::Positions,
            &format!(
                "Wallet-history sync failed to release round {round_key} from closed position {position_id}: {e}"
            ),
        ),
    }
}

// =============================================================================
// LIVE RESYNC
// =============================================================================

/// Coalescing window for [`schedule_resync`]. One swap produces several confirmed
/// transactions in quick succession (approve, swap, close-account), and each of them
/// broadcasts activity — one reduction over the whole burst is both cheaper and more
/// correct than three over partial history.
const RESYNC_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(5);

/// Set while a resync is scheduled but has not started reducing yet.
static RESYNC_SCHEDULED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Serialises the reductions themselves, so two overlapping bursts cannot plan against
/// the same rows at once and write each other's stale view back.
static RESYNC_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Re-derive positions from wallet history shortly after the wallet moves.
///
/// The boot sync alone is not enough: a token sold in another wallet app while the bot
/// is running would keep its position open on screen until the next restart. Every
/// confirmed own-wallet transaction calls this, and the debounce plus single-flight
/// guard turn a burst of them into one reduction.
///
/// Fire-and-forget by design — the caller is a hot notification path and must never wait
/// on a database, an RPC read or another sync.
pub fn schedule_resync() {
    use std::sync::atomic::Ordering;

    if RESYNC_SCHEDULED.swap(true, Ordering::SeqCst) {
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(RESYNC_DEBOUNCE).await;
        // Cleared BEFORE the work starts: activity that arrives while this pass is
        // reducing schedules the NEXT one instead of being swallowed by it.
        RESYNC_SCHEDULED.store(false, Ordering::SeqCst);

        let _guard = RESYNC_LOCK.lock().await;
        if let Err(e) = sync_wallet_history().await {
            logger::warning(
                LogTag::Positions,
                &format!("Wallet-history resync failed: {e}"),
            );
        }
    });
}
