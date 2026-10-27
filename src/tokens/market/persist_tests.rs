// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for storing a market answer: a source that answers for a token no other path stored yet still lands its market row, and a failed write is retried once and then returned.

use std::cell::Cell;

use chrono::Utc;

use super::persist::{store_market_answer, AnsweredMetadata};
use crate::chains::ChainId;
use crate::errors::DatabaseError;
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::DexScreenerData;
use crate::tokens::Error;

const MINT: &str = "So11111111111111111111111111111111111111112";

fn store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("tokens store");
    (directory, db)
}

fn answer() -> DexScreenerData {
    let now = Utc::now();
    DexScreenerData {
        price_usd: 180.0,
        price_sol: 1.0,
        price_native: "1".to_owned(),
        price_change_5m: None,
        price_change_1h: None,
        price_change_6h: None,
        price_change_24h: None,
        market_cap: None,
        fdv: None,
        liquidity_usd: Some(1_000_000.0),
        volume_5m: None,
        volume_1h: None,
        volume_6h: None,
        volume_24h: None,
        txns_5m: None,
        txns_1h: None,
        txns_6h: None,
        txns_24h: None,
        pair_address: None,
        chain_id: None,
        dex_id: None,
        url: None,
        image_url: None,
        header_image_url: None,
        pair_blockchain_created_at: None,
        market_data_last_fetched_at: now,
        market_data_first_fetched_at: now,
    }
}

fn metadata() -> AnsweredMetadata<'static> {
    AnsweredMetadata {
        symbol: Some("SOL"),
        name: Some("Wrapped SOL"),
        decimals: None,
    }
}

#[test]
fn a_source_answering_before_the_token_is_stored_still_lands_its_market_row() {
    let (_directory, db) = store();
    let data = answer();

    // The market table references the token row: written alone, the answer is refused.
    assert!(db.upsert_dexscreener_data(MINT, &data).is_err());
    assert!(db.get_dexscreener_data(MINT).expect("read").is_none());

    store_market_answer(&db, "DEXSCREENER", MINT, metadata(), || {
        db.upsert_dexscreener_data(MINT, &data)
    })
    .expect("answer stored");

    assert!(db.get_dexscreener_data(MINT).expect("read").is_some());
    let token = db.get_token(MINT).expect("read").expect("token row");
    assert_eq!(token.symbol.as_deref(), Some("SOL"));
}

#[test]
fn a_write_that_fails_once_is_stored_on_the_retry() {
    let (_directory, db) = store();
    let attempts = Cell::new(0);

    store_market_answer(&db, "DEXSCREENER", MINT, metadata(), || {
        attempts.set(attempts.get() + 1);
        if attempts.get() == 1 {
            Err(Error::Database(DatabaseError::Query {
                operation: "upsert market row".to_owned(),
                message: "FOREIGN KEY constraint failed".to_owned(),
            }))
        } else {
            Ok(())
        }
    })
    .expect("stored on the retry");

    assert_eq!(attempts.get(), 2);
}

#[test]
fn a_write_that_keeps_failing_is_retried_once_and_returned() {
    let (_directory, db) = store();
    let attempts = Cell::new(0);

    let result = store_market_answer(&db, "DEXSCREENER", MINT, metadata(), || {
        attempts.set(attempts.get() + 1);
        Err(Error::Database(DatabaseError::Query {
            operation: "upsert market row".to_owned(),
            message: "FOREIGN KEY constraint failed".to_owned(),
        }))
    });

    assert_eq!(attempts.get(), 2);
    let error = result.expect_err("the failure is returned");
    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
}
