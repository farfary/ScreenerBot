//! Centralized database maintenance for ScreenerBot.
//!
//! This module handles:
//! - Auto-vacuum mode migration (one-time conversion from NONE to INCREMENTAL)
//! - Periodic incremental vacuum to reclaim free pages
//! - Periodic WAL checkpoint to prevent unbounded WAL file growth
//! - Monitoring and logging of database health
//!
//! ## Background
//!
//! SQLite's auto_vacuum mode must be set BEFORE a database is created, or requires
//! a full VACUUM to convert. Phase A set `PRAGMA auto_vacuum = INCREMENTAL` per-connection,
//! but existing databases retained their original mode (0 = NONE). This causes free pages
//! to accumulate indefinitely (e.g., pools.db at 729 MB with 0 rows).
//!
//! WAL (Write-Ahead Logging) files grow with writes and are only reset via checkpoint.
//! Without periodic checkpoints, WAL files can grow to hundreds of MB on a 24/7 bot.
//!
//! This module performs a one-time migration then maintains all databases via
//! periodic incremental vacuum and WAL checkpoint operations.

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::logger::{self, LogTag};
use crate::paths;

// =============================================================================
// DATABASE PATH DISCOVERY
// =============================================================================

/// Returns all ScreenerBot database name+path pairs that exist on disk.
///
/// Uses `crate::paths` to resolve standard database paths and filters to only
/// paths that currently exist. This ensures maintenance operations only run
/// on active databases.
pub fn get_all_db_paths() -> Vec<(String, PathBuf)> {
    let candidates = vec![
        ("tokens.db", paths::get_tokens_db_path()),
        ("transactions.db", paths::get_transactions_db_path()),
        ("positions.db", paths::get_positions_db_path()),
        ("wallet.db", paths::get_wallet_db_path()),
        ("events.db", paths::get_events_db_path()),
        ("pools.db", paths::get_pools_db_path()),
        ("strategies.db", paths::get_strategies_db_path()),
        ("ohlcvs.db", paths::get_ohlcvs_db_path()),
        ("actions.db", paths::get_actions_db_path()),
        ("tools.db", paths::get_tools_db_path()),
        ("ai.db", paths::get_ai_db_path()),
        ("ai_chat.db", paths::get_ai_chat_db_path()),
        // rpc_stats.db uses its own path function (not in paths module)
        (
            "rpc_stats.db",
            paths::get_data_directory().join("rpc_stats.db"),
        ),
    ];

    candidates
        .into_iter()
        .filter(|(_, path)| path.exists())
        .map(|(name, path)| (name.to_string(), path))
        .collect()
}

// =============================================================================
// AUTO-VACUUM MODE MIGRATION
// =============================================================================

/// Ensures a database is in INCREMENTAL auto-vacuum mode.
///
/// SQLite's auto_vacuum mode is stored in the database file header and cannot
/// be changed after creation except via a full VACUUM. This function:
///
/// 1. Checks the current auto_vacuum mode (0=NONE, 1=FULL, 2=INCREMENTAL)
/// 2. If not INCREMENTAL: sets the pragma and runs VACUUM to convert
/// 3. If already INCREMENTAL: returns immediately
///
/// ## Arguments
///
/// * `path` - Path to the database file
///
/// ## Returns
///
/// - `Ok(true)` if conversion was performed
/// - `Ok(false)` if database was already in INCREMENTAL mode
/// - `Err(String)` if any operation failed
///
/// ## Notes
///
/// This is I/O heavy (full VACUUM) and should only run during the initial
/// maintenance cycle, not on every periodic run.
pub fn ensure_auto_vacuum_mode(path: &Path) -> Result<bool, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("Failed to open database {}: {}", path.display(), e))?;

    // Configure connection for safe operation
    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(|e| format!("Failed to set busy_timeout: {e}"))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to set journal_mode: {e}"))?;

    // Check current auto_vacuum mode
    let auto_vacuum_mode: i64 = conn
        .pragma_query_value(None, "auto_vacuum", |row| row.get(0))
        .map_err(|e| format!("Failed to query auto_vacuum: {e}"))?;

    // Mode 2 = INCREMENTAL, we're done
    if auto_vacuum_mode == 2 {
        return Ok(false);
    }

    logger::info(
        LogTag::System,
        &format!(
            "Database {} has auto_vacuum={}, converting to INCREMENTAL",
            path.display(),
            auto_vacuum_mode
        ),
    );

    // Set INCREMENTAL mode
    conn.pragma_update(None, "auto_vacuum", 2)
        .map_err(|e| format!("Failed to set auto_vacuum: {e}"))?;

    // Run full VACUUM to convert (this rewrites the entire database)
    let start = std::time::Instant::now();
    conn.execute_batch("VACUUM;")
        .map_err(|e| format!("Failed to execute VACUUM: {e}"))?;
    let elapsed = start.elapsed();

    logger::info(
        LogTag::System,
        &format!(
            "Converted {} to INCREMENTAL mode in {:.2}s",
            path.display(),
            elapsed.as_secs_f64()
        ),
    );

    Ok(true)
}

// =============================================================================
// INCREMENTAL VACUUM
// =============================================================================

/// Runs incremental vacuum on a database to reclaim free pages.
///
/// Incremental vacuum removes pages from the database freelist in batches,
/// reducing file size without requiring a full VACUUM (which can take minutes
/// for large databases and locks the entire database).
///
/// ## Arguments
///
/// * `path` - Path to the database file
/// * `pages` - Number of pages to free (0 = free all available pages)
///
/// ## Returns
///
/// - `Ok(freed)` where `freed` is the number of pages freed
/// - `Err(String)` if any operation failed
///
/// ## Notes
///
/// - Each page is 4 KB (SQLite default)
/// - Batch size controls I/O impact (500 pages = ~2 MB)
/// - Only works if auto_vacuum mode is INCREMENTAL
pub fn run_incremental_vacuum(path: &Path, pages: u32) -> Result<u64, String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("Failed to open database {}: {}", path.display(), e))?;

    // Configure connection
    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(|e| format!("Failed to set busy_timeout: {e}"))?;

    // Get freelist count before vacuum
    let freelist_before: u64 = conn
        .pragma_query_value(None, "freelist_count", |row| row.get(0))
        .map_err(|e| format!("Failed to query freelist_count: {e}"))?;

    if freelist_before == 0 {
        return Ok(0); // Nothing to free
    }

    // Run incremental vacuum
    let start = std::time::Instant::now();
    let sql = format!("PRAGMA incremental_vacuum({pages});");
    conn.execute_batch(&sql)
        .map_err(|e| format!("Failed to execute incremental_vacuum: {e}"))?;
    let elapsed = start.elapsed();

    // Get freelist count after vacuum
    let freelist_after: u64 = conn
        .pragma_query_value(None, "freelist_count", |row| row.get(0))
        .map_err(|e| format!("Failed to query freelist_count: {e}"))?;

    let freed = freelist_before.saturating_sub(freelist_after);

    if freed > 0 {
        logger::info(
            LogTag::System,
            &format!(
                "Incremental vacuum on {} freed {} pages ({:.2} MB) in {:.2}s",
                path.display(),
                freed,
                (freed as f64 * 4.0) / 1024.0,
                elapsed.as_secs_f64()
            ),
        );
    }

    Ok(freed)
}

// =============================================================================
// WAL CHECKPOINT
// =============================================================================

/// Runs a WAL checkpoint on a database to prevent unbounded WAL file growth.
///
/// Uses TRUNCATE mode which checkpoints all frames, resets the WAL file to zero
/// bytes, and ensures the WAL file doesn't grow indefinitely on long-running bots.
///
/// ## Arguments
///
/// * `path` - Path to the database file
///
/// ## Returns
///
/// - `Ok(())` on success
/// - `Err(String)` if the checkpoint failed
///
/// ## Notes
///
/// - TRUNCATE mode requires exclusive access briefly; busy_timeout handles contention
/// - This is lightweight compared to VACUUM — typically completes in milliseconds
/// - Only meaningful for databases using WAL journal mode
pub fn run_wal_checkpoint(path: &Path) -> Result<(), String> {
    let conn = Connection::open(path)
        .map_err(|e| format!("Failed to open database {}: {}", path.display(), e))?;

    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(|e| format!("Failed to set busy_timeout: {e}"))?;

    // Check journal mode — only checkpoint WAL databases
    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|e| format!("Failed to query journal_mode: {e}"))?;

    if journal_mode.to_lowercase() != "wal" {
        return Ok(()); // Not in WAL mode, nothing to do
    }

    // TRUNCATE mode: checkpoint all frames and reset WAL file to zero bytes
    let start = std::time::Instant::now();
    conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")
        .map_err(|e| format!("WAL checkpoint failed: {e}"))?;
    let elapsed = start.elapsed();

    if elapsed.as_millis() > 100 {
        logger::info(
            LogTag::System,
            &format!(
                "WAL checkpoint on {} took {:.2}s",
                path.display(),
                elapsed.as_secs_f64()
            ),
        );
    }

    Ok(())
}

// =============================================================================
// BACKGROUND MAINTENANCE TASK
// =============================================================================

/// Starts the background database maintenance task.
///
/// This spawns a tokio task that:
///
/// 1. Waits 60 seconds after startup (avoid interfering with initialization)
/// 2. One-time: runs `ensure_auto_vacuum_mode()` on all databases (migration)
/// 3. Periodic: runs WAL checkpoint (default: every 1h) and incremental vacuum
///    (default: every 6h) on all databases. Intervals are read from config.
///
/// ## Operation
///
/// - All SQLite operations run in `spawn_blocking` to avoid blocking async runtime
/// - Each database is processed independently (errors are logged but don't stop processing)
/// - Timing and results are logged for monitoring
/// - Uses `tokio::select!` to interleave two different interval timers
///
/// ## Call this from startup sequence
///
/// ```rust
/// use screenerbot::database::start_db_maintenance_task;
///
/// tokio::spawn(start_db_maintenance_task());
/// ```
pub async fn start_maintenance_task() {
    // Read config intervals (with sane minimums)
    let (vacuum_secs, wal_secs) = crate::config::with_config(|cfg| {
        let vacuum = cfg.maintenance.vacuum_interval_secs.max(3600); // min 1h
        let wal = cfg.maintenance.wal_checkpoint_interval_secs.max(300); // min 5min
        (vacuum, wal)
    });

    logger::info(
        LogTag::System,
        &format!(
            "Database maintenance task started (60s delay, vacuum every {}h, WAL checkpoint every {}m)",
            vacuum_secs / 3600,
            wal_secs / 60
        ),
    );

    // Wait for system initialization
    tokio::time::sleep(Duration::from_secs(60)).await;

    // Phase 1: One-time auto-vacuum mode migration
    logger::info(
        LogTag::System,
        "Starting one-time auto-vacuum mode migration for all databases",
    );

    let db_paths = get_all_db_paths();
    let migration_count = db_paths.len();

    for (name, path) in db_paths {
        let name_clone = name.clone();
        let path_clone = path.clone();

        match tokio::task::spawn_blocking(move || ensure_auto_vacuum_mode(&path_clone)).await {
            Ok(Ok(converted)) => {
                if converted {
                    logger::info(
                        LogTag::System,
                        &format!("✓ Migrated {name} to INCREMENTAL mode"),
                    );
                } else {
                    logger::info(
                        LogTag::System,
                        &format!("✓ {name} already in INCREMENTAL mode"),
                    );
                }
            }
            Ok(Err(e)) => {
                logger::warning(
                    LogTag::System,
                    &format!("✗ Failed to migrate {name}: {e}"),
                );
            }
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!("✗ Task panic during migration of {name_clone}: {e}"),
                );
            }
        }
    }

    logger::info(
        LogTag::System,
        &format!(
            "Auto-vacuum migration complete ({} databases processed)",
            migration_count
        ),
    );

    // Phase 2: Periodic maintenance with two timers
    let mut vacuum_interval = tokio::time::interval(Duration::from_secs(vacuum_secs));
    vacuum_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut wal_interval = tokio::time::interval(Duration::from_secs(wal_secs));
    wal_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = vacuum_interval.tick() => {
                run_vacuum_cycle().await;
            }
            _ = wal_interval.tick() => {
                run_wal_cycle().await;
            }
        }
    }
}

/// Runs incremental vacuum on all databases.
async fn run_vacuum_cycle() {
    logger::info(
        LogTag::System,
        "Running periodic incremental vacuum on all databases",
    );

    let db_paths = get_all_db_paths();
    let mut total_freed: u64 = 0;
    let mut successful = 0;

    for (name, path) in db_paths {
        let name_clone = name.clone();
        let path_clone = path.clone();

        match tokio::task::spawn_blocking(move || run_incremental_vacuum(&path_clone, 500)).await {
            Ok(Ok(freed)) => {
                total_freed += freed;
                if freed > 0 {
                    logger::info(LogTag::System, &format!("✓ {name} freed {freed} pages"));
                }
                successful += 1;
            }
            Ok(Err(e)) => {
                logger::warning(
                    LogTag::System,
                    &format!("✗ Failed to vacuum {name}: {e}"),
                );
            }
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!("✗ Task panic during vacuum of {name_clone}: {e}"),
                );
            }
        }
    }

    logger::info(
        LogTag::System,
        &format!(
            "Incremental vacuum cycle complete: {} databases processed, {} pages freed ({:.2} MB)",
            successful,
            total_freed,
            (total_freed as f64 * 4.0) / 1024.0
        ),
    );
}

/// Runs WAL checkpoint on all databases.
async fn run_wal_cycle() {
    let db_paths = get_all_db_paths();
    let mut successful = 0;
    let mut errors = 0;

    for (name, path) in db_paths {
        let name_clone = name.clone();
        let path_clone = path.clone();

        match tokio::task::spawn_blocking(move || run_wal_checkpoint(&path_clone)).await {
            Ok(Ok(())) => {
                successful += 1;
            }
            Ok(Err(e)) => {
                logger::warning(
                    LogTag::System,
                    &format!("✗ WAL checkpoint failed for {name}: {e}"),
                );
                errors += 1;
            }
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!(
                        "✗ Task panic during WAL checkpoint of {}: {}",
                        name_clone, e
                    ),
                );
                errors += 1;
            }
        }
    }

    if errors > 0 {
        logger::warning(
            LogTag::System,
            &format!("WAL checkpoint cycle: {successful} ok, {errors} errors"),
        );
    }
}
