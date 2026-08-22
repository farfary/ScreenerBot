//! The write-transaction entry point every SQLite writer in the bot must use.
//!
//! `Connection::transaction()` opens a DEFERRED transaction: SQLite takes no
//! lock until the first statement, so a transaction that SELECTs before it
//! writes starts as a reader and must *upgrade* to a writer on its first
//! `INSERT`/`UPDATE`/`DELETE`. In WAL mode that upgrade cannot block: another
//! connection may have committed since the read snapshot was taken, so waiting
//! would either deadlock or silently serve a stale snapshot. SQLite therefore
//! fails the upgrade with `SQLITE_BUSY` **immediately, ignoring
//! `busy_timeout`** — the 5s timeout set in `configure::configure_connection`
//! buys a read-then-write transaction exactly nothing.
//!
//! That is not a theoretical hazard: it is what made the eight `TOKEN_POOLS`
//! refresh workers spray `Failed to clear token pools: database is locked`
//! while writing perfectly ordinary snapshots.
//!
//! IMMEDIATE takes the write lock up front, before any read. Contending writers
//! then queue on `busy_timeout` like they were always meant to, and the read a
//! transaction performs is guaranteed to be from the same snapshot it writes
//! into. Every transaction in this codebase writes, so IMMEDIATE is simply the
//! correct behaviour everywhere — there is no read-only caller to penalise.
//!
//! `tests/architecture_boundaries.rs::sqlite_writers_use_immediate_transactions`
//! fails the build if a bare `.transaction()` reappears in `src/`.

use rusqlite::{Connection, Transaction, TransactionBehavior};

/// Opens SQLite write transactions that cannot fail on a lock upgrade.
pub trait WriteTransaction {
    /// Begin an IMMEDIATE transaction — the write lock is acquired now, so a
    /// later `INSERT`/`UPDATE`/`DELETE` can never fail with "database is
    /// locked" because it started life as a reader.
    fn write_tx(&mut self) -> rusqlite::Result<Transaction<'_>>;
}

impl WriteTransaction for Connection {
    fn write_tx(&mut self) -> rusqlite::Result<Transaction<'_>> {
        self.transaction_with_behavior(TransactionBehavior::Immediate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression this trait exists for: a DEFERRED transaction that reads
    /// before it writes fails to upgrade the moment another connection holds
    /// the write lock, and does so *without* honouring `busy_timeout`.
    /// `write_tx` takes the lock before the read, so the same sequence queues
    /// instead of failing.
    #[test]
    fn immediate_transaction_survives_a_concurrent_writer() {
        let dir = std::env::temp_dir().join(format!("sb_write_tx_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("upgrade.db");
        let _ = std::fs::remove_file(&path);

        let setup = Connection::open(&path).expect("open setup connection");
        setup
            .pragma_update(None, "journal_mode", "WAL")
            .expect("WAL");
        setup
            .execute(
                "CREATE TABLE t (k INTEGER PRIMARY KEY, v INTEGER NOT NULL)",
                [],
            )
            .expect("create table");
        setup
            .execute("INSERT INTO t (k, v) VALUES (1, 1)", [])
            .expect("seed row");

        let mut blocker = Connection::open(&path).expect("open blocker");
        blocker
            .pragma_update(None, "busy_timeout", 5000)
            .expect("busy_timeout");
        let mut victim = Connection::open(&path).expect("open victim");
        victim
            .pragma_update(None, "busy_timeout", 5000)
            .expect("busy_timeout");

        // A writer holds the write lock for the whole test.
        let held = blocker.write_tx().expect("blocker begins");
        held.execute("UPDATE t SET v = 2 WHERE k = 1", [])
            .expect("blocker writes");

        // DEFERRED: the SELECT succeeds, the UPDATE cannot upgrade, and
        // busy_timeout is not honoured -- this is the production failure.
        let deferred = victim.transaction().expect("deferred begins");
        let _: i64 = deferred
            .query_row("SELECT v FROM t WHERE k = 1", [], |row| row.get(0))
            .expect("deferred reads");
        let upgrade = deferred.execute("UPDATE t SET v = 3 WHERE k = 1", []);
        assert!(
            upgrade.is_err(),
            "a DEFERRED read-then-write upgrade must fail while another writer holds the lock"
        );
        drop(deferred);

        // IMMEDIATE: the same sequence queues on busy_timeout instead, so it
        // reports the lock as a timeout rather than corrupting the snapshot.
        assert!(
            victim.write_tx().is_err(),
            "IMMEDIATE must refuse to begin while another writer holds the lock, \
             rather than beginning and failing mid-transaction"
        );

        drop(held);

        // With the lock released, the write path that previously failed works.
        let tx = victim
            .write_tx()
            .expect("immediate begins once lock is free");
        let _: i64 = tx
            .query_row("SELECT v FROM t WHERE k = 1", [], |row| row.get(0))
            .expect("immediate reads");
        tx.execute("UPDATE t SET v = 4 WHERE k = 1", [])
            .expect("immediate writes after reading");
        tx.commit().expect("commit");

        let _ = std::fs::remove_file(&path);
    }
}
