// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The chain-scope migration for the shared-file stores: `events`, the five
//! RPC-stats tables, `strategy_performance`, tools `watched_tokens` and
//! `mw_sessions`, `actions`, and `ai_decision_history` gain
//! `chain_id TEXT NOT NULL DEFAULT 'solana'`.
//!
//! Each database is seeded from DDL frozen at release tag `v0.2.13`
//! (`tests/fixtures/v0.2.13-*.sql`) with one sentinel row per table, then
//! opened through the same public entry points `tests/storage_startup.rs`
//! exercises — in a CHILD process, twice, because every one of these stores
//! memoises its handle in a process global, and the real upgrade path is
//! "boot, migrate, reboot". A third child boots the same code on an empty
//! directory, proving the fresh-schema `CREATE`s and the migrated tables
//! agree column-for-column (an `ADD COLUMN` can only append, so `chain_id`
//! must be last in both shapes).
//!
//! The `strategies` and `tools` fixtures deliberately carry `schema_version`
//! = 1 — the state a real v0.2.13 database holds — which is exactly why the
//! chain-column migration gates on the live schema and never on that stamp.

use rusqlite::Connection;
use std::process::Command;

const CHILD_ENV: &str = "SCREENERBOT_CHAIN_COLUMNS_MIGRATION_CHILD";

/// (database file, table) for every chain-scoped table this unit covers —
/// the design's list, no more.
const CHAIN_SCOPED_TABLES: &[(&str, &str)] = &[
    ("events.db", "events"),
    ("rpc_stats.db", "sessions"),
    ("rpc_stats.db", "providers"),
    ("rpc_stats.db", "calls"),
    ("rpc_stats.db", "minute_buckets"),
    ("rpc_stats.db", "provider_health"),
    ("strategies.db", "strategy_performance"),
    ("tools.db", "watched_tokens"),
    ("tools.db", "mw_sessions"),
    ("actions.db", "actions"),
    ("ai.db", "ai_decision_history"),
];

/// The audit: every table in these databases that is NOT chain-scoped. The
/// strategy/tool definition and schema-version tables are chain-neutral by
/// design; the ATA tables belong to Solana-only tools (they move under
/// `chains/solana` in a later unit); `action_steps` rows inherit their
/// action's chain; `ai_instructions` are user-authored configuration.
const TABLES_WITHOUT_CHAIN_SCOPE: &[(&str, &[&str])] = &[
    ("events.db", &[]),
    ("rpc_stats.db", &[]),
    (
        "strategies.db",
        &[
            "schema_version",
            "strategies",
            "strategy_assignments",
            "strategy_backtests",
            "strategy_templates",
        ],
    ),
    (
        "tools.db",
        &[
            "ata_closures",
            "ata_failed_cache",
            "ata_sessions",
            "mw_wallet_ops",
            "schema_version",
            "tool_favorites",
        ],
    ),
    ("actions.db", &["action_steps"]),
    ("ai.db", &["ai_instructions"]),
];

fn seed_fixture(directory: &std::path::Path, database: &str, fixture: &str) {
    let data = directory.join("data");
    std::fs::create_dir_all(&data).expect("create fixture data directory");
    let conn = Connection::open(data.join(database)).expect("open fixture database");
    conn.execute_batch(fixture).expect("seed v0.2.13 fixture");
}

fn run_child(directory: &std::path::Path) {
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("v0_2_13_fixtures_gain_chain_scope_and_survive_a_second_boot")
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env("SCREENERBOT_DATA_DIR", directory)
        .status()
        .expect("run chain-columns migration child");
    assert!(
        status.success(),
        "chain-columns migration child failed for {}",
        directory.display()
    );
}

fn open_database(directory: &std::path::Path, database: &str) -> Connection {
    Connection::open(directory.join("data").join(database))
        .unwrap_or_else(|e| panic!("open migrated {database}: {e}"))
}

fn column_names(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap_or_else(|e| panic!("inspect {table}: {e}"));
    stmt.query_map([], |row| row.get::<_, String>(1))
        .unwrap_or_else(|e| panic!("read {table} columns: {e}"))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| panic!("collect {table} columns: {e}"))
}

#[test]
fn v0_2_13_fixtures_gain_chain_scope_and_survive_a_second_boot() {
    if std::env::var_os(CHILD_ENV).is_some() {
        initialize_shared_stores();
        return;
    }

    let fixture_dir = tempfile::tempdir().expect("create fixture data directory");
    seed_fixture(
        fixture_dir.path(),
        "events.db",
        include_str!("fixtures/v0.2.13-events.sql"),
    );
    seed_fixture(
        fixture_dir.path(),
        "rpc_stats.db",
        include_str!("fixtures/v0.2.13-rpc_stats.sql"),
    );
    seed_fixture(
        fixture_dir.path(),
        "strategies.db",
        include_str!("fixtures/v0.2.13-strategies.sql"),
    );
    seed_fixture(
        fixture_dir.path(),
        "tools.db",
        include_str!("fixtures/v0.2.13-tools.sql"),
    );
    seed_fixture(
        fixture_dir.path(),
        "actions.db",
        include_str!("fixtures/v0.2.13-actions.sql"),
    );
    seed_fixture(
        fixture_dir.path(),
        "ai.db",
        include_str!("fixtures/v0.2.13-ai.sql"),
    );

    // Boot one migrates; boot two reopens the migrated files in a fresh
    // process — the idempotence a real user's next reboot exercises.
    run_child(fixture_dir.path());
    run_child(fixture_dir.path());

    // Boot three: the same code on an empty directory. The fresh-schema
    // CREATEs and the migrated tables must agree column-for-column.
    let fresh_dir = tempfile::tempdir().expect("create fresh data directory");
    run_child(fresh_dir.path());

    for (database, table) in CHAIN_SCOPED_TABLES {
        let migrated = open_database(fixture_dir.path(), database);

        // The chain column exists, is the LAST column (ADD COLUMN can only
        // append), and every migrated row carries Solana.
        let columns = column_names(&migrated, table);
        assert_eq!(
            columns.last().map(String::as_str),
            Some("chain_id"),
            "{database}/{table}: chain_id must exist as the last column, got {columns:?}"
        );
        let chains: Vec<String> = migrated
            .prepare(&format!("SELECT DISTINCT chain_id FROM {table}"))
            .unwrap_or_else(|e| panic!("read distinct chains on {database}/{table}: {e}"))
            .query_map([], |row| row.get(0))
            .unwrap_or_else(|e| panic!("scan chains on {database}/{table}: {e}"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("collect chains on {database}/{table}: {e}"));
        assert_eq!(
            chains,
            vec!["solana".to_owned()],
            "{database}/{table}: every pre-migration row must carry chain_id = 'solana'"
        );

        // The sentinel row survived the migration and the second boot.
        let rows: i64 = migrated
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("count {database}/{table}: {e}"));
        assert_eq!(rows, 1, "{database}/{table}: sentinel row count");

        // Migrated and fresh shapes agree column-for-column.
        let fresh = open_database(fresh_dir.path(), database);
        assert_eq!(
            column_names(&migrated, table),
            column_names(&fresh, table),
            "{database}/{table}: fresh schema and migrated schema disagree"
        );
    }

    // Per-database integrity: the migrations are non-destructive and keep
    // every foreign key satisfied.
    let mut seen = std::collections::BTreeSet::new();
    for (database, _) in CHAIN_SCOPED_TABLES {
        if !seen.insert(database.to_owned()) {
            continue;
        }
        let conn = open_database(fixture_dir.path(), database);
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("integrity check on {database}: {e}"));
        assert_eq!(integrity, "ok", "{database}: integrity check");
        let foreign_key_errors: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("foreign-key check on {database}: {e}"));
        assert_eq!(
            foreign_key_errors, 0,
            "{database}: foreign-key violations after migration"
        );
    }

    // The audit: no table in these databases is chain-scoped without a
    // chain_id column. A new table in one of these files appears here and
    // fails this assertion until it is either given chain_id or listed with
    // a reason. It reads the FRESH directory: the fixture databases carry
    // the v0.2.13 table set (their version stamp suppresses the tables that
    // did not exist yet), while a fresh boot creates the full current
    // schema, which is what the audit must cover.
    for (database, expected) in TABLES_WITHOUT_CHAIN_SCOPE {
        let conn = open_database(fresh_dir.path(), database);
        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master
                 WHERE type = 'table'
                   AND name NOT LIKE 'sqlite_%'
                   AND name NOT LIKE '__startup%'
                 ORDER BY name",
            )
            .unwrap_or_else(|e| panic!("list tables in {database}: {e}"));
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap_or_else(|e| panic!("scan tables in {database}: {e}"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("collect tables in {database}: {e}"));
        let without_chain: Vec<String> = tables
            .iter()
            .filter(|table| !column_names(&conn, table).iter().any(|c| c == "chain_id"))
            .cloned()
            .collect();
        assert_eq!(
            without_chain,
            expected.to_vec(),
            "{database}: tables without chain_id must be exactly the chain-neutral set"
        );
    }
}

/// The production openers, spelled exactly as `tests/storage_startup.rs`
/// spells them — the same entry points a real boot takes.
fn initialize_shared_stores() {
    screenerbot::paths::ensure_all_directories().expect("create startup-storage directories");
    screenerbot::config::load_config().expect("load isolated configuration");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create multi-thread runtime");
    runtime.block_on(async {
        screenerbot::actions::init_database()
            .await
            .expect("initialize actions database");
        screenerbot::events::database::EventsDatabase::new()
            .await
            .expect("initialize events database");
        screenerbot::strategies::init_strategy_system(
            screenerbot::strategies::engine::EngineConfig::default(),
        )
        .await
        .expect("initialize strategies database");
    });

    screenerbot::tools::init_tools_db().expect("initialize tools database");
    screenerbot::llm_analysis::init_analysis_database().expect("initialize analysis database");
    screenerbot::rpc::RpcStatsDatabase::new().expect("initialize RPC statistics database");
}
