// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the strategies store: the paper or live mode a store gains, the rules clock a
//! v4 store gains and keeps in line with edits by an older core, the versions that read
//! the store, the evaluation totals a batch adds to in one transaction, the one order
//! every strategy listing reads, and the enabled counts of each mode.

use rusqlite::{params, Connection};

use super::*;
use crate::chains::ChainId;
use crate::database::schema::ledger::{self, Assessment};
use crate::database::{catalog, open};
use crate::strategies::types::EvaluationTotals;

/// Opens `path` as the strategies store, as the storage phase does.
fn open_store(path: &std::path::Path) {
    open::pool_at(&catalog::STRATEGIES, path, vec![ChainId::Solana]).unwrap();
}

fn insert_row(conn: &Connection, id: &str, kind: &str, enabled: bool) {
    conn.execute(
        "INSERT INTO strategies (id, name, type, enabled, rules_json, created_at, updated_at)
         VALUES (?1, ?1, ?2, ?3, '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        params![id, kind, enabled],
    )
    .unwrap();
}

/// A store as a version 3 core left it: the strategies table as that core created it,
/// with no mode column, and the ledger at version 3. The table is recreated from its
/// version 3 definition rather than altered, so it carries exactly the shape a user's
/// store has.
fn version_3_store(path: &std::path::Path) {
    open_store(path);
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "DROP TABLE strategies;
         CREATE TABLE strategies (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             description TEXT,
             type TEXT NOT NULL, -- 'ENTRY' or 'EXIT'
             enabled INTEGER NOT NULL DEFAULT 1,
             priority INTEGER NOT NULL DEFAULT 10,
             timeframe TEXT NOT NULL DEFAULT '5m', -- Which timeframe to use for OHLCV conditions
             rules_json TEXT NOT NULL,
             parameters_json TEXT,
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL,
             author TEXT,
             version INTEGER NOT NULL DEFAULT 1,
             entry_strategy_ids_json TEXT NOT NULL DEFAULT '[]' -- exit strategies: the entry strategies managed, empty = all
         );
         CREATE INDEX idx_strategies_type ON strategies(type);
         CREATE INDEX idx_strategies_enabled ON strategies(enabled);
         CREATE INDEX idx_strategies_priority ON strategies(priority);
         UPDATE store_schema SET version = 3, min_reader_version = 2 WHERE id = 1;",
    )
    .unwrap();
    insert_row(&conn, "entry-on", "ENTRY", true);
    insert_row(&conn, "exit-off", "EXIT", false);
}

#[test]
fn a_version_3_store_gains_the_live_mode_and_survives_a_second_open() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    version_3_store(&path);

    open_store(&path);
    open_store(&path);

    let conn = Connection::open(&path).unwrap();
    let rows: Vec<(String, bool, String)> = conn
        .prepare("SELECT id, enabled, mode FROM strategies ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows,
        [
            ("entry-on".to_owned(), true, "live".to_owned()),
            ("exit-off".to_owned(), false, "live".to_owned()),
        ],
        "every strategy of an older store keeps trading live"
    );
    let row = ledger::read(&conn).unwrap().unwrap();
    assert_eq!(
        (row.version, row.min_reader_version),
        (STORE_VERSION, MIN_READER_VERSION)
    );
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
    assert!(
        conn.execute(
            "UPDATE strategies SET mode = 'shadow' WHERE id = 'entry-on'",
            []
        )
        .is_err(),
        "the mode is paper or live only"
    );
}

/// The mode changes how a strategy trades, so a core without it must refuse the store;
/// version 5 changes the meaning of no existing column, so a version 4 core still opens it.
#[test]
fn a_v4_core_opens_the_v5_store_and_a_v3_core_does_not() {
    assert_eq!(MIN_READER_VERSION, 4);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let row = ledger::read(&Connection::open(&path).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!((row.version, row.min_reader_version), (5, 4));
    let core = |version| StoreSchema { version, ..SCHEMA };
    assert_eq!(
        ledger::assess(Some(&row), &core(4)),
        Assessment::CompatibleNewer
    );
    assert_eq!(ledger::assess(Some(&row), &core(3)), Assessment::TooNew);
    assert_eq!(ledger::assess(Some(&row), &SCHEMA), Assessment::Current);
}

/// The `strategies` table as a version 4 core created it: no rules clock.
const VERSION_4_STRATEGIES: &str = "
    DROP TABLE strategy_evaluation_totals;
    DROP TABLE strategies;
    CREATE TABLE strategies (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        type TEXT NOT NULL, -- 'ENTRY' or 'EXIT'
        enabled INTEGER NOT NULL DEFAULT 1,
        priority INTEGER NOT NULL DEFAULT 10,
        timeframe TEXT NOT NULL DEFAULT '5m', -- Which timeframe to use for OHLCV conditions
        rules_json TEXT NOT NULL,
        parameters_json TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        author TEXT,
        version INTEGER NOT NULL DEFAULT 1,
        entry_strategy_ids_json TEXT NOT NULL DEFAULT '[]', -- exit strategies: the entry strategies managed, empty = all
        mode TEXT NOT NULL DEFAULT 'live' CHECK(mode IN ('paper', 'live')) -- the book the strategy trades
    );
    CREATE INDEX idx_strategies_type ON strategies(type);
    CREATE INDEX idx_strategies_enabled ON strategies(enabled);
    CREATE INDEX idx_strategies_priority ON strategies(priority);
    UPDATE store_schema SET version = 4, min_reader_version = 4 WHERE id = 1;";

/// A strategy row as a version 4 core writes it: every column it knows, no rules clock.
fn insert_as_older_core(conn: &Connection, id: &str, kind: &str, created: &str, updated: &str) {
    conn.execute(
        "INSERT INTO strategies (id, name, type, enabled, rules_json, parameters_json, \
         created_at, updated_at)
         VALUES (?1, ?1, ?2, 1, '{}', '{}', ?3, ?4)",
        params![id, kind, created, updated],
    )
    .unwrap();
}

/// One evaluation row as a version 4 core's recorder writes it.
fn insert_evaluation_row(
    conn: &Connection,
    strategy_id: &str,
    signalled: bool,
    millis: i64,
    at: &str,
) {
    conn.execute(
        "INSERT INTO strategy_performance (strategy_id, execution_time_ms, result, confidence, \
         details_json, token_mint, execution_timestamp, chain_id)
         VALUES (?1, ?2, ?3, 1.0, '{}', 'mint', ?4, 'solana')",
        params![strategy_id, millis, signalled, at],
    )
    .unwrap();
}

/// A store as a version 4 core left it: two strategies, an entry created on 1 January and
/// last updated on 5 January and an exit never updated, with no rules clock and no
/// evaluation totals.
fn version_4_store(path: &std::path::Path) {
    open_store(path);
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(VERSION_4_STRATEGIES).unwrap();
    insert_as_older_core(
        &conn,
        "entry",
        "ENTRY",
        "2026-01-01T00:00:00+00:00",
        "2026-01-05T00:00:00+00:00",
    );
    insert_as_older_core(
        &conn,
        "exit",
        "EXIT",
        "2026-01-02T00:00:00+00:00",
        "2026-01-02T00:00:00+00:00",
    );
}

fn time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

/// The stored strategy `id`.
fn stored(conn: &Connection, id: &str) -> Strategy {
    all_strategies(conn)
        .unwrap()
        .into_iter()
        .find(|strategy| strategy.id == id)
        .unwrap()
}

/// Every strategy's rules clock and fingerprint as stored.
fn clock_columns(conn: &Connection) -> Vec<(String, Option<String>, Option<String>)> {
    conn.prepare("SELECT id, rules_changed_at, rules_fingerprint FROM strategies ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn a_version_4_store_upgrades_without_restarting_readiness() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    version_4_store(&path);

    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    for (id, created) in [
        ("entry", "2026-01-01T00:00:00+00:00"),
        ("exit", "2026-01-02T00:00:00+00:00"),
    ] {
        let strategy = stored(&conn, id);
        assert_eq!(
            strategy.rules_changed_at,
            time(created),
            "{id}: the clock starts at the creation, not the last update"
        );
        let (_, _, fingerprint) = clock_columns(&conn)
            .into_iter()
            .find(|(stored_id, _, _)| stored_id == id)
            .unwrap();
        assert_eq!(fingerprint, Some(strategy.rules_fingerprint()), "{id}");
    }
    let row = ledger::read(&conn).unwrap().unwrap();
    assert_eq!((row.version, row.min_reader_version), (5, 4));

    let upgraded = clock_columns(&conn);
    drop(conn);
    open_store(&path);
    assert_eq!(
        clock_columns(&Connection::open(&path).unwrap()),
        upgraded,
        "a second open changes nothing"
    );
}

#[test]
fn an_edit_by_an_older_core_restarts_the_rules_clock_at_the_next_open() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    version_4_store(&path);
    open_store(&path);

    // A version 4 core writes the new rules and its update time, and leaves the
    // fingerprint as it was; it also switches the exit strategy off, which is no rules
    // change.
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE strategies SET rules_json = '{\"operator\":\"AND\",\"conditions\":[]}', \
         updated_at = '2026-02-01T00:00:00+00:00' WHERE id = 'entry'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE strategies SET enabled = 0, updated_at = '2026-02-02T00:00:00+00:00' \
         WHERE id = 'exit'",
        [],
    )
    .unwrap();
    drop(conn);

    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    let entry = stored(&conn, "entry");
    assert_eq!(entry.rules_changed_at, time("2026-02-01T00:00:00+00:00"));
    assert_eq!(
        stored(&conn, "exit").rules_changed_at,
        time("2026-01-02T00:00:00+00:00"),
        "switching a strategy off is not a rules change"
    );
    let reconciled = clock_columns(&conn);
    assert!(reconciled.iter().any(|(id, _, fingerprint)| id == "entry"
        && fingerprint.as_deref() == Some(entry.rules_fingerprint().as_str())));
    drop(conn);
    open_store(&path);
    assert_eq!(clock_columns(&Connection::open(&path).unwrap()), reconciled);
}

#[test]
fn a_row_inserted_by_an_older_core_gets_a_clock() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    insert_as_older_core(
        &conn,
        "late",
        "ENTRY",
        "2026-03-01T00:00:00+00:00",
        "2026-03-04T00:00:00+00:00",
    );
    drop(conn);

    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    let late = stored(&conn, "late");
    assert_eq!(
        late.rules_changed_at,
        time("2026-03-04T00:00:00+00:00"),
        "a clock it never had is read no earlier than its last update"
    );
    assert_eq!(
        clock_columns(&conn),
        [(
            "late".to_owned(),
            Some(late.rules_changed_at.to_rfc3339()),
            Some(late.rules_fingerprint())
        )]
    );
}

/// An exit strategy created on 1 January, bound to two entry strategies, carrying a rules
/// clock the store must ignore.
fn exit_strategy() -> Strategy {
    let created_at = time("2026-01-01T00:00:00+00:00");
    Strategy {
        id: "exit".to_owned(),
        name: "Exit".to_owned(),
        description: None,
        strategy_type: StrategyType::Exit,
        enabled: true,
        priority: 10,
        timeframe: "5m".to_owned(),
        rules: serde_json::from_value(serde_json::json!({
            "condition": {
                "type": "PositionProfit",
                "parameters": { "threshold_pct": { "value": 20.0, "default": 20.0 } }
            }
        }))
        .unwrap(),
        parameters: std::collections::HashMap::new(),
        created_at,
        updated_at: created_at,
        author: None,
        version: 1,
        entry_strategy_ids: vec!["a".to_owned(), "b".to_owned()],
        mode: TradingMode::Paper,
        rules_changed_at: time("2030-01-01T00:00:00+00:00"),
    }
}

#[test]
fn update_moves_the_rules_clock_only_when_the_rules_change() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let mut conn = Connection::open(&path).unwrap();
    insert_strategy_in(&conn, &exit_strategy()).unwrap();
    let created = time("2026-01-01T00:00:00+00:00");
    assert_eq!(
        stored(&conn, "exit").rules_changed_at,
        created,
        "an insert starts the clock at the creation, whatever the strategy carries"
    );

    let unchanged = [
        Strategy {
            name: "Renamed".to_owned(),
            ..exit_strategy()
        },
        Strategy {
            priority: 3,
            ..exit_strategy()
        },
        Strategy {
            enabled: false,
            ..exit_strategy()
        },
        Strategy {
            description: Some("Described".to_owned()),
            ..exit_strategy()
        },
        Strategy {
            entry_strategy_ids: vec!["b".to_owned(), "a".to_owned()],
            ..exit_strategy()
        },
    ];
    for strategy in &unchanged {
        assert!(!update_strategy_in(&mut conn, strategy).unwrap());
        assert_eq!(stored(&conn, "exit").rules_changed_at, created);
    }
    set_strategy_mode_in(&conn, "exit", TradingMode::Live).unwrap();
    assert_eq!(stored(&conn, "exit").rules_changed_at, created);

    let mut edited = exit_strategy();
    edited.rules_changed_at = time("2000-01-01T00:00:00+00:00");
    edited
        .rules
        .condition
        .as_mut()
        .unwrap()
        .parameters
        .get_mut("threshold_pct")
        .unwrap()
        .value = serde_json::json!(25.0);
    let before = Utc::now();
    assert!(update_strategy_in(&mut conn, &edited).unwrap());
    let changed_at = stored(&conn, "exit").rules_changed_at;
    assert!(
        changed_at >= before && changed_at <= Utc::now(),
        "the clock moves to the update, never to the time the strategy carried: {changed_at}"
    );
    assert!(!update_strategy_in(&mut conn, &edited).unwrap());
    assert_eq!(stored(&conn, "exit").rules_changed_at, changed_at);

    assert!(
        update_strategy_in(
            &mut conn,
            &Strategy {
                id: "missing".to_owned(),
                ..exit_strategy()
            }
        )
        .is_err(),
        "updating a strategy that does not exist stays an error"
    );
}

fn evaluation(
    strategy_id: &str,
    result: bool,
    millis: u64,
    evaluated_at: &str,
) -> EvaluationRecord {
    EvaluationRecord {
        result: EvaluationResult {
            strategy_id: strategy_id.to_owned(),
            result,
            confidence: 1.0,
            execution_time_ms: millis,
            details: std::collections::HashMap::new(),
        },
        evaluated_at: DateTime::parse_from_rfc3339(evaluated_at)
            .unwrap()
            .with_timezone(&Utc),
    }
}

fn totals(
    evaluations: u64,
    signals: u64,
    total_exec_ms: u64,
    last_evaluated_at: &str,
    last_signal_at: Option<&str>,
) -> EvaluationTotals {
    EvaluationTotals {
        evaluations,
        signals,
        total_exec_ms,
        last_evaluated_at: Some(time(last_evaluated_at)),
        last_signal_at: last_signal_at.map(time),
    }
}

/// The evaluations a version 4 core recorded per row become the totals at the upgrade,
/// and the rows go; rows an older core writes later are added at the next open.
#[test]
fn the_totals_start_from_the_recorded_evaluations() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    version_4_store(&path);
    let conn = Connection::open(&path).unwrap();
    insert_evaluation_row(&conn, "entry", true, 4, "2026-01-02T00:00:00+00:00");
    insert_evaluation_row(&conn, "entry", false, 2, "2026-01-03T00:00:00.250+00:00");
    insert_evaluation_row(&conn, "entry", true, 6, "2026-01-01T00:00:00+00:00");
    insert_evaluation_row(&conn, "exit", false, 1, "2026-01-02T12:00:00+00:00");
    drop(conn);

    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    let recorded = evaluation_totals(&conn).unwrap();
    assert_eq!(
        recorded.get("entry"),
        Some(&totals(
            3,
            2,
            12,
            "2026-01-03T00:00:00.250Z",
            Some("2026-01-02T00:00:00Z")
        ))
    );
    assert_eq!(
        recorded.get("exit"),
        Some(&totals(1, 0, 1, "2026-01-02T12:00:00Z", None))
    );
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM strategy_performance", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(remaining, 0, "the folded rows are deleted");

    insert_evaluation_row(&conn, "exit", true, 5, "2026-01-04T00:00:00+00:00");
    drop(conn);
    open_store(&path);
    open_store(&path);
    assert_eq!(
        evaluation_totals(&Connection::open(&path).unwrap())
            .unwrap()
            .get("exit"),
        Some(&totals(
            2,
            1,
            6,
            "2026-01-04T00:00:00Z",
            Some("2026-01-04T00:00:00Z")
        )),
        "a row an older core wrote since is counted once"
    );
}

/// The totals equal the aggregate of the recorded evaluations; a later batch adds to
/// them, and a last time moves only later, the last signal only on a signal.
#[test]
fn a_batch_adds_to_the_totals_in_its_transaction() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let mut conn = Connection::open(&path).unwrap();
    insert_row(&conn, "entry", "ENTRY", true);
    insert_row(&conn, "exit", "EXIT", true);
    insert_row(&conn, "never-run", "ENTRY", true);

    let first = [
        evaluation("entry", true, 4, "2026-01-02T00:00:00Z"),
        evaluation("entry", false, 2, "2026-01-03T00:00:00Z"),
        evaluation("entry", true, 6, "2026-01-01T00:00:00Z"),
        evaluation("exit", false, 1, "2026-01-01T00:00:00Z"),
    ];
    insert_evaluations(&mut conn, &first).unwrap();
    let recorded = evaluation_totals(&conn).unwrap();
    let aggregate = batch_totals(&first);
    assert_eq!(recorded.len(), aggregate.len());
    for (strategy_id, expected) in &aggregate {
        assert_eq!(recorded.get(*strategy_id), Some(expected), "{strategy_id}");
    }
    assert_eq!(
        recorded.get("entry"),
        Some(&totals(
            3,
            2,
            12,
            "2026-01-03T00:00:00Z",
            Some("2026-01-02T00:00:00Z")
        ))
    );
    assert!(
        !recorded.contains_key("never-run"),
        "a strategy never evaluated has no totals"
    );

    insert_evaluations(
        &mut conn,
        &[
            evaluation("entry", false, 3, "2026-01-04T00:00:00Z"),
            evaluation("entry", true, 1, "2025-12-31T00:00:00Z"),
        ],
    )
    .unwrap();
    assert_eq!(
        evaluation_totals(&conn).unwrap().get("entry"),
        Some(&totals(
            5,
            3,
            16,
            "2026-01-04T00:00:00Z",
            Some("2026-01-02T00:00:00Z")
        )),
        "a no-signal moves only the last evaluation; an older signal moves no last time"
    );
}

/// An evaluation of a strategy deleted between its evaluation and the write is dropped
/// alone; the rest of its batch is counted.
#[test]
fn an_evaluation_of_a_deleted_strategy_does_not_fail_its_batch() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let mut conn = Connection::open(&path).unwrap();
    insert_row(&conn, "entry", "ENTRY", true);

    insert_evaluations(
        &mut conn,
        &[
            evaluation("deleted", true, 1, "2026-01-01T00:00:00Z"),
            evaluation("entry", true, 2, "2026-01-01T00:00:00Z"),
        ],
    )
    .unwrap();
    let recorded = evaluation_totals(&conn).unwrap();
    assert_eq!(
        recorded.get("entry"),
        Some(&totals(
            1,
            1,
            2,
            "2026-01-01T00:00:00Z",
            Some("2026-01-01T00:00:00Z")
        ))
    );
    assert!(!recorded.contains_key("deleted"));
}

/// A batch is one transaction: a write that fails part way leaves the totals of every
/// strategy in the batch as they were.
#[test]
fn a_batch_that_fails_part_way_writes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let mut conn = Connection::open(&path).unwrap();
    insert_row(&conn, "a", "ENTRY", true);
    insert_row(&conn, "b", "ENTRY", true);
    conn.execute_batch(
        "CREATE TRIGGER refuse_b BEFORE INSERT ON strategy_evaluation_totals
         WHEN NEW.strategy_id = 'b' BEGIN SELECT RAISE(ABORT, 'refused'); END;",
    )
    .unwrap();

    assert!(insert_evaluations(
        &mut conn,
        &[
            evaluation("a", true, 1, "2026-01-01T00:00:00Z"),
            evaluation("b", true, 1, "2026-01-01T00:00:00Z"),
        ],
    )
    .is_err());
    assert!(evaluation_totals(&conn).unwrap().is_empty());
}

/// The trader's enabled entries are the full listing filtered to enabled entries, element
/// by element, so the evaluation order of each book is the order the strategies overview
/// ranks: lowest priority first, the name breaking a tie whatever order the rows were
/// written in.
#[test]
fn the_enabled_entries_are_the_full_listing_filtered() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    // Written out of name order, with ids that sort differently from the names.
    for (id, name, kind, enabled, priority, mode) in [
        ("a", "zeta", "ENTRY", true, 1, "live"),
        ("b", "delta", "ENTRY", true, 1, "paper"),
        ("c", "alpha", "EXIT", true, 1, "live"),
        ("d", "beta", "ENTRY", false, 1, "live"),
        ("e", "omega", "ENTRY", true, 2, "live"),
        ("f", "epsilon", "ENTRY", true, 1, "live"),
        ("g", "gamma", "ENTRY", true, 2, "paper"),
    ] {
        conn.execute(
            "INSERT INTO strategies (id, name, type, enabled, priority, rules_json, \
             parameters_json, created_at, updated_at, mode)
             VALUES (?1, ?2, ?3, ?4, ?5, '{}', '{}', '2026-01-01T00:00:00Z', \
             '2026-01-01T00:00:00Z', ?6)",
            params![id, name, kind, enabled, priority, mode],
        )
        .unwrap();
    }

    let ids = |strategies: Vec<Strategy>| -> Vec<String> {
        strategies.into_iter().map(|strategy| strategy.id).collect()
    };
    let enabled = ids(enabled_strategies(&conn, StrategyType::Entry).unwrap());
    let listed = ids(all_strategies(&conn)
        .unwrap()
        .into_iter()
        .filter(|strategy| strategy.enabled && strategy.strategy_type == StrategyType::Entry)
        .collect());
    assert_eq!(enabled, listed);
    assert_eq!(enabled, ["b", "f", "a", "g", "e"]);
}

/// The counts split the enabled strategies of each mode, and the entry strategies among
/// them; a disabled strategy counts nowhere.
#[test]
fn enabled_strategy_counts_split_entry_strategies_by_mode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let conn = Connection::open(&path).unwrap();
    for (id, kind, enabled, mode) in [
        ("a", "ENTRY", true, "live"),
        ("b", "ENTRY", true, "live"),
        ("c", "EXIT", true, "live"),
        ("d", "ENTRY", false, "live"),
        ("e", "EXIT", true, "paper"),
        ("f", "ENTRY", false, "paper"),
    ] {
        conn.execute(
            "INSERT INTO strategies (id, name, type, enabled, rules_json, created_at, \
             updated_at, mode)
             VALUES (?1, ?1, ?2, ?3, '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', ?4)",
            params![id, kind, enabled, mode],
        )
        .unwrap();
    }

    assert_eq!(
        strategy_counts(&conn).unwrap(),
        EnabledStrategyCounts {
            live: 3,
            paper: 1,
            live_entry: 2,
            paper_entry: 0,
        }
    );
}
