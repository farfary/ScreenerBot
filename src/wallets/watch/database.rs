//! `watch_targets` and `watch_cursors` tables, in `wallets.db`.
//!
//! A row here is a wallet the user pasted in for observation (alert-only in this
//! phase). The own wallet is never a row: the service synthesizes it in-process,
//! since it is not something the user adds, removes or disables (§6.5) -- it is
//! structural, not a target. Cursors let a restart resume paging from the last
//! signature already seen instead of re-reading a page on every boot (§6.2).
//!
//! A separate `r2d2` pool from `WalletsDatabase`'s (which owns `wallets` /
//! `token_balances` in the same physical file) -- SQLite's WAL mode supports
//! multiple pools against one file, and the codebase already relies on exactly that
//! for `events.db` (`EVENTS_WRITE_DB` / `EVENTS_READ_DB`, both opened by
//! `events::database::mod`). Every method wraps its rusqlite work in
//! `spawn_blocking`, matching the async-wrapper pattern in `tokens/database/
//! async_api.rs`.

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};

use crate::errors::{DatabaseError, InternalError, IoError};
use crate::paths::get_wallets_db_path;
use crate::wallets::Error;
use crate::{chains::ChainId, database};

use super::types::{WatchSource, WatchTarget};
use crate::database::WriteTransaction;

const SCHEMA_WATCH_TARGETS: &str = r#"
CREATE TABLE IF NOT EXISTS watch_targets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL DEFAULT 'solana',
    address TEXT NOT NULL,
    label TEXT,
    sources TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (chain_id, address)
);
"#;

const SCHEMA_WATCH_CURSORS: &str = r#"
CREATE TABLE IF NOT EXISTS watch_cursors (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    address TEXT NOT NULL,
    last_signature TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, address)
);
"#;

const INDEXES: &[&str] =
    &["CREATE INDEX IF NOT EXISTS idx_watch_targets_chain_enabled ON watch_targets(chain_id, enabled);"];

/// Owns `watch_targets` and `watch_cursors`.
#[derive(Clone)]
pub struct WatchDatabase {
    pool: Pool<SqliteConnectionManager>,
    pub(super) chain: ChainId,
}

impl WatchDatabase {
    /// Create or open the database at its real location and ensure its schema exists.
    pub fn new(chain: ChainId) -> Result<Self, Error> {
        Self::open(get_wallets_db_path(), chain)
    }

    /// Create or open the database at an explicit path. Test-only: unit tests must
    /// never resolve through `get_wallets_db_path()` / `paths::get_data_directory()`,
    /// whose base directory memoises in a process-wide `LazyLock` -- the first test in
    /// the binary to touch it would pin that directory for every other test co-located
    /// in the same `cargo test --lib` run, which is exactly the hazard `TransactionDatabase::
    /// new_with_path` exists to avoid for the same reason. See `tests/common/mod.rs`'s
    /// `real_data_dir` doc for the same footgun from the integration-test side.
    #[cfg(test)]
    pub(crate) fn new_with_path<P: AsRef<std::path::Path>>(
        path: P,
        chain: ChainId,
    ) -> Result<Self, Error> {
        Self::open(path.as_ref().to_path_buf(), chain)
    }

    fn open(db_path: std::path::PathBuf, chain: ChainId) -> Result<Self, Error> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(IoError::from)?;
        }

        let manager = SqliteConnectionManager::file(&db_path)
            .with_init(|c| database::configure_connection(c, database::WALLETS_DB));
        let pool = Pool::builder()
            .max_size(3)
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(manager)
            .map_err(DatabaseError::from)?;

        let db = Self { pool, chain };
        db.initialize_sync()?;
        Ok(db)
    }

    /// Whether a live table already declares a column. `PRAGMA table_info` is the
    /// only honest answer — a schema version stamp cannot know what an older
    /// build actually wrote.
    pub(super) fn column_exists(
        conn: &rusqlite::Connection,
        table: &'static str,
        column: &str,
    ) -> Result<bool, Error> {
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|e| Error::SchemaInspect {
                table,
                detail: e.to_string(),
            })?;
        let mut columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| Error::SchemaInspect {
                table,
                detail: e.to_string(),
            })?;
        columns.try_fold(false, |found, name| {
            let name = name.map_err(|e| Error::SchemaInspect {
                table,
                detail: e.to_string(),
            })?;
            Ok(found || name == column)
        })
    }

    pub(super) fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        self.pool.get().map_err(|e| DatabaseError::from(e).into())
    }

    fn initialize_sync(&self) -> Result<(), Error> {
        let conn = self.conn()?;
        conn.execute(SCHEMA_WATCH_TARGETS, [])
            .map_err(DatabaseError::from)?;
        conn.execute(SCHEMA_WATCH_CURSORS, [])
            .map_err(DatabaseError::from)?;
        // Bring a database written before chain identity onto the chain-scoped
        // keys the queries in this module name. `CREATE TABLE IF NOT EXISTS` is
        // a no-op on an existing table, and `ALTER TABLE ADD COLUMN` cannot
        // change a key, so only a rebuild can: see `migrations`. Both steps are
        // gated on the live schema and run before anything names the column.
        let mut conn = conn;
        let tx = conn.write_tx().map_err(|e| Error::Migration {
            step: "begin".to_owned(),
            detail: e.to_string(),
        })?;
        Self::rebuild_watch_targets(&tx)?;
        Self::rebuild_watch_cursors(&tx)?;
        tx.commit().map_err(|e| Error::Migration {
            step: "commit".to_owned(),
            detail: e.to_string(),
        })?;
        for index_sql in INDEXES {
            conn.execute(index_sql, []).map_err(DatabaseError::from)?;
        }
        Ok(())
    }

    // =========================================================================
    // watch_targets
    // =========================================================================

    pub async fn list_targets(&self) -> Result<Vec<WatchTarget>, Error> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.list_targets_sync())
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn list_targets_sync(&self) -> Result<Vec<WatchTarget>, Error> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, address, label, sources, enabled, created_at, updated_at \
                 FROM watch_targets WHERE chain_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(DatabaseError::from)?;
        let rows = stmt
            .query_map(params![self.chain.as_str()], Self::row_to_target)
            .map_err(DatabaseError::from)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(rows)
    }

    pub async fn get_target(&self, id: i64) -> Result<Option<WatchTarget>, Error> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.get_target_sync(id))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    pub(super) fn get_target_sync(&self, id: i64) -> Result<Option<WatchTarget>, Error> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT id, address, label, sources, enabled, created_at, updated_at \
             FROM watch_targets WHERE chain_id = ?1 AND id = ?2",
            params![self.chain.as_str(), id],
            Self::row_to_target,
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    pub async fn get_target_by_address(&self, address: &str) -> Result<Option<WatchTarget>, Error> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || db.get_target_by_address_sync(&address))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn get_target_by_address_sync(&self, address: &str) -> Result<Option<WatchTarget>, Error> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT id, address, label, sources, enabled, created_at, updated_at \
             FROM watch_targets WHERE chain_id = ?1 AND address = ?2",
            params![self.chain.as_str(), address],
            Self::row_to_target,
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// Insert a new alert-only target. Fails if `address` is already watched
    /// (`UNIQUE` constraint). Its `sources` is `[WatchSource::Alert { rule_id }]`
    /// where `rule_id` is the row's OWN id -- there is no separate alert-rule table
    /// in this phase, so the target IS the rule, and the id has to be known before
    /// `sources` can be written, hence the insert-then-update-in-place below rather
    /// than a single `INSERT`.
    pub async fn insert_alert_target(
        &self,
        address: &str,
        label: Option<&str>,
    ) -> Result<WatchTarget, Error> {
        let db = self.clone();
        let address = address.to_owned();
        let label = label.map(|s| s.to_owned());
        tokio::task::spawn_blocking(move || db.insert_alert_target_sync(&address, label.as_deref()))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn insert_alert_target_sync(
        &self,
        address: &str,
        label: Option<&str>,
    ) -> Result<WatchTarget, Error> {
        let mut conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let tx = conn.write_tx().map_err(DatabaseError::from)?;
        tx.execute(
            "INSERT INTO watch_targets (chain_id, address, label, sources, enabled, created_at, updated_at) \
             VALUES (?1, ?2, ?3, '[]', 1, ?4, ?4)",
            params![self.chain.as_str(), address, label, now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                DatabaseError::Query {
                    operation: "insert_alert_target".to_owned(),
                    message: format!("{address} is already watched"),
                }
            } else {
                DatabaseError::from(e)
            }
        })?;

        let id = tx.last_insert_rowid();
        let sources = vec![WatchSource::Alert { rule_id: id }];
        let sources_json = serde_json::to_string(&sources).map_err(|e| {
            Error::Internal(InternalError::InvariantViolation {
                message: format!("could not serialize watch sources: {e}"),
            })
        })?;
        tx.execute(
            "UPDATE watch_targets SET sources = ?1 WHERE chain_id = ?2 AND id = ?3",
            params![sources_json, self.chain.as_str(), id],
        )
        .map_err(DatabaseError::from)?;
        tx.commit().map_err(DatabaseError::from)?;

        drop(conn);
        self.get_target_sync(id)?.ok_or_else(|| {
            Error::Internal(InternalError::InvariantViolation {
                message: format!("watch target {id} vanished immediately after insert"),
            })
        })
    }

    pub async fn set_enabled(&self, id: i64, enabled: bool) -> Result<(), Error> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.set_enabled_sync(id, enabled))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn set_enabled_sync(&self, id: i64, enabled: bool) -> Result<(), Error> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE watch_targets SET enabled = ?1, updated_at = ?2 WHERE chain_id = ?3 AND id = ?4",
                params![enabled, now, self.chain.as_str(), id],
            )
            .map_err(DatabaseError::from)?;
        if affected == 0 {
            return Err(Error::WatchTargetNotFound {
                address: format!("id={id}"),
            });
        }
        Ok(())
    }

    pub async fn delete_target(&self, id: i64) -> Result<(), Error> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.delete_target_sync(id))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn delete_target_sync(&self, id: i64) -> Result<(), Error> {
        let mut conn = self.conn()?;
        let tx = conn.write_tx().map_err(DatabaseError::from)?;
        let address: Option<String> = tx
            .query_row(
                "SELECT address FROM watch_targets WHERE chain_id = ?1 AND id = ?2",
                params![self.chain.as_str(), id],
                |row| row.get(0),
            )
            .optional()
            .map_err(DatabaseError::from)?;
        let Some(address) = address else {
            return Err(Error::WatchTargetNotFound {
                address: format!("id={id}"),
            });
        };

        tx.execute(
            "DELETE FROM watch_targets WHERE chain_id = ?1 AND id = ?2",
            params![self.chain.as_str(), id],
        )
        .map_err(DatabaseError::from)?;
        tx.execute(
            "DELETE FROM watch_cursors WHERE chain_id = ?1 AND address = ?2",
            params![self.chain.as_str(), address],
        )
        .map_err(DatabaseError::from)?;
        tx.commit().map_err(DatabaseError::from)?;
        Ok(())
    }

    fn row_to_target(row: &rusqlite::Row) -> rusqlite::Result<WatchTarget> {
        let sources_json: String = row.get(3)?;
        let sources: Vec<WatchSource> = serde_json::from_str(&sources_json).unwrap_or_default();
        let created_str: String = row.get(5)?;
        let updated_str: String = row.get(6)?;
        Ok(WatchTarget {
            id: Some(row.get(0)?),
            address: row.get(1)?,
            label: row.get(2)?,
            sources,
            enabled: row.get::<_, i64>(4)? != 0,
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    // =========================================================================
    // watch_cursors
    // =========================================================================

    /// The last signature this address's paging (baseline poll, escalated poll or
    /// gap-fill) has already advanced past. `None` before the first successful page.
    pub async fn get_cursor(&self, address: &str) -> Result<Option<String>, Error> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || db.get_cursor_sync(&address))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    /// Whether observation has been baselined for this address, even when the
    /// address had no signatures and therefore its cursor value is still NULL.
    pub async fn has_cursor_row(&self, address: &str) -> Result<bool, Error> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = db.conn()?;
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM watch_cursors WHERE chain_id = ?1 AND address = ?2)",
                params![db.chain.as_str(), address],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|e| Error::Database(DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    /// Persist that the initial observation baseline ran even if the address had no
    /// signatures. Without this marker, the first future activity after a restart
    /// would be mistaken for pre-watch history and skipped.
    pub async fn mark_cursor_initialized(&self, address: &str) -> Result<(), Error> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = db.conn()?;
            conn.execute(
                "INSERT OR IGNORE INTO watch_cursors (chain_id, address, last_signature, updated_at) \
                 VALUES (?1, ?2, NULL, ?3)",
                params![db.chain.as_str(), address, Utc::now().to_rfc3339()],
            )
            .map_err(DatabaseError::from)?;
            Ok(())
        })
        .await
        .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn get_cursor_sync(&self, address: &str) -> Result<Option<String>, Error> {
        let conn = self.conn()?;
        let cursor = conn
            .query_row(
                "SELECT last_signature FROM watch_cursors WHERE chain_id = ?1 AND address = ?2",
                params![self.chain.as_str(), address],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(DatabaseError::from)?;
        Ok(cursor.flatten())
    }

    /// When this address's cursor last advanced -- i.e. the last time a poll or
    /// gap-fill pass last established or advanced its baseline. Used as the per-target
    /// `last_activity_at` on `/api/wallets/watch/:id/status`.
    pub async fn get_cursor_updated_at(
        &self,
        address: &str,
    ) -> Result<Option<DateTime<Utc>>, Error> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || db.get_cursor_updated_at_sync(&address))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn get_cursor_updated_at_sync(&self, address: &str) -> Result<Option<DateTime<Utc>>, Error> {
        let conn = self.conn()?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM watch_cursors WHERE chain_id = ?1 AND address = ?2",
                params![self.chain.as_str(), address],
                |row| row.get(0),
            )
            .optional()
            .map_err(DatabaseError::from)?;
        Ok(raw.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        }))
    }

    /// Delete every cursor that belongs to neither the own wallet nor a persisted
    /// target. A poll already in flight when its target is removed can write its
    /// cursor after `remove_source` deleted it; this sweep, run on every target
    /// reload, is what makes that write harmless.
    pub async fn purge_orphan_cursors(&self, own_address: &str) -> Result<usize, Error> {
        let db = self.clone();
        let own_address = own_address.to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = db.conn()?;
            conn.execute(
                "DELETE FROM watch_cursors WHERE chain_id = ?1 AND address != ?2 \
                 AND address NOT IN (SELECT address FROM watch_targets WHERE chain_id = ?1)",
                params![db.chain.as_str(), own_address],
            )
            .map_err(|e| Error::Database(DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    pub async fn set_cursor(&self, address: &str, last_signature: &str) -> Result<(), Error> {
        let db = self.clone();
        let address = address.to_owned();
        let last_signature = last_signature.to_owned();
        tokio::task::spawn_blocking(move || db.set_cursor_sync(&address, &last_signature))
            .await
            .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    fn set_cursor_sync(&self, address: &str, last_signature: &str) -> Result<(), Error> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO watch_cursors (chain_id, address, last_signature, updated_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(chain_id, address) DO UPDATE SET last_signature = excluded.last_signature, updated_at = excluded.updated_at",
            params![self.chain.as_str(), address, last_signature, now],
        )
        .map_err(DatabaseError::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
