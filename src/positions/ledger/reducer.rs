//! The reduction itself: `subject_asset_deltas` rows in, [`LedgerRound`]s out.
//!
//! Pure and total. No I/O, no clock, no global state, no panics — a malformed row
//! degrades the completeness flags rather than aborting the reduction.

use std::collections::{HashMap, HashSet};

use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT};
use crate::transactions::deltas::{DeltaKind, SubjectAssetDelta, NATIVE_SOL_SENTINEL};

use super::{
    raw_to_whole, LedgerEvent, LedgerEventKind, LedgerRound, QuoteAsset, QuoteLeg, WalletHolding,
    DUST,
};

/// Reduce every observed delta into rounds, oldest first.
///
/// `deltas` may arrive in any order; it is sorted by `(slot, tx_index, signature)`
/// internally so that grouping by transaction is correct regardless of the caller.
/// Both closed and still-open rounds are returned.
pub fn reduce_rounds(deltas: &[SubjectAssetDelta]) -> Vec<LedgerRound> {
    let mut ordered: Vec<&SubjectAssetDelta> = deltas.iter().filter(|d| d.success).collect();
    ordered.sort_by(|a, b| {
        a.slot
            .is_none()
            .cmp(&b.slot.is_none())
            .then_with(|| a.slot.cmp(&b.slot))
            .then_with(|| a.tx_index.cmp(&b.tx_index))
            .then_with(|| a.signature.cmp(&b.signature))
            .then_with(|| a.mint.cmp(&b.mint))
    });

    let mut open: HashMap<String, Round> = HashMap::new();
    let mut finished: Vec<LedgerRound> = Vec::new();
    // Mints that have already spent the bare `genesis:<mint>` key. History with two
    // holes in the same mint reduces to two genesis rounds, and a round key is a UNIQUE
    // index on `positions` — a second bare key would make one of the two rows fail to
    // insert, or collapse both onto one row and hide a whole holding.
    let mut genesis_used: HashSet<String> = HashSet::new();

    let mut cursor = 0usize;
    while cursor < ordered.len() {
        // One transaction is one contiguous run of equal signatures. The whole run is
        // needed at once: an acquisition's consideration is a SIBLING delta in the same
        // transaction, which is exactly how a token -> token swap is recognised.
        let signature = &ordered[cursor].signature;
        let mut end = cursor;
        while end < ordered.len() && &ordered[end].signature == signature {
            end += 1;
        }
        let group = &ordered[cursor..end];
        cursor = end;

        for delta in group {
            if is_reference_asset(&delta.mint) || delta.delta_raw == 0 {
                continue;
            }
            apply_delta(&mut open, &mut finished, &mut genesis_used, group, delta);
        }
    }

    for (_, round) in open.into_iter() {
        finished.push(round.finish(true));
    }

    finished.sort_by(|a, b| {
        a.opened_at
            .cmp(&b.opened_at)
            .then_with(|| a.mint.cmp(&b.mint))
            .then_with(|| a.round_key.cmp(&b.round_key))
    });
    finished
}

/// Route one delta into its round, creating and closing rounds as the balance crosses
/// zero.
fn apply_delta(
    open: &mut HashMap<String, Round>,
    finished: &mut Vec<LedgerRound>,
    genesis_used: &mut HashSet<String>,
    group: &[&SubjectAssetDelta],
    delta: &SubjectAssetDelta,
) {
    let grounded = delta.delta_raw > 0 && delta.before_raw == Some(0);

    if grounded {
        // The chain says this mint started the transaction at zero. Anything we still
        // had open for it is stale history we never saw close, so close it as
        // incomplete rather than letting the new round inherit a phantom balance.
        if let Some(stale) = open.remove(&delta.mint) {
            if stale.balance > 0 {
                let mut closed = stale.finish(false);
                closed.history_complete = false;
                closed.basis_complete = false;
                finished.push(closed);
            } else {
                finished.push(stale.finish(false));
            }
        }
        open.insert(
            delta.mint.clone(),
            Round::grounded(delta, format!("{}:{}", delta.signature, delta.mint)),
        );
    } else if !open.contains_key(&delta.mint) {
        // First thing we ever see for this mint is a disposal, or an acquisition onto a
        // non-zero balance: the round was already running before our history begins.
        // The first gap in a mint keeps the bare key, so rows written by earlier
        // versions still match; any further gap is qualified by the delta that revealed
        // it, which is as stable as the history it was reduced from.
        let round_key = if genesis_used.insert(delta.mint.clone()) {
            format!("genesis:{}", delta.mint)
        } else {
            format!("genesis:{}:{}", delta.signature, delta.mint)
        };
        open.insert(delta.mint.clone(), Round::genesis(delta, round_key));
    }

    let Some(round) = open.get_mut(&delta.mint) else {
        return;
    };
    round.apply(group, delta);

    if round.balance <= 0 {
        if let Some(closed) = open.remove(&delta.mint) {
            finished.push(closed.finish(false));
        }
    }
}

/// Reconcile reduced rounds against the wallet's current on-chain balances.
///
/// The deltas are what we OBSERVED; `holdings` is what the wallet actually has. Where
/// they disagree the on-chain number wins and the round is marked incomplete — never
/// the other way around. A held mint with no round at all gets a genesis round so the
/// dashboard shows everything the wallet holds, and an open round the wallet no longer
/// holds is closed without inventing a closing transaction or timestamp.
pub fn reconcile_with_wallet(rounds: &mut Vec<LedgerRound>, holdings: &[WalletHolding]) {
    let on_chain: HashMap<&str, &WalletHolding> =
        holdings.iter().map(|h| (h.mint.as_str(), h)).collect();

    for round in rounds.iter_mut() {
        if !round.is_open {
            continue;
        }
        let actual = on_chain
            .get(round.mint.as_str())
            .map(|h| h.amount_raw)
            .unwrap_or(0);

        if actual == round.balance_raw {
            continue;
        }

        round.balance_raw = actual;
        round.history_complete = false;
        round.basis_complete = false;
        round.realized_pnl_sol = None;
        round.average_entry_price_sol = None;

        if actual == 0 {
            // Gone from the wallet with no disposal in our history. It is closed, but
            // we do not know when or into what, so no timestamp and no proceeds.
            round.is_open = false;
            round.remaining_basis_sol = 0.0;
        }
    }

    let known: HashSet<&str> = rounds
        .iter()
        .filter(|r| r.is_open)
        .map(|r| r.mint.as_str())
        .collect();
    // Keys already taken by rounds we reduced, including closed ones: the synthetic
    // round below must never reuse one.
    let taken: HashSet<String> = rounds.iter().map(|r| r.round_key.clone()).collect();

    let missing: Vec<&WalletHolding> = holdings
        .iter()
        .filter(|h| h.amount_raw > 0 && !known.contains(h.mint.as_str()))
        .collect();

    for holding in missing {
        let bare = format!("genesis:{}", holding.mint);
        let round_key = if taken.contains(&bare) {
            format!("genesis:holding:{}", holding.mint)
        } else {
            bare
        };
        rounds.push(LedgerRound {
            mint: holding.mint.clone(),
            decimals: holding.decimals,
            round_key,
            opened_at: None,
            closed_at: None,
            is_open: true,
            balance_raw: holding.amount_raw,
            total_acquired_raw: holding.amount_raw,
            total_disposed_raw: 0,
            entry_count: 0,
            exit_count: 0,
            invested_sol: 0.0,
            remaining_basis_sol: 0.0,
            realized_proceeds_sol: 0.0,
            realized_cost_sol: 0.0,
            average_entry_price_sol: None,
            average_exit_price_sol: None,
            realized_pnl_sol: None,
            basis_complete: false,
            history_complete: false,
            entry_signature: None,
            exit_signature: None,
            events: Vec::new(),
        });
    }
}

// ==================== internal accumulator ====================

/// A round while it is still being built. Balances stay in exact raw base units; only
/// the money math is `f64`.
struct Round {
    mint: String,
    decimals: u8,
    round_key: String,
    opened_at: Option<i64>,
    closed_at: Option<i64>,
    balance: i128,
    total_acquired: i128,
    total_disposed: i128,
    entry_count: u32,
    exit_count: u32,
    quote_asset: Option<QuoteAsset>,
    invested: f64,
    remaining_basis: f64,
    realized_proceeds: f64,
    realized_cost: f64,
    priced_acquired: f64,
    priced_disposed: f64,
    basis_complete: bool,
    history_complete: bool,
    proceeds_complete: bool,
    entry_signature: Option<String>,
    exit_signature: Option<String>,
    events: Vec<LedgerEvent>,
}

impl Round {
    /// A round whose opening acquisition we watched happen from a zero balance.
    fn grounded(delta: &SubjectAssetDelta, round_key: String) -> Self {
        Self {
            mint: delta.mint.clone(),
            decimals: delta.decimals,
            round_key,
            opened_at: delta.block_time,
            closed_at: None,
            balance: 0,
            total_acquired: 0,
            total_disposed: 0,
            entry_count: 0,
            exit_count: 0,
            quote_asset: None,
            invested: 0.0,
            remaining_basis: 0.0,
            realized_proceeds: 0.0,
            realized_cost: 0.0,
            priced_acquired: 0.0,
            priced_disposed: 0.0,
            basis_complete: true,
            history_complete: true,
            proceeds_complete: true,
            entry_signature: Some(delta.signature.clone()),
            exit_signature: None,
            events: Vec::new(),
        }
    }

    /// A round that was already running before the history we hold. Its basis can never
    /// be established, because the acquisitions that set it are not in our data.
    fn genesis(delta: &SubjectAssetDelta, round_key: String) -> Self {
        let opening = delta.before_raw.and_then(|v| i128::try_from(v).ok());
        let mut round = Self::grounded(delta, round_key);
        round.opened_at = None;
        round.entry_signature = None;
        round.basis_complete = false;
        round.history_complete = false;
        if let Some(balance) = opening {
            round.balance = balance;
            round.total_acquired = balance;
        }
        round
    }

    fn apply(&mut self, group: &[&SubjectAssetDelta], delta: &SubjectAssetDelta) {
        if self.decimals == 0 && delta.decimals != 0 {
            self.decimals = delta.decimals;
        }
        self.reconcile_before(delta);

        let before = self.balance.max(0);
        let reported_after = delta.after_raw.and_then(|v| i128::try_from(v).ok());
        let computed_after = (before + delta.delta_raw).max(0);
        let after = reported_after.unwrap_or(computed_after);
        if reported_after.is_none() || reported_after != Some(computed_after) {
            // Either the chain did not tell us the resulting balance, or it disagrees
            // with what our arithmetic expected — both mean the history has a hole.
            self.history_complete = false;
            self.basis_complete = false;
        }

        if delta.delta_raw > 0 {
            self.apply_acquisition(group, delta, before, after);
        } else {
            self.apply_disposal(group, delta, before, after);
        }

        self.balance = after;
        if after <= 0 {
            self.closed_at = delta.block_time;
        }
    }

    /// The chain's own "balance before" is authoritative. If it disagrees with the
    /// balance we carried, we missed a transaction.
    fn reconcile_before(&mut self, delta: &SubjectAssetDelta) {
        let Some(before) = delta.before_raw.and_then(|v| i128::try_from(v).ok()) else {
            self.history_complete = false;
            self.basis_complete = false;
            return;
        };
        if before != self.balance {
            self.balance = before;
            self.total_acquired = self.total_acquired.max(before);
            self.history_complete = false;
            self.basis_complete = false;
        }
    }

    fn apply_acquisition(
        &mut self,
        group: &[&SubjectAssetDelta],
        delta: &SubjectAssetDelta,
        before: i128,
        after: i128,
    ) {
        let amount_raw = delta.delta_raw;
        let traded = delta.kind == DeltaKind::Trade;

        let kind = if traded {
            self.entry_count = self.entry_count.saturating_add(1);
            if before <= 0 {
                LedgerEventKind::Entry
            } else {
                LedgerEventKind::Add
            }
        } else {
            // Tokens that arrived without a trade have no cost. Treating that as a
            // zero-cost basis would render an infinite gain.
            self.basis_complete = false;
            LedgerEventKind::Receive
        };
        self.total_acquired += amount_raw;

        if self.entry_signature.is_none() && kind == LedgerEventKind::Entry {
            self.entry_signature = Some(delta.signature.clone());
        }
        if self.opened_at.is_none() && before <= 0 {
            self.opened_at = delta.block_time;
        }

        let amount = raw_to_whole(amount_raw, self.decimals);
        let quote = traded
            .then(|| quote_for(group, &delta.mint, true))
            .flatten();

        let mut price_sol = None;
        match quote {
            Some(leg) => {
                if self.accept_quote(leg) && amount > DUST {
                    self.invested += leg.amount;
                    self.remaining_basis += leg.amount;
                    self.priced_acquired += amount;
                    price_sol = Some(leg.amount / amount);
                }
            }
            None => {
                if traded {
                    // A trade whose consideration we cannot see (a token -> token swap
                    // with no SOL leg, or a route we did not decode).
                    self.basis_complete = false;
                }
            }
        }

        self.events.push(LedgerEvent {
            signature: delta.signature.clone(),
            slot: delta.slot,
            block_time: delta.block_time,
            kind,
            amount,
            balance_after: raw_to_whole(after, self.decimals),
            quote,
            price_sol,
            venue: delta.venue.clone(),
        });
    }

    /// Record the quote asset, returning whether it can contribute to a SOL basis.
    fn accept_quote(&mut self, leg: QuoteLeg) -> bool {
        match self.quote_asset {
            Some(existing) if existing != leg.asset => {
                // Half bought with SOL and half with USDC: neither number alone is the
                // basis, and they cannot be added together.
                self.basis_complete = false;
                self.proceeds_complete = false;
                return false;
            }
            None => self.quote_asset = Some(leg.asset),
            _ => {}
        }
        if leg.asset != QuoteAsset::Sol {
            // A real basis we simply cannot express in the SOL-denominated model.
            self.basis_complete = false;
            return false;
        }
        true
    }

    fn apply_disposal(
        &mut self,
        group: &[&SubjectAssetDelta],
        delta: &SubjectAssetDelta,
        before: i128,
        after: i128,
    ) {
        let amount_raw = if before > 0 {
            (-delta.delta_raw).min(before)
        } else {
            -delta.delta_raw
        };
        let traded = delta.kind == DeltaKind::Trade;
        let closing = after <= 0;

        let kind = if traded {
            self.exit_count = self.exit_count.saturating_add(1);
            if closing {
                LedgerEventKind::Exit
            } else {
                LedgerEventKind::PartialExit
            }
        } else {
            // Tokens sent out are not proceeds; the basis leaves with them.
            self.proceeds_complete = false;
            LedgerEventKind::Send
        };
        self.total_disposed += amount_raw;

        if closing && traded {
            self.exit_signature = Some(delta.signature.clone());
        }

        // Basis is released pro rata against the balance standing before this disposal.
        let allocated_basis = if before > 0 {
            let fraction = (amount_raw as f64 / before as f64).clamp(0.0, 1.0);
            self.remaining_basis * fraction
        } else {
            self.history_complete = false;
            self.basis_complete = false;
            0.0
        };
        self.remaining_basis = (self.remaining_basis - allocated_basis).max(0.0);

        let amount = raw_to_whole(amount_raw, self.decimals);
        let quote = traded
            .then(|| quote_for(group, &delta.mint, false))
            .flatten();

        let mut price_sol = None;
        if traded {
            match quote {
                Some(leg)
                    if leg.asset == QuoteAsset::Sol
                        && self.quote_asset != Some(QuoteAsset::Usd) =>
                {
                    self.realized_proceeds += leg.amount;
                    self.realized_cost += allocated_basis;
                    self.priced_disposed += amount;
                    if amount > DUST {
                        price_sol = Some(leg.amount / amount);
                    }
                }
                _ => self.proceeds_complete = false,
            }
        }

        self.events.push(LedgerEvent {
            signature: delta.signature.clone(),
            slot: delta.slot,
            block_time: delta.block_time,
            kind,
            amount,
            balance_after: raw_to_whole(after, self.decimals),
            quote,
            price_sol,
            venue: delta.venue.clone(),
        });

        if closing {
            self.remaining_basis = 0.0;
        }
    }

    fn finish(mut self, still_open: bool) -> LedgerRound {
        // A basis is only complete if every acquisition was priced in SOL and there was
        // at least one priced acquisition to price.
        let basis_complete = self.basis_complete
            && self.quote_asset == Some(QuoteAsset::Sol)
            && self.entry_count > 0;

        let average_entry_price_sol = (basis_complete && self.priced_acquired > DUST)
            .then(|| self.invested / self.priced_acquired);
        let average_exit_price_sol = (self.proceeds_complete && self.priced_disposed > DUST)
            .then(|| self.realized_proceeds / self.priced_disposed);
        let realized_pnl_sol = (basis_complete && self.proceeds_complete && self.exit_count > 0)
            .then(|| self.realized_proceeds - self.realized_cost);

        if !basis_complete {
            self.invested = 0.0;
            self.remaining_basis = 0.0;
        }

        LedgerRound {
            mint: self.mint,
            decimals: self.decimals,
            round_key: self.round_key,
            opened_at: self.opened_at,
            closed_at: if still_open { None } else { self.closed_at },
            is_open: still_open,
            balance_raw: self.balance.max(0) as u128,
            total_acquired_raw: self.total_acquired.max(0) as u128,
            total_disposed_raw: self.total_disposed.max(0) as u128,
            entry_count: self.entry_count,
            exit_count: self.exit_count,
            invested_sol: self.invested,
            remaining_basis_sol: self.remaining_basis,
            realized_proceeds_sol: self.realized_proceeds,
            realized_cost_sol: self.realized_cost,
            average_entry_price_sol,
            average_exit_price_sol,
            realized_pnl_sol,
            basis_complete,
            history_complete: self.history_complete,
            entry_signature: self.entry_signature,
            exit_signature: self.exit_signature,
            events: self.events,
        }
    }
}

// ==================== quote resolution ====================

/// Find what a movement of `target_mint` was traded against, inside the same
/// transaction.
///
/// Priority is native SOL, then wSOL, then USDC/USDT — the first leg found wins, so a
/// route that momentarily wraps SOL is still read as a SOL trade. Returning `None` is a
/// normal outcome (a token -> token swap has no reference leg at all) and the caller
/// degrades completeness rather than guessing.
fn quote_for(
    group: &[&SubjectAssetDelta],
    target_mint: &str,
    acquisition: bool,
) -> Option<QuoteLeg> {
    // One reference leg cannot be split across several positions. If this transaction
    // moved more than one non-reference mint in the SAME direction, the single SOL leg
    // belongs to no one position in particular, and attributing it to each would
    // overstate every basis involved. A token -> token swap is unaffected: its two legs
    // move in opposite directions.
    let same_direction_positions = group
        .iter()
        .filter(|delta| {
            !is_reference_asset(&delta.mint)
                && if acquisition {
                    delta.delta_raw > 0
                } else {
                    delta.delta_raw < 0
                }
        })
        .count();
    if same_direction_positions > 1 {
        return None;
    }

    // Buying a token spends the quote; selling it receives the quote.
    let matches_direction = |raw: i128| if acquisition { raw < 0 } else { raw > 0 };

    let candidate = |delta: &SubjectAssetDelta| -> Option<f64> {
        (delta.mint != target_mint && matches_direction(delta.delta_raw))
            .then(|| raw_to_whole(delta.delta_raw.abs(), delta.decimals))
    };

    let find = |wanted: &dyn Fn(&str) -> bool| -> Option<f64> {
        group
            .iter()
            .find_map(|delta| wanted(&delta.mint).then(|| candidate(delta)).flatten())
    };

    find(&|mint| mint == NATIVE_SOL_SENTINEL)
        .or_else(|| find(&|mint| mint == SOL_MINT))
        .map(|amount| QuoteLeg {
            asset: QuoteAsset::Sol,
            amount,
        })
        .or_else(|| {
            find(&|mint| mint == USDC_MINT || mint == USDT_MINT).map(|amount| QuoteLeg {
                asset: QuoteAsset::Usd,
                amount,
            })
        })
}

/// Assets that denominate trades rather than being positions themselves.
fn is_reference_asset(mint: &str) -> bool {
    matches!(mint, NATIVE_SOL_SENTINEL | SOL_MINT | USDC_MINT | USDT_MINT)
}
