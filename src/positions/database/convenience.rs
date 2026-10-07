// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position database convenience functions — simplified wrappers for common queries.

use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::positions::types::{EntryRecord, ExitRecord, Position, PositionManagement};
use crate::positions::{Error, Result};

use super::booking::{query_other_open_held, Booking, BookingReads, Committed};
use super::global::GLOBAL_POSITIONS_DB;
use super::queries::{query_trader_swap_legs, TraderSwapLeg};
use super::types::{DailyTradingStats, PeriodTradingStats, TokenSnapshot};

// =============================================================================
// HELPER FUNCTIONS FOR POSITIONS MANAGEMENT
// =============================================================================

/// What the open positions of `mint` other than `excluded` hold, as storage has them; see
/// [`query_other_open_held`].
pub(crate) async fn get_other_open_held(
    mint: &str,
    excluded: Option<i64>,
) -> Result<crate::chains::RawAmount> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard.as_ref().ok_or(Error::NotInitialised)?;
    let conn = db.get_connection()?;
    let wallet_address =
        crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;
    query_other_open_held(&conn, db.chain.as_str(), &wallet_address, mint, excluded)
}

/// Load all positions from database
pub async fn load_all_positions() -> Result<Vec<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_positions(None, None).await,
        None => Err(Error::NotInitialised),
    }
}

/// Insert a new position row and return its id. An existing row changes only through a
/// booking ([`commit_booking`]), so a position that already has an id is refused.
pub async fn save_position(position: &Position) -> Result<i64> {
    if let Some(position_id) = position.id {
        return Err(Error::AlreadyStored { position_id });
    }
    logger::debug(
        LogTag::Positions,
        &format!(
            "Saving position for mint {} with entry price {:.6} SOL",
            position.mint, position.entry_price
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
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
        None => Err(Error::NotInitialised),
    }
}

/// Delete position by ID
pub async fn delete_position_by_id(id: i64) -> Result<bool> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.delete_position(id).await,
        None => Err(Error::NotInitialised),
    }
}

/// Archive or unarchive a position by ID (reversible flag; no data is deleted)
pub async fn set_position_archived_db(id: i64, archived: bool) -> Result<bool> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_position_archived(id, archived).await,
        None => Err(Error::NotInitialised),
    }
}

/// Change action ownership for a position by ID.
pub async fn set_position_management_db(id: i64, management: PositionManagement) -> Result<bool> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_position_management(id, management).await,
        None => Err(Error::NotInitialised),
    }
}

/// Hard-delete all archived positions (cascades only to this position's child rows)
pub async fn delete_archived_positions() -> Result<usize> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.delete_archived_positions().await,
        None => Err(Error::NotInitialised),
    }
}

/// Update only the price-related fields for a position using the latest in-memory state
pub async fn update_position_price_fields(position: &Position) -> Result<()> {
    let position_id = position.id.ok_or_else(|| Error::TransitionFailed {
        transition: "update_price_fields",
        mint: position.mint.clone(),
        detail: "position has no id".to_owned(),
    })?;

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
        None => Err(Error::NotInitialised),
    }
}

/// Update the price fields and the unrealized P&L of a position from the latest in-memory
/// state. No booking column is written.
pub async fn update_position_price_and_pnl_fields(position: &Position) -> Result<()> {
    let position_id = position.id.ok_or_else(|| Error::TransitionFailed {
        transition: "update_price_and_pnl_fields",
        mint: position.mint.clone(),
        detail: "position has no id".to_owned(),
    })?;

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            db.update_position_prices_and_pnl(
                position_id,
                position.current_price,
                position.current_price_updated,
                position.price_highest,
                position.price_lowest,
                position.unrealized_pnl,
                position.unrealized_pnl_percent,
            )
            .await
        }
        None => Err(Error::NotInitialised),
    }
}

/// Record a submitted full-exit swap on its position row. See
/// [`PositionsDatabase::record_exit_submission`].
pub async fn record_exit_submission(
    position_id: i64,
    exit_signature: &str,
    exit_price: f64,
    closed_reason: &str,
) -> Result<()> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            db.record_exit_submission(position_id, exit_signature, exit_price, closed_reason)
                .await
        }
        None => Err(Error::NotInitialised),
    }
}

/// Force database synchronization after critical updates
pub async fn force_database_sync() -> Result<()> {
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
        None => Err(Error::NotInitialised),
    }
}

/// Store a key-value metadata pair in the positions database
pub async fn set_metadata(key: &str, value: &str) -> Result<()> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_metadata_value(key, value),
        None => Err(Error::NotInitialised),
    }
}

/// Retrieve a metadata value by key from the positions database
pub async fn get_metadata(key: &str) -> Result<Option<String>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_metadata_value(key),
        None => Err(Error::NotInitialised),
    }
}

/// The chain whose positions the store holds
pub async fn get_store_chain() -> Result<crate::chains::ChainId> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => Ok(db.chain()),
        None => Err(Error::NotInitialised),
    }
}

/// Get open positions from database
pub async fn get_open_positions() -> Result<Vec<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_open_positions().await,
        None => Err(Error::NotInitialised),
    }
}

/// Get closed positions from database
pub async fn get_closed_positions() -> Result<Vec<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_closed_positions().await,
        None => Err(Error::NotInitialised),
    }
}

/// Get closed positions since a specific date
pub async fn get_closed_positions_since(since: DateTime<Utc>) -> Result<Vec<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_closed_positions_since(since).await,
        None => Err(Error::NotInitialised),
    }
}

/// Count closed positions since the provided UTC timestamp
pub async fn get_closed_positions_count_since(since: DateTime<Utc>) -> Result<i64> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.count_closed_positions_since(since).await,
        None => Err(Error::NotInitialised),
    }
}

/// Get aggregated trading statistics for a time period (OPTIMIZED)
pub async fn get_period_trading_stats(
    period_start: DateTime<Utc>,
    period_end: Option<DateTime<Utc>>,
) -> Result<PeriodTradingStats> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_period_trading_stats(period_start, period_end).await,
        None => Err(Error::NotInitialised),
    }
}

/// Get realized trading statistics grouped by calendar day for a period
pub async fn get_daily_trading_stats(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<DailyTradingStats>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_daily_trading_stats(period_start, period_end).await,
        None => Err(Error::NotInitialised),
    }
}

/// The most recent position for a mint from the database, OPEN OR CLOSED.
pub async fn get_latest_position_by_mint(mint: &str) -> Result<Option<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_latest_position_by_mint(mint).await,
        None => Err(Error::NotInitialised),
    }
}

/// Get position by ID from database
pub async fn get_position_by_id(id: i64) -> Result<Option<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_position_by_id(id).await,
        None => Err(Error::NotInitialised),
    }
}

/// Save token snapshot to database
pub async fn save_token_snapshot(snapshot: &TokenSnapshot) -> Result<i64> {
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
        None => Err(Error::NotInitialised),
    }
}

/// Get token snapshots for a position
pub async fn get_token_snapshots(position_id: i64) -> Result<Vec<TokenSnapshot>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_snapshots(position_id).await,
        None => Err(Error::NotInitialised),
    }
}

/// Get specific token snapshot by type
pub async fn get_token_snapshot(
    position_id: i64,
    snapshot_type: &str,
) -> Result<Option<TokenSnapshot>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_snapshot(position_id, snapshot_type).await,
        None => Err(Error::NotInitialised),
    }
}

/// Get recent closed positions for a specific mint
pub async fn get_recent_closed_positions_for_mint(
    mint: &str,
    limit: usize,
) -> Result<Vec<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_recent_closed_positions_for_mint(mint, limit).await,
        None => Err(Error::NotInitialised),
    }
}

/// Every position ever opened on a mint (open, closed, archived), oldest first.
pub async fn get_all_positions_for_mint(mint: &str) -> Result<Vec<Position>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_all_positions_for_mint(mint).await,
        None => Err(Error::NotInitialised),
    }
}

// ==================== EXIT/ENTRY HISTORY FUNCTIONS ====================

/// Book onto the stored row of `position_id` in one transaction. See
/// [`PositionsDatabase::commit_booking`].
pub(crate) async fn commit_booking<T>(
    position_id: i64,
    book: impl FnOnce(&mut Position, &BookingReads<'_>) -> Result<Booking<T>>,
) -> Result<Committed<T>> {
    // Records and trader legs are wallet-scoped; a booking that needs neither still
    // commits without a wallet, and one that needs it reports why it is unavailable.
    let wallet_address = crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
        detail: e.to_string(),
    });

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.commit_booking(
            position_id,
            wallet_address.as_deref().map_err(Clone::clone),
            book,
        ),
        None => Err(Error::NotInitialised),
    }
}

/// Get exit history for a position
/// Exits in CHRONOLOGICAL order, matching `get_entry_history`. Callers number
/// them by index ("Exit 1", "Exit 2"), so a DESC order silently labelled the
/// newest partial exit as the first one on the position chart.
pub async fn get_exit_history(position_id: i64) -> Result<Vec<ExitRecord>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard.as_ref().ok_or(Error::NotInitialised)?;

    let conn = db.pool.get().map_err(|e| DatabaseError::Connection {
        message: e.to_string(),
    })?;

    let wallet_address =
        crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;

    let mut stmt = conn
        .prepare(
            "SELECT id, position_id, timestamp, amount, price, native_received, 
       transaction_signature, is_partial, percentage, fees_raw 
       FROM position_exits WHERE position_id = ?1 AND wallet_address = ?2 ORDER BY timestamp ASC",
        )
        .map_err(|e| DatabaseError::Query {
            operation: "prepare statement".to_owned(),
            message: e.to_string(),
        })?;

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
                amount: row.get("amount")?,
                price: row.get(4)?,
                native_received: row.get(5)?,
                transaction_signature: row.get(6)?,
                is_partial: row.get(7)?,
                percentage: row.get(8)?,
                fees_raw: row.get::<_, Option<i64>>(9)?.map(|f| f as u64),
            })
        })
        .map_err(|e| DatabaseError::Query {
            operation: "query exit records".to_owned(),
            message: e.to_string(),
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| DatabaseError::Query {
            operation: "collect exit records".to_owned(),
            message: e.to_string(),
        })?;

    Ok(records)
}

/// Get entry history for a position
pub async fn get_entry_history(position_id: i64) -> Result<Vec<EntryRecord>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard.as_ref().ok_or(Error::NotInitialised)?;

    let conn = db.pool.get().map_err(|e| DatabaseError::Connection {
        message: e.to_string(),
    })?;

    let wallet_address =
        crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;

    let mut stmt = conn
        .prepare(
            "SELECT id, position_id, timestamp, amount, price, native_spent, 
       transaction_signature, is_dca, fees_raw 
       FROM position_entries WHERE position_id = ?1 AND wallet_address = ?2 ORDER BY timestamp ASC",
        )
        .map_err(|e| DatabaseError::Query {
            operation: "prepare statement".to_owned(),
            message: e.to_string(),
        })?;

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
                amount: row.get("amount")?,
                price: row.get(4)?,
                native_spent: row.get(5)?,
                transaction_signature: row.get(6)?,
                is_dca: row.get(7)?,
                fees_raw: row.get::<_, Option<i64>>(8)?.map(|f| f as u64),
            })
        })
        .map_err(|e| DatabaseError::Query {
            operation: "query entry records".to_owned(),
            message: e.to_string(),
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| DatabaseError::Query {
            operation: "collect entry records".to_owned(),
            message: e.to_string(),
        })?;

    Ok(records)
}

/// Every swap leg the TRADER itself booked, for the whole wallet, in one query.
///
/// The wallet-history ledger uses it to tell the legs it already has a fee-exact number
/// for apart from the ones the user executed elsewhere, so a bot-owned position can absorb
/// an outside buy without double-counting its own.
pub async fn get_trader_swap_legs() -> Result<Vec<TraderSwapLeg>> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard.as_ref().ok_or(Error::NotInitialised)?;

    let conn = db.pool.get().map_err(|e| DatabaseError::Connection {
        message: e.to_string(),
    })?;

    let wallet_address =
        crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;

    Ok(query_trader_swap_legs(&conn, &wallet_address, None)
        .map_err(|e| DatabaseError::classify_sqlite_failure("get_trader_swap_legs", e))?)
}
