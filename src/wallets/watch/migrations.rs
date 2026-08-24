//! Chain-scoped key rebuilds for `watch_targets` and `watch_cursors`.
//!
//! `wallets.db` was the one database left behind when chain identity landed.
//! Every other chain-scoped store — `tokens`, `transactions`, `wallet`,
//! `pools`, `ohlcvs` — rebuilt its tables onto a `chain_id`-leading composite
//! key. These two only had `chain_id` bolted on with `ALTER TABLE ADD COLUMN`,
//! which cannot change a key, so on any database written before chain identity
//! they kept `watch_targets UNIQUE (address)` and `watch_cursors PRIMARY KEY
//! (address)`.
//!
//! That is not merely "narrower than intended". SQLite resolves an upsert's
//! `ON CONFLICT (<columns>)` target against a real unique index, so
//! `set_cursor`'s `ON CONFLICT(chain_id, address)` matched nothing and every
//! cursor write failed with `ON CONFLICT clause does not match any PRIMARY KEY
//! or UNIQUE constraint` — the watch poller could never persist a resume point
//! and re-read the same signature page on every pass, forever.
//!
//! These rebuilds follow the same create/copy/drop/rename shape as
//! `transactions::database::migrations_rebuild`, gated on the live schema via
//! `PRAGMA index_list` rather than a version stamp: only the on-disk index can
//! say what an older build actually wrote.

use rusqlite::Connection;

use super::database::WatchDatabase;
use crate::wallets::Error;

impl WatchDatabase {
    /// Whether `table` already carries a unique index over exactly
    /// `(chain_id, address)` — the structural gate that makes both rebuilds
    /// idempotent. A freshly created database satisfies it from
    /// `SCHEMA_WATCH_TARGETS` / `SCHEMA_WATCH_CURSORS` and skips the work.
    pub(super) fn has_chain_scoped_key(
        conn: &Connection,
        table: &'static str,
    ) -> Result<bool, Error> {
        let mut index_stmt = conn
            .prepare(&format!("PRAGMA index_list({table})"))
            .map_err(|e| Error::SchemaInspect {
                table,
                detail: e.to_string(),
            })?;
        let indexes: Vec<(String, bool)> = index_stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)? == 1))
            })
            .map_err(|e| Error::SchemaInspect {
                table,
                detail: e.to_string(),
            })?
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| Error::SchemaInspect {
                table,
                detail: e.to_string(),
            })?;

        for (index_name, is_unique) in indexes {
            if !is_unique {
                continue;
            }
            let mut column_stmt = conn
                .prepare(&format!("PRAGMA index_info({index_name})"))
                .map_err(|e| Error::SchemaInspect {
                    table,
                    detail: e.to_string(),
                })?;
            let columns: Vec<String> = column_stmt
                .query_map([], |row| row.get::<_, Option<String>>(2))
                .map_err(|e| Error::SchemaInspect {
                    table,
                    detail: e.to_string(),
                })?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| Error::SchemaInspect {
                    table,
                    detail: e.to_string(),
                })?
                .into_iter()
                .flatten()
                .collect();
            if columns.len() == 2
                && columns.iter().any(|c| c == "chain_id")
                && columns.iter().any(|c| c == "address")
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The expression that yields a row's chain for the copy step. A database
    /// written before chain identity has no `chain_id` column at all, while one
    /// that met the previous `ALTER TABLE ADD COLUMN` repair has the column but
    /// still the wrong key — both must rebuild, so the copy reads a literal in
    /// the first case and the (default-backfilled) column in the second.
    fn chain_source_expr(conn: &Connection, table: &'static str) -> Result<&'static str, Error> {
        Ok(if Self::column_exists(conn, table, "chain_id")? {
            "COALESCE(NULLIF(chain_id, ''), 'solana')"
        } else {
            "'solana'"
        })
    }

    /// Rebuild `watch_targets` onto `UNIQUE (chain_id, address)`.
    ///
    /// Rows are copied verbatim; a legacy row predates chain identity and is by
    /// definition Solana. `id` is preserved because `WatchSource::Alert {
    /// rule_id }` stores it inside `sources`.
    pub(super) fn rebuild_watch_targets(tx: &rusqlite::Transaction) -> Result<(), Error> {
        if Self::has_chain_scoped_key(tx, "watch_targets")? {
            return Ok(());
        }

        let migration_step = |step: &str, e: rusqlite::Error| Error::Migration {
            step: step.to_owned(),
            detail: e.to_string(),
        };

        tx.execute(
            "CREATE TABLE watch_targets__chain (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL DEFAULT 'solana',
                address TEXT NOT NULL,
                label TEXT,
                sources TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE (chain_id, address)
            )",
            [],
        )
        .map_err(|e| migration_step("create watch_targets__chain", e))?;

        let chain_expr = Self::chain_source_expr(tx, "watch_targets")?;
        tx.execute(
            &format!(
                "INSERT INTO watch_targets__chain
                    (id, chain_id, address, label, sources, enabled, created_at, updated_at)
                 SELECT id, {chain_expr}, address, label, sources,
                        enabled, created_at, updated_at
                 FROM watch_targets"
            ),
            [],
        )
        .map_err(|e| migration_step("copy watch_targets rows", e))?;

        tx.execute("DROP TABLE watch_targets", [])
            .map_err(|e| migration_step("drop legacy watch_targets", e))?;
        tx.execute(
            "ALTER TABLE watch_targets__chain RENAME TO watch_targets",
            [],
        )
        .map_err(|e| migration_step("rename watch_targets__chain", e))?;

        Ok(())
    }

    /// Rebuild `watch_cursors` onto `PRIMARY KEY (chain_id, address)` — the key
    /// `set_cursor`'s upsert has always named.
    pub(super) fn rebuild_watch_cursors(tx: &rusqlite::Transaction) -> Result<(), Error> {
        if Self::has_chain_scoped_key(tx, "watch_cursors")? {
            return Ok(());
        }

        let migration_step = |step: &str, e: rusqlite::Error| Error::Migration {
            step: step.to_owned(),
            detail: e.to_string(),
        };

        tx.execute(
            "CREATE TABLE watch_cursors__chain (
                chain_id TEXT NOT NULL DEFAULT 'solana',
                address TEXT NOT NULL,
                last_signature TEXT,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (chain_id, address)
            )",
            [],
        )
        .map_err(|e| migration_step("create watch_cursors__chain", e))?;

        let chain_expr = Self::chain_source_expr(tx, "watch_cursors")?;
        tx.execute(
            &format!(
                "INSERT INTO watch_cursors__chain (chain_id, address, last_signature, updated_at)
                 SELECT {chain_expr}, address, last_signature, updated_at
                 FROM watch_cursors"
            ),
            [],
        )
        .map_err(|e| migration_step("copy watch_cursors rows", e))?;

        tx.execute("DROP TABLE watch_cursors", [])
            .map_err(|e| migration_step("drop legacy watch_cursors", e))?;
        tx.execute(
            "ALTER TABLE watch_cursors__chain RENAME TO watch_cursors",
            [],
        )
        .map_err(|e| migration_step("rename watch_cursors__chain", e))?;

        Ok(())
    }
}
