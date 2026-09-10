use super::*;

/// The pre-chain-identity shape a real `wallets.db` still has on disk.
const LEGACY_WATCH_TARGETS: &str = "CREATE TABLE watch_targets (\n\
     id INTEGER PRIMARY KEY AUTOINCREMENT,\n\
     address TEXT NOT NULL UNIQUE,\n\
     label TEXT,\n\
     sources TEXT NOT NULL,\n\
     enabled INTEGER NOT NULL DEFAULT 1,\n\
     created_at TEXT NOT NULL DEFAULT (datetime('now')),\n\
     updated_at TEXT NOT NULL DEFAULT (datetime('now')))";
const LEGACY_WATCH_CURSORS: &str = "CREATE TABLE watch_cursors (\n\
     address TEXT PRIMARY KEY,\n\
     last_signature TEXT,\n\
     updated_at TEXT NOT NULL DEFAULT (datetime('now')))";

/// The half-migrated shape a real `wallets.db` carries today: the legacy
/// key, plus the `chain_id` column the previous `ALTER TABLE ADD COLUMN`
/// repair bolted on. `ALTER TABLE` cannot change a key, so this database
/// still has `PRIMARY KEY (address)` / `UNIQUE (address)`.
const BOLTED_ON_CHAIN_COLUMN: &[&str] = &[
    "ALTER TABLE watch_targets ADD COLUMN chain_id TEXT NOT NULL DEFAULT 'solana'",
    "ALTER TABLE watch_cursors ADD COLUMN chain_id TEXT NOT NULL DEFAULT 'solana'",
];

/// Opening a `wallets.db` written before chain identity must rebuild its
/// watch tables onto the chain-scoped keys, not merely add the column.
///
/// Adding `chain_id` with `ALTER TABLE` leaves `watch_cursors` on its
/// original `PRIMARY KEY (address)`. SQLite resolves an upsert's
/// `ON CONFLICT (<columns>)` against a real unique index, so `set_cursor`'s
/// `ON CONFLICT(chain_id, address)` matched nothing and every cursor write
/// failed with `ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE
/// constraint`. The poller could never persist a resume point, so it
/// re-read the same signature page on every pass forever.
///
/// Both legacy shapes are exercised: never-migrated, and the half-migrated
/// one the additive repair produced.
#[tokio::test]
async fn a_pre_chain_identity_database_is_rebuilt_so_the_cursor_upsert_works() {
    for bolt_on_chain_column in [false, true] {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("wallets.db");
        {
            let conn = rusqlite::Connection::open(&path).expect("create legacy fixture");
            conn.execute(LEGACY_WATCH_TARGETS, [])
                .expect("create legacy watch_targets");
            conn.execute(LEGACY_WATCH_CURSORS, [])
                .expect("create legacy watch_cursors");
            conn.execute(
                "INSERT INTO watch_targets (address, label, sources) VALUES ('Addr1111', 'KOL', '[]')",
                [],
            )
            .expect("seed a legacy target");
            conn.execute(
                "INSERT INTO watch_cursors (address, last_signature) VALUES ('Addr1111', 'Sig0000')",
                [],
            )
            .expect("seed a legacy cursor");
            if bolt_on_chain_column {
                for statement in BOLTED_ON_CHAIN_COLUMN {
                    conn.execute(statement, []).expect("bolt on chain_id");
                }
            }
        }

        let db = WatchDatabase::new_with_path(&path, ChainId::Solana).unwrap_or_else(|error| {
            panic!(
                "opening a pre-chain-identity wallets.db failed: {error}. The wallet_watch \
                 service starts with this call, so the whole process exits during boot."
            )
        });

        // Rows survive the rebuild and are adopted onto the active chain.
        let targets = db.list_targets().await.expect("list rebuilt targets");
        assert_eq!(targets.len(), 1, "the legacy row must survive the rebuild");
        assert_eq!(targets[0].address, "Addr1111");
        assert_eq!(
            db.get_cursor("Addr1111").await.expect("read legacy cursor"),
            Some("Sig0000".to_owned()),
            "the legacy cursor must survive the rebuild"
        );

        // The upsert that could never run on the legacy key: it must both
        // insert a new address and update an existing one.
        db.set_cursor("Addr2222", "Sig1111")
            .await
            .expect("insert a cursor for an address that has none");
        db.set_cursor("Addr1111", "Sig2222")
            .await
            .expect("advance an existing cursor -- this is the write that always failed");
        assert_eq!(
            db.get_cursor("Addr1111")
                .await
                .expect("read advanced cursor"),
            Some("Sig2222".to_owned()),
            "advancing a cursor must overwrite, not duplicate or fail"
        );

        // Re-opening is a clean no-op: the live key is the gate.
        let reopened =
            WatchDatabase::new_with_path(&path, ChainId::Solana).expect("second open is a no-op");
        assert_eq!(reopened.list_targets().await.expect("list again").len(), 1);
        reopened
            .set_cursor("Addr1111", "Sig3333")
            .await
            .expect("cursor writes still work after a no-op reopen");
    }
}

fn temp_db() -> (WatchDatabase, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let db = WatchDatabase::new_with_path(dir.path().join("wallets.db"), ChainId::Solana)
        .expect("create watch database");
    (db, dir)
}

#[tokio::test]
async fn insert_list_and_delete_a_target_round_trips() {
    let (db, _dir) = temp_db();

    let inserted = db
        .insert_alert_target("Addr1111", Some("KOL"))
        .await
        .expect("insert target");
    assert_eq!(inserted.address, "Addr1111");
    assert_eq!(inserted.label.as_deref(), Some("KOL"));
    assert!(inserted.enabled);
    assert_eq!(
        inserted.sources,
        vec![WatchSource::Alert {
            rule_id: inserted.id.unwrap()
        }]
    );

    let listed = db.list_targets().await.expect("list targets");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].address, "Addr1111");

    db.set_enabled(inserted.id.unwrap(), false)
        .await
        .expect("disable target");
    let fetched = db
        .get_target(inserted.id.unwrap())
        .await
        .expect("get target")
        .expect("target exists");
    assert!(!fetched.enabled);

    db.delete_target(inserted.id.unwrap())
        .await
        .expect("delete target");
    assert!(db
        .get_target(inserted.id.unwrap())
        .await
        .expect("get target after delete")
        .is_none());
}

#[tokio::test]
async fn duplicate_address_is_rejected() {
    let (db, _dir) = temp_db();
    db.insert_alert_target("Dup1111", None)
        .await
        .expect("first insert");
    let second = db.insert_alert_target("Dup1111", None).await;
    assert!(second.is_err());
}

#[tokio::test]
async fn cursor_advances_and_persists() {
    let (db, _dir) = temp_db();
    assert_eq!(db.get_cursor("Addr2222").await.unwrap(), None);
    assert!(!db.has_cursor_row("Addr2222").await.unwrap());

    db.mark_cursor_initialized("Addr2222")
        .await
        .expect("mark empty baseline");
    assert!(db.has_cursor_row("Addr2222").await.unwrap());
    assert_eq!(db.get_cursor("Addr2222").await.unwrap(), None);

    db.set_cursor("Addr2222", "sig1").await.expect("set cursor");
    assert_eq!(
        db.get_cursor("Addr2222").await.unwrap(),
        Some("sig1".to_owned())
    );

    db.set_cursor("Addr2222", "sig2")
        .await
        .expect("advance cursor");
    assert_eq!(
        db.get_cursor("Addr2222").await.unwrap(),
        Some("sig2".to_owned())
    );
}

#[tokio::test]
async fn orphan_cursors_are_purged_but_own_and_target_cursors_survive() {
    let (db, _dir) = temp_db();
    db.insert_alert_target("Kept1111", None).await.unwrap();
    db.set_cursor("Kept1111", "sig").await.unwrap();
    db.set_cursor("Own1111", "sig").await.unwrap();
    // A poll that finished after its target was removed.
    db.set_cursor("Removed1111", "sig").await.unwrap();

    assert_eq!(db.purge_orphan_cursors("Own1111").await.unwrap(), 1);
    assert!(db.has_cursor_row("Kept1111").await.unwrap());
    assert!(db.has_cursor_row("Own1111").await.unwrap());
    assert!(!db.has_cursor_row("Removed1111").await.unwrap());
}

#[tokio::test]
async fn removing_one_source_preserves_the_shared_target_and_cursor() {
    let (db, _dir) = temp_db();
    let target = db
        .insert_alert_target("Shared1111", Some("shared"))
        .await
        .unwrap();
    let alert = target.sources[0];
    db.upsert_source("Shared1111", None, WatchSource::Copy { task_id: 42 })
        .await
        .unwrap();
    db.set_cursor("Shared1111", "sig").await.unwrap();

    db.remove_source("Shared1111", alert).await.unwrap();
    let remaining = db
        .get_target_by_address("Shared1111")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(remaining.sources, [WatchSource::Copy { task_id: 42 }]);
    assert_eq!(
        db.get_cursor("Shared1111").await.unwrap().as_deref(),
        Some("sig")
    );

    db.remove_source("Shared1111", WatchSource::Copy { task_id: 42 })
        .await
        .unwrap();
    assert!(db
        .get_target_by_address("Shared1111")
        .await
        .unwrap()
        .is_none());
    assert!(!db.has_cursor_row("Shared1111").await.unwrap());
}
