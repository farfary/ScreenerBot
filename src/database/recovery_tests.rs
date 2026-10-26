// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the store probe, quarantine, restore and salvage.

use super::*;
use crate::database::catalog::{OHLCVS, POOLS};

fn store_with(path: &Path, sql: &str) {
    let connection = Connection::open(path).expect("create store");
    configure_connection(&connection, POOLS.config).expect("configure store");
    connection.execute_batch(sql).expect("seed store");
}

#[test]
fn probe_classifies_foreign_truncated_and_healthy_files() {
    let directory = tempfile::tempdir().expect("temp dir");
    let missing = directory.path().join("missing.db");
    assert!(probe(&missing).is_ok());

    let empty = directory.path().join("empty.db");
    fs::write(&empty, b"").expect("write empty file");
    assert!(matches!(
        probe(&empty),
        Err(DatabaseError::NotADatabase { .. })
    ));

    let garbage = directory.path().join("garbage.db");
    fs::write(&garbage, vec![0x5a_u8; 8192]).expect("write garbage");
    assert!(matches!(
        probe(&garbage),
        Err(DatabaseError::NotADatabase { .. })
    ));

    let short = directory.path().join("short.db");
    fs::write(&short, b"SQLite format 3\0").expect("write short file");
    assert!(matches!(
        probe(&short),
        Err(DatabaseError::NotADatabase { .. })
    ));

    let healthy = directory.path().join("healthy.db");
    store_with(
        &healthy,
        "CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);",
    );
    assert!(probe(&healthy).is_ok());

    let truncated = directory.path().join("truncated.db");
    let mut bytes = fs::read(&healthy).expect("read healthy store");
    bytes.truncate(bytes.len() - 1);
    fs::write(&truncated, bytes).expect("write truncated copy");
    assert!(matches!(
        probe(&truncated),
        Err(DatabaseError::NotADatabase { .. })
    ));
}

#[test]
fn quarantine_moves_the_store_and_sidecars_and_writes_a_manifest() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("pools.db");
    // The open connection keeps the WAL and shared-memory files on disk.
    let open = Connection::open(&path).expect("create store");
    configure_connection(&open, POOLS.config).expect("configure store");
    open.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);")
        .expect("seed store");
    let sidecars: Vec<PathBuf> = get_db_with_wal_files(path.clone())
        .into_iter()
        .filter(|file| file.exists())
        .collect();
    assert!(
        sidecars.len() > 1,
        "WAL sidecars exist while the store is open"
    );
    drop(open);
    let sidecars: Vec<PathBuf> = get_db_with_wal_files(path.clone())
        .into_iter()
        .filter(|file| file.exists())
        .collect();

    let root = directory.path().join("quarantine");
    let moved =
        quarantine_into(&root, &POOLS, &path, "file is not a database").expect("quarantine store");

    assert!(moved.directory.starts_with(&root));
    assert_eq!(moved.store_file, moved.directory.join("pools.db"));
    for file in &sidecars {
        assert!(!file.exists(), "{} left behind", file.display());
        assert!(moved.directory.join(file.file_name().unwrap()).exists());
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(moved.directory.join("manifest.json")).expect("read manifest"),
    )
    .expect("decode manifest");
    assert_eq!(manifest["store"], "pools");
    assert_eq!(manifest["class"], "cache");
    assert_eq!(manifest["reason"], "file is not a database");
    assert_eq!(
        manifest["originalFiles"].as_array().map(Vec::len),
        Some(sidecars.len())
    );
    let rows: i64 = Connection::open(&moved.store_file)
        .and_then(|copy| copy.query_row("SELECT count(*) FROM t", [], |row| row.get(0)))
        .expect("read quarantined store");
    assert_eq!(rows, 1, "the quarantined store keeps its rows");
}

#[test]
fn moving_store_files_skips_missing_files_and_resumes_an_interrupted_move() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("positions.db");
    let wal = directory.path().join("positions.db-wal");
    fs::write(&path, b"store").expect("write store");
    fs::write(&wal, b"wal").expect("write wal");
    let target = directory.path().join("backup");

    // An interrupted move left the sidecar in the target and the store in place.
    fs::create_dir(&target).expect("create target");
    fs::rename(&wal, target.join("positions.db-wal")).expect("pre-move wal");

    let moved = move_store_files(&path, &target).expect("move store files");

    assert_eq!(moved, vec![path.display().to_string()]);
    assert!(!path.exists());
    assert_eq!(
        fs::read(target.join("positions.db")).expect("read moved"),
        b"store"
    );
    assert_eq!(
        fs::read(target.join("positions.db-wal")).expect("read wal"),
        b"wal"
    );
    assert!(
        move_store_files(&path, &target)
            .expect("repeat move")
            .is_empty(),
        "a finished move has nothing left to move"
    );
}

#[test]
fn moving_store_files_refuses_an_existing_target_and_keeps_both_files() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("positions.db");
    let wal = directory.path().join("positions.db-wal");
    let target = directory.path().join("backup");
    fs::create_dir(&target).expect("create target");
    fs::write(target.join("positions.db-wal"), b"original wal").expect("write moved wal");
    fs::write(&path, b"re-created store").expect("write store");
    fs::write(&wal, b"re-created wal").expect("write wal");

    let refused = move_store_files(&path, &target).expect_err("an existing target refuses");

    match refused {
        DatabaseError::MoveTargetExists {
            source_path,
            target_path,
        } => {
            assert_eq!(source_path, wal.display().to_string());
            assert_eq!(
                target_path,
                target.join("positions.db-wal").display().to_string()
            );
        }
        other => panic!("expected a refused move, got {other}"),
    }
    assert_eq!(
        fs::read(target.join("positions.db-wal")).expect("read original"),
        b"original wal"
    );
    assert_eq!(fs::read(&wal).expect("read wal"), b"re-created wal");
    assert_eq!(fs::read(&path).expect("read store"), b"re-created store");
    assert!(
        !target.join("positions.db").exists(),
        "nothing after the refused file moves"
    );
}

#[test]
fn salvage_copies_only_the_listed_rows_and_columns_and_never_writes_the_source() {
    let directory = tempfile::tempdir().expect("temp dir with a space");
    let quarantined_dir = directory.path().join("quarantine dir");
    fs::create_dir(&quarantined_dir).expect("create quarantine dir");
    let quarantined = quarantined_dir.join("ohlcvs.db");
    store_with(
        &quarantined,
        "CREATE TABLE ohlcv_monitor_config (chain_id TEXT, mint TEXT, priority TEXT, source TEXT, backfill_1m_complete INTEGER DEFAULT 0, PRIMARY KEY (chain_id, mint));
         INSERT INTO ohlcv_monitor_config VALUES ('solana', 'manual-a', 'high', 'manual', 1), ('solana', 'auto-b', 'low', 'auto', 1), ('solana', 'kept', 'low', 'manual', 1);",
    );
    let source_before = fs::read(&quarantined).expect("read source");
    let live = directory.path().join("live-ohlcvs.db");
    store_with(
        &live,
        "CREATE TABLE ohlcv_monitor_config (chain_id TEXT, mint TEXT, priority TEXT, source TEXT, backfill_1m_complete INTEGER DEFAULT 0, PRIMARY KEY (chain_id, mint));
         INSERT INTO ohlcv_monitor_config VALUES ('solana', 'kept', 'critical', 'manual', 0);",
    );

    let rows = salvage(&OHLCVS, &quarantined, &live).expect("salvage");

    assert_eq!(rows, 1);
    let connection = Connection::open(&live).expect("open live store");
    let monitors: Vec<(String, String, i64)> = connection
        .prepare(
            "SELECT mint, priority, backfill_1m_complete FROM ohlcv_monitor_config ORDER BY mint",
        )
        .expect("prepare")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("query")
        .collect::<rusqlite::Result<_>>()
        .expect("decode");
    assert_eq!(
        monitors,
        vec![
            ("kept".to_owned(), "critical".to_owned(), 0),
            ("manual-a".to_owned(), "high".to_owned(), 0),
        ],
        "manual monitors salvaged without backfill progress; live rows kept"
    );
    assert_eq!(
        fs::read(&quarantined).expect("reread source"),
        source_before
    );
}

#[test]
fn salvage_skips_an_unreadable_source() {
    let directory = tempfile::tempdir().expect("temp dir");
    let quarantined = directory.path().join("garbage.db");
    fs::write(&quarantined, vec![0x5a_u8; 8192]).expect("write garbage");
    let live = directory.path().join("ohlcvs.db");
    store_with(
        &live,
        "CREATE TABLE ohlcv_monitor_config (chain_id TEXT, mint TEXT, source TEXT, PRIMARY KEY (chain_id, mint));",
    );

    let outcome = salvage(&OHLCVS, &quarantined, &live);

    assert!(matches!(
        outcome,
        Ok(0) | Err(DatabaseError::NotADatabase { .. })
    ));
}

#[test]
fn read_only_uri_escapes_every_reserved_byte() {
    assert_eq!(
        read_only_uri(Path::new("/a b/c?d#e%f.db")),
        "file:/a%20b/c%3Fd%23e%25f.db?mode=ro"
    );
    #[cfg(windows)]
    assert_eq!(
        read_only_uri(Path::new("C:\\Users\\a b\\ohlcvs.db")),
        "file:/C:/Users/a%20b/ohlcvs.db?mode=ro"
    );
}
