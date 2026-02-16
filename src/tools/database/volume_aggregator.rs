//! Volume Aggregator database operations

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use super::schema::get_connection;
use super::types::{VaAnalyticsSummary, VaSessionRow, VaSwapRow};
use crate::tools::types::{DelayConfig, DistributionStrategy, SizingConfig, ToolStatus, WalletMode};

// =============================================================================
// VOLUME AGGREGATOR SESSION OPERATIONS
// =============================================================================

/// Insert a new VA session
pub fn insert_va_session(
    session_id: &str,
    token_mint: &str,
    target_volume_sol: f64,
    delay_config: &DelayConfig,
    sizing_config: &SizingConfig,
    strategy: &DistributionStrategy,
    wallet_mode: &WalletMode,
    wallet_addresses: Option<&[String]>,
) -> Result<i64, String> {
    let conn = get_connection()?;

    let (delay_type, delay_ms, delay_max_ms) = delay_config.to_db_values();
    let (sizing_type, amount_sol, amount_max_sol) = sizing_config.to_db_values();
    let strategy_str = strategy.to_db_value();
    let wallet_mode_str = wallet_mode.to_db_value();
    let wallet_addresses_json = wallet_addresses
        .map(|addrs| serde_json::to_string(addrs).ok())
        .flatten();

    conn.execute(
        r#"
        INSERT INTO va_sessions (
            session_id, token_mint, target_volume_sol,
            delay_type, delay_ms, delay_max_ms,
            sizing_type, amount_sol, amount_max_sol,
            strategy, wallet_mode, wallet_addresses,
            status
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            session_id,
            token_mint,
            target_volume_sol,
            delay_type,
            delay_ms,
            delay_max_ms,
            sizing_type,
            amount_sol,
            amount_max_sol,
            strategy_str,
            wallet_mode_str,
            wallet_addresses_json,
            ToolStatus::Ready.to_string(),
        ],
    )
    .map_err(|e| format!("Failed to insert VA session: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// Update VA session status
pub fn update_va_session_status(
    session_id: &str,
    status: &ToolStatus,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    let started_at = if *status == ToolStatus::Running {
        Some(now.clone())
    } else {
        None
    };

    let ended_at = if matches!(
        status,
        ToolStatus::Completed | ToolStatus::Failed | ToolStatus::Aborted
    ) {
        Some(now.clone())
    } else {
        None
    };

    conn.execute(
        r#"
        UPDATE va_sessions 
        SET status = ?1, 
            error_message = ?2,
            started_at = COALESCE(?3, started_at),
            ended_at = COALESCE(?4, ended_at),
            updated_at = ?5
        WHERE session_id = ?6
        "#,
        params![
            status.to_string(),
            error_message,
            started_at,
            ended_at,
            now,
            session_id,
        ],
    )
    .map_err(|e| format!("Failed to update VA session status: {}", e))?;

    Ok(())
}

/// Update VA session metrics
pub fn update_va_session_metrics(
    session_id: &str,
    actual_volume_sol: f64,
    successful_buys: i32,
    successful_sells: i32,
    failed_count: i32,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE va_sessions 
        SET actual_volume_sol = ?1,
            successful_buys = ?2,
            successful_sells = ?3,
            failed_count = ?4,
            updated_at = ?5
        WHERE session_id = ?6
        "#,
        params![
            actual_volume_sol,
            successful_buys,
            successful_sells,
            failed_count,
            now,
            session_id,
        ],
    )
    .map_err(|e| format!("Failed to update VA session metrics: {}", e))?;

    Ok(())
}

/// Get VA session by session_id
pub fn get_va_session(session_id: &str) -> Result<Option<VaSessionRow>, String> {
    let conn = get_connection()?;

    conn.query_row(
        r#"
        SELECT id, session_id, token_mint, target_volume_sol, actual_volume_sol,
               delay_type, delay_ms, delay_max_ms,
               sizing_type, amount_sol, amount_max_sol,
               strategy, wallet_mode, wallet_addresses,
               status, started_at, ended_at, error_message,
               successful_buys, successful_sells, failed_count,
               created_at, updated_at
        FROM va_sessions WHERE session_id = ?1
        "#,
        params![session_id],
        |row| Ok(VaSessionRow::from_row(row)),
    )
    .optional()
    .map_err(|e| format!("Failed to get VA session: {}", e))?
    .transpose()
}

/// Get recent VA sessions
pub fn get_recent_va_sessions(limit: i32) -> Result<Vec<VaSessionRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, session_id, token_mint, target_volume_sol, actual_volume_sol,
                   delay_type, delay_ms, delay_max_ms,
                   sizing_type, amount_sol, amount_max_sol,
                   strategy, wallet_mode, wallet_addresses,
                   status, started_at, ended_at, error_message,
                   successful_buys, successful_sells, failed_count,
                   created_at, updated_at
            FROM va_sessions 
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![limit], |row| Ok(VaSessionRow::from_row(row)))
        .map_err(|e| format!("Failed to query sessions: {}", e))?;

    let mut sessions = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(session)) => sessions.push(session),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(format!("Failed to read row: {}", e)),
        }
    }

    Ok(sessions)
}

/// Get VA session analytics summary
pub fn get_va_sessions_analytics() -> Result<VaAnalyticsSummary, String> {
    let conn = get_connection()?;

    conn.query_row(
        r#"
        SELECT 
            COUNT(*) as total_sessions,
            COALESCE(SUM(actual_volume_sol), 0) as total_volume_sol,
            COALESCE(AVG(
                CASE WHEN (successful_buys + successful_sells + failed_count) > 0 
                THEN CAST(successful_buys + successful_sells AS REAL) / 
                     (successful_buys + successful_sells + failed_count) * 100
                ELSE 0 END
            ), 0) as avg_success_rate,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_sessions,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_sessions,
            COUNT(CASE WHEN status = 'aborted' THEN 1 END) as aborted_sessions
        FROM va_sessions
        "#,
        [],
        |row| {
            Ok(VaAnalyticsSummary {
                total_sessions: row.get(0)?,
                total_volume_sol: row.get(1)?,
                avg_success_rate: row.get(2)?,
                completed_sessions: row.get(3)?,
                failed_sessions: row.get(4)?,
                aborted_sessions: row.get(5)?,
            })
        },
    )
    .map_err(|e| format!("Failed to get VA analytics: {}", e))
}

// =============================================================================
// VOLUME AGGREGATOR SWAP OPERATIONS
// =============================================================================

/// Insert a new VA swap
pub fn insert_va_swap(
    session_id: &str,
    tx_index: i32,
    wallet_address: &str,
    is_buy: bool,
    amount_sol: f64,
) -> Result<i64, String> {
    let conn = get_connection()?;

    conn.execute(
        r#"
        INSERT INTO va_swaps (session_id, tx_index, wallet_address, is_buy, amount_sol, status)
        VALUES (?1, ?2, ?3, ?4, ?5, 'pending')
        "#,
        params![
            session_id,
            tx_index,
            wallet_address,
            is_buy as i32,
            amount_sol
        ],
    )
    .map_err(|e| format!("Failed to insert VA swap: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// Update VA swap result
pub fn update_va_swap_result(
    id: i64,
    signature: Option<&str>,
    token_amount: Option<f64>,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE va_swaps 
        SET signature = ?1,
            token_amount = ?2,
            status = ?3,
            error_message = ?4,
            executed_at = ?5
        WHERE id = ?6
        "#,
        params![signature, token_amount, status, error_message, now, id],
    )
    .map_err(|e| format!("Failed to update VA swap: {}", e))?;

    Ok(())
}

/// Get swaps for a session
pub fn get_va_swaps(session_id: &str) -> Result<Vec<VaSwapRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, session_id, tx_index, wallet_address,
                   is_buy, amount_sol, token_amount, signature,
                   status, error_message, executed_at, created_at
            FROM va_swaps 
            WHERE session_id = ?1
            ORDER BY tx_index ASC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![session_id], |row| Ok(VaSwapRow::from_row(row)))
        .map_err(|e| format!("Failed to query swaps: {}", e))?;

    let mut swaps = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(swap)) => swaps.push(swap),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(format!("Failed to read row: {}", e)),
        }
    }

    Ok(swaps)
}
