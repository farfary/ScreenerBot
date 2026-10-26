// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the snapshot build: the stamp, the rejection start, the pass time, the tokens held back only by a missing RugCheck report (never queued for one while blacklisted by token, pool or account), and that the snapshot carries exactly the rejection state the token store holds after the build.

use std::collections::HashSet;
use std::sync::Arc;

use rusqlite::{params, Connection};

use super::engine::compute_snapshot;
use super::sources::FilterRejectionReason;
use super::types::FilteringSnapshot;
use crate::chains::ChainId;
use crate::config::schemas::Config;
use crate::config::FilteringConfig;
use crate::tokens::TokenDatabase;

const CHAIN: ChainId = ChainId::Solana;
const NEW_TOKEN: &str = "NewToken1111111111111111111111111111111111";
const OLD_TOKEN: &str = "OldToken1111111111111111111111111111111111";
const TEN_DAYS: i64 = 10 * 24 * 3600;

/// A fresh token store installed as the chain's database, with the global state a build
/// reads (config, chain runtime, data directory) pointing at throwaway values.
fn setup() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", directory.path());
    let mut config = Config::default();
    config.events.enabled = false;
    let _ = crate::config::utils::CONFIG.get_or_init(|| std::sync::RwLock::new(config));
    crate::chains::install_enabled_runtimes();

    let db = Arc::new(
        TokenDatabase::new(&directory.path().join("tokens.db").to_string_lossy(), CHAIN)
            .expect("tokens store"),
    );
    crate::tokens::install_database(db);
    directory
}

/// A second connection to the installed store, for seeding and reading rows.
fn raw(directory: &tempfile::TempDir) -> Connection {
    Connection::open(directory.path().join("tokens.db")).expect("open tokens store")
}

/// A token with a priced, fresh DexScreener row and a RugCheck report, discovered
/// `age_seconds` ago.
fn seed_token(directory: &tempfile::TempDir, mint: &str, age_seconds: i64) {
    seed_listing(directory, mint, age_seconds);
    let now = chrono::Utc::now().timestamp();
    raw(directory)
        .execute(
            "INSERT INTO security_rugcheck (chain_id, mint, score, security_data_last_fetched_at, security_data_first_fetched_at)
             VALUES ('solana', ?1, 1, ?2, ?2)",
            params![mint, now],
        )
        .expect("insert report row");
}

/// A token with a priced, fresh DexScreener row and no RugCheck report yet.
fn seed_listing(directory: &tempfile::TempDir, mint: &str, age_seconds: i64) {
    let now = chrono::Utc::now().timestamp();
    let conn = raw(directory);
    conn.execute(
        "INSERT INTO tokens (chain_id, mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
         VALUES ('solana', ?1, 'TKN', 'Token', 6, ?2, ?3, ?3)",
        params![mint, now - age_seconds, now],
    )
    .expect("insert token");
    conn.execute(
        "INSERT INTO market_dexscreener (chain_id, mint, price_usd, market_data_last_fetched_at, market_data_first_fetched_at)
         VALUES ('solana', ?1, 1.0, ?2, ?2)",
        params![mint, now],
    )
    .expect("insert market row");
    conn.execute(
        "INSERT INTO update_tracking (chain_id, mint) VALUES ('solana', ?1)",
        params![mint],
    )
    .expect("insert tracking row");
}

/// Every source off, so a token passes unless the case switches one rule on.
fn open_config() -> FilteringConfig {
    let mut config = FilteringConfig::default();
    config.cooldown_enabled = false;
    config.age_enabled = false;
    config.onchain.enabled = false;
    config.dexscreener.enabled = false;
    config.geckoterminal.enabled = false;
    config.rugcheck.enabled = false;
    config
}

/// Admits everything that carries a RugCheck report: the RugCheck stage is on, so a
/// token without one waits for it.
fn report_required_config() -> FilteringConfig {
    let mut config = open_config();
    config.rugcheck.enabled = true;
    config
}

/// Rejects every token discovered less than an hour ago.
fn too_new_config() -> FilteringConfig {
    let mut config = open_config();
    config.age_enabled = true;
    config.min_token_age_minutes = 60;
    config
}

/// Rejects every token without a logo.
fn missing_logo_config() -> FilteringConfig {
    let mut config = open_config();
    config.dexscreener.enabled = true;
    config.dexscreener.token_info_enabled = true;
    config.dexscreener.require_logo_url = true;
    config
}

/// `(reason, source, at, stamp)` of one stored tracking row.
type Row = (Option<String>, Option<String>, Option<i64>, Option<i64>);

fn stored(directory: &tempfile::TempDir, mint: &str) -> Row {
    raw(directory)
        .query_row(
            "SELECT last_rejection_reason, last_rejection_source, last_rejection_at, filter_evaluated_at
             FROM update_tracking WHERE chain_id = 'solana' AND mint = ?1",
            params![mint],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read tracking row")
}

async fn build(config: FilteringConfig, previous: Option<&FilteringSnapshot>) -> FilteringSnapshot {
    compute_snapshot(CHAIN, config, previous)
        .await
        .expect("snapshot")
}

#[tokio::test]
async fn since_is_kept_while_the_reason_is_unchanged_and_restarts_when_it_changes() {
    let directory = setup();
    seed_token(&directory, NEW_TOKEN, 0);

    let first = build(too_new_config(), None).await;
    let second = build(too_new_config(), Some(&first)).await;
    let third = build(missing_logo_config(), Some(&second)).await;

    let too_new = FilterRejectionReason::TokenTooNew.label();
    let no_logo = FilterRejectionReason::DexScreenerEmptyLogoUrl.label();
    assert_eq!(first.rejected_tokens[0].reason, too_new);
    assert_eq!(first.rejected_tokens[0].rejection_time, first.evaluated_at);
    assert_eq!(
        second.rejected_tokens[0].rejection_time, first.evaluated_at,
        "an unchanged reason keeps its start"
    );
    assert_eq!(third.rejected_tokens[0].reason, no_logo);
    assert_eq!(
        third.rejected_tokens[0].rejection_time, third.evaluated_at,
        "a new reason restarts it"
    );

    let (reason, _, at, stamp) = stored(&directory, NEW_TOKEN);
    assert_eq!(reason, Some(no_logo));
    assert_eq!(at, Some(third.evaluated_at));
    assert_eq!(stamp, Some(third.evaluated_at));
}

#[tokio::test]
async fn the_snapshot_carries_the_reason_the_database_holds() {
    let directory = setup();
    seed_token(&directory, NEW_TOKEN, 0);
    seed_token(&directory, OLD_TOKEN, TEN_DAYS);

    let first = build(too_new_config(), None).await;
    let second = build(too_new_config(), Some(&first)).await;

    for snapshot in [&first, &second] {
        let rejected: HashSet<&String> = snapshot.rejected_mints.iter().collect();
        assert_eq!(rejected.len(), 1);
        for (mint, entry) in &snapshot.tokens {
            let (reason, source, at, stamp) = stored(&directory, mint);
            let token = &entry.token;
            assert_eq!(token.last_rejection_reason, reason);
            assert_eq!(token.last_rejection_source, source);
            assert_eq!(token.last_rejection_at.map(|t| t.timestamp()), at);
            assert_eq!(rejected.contains(mint), reason.is_some());
            assert_eq!(stamp, Some(second.evaluated_at));
        }
    }
    assert_eq!(second.filtered_mints, vec![OLD_TOKEN.to_owned()]);
    assert_eq!(second.rejected_mints, vec![NEW_TOKEN.to_owned()]);
}

#[tokio::test]
async fn a_passed_token_keeps_its_first_pass_time_while_it_keeps_passing() {
    let directory = setup();
    seed_token(&directory, OLD_TOKEN, TEN_DAYS);

    let first = build(open_config(), None).await;
    let second = build(open_config(), Some(&first)).await;

    assert_eq!(first.passed_tokens[0].passed_time, first.evaluated_at);
    assert_eq!(second.passed_tokens[0].passed_time, first.evaluated_at);
    assert_eq!(second.tokens[OLD_TOKEN].passed_at, Some(first.evaluated_at));
    assert_eq!(second.tokens[OLD_TOKEN].token.last_rejection_reason, None);
}

#[tokio::test]
async fn an_empty_corpus_is_a_complete_build() {
    let directory = setup();
    seed_token(&directory, NEW_TOKEN, 0);

    let first = build(too_new_config(), None).await;
    assert_eq!(first.rejected_mints, vec![NEW_TOKEN.to_owned()]);
    raw(&directory)
        .execute("DELETE FROM market_dexscreener", [])
        .expect("drop the market rows");

    let second = build(too_new_config(), Some(&first)).await;

    assert!(second.tokens.is_empty());
    assert!(second.evaluated_at > first.evaluated_at);
    assert_eq!(
        stored(&directory, NEW_TOKEN).0,
        None,
        "the stale rejection is cleared"
    );
}

#[tokio::test]
async fn stamps_strictly_increase() {
    let directory = setup();
    seed_token(&directory, NEW_TOKEN, 0);

    let first = build(too_new_config(), None).await;
    let second = build(too_new_config(), Some(&first)).await;
    let third = build(too_new_config(), Some(&second)).await;
    assert!(first.evaluated_at < second.evaluated_at);
    assert!(second.evaluated_at < third.evaluated_at);

    let mut ahead = build(too_new_config(), Some(&third)).await;
    ahead.evaluated_at += 1_000;
    let after = build(too_new_config(), Some(&ahead)).await;
    assert_eq!(after.evaluated_at, ahead.evaluated_at + 1);
}

#[tokio::test]
async fn a_token_without_a_report_awaits_it_and_is_neither_passed_nor_rejected() {
    let directory = setup();
    seed_listing(&directory, NEW_TOKEN, TEN_DAYS);
    seed_listing(&directory, OLD_TOKEN, 2 * TEN_DAYS);

    let snapshot = build(report_required_config(), None).await;

    assert!(snapshot.filtered_mints.is_empty());
    assert!(snapshot.passed_tokens.is_empty());
    assert!(snapshot.rejected_mints.is_empty());
    assert!(snapshot.rejected_tokens.is_empty());
    assert_eq!(
        snapshot.awaiting_rugcheck_report,
        vec![NEW_TOKEN.to_owned(), OLD_TOKEN.to_owned()],
        "awaiting tokens are listed newest first"
    );
    assert_eq!(stored(&directory, NEW_TOKEN).0, None, "no rejection row");
}

/// Where a token's blacklisting comes from: its own entry, a pool of it that keeps
/// failing, or an account of it that failed.
#[derive(Debug, Clone, Copy)]
enum BlacklistSource {
    Token,
    Pool,
    Account,
}

/// Blacklist `mint` through `source`.
async fn blacklist(directory: &tempfile::TempDir, mint: &str, source: BlacklistSource) {
    match source {
        BlacklistSource::Token => {
            raw(directory)
                .execute(
                    "INSERT INTO blacklist (chain_id, mint, reason, source, added_at)
                     VALUES ('solana', ?1, 'pool', 'test', ?2)",
                    params![mint, chrono::Utc::now().timestamp()],
                )
                .expect("insert blacklist row");
        }
        BlacklistSource::Pool => {
            crate::pools::db::initialize_database(CHAIN)
                .await
                .expect("open the pools store");
            let threshold = crate::pools::types::PoolBlacklistPolicy::from_config().threshold;
            crate::pools::db::add_pool_to_blacklist(
                CHAIN,
                "FailingPool11111111111111111111111111111111",
                "decode failure",
                Some(mint),
                None,
                threshold,
            )
            .await
            .expect("blacklist the pool");
        }
        BlacklistSource::Account => {
            crate::pools::db::initialize_database(CHAIN)
                .await
                .expect("open the pools store");
            crate::pools::db::add_account_to_blacklist(
                CHAIN,
                "FailingAccount111111111111111111111111111111",
                "decode failure",
                Some("test"),
                None,
                Some(mint),
            )
            .await
            .expect("blacklist the account");
        }
    }
}

/// A token blacklisted through `source` and waiting for its report is counted but never
/// takes one of the direct report fetches.
async fn assert_a_blacklisted_token_is_not_queued(source: BlacklistSource) {
    let directory = setup();
    seed_listing(&directory, NEW_TOKEN, TEN_DAYS);
    seed_listing(&directory, OLD_TOKEN, 2 * TEN_DAYS);
    blacklist(&directory, OLD_TOKEN, source).await;

    let snapshot = build(report_required_config(), None).await;

    assert_eq!(
        snapshot.awaiting_rugcheck_report,
        vec![NEW_TOKEN.to_owned()],
        "a token blacklisted by {source:?} never takes one of the direct report fetches"
    );
    assert!(snapshot.filtered_mints.is_empty());
}

#[tokio::test]
async fn a_blacklisted_token_without_a_report_is_not_queued_for_one() {
    assert_a_blacklisted_token_is_not_queued(BlacklistSource::Token).await;
}

#[tokio::test]
async fn a_token_of_a_blacklisted_pool_is_not_queued_for_a_report() {
    assert_a_blacklisted_token_is_not_queued(BlacklistSource::Pool).await;
}

#[tokio::test]
async fn a_token_of_a_blacklisted_account_is_not_queued_for_a_report() {
    assert_a_blacklisted_token_is_not_queued(BlacklistSource::Account).await;
}

#[tokio::test]
async fn an_earlier_stage_rejection_shows_its_own_reason_before_the_missing_report() {
    let directory = setup();
    seed_listing(&directory, NEW_TOKEN, 0);
    let mut config = too_new_config();
    config.rugcheck.enabled = true;

    let snapshot = build(config, None).await;

    assert!(snapshot.awaiting_rugcheck_report.is_empty());
    assert_eq!(
        snapshot.rejected_tokens[0].reason,
        FilterRejectionReason::TokenTooNew.label()
    );
}

#[tokio::test]
async fn with_the_rugcheck_stage_off_a_token_without_a_report_passes() {
    let directory = setup();
    seed_listing(&directory, NEW_TOKEN, TEN_DAYS);

    let snapshot = build(open_config(), None).await;

    assert!(
        snapshot.awaiting_rugcheck_report.is_empty(),
        "nothing awaits a report"
    );
    assert!(snapshot.rejected_tokens.is_empty());
    assert_eq!(snapshot.filtered_mints, vec![NEW_TOKEN.to_owned()]);
}
