//! Watched tokens database operations

use chrono::Utc;
use rusqlite::params;

use super::schema::get_connection;
use super::types::{WatchedToken, WatchedTokenConfig};
use crate::errors::DatabaseError;
use crate::tools::Error;

// =============================================================================
// WATCHED TOKENS OPERATIONS
// =============================================================================

/// Add a new watched token
pub fn add_watched_token(config: &WatchedTokenConfig) -> Result<i64, Error> {
    let conn = get_connection()?;

    conn.execute(
        r#"
        INSERT INTO watched_tokens (
            mint, symbol, pool_address, pool_source, pool_dex, pool_pair, pool_liquidity,
            watch_type, trigger_amount_sol, action_amount_sol, slippage_bps, is_active
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1)
        "#,
        params![
            config.mint,
            config.symbol,
            config.pool_address,
            config.pool_source,
            config.pool_dex,
            config.pool_pair,
            config.pool_liquidity,
            config.watch_type,
            config.trigger_amount_sol,
            config.action_amount_sol,
            config.slippage_bps.unwrap_or(500),
        ],
    )
    .map_err(DatabaseError::from)?;

    Ok(conn.last_insert_rowid())
}

/// Get all watched tokens
pub fn get_watched_tokens() -> Result<Vec<WatchedToken>, Error> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, mint, symbol, pool_address, pool_source, pool_dex, pool_pair, pool_liquidity,
                   watch_type, trigger_amount_sol, action_amount_sol, slippage_bps, is_active,
                   last_checked_at, last_trade_signature, trades_detected, actions_triggered,
                   created_at, updated_at
            FROM watched_tokens
            ORDER BY created_at DESC
            "#,
        )
        .map_err(DatabaseError::from)?;

    let rows = stmt
        .query_map([], |row| Ok(WatchedToken::from_row(row)))
        .map_err(DatabaseError::from)?;

    let mut tokens = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(token)) => tokens.push(token),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(DatabaseError::from(e).into()),
        }
    }

    Ok(tokens)
}

/// Get active watched tokens only
pub fn get_active_watched_tokens() -> Result<Vec<WatchedToken>, Error> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, mint, symbol, pool_address, pool_source, pool_dex, pool_pair, pool_liquidity,
                   watch_type, trigger_amount_sol, action_amount_sol, slippage_bps, is_active,
                   last_checked_at, last_trade_signature, trades_detected, actions_triggered,
                   created_at, updated_at
            FROM watched_tokens
            WHERE is_active = 1
            ORDER BY created_at DESC
            "#,
        )
        .map_err(DatabaseError::from)?;

    let rows = stmt
        .query_map([], |row| Ok(WatchedToken::from_row(row)))
        .map_err(DatabaseError::from)?;

    let mut tokens = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(token)) => tokens.push(token),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(DatabaseError::from(e).into()),
        }
    }

    Ok(tokens)
}

/// Update watched token active status
pub fn update_watched_token_status(id: i64, is_active: bool) -> Result<(), Error> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE watched_tokens SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
        params![is_active as i32, now, id],
    )
    .map_err(DatabaseError::from)?;

    Ok(())
}

/// Delete a watched token by ID
pub fn delete_watched_token(id: i64) -> Result<(), Error> {
    let conn = get_connection()?;

    conn.execute("DELETE FROM watched_tokens WHERE id = ?1", params![id])
        .map_err(DatabaseError::from)?;

    Ok(())
}

/// Update watched token tracking information
pub fn update_watched_token_tracking(
    id: i64,
    last_checked_at: Option<&str>,
    last_trade_signature: Option<&str>,
    trades_detected: Option<i32>,
    actions_triggered: Option<i32>,
) -> Result<(), Error> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE watched_tokens SET
            last_checked_at = COALESCE(?1, last_checked_at),
            last_trade_signature = COALESCE(?2, last_trade_signature),
            trades_detected = COALESCE(?3, trades_detected),
            actions_triggered = COALESCE(?4, actions_triggered),
            updated_at = ?5
        WHERE id = ?6
        "#,
        params![
            last_checked_at,
            last_trade_signature,
            trades_detected,
            actions_triggered,
            now,
            id
        ],
    )
    .map_err(DatabaseError::from)?;

    Ok(())
}
