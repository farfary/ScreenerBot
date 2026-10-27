// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the positions module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong managing a position.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    // --- persistence lifecycle ---
    #[error("the positions database is not initialised")]
    NotInitialised,
    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
    #[error("could not decode column {column} of a position row: {detail}")]
    RowDecode {
        column: &'static str,
        detail: String,
    },
    #[error("positions schema migration failed: {detail}")]
    SchemaMigration { detail: String },
    /// The store was written by a build with a newer positions schema; opening it with
    /// this build could lose what that schema added, so nothing is changed.
    #[error("positions database {operation} failed: {detail}")]
    Maintenance {
        operation: &'static str,
        detail: String,
    },

    // --- lookup ---
    #[error("no position for token {mint}")]
    NotFound { mint: String },
    #[error("no position with id {position_id}")]
    NotFoundById { position_id: i64 },
    #[error("no position with signature {signature}")]
    NotFoundBySignature { signature: String },
    #[error("token {mint} is not in the token store")]
    TokenNotFound { mint: String },
    /// The chain of a token could not be resolved from its address.
    #[error(transparent)]
    Chain(#[from] crate::chains::Error),

    // --- state conflicts ---
    #[error("an open position already exists for token {mint}: position {open_position_id}")]
    AlreadyOpen { mint: String, open_position_id: i64 },
    /// Another open of the token was sent and may still land, so a second is not sent.
    #[error("an open of token {mint} is still pending")]
    OpenPending { mint: String },
    /// A partial exit of the token is still being confirmed, so a second one is not sized
    /// against a holding the first may already have sold.
    #[error("a partial exit of token {mint} is still being confirmed")]
    PartialExitPending { mint: String },
    /// The store holds several open positions of one mint; the change would leave more than
    /// one of them active.
    #[error("token {mint} has more than one open position ({position_ids:?}); only one of them may be active")]
    DuplicateOpenRound {
        mint: String,
        position_ids: Vec<i64>,
    },
    #[error("position {position_id} is already closed")]
    AlreadyClosed { position_id: i64 },
    #[error("the entry transaction for position {position_id} is not verified")]
    UnverifiedEntryArchive { position_id: i64 },
    /// An insert was asked for a position that already has a row; an existing row changes
    /// only through a booking.
    #[error("position {position_id} is already stored")]
    AlreadyStored { position_id: i64 },
    /// A removal for an entry that never landed was refused: the row shows the entry
    /// `signature` landed, or the row's entry is a different signature.
    #[error(
        "the entry {signature} of position {position_id} landed; the position was not removed"
    )]
    EntryLanded { position_id: i64, signature: String },
    /// The wallet's holding of a mint cannot be split between its positions yet: another
    /// position's holding is not booked, or another swap of the mint is in flight.
    #[error("the holding of token {mint} cannot be attributed yet: {detail}")]
    HoldingUnattributable { mint: String, detail: String },
    #[error("persisted position {field} has unknown value '{value}'")]
    UnknownPersistedValue { field: &'static str, value: String },
    /// The positions store holds restored or recreated contents that the wallet history
    /// has not been reconciled against yet, so a change sized from the stored position
    /// is refused before anything is sent.
    #[error("the {transition} of token {mint} waits until the {state} positions store is reconciled with the wallet history")]
    StoreNotReconciled {
        transition: &'static str,
        mint: String,
        state: &'static str,
    },

    // --- validation ---
    #[error("invalid price {price} for token {mint}")]
    InvalidPrice { mint: String, price: f64 },
    #[error("invalid trade size {amount_sol} SOL: {reason}", amount_sol = .amount_native)]
    InvalidTradeSize { amount_native: f64, reason: String },
    #[error("invalid exit percentage {percent}: {reason}")]
    InvalidExitPercentage { percent: f64, reason: String },
    #[error("calculated exit amount for token {mint} is zero")]
    ZeroExitAmount { mint: String },
    #[error("DCA is disabled in configuration")]
    DcaDisabled,

    // --- transition and execution ---
    #[error("could not apply the {transition} transition to {mint}: {detail}")]
    TransitionFailed {
        transition: &'static str,
        mint: String,
        detail: String,
    },
    /// Checked token-amount arithmetic on a position overflowed; the stored amounts cannot
    /// represent the result, so nothing was changed.
    #[error("token amount arithmetic overflowed while {operation} for token {mint}")]
    AmountOverflow {
        mint: String,
        operation: &'static str,
    },
    /// No router would quote the trade. Distinct from [`Error::SwapFailed`]
    /// because nothing was built, signed or submitted — the trade stopped one
    /// step earlier, and the manual-trade timeline reports that step from this
    /// variant rather than by reading the message.
    #[error("could not quote a swap for token {mint}: {detail}")]
    QuoteFailed { mint: String, detail: String },
    /// The swap failed. `not_submitted` says why, when the last attempt stopped
    /// before its transaction was sent.
    #[error("swap for token {mint} failed: {detail}")]
    SwapFailed {
        mint: String,
        detail: String,
        not_submitted: Option<crate::swaps::NotSubmittedReason>,
    },
    /// The last send of a partial exit failed with no signature, so nothing proves whether
    /// it sold; the mint's partial exits are held until the wallet shows the outcome.
    #[error("the partial exit of token {mint} may have sold; further partial exits wait until the wallet shows the outcome: {detail}")]
    PartialExitHeld { mint: String, detail: String },
    #[error("wallet-history sync failed: {detail}")]
    WalletHistorySync { detail: String },

    /// The active wallet address could not be resolved (no fitting variant above:
    /// this is a config-layer failure, not a database, lookup or transition one,
    /// and it recurs at ~20 wallet-scoped query/write sites).
    #[error("could not resolve the active wallet address: {detail}")]
    WalletUnavailable { detail: String },

    /// The global position-capacity semaphore has no free permit (no fitting variant
    /// above: this is neither a per-mint conflict nor a validation failure).
    #[error("no free position slot ({remaining} remaining)")]
    SlotUnavailable { remaining: usize },

    /// The quote evidence of a paper leg could not be encoded for the paper store.
    #[error("could not encode the paper leg evidence: {detail}")]
    PaperLegEncode { detail: String },

    /// A paper fill lacks the quote evidence or the amounts its booking needs.
    #[error("the paper fill for {mint} cannot be booked: {detail}")]
    PaperFillUnbookable { mint: String, detail: String },

    /// The cash of the current paper run does not cover a paper buy of `mint`.
    #[error("the paper balance does not cover the buy of {mint}: {cash_native} SOL available, {needed_native} SOL needed")]
    PaperCashShort {
        mint: String,
        cash_native: f64,
        needed_native: f64,
    },

    /// A paper round was opened less than the open cooldown ago.
    #[error("the paper open cooldown is active: wait {wait_secs}s")]
    PaperOpenCooldown { wait_secs: u64 },

    /// The paper run a buy of `mint` was priced in has ended; the buy is not booked into
    /// the next one.
    #[error("the paper run the buy of {mint} was priced in has ended")]
    PaperRunEnded { mint: String },
}

impl Error {
    /// Why the swap stopped before it was sent, when this is such a failure.
    pub fn not_submitted_reason(&self) -> Option<&crate::swaps::NotSubmittedReason> {
        match self {
            Error::SwapFailed { not_submitted, .. } => not_submitted.as_ref(),
            _ => None,
        }
    }
}

/// Result alias for the positions module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            // Startup race — the caller may succeed if it waits for init.
            Error::NotInitialised => true,
            Error::Database(e) => e.is_retryable(),
            Error::Chain(e) => e.is_retryable(),
            // Lookups are a final verdict at the time of the call; nothing
            // about repeating the same call changes the answer.
            Error::NotFound { .. }
            | Error::NotFoundById { .. }
            | Error::NotFoundBySignature { .. }
            | Error::TokenNotFound { .. } => false,
            // State conflicts describe the world as it is right now.
            Error::AlreadyOpen { .. }
            | Error::OpenPending { .. }
            | Error::PartialExitPending { .. }
            | Error::AlreadyClosed { .. }
            | Error::AlreadyStored { .. }
            | Error::DuplicateOpenRound { .. }
            | Error::EntryLanded { .. }
            | Error::UnverifiedEntryArchive { .. }
            | Error::UnknownPersistedValue { .. }
            | Error::StoreNotReconciled { .. } => false,
            // Validation failures are a property of the input, not the attempt.
            Error::InvalidPrice { .. }
            | Error::InvalidTradeSize { .. }
            | Error::InvalidExitPercentage { .. }
            | Error::ZeroExitAmount { .. }
            | Error::DcaDisabled => false,
            // Money-state failures are not safely retried without a human
            // or the verifier reconciling what actually happened on chain.
            Error::RowDecode { .. }
            | Error::SchemaMigration { .. }
            | Error::TransitionFailed { .. }
            | Error::AmountOverflow { .. }
            | Error::QuoteFailed { .. }
            | Error::SwapFailed { .. }
            | Error::PartialExitHeld { .. } => false,
            Error::Maintenance { .. } => false,
            Error::WalletHistorySync { .. } => true,
            // The other swap or entry settles on its own; the holding is read again then.
            Error::HoldingUnattributable { .. } => true,
            Error::WalletUnavailable { .. } => false,
            Error::SlotUnavailable { .. } => true,
            Error::PaperLegEncode { .. } => false,
            Error::PaperFillUnbookable { .. } => false,
            Error::PaperCashShort { .. } | Error::PaperRunEnded { .. } => false,
            Error::PaperOpenCooldown { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::NotInitialised => Some(Duration::from_millis(500)),
            Error::Database(e) => e.retry_after(),
            Error::Chain(e) => e.retry_after(),
            Error::WalletHistorySync { .. } => Some(Duration::from_secs(2)),
            Error::PaperOpenCooldown { wait_secs } => Some(Duration::from_secs(*wait_secs)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::NotInitialised => Severity::Warning,
            Error::Database(e) => e.severity(),
            Error::Chain(e) => e.severity(),
            Error::RowDecode { .. } | Error::SchemaMigration { .. } => Severity::Critical,
            Error::Maintenance { .. } => Severity::Error,
            Error::NotFound { .. }
            | Error::NotFoundById { .. }
            | Error::NotFoundBySignature { .. }
            | Error::TokenNotFound { .. } => Severity::Info,
            Error::AlreadyOpen { .. }
            | Error::OpenPending { .. }
            | Error::PartialExitPending { .. }
            | Error::AlreadyClosed { .. }
            | Error::EntryLanded { .. }
            | Error::UnverifiedEntryArchive { .. }
            | Error::HoldingUnattributable { .. }
            | Error::UnknownPersistedValue { .. }
            | Error::StoreNotReconciled { .. } => Severity::Warning,
            Error::AlreadyStored { .. } | Error::DuplicateOpenRound { .. } => Severity::Error,
            Error::InvalidPrice { .. }
            | Error::InvalidTradeSize { .. }
            | Error::InvalidExitPercentage { .. }
            | Error::ZeroExitAmount { .. }
            | Error::DcaDisabled => Severity::Warning,
            Error::TransitionFailed { .. }
            | Error::AmountOverflow { .. }
            | Error::SwapFailed { .. }
            | Error::PartialExitHeld { .. } => Severity::Critical,
            // Nothing was submitted, so no money moved and no state is at risk.
            Error::QuoteFailed { .. } => Severity::Warning,
            Error::WalletHistorySync { .. } => Severity::Error,
            Error::WalletUnavailable { .. } => Severity::Critical,
            Error::SlotUnavailable { .. } => Severity::Info,
            Error::PaperLegEncode { .. } => Severity::Error,
            Error::PaperFillUnbookable { .. } => Severity::Error,
            // Expected outcomes of a simulated book: no money and no live state is involved.
            Error::PaperCashShort { .. }
            | Error::PaperOpenCooldown { .. }
            | Error::PaperRunEnded { .. } => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::NotInitialised => 503,
            Error::Database(e) => e.http_status(),
            Error::Chain(e) => e.http_status(),
            Error::NotFound { .. }
            | Error::NotFoundById { .. }
            | Error::NotFoundBySignature { .. }
            | Error::TokenNotFound { .. } => 404,
            Error::AlreadyOpen { .. }
            | Error::OpenPending { .. }
            | Error::PartialExitPending { .. }
            | Error::AlreadyClosed { .. }
            | Error::AlreadyStored { .. }
            | Error::DuplicateOpenRound { .. }
            | Error::EntryLanded { .. }
            | Error::UnverifiedEntryArchive { .. }
            | Error::HoldingUnattributable { .. }
            | Error::StoreNotReconciled { .. }
            | Error::ZeroExitAmount { .. } => 409,
            Error::UnknownPersistedValue { .. }
            | Error::InvalidPrice { .. }
            | Error::InvalidTradeSize { .. }
            | Error::InvalidExitPercentage { .. }
            | Error::DcaDisabled => 400,
            // The trade is well-formed; no provider would price it.
            Error::QuoteFailed { .. } => 422,
            Error::RowDecode { .. }
            | Error::SchemaMigration { .. }
            | Error::Maintenance { .. }
            | Error::TransitionFailed { .. }
            | Error::AmountOverflow { .. }
            | Error::SwapFailed { .. }
            | Error::PartialExitHeld { .. }
            | Error::WalletHistorySync { .. }
            | Error::WalletUnavailable { .. }
            | Error::PaperLegEncode { .. }
            | Error::PaperFillUnbookable { .. } => 500,
            Error::SlotUnavailable { .. } => 503,
            Error::PaperCashShort { .. }
            | Error::PaperOpenCooldown { .. }
            | Error::PaperRunEnded { .. } => 409,
        }
    }
}
