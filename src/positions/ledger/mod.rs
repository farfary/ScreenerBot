//! Wallet-history ledger — deterministic position rounds from observed balance deltas.
//!
//! The truth this module reduces over is the LOCAL, fully-processed transaction store:
//! `subject_asset_deltas` rows produced by [`crate::transactions::deltas`]. It makes no
//! network call, reads no price feed and asks no provider what a position "should" be.
//! Given the same rows it always produces the same rounds.
//!
//! # What a round is
//!
//! One round is one `0 -> ... -> 0` lifecycle of a single mint in one wallet. The first
//! acquisition that lifts the balance off zero OPENS it; the disposal that returns the
//! balance to zero CLOSES it. Buying the same mint again later is a NEW round, never a
//! reopen of the old one. Within a round, a second buy is an add (DCA) and a sell that
//! leaves a remainder is a partial exit — every entry and every exit is kept as its own
//! [`LedgerEvent`] so the dashboard can show them individually rather than as one
//! averaged blob.
//!
//! A token -> token swap needs no special case: the sold mint's balance hits zero (its
//! round closes) and the bought mint's balance leaves zero (a round opens) inside the
//! same signature.
//!
//! # Honesty over completeness
//!
//! SOL is the monetary unit, so a cost basis is only ever taken from a SOL or wSOL leg.
//! An airdrop, a USD-quoted fill, a mixed-quote round, a token -> token swap with no
//! SOL leg, or history that does not reconcile against the observed balance all clear
//! [`LedgerRound::basis_complete`] / [`LedgerRound::history_complete`] instead of being
//! assigned an invented number. USD is never back-converted to SOL at today's rate: a
//! round bought with USDC has a real basis we simply cannot express in SOL, and
//! guessing it would produce a plausible, permanently wrong P&L.
//!
//! A round with `basis_complete == false` must never be rendered with a P&L.

mod reducer;
pub mod sync;

pub use reducer::{reconcile_with_wallet, reduce_rounds};
pub use sync::{
    schedule_resync, sync_wallet_history, RoundMetadata, SyncPlan, SyncSummary, CLOSED_EXTERNALLY,
};

/// Whole-unit amounts below this are treated as zero. Raw integer balances are exact;
/// this guards only the derived `f64` money math.
pub const DUST: f64 = 1e-12;

/// The asset a trade's consideration was denominated in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteAsset {
    /// Native SOL or wSOL — the only basis the position model can store.
    Sol,
    /// USDC/USDT. Recorded for display, never converted into a SOL basis.
    Usd,
}

impl QuoteAsset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sol => "SOL",
            Self::Usd => "USD",
        }
    }
}

/// The consideration leg of one trade, in whole units of [`QuoteAsset`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuoteLeg {
    pub asset: QuoteAsset,
    pub amount: f64,
}

/// What one balance movement meant for its round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEventKind {
    /// Traded acquisition that opened the round.
    Entry,
    /// Traded acquisition into an already-open round (DCA).
    Add,
    /// Traded disposal that left a remainder.
    PartialExit,
    /// Traded disposal that returned the balance to zero.
    Exit,
    /// Non-trade acquisition (airdrop, incoming transfer) — no cost basis.
    Receive,
    /// Non-trade disposal (outgoing transfer) — not proceeds.
    Send,
}

impl LedgerEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Add => "add",
            Self::PartialExit => "partial_exit",
            Self::Exit => "exit",
            Self::Receive => "receive",
            Self::Send => "send",
        }
    }

    pub fn is_acquisition(self) -> bool {
        matches!(self, Self::Entry | Self::Add | Self::Receive)
    }
}

/// One balance movement inside a round, kept individually so entries and exits are
/// shown separately rather than collapsed into an average.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEvent {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub kind: LedgerEventKind,
    /// Absolute size of the movement, whole tokens.
    pub amount: f64,
    /// Round balance after this movement, whole tokens.
    pub balance_after: f64,
    /// Consideration observed in the same transaction, when there was one.
    pub quote: Option<QuoteLeg>,
    /// Price per whole token in SOL. `None` whenever the quote was not SOL.
    pub price_sol: Option<f64>,
    pub venue: Option<String>,
}

/// One `0 -> ... -> 0` lifecycle of one mint.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerRound {
    pub mint: String,
    pub decimals: u8,
    /// Stable identity: `"<signature>:<mint>"` for a round whose opening acquisition we
    /// observed, `"genesis:<mint>"` for one already open before our history begins.
    /// Qualified by mint because one transaction can open rounds in several mints.
    pub round_key: String,
    pub opened_at: Option<i64>,
    pub closed_at: Option<i64>,
    pub is_open: bool,
    /// Current holding in raw base units — exact, never rounded.
    pub balance_raw: u128,
    pub total_acquired_raw: u128,
    pub total_disposed_raw: u128,
    /// Traded acquisitions (entry + adds). Excludes `Receive`.
    pub entry_count: u32,
    /// Traded disposals (partials + final exit). Excludes `Send`.
    pub exit_count: u32,
    /// SOL paid in across every priced acquisition. Meaningful only when
    /// `basis_complete`.
    pub invested_sol: f64,
    /// Cost basis still attached to the remaining balance.
    pub remaining_basis_sol: f64,
    /// SOL received across every priced disposal.
    pub realized_proceeds_sol: f64,
    /// Basis consumed by those disposals.
    pub realized_cost_sol: f64,
    pub average_entry_price_sol: Option<f64>,
    pub average_exit_price_sol: Option<f64>,
    /// `Some` only when both the basis and the proceeds are fully established.
    pub realized_pnl_sol: Option<f64>,
    /// False when the cost basis could not be established from what we observed.
    pub basis_complete: bool,
    /// False when the observed deltas do not reconcile with the balances we saw.
    pub history_complete: bool,
    /// Signature of the opening acquisition, when observed.
    pub entry_signature: Option<String>,
    /// Signature of the disposal that closed the round, when observed.
    pub exit_signature: Option<String>,
    pub events: Vec<LedgerEvent>,
}

impl LedgerRound {
    /// Balance in whole tokens.
    pub fn balance(&self) -> f64 {
        raw_to_whole(self.balance_raw as i128, self.decimals)
    }

    /// True when this round was reconstructed without ever seeing its opening
    /// acquisition.
    pub fn is_genesis(&self) -> bool {
        self.round_key.starts_with("genesis:")
    }

    /// The latest moment this round was observed to exist.
    ///
    /// The honest close stamp for a round the wallet no longer holds but whose disposal
    /// we never saw. "When we noticed" would be today's date, which ranks a holding
    /// abandoned months ago above the wallet's genuinely most recent exit; the last
    /// event we did see is a fact, and it orders the Closed tab truthfully.
    pub fn last_seen_at(&self) -> Option<i64> {
        self.events
            .iter()
            .filter_map(|event| event.block_time)
            .max()
            .or(self.opened_at)
    }
}

/// A currently-held balance read from the wallet, used to reconcile the reduced rounds
/// against on-chain truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletHolding {
    pub mint: String,
    pub amount_raw: u128,
    pub decimals: u8,
}

/// Convert raw base units to whole units.
pub(crate) fn raw_to_whole(raw: i128, decimals: u8) -> f64 {
    raw as f64 / 10f64.powi(decimals as i32)
}
