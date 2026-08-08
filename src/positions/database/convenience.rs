//! Position database convenience functions — simplified wrappers for common queries.

use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::logger::{self, LogTag};
use crate::positions::types::{EntryRecord, ExitRecord, Position, PositionManagement};

use super::global::GLOBAL_POSITIONS_DB;
use super::types::{
    DailyTradingStats, PeriodTradingStats, PositionState, PositionTracking, PositionsDatabaseStats,
    TokenSnapshot,
};

// =============================================================================
// HELPER FUNCTIONS FOR POSITIONS MANAGEMENT
// =============================================================================

/// Load all positions from database
pub async fn load_all_positions() -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_positions(None, None).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Save position to database
pub async fn save_position(position: &Position) -> Result<i64, String> {
    logger::debug(
        LogTag::Positions,
        &format!(
            "Saving position for mint {} (ID: {:?}) with entry price {:.6} SOL",
            position.mint, position.id, position.entry_price
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            if let Some(id) = position.id {
                db.update_position(position).await?;
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Updated existing position ID {} for mint {}",
                        id, position.mint
                    ),
                );
                Ok(id)
            } else {
                let new_id = db.insert_position(position).await?;
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Created new position ID {} for mint {}",
                        new_id, position.mint
                    ),
                );
                Ok(new_id)
            }
        }
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Delete position by ID
pub async fn delete_position_by_id(id: i64) -> Result<bool, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.delete_position(id).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Archive or unarchive a position by ID (reversible flag; no data is deleted)
pub async fn set_position_archived_db(id: i64, archived: bool) -> Result<bool, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_position_archived(id, archived).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Change action ownership for a position by ID.
pub async fn set_position_management_db(
    id: i64,
    management: PositionManagement,
) -> Result<bool, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_position_management(id, management).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Hard-delete all archived positions (cascades only to this position's child rows)
pub async fn delete_archived_positions() -> Result<usize, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.delete_archived_positions().await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Update position in database
pub async fn update_position(position: &Position) -> Result<(), String> {
    logger::debug(
        LogTag::Positions,
        &format!(
            "Updating position ID {:?} for mint {} with current price {:.11} SOL",
            position.id,
            position.mint,
            position.current_price.unwrap_or_default()
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = db.update_position(position).await;
            match &result {
                Ok(_) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Successfully updated position ID {:?} for mint {}",
                        position.id, position.mint
                    ),
                ),
                Err(e) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Failed to update position ID {:?} for mint {}: {}",
                        position.id, position.mint, e
                    ),
                ),
            }
            result
        }
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Update only the price-related fields for a position using the latest in-memory state
pub async fn update_position_price_fields(position: &Position) -> Result<(), String> {
    let position_id = position
        .id
        .ok_or_else(|| "Cannot update price fields without position ID".to_owned())?;

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            db.update_position_prices(
                position_id,
                position.current_price,
                position.current_price_updated,
                position.price_highest,
                position.price_lowest,
            )
            .await
        }
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Force database synchronization after critical updates
pub async fn force_database_sync() -> Result<(), String> {
    logger::debug(LogTag::Positions, "Forcing database synchronization...");

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = db.force_sync().await;
            match &result {
                Ok(_) => logger::debug(
                    LogTag::Positions,
                    "Database synchronization completed successfully",
                ),
                Err(e) => logger::debug(
                    LogTag::Positions,
                    &format!("Database synchronization failed: {e}"),
                ),
            }
            result
        }
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Store a key-value metadata pair in the positions database
pub async fn set_metadata(key: &str, value: &str) -> Result<(), String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_metadata_value(key, value),
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Retrieve a metadata value by key from the positions database
pub async fn get_metadata(key: &str) -> Result<Option<String>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_metadata_value(key),
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get open positions from database
pub async fn get_open_positions() -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_open_positions().await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get closed positions from database
pub async fn get_closed_positions() -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_closed_positions().await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get closed positions since a specific date
pub async fn get_closed_positions_since(since: DateTime<Utc>) -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_closed_positions_since(since).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Count closed positions since the provided UTC timestamp
pub async fn get_closed_positions_count_since(since: DateTime<Utc>) -> Result<i64, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.count_closed_positions_since(since).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get aggregated trading statistics for a time period (OPTIMIZED)
pub async fn get_period_trading_stats(
    period_start: DateTime<Utc>,
    period_end: Option<DateTime<Utc>>,
) -> Result<PeriodTradingStats, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_period_trading_stats(period_start, period_end).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get realized trading statistics grouped by calendar day for a period
pub async fn get_daily_trading_stats(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<DailyTradingStats>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_daily_trading_stats(period_start, period_end).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// The most recent position for a mint from the database, OPEN OR CLOSED.
pub async fn get_latest_position_by_mint(mint: &str) -> Result<Option<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_latest_position_by_mint(mint).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get position by ID from database
pub async fn get_position_by_id(id: i64) -> Result<Option<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_position_by_id(id).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Save token snapshot to database
pub async fn save_token_snapshot(snapshot: &TokenSnapshot) -> Result<i64, String> {
    logger::debug(
        LogTag::Positions,
        &format!(
            "Saving token snapshot for position ID {} (type: {}) with mint {}",
            snapshot.position_id, snapshot.snapshot_type, snapshot.mint
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = db.save_token_snapshot(snapshot).await;
            match &result {
                Ok(snapshot_id) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Successfully saved token snapshot ID {} for position ID {} (type: {})",
                        snapshot_id, snapshot.position_id, snapshot.snapshot_type
                    ),
                ),
                Err(e) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Failed to save token snapshot for position ID {} (type: {}): {}",
                        snapshot.position_id, snapshot.snapshot_type, e
                    ),
                ),
            }
            result
        }
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get token snapshots for a position
pub async fn get_token_snapshots(position_id: i64) -> Result<Vec<TokenSnapshot>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_snapshots(position_id).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get specific token snapshot by type
pub async fn get_token_snapshot(
    position_id: i64,
    snapshot_type: &str,
) -> Result<Option<TokenSnapshot>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_snapshot(position_id, snapshot_type).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Get recent closed positions for a specific mint
pub async fn get_recent_closed_positions_for_mint(
    mint: &str,
    limit: usize,
) -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_recent_closed_positions_for_mint(mint, limit).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

/// Every position ever opened on a mint (open, closed, archived), oldest first.
pub async fn get_all_positions_for_mint(mint: &str) -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_all_positions_for_mint(mint).await,
        None => Err("Positions database not initialized".to_owned()),
    }
}

// ==================== EXIT/ENTRY HISTORY FUNCTIONS ====================

/// Save an exit record to history
pub async fn save_exit_record(
    position_id: i64,
    timestamp: DateTime<Utc>,
    amount: u64,
    price: f64,
    sol_received: f64,
    transaction_signature: &str,
    is_partial: bool,
    percentage: f64,
    fees_lamports: Option<u64>,
) -> Result<(), String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

    // Idempotent per (position, signature): one on-chain swap = one exit record. The table
    // has no unique constraint, so a re-applied verification (retry, restart) would
    // otherwise insert the SAME exit twice and double it everywhere the records are summed
    // — the History tab, the chart's exit markers, the "% exited" fallback.
    conn.execute(
    "INSERT INTO position_exits (position_id, wallet_address, timestamp, amount, price, sol_received,
     transaction_signature, is_partial, percentage, fees_lamports)
     SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
     WHERE NOT EXISTS (
       SELECT 1 FROM position_exits WHERE position_id = ?1 AND transaction_signature = ?7
     )",
    params![
      position_id,
      wallet_address,
      timestamp.to_rfc3339(),
      amount as i64,
      price,
      sol_received,
      transaction_signature,
      is_partial,
      percentage,
      fees_lamports.map(|f| f as i64),
    ],
  )
  .map_err(|e| format!("Failed to save exit record: {e}"))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "Saved exit record: position={} amount={} partial={} tx={}",
            position_id, amount, is_partial, transaction_signature
        ),
    );

    Ok(())
}

/// Get exit history for a position
/// Has this exact swap already been recorded as an exit for this position?
///
/// The idempotency token for applying an exit transition. `PartialExitVerified` ACCUMULATES
/// (`sol_received +=`, `total_exited_amount +=`, `partial_exit_count += 1`), so applying it
/// twice for the same signature would double the proceeds and the tokens sold. The queue
/// dedupes by signature only while an item is IN it — once polled it is gone, so a
/// re-enqueue (startup rehydrate, a manual re-verification) can hand the same swap back.
pub async fn exit_record_exists(position_id: i64, transaction_signature: &str) -> bool {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let Some(db) = db_guard.as_ref() else {
        return false;
    };

    let Ok(conn) = db.pool.get() else {
        return false;
    };

    conn.query_row(
        "SELECT 1 FROM position_exits WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
        params![position_id, transaction_signature],
        |_| Ok(()),
    )
    .is_ok()
}

/// Exits in CHRONOLOGICAL order, matching `get_entry_history`. Callers number
/// them by index ("Exit 1", "Exit 2"), so a DESC order silently labelled the
/// newest partial exit as the first one on the position chart.
pub async fn get_exit_history(position_id: i64) -> Result<Vec<ExitRecord>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, position_id, timestamp, amount, price, sol_received, 
       transaction_signature, is_partial, percentage, fees_lamports 
       FROM position_exits WHERE position_id = ?1 AND wallet_address = ?2 ORDER BY timestamp ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let records = stmt
        .query_map(params![position_id, wallet_address], |row| {
            Ok(ExitRecord {
                id: row.get(0)?,
                position_id: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
                amount: row.get::<_, i64>(3)? as u64,
                price: row.get(4)?,
                sol_received: row.get(5)?,
                transaction_signature: row.get(6)?,
                is_partial: row.get(7)?,
                percentage: row.get(8)?,
                fees_lamports: row.get::<_, Option<i64>>(9)?.map(|f| f as u64),
            })
        })
        .map_err(|e| format!("Failed to query exit records: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect exit records: {e}"))?;

    Ok(records)
}

/// Save an entry record to history
pub async fn save_entry_record(
    position_id: i64,
    timestamp: DateTime<Utc>,
    amount: u64,
    price: f64,
    sol_spent: f64,
    transaction_signature: &str,
    is_dca: bool,
    fees_lamports: Option<u64>,
) -> Result<(), String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

    // Idempotent per (position, signature) — see save_exit_record. A DCA add re-verified
    // after a restart must not be recorded (and averaged in) twice.
    conn.execute(
    "INSERT INTO position_entries (position_id, wallet_address, timestamp, amount, price, sol_spent,
     transaction_signature, is_dca, fees_lamports)
     SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
     WHERE NOT EXISTS (
       SELECT 1 FROM position_entries WHERE position_id = ?1 AND transaction_signature = ?7
     )",
    params![
      position_id,
      wallet_address,
      timestamp.to_rfc3339(),
      amount as i64,
      price,
      sol_spent,
      transaction_signature,
      is_dca,
      fees_lamports.map(|f| f as i64),
    ],
  )
  .map_err(|e| format!("Failed to save entry record: {e}"))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "Saved entry record: position={} amount={} dca={} tx={}",
            position_id, amount, is_dca, transaction_signature
        ),
    );

    Ok(())
}

/// Get entry history for a position
pub async fn get_entry_history(position_id: i64) -> Result<Vec<EntryRecord>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {e}"))?;

    let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, position_id, timestamp, amount, price, sol_spent, 
       transaction_signature, is_dca, fees_lamports 
       FROM position_entries WHERE position_id = ?1 AND wallet_address = ?2 ORDER BY timestamp ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {e}"))?;

    let records = stmt
        .query_map(params![position_id, wallet_address], |row| {
            Ok(EntryRecord {
                id: row.get(0)?,
                position_id: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
                amount: row.get::<_, i64>(3)? as u64,
                price: row.get(4)?,
                sol_spent: row.get(5)?,
                transaction_signature: row.get(6)?,
                is_dca: row.get(7)?,
                fees_lamports: row.get::<_, Option<i64>>(8)?.map(|f| f as u64),
            })
        })
        .map_err(|e| format!("Failed to query entry records: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect entry records: {e}"))?;

    Ok(records)
}
