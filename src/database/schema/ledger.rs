// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The per-store schema ledger: the store version a file holds, the oldest version able to read it and the core that wrote it, and how a core judges a store by it.

use rusqlite::{Connection, OptionalExtension};

use crate::errors::DatabaseError;

use super::inspect::table_exists;
use super::types::StoreSchema;

/// The ledger's table, one row per store file.
pub const TABLE: &str = "store_schema";

/// The core version recorded as the writer of every ledger row this build writes.
pub const WRITTEN_BY: &str = env!("CARGO_PKG_VERSION");

/// Legacy positions version that stops cores without the ledger. Installed cores up to
/// v0.2.14 refuse a positions store stamped above 6, so the stamp rises to this value
/// once a ledger store takes a breaking change those cores cannot read.
pub const POSITIONS_FENCE: i64 = 7;

const CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS store_schema (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    version INTEGER NOT NULL,
    min_reader_version INTEGER NOT NULL,
    written_by TEXT NOT NULL,
    updated_at TEXT NOT NULL
)";

/// The ledger row of a store file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRow {
    pub version: i64,
    pub min_reader_version: i64,
    pub written_by: String,
}

/// What a core does with a store, judged by its ledger row against the schema the core
/// supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assessment {
    /// No ledger: a fresh file or one last written by a core without the ledger. The
    /// historical steps run and the ledger is written.
    Unrecorded,
    /// An older store version. `breaking` when a breaking change lies between it and
    /// this core's version, so the store is copied before the step runs.
    Older { from: i64, breaking: bool },
    /// The version this core writes: the step runs and the ledger is left as it is.
    Current,
    /// A newer store this core can still read: only renames are reconciled, and the
    /// ledger is never lowered.
    CompatibleNewer,
    /// A newer store this core cannot read: the open is refused before any write.
    TooNew,
}

/// The ledger row of the store `conn` opens, `None` when it has none.
pub fn read(conn: &Connection) -> Result<Option<LedgerRow>, DatabaseError> {
    let operation = "read store_schema";
    let classify = |error| DatabaseError::classify_sqlite_failure(operation, error);
    if !table_exists(conn, TABLE).map_err(classify)? {
        return Ok(None);
    }
    conn.query_row(
        "SELECT version, min_reader_version, written_by FROM store_schema WHERE id = 1",
        [],
        |row| {
            Ok(LedgerRow {
                version: row.get(0)?,
                min_reader_version: row.get(1)?,
                written_by: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(classify)
}

/// Judge a store by its ledger row against `schema`.
pub fn assess(row: Option<&LedgerRow>, schema: &StoreSchema) -> Assessment {
    let Some(row) = row else {
        return Assessment::Unrecorded;
    };
    if row.version < schema.version {
        Assessment::Older {
            from: row.version,
            breaking: row.version < schema.min_reader_version,
        }
    } else if row.version == schema.version {
        Assessment::Current
    } else if row.min_reader_version <= schema.version {
        Assessment::CompatibleNewer
    } else {
        Assessment::TooNew
    }
}

/// Record `schema` as the store's version, written by this core.
pub fn write(conn: &Connection, schema: &StoreSchema) -> Result<(), DatabaseError> {
    let classify = |error| DatabaseError::classify_sqlite_failure("write store_schema", error);
    conn.execute(CREATE_TABLE, []).map_err(classify)?;
    conn.execute(
        "INSERT INTO store_schema (id, version, min_reader_version, written_by, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
             version = excluded.version,
             min_reader_version = excluded.min_reader_version,
             written_by = excluded.written_by,
             updated_at = excluded.updated_at",
        rusqlite::params![
            schema.version,
            schema.min_reader_version,
            WRITTEN_BY,
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(classify)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::SchemaContext;

    fn step(_: &rusqlite::Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
        Ok(())
    }

    const SCHEMA: StoreSchema = StoreSchema {
        version: 5,
        min_reader_version: 4,
        baseline: 3,
        renames: None,
        step,
        legacy_stamp: None,
    };

    fn row(version: i64, min_reader_version: i64) -> LedgerRow {
        LedgerRow {
            version,
            min_reader_version,
            written_by: "9.9.9".to_owned(),
        }
    }

    #[test]
    fn a_store_is_judged_by_its_version_and_oldest_reader() {
        assert_eq!(assess(None, &SCHEMA), Assessment::Unrecorded);
        assert_eq!(
            assess(Some(&row(3, 3)), &SCHEMA),
            Assessment::Older {
                from: 3,
                breaking: true
            }
        );
        assert_eq!(
            assess(Some(&row(4, 4)), &SCHEMA),
            Assessment::Older {
                from: 4,
                breaking: false
            }
        );
        assert_eq!(assess(Some(&row(5, 4)), &SCHEMA), Assessment::Current);
        assert_eq!(
            assess(Some(&row(8, 5)), &SCHEMA),
            Assessment::CompatibleNewer
        );
        assert_eq!(assess(Some(&row(8, 6)), &SCHEMA), Assessment::TooNew);
    }

    #[test]
    fn the_ledger_round_trips_and_names_this_core() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        assert_eq!(read(&conn).expect("read empty"), None);
        write(&conn, &SCHEMA).expect("write ledger");
        write(&conn, &SCHEMA).expect("rewrite ledger");
        assert_eq!(
            read(&conn).expect("read ledger"),
            Some(LedgerRow {
                version: 5,
                min_reader_version: 4,
                written_by: WRITTEN_BY.to_owned(),
            })
        );
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM store_schema", [], |row| row.get(0))
            .expect("count rows");
        assert_eq!(rows, 1);
    }
}
