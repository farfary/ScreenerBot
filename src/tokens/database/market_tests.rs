// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the No Market Data listing: each token comes back with the outcome of its latest failed market fetch, and with nothing for a token that was never checked.

use rusqlite::params;

use super::TokenDatabase;
use crate::chains::ChainId;
use crate::tokens::types::MarketCheck;

fn store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("tokens store");
    (directory, db)
}

fn insert_token(db: &TokenDatabase, mint: &str) {
    let conn = db.conn().expect("connection");
    conn.execute(
        "INSERT INTO tokens (chain_id, mint, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
         VALUES ('solana', ?1, 1, 1, 1)",
        params![mint],
    )
    .expect("insert token");
    conn.execute(
        "INSERT INTO update_tracking (chain_id, mint) VALUES ('solana', ?1)",
        params![mint],
    )
    .expect("insert tracking row");
}

fn check_of(db: &TokenDatabase, mint: &str) -> MarketCheck {
    db.get_tokens_no_market(0, 0, None, None)
        .expect("no-market listing")
        .into_iter()
        .find(|(token, _)| token.mint == mint)
        .map(|(_, check)| check)
        .expect("token is listed")
}

#[test]
fn a_token_never_checked_has_no_market_check() {
    let (_directory, db) = store();
    insert_token(&db, "Unchecked");

    assert_eq!(
        check_of(&db, "Unchecked"),
        MarketCheck {
            state: None,
            checked_at: None
        }
    );
}

#[test]
fn a_recorded_not_listed_error_is_returned_with_its_time() {
    let (_directory, db) = store();
    insert_token(&db, "NotListed");
    db.record_market_error("NotListed", "not listed on the source", "not_listed")
        .expect("record error");

    let check = check_of(&db, "NotListed");
    assert_eq!(check.state.as_deref(), Some("not_listed"));
    assert!(check.checked_at.is_some());
}

#[test]
fn a_permanent_token_reads_permanent() {
    let (_directory, db) = store();
    insert_token(&db, "Permanent");
    db.record_market_error("Permanent", "not listed on the source", "not_listed")
        .expect("record error");
    db.mark_market_permanent("Permanent")
        .expect("mark permanent");

    let check = check_of(&db, "Permanent");
    assert_eq!(check.state.as_deref(), Some("permanent"));
    assert!(check.checked_at.is_some());
}
