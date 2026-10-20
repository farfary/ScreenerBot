// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the authority reputation table: which rows startup cleanup removes, how the data
//! service's creator list replaces and expires its own rows, and which rows both leave alone.

use rusqlite::params;

use super::authority::{CreatorReputationRow, CREATOR_HISTORY_SOURCE};
use super::TokenDatabase;
use crate::chains::ChainId;

fn insert_row(db: &TokenDatabase, chain: &str, address: &str, source: &str) {
    db.conn()
        .expect("connection")
        .execute(
            "INSERT INTO authority_reputation (chain_id, address, is_blocked, source, first_seen_at, last_updated_at)
             VALUES (?1, ?2, 1, ?3, 1, 1)",
            params![chain, address, source],
        )
        .expect("insert reputation row");
}

fn addresses(db: &TokenDatabase, chain: &str) -> Vec<String> {
    let conn = db.conn().expect("connection");
    let mut stmt = conn
        .prepare("SELECT address FROM authority_reputation WHERE chain_id = ?1 ORDER BY address")
        .expect("prepare");
    let rows = stmt
        .query_map(params![chain], |row| row.get::<_, String>(0))
        .expect("query");
    rows.map(|row| row.expect("row")).collect()
}

#[test]
fn startup_removes_automatic_reputation_and_keeps_manual_rows() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db =
        TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("open tokens store");

    insert_row(&db, "solana", "AutoAuthority", "auto");
    insert_row(&db, "solana", "ManualAuthority", "manual");
    insert_row(&db, "other", "OtherChainAuto", "auto");

    assert_eq!(db.remove_auto_reputation().expect("remove"), 1);

    assert_eq!(addresses(&db, "solana"), vec!["ManualAuthority".to_owned()]);
    assert_eq!(addresses(&db, "other"), vec!["OtherChainAuto".to_owned()]);
    assert_eq!(
        db.load_blocked_authorities().expect("load"),
        vec!["ManualAuthority".to_owned()]
    );
}

fn open_store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db =
        TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("open tokens store");
    (directory, db)
}

fn creator(address: &str, tokens: u32, rugged: u32) -> CreatorReputationRow {
    CreatorReputationRow {
        address: address.to_owned(),
        tokens,
        rugged,
    }
}

/// `(source, first_seen_at, last_updated_at, total_token_count, flagged_token_count, confidence)`.
type StoredRow = (String, i64, i64, i64, i64, f64);

fn stored(db: &TokenDatabase, chain: &str, address: &str) -> Option<StoredRow> {
    db.conn()
        .expect("connection")
        .query_row(
            "SELECT source, first_seen_at, last_updated_at, total_token_count,
                    flagged_token_count, confidence
             FROM authority_reputation WHERE chain_id = ?1 AND address = ?2",
            params![chain, address],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .ok()
}

#[test]
fn a_creator_list_blocks_its_wallets_with_the_share_of_removed_liquidity() {
    let (_directory, db) = open_store();

    let written = db
        .replace_creator_reputation(&[creator("CreatorA", 5, 4), creator("CreatorB", 8, 8)], 100)
        .expect("replace");

    assert_eq!(written, 2);
    assert_eq!(
        db.load_blocked_authorities().expect("load").len(),
        2,
        "every listed wallet is blocked"
    );
    let (source, first_seen, updated, total, flagged, confidence) =
        stored(&db, "solana", "CreatorA").expect("row");
    assert_eq!(source, CREATOR_HISTORY_SOURCE);
    assert_eq!((first_seen, updated, total, flagged), (100, 100, 5, 4));
    assert!((confidence - 0.8).abs() < f64::EPSILON);
}

#[test]
fn a_row_missing_from_a_new_answer_is_deleted() {
    let (_directory, db) = open_store();
    db.replace_creator_reputation(&[creator("Kept", 5, 5), creator("Lifted", 6, 5)], 100)
        .expect("first sync");

    db.replace_creator_reputation(&[creator("Kept", 5, 5)], 200)
        .expect("second sync");
    assert_eq!(addresses(&db, "solana"), vec!["Kept".to_owned()]);

    db.replace_creator_reputation(&[], 300)
        .expect("empty answer");
    assert!(addresses(&db, "solana").is_empty());
    assert!(db.load_blocked_authorities().expect("load").is_empty());
}

#[test]
fn a_row_of_another_source_is_neither_overwritten_nor_deleted() {
    let (_directory, db) = open_store();
    insert_row(&db, "solana", "ManualWallet", "manual");
    insert_row(&db, "solana", "ManualUnlisted", "manual");

    let written = db
        .replace_creator_reputation(
            &[creator("ManualWallet", 9, 9), creator("Listed", 5, 5)],
            500,
        )
        .expect("replace");

    assert_eq!(written, 1, "the manual row is not written");
    assert_eq!(
        stored(&db, "solana", "ManualWallet"),
        Some(("manual".to_owned(), 1, 1, 0, 0, 0.0))
    );
    assert!(stored(&db, "solana", "ManualUnlisted").is_some());

    db.replace_creator_reputation(&[], 600)
        .expect("empty answer");
    assert_eq!(
        addresses(&db, "solana"),
        vec!["ManualUnlisted".to_owned(), "ManualWallet".to_owned()]
    );
}

#[test]
fn a_second_sync_keeps_first_seen_and_refreshes_the_counts() {
    let (_directory, db) = open_store();
    db.replace_creator_reputation(&[creator("Creator", 5, 4)], 100)
        .expect("first sync");

    db.replace_creator_reputation(&[creator("Creator", 10, 9)], 1_000)
        .expect("second sync");

    let (_, first_seen, updated, total, flagged, confidence) =
        stored(&db, "solana", "Creator").expect("row");
    assert_eq!((first_seen, updated, total, flagged), (100, 1_000, 10, 9));
    assert!((confidence - 0.9).abs() < f64::EPSILON);
}

#[test]
fn replacing_one_chain_leaves_another_chains_rows_untouched() {
    let (_directory, db) = open_store();
    insert_row(&db, "other", "OtherCreator", CREATOR_HISTORY_SOURCE);

    db.replace_creator_reputation(&[creator("Creator", 5, 5)], 100)
        .expect("replace");
    db.replace_creator_reputation(&[], 200)
        .expect("empty answer");
    db.expire_creator_reputation(i64::MAX).expect("expire");

    assert_eq!(addresses(&db, "other"), vec!["OtherCreator".to_owned()]);
}

#[test]
fn expiry_removes_only_stale_rows_of_the_creator_source() {
    let (_directory, db) = open_store();
    db.replace_creator_reputation(&[creator("FreshCreator", 5, 5)], 1_000)
        .expect("replace");
    insert_row(&db, "solana", "StaleCreator", CREATOR_HISTORY_SOURCE);
    insert_row(&db, "solana", "StaleManual", "manual");

    let removed = db.expire_creator_reputation(500).expect("expire");

    assert_eq!(removed, 1);
    assert_eq!(
        addresses(&db, "solana"),
        vec!["FreshCreator".to_owned(), "StaleManual".to_owned()]
    );
}

#[test]
fn startup_cleanup_keeps_creator_rows() {
    let (_directory, db) = open_store();
    insert_row(&db, "solana", "AutoAuthority", "auto");
    db.replace_creator_reputation(&[creator("Creator", 5, 5)], 100)
        .expect("replace");

    assert_eq!(db.remove_auto_reputation().expect("remove"), 1);

    assert_eq!(addresses(&db, "solana"), vec!["Creator".to_owned()]);
}
