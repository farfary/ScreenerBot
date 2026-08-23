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
//! book (see [`TraderLegs`]). Nothing else the trader owns is rewritten, and a row with
//! work in flight (an unverified entry, a pending DCA or partial exit) is left
//! completely alone until that work lands — including not being duplicated.
//!
//! Freezing is a FLAG, never an action: a frozen token account sets
//! `holding_state = "frozen"` so the user can see the holding cannot be sold and archive
//! it if they choose. Nothing here archives, hides or closes a frozen position.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::{
    reconcile_with_wallet, reduce_rounds, LedgerEventKind, LedgerRound, QuoteAsset, WalletHolding,
    DUST,
};
use crate::logger::{self, LogTag};
use crate::positions::types::{Position, PositionManagement, PositionOrigin, HOLDING_STATE_FROZEN};

/// `closed_reason` for a bot-executed position whose token left the wallet through a
/// sale or transfer the bot did not make.
pub const CLOSED_EXTERNALLY: &str = "closed_externally";

/// The swap legs the TRADER itself executed for one position.
///
/// A bot-owned round can also contain acquisitions the user made in another wallet app.
/// Telling the two apart is what lets the row absorb the outside buy — the trader's own
/// legs keep their fee-exact SOL, and only the rest is taken from the chain.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraderLegs {
    /// Signatures of acquisitions the trader booked.
    pub entry_signatures: HashSet<String>,
    /// SOL those acquisitions cost, fee-exact as the trader recorded it.
    pub booked_invested_sol: f64,
}

impl TraderLegs {
    /// Build the per-position map from `(position_id, signature, is_exit, sol)` rows.
    pub fn from_rows(
        rows: impl IntoIterator<Item = (i64, String, bool, f64)>,
    ) -> HashMap<i64, Self> {
        let mut map: HashMap<i64, Self> = HashMap::new();
        for (position_id, signature, is_exit, sol) in rows {
            if is_exit {
                continue;
            }
            let legs = map.entry(position_id).or_default();
            if legs.entry_signatures.insert(signature) {
                legs.booked_invested_sol += sol;
            }
        }
        map
    }
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
    pub updates: Vec<Position>,
}

impl SyncPlan {
    pub fn is_empty(&self) -> bool {
        self.inserts.is_empty() && self.updates.is_empty()
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
/// `busy_mints` are mints with bot work in flight (a pending DCA or partial exit). A
/// matched row for one of them is neither reconciled nor duplicated — the trader's own
/// bookkeeping lands first and the next sync sees a settled position.
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

    let mut claimed: HashSet<i64> = HashSet::new();
    let mut plan = SyncPlan::default();

    for round in rounds {
        let meta = metadata.get(&round.mint).cloned().unwrap_or_default();
        let current = by_round_key
            .get(round.round_key.as_str())
            .copied()
            .or_else(|| adopt_row(round, &adoptable, &mut claimed));

        match current {
            // The bot's own row. Reconcile it against the chain; never rewrite what the
            // trader owns, and never insert a second row for the same round.
            Some(owned) if !owned.is_wallet_derived() => {
                if is_busy(owned, busy_mints) {
                    continue;
                }
                let legs = owned.id.and_then(|id| trader_legs.get(&id));
                let fresh = reconcile_owned_position(owned, round, &meta, legs, now);
                if differs_owned(owned, &fresh) {
                    plan.updates.push(fresh);
                }
            }
            Some(derived) => {
                let fresh = build_position(round, &meta, Some(derived), now);
                if differs(derived, &fresh) {
                    plan.updates.push(fresh);
                }
            }
            None => plan.inserts.push(build_position(round, &meta, None, now)),
        }
    }

    plan
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
/// bought, sold, and bought again) can never collapse onto the same row.
fn adopt_row<'a>(
    round: &LedgerRound,
    adoptable: &HashMap<(&str, &str), Vec<&'a Position>>,
    claimed: &mut HashSet<i64>,
) -> Option<&'a Position> {
    let mut signatures: Vec<&str> = Vec::new();
    if let Some(signature) = round.entry_signature.as_deref() {
        signatures.push(signature);
    }
    for event in &round.events {
        if event.kind.is_acquisition() && !signatures.contains(&event.signature.as_str()) {
            signatures.push(event.signature.as_str());
        }
    }

    for signature in signatures {
        let Some(candidates) = adoptable.get(&(round.mint.as_str(), signature)) else {
            continue;
        };
        for candidate in candidates {
            let Some(id) = candidate.id else { continue };
            if claimed.insert(id) {
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
        round.realized_pnl_sol
    } else {
        None
    };
    let realized_pnl_percent = realized_pnl.and_then(|pnl| {
        (round.realized_cost_sol > super::DUST).then(|| pnl / round.realized_cost_sol * 100.0)
    });

    let entry_price = round.average_entry_price_sol.unwrap_or(0.0);

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
        exit_price: round.average_exit_price_sol,
        exit_time,
        position_type: "buy".to_owned(),
        entry_size_sol: round.invested_sol,
        total_size_sol: round.invested_sol,
        // Price extremes belong to the price updater; seed them from the entry price so
        // a brand-new row is not stuck at zero.
        price_highest: existing.map(|p| p.price_highest).unwrap_or(entry_price),
        price_lowest: existing.map(|p| p.price_lowest).unwrap_or(entry_price),
        entry_transaction_signature: round.entry_signature.clone(),
        exit_transaction_signature: round.exit_signature.clone(),
        token_amount: Some(clamp_raw(round.total_acquired_raw)),
        effective_entry_price: round.average_entry_price_sol,
        effective_exit_price: round.average_exit_price_sol,
        sol_received: (round.exit_count > 0).then_some(round.realized_proceeds_sol),
        profit_target_min: None,
        profit_target_max: None,
        liquidity_tier: existing.and_then(|p| p.liquidity_tier.clone()),
        // The chain is the verification: these rounds are reduced from confirmed,
        // fully-processed transactions, so there is nothing left to verify. A closed
        // round MUST be marked verified or it would never appear in the Closed tab.
        transaction_entry_verified: true,
        transaction_exit_verified: !round.is_open,
        entry_fee_lamports: None,
        exit_fee_lamports: None,
        current_price: existing.and_then(|p| p.current_price),
        current_price_updated: existing.and_then(|p| p.current_price_updated),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: (!round.is_open).then(|| "wallet_history".to_owned()),
        pnl: realized_pnl,
        pnl_percent: realized_pnl_percent,
        unrealized_pnl: existing.and_then(|p| p.unrealized_pnl),
        unrealized_pnl_percent: existing.and_then(|p| p.unrealized_pnl_percent),
        remaining_token_amount: Some(clamp_raw(round.balance_raw)),
        total_exited_amount: clamp_raw(round.total_disposed_raw),
        average_exit_price: round.average_exit_price_sol,
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
/// A round is one wallet fact and one row, so an acquisition the user made in another
/// wallet app belongs to the same round as the bot's own buy. Ignoring it — which is
/// what this used to do — left the row claiming the balance and the cost basis it had
/// before the outside buy: the Positions tab showed 0.005 SOL invested while the wallet
/// held three times the tokens, and the next exit would have been sized against a
/// holding that no longer existed.
///
/// So the growth is ADOPTED, but only from legs the trader did not book itself:
/// `legs` carries the signatures and fee-exact SOL of the entries this position already
/// recorded, and everything else in the round is taken from the chain. That makes the
/// basis `booked + external`, which is recomputed identically on every resync instead of
/// accumulating.
///
/// Adoption requires the round to reconcile. A round whose history has a hole
/// (`history_complete == false`) keeps the old shrink-only behaviour, and a round whose
/// consideration could not be established in SOL (`basis_complete == false`) leaves the
/// trader's basis alone and clears `basis_complete` on the row, so the dashboard hides a
/// P&L it cannot compute rather than showing a wrong one.
fn reconcile_owned_position(
    existing: &Position,
    round: &LedgerRound,
    meta: &RoundMetadata,
    legs: Option<&TraderLegs>,
    now: DateTime<Utc>,
) -> Position {
    let mut position = existing.clone();
    position.round_key = Some(round.round_key.clone());
    position.holding_state =
        (meta.frozen && round.is_open).then(|| HOLDING_STATE_FROZEN.to_owned());

    let observed_remaining = clamp_raw(round.balance_raw);
    let claimed_remaining = existing.remaining_token_amount.unwrap_or(0);
    let grew = observed_remaining > claimed_remaining;

    if grew && round.history_complete {
        adopt_external_growth(&mut position, existing, round, legs);
    }

    if round.is_open {
        if observed_remaining < claimed_remaining {
            position.remaining_token_amount = Some(observed_remaining);
            position.total_exited_amount = existing
                .total_exited_amount
                .saturating_add(claimed_remaining - observed_remaining);
        }
        return position;
    }

    // The round is closed on chain. A position that already booked its own exit is
    // settled — the trader wrote the fee-exact numbers and there is nothing to add.
    if existing.exit_time.is_some() {
        return position;
    }

    position.remaining_token_amount = Some(0);
    position.total_exited_amount = existing
        .total_exited_amount
        .saturating_add(claimed_remaining);
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

    // Proceeds only when every disposal in the round was priced in SOL AND the history
    // reconciles. A round whose tokens left the wallet without a disposal we could
    // observe still carries the average price of the disposals we DID see, so pricing
    // the close from it would book the proceeds of part of the position against the cost
    // of all of it — a plausible, permanently wrong loss. Flag it instead.
    if !round.history_complete {
        position.history_complete = false;
        return position;
    }

    if let Some(exit_price) = round.average_exit_price_sol {
        position.exit_price = Some(exit_price);
        position.effective_exit_price = Some(exit_price);
        position.average_exit_price = Some(exit_price);
        position.sol_received = Some(round.realized_proceeds_sol);
        // The basis is the position's own cumulative `total_size_sol` (entry + every
        // DCA), which is fee-exact because the trader booked it; the proceeds are the
        // chain's. Mixing the two is the only complete number available here.
        if position.total_size_sol > DUST {
            let pnl = round.realized_proceeds_sol - position.total_size_sol;
            position.pnl = Some(pnl);
            position.pnl_percent = Some(pnl / position.total_size_sol * 100.0);
        }
    }

    position
}

/// Fold acquisitions the user made outside the bot into a bot-owned row.
///
/// The trader's own legs are identified by signature and keep the SOL it recorded (which
/// includes the fee it actually paid); every other traded acquisition in the round
/// contributes the SOL the chain shows. Recomputing the whole basis from those two parts
/// on each pass is what makes a resync idempotent — adding the difference would inflate
/// the position a little more every time the wallet moved.
fn adopt_external_growth(
    position: &mut Position,
    existing: &Position,
    round: &LedgerRound,
    legs: Option<&TraderLegs>,
) {
    // Chain truth for the sizes, whoever bought them.
    position.remaining_token_amount = Some(clamp_raw(round.balance_raw));
    position.token_amount = Some(clamp_raw(round.total_acquired_raw));
    position.total_exited_amount = clamp_raw(round.total_disposed_raw);
    position.dca_count = round.entry_count.saturating_sub(1);

    if !round.basis_complete {
        // The outside buy has no SOL price we can trust (an airdrop, a USD fill, a
        // token -> token swap). The holding is real, the cost of part of it is not.
        position.basis_complete = false;
        return;
    }

    // Legs the trader booked itself; without any records the row's own total is the
    // best fee-exact number we have and its entry signature is the only leg we can
    // attribute.
    let (booked_signatures, booked_sol) = match legs.filter(|l| !l.entry_signatures.is_empty()) {
        Some(legs) => (legs.entry_signatures.clone(), legs.booked_invested_sol),
        None => (
            existing
                .entry_transaction_signature
                .iter()
                .cloned()
                .collect(),
            existing.total_size_sol,
        ),
    };

    let external_sol: f64 = round
        .events
        .iter()
        .filter(|event| {
            matches!(event.kind, LedgerEventKind::Entry | LedgerEventKind::Add)
                && !booked_signatures.contains(&event.signature)
        })
        .filter_map(|event| event.quote)
        .filter(|quote| quote.asset == QuoteAsset::Sol)
        .map(|quote| quote.amount)
        .sum();

    position.total_size_sol = booked_sol + external_sol;

    let acquired = super::raw_to_whole(round.total_acquired_raw as i128, round.decimals);
    if acquired > DUST {
        position.average_entry_price = position.total_size_sol / acquired;
    }
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
        || !same_money(existing.total_size_sol, fresh.total_size_sol)
        || !same_money(existing.average_entry_price, fresh.average_entry_price)
        || existing.exit_time != fresh.exit_time
        || existing.exit_transaction_signature != fresh.exit_transaction_signature
        || existing.transaction_exit_verified != fresh.transaction_exit_verified
        || existing.closed_reason != fresh.closed_reason
        || existing.unrealized_pnl != fresh.unrealized_pnl
        || !same_opt_money(existing.exit_price, fresh.exit_price)
        || !same_opt_money(existing.sol_received, fresh.sol_received)
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
        || !same_money(existing.total_size_sol, fresh.total_size_sol)
        || !same_money(existing.entry_price, fresh.entry_price)
        || !same_opt_money(existing.exit_price, fresh.exit_price)
        || !same_opt_money(existing.sol_received, fresh.sol_received)
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

/// Raw base units are `u128` and exact; the position schema stores `u64`. A balance that
/// genuinely exceeds `u64::MAX` cannot exist for any real SPL mint (supply itself is
/// `u64`), so saturating is the correct total behaviour rather than a panic.
fn clamp_raw(raw: u128) -> u64 {
    u64::try_from(raw).unwrap_or(u64::MAX)
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
    use crate::positions::error::Error;

    let wallet_address =
        crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;

    let Some(transactions_db) = crate::transactions::database::get_transaction_database().await
    else {
        return Err(Error::NotInitialised);
    };

    let deltas = transactions_db
        .get_subject_deltas(&wallet_address)
        .await
        .map_err(|e| Error::WalletHistorySync { detail: e })?;
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
                amount_raw: account.balance as u128,
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
    // Which legs the trader booked itself, so a bot-owned round can absorb an outside
    // buy without double-counting the bot's own. A failure here is not fatal: the row
    // falls back to its own recorded total.
    let trader_legs = TraderLegs::from_rows(
        crate::positions::db::get_trader_swap_legs()
            .await
            .unwrap_or_default(),
    );
    let plan = plan_position_writes(
        &rounds,
        &existing,
        &metadata,
        &trader_legs,
        &busy_mints,
        Utc::now(),
    );

    let summary = SyncSummary {
        rounds: rounds.len(),
        inserted: plan.inserts.len(),
        updated: plan.updates.len(),
        unchanged: rounds.len() - plan.inserts.len() - plan.updates.len(),
    };

    apply_plan(plan).await;

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

    let info = crate::tokens::database::get_token_info_batch_async(mints.clone())
        .await
        .unwrap_or_default();

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
/// A single failed row is logged and skipped: one unwritable position must not abort the
/// import of the rest of the wallet's history.
async fn apply_plan(plan: SyncPlan) {
    for mut position in plan.inserts {
        match crate::positions::db::save_position(&position).await {
            Ok(id) => {
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

    for position in plan.updates {
        let Some(id) = position.id else { continue };
        if let Err(e) = crate::positions::db::update_position(&position).await {
            logger::warning(
                LogTag::Positions,
                &format!("Wallet-history sync failed to update position {id}: {e}"),
            );
            continue;
        }

        let closed_a_bot_position =
            !position.is_wallet_derived() && !crate::positions::state::is_position_open(&position);

        crate::positions::state::update_position_state_by_id(id, |stored| {
            *stored = position.clone();
        })
        .await;

        // A bot position the ledger just closed still holds the trading slot it took
        // when it opened. Releasing it is idempotent, so a row that was already closed
        // (or never held one) costs nothing.
        if closed_a_bot_position {
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
                position.total_size_sol,
                position.token_amount.unwrap_or_default(),
                position.pnl,
                position.pnl_percent,
            )
            .await;
        }
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
