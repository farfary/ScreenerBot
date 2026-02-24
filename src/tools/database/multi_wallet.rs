//! Multi-wallet database operations

use chrono::Utc;
use rusqlite::params;

use super::schema::get_connection;
use super::types::{MwSessionConfig, MwSessionRow, MwWalletOpRow};

// =============================================================================
// MULTI-WALLET SESSION OPERATIONS
// =============================================================================

/// Create a new multi-wallet session
pub fn create_mw_session(config: &MwSessionConfig) -> Result<String, String> {
    let conn = get_connection()?;
    let session_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        r#"
        INSERT INTO mw_sessions (
            session_id, session_type, token_mint,
            total_wallets, target_amount_sol, min_amount_sol, max_amount_sol,
            delay_ms, delay_max_ms, concurrency, sol_buffer,
            status
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending')
        "#,
        params![
            session_id,
            config.session_type,
            config.token_mint,
            config.total_wallets,
            config.target_amount_sol,
            config.min_amount_sol,
            config.max_amount_sol,
            config.delay_ms,
            config.delay_max_ms,
            config.concurrency,
            config.sol_buffer,
        ],
    )
    .map_err(|e| format!("Failed to create MW session: {}", e))?;

    Ok(session_id)
}

/// Get a multi-wallet session by session_id
pub fn get_mw_session(session_id: &str) -> Result<MwSessionRow, String> {
    let conn = get_connection()?;

    conn.query_row(
        r#"
        SELECT id, session_id, session_type, token_mint,
               total_wallets, target_amount_sol, min_amount_sol, max_amount_sol,
               delay_ms, delay_max_ms, concurrency, sol_buffer,
               status, started_at, ended_at, error_message,
               wallets_funded, successful_ops, failed_ops,
               total_sol_spent, total_sol_recovered,
               created_at, updated_at
        FROM mw_sessions WHERE session_id = ?1
        "#,
        params![session_id],
        |row| {
            MwSessionRow::from_row(row).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })
        },
    )
    .map_err(|e| format!("Failed to get MW session: {}", e))
}

/// Update multi-wallet session status
pub fn update_mw_session_status(
    session_id: &str,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    let started_at = if status == "running" {
        Some(now.clone())
    } else {
        None
    };

    let ended_at = if matches!(status, "completed" | "failed" | "aborted") {
        Some(now.clone())
    } else {
        None
    };

    conn.execute(
        r#"
        UPDATE mw_sessions 
        SET status = ?1, 
            error_message = ?2,
            started_at = COALESCE(?3, started_at),
            ended_at = COALESCE(?4, ended_at),
            updated_at = ?5
        WHERE session_id = ?6
        "#,
        params![status, error_message, started_at, ended_at, now, session_id,],
    )
    .map_err(|e| format!("Failed to update MW session status: {}", e))?;

    Ok(())
}

/// Update multi-wallet session metrics
pub fn update_mw_session_metrics(
    session_id: &str,
    wallets_funded: Option<i32>,
    successful_ops: Option<i32>,
    failed_ops: Option<i32>,
    total_sol_spent: Option<f64>,
    total_sol_recovered: Option<f64>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE mw_sessions 
        SET wallets_funded = COALESCE(?1, wallets_funded),
            successful_ops = COALESCE(?2, successful_ops),
            failed_ops = COALESCE(?3, failed_ops),
            total_sol_spent = COALESCE(?4, total_sol_spent),
            total_sol_recovered = COALESCE(?5, total_sol_recovered),
            updated_at = ?6
        WHERE session_id = ?7
        "#,
        params![
            wallets_funded,
            successful_ops,
            failed_ops,
            total_sol_spent,
            total_sol_recovered,
            now,
            session_id,
        ],
    )
    .map_err(|e| format!("Failed to update MW session metrics: {}", e))?;

    Ok(())
}

/// Add a wallet operation to a session
pub fn add_wallet_op(
    session_id: &str,
    wallet_id: i32,
    wallet_address: &str,
    op_index: i32,
    op_type: &str,
    amount_sol: Option<f64>,
    token_amount: Option<f64>,
) -> Result<i64, String> {
    let conn = get_connection()?;

    conn.execute(
        r#"
        INSERT INTO mw_wallet_ops (
            session_id, wallet_id, wallet_address, op_index,
            op_type, amount_sol, token_amount, status
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending')
        "#,
        params![
            session_id,
            wallet_id,
            wallet_address,
            op_index,
            op_type,
            amount_sol,
            token_amount,
        ],
    )
    .map_err(|e| format!("Failed to add wallet op: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// Update wallet operation status
pub fn update_wallet_op_status(
    op_id: i64,
    status: &str,
    signature: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE mw_wallet_ops 
        SET status = ?1,
            signature = COALESCE(?2, signature),
            error_message = ?3,
            executed_at = ?4
        WHERE id = ?5
        "#,
        params![status, signature, error_message, now, op_id],
    )
    .map_err(|e| format!("Failed to update wallet op status: {}", e))?;

    Ok(())
}

/// Get all operations for a session
pub fn get_session_ops(session_id: &str) -> Result<Vec<MwWalletOpRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, session_id, wallet_id, wallet_address, op_index,
                   op_type, amount_sol, token_amount, signature,
                   status, error_message, executed_at, created_at
            FROM mw_wallet_ops 
            WHERE session_id = ?1
            ORDER BY op_index ASC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![session_id], |row| {
            MwWalletOpRow::from_row(row).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })
        })
        .map_err(|e| format!("Failed to query session ops: {}", e))?;

    let mut ops = Vec::new();
    for row in rows {
        match row {
            Ok(op) => ops.push(op),
            Err(e) => return Err(format!("Failed to read row: {e}")),
        }
    }

    Ok(ops)
}

/// Get recent multi-wallet sessions
pub fn get_recent_mw_sessions(limit: i32) -> Result<Vec<MwSessionRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, session_id, session_type, token_mint,
                   total_wallets, target_amount_sol, min_amount_sol, max_amount_sol,
                   delay_ms, delay_max_ms, concurrency, sol_buffer,
                   status, started_at, ended_at, error_message,
                   wallets_funded, successful_ops, failed_ops,
                   total_sol_spent, total_sol_recovered,
                   created_at, updated_at
            FROM mw_sessions 
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![limit], |row| {
            MwSessionRow::from_row(row).map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            })
        })
        .map_err(|e| format!("Failed to query sessions: {}", e))?;

    let mut sessions = Vec::new();
    for row in rows {
        match row {
            Ok(session) => sessions.push(session),
            Err(e) => return Err(format!("Failed to read row: {e}")),
        }
    }

    Ok(sessions)
}
