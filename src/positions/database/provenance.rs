//! Idempotent migration from legacy position ownership to typed provenance.

use rusqlite::{params, Connection};

use crate::positions::error::{Error, Result};

fn has_column(conn: &Connection, name: &str) -> Result<bool> {
    let mut statement =
        conn.prepare("PRAGMA table_info(positions)")
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to inspect positions schema: {e}"),
            })?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to read positions schema: {e}"),
        })?;
    for column in names {
        if column.map_err(|e| Error::SchemaMigration {
            detail: format!("failed to decode positions schema: {e}"),
        })? == name
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn migrate_position_provenance(conn: &Connection) -> Result<()> {
    let legacy_manual = has_column(conn, "manual_management")?;
    for (column, sql) in [
        (
            "origin_kind",
            "ALTER TABLE positions ADD COLUMN origin_kind TEXT NOT NULL DEFAULT 'auto'",
        ),
        (
            "origin_ref",
            "ALTER TABLE positions ADD COLUMN origin_ref TEXT",
        ),
        (
            "management",
            "ALTER TABLE positions ADD COLUMN management TEXT NOT NULL DEFAULT 'auto_trader'",
        ),
        (
            "round_key",
            "ALTER TABLE positions ADD COLUMN round_key TEXT",
        ),
        (
            "basis_complete",
            "ALTER TABLE positions ADD COLUMN basis_complete BOOLEAN NOT NULL DEFAULT 1",
        ),
        (
            "history_complete",
            "ALTER TABLE positions ADD COLUMN history_complete BOOLEAN NOT NULL DEFAULT 1",
        ),
        (
            "holding_state",
            "ALTER TABLE positions ADD COLUMN holding_state TEXT",
        ),
    ] {
        if !has_column(conn, column)? {
            conn.execute(sql, []).map_err(|e| Error::SchemaMigration {
                detail: format!("failed to add positions.{column}: {e}"),
            })?;
        }
    }

    if legacy_manual {
        conn.execute(
            "UPDATE positions SET origin_kind = CASE WHEN manual_management THEN 'manual' ELSE 'auto' END, origin_ref = NULL, management = CASE WHEN manual_management THEN 'user_only' ELSE 'auto_trader' END WHERE origin_kind = 'auto' AND origin_ref IS NULL AND management = 'auto_trader'",
            [],
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to backfill position provenance: {e}"),
        })?;
    }
    Ok(())
}

/// Child tables keyed by `position_id`, cleaned up with the row they belong to.
const POSITION_CHILD_TABLES: &[&str] = &[
    "position_states",
    "position_entries",
    "position_exits",
    "position_tracking",
    "token_snapshots",
];

/// Collapse the duplicate row pairs left by the first wallet-history import.
///
/// That import materialised every round it reduced, including the rounds the bot had
/// executed itself, so a token the trader bought ended up with TWO rows: the trader's
/// own, and an imported `external` twin of the same swap. Both showed in Positions, both
/// counted in every portfolio total, and only the twin ever closed when the token was
/// sold somewhere else — the trader's row stayed open forever.
///
/// The pair is identified by the one thing that cannot coincide: the same wallet, the
/// same mint and the same entry transaction signature. The BOT's row survives, because
/// it carries the fee-exact basis, the entry/exit records, the strategy attribution and
/// the origin every performance query filters on; the imported twin is deleted and its
/// `round_key` moves onto the survivor, which is what lets the ledger reconcile that row
/// from then on instead of importing it again.
///
/// Idempotent: once a row carries a round key it is no longer a candidate.
pub(super) fn merge_ledger_duplicates(conn: &Connection) -> Result<u64> {
    if !has_column(conn, "round_key")? {
        return Ok(0);
    }

    let pairs: Vec<(i64, i64, String)> = {
        let mut statement = conn
            .prepare(
                "SELECT legacy.id, imported.id, imported.round_key
                 FROM positions legacy
                 JOIN positions imported
                   ON imported.chain_id = legacy.chain_id
                  AND imported.wallet_address = legacy.wallet_address
                  AND imported.mint = legacy.mint
                  AND imported.entry_transaction_signature = legacy.entry_transaction_signature
                  AND imported.origin_kind = 'external'
                  AND imported.round_key IS NOT NULL
                 WHERE legacy.round_key IS NULL
                   AND legacy.origin_kind != 'external'
                   AND legacy.entry_transaction_signature IS NOT NULL
                 ORDER BY legacy.entry_time, legacy.id",
            )
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to prepare duplicate position query: {e}"),
            })?;

        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to read duplicate positions: {e}"),
            })?;

        let mut collected = Vec::new();
        for row in rows {
            collected.push(row.map_err(|e| Error::SchemaMigration {
                detail: format!("failed to decode duplicate position: {e}"),
            })?);
        }
        collected
    };

    if pairs.is_empty() {
        return Ok(0);
    }

    // One survivor per twin and one twin per survivor: two rows of the same mint that
    // share an entry signature must not both claim the same round key, which the unique
    // index would reject anyway.
    let mut claimed_legacy = std::collections::HashSet::new();
    let mut claimed_imported = std::collections::HashSet::new();
    let mut merged = 0u64;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to start duplicate position merge: {e}"),
        })?;

    for (legacy_id, imported_id, round_key) in pairs {
        if !claimed_legacy.insert(legacy_id) || !claimed_imported.insert(imported_id) {
            continue;
        }

        // Delete the twin FIRST: the round key is unique per wallet, so the survivor
        // cannot take it while the twin still holds it.
        for table in POSITION_CHILD_TABLES {
            tx.execute(
                &format!("DELETE FROM {table} WHERE position_id = ?1"),
                params![imported_id],
            )
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to delete {table} rows for position {imported_id}: {e}"),
            })?;
        }
        tx.execute("DELETE FROM positions WHERE id = ?1", params![imported_id])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to delete imported duplicate {imported_id}: {e}"),
            })?;

        tx.execute(
            "UPDATE positions SET round_key = ?1 WHERE id = ?2",
            params![round_key, legacy_id],
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to stamp round key on position {legacy_id}: {e}"),
        })?;

        merged += 1;
    }

    tx.commit().map_err(|e| Error::SchemaMigration {
        detail: format!("failed to commit duplicate position merge: {e}"),
    })?;

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enough of the positions schema to exercise the merge: the columns it joins on,
    /// the uniqueness it must respect, and one child table it has to clean up.
    fn ledger_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE positions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL DEFAULT 'solana',
                wallet_address TEXT NOT NULL,
                mint TEXT NOT NULL,
                entry_time TEXT NOT NULL,
                entry_transaction_signature TEXT,
                origin_kind TEXT NOT NULL DEFAULT 'auto',
                round_key TEXT
             );
             CREATE UNIQUE INDEX idx_positions_round_key
                ON positions(chain_id, wallet_address, round_key) WHERE round_key IS NOT NULL;
             CREATE TABLE position_states (position_id INTEGER, state TEXT);
             CREATE TABLE position_entries (position_id INTEGER);
             CREATE TABLE position_exits (position_id INTEGER);
             CREATE TABLE position_tracking (position_id INTEGER);
             CREATE TABLE token_snapshots (position_id INTEGER);",
        )
        .unwrap();
    }

    fn insert_position(
        conn: &Connection,
        wallet: &str,
        mint: &str,
        entry_time: &str,
        signature: Option<&str>,
        origin: &str,
        round_key: Option<&str>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO positions (wallet_address, mint, entry_time, entry_transaction_signature, origin_kind, round_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![wallet, mint, entry_time, signature, origin, round_key],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn surviving(conn: &Connection) -> Vec<(i64, String, Option<String>)> {
        conn.prepare("SELECT id, origin_kind, round_key FROM positions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn the_bot_row_survives_the_merge_and_takes_the_round_key() {
        let conn = Connection::open_in_memory().unwrap();
        ledger_schema(&conn);
        let bot = insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "auto",
            None,
        );
        let imported = insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "external",
            Some("buy-sig:MINT"),
        );
        conn.execute(
            "INSERT INTO position_states (position_id, state) VALUES (?1, 'open')",
            params![imported],
        )
        .unwrap();

        assert_eq!(merge_ledger_duplicates(&conn).unwrap(), 1);

        assert_eq!(
            surviving(&conn),
            vec![(bot, "auto".to_owned(), Some("buy-sig:MINT".to_owned()))],
            "the trader's row survives, carrying the round key of the twin"
        );
        let orphans: i64 = conn
            .query_row("SELECT COUNT(*) FROM position_states", [], |row| row.get(0))
            .unwrap();
        assert_eq!(orphans, 0, "the twin's child rows go with it");
    }

    #[test]
    fn merging_twice_changes_nothing() {
        let conn = Connection::open_in_memory().unwrap();
        ledger_schema(&conn);
        insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "auto",
            None,
        );
        insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "external",
            Some("buy-sig:MINT"),
        );

        assert_eq!(merge_ledger_duplicates(&conn).unwrap(), 1);
        let after_first = surviving(&conn);
        assert_eq!(merge_ledger_duplicates(&conn).unwrap(), 0);
        assert_eq!(surviving(&conn), after_first);
    }

    #[test]
    fn an_imported_round_with_no_bot_row_is_left_alone() {
        let conn = Connection::open_in_memory().unwrap();
        ledger_schema(&conn);
        insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "external",
            Some("buy-sig:MINT"),
        );
        insert_position(
            &conn,
            "W",
            "OTHER",
            "2026-01-02",
            Some("other-sig"),
            "manual",
            None,
        );

        assert_eq!(merge_ledger_duplicates(&conn).unwrap(), 0);
        assert_eq!(surviving(&conn).len(), 2);
    }

    #[test]
    fn only_the_earliest_bot_row_claims_a_twin() {
        // Two rows of the same mint stamped with the same entry signature: only one may
        // take the round key, or the unique index would reject the second.
        let conn = Connection::open_in_memory().unwrap();
        ledger_schema(&conn);
        let first = insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "auto",
            None,
        );
        let second = insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-02",
            Some("buy-sig"),
            "auto",
            None,
        );
        insert_position(
            &conn,
            "W",
            "MINT",
            "2026-01-01",
            Some("buy-sig"),
            "external",
            Some("buy-sig:MINT"),
        );

        assert_eq!(merge_ledger_duplicates(&conn).unwrap(), 1);
        assert_eq!(
            surviving(&conn),
            vec![
                (first, "auto".to_owned(), Some("buy-sig:MINT".to_owned())),
                (second, "auto".to_owned(), None),
            ]
        );
    }

    #[test]
    fn a_different_mint_is_never_treated_as_the_same_round() {
        // One signature can move two mints (a token -> token swap): the round key must
        // only ever land on the row holding that mint.
        let conn = Connection::open_in_memory().unwrap();
        ledger_schema(&conn);
        let bot = insert_position(
            &conn,
            "W",
            "SOLD",
            "2026-01-01",
            Some("swap-sig"),
            "auto",
            None,
        );
        insert_position(
            &conn,
            "W",
            "BOUGHT",
            "2026-01-01",
            Some("swap-sig"),
            "external",
            Some("swap-sig:BOUGHT"),
        );

        assert_eq!(merge_ledger_duplicates(&conn).unwrap(), 0);
        assert_eq!(surviving(&conn)[0], (bot, "auto".to_owned(), None));
    }

    #[test]
    fn legacy_backfill_is_typed_and_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE positions (id INTEGER PRIMARY KEY, manual_management BOOLEAN NOT NULL DEFAULT 0); INSERT INTO positions (id, manual_management) VALUES (1, 0), (2, 1);",
        )
        .unwrap();

        migrate_position_provenance(&conn).unwrap();
        migrate_position_provenance(&conn).unwrap();

        let values = conn
            .prepare("SELECT id, origin_kind, origin_ref, management FROM positions ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(
            values,
            vec![
                (1, "auto".to_owned(), None, "auto_trader".to_owned()),
                (2, "manual".to_owned(), None, "user_only".to_owned()),
            ]
        );
    }

    #[test]
    fn ledger_columns_are_added_once_and_get_safe_defaults() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE positions (id INTEGER PRIMARY KEY); INSERT INTO positions (id) VALUES (1);",
        )
        .unwrap();

        migrate_position_provenance(&conn).unwrap();
        migrate_position_provenance(&conn).unwrap();

        let values: (Option<String>, bool, bool, Option<String>) = conn
            .query_row(
                "SELECT round_key, basis_complete, history_complete, holding_state FROM positions WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(values, (None, true, true, None));
    }

    #[test]
    fn fresh_schema_without_legacy_flag_gets_safe_defaults() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE positions (id INTEGER PRIMARY KEY); INSERT INTO positions (id) VALUES (1);")
            .unwrap();
        migrate_position_provenance(&conn).unwrap();
        let values: (String, Option<String>, String) = conn
            .query_row(
                "SELECT origin_kind, origin_ref, management FROM positions WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(values, ("auto".to_owned(), None, "auto_trader".to_owned()));
    }
}
