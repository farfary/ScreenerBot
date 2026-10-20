// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Tests for the creator list sync: which entries are refused, and that an absent or not-ready
//! answer only expires rows instead of replacing them.

use super::creator_reputation::{accepted_rows, apply_answer, CreatorsResponse, ServiceCreator};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::solana_sdk::signature::Keypair;
use crate::chains::solana::solana_sdk::signer::Signer;
use crate::chains::ChainId;
use crate::tokens::database::TokenDatabase;

const HOUR: i64 = 3_600;

fn open_store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db =
        TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("open tokens store");
    (directory, db)
}

fn wallet() -> String {
    Keypair::new().pubkey().to_string()
}

fn entry(address: &str, tokens: u32, rugged: u32) -> ServiceCreator {
    ServiceCreator {
        address: address.to_owned(),
        tokens,
        rugged,
    }
}

fn answer(ready: bool, creators: Vec<ServiceCreator>) -> CreatorsResponse {
    CreatorsResponse { ready, creators }
}

#[test]
fn entries_that_cannot_be_a_serial_remover_are_refused() {
    let valid = wallet();
    let (derived, _) = Pubkey::find_program_address(&[b"creator"], &Keypair::new().pubkey());

    let (rows, refused) = accepted_rows(
        ChainId::Solana,
        &[
            entry(&valid, 5, 4),
            entry(&derived.to_string(), 9, 9),
            entry("not-an-address", 9, 9),
            entry(&wallet(), 0, 0),
            entry(&wallet(), 5, 6),
        ],
    );

    assert_eq!(refused, 4);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].address, valid);
    assert_eq!((rows[0].tokens, rows[0].rugged), (5, 4));
}

#[test]
fn a_ready_list_replaces_the_stored_creators() {
    let (_directory, db) = open_store();
    let (kept, lifted) = (wallet(), wallet());
    apply_answer(
        &db,
        Some(&answer(
            true,
            vec![entry(&kept, 5, 5), entry(&lifted, 6, 5)],
        )),
        1_000,
    )
    .expect("first list");

    let blocked = apply_answer(&db, Some(&answer(true, vec![entry(&kept, 5, 5)])), 2_000)
        .expect("second list");

    assert_eq!(blocked, 1);
    assert_eq!(db.load_blocked_authorities().expect("load"), vec![kept]);
}

#[test]
fn no_answer_only_expires_rows_older_than_three_days() {
    let (_directory, db) = open_store();
    let (old, recent) = (wallet(), wallet());
    apply_answer(&db, Some(&answer(true, vec![entry(&old, 5, 5)])), 0).expect("old list");
    apply_answer(
        &db,
        Some(&answer(true, vec![entry(&old, 5, 5), entry(&recent, 5, 5)])),
        10 * HOUR,
    )
    .expect("list with a refreshed old row");

    // The old row was refreshed at 10 h; at 80 h it is 70 h old and survives.
    assert_eq!(apply_answer(&db, None, 80 * HOUR).expect("no answer"), 2);
    // At 83 h both rows are older than 72 h.
    assert_eq!(apply_answer(&db, None, 83 * HOUR).expect("no answer"), 0);
}

#[test]
fn a_list_that_is_not_ready_never_replaces_the_stored_creators() {
    let (_directory, db) = open_store();
    let stored = wallet();
    apply_answer(&db, Some(&answer(true, vec![entry(&stored, 5, 5)])), 1_000).expect("list");

    let blocked = apply_answer(
        &db,
        Some(&answer(false, vec![entry(&wallet(), 5, 5)])),
        2_000,
    )
    .expect("not ready");

    assert_eq!(blocked, 1);
    assert_eq!(db.load_blocked_authorities().expect("load"), vec![stored]);
}
