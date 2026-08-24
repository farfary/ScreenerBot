//! ATA failed cache database operations

use chrono::Utc;
use rusqlite::params;

use super::schema::get_connection;
use super::types::FailedAtaRow;
use crate::errors::DatabaseError;
use crate::tools::Error;

// =============================================================================
// ATA FAILED CACHE OPERATIONS
// =============================================================================

/// Add or update failed ATA entry
pub fn upsert_failed_ata(
    ata_address: &str,
    token_mint: Option<&str>,
    wallet_address: &str,
    error: &str,
    is_permanent: bool,
) -> Result<(), Error> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO ata_failed_cache (ata_address, token_mint, wallet_address, last_error, is_permanent_failure, last_failed_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(ata_address) DO UPDATE SET
            failure_count = failure_count + 1,
            last_error = ?4,
            is_permanent_failure = ?5,
            last_failed_at = ?6
        "#,
        params![ata_address, token_mint, wallet_address, error, is_permanent as i32, now],
    )
    .map_err(DatabaseError::from)?;

    Ok(())
}

/// Check if ATA is in failed cache
pub fn is_ata_failed(ata_address: &str) -> Result<bool, Error> {
    let conn = get_connection()?;

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM ata_failed_cache WHERE ata_address = ?1",
            params![ata_address],
            |row| row.get(0),
        )
        .map_err(DatabaseError::from)?;

    Ok(count > 0)
}

/// Get all failed ATAs for a wallet
pub fn get_failed_atas_for_wallet(wallet_address: &str) -> Result<Vec<FailedAtaRow>, Error> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT ata_address, token_mint, wallet_address,
                   failure_count, last_error, first_failed_at, last_failed_at,
                   next_retry_at, is_permanent_failure
            FROM ata_failed_cache 
            WHERE wallet_address = ?1
            ORDER BY last_failed_at DESC
            "#,
        )
        .map_err(DatabaseError::from)?;

    let rows = stmt
        .query_map(params![wallet_address], |row| {
            Ok(FailedAtaRow::from_row(row))
        })
        .map_err(DatabaseError::from)?;

    let mut atas = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(ata)) => atas.push(ata),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(DatabaseError::from(e).into()),
        }
    }

    Ok(atas)
}

/// Remove ATA from failed cache
pub fn remove_failed_ata(ata_address: &str) -> Result<(), Error> {
    let conn = get_connection()?;

    conn.execute(
        "DELETE FROM ata_failed_cache WHERE ata_address = ?1",
        params![ata_address],
    )
    .map_err(DatabaseError::from)?;

    Ok(())
}

/// Clear all non-permanent failed ATAs older than specified days
pub fn cleanup_old_failed_atas(max_age_days: i32) -> Result<i32, Error> {
    let conn = get_connection()?;

    let deleted = conn
        .execute(
            r#"
            DELETE FROM ata_failed_cache 
            WHERE is_permanent_failure = 0 
              AND last_failed_at < datetime('now', '-' || ?1 || ' days')
            "#,
            params![max_age_days],
        )
        .map_err(DatabaseError::from)?;

    Ok(deleted as i32)
}
