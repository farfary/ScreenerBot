// Database CRUD operations for all token data tables

use crate::logger::{log, LogTag};
use crate::tokens::api::rugcheck_types::RugcheckInfo;
use crate::tokens::storage::database::Database;
use crate::tokens::types::{DataSource, TokenMetadata};
use chrono::Utc;
use rusqlite::{params, Result as SqliteResult, Row};
use std::sync::Arc;

/// Save or update token metadata
pub fn upsert_token_metadata(
    db: &Database,
    mint: &str,
    symbol: Option<&str>,
    name: Option<&str>,
    decimals: Option<u8>,
) -> Result<(), String> {
    let now = Utc::now().timestamp();

    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;

    conn.execute(
        r#"
        INSERT INTO tokens (mint, symbol, name, decimals, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(mint) DO UPDATE SET
            symbol = COALESCE(?2, symbol),
            name = COALESCE(?3, name),
            decimals = COALESCE(?4, decimals),
            updated_at = ?6
        "#,
        params![mint, symbol, name, decimals, now, now],
    )
    .map_err(|e| format!("Failed to upsert token metadata: {}", e))?;

    log(LogTag::Tokens, "DEBUG", &format!("Upserted token metadata: mint={}", mint));

    Ok(())
}

/// Save Rugcheck info (replaces existing data for this mint)
pub fn save_rugcheck_info(db: &Database, mint: &str, info: &RugcheckInfo) -> Result<(), String> {
    let now = Utc::now().timestamp();

    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;

    conn.execute(
        r#"
        INSERT OR REPLACE INTO data_rugcheck_info (
            mint, token_type, symbol, name, decimals, supply,
            rugcheck_score, rugcheck_score_description,
            market_solscan_tags, market_top_holders_percentage,
            risks, top_holders, fetched_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            mint,
            info.token_type,
            info.token_symbol,
            info.token_name,
            info.token_decimals,
            info.token_supply,
            info.score,
            info.score.map(|s| format!("Score: {}", s)),
            None::<String>, // market_solscan_tags - not in current schema
            None::<f64>,    // market_top_holders_percentage - not in current schema
            serde_json::to_string(&info.risks).ok(),
            serde_json::to_string(&info.top_holders).ok(),
            now
        ],
    )
    .map_err(|e| format!("Failed to save Rugcheck info: {}", e))?;

    log(LogTag::Tokens, "DEBUG", &format!("Saved Rugcheck info for mint={}", mint));

    Ok(())
}

/// Get token metadata from database
pub fn get_token_metadata(db: &Database, mint: &str) -> Result<Option<TokenMetadata>, String> {
    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;

    let result = conn.query_row(
        "SELECT mint, symbol, name, decimals, created_at, updated_at FROM tokens WHERE mint = ?1",
        params![mint],
        |row| {
            Ok(TokenMetadata {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    );

    match result {
        Ok(metadata) => Ok(Some(metadata)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Failed to query token metadata: {}", e)),
    }
}

/// Get Rugcheck info for a token
pub fn get_rugcheck_info(db: &Database, mint: &str) -> Result<Option<RugcheckInfo>, String> {
    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;

    let result = conn.query_row(
        "SELECT * FROM data_rugcheck_info WHERE mint = ?1",
        params![mint],
        |row| parse_rugcheck_row(row),
    );

    match result {
        Ok(info) => Ok(Some(info)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Failed to query Rugcheck info: {}", e)),
    }
}

// ===================== BLACKLIST OPS =====================

pub fn upsert_blacklist(db: &Database, mint: &str, reason: Option<&str>) -> Result<(), String> {
    let now = Utc::now().timestamp();
    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    conn.execute(
        r#"
        INSERT INTO blacklist (mint, reason, added_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(mint) DO UPDATE SET
            reason = COALESCE(?2, reason),
            added_at = ?3
        "#,
        params![mint, reason, now],
    )
    .map_err(|e| format!("Failed to upsert blacklist: {}", e))?;
    Ok(())
}

pub fn remove_blacklist(db: &Database, mint: &str) -> Result<(), String> {
    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    conn.execute("DELETE FROM blacklist WHERE mint = ?1", params![mint])
        .map_err(|e| format!("Failed to delete from blacklist: {}", e))?;
    Ok(())
}

pub fn list_blacklist(db: &Database) -> Result<Vec<(String, Option<String>)>, String> {
    let conn = db.get_connection();
    let conn = conn
        .lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    let mut stmt = conn
        .prepare("SELECT mint, reason FROM blacklist")
        .map_err(|e| format!("Failed to prepare blacklist query: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| format!("Failed to query blacklist: {}", e))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn parse_rugcheck_row(row: &Row) -> SqliteResult<RugcheckInfo> {
    Ok(RugcheckInfo {
        mint: row.get(0)?,
        token_type: row.get(1)?,
        token_symbol: row.get(2)?,
        token_name: row.get(3)?,
        token_decimals: row.get(4)?,
        token_supply: row.get(5)?,
        score: row.get(6)?,
        score_normalised: row.get(7)?,
        // Parse JSON fields
        risks: row
            .get::<_, Option<String>>(10)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        top_holders: row
            .get::<_, Option<String>>(11)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        // Use defaults for unparsed fields
        ..Default::default()
    })
}
