//! Collecting a token's activity from every source that holds a piece of it.
//!
//! Four sources, and each knows something the others do not:
//!
//!   1. the entry/exit RECORDS of every position ever opened on the mint — authoritative
//!      for what the position actually booked (amount, price, SOL, exit percentage);
//!   2. the positions' own `entry_transaction_signature` / `exit_transaction_signature` —
//!      the only trace of a swap that is submitted but not yet verified (no record yet);
//!   3. the PENDING REGISTRIES — a DCA add or a partial exit in flight is not stamped on
//!      the position at all, so until verification its signature lives nowhere else;
//!   4. the wallet's TRANSACTIONS — every tx that ever touched the mint, which is how
//!      transfers, airdrops and swaps made outside the bot get into the timeline.

use crate::positions::Position;
use crate::transactions::{
    get_transaction_database, TransactionListFilters, TransactionListRow, WSOL_MINT,
};

use super::super::types::{EntryRecordResponse, ExitRecordResponse};

/// Which side of the book an event sits on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Entry,
    Exit,
    /// Touched the token but belongs to no position — never moves a cost basis.
    Wallet,
}

impl Side {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Side::Entry => "entry",
            Side::Exit => "exit",
            Side::Wallet => "wallet",
        }
    }
}

/// One event before its on-chain half is resolved.
pub(super) struct Draft {
    pub kind: String,
    pub side: Side,
    pub signature: Option<String>,
    /// Record time, else the pending registry's submit time, else the position's own
    /// entry/exit time. The chain's timestamp is only a last resort — a swap whose
    /// transaction is not in the cache still has a real time.
    pub timestamp: Option<i64>,
    pub position_id: Option<i64>,
    pub position_index: Option<u32>,
    pub recorded: bool,
    pub record_id: Option<i64>,
    /// UI amount (whole tokens).
    pub token_amount: Option<f64>,
    pub price: Option<f64>,
    pub sol_amount: Option<f64>,
    pub exit_percentage: Option<f64>,
    pub record_fee_sol: Option<f64>,
    pub synthetic: bool,
    /// Wallet events carry their own chain data — no per-signature lookup needed.
    pub wallet: Option<TransactionListRow>,
}

impl Draft {
    fn new(kind: &str, side: Side, signature: Option<String>, timestamp: Option<i64>) -> Self {
        Self {
            kind: kind.to_owned(),
            side,
            signature,
            timestamp,
            position_id: None,
            position_index: None,
            recorded: false,
            record_id: None,
            token_amount: None,
            price: None,
            sol_amount: None,
            exit_percentage: None,
            record_fee_sol: None,
            synthetic: false,
            wallet: None,
        }
    }

    fn in_position(mut self, position: &Position, index: u32) -> Self {
        self.position_id = position.id;
        self.position_index = Some(index);
        self
    }
}

/// The swaps of ONE position (its records, its unverified signatures, its synthetic close).
pub(super) fn position_drafts(
    position: &Position,
    index: u32,
    entries: &[EntryRecordResponse],
    exits: &[ExitRecordResponse],
    to_ui: impl Fn(u64) -> f64,
) -> Vec<Draft> {
    let mut drafts: Vec<Draft> = Vec::with_capacity(entries.len() + exits.len() + 2);

    for entry in entries {
        let kind = if entry.is_dca { "dca" } else { "entry" };
        let mut draft = Draft::new(
            kind,
            Side::Entry,
            Some(entry.transaction_signature.clone()),
            Some(entry.timestamp),
        )
        .in_position(position, index);
        draft.recorded = true;
        draft.record_id = entry.id;
        draft.token_amount = Some(to_ui(entry.amount));
        draft.price = Some(entry.price);
        draft.sol_amount = Some(entry.sol_spent);
        draft.record_fee_sol = entry.fees_sol;
        push(draft, &mut drafts);
    }

    for exit in exits {
        let kind = if exit.is_partial {
            "partial_exit"
        } else {
            "exit"
        };
        let mut draft = Draft::new(
            kind,
            Side::Exit,
            Some(exit.transaction_signature.clone()),
            Some(exit.timestamp),
        )
        .in_position(position, index);
        draft.recorded = true;
        draft.record_id = exit.id;
        draft.token_amount = Some(to_ui(exit.amount));
        draft.price = Some(exit.price);
        draft.sol_amount = Some(exit.sol_received);
        draft.exit_percentage = Some(exit.percentage);
        draft.record_fee_sol = exit.fees_sol;
        push(draft, &mut drafts);
    }

    // Entry still confirming (or a legacy position that predates the records).
    push(
        Draft::new(
            "entry",
            Side::Entry,
            position.entry_transaction_signature.clone(),
            Some(position.entry_time.timestamp()),
        )
        .in_position(position, index),
        &mut drafts,
    );

    // Close still confirming.
    push(
        Draft::new(
            "exit",
            Side::Exit,
            position.exit_transaction_signature.clone(),
            position.exit_time.map(|time| time.timestamp()),
        )
        .in_position(position, index),
        &mut drafts,
    );

    // A position that exited without a usable signature (synthetic / force close) still owes
    // the user an exit row. An OPEN position that merely took partial profits has exit
    // records but has NOT exited — it must not get an empty "Exit" placeholder.
    let has_exited = position.exit_time.is_some() || position.synthetic_exit;
    if has_exited && !drafts.iter().any(|draft| draft.kind == "exit") {
        let mut draft = Draft::new(
            "exit",
            Side::Exit,
            None,
            position.exit_time.map(|time| time.timestamp()),
        )
        .in_position(position, index);
        draft.synthetic = true;
        drafts.push(draft);
    }

    drafts
}

/// Swaps submitted but not yet verified, which exist ONLY in the pending registries.
///
/// A DCA add and a partial exit are not stamped on the position — `entry_transaction_
/// signature` never changes and a partial's signature is deliberately kept off
/// `exit_transaction_signature` — so until verification writes their record this is the only
/// place they exist. `recorded` stays false: the amounts here are what was SUBMITTED, not
/// what the position has booked.
pub(super) async fn pending_drafts(
    position: &Position,
    index: u32,
    to_ui: impl Fn(u64) -> f64,
) -> Vec<Draft> {
    let mut drafts = Vec::new();

    for pending in crate::positions::get_pending_dca_swaps_for_mint(&position.mint).await {
        drafts.push(
            Draft::new(
                "dca",
                Side::Entry,
                Some(pending.signature),
                Some(pending.created_at.timestamp()),
            )
            .in_position(position, index),
        );
    }

    for pending in crate::positions::get_pending_partial_exits_for_mint(&position.mint).await {
        let mut draft = Draft::new(
            "partial_exit",
            Side::Exit,
            Some(pending.signature),
            Some(pending.created_at.timestamp()),
        )
        .in_position(position, index);
        draft.token_amount = Some(to_ui(pending.expected_exit_amount));
        draft.exit_percentage = Some(pending.requested_exit_percentage);
        drafts.push(draft);
    }

    drafts
}

/// Newest wallet transactions that touched the mint. Bounded — this is a scan.
const WALLET_EVENT_LIMIT: usize = 200;

/// Every wallet transaction that touched the mint but belongs to NO position: transfers in
/// and out, airdrops, ATA rent, and swaps the owner made in another app.
///
/// These carry their own chain data, so unlike a position swap they need no per-signature
/// lookup. `known` holds the signatures the positions already claimed.
pub(super) async fn wallet_drafts(mint: &str, known: &[String]) -> Vec<Draft> {
    let Some(db) = get_transaction_database().await else {
        return Vec::new();
    };

    let filters = TransactionListFilters {
        mint: Some(mint.to_owned()),
        ..Default::default()
    };

    let rows = match db
        .list_transactions(&filters, None, WALLET_EVENT_LIMIT)
        .await
    {
        Ok(result) => result.items,
        // No wallet yet (discovery-only boot) or a read error — the position swaps still
        // stand on their own.
        Err(_) => return Vec::new(),
    };

    rows.into_iter()
        .filter(|row| !known.iter().any(|signature| *signature == row.signature))
        .map(|row| {
            let kind = classify(&row);
            let mut draft = Draft::new(
                kind,
                Side::Wallet,
                Some(row.signature.clone()),
                Some(row.timestamp.timestamp()),
            );
            draft.token_amount = row.token_amount;
            draft.wallet = Some(row);
            draft
        })
        .collect()
}

/// What a wallet transaction did to the token. The raw `transaction_type` is surfaced as-is
/// in the expanded panel; this is only the label and the icon.
fn classify(row: &TransactionListRow) -> &'static str {
    let kind = row
        .transaction_type
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();

    if kind.contains("ata") {
        return "ata";
    }
    if row.router.is_some()
        || kind.contains("swap")
        || kind.contains("buy")
        || kind.contains("sell")
    {
        // SOL left the wallet to acquire the token, or came back for selling it.
        return if row.sol_delta < 0.0 { "buy" } else { "sell" };
    }
    if kind.contains("transfer") {
        return "transfer";
    }
    "other"
}

/// The mint of the token itself, never wSOL — guards a caller passing the wrapped-SOL mint,
/// which every swap touches and which would match the whole wallet.
pub(super) fn is_tradeable_mint(mint: &str) -> bool {
    mint != WSOL_MINT
}

/// Add a draft unless it has no signature or one already claimed.
///
/// A swap without a signature is not a swap. An OPEN position has no
/// `exit_transaction_signature`, and admitting that `None` would give every open position a
/// phantom "Exit" row. The ONLY signature-less event is the synthetic close, appended
/// directly by the caller.
fn push(draft: Draft, drafts: &mut Vec<Draft>) {
    let Some(signature) = draft.signature.as_deref() else {
        return;
    };
    if signature.is_empty() {
        return;
    }
    if drafts
        .iter()
        .any(|existing| existing.signature.as_deref() == Some(signature))
    {
        return;
    }
    drafts.push(draft);
}
