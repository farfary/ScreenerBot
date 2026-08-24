//! Per-subject persistence policy (§5.4).
//!
//! | Field | Own wallet | Watched target |
//! |---|---|---|
//! | `raw_transactions.raw_transaction_data` | stored | not stored (the injected runtime's `decode_transaction(is_own=false)` skips it) |
//! | `processed_transactions` | stored | stored |
//! | retention | unlimited | rolling window (`wallet.watch_retention_days`) |
//! | `record_transaction_event` | yes | no -- the watch service emits its own `WalletActivity` |
//!
//! The raw-JSON split happens earlier, inside the runtime's decode step; this
//! module only decides the two things that are actually about RECORDING: whether an
//! event fires, and whether the failed-transaction path (which does not classify)
//! still gets a row.

use crate::errors::InternalError;
use crate::events;
use crate::logger::{self, LogTag};
use crate::transactions::database::get_transaction_database;
use crate::transactions::types::{Subject, Transaction};
use crate::wallets::Error;

use super::types::WatchSource;

/// Persist a decoded transaction per the subject's policy. Call this AFTER
/// classification (or after the failed-transaction short-circuit) and BEFORE the
/// dedupe commit -- see `dedupe`'s doc for why the ordering matters.
pub(super) async fn record(
    subject: Subject,
    sources: &[WatchSource],
    transaction: &Transaction,
) -> Result<(), Error> {
    let is_own_wallet = sources.contains(&WatchSource::OwnWallet);

    let Some(db) = get_transaction_database().await else {
        logger::warning(
            LogTag::WalletWatch,
            "Transaction database not initialized; dropping a decoded row",
        );
        return Err(Error::Internal(InternalError::InvariantViolation {
            message: "transaction database not initialized".to_owned(),
        }));
    };

    if let Err(e) = db
        .store_processed_transaction(subject.clone(), transaction)
        .await
    {
        logger::warning(
            LogTag::WalletWatch,
            &format!(
                "Failed to store processed transaction {} for {}: {e}",
                transaction.signature, subject
            ),
        );
        return Err(e.into());
    }

    // The own wallet's ledger rows. Detection moved to this funnel, so this is the
    // LIVE half of `subject_asset_deltas`: without it a swap made while the bot runs is
    // stored and marked known, and the bootstrap that would have extracted its deltas
    // skips it forever after — the position it opens or closes never appears. A failure
    // is not fatal to recording: the boot gap-fill re-scans anything missing.
    if is_own_wallet {
        if let Err(e) = db
            .store_transaction_deltas(&subject.address(), transaction)
            .await
        {
            logger::warning(
                LogTag::WalletWatch,
                &format!(
                    "Failed to store subject deltas for {}: {e}",
                    transaction.signature
                ),
            );
        }
    }

    // Only the own wallet's transactions drive the events feed / position
    // verification side effects -- a watched target's activity is surfaced through
    // `WalletActivity` instead, which is what the alert (and, later, copy) consumers
    // subscribe to. Recording it as an own-wallet-shaped event too would double up
    // the dashboard's notification centre for every trade a followed wallet makes.
    if is_own_wallet {
        events::record_transaction_event(
            &transaction.signature,
            "processed",
            transaction.success,
            transaction.fee_lamports,
            transaction.slot,
            None,
        )
        .await;
    }

    Ok(())
}
