// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The SQLite operations housekeeping runs on an open store's connection: WAL checkpoint, one-time conversion to incremental auto-vacuum, and free-page reclaim.
//!
//! Every function takes a connection checked out of the store owner's own pool, so
//! no operation here ever opens a store file by path. Which store gets which
//! operation, and when, is decided by `crate::storage`.

use rusqlite::Connection;
use std::time::{Duration, Instant};

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};

/// Pages reclaimed per `incremental_vacuum` statement (16 MB at 4 KB pages).
const VACUUM_CHUNK_PAGES: u32 = 4096;

/// Pause between chunks so concurrent writers get the lock back.
const VACUUM_CHUNK_PAUSE: Duration = Duration::from_millis(25);

/// Largest live content a full `VACUUM` may rewrite: converting a file to
/// incremental auto-vacuum, or rebuilding a mostly-free file in place of the
/// chunked path. Rewriting this much finishes well inside the 5 s `busy_timeout`,
/// so writers wait rather than fail.
pub const REBUILD_MAX_LIVE_BYTES: u64 = 64 * 1024 * 1024;

/// `PRAGMA auto_vacuum` value of a file in incremental mode.
const AUTO_VACUUM_INCREMENTAL: i64 = 2;

fn query_i64(conn: &Connection, pragma: &str) -> Result<i64, DatabaseError> {
    conn.pragma_query_value(None, pragma, |row| row.get(0))
        .map_err(|e| DatabaseError::Query {
            operation: format!("query {pragma}"),
            message: e.to_string(),
        })
}

/// Checkpoint every WAL frame into the store and reset the WAL file to zero bytes.
/// A store not in WAL mode is left alone.
pub fn checkpoint(store: &str, conn: &Connection) -> Result<(), DatabaseError> {
    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|e| DatabaseError::Query {
            operation: format!("query journal mode of {store}"),
            message: e.to_string(),
        })?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Ok(());
    }

    let start = Instant::now();
    conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")
        .map_err(|e| DatabaseError::Query {
            operation: format!("checkpoint the WAL of {store}"),
            message: e.to_string(),
        })?;
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 100 {
        logger::info(
            LogTag::System,
            &format!(
                "WAL checkpoint of {store} took {:.2}s",
                elapsed.as_secs_f64()
            ),
        );
    }
    Ok(())
}

/// Convert a store to incremental auto-vacuum with one full `VACUUM`, which rewrites
/// the whole file. Returns whether a conversion ran; a store already in incremental
/// mode is untouched.
pub fn convert_to_incremental(store: &str, conn: &Connection) -> Result<bool, DatabaseError> {
    let mode = query_i64(conn, "auto_vacuum")?;
    if mode == AUTO_VACUUM_INCREMENTAL {
        return Ok(false);
    }

    let start = Instant::now();
    conn.pragma_update(None, "auto_vacuum", AUTO_VACUUM_INCREMENTAL)
        .map_err(|e| DatabaseError::Query {
            operation: format!("set incremental auto-vacuum on {store}"),
            message: e.to_string(),
        })?;
    conn.execute_batch("VACUUM;")
        .map_err(|e| DatabaseError::Query {
            operation: format!("vacuum {store} into incremental auto-vacuum"),
            message: e.to_string(),
        })?;
    truncate_wal_after_rewrite(store, conn);

    logger::info(
        LogTag::System,
        &format!(
            "Converted {store} from auto_vacuum={mode} to incremental in {:.2}s",
            start.elapsed().as_secs_f64()
        ),
    );
    Ok(true)
}

/// Reclaim a store's free pages and return how many left the file.
///
/// The whole freelist is reclaimed in `VACUUM_CHUNK_PAGES` steps with a short pause
/// between them, so writers are never locked out for long and a store that shed
/// gigabytes of rows shrinks in one pass. A store that is mostly free pages around a
/// small live core is rebuilt with one `VACUUM` instead: relocating pages chunk by
/// chunk around a tiny core runs for hours.
pub fn reclaim(store: &str, conn: &Connection) -> Result<u64, DatabaseError> {
    let freelist_before = u64::try_from(query_i64(conn, "freelist_count")?).unwrap_or(0);
    if freelist_before == 0 {
        return Ok(0);
    }
    let page_count = u64::try_from(query_i64(conn, "page_count")?).unwrap_or(0);
    let page_size = u64::try_from(query_i64(conn, "page_size")?).unwrap_or(0);
    let live_bytes = page_count.saturating_sub(freelist_before) * page_size;
    if freelist_before * 2 >= page_count && live_bytes <= REBUILD_MAX_LIVE_BYTES {
        return rebuild(store, conn, page_count, page_size);
    }

    let start = Instant::now();
    let sql = format!("PRAGMA incremental_vacuum({VACUUM_CHUNK_PAGES});");
    let mut freelist_after = freelist_before;
    while freelist_after > 0 {
        conn.execute_batch(&sql).map_err(|e| DatabaseError::Query {
            operation: format!("incrementally vacuum {store}"),
            message: e.to_string(),
        })?;
        let remaining = u64::try_from(query_i64(conn, "freelist_count")?).unwrap_or(0);
        // A step that frees nothing means the rest is not reclaimable right now (a
        // reader holds the tail); the next compaction picks it up.
        if remaining >= freelist_after {
            break;
        }
        freelist_after = remaining;
        std::thread::sleep(VACUUM_CHUNK_PAUSE);
    }

    let freed = freelist_before.saturating_sub(freelist_after);
    if freed > 0 {
        logger::info(
            LogTag::System,
            &format!(
                "Incremental vacuum of {store} freed {freed} pages ({:.2} MB) in {:.2}s",
                (freed * page_size) as f64 / (1024.0 * 1024.0),
                start.elapsed().as_secs_f64()
            ),
        );
    }
    Ok(freed)
}

/// Rewrite a mostly-free store with a full `VACUUM`, then truncate the WAL so the
/// reclaimed space leaves the disk. Returns the pages freed.
fn rebuild(
    store: &str,
    conn: &Connection,
    page_count_before: u64,
    page_size: u64,
) -> Result<u64, DatabaseError> {
    let start = Instant::now();
    conn.execute_batch("VACUUM;")
        .map_err(|e| DatabaseError::Query {
            operation: format!("rebuild {store} with VACUUM"),
            message: e.to_string(),
        })?;
    truncate_wal_after_rewrite(store, conn);
    let page_count_after = u64::try_from(query_i64(conn, "page_count")?).unwrap_or(0);

    let freed = page_count_before.saturating_sub(page_count_after);
    logger::info(
        LogTag::System,
        &format!(
            "Rebuilt {store} with VACUUM: freed {freed} pages ({:.2} MB) in {:.2}s",
            (freed * page_size) as f64 / (1024.0 * 1024.0),
            start.elapsed().as_secs_f64()
        ),
    );
    Ok(freed)
}

/// In WAL mode a rewritten image sits in the WAL until a checkpoint; a busy reader
/// only defers it to the next housekeeping pass.
fn truncate_wal_after_rewrite(store: &str, conn: &Connection) {
    if let Err(e) = conn.pragma_update(None, "wal_checkpoint", "TRUNCATE") {
        logger::warning(
            LogTag::System,
            &format!("WAL checkpoint after rewriting {store} deferred: {e}"),
        );
    }
}
