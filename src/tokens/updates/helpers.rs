// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token update helpers — utility functions for token data enrichment and merging.

use super::blocking::blocking_db;
use super::core::UpdateResult;
use crate::chains::ChainId;
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;
use std::collections::HashSet;
use std::sync::Mutex as StdMutex;
use std::sync::{Arc, LazyLock};

/// Number of consecutive "not listed" answers before a token is marked permanently failed for
/// market data
pub(super) const PERMANENT_FAILURE_THRESHOLD: u32 = 3;

/// In-flight token tracking to prevent duplicate fetches across loops
pub(super) static IN_FLIGHT_TOKENS: LazyLock<StdMutex<HashSet<(ChainId, String)>>> =
    LazyLock::new(|| StdMutex::new(HashSet::new()));

/// Try to mark a token as in-flight. Returns true if marked, false if already in-flight.
pub(super) fn try_mark_in_flight(chain: ChainId, mint: &str) -> bool {
    if let Ok(mut set) = IN_FLIGHT_TOKENS.lock() {
        set.insert((chain, mint.to_string()))
    } else {
        true // If lock poisoned, allow fetch
    }
}

/// Clear in-flight marker for a token
pub(super) fn clear_in_flight(chain: ChainId, mint: &str) {
    if let Ok(mut set) = IN_FLIGHT_TOKENS.lock() {
        set.remove(&(chain, mint.to_string()));
    }
}

/// Check if updates should be paused due to active tools
pub(super) fn should_skip_for_tools() -> bool {
    crate::global::are_tools_active()
}

/// How a market-data fetch failed. Stored as `market_error_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketFetchFailure {
    /// Every asked source answered and none lists the token. The only
    /// classification that counts toward [`PERMANENT_FAILURE_THRESHOLD`].
    NotListed,
    /// At least one source did not answer (endpoint down, request failed, rate
    /// limit). Retried with backoff and never marks the token permanent.
    Temporary,
}

impl MarketFetchFailure {
    /// The `market_error_type` column value.
    pub const fn error_type(self) -> &'static str {
        match self {
            Self::NotListed => "not_listed",
            Self::Temporary => "temporary",
        }
    }

    /// Classify the answers of the market sources asked for one token. `None`
    /// when a source lists the token; `NotListed` only when every source
    /// answered and none lists it; `Temporary` otherwise, including when no
    /// source was asked.
    pub(super) fn from_answers(answers: &[MarketAnswer]) -> Option<Self> {
        if answers.contains(&MarketAnswer::Listed) {
            None
        } else if !answers.is_empty() && answers.iter().all(|a| *a == MarketAnswer::NotListed) {
            Some(Self::NotListed)
        } else {
            Some(Self::Temporary)
        }
    }
}

/// What one market source said about one token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MarketAnswer {
    /// The source returned market data.
    Listed,
    /// The source answered and has no pair for the token.
    NotListed,
    /// The source could not be asked or did not answer.
    Failed,
}

impl MarketAnswer {
    /// Read a fetcher result: `Ok(None)` is the answer "not listed", `Err` is a failure.
    pub(super) fn of<T>(result: &TokenResult<Option<T>>) -> Self {
        match result {
            Ok(Some(_)) => Self::Listed,
            Ok(None) => Self::NotListed,
            Err(_) => Self::Failed,
        }
    }
}

/// Whether a recorded market error makes the token permanently failed:
/// consecutive "not listed" answers reached the threshold.
fn reaches_permanent(failure: MarketFetchFailure, error_count: u32) -> bool {
    failure == MarketFetchFailure::NotListed && error_count >= PERMANENT_FAILURE_THRESHOLD
}

/// Record a market error and potentially mark as permanent failure
/// Returns true if the token was marked as permanently failed
pub(super) async fn handle_market_failure(db: &Arc<TokenDatabase>, result: &UpdateResult) -> bool {
    let mint = result.mint.as_str();
    let failure = result
        .market_failure
        .unwrap_or(MarketFetchFailure::Temporary);
    let error_type = failure.error_type();
    let message = result.failures.join(" | ");

    let record_mint = mint.to_string();
    let recorded = blocking_db(db, move |db| {
        db.record_market_error(&record_mint, &message, error_type)
    })
    .await;

    match recorded {
        Ok(error_count) => {
            if reaches_permanent(failure, error_count) {
                // Update to permanent status (without incrementing count again)
                let permanent_mint = mint.to_string();
                if let Err(e) =
                    blocking_db(db, move |db| db.mark_market_permanent(&permanent_mint)).await
                {
                    logger::error(
                        LogTag::Tokens,
                        &format!("Failed to mark {mint} as permanent failure: {e}"),
                    );
                    return false;
                }
                logger::debug(
                    LogTag::Tokens,
                    &format!(
                        "Marked {} as permanently failed for market data after {} attempts (not listed on any exchange)",
                        mint, error_count
                    ),
                );
                return true;
            }
        }
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to record market error for {mint}: {e}"),
            );
        }
    }
    false
}

/// Filter out the token currently being viewed in the dashboard
/// Dashboard-active tokens get priority updates via the UI, so skip them in batch updates
pub(super) fn filter_dashboard_active_token(tokens: Vec<String>) -> Vec<String> {
    if let Some(active_mint) = crate::global::get_dashboard_active_token() {
        let original_count = tokens.len();
        let filtered: Vec<String> = tokens.into_iter().filter(|m| m != &active_mint).collect();
        if filtered.len() < original_count {
            logger::debug(
                LogTag::Tokens,
                &format!(
                    "Skipping dashboard-active token {} in batch update (getting priority updates via UI)",
                    active_mint
                ),
            );
        }
        filtered
    } else {
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::Error;

    const LISTED: MarketAnswer = MarketAnswer::Listed;
    const NOT_LISTED: MarketAnswer = MarketAnswer::NotListed;
    const FAILED: MarketAnswer = MarketAnswer::Failed;

    fn store() -> (tempfile::TempDir, Arc<TokenDatabase>) {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("tokens.db");
        let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana)
            .expect("open tokens database");
        db.upsert_token("mint-a", None, None, None)
            .expect("insert token");
        (directory, Arc::new(db))
    }

    fn failed_update(answers: &[MarketAnswer]) -> UpdateResult {
        UpdateResult {
            mint: "mint-a".to_owned(),
            successes: Vec::new(),
            failures: vec!["DexScreener: failure".to_owned()],
            market_failure: MarketFetchFailure::from_answers(answers),
        }
    }

    fn tracked(db: &TokenDatabase) -> (u64, Option<String>) {
        let info = db
            .get_update_tracking_info("mint-a")
            .expect("read tracking")
            .expect("tracking row");
        (info.market_error_count, info.market_error_type)
    }

    #[test]
    fn each_failure_maps_to_its_stored_error_type() {
        assert_eq!(MarketFetchFailure::NotListed.error_type(), "not_listed");
        assert_eq!(MarketFetchFailure::Temporary.error_type(), "temporary");
    }

    #[test]
    fn answers_classify_by_what_every_source_said() {
        assert_eq!(MarketFetchFailure::from_answers(&[LISTED]), None);
        assert_eq!(
            MarketFetchFailure::from_answers(&[NOT_LISTED, LISTED]),
            None
        );
        assert_eq!(
            MarketFetchFailure::from_answers(&[NOT_LISTED]),
            Some(MarketFetchFailure::NotListed)
        );
        assert_eq!(
            MarketFetchFailure::from_answers(&[NOT_LISTED, NOT_LISTED]),
            Some(MarketFetchFailure::NotListed)
        );
        assert_eq!(
            MarketFetchFailure::from_answers(&[NOT_LISTED, FAILED]),
            Some(MarketFetchFailure::Temporary)
        );
        assert_eq!(
            MarketFetchFailure::from_answers(&[FAILED]),
            Some(MarketFetchFailure::Temporary)
        );
        assert_eq!(
            MarketFetchFailure::from_answers(&[]),
            Some(MarketFetchFailure::Temporary)
        );
    }

    #[test]
    fn a_fetch_error_is_a_failed_answer_and_only_an_empty_answer_is_not_listed() {
        let failed: TokenResult<Option<()>> = Err(Error::Api {
            provider: "DexScreener".to_owned(),
            message: "unavailable".to_owned(),
        });
        assert_eq!(MarketAnswer::of(&failed), FAILED);
        assert_eq!(MarketAnswer::of::<()>(&Ok(None)), NOT_LISTED);
        assert_eq!(MarketAnswer::of(&Ok(Some(()))), LISTED);
    }

    #[tokio::test]
    async fn a_failed_dexscreener_batch_never_counts_toward_not_listed() {
        let (_directory, db) = store();
        let failed = failed_update(&[FAILED]);

        for attempt in 1..=PERMANENT_FAILURE_THRESHOLD + 2 {
            assert!(!handle_market_failure(&db, &failed).await);
            assert_eq!(
                tracked(&db),
                (u64::from(attempt), Some("temporary".to_owned()))
            );
        }

        // Failures followed by one "not listed" answer start the not-listed count at one.
        assert!(!handle_market_failure(&db, &failed_update(&[NOT_LISTED])).await);
        assert_eq!(tracked(&db), (1, Some("not_listed".to_owned())));
    }

    #[tokio::test]
    async fn a_listed_answer_without_a_pair_counts_as_not_listed() {
        let (_directory, db) = store();
        let not_listed = failed_update(&[NOT_LISTED]);

        for attempt in 1..PERMANENT_FAILURE_THRESHOLD {
            assert!(!handle_market_failure(&db, &not_listed).await);
            assert_eq!(
                tracked(&db),
                (u64::from(attempt), Some("not_listed".to_owned()))
            );
        }

        assert!(handle_market_failure(&db, &not_listed).await);
        assert_eq!(
            tracked(&db),
            (
                u64::from(PERMANENT_FAILURE_THRESHOLD),
                Some("permanent".to_owned())
            )
        );
    }

    #[tokio::test]
    async fn a_failure_between_not_listed_answers_restarts_the_not_listed_count() {
        let (_directory, db) = store();
        let not_listed = failed_update(&[NOT_LISTED]);

        for _ in 1..PERMANENT_FAILURE_THRESHOLD {
            handle_market_failure(&db, &not_listed).await;
        }
        handle_market_failure(&db, &failed_update(&[FAILED])).await;

        assert!(!handle_market_failure(&db, &not_listed).await);
        assert_eq!(tracked(&db), (1, Some("not_listed".to_owned())));
    }
}
