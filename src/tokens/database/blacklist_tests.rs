// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the token blacklist write rule: what a user's removal blocks, what a manual
//! blacklist clears, which entries an automatic write may replace, and the compare-and-lift.

use super::{BlacklistSource, BlacklistWrite, TokenDatabase};
use crate::chains::ChainId;
use crate::tokens::database::is_automatic_source;

const MINT: &str = "BlacklistTestMint1111111111111111111111111";

fn open_store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db =
        TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("open tokens store");
    (directory, db)
}

fn automatic_sources() -> impl Iterator<Item = BlacklistSource> {
    BlacklistSource::ALL
        .into_iter()
        .filter(|source| source.code() != BlacklistSource::Manual.code())
}

#[test]
fn source_codes_split_into_manual_and_automatic() {
    assert!(!is_automatic_source(BlacklistSource::Manual.code()));
    assert!(!is_automatic_source("filtering"));
    for source in automatic_sources() {
        assert!(is_automatic_source(source.code()), "{}", source.code());
    }
}

/// The dashboard names who filed an entry from the stored source code through a label map
/// keyed by these codes (`BLACKLIST_SOURCE_LABELS`, ui/token_blacklist.js); the match is
/// exhaustive, so a new source fails to compile until it is mapped here.
fn label_key(source: BlacklistSource) -> &'static str {
    match source {
        BlacklistSource::Manual => "tokens-blacklist-source-manual",
        BlacklistSource::AutoCleanup => "tokens-blacklist-source-auto-cleanup",
        BlacklistSource::AutoLoss => "tokens-blacklist-source-auto-loss",
        BlacklistSource::AutoLlmAnalysis => "tokens-blacklist-source-auto-llm-analysis",
        BlacklistSource::AutoSwapRoute => "tokens-blacklist-source-auto-swap-route",
    }
}

#[test]
fn every_source_has_a_label_in_the_catalog() {
    for source in BlacklistSource::ALL {
        let key = label_key(source);
        assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
    }
}

#[test]
fn a_removal_blocks_every_automatic_source_until_a_manual_blacklist() {
    for source in automatic_sources() {
        let (_directory, db) = open_store();
        assert_eq!(
            db.add_to_blacklist(MINT, "first", source.code()).unwrap(),
            BlacklistWrite::Written
        );

        let removed = db.clear_blacklist_entry(MINT).unwrap().expect("an entry");
        assert_eq!(removed.source, source.code());
        assert_eq!(removed.reason, "first");
        assert!(!db.is_blacklisted(MINT).unwrap());
        assert!(db.is_blacklist_cleared(MINT).unwrap());

        for blocked in automatic_sources() {
            assert_eq!(
                db.add_to_blacklist(MINT, "again", blocked.code()).unwrap(),
                BlacklistWrite::RefusedUserRemoved,
                "{} after the removal of a {} entry",
                blocked.code(),
                source.code()
            );
            assert!(!db.is_blacklisted(MINT).unwrap());
        }
    }
}

#[test]
fn a_manual_blacklist_clears_the_removal_and_writes() {
    let (_directory, db) = open_store();
    db.add_to_blacklist(MINT, "authority", BlacklistSource::AutoCleanup.code())
        .unwrap();
    db.clear_blacklist_entry(MINT).unwrap();

    assert_eq!(
        db.add_to_blacklist(MINT, "by hand", BlacklistSource::Manual.code())
            .unwrap(),
        BlacklistWrite::Written
    );
    assert!(!db.is_blacklist_cleared(MINT).unwrap());
    let entry = db.get_blacklist_entry(MINT).unwrap().expect("an entry");
    assert_eq!(
        (entry.reason.as_str(), entry.source.as_str()),
        ("by hand", "manual")
    );
}

#[test]
fn an_automatic_write_never_replaces_a_manual_entry() {
    let (_directory, db) = open_store();
    db.add_to_blacklist(MINT, "by hand", BlacklistSource::Manual.code())
        .unwrap();

    for source in automatic_sources() {
        assert_eq!(
            db.add_to_blacklist(MINT, "rule", source.code()).unwrap(),
            BlacklistWrite::RefusedFirmerEntry,
            "{}",
            source.code()
        );
    }
    let entry = db.get_blacklist_entry(MINT).unwrap().expect("an entry");
    assert_eq!(
        (entry.reason.as_str(), entry.source.as_str()),
        ("by hand", "manual")
    );
}

#[test]
fn a_liftable_source_never_replaces_an_entry_that_stays() {
    for lasting in [BlacklistSource::AutoLoss, BlacklistSource::AutoLlmAnalysis] {
        let (_directory, db) = open_store();
        assert_eq!(
            db.add_to_blacklist(MINT, "stays", lasting.code()).unwrap(),
            BlacklistWrite::Written
        );
        for liftable in [BlacklistSource::AutoSwapRoute, BlacklistSource::AutoCleanup] {
            assert_eq!(
                db.add_to_blacklist(MINT, "lifts", liftable.code()).unwrap(),
                BlacklistWrite::RefusedFirmerEntry,
                "{} over {}",
                liftable.code(),
                lasting.code()
            );
        }
        assert_eq!(
            db.get_blacklist_entry(MINT)
                .unwrap()
                .expect("an entry")
                .source,
            lasting.code()
        );
    }
}

#[test]
fn an_entry_that_lifts_is_replaced_by_one_that_stays() {
    let (_directory, db) = open_store();
    assert_eq!(
        db.add_to_blacklist(MINT, "route", BlacklistSource::AutoSwapRoute.code())
            .unwrap(),
        BlacklistWrite::Written
    );
    assert_eq!(
        db.add_to_blacklist(MINT, "loss", BlacklistSource::AutoLoss.code())
            .unwrap(),
        BlacklistWrite::Written
    );
    assert_eq!(
        db.get_blacklist_entry(MINT)
            .unwrap()
            .expect("an entry")
            .source,
        BlacklistSource::AutoLoss.code()
    );
}

#[test]
fn removing_a_token_without_an_entry_remembers_nothing() {
    let (_directory, db) = open_store();
    assert!(db.clear_blacklist_entry(MINT).unwrap().is_none());
    assert!(!db.is_blacklist_cleared(MINT).unwrap());
    assert_eq!(
        db.add_to_blacklist(MINT, "rule", BlacklistSource::AutoLoss.code())
            .unwrap(),
        BlacklistWrite::Written
    );
}

#[test]
fn a_removal_applies_to_its_own_token_only() {
    let (_directory, db) = open_store();
    let other = "OtherBlacklistTestMint11111111111111111111";
    db.add_to_blacklist(MINT, "rule", BlacklistSource::AutoLoss.code())
        .unwrap();
    db.clear_blacklist_entry(MINT).unwrap();

    assert_eq!(
        db.add_to_blacklist(other, "rule", BlacklistSource::AutoLoss.code())
            .unwrap(),
        BlacklistWrite::Written
    );
}

#[test]
fn a_lift_removes_only_the_entry_it_read() {
    let (_directory, db) = open_store();
    db.add_to_blacklist(MINT, "route", BlacklistSource::AutoSwapRoute.code())
        .unwrap();
    let read = db.get_blacklist_entry(MINT).unwrap().expect("an entry");

    // The user blacklists the token between the read and the lift.
    db.add_to_blacklist(MINT, "by hand", BlacklistSource::Manual.code())
        .unwrap();
    assert!(!db.lift_blacklist_entry(&read).unwrap());
    assert!(db.is_blacklisted(MINT).unwrap());

    let current = db.get_blacklist_entry(MINT).unwrap().expect("an entry");
    assert!(db.lift_blacklist_entry(&current).unwrap());
    assert!(!db.is_blacklisted(MINT).unwrap());
    assert!(
        !db.is_blacklist_cleared(MINT).unwrap(),
        "a lift is not a removal by the user"
    );
}
