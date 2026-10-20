// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the authority reputation table: which rows startup cleanup removes and which it keeps.

use rusqlite::params;

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
