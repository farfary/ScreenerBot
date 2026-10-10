// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for store snapshots: WAL-only rows reach the copy and a restore, retention follows the class, and only a verified snapshot is offered for a restore.

use super::*;
use crate::database::catalog::{POSITIONS, RPC_STATS, WALLETS};
use crate::database::configure::POSITIONS_DB;
use crate::database::recovery;

fn rows(path: &Path) -> i64 {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
        .unwrap()
}

fn snapshot_at(day: u32, hour: u32, kind: SnapshotKind) -> Snapshot {
    let taken_at = NaiveDateTime::parse_from_str(
        &format!("202610{day:02}T{hour:02}0000000000Z"),
        STAMP_FORMAT,
    )
    .unwrap()
    .and_utc();
    Snapshot {
        path: PathBuf::from(format!("{day:02}-{hour:02}-{}.db", kind.suffix())),
        taken_at,
        kind,
    }
}

fn names(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// A store whose rows still sit only in its write-ahead log is copied whole, and a
/// restore from that copy brings the rows back in place of a damaged file.
#[test]
fn a_snapshot_keeps_rows_only_the_wal_holds_through_a_restore() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("positions.db");
    let writer = Connection::open(&path).unwrap();
    crate::database::configure_connection(&writer, POSITIONS_DB).unwrap();
    writer
        .execute_batch("PRAGMA wal_autocheckpoint = 0; CREATE TABLE t (v INTEGER NOT NULL);")
        .unwrap();
    for v in 0..25 {
        writer
            .execute("INSERT INTO t (v) VALUES (?1)", [v])
            .unwrap();
    }
    assert!(
        fs::metadata(path.with_extension("db-wal")).unwrap().len() > 0,
        "the rows must still be in the write-ahead log"
    );

    let file = take(&POSITIONS, &path, SnapshotKind::Boot)
        .unwrap()
        .expect("a ledger store is snapshotted");
    assert_eq!(rows(&file), 25);
    assert!(!file.with_extension("db-wal").exists());
    assert_eq!(list(&path).len(), 1);
    drop(writer);

    fs::write(&path, b"damaged".repeat(512)).unwrap();
    let snapshot = latest_verified(&POSITIONS, &path).expect("a verified snapshot");
    let quarantined = recovery::restore(&POSITIONS, &path, &snapshot.path, "test").unwrap();
    assert_eq!(rows(&path), 25);
    assert_eq!(
        fs::read(&quarantined.store_file).unwrap(),
        b"damaged".repeat(512)
    );
    assert!(
        snapshot.path.exists(),
        "a restore never consumes the snapshot"
    );
}

#[test]
fn cache_and_derived_stores_are_never_snapshotted() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("rpc_stats.db");
    Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE t (v INTEGER)")
        .unwrap();
    assert_eq!(take(&RPC_STATS, &path, SnapshotKind::Boot).unwrap(), None);
    assert!(!super::directory(&path).exists());
    assert!(!is_snapshotted(StoreClass::Derived));
    assert!(!is_snapshotted(StoreClass::Cache));
}

/// A ledger store keeps its three newest startup snapshots and one periodic snapshot for
/// each of its seven newest days; everything else expires.
#[test]
fn a_ledger_store_keeps_three_boots_and_seven_days() {
    let mut snapshots = Vec::new();
    for day in (1..=9).rev() {
        snapshots.push(snapshot_at(day, 18, SnapshotKind::Periodic));
        snapshots.push(snapshot_at(day, 12, SnapshotKind::Boot));
        snapshots.push(snapshot_at(day, 6, SnapshotKind::Periodic));
    }
    let expired = names(&expired(StoreClass::Ledger, &snapshots));

    let mut expected = Vec::new();
    for day in (1..=9).rev() {
        if day <= 2 {
            expected.push(format!("{day:02}-18-periodic.db"));
        }
        if day <= 6 {
            expected.push(format!("{day:02}-12-boot.db"));
        }
        expected.push(format!("{day:02}-06-periodic.db"));
    }
    assert_eq!(expired, expected);
}

#[test]
fn the_vault_keeps_its_ten_newest_snapshots_of_either_kind() {
    let snapshots: Vec<Snapshot> = (1..=12)
        .rev()
        .map(|day| {
            let kind = if day % 2 == 0 {
                SnapshotKind::Boot
            } else {
                SnapshotKind::Periodic
            };
            snapshot_at(day, 0, kind)
        })
        .collect();
    assert_eq!(
        names(&expired(StoreClass::Vault, &snapshots)),
        ["02-00-boot.db", "01-00-periodic.db"]
    );
}

/// Taking snapshots prunes the store's own directory to its retention.
#[test]
fn taking_a_snapshot_prunes_the_expired_ones() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("positions.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE t (v INTEGER)").unwrap();
    for _ in 0..5 {
        take(&POSITIONS, &path, SnapshotKind::Boot).unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let kept = list(&path);
    assert_eq!(kept.len(), LEDGER_BOOT_KEPT);
    assert_eq!(
        fs::read_dir(super::directory(&path)).unwrap().count(),
        LEDGER_BOOT_KEPT
    );
}

/// The newest snapshot that fails `quick_check` or, for the vault, its key check is
/// passed over for an older one that passes.
#[test]
fn only_a_verified_snapshot_is_offered_for_a_restore() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("wallets.db");
    let snapshots = super::directory(&path);
    fs::create_dir_all(&snapshots).unwrap();
    let wallets = |name: &str, key: Option<&[u8]>| {
        let conn = Connection::open(snapshots.join(name)).unwrap();
        conn.execute_batch("CREATE TABLE wallets (encrypted_key TEXT, nonce TEXT)")
            .unwrap();
        if let Some(key) = key {
            conn.execute(
                "INSERT INTO wallets (encrypted_key, nonce) VALUES (?1, ?2)",
                [String::from_utf8_lossy(key), "AAAA".into()],
            )
            .unwrap();
        }
    };
    wallets("20261001T000000000000Z-boot.db", None);
    wallets("20261002T000000000000Z-boot.db", Some(b"not a key"));
    fs::write(
        snapshots.join("20261003T000000000000Z-periodic.db"),
        b"damaged".repeat(512),
    )
    .unwrap();
    fs::write(snapshots.join("notes.txt"), b"not a snapshot").unwrap();

    assert_eq!(list(&path).len(), 3);
    let chosen = latest_verified(&WALLETS, &path).expect("the oldest snapshot verifies");
    assert!(chosen.path.ends_with("20261001T000000000000Z-boot.db"));

    let ledger = directory.path().join("positions.db");
    let ledger_snapshots = super::directory(&ledger);
    fs::create_dir_all(&ledger_snapshots).unwrap();
    fs::write(
        ledger_snapshots.join("20261003T000000000000Z-boot.db"),
        b"damaged".repeat(512),
    )
    .unwrap();
    assert_eq!(latest_verified(&POSITIONS, &ledger), None);
}
