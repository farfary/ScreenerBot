// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the Rugcheck conversion and storage: the creator the report names reaches the
//! stored security data and comes back from the store.

use super::convert_rugcheck_to_data;
use crate::apis::rugcheck::RugcheckInfo;
use crate::chains::ChainId;
use crate::tokens::database::TokenDatabase;

const MINT: &str = "CreatorMint1111111111111111111111111111111";
const CREATOR: &str = "CreatorWallet11111111111111111111111111111";

fn report(creator: Option<&str>) -> RugcheckInfo {
    RugcheckInfo {
        mint: MINT.to_owned(),
        creator: creator.map(str::to_owned),
        ..Default::default()
    }
}

fn open_store_with_token() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db =
        TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("open tokens store");
    db.conn()
        .expect("connection")
        .execute(
            "INSERT INTO tokens (chain_id, mint, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES ('solana', ?1, 1, 1, 1)",
            [MINT],
        )
        .expect("insert token");
    (directory, db)
}

#[test]
fn the_reported_creator_reaches_the_security_data() {
    assert_eq!(
        convert_rugcheck_to_data(&report(Some(CREATOR)))
            .creator
            .as_deref(),
        Some(CREATOR)
    );
    assert_eq!(convert_rugcheck_to_data(&report(None)).creator, None);
}

#[test]
fn the_creator_survives_a_store_round_trip() {
    let (_directory, db) = open_store_with_token();

    db.upsert_rugcheck_data(MINT, &convert_rugcheck_to_data(&report(Some(CREATOR))))
        .expect("store a report with a creator");
    let stored = db.get_rugcheck_data(MINT).expect("read").expect("row");
    assert_eq!(stored.creator.as_deref(), Some(CREATOR));

    db.upsert_rugcheck_data(MINT, &convert_rugcheck_to_data(&report(None)))
        .expect("store a report without a creator");
    let stored = db.get_rugcheck_data(MINT).expect("read").expect("row");
    assert_eq!(stored.creator, None);
}
