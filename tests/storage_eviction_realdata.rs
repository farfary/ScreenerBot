// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Eviction against a clone of the real token and chart stores: every transaction of a sweep stays inside the time budget a running app can afford to wait on its store.
//!
//! # Safety
//!
//! Eviction deletes rows, so these tests only ever run on a temp clone made by
//! [`common::clone_real_stores`]; the source directory is opened read-only for the copy.
//!
//! # Tier
//!
//! `#[ignore]`, run by `./test.sh live`. They self-skip when no real data directory
//! exists. Each sweep stops after [`DEFAULT_TRANSACTIONS`] transactions unless
//! `SB_TEST_REALDB_FULL=1` asks for the whole store.

use crate::common;

use std::cell::Cell;
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::tokens::TokenDatabase;

const CHAIN: ChainId = ChainId::Solana;

/// The longest one transaction, with the pause after it, may hold a store's writer.
const TRANSACTION_BUDGET: Duration = Duration::from_millis(250);

/// Transactions sampled per sweep unless the whole store is requested.
const DEFAULT_TRANSACTIONS: u32 = 40;

/// Tokens and charts unused for longer than this are evicted on the clone.
const UNUSED_FOR: Duration = Duration::from_secs(24 * 60 * 60);

fn full_store_requested() -> bool {
    matches!(
        std::env::var("SB_TEST_REALDB_FULL").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// The `stop` callback a sweep checks between transactions. It records the longest
/// gap between two checks, which is one transaction and the pause after it, and
/// stops the sweep after the sample unless the whole store is requested.
struct Pacer {
    last: Cell<Option<Instant>>,
    longest: Cell<Duration>,
    transactions: Cell<u32>,
}

impl Pacer {
    fn new() -> Self {
        Self {
            last: Cell::new(None),
            longest: Cell::new(Duration::ZERO),
            transactions: Cell::new(0),
        }
    }

    fn stop(&self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last.replace(Some(now)) {
            self.longest.set(self.longest.get().max(now - last));
            self.transactions.set(self.transactions.get() + 1);
        }
        !full_store_requested() && self.transactions.get() >= DEFAULT_TRANSACTIONS
    }

    fn assert_within_budget(&self, store: &str) {
        eprintln!(
            "REALDATA {store}: {} transactions, longest {:?}",
            self.transactions.get(),
            self.longest.get()
        );
        assert!(
            self.longest.get() <= TRANSACTION_BUDGET,
            "{store}: a transaction held the writer for {:?}",
            self.longest.get()
        );
    }
}

fn cutoff_seconds() -> i64 {
    let cutoff = SystemTime::now() - UNUSED_FOR;
    i64::try_from(cutoff.duration_since(UNIX_EPOCH).expect("clock").as_secs()).expect("cutoff fits")
}

fn open(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("open cloned store");
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
        .expect("configure cloned store");
    conn
}

#[test]
#[ignore = "real data: clones the local token store"]
fn a_token_sweep_on_real_data_keeps_every_transaction_within_budget() {
    let Some(dir) = common::clone_real_stores(&["tokens.db"]) else {
        return;
    };
    let path = dir.path().join("data").join("tokens.db");
    if !path.is_file() {
        eprintln!("SKIP real-db: no tokens.db to clone");
        return;
    }
    // The production opener brings the clone to the current schema first.
    drop(TokenDatabase::new(&path.to_string_lossy(), CHAIN).expect("migrate cloned tokens.db"));

    let mut conn = open(&path);
    let pacer = Pacer::new();
    let eviction = screenerbot::tokens::database::retention::evict_tokens_unused_before(
        &mut conn,
        CHAIN,
        cutoff_seconds(),
        &HashSet::new(),
        &|| pacer.stop(),
    )
    .expect("evict tokens");
    eprintln!(
        "REALDATA tokens: examined {}, evicted {} tokens and {} rows",
        eviction.examined, eviction.tokens, eviction.rows
    );
    pacer.assert_within_budget("tokens.db");
}

#[test]
#[ignore = "real data: clones the local chart store"]
fn chart_evictions_on_real_data_keep_every_transaction_within_budget() {
    let Some(dir) = common::clone_real_stores(&["ohlcvs.db"]) else {
        return;
    };
    let path = dir.path().join("data").join("ohlcvs.db");
    if !path.is_file() {
        eprintln!("SKIP real-db: no ohlcvs.db to clone");
        return;
    }

    let mut conn = open(&path);
    let (_, charts) =
        screenerbot::ohlcvs::retention::inactive_charts(&conn, CHAIN).expect("list charts");
    let cutoff = cutoff_seconds();
    let pacer = Pacer::new();
    let mut rows = 0;
    for chart in charts.iter().filter(|chart| {
        chrono::DateTime::parse_from_rfc3339(&chart.last_activity)
            .is_ok_and(|time| time.timestamp() < cutoff)
    }) {
        let eviction = screenerbot::ohlcvs::retention::evict_chart_history(
            &mut conn,
            CHAIN,
            &chart.mint,
            &chart.last_activity,
            &|| pacer.stop(),
        )
        .expect("evict chart");
        rows += eviction.rows;
        if eviction.interrupted && !full_store_requested() {
            break;
        }
    }
    eprintln!(
        "REALDATA ohlcvs: {} inactive charts, evicted {rows} rows",
        charts.len()
    );
    pacer.assert_within_budget("ohlcvs.db");
}
