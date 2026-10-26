// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Promo generators for the Filtering page: the snapshot counts, the rejection
//! breakdowns and the rejected-token lists behind `/api/filtering/*`.
//!
//! Everything is read from the token universe in `tokens.rs`, so the Filtering
//! tabs, the Tokens page and the header count one corpus: a reason's count is the
//! number of universe tokens whose latest rejection carries it, and a token listed
//! under a reason is a row the Tokens page shows with that same reason. A time
//! range counts the tokens whose rejection started inside it, the population the
//! production history table counts.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::LazyLock;

use chrono::{DateTime, Duration, Utc};

use crate::filtering::FilteringStatsSnapshot;
use crate::tokens::{types::Token, RecentRejectionRow};

use super::tokens::{get_promo_tokens_stats, promo_universe};

/// Age of the snapshot when the session starts, so the first frame reads "1m ago"
/// rather than a refresh nobody pressed.
const PROMO_SNAPSHOT_AGE_SECS: i64 = 70;

/// When the promo snapshot was last rebuilt, in Unix seconds. Refresh moves it to now.
static REFRESHED_AT: LazyLock<AtomicI64> = LazyLock::new(|| {
    AtomicI64::new((Utc::now() - Duration::seconds(PROMO_SNAPSHOT_AGE_SECS)).timestamp())
});

fn rejected() -> impl Iterator<Item = &'static Token> {
    promo_universe()
        .filter(|(_, passed)| !passed)
        .map(|(token, _)| token)
}

fn reason(token: &Token) -> &str {
    token.last_rejection_reason.as_deref().unwrap_or_default()
}

fn source(token: &Token) -> &str {
    token.last_rejection_source.as_deref().unwrap_or_default()
}

fn since(token: &Token) -> i64 {
    token
        .last_rejection_at
        .map(|at| at.timestamp())
        .unwrap_or_default()
}

/// Rejected tokens, newest rejection first; ties fall back to the mint so every
/// page of a list is stable.
fn rejected_newest_first() -> Vec<&'static Token> {
    let mut tokens: Vec<&Token> = rejected().collect();
    tokens.sort_by(|a, b| since(b).cmp(&since(a)).then_with(|| a.mint.cmp(&b.mint)));
    tokens
}

/// The filtering snapshot of the promo universe.
pub fn get_promo_filtering_snapshot() -> FilteringStatsSnapshot {
    let (total, priced, positions, blacklisted, with_ohlcv) = get_promo_tokens_stats();
    let refreshed = REFRESHED_AT.load(Ordering::Relaxed);
    FilteringStatsSnapshot {
        total_tokens_in_database: total,
        total_tokens: total,
        with_pool_price: priced,
        open_positions: positions,
        blacklisted,
        with_ohlcv,
        passed_filtering: total - rejected().count(),
        awaiting_rugcheck_report: 0,
        updated_at: DateTime::from_timestamp(refreshed, 0).unwrap_or_else(Utc::now),
    }
}

/// A refresh rebuilds the promo snapshot: the corpus is fixed, so only its time moves.
pub fn record_promo_filtering_refresh() {
    REFRESHED_AT.store(Utc::now().timestamp(), Ordering::Relaxed);
}

/// Rejected tokens per (reason, source), most first, optionally only those whose
/// rejection started inside `[start, end]`.
pub fn get_promo_rejection_counts(
    start: Option<i64>,
    end: Option<i64>,
) -> Vec<(String, String, i64)> {
    let mut counts: BTreeMap<(&str, &str), i64> = BTreeMap::new();
    for token in rejected() {
        let at = since(token);
        if start.is_some_and(|start| at < start) || end.is_some_and(|end| at > end) {
            continue;
        }
        *counts.entry((reason(token), source(token))).or_default() += 1;
    }
    let mut rows: Vec<(String, String, i64)> = counts
        .into_iter()
        .map(|((reason, source), count)| (reason.to_owned(), source.to_owned(), count))
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2));
    rows
}

/// The newest rejections, with each token's identity.
pub fn get_promo_recent_rejections(limit: usize) -> Vec<RecentRejectionRow> {
    rejected_newest_first()
        .into_iter()
        .take(limit)
        .map(|token| {
            (
                token.mint.clone(),
                reason(token).to_owned(),
                source(token).to_owned(),
                since(token),
                Some(token.symbol.clone()),
                Some(token.name.clone()),
                token.image_url.clone(),
            )
        })
        .collect()
}

/// One page of rejected tokens as (mint, reason, source, Unix time), newest first,
/// filtered like the production query: exact reason and source, and a search that
/// matches the mint, symbol or name anywhere, ignoring case.
pub fn get_promo_rejected_tokens(
    reason_filter: Option<&str>,
    source_filter: Option<&str>,
    search: Option<&str>,
    limit: usize,
    offset: usize,
) -> Vec<(String, String, String, i64)> {
    let needle = search.map(str::to_lowercase);
    rejected_newest_first()
        .into_iter()
        .filter(|token| reason_filter.is_none_or(|wanted| reason(token) == wanted))
        .filter(|token| source_filter.is_none_or(|wanted| source(token) == wanted))
        .filter(|token| {
            needle.as_deref().is_none_or(|needle| {
                token.mint.to_lowercase().contains(needle)
                    || token.symbol.to_lowercase().contains(needle)
                    || token.name.to_lowercase().contains(needle)
            })
        })
        .skip(offset)
        .take(limit)
        .map(|token| {
            (
                token.mint.clone(),
                reason(token).to_owned(),
                source(token).to_owned(),
                since(token),
            )
        })
        .collect()
}

/// Symbol, name and logo per mint, as the token info batch returns them.
pub fn get_promo_token_info(
    mints: &[String],
) -> HashMap<String, (Option<String>, Option<String>, Option<String>)> {
    promo_universe()
        .filter(|(token, _)| mints.contains(&token.mint))
        .map(|(token, _)| {
            (
                token.mint.clone(),
                (
                    Some(token.symbol.clone()),
                    Some(token.name.clone()),
                    token.image_url.clone(),
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Passed and rejected split the corpus, and the reason counts add up to the
    /// rejected total, per source too: the Status chips, the reason list and the
    /// header describe the same tokens.
    #[test]
    fn counts_reconcile_with_the_snapshot() {
        let snapshot = get_promo_filtering_snapshot();
        let counts = get_promo_rejection_counts(None, None);
        let rejected: i64 = counts.iter().map(|(_, _, count)| count).sum();

        assert_eq!(rejected as usize, snapshot.rejected());
        assert!(snapshot.passed_filtering > 0);
        assert!(counts.windows(2).all(|pair| pair[0].2 >= pair[1].2));

        let sources: HashSet<&str> = counts
            .iter()
            .map(|(_, source, _)| source.as_str())
            .collect();
        for expected in [
            "core",
            "onchain",
            "dexscreener",
            "geckoterminal",
            "rugcheck",
        ] {
            assert!(sources.contains(expected), "no rejection from {expected}");
        }
    }

    /// The recent list is newest first and reads like a live session: several
    /// different reasons, not one reason repeated down the panel.
    #[test]
    fn recent_rejections_are_newest_first_and_varied() {
        let recent = get_promo_recent_rejections(20);
        assert_eq!(recent.len(), 20);
        assert!(recent.windows(2).all(|pair| pair[0].3 >= pair[1].3));
        let reasons: HashSet<&str> = recent.iter().map(|row| row.1.as_str()).collect();
        assert!(reasons.len() >= 4, "only {reasons:?}");
    }

    /// Every listed token is a universe token rejected for the reason asked, its
    /// identity resolves, and consecutive pages do not overlap.
    #[test]
    fn reason_pages_list_the_tokens_counted_under_the_reason() {
        let (reason, _, count) = get_promo_rejection_counts(None, None)
            .into_iter()
            .next()
            .expect("at least one reason");
        let first = get_promo_rejected_tokens(Some(&reason), None, None, 50, 0);
        let second = get_promo_rejected_tokens(Some(&reason), None, None, 50, 50);
        let all = get_promo_rejected_tokens(Some(&reason), None, None, usize::MAX, 0);

        assert_eq!(all.len() as i64, count);
        assert!(all.iter().all(|row| row.1 == reason));
        let first_mints: HashSet<&str> = first.iter().map(|row| row.0.as_str()).collect();
        assert!(second
            .iter()
            .all(|row| !first_mints.contains(row.0.as_str())));

        let mints: Vec<String> = first.iter().map(|row| row.0.clone()).collect();
        assert_eq!(get_promo_token_info(&mints).len(), mints.len());
    }

    /// A range counts only rejections that started inside it.
    #[test]
    fn a_time_range_counts_rejections_that_started_inside_it() {
        let now = Utc::now().timestamp();
        let hour: i64 = get_promo_rejection_counts(Some(now - 3_600), Some(now))
            .iter()
            .map(|(_, _, count)| count)
            .sum();
        let all: i64 = get_promo_rejection_counts(None, None)
            .iter()
            .map(|(_, _, count)| count)
            .sum();
        let started = rejected()
            .filter(|token| since(token) >= now - 3_600 && since(token) <= now)
            .count();

        assert_eq!(hour as usize, started);
        assert!(hour > 0 && hour < all);
    }

    #[test]
    fn refresh_moves_the_snapshot_time_to_now() {
        record_promo_filtering_refresh();
        let age = Utc::now() - get_promo_filtering_snapshot().updated_at;
        assert!(age.num_seconds() <= 1);
    }
}
