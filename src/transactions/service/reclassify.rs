//! Offline reclassification of processed rows written by an older analyzer.
//!
//! `analysis_version` was written on every row and never read, so a classifier fix
//! only ever applied to transactions that arrived after it — the recorded history
//! kept whatever verdict it was first given. Every own-wallet transaction retains
//! its full jsonParsed response in `raw_transactions`, so bringing that history
//! forward needs no RPC at all: re-run the analyzer in cache-only mode.
//!
//! The sweep runs once per start, in bounded batches, off the hot path. It never
//! touches a row that is already current, and a row whose raw blob is missing (a
//! watched subject, which is decoded but never retained) is skipped rather than
//! retried forever.

use std::time::Duration;

use crate::chains::solana::transactions::processor::TransactionProcessor;
use crate::logger::{self, LogTag};
use crate::transactions::database::get_transaction_database;
use crate::transactions::types::{Subject, ANALYSIS_CACHE_VERSION};

/// Rows re-analyzed per batch before yielding.
const BATCH_SIZE: usize = 100;
/// Pause between batches so a large history cannot starve the runtime at boot.
const BATCH_PAUSE: Duration = Duration::from_millis(250);
/// Hard ceiling for one start, so a pathological history cannot run forever.
const MAX_ROWS_PER_RUN: usize = 20_000;

/// Re-derives every processed row older than [`ANALYSIS_CACHE_VERSION`].
pub async fn reclassify_stale_rows(subject: Subject) {
    let Some(database) = get_transaction_database().await else {
        return;
    };

    let processor =
        match TransactionProcessor::for_subject_with_cache_options(&subject, true, false) {
            Ok(processor) => processor,
            Err(e) => {
                logger::warning(
                    LogTag::Transactions,
                    &format!("Reclassification skipped: {e}"),
                );
                return;
            }
        };

    let mut total = 0usize;
    let mut failed = 0usize;

    loop {
        let signatures = match database
            .stale_analysis_signatures(subject.clone(), ANALYSIS_CACHE_VERSION, BATCH_SIZE)
            .await
        {
            Ok(signatures) => signatures,
            Err(e) => {
                logger::warning(
                    LogTag::Transactions,
                    &format!("Reclassification query failed: {e}"),
                );
                return;
            }
        };

        if signatures.is_empty() {
            break;
        }

        let batch = signatures.len();
        for signature in signatures {
            match processor.decode(&signature).await {
                Ok(transaction) => {
                    if let Err(e) = database
                        .store_processed_transaction(subject.clone(), &transaction)
                        .await
                    {
                        failed += 1;
                        logger::warning(
                            LogTag::Transactions,
                            &format!("Reclassification write failed for {signature}: {e}"),
                        );
                    }
                }
                Err(_) => {
                    // No cached raw response to re-read. Stamping the row current
                    // keeps the sweep from selecting it again on every start.
                    failed += 1;
                    let _ = database
                        .mark_analysis_version(subject.clone(), &signature, ANALYSIS_CACHE_VERSION)
                        .await;
                }
            }
            total += 1;
        }

        if total >= MAX_ROWS_PER_RUN {
            logger::warning(
                LogTag::Transactions,
                &format!("Reclassification stopped at the {MAX_ROWS_PER_RUN}-row ceiling; the remainder is picked up on the next start"),
            );
            break;
        }

        if batch < BATCH_SIZE {
            break;
        }
        tokio::time::sleep(BATCH_PAUSE).await;
    }

    if total > 0 {
        logger::info(
            LogTag::Transactions,
            &format!(
                "Reclassified {} transaction{} to analysis v{ANALYSIS_CACHE_VERSION} ({failed} without cached data)",
                total,
                if total == 1 { "" } else { "s" }
            ),
        );
    }
}
