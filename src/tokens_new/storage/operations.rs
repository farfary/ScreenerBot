// Database CRUD operations for all token data tables

use crate::tokens_new::storage::database::Database;
use crate::tokens_new::types::{DexScreenerPool, GeckoTerminalPool, RugcheckInfo, DataSource};
use chrono::Utc;
use log::{debug, error, warn};
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
    let conn = conn.lock()
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
    ).map_err(|e| format!("Failed to upsert token metadata: {}", e))?;
    
    debug!("[TOKENS_NEW] Upserted token metadata: mint={}", mint);
    
    Ok(())
}

/// Save DexScreener pools (replaces existing data for this mint)
pub fn save_dexscreener_pools(
    db: &Database,
    mint: &str,
    pools: &[DexScreenerPool],
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    
    let conn = db.get_connection();
    let conn = conn.lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    
    // Delete existing pools for this mint
    conn.execute(
        "DELETE FROM data_dexscreener_pools WHERE mint = ?1",
        params![mint],
    ).map_err(|e| format!("Failed to delete old DexScreener pools: {}", e))?;
    
    // Insert new pools
    for pool in pools {
        conn.execute(
            r#"
            INSERT INTO data_dexscreener_pools (
                mint, chain_id, dex_id, pair_address,
                base_token_address, base_token_name, base_token_symbol,
                quote_token_address, quote_token_name, quote_token_symbol,
                price_native, price_usd, liquidity_usd, liquidity_base, liquidity_quote,
                fdv, market_cap,
                price_change_m5, price_change_h1, price_change_h6, price_change_h24,
                volume_m5, volume_h1, volume_h6, volume_h24,
                txns_m5_buys, txns_m5_sells, txns_h1_buys, txns_h1_sells,
                txns_h6_buys, txns_h6_sells, txns_h24_buys, txns_h24_sells,
                pair_created_at, labels, url,
                info_image_url, info_header, info_open_graph, info_websites, info_socials,
                boosts_active, fetched_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40,
                ?41, ?42
            )
            "#,
            params![
                mint, pool.chain_id, pool.dex_id, pool.pair_address,
                pool.base_token_address, pool.base_token_name, pool.base_token_symbol,
                pool.quote_token_address, pool.quote_token_name, pool.quote_token_symbol,
                pool.price_native, pool.price_usd, pool.liquidity_usd, pool.liquidity_base, pool.liquidity_quote,
                pool.fdv, pool.market_cap,
                pool.price_change_m5, pool.price_change_h1, pool.price_change_h6, pool.price_change_h24,
                pool.volume_m5, pool.volume_h1, pool.volume_h6, pool.volume_h24,
                pool.txns_m5_buys, pool.txns_m5_sells, pool.txns_h1_buys, pool.txns_h1_sells,
                pool.txns_h6_buys, pool.txns_h6_sells, pool.txns_h24_buys, pool.txns_h24_sells,
                pool.pair_created_at,
                serde_json::to_string(&pool.labels).ok(),
                pool.url,
                pool.info_image_url, pool.info_header, pool.info_open_graph,
                serde_json::to_string(&pool.info_websites).ok(),
                serde_json::to_string(&pool.info_socials).ok(),
                None::<i64>, // boosts_active - not in current schema
                now
            ],
        ).map_err(|e| format!("Failed to insert DexScreener pool: {}", e))?;
    }
    
    debug!("[TOKENS_NEW] Saved {} DexScreener pools for mint={}", pools.len(), mint);
    
    Ok(())
}

/// Save GeckoTerminal pools (replaces existing data for this mint)
pub fn save_geckoterminal_pools(
    db: &Database,
    mint: &str,
    pools: &[GeckoTerminalPool],
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    
    let conn = db.get_connection();
    let conn = conn.lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    
    // Delete existing pools for this mint
    conn.execute(
        "DELETE FROM data_geckoterminal_pools WHERE mint = ?1",
        params![mint],
    ).map_err(|e| format!("Failed to delete old GeckoTerminal pools: {}", e))?;
    
    // Insert new pools
    for pool in pools {
        conn.execute(
            r#"
            INSERT INTO data_geckoterminal_pools (
                mint, pool_address, pool_name, dex_id,
                base_token_id, quote_token_id,
                base_token_price_usd, base_token_price_native, base_token_price_quote,
                quote_token_price_usd, quote_token_price_native, quote_token_price_base,
                token_price_usd,
                fdv_usd, market_cap_usd, reserve_usd,
                volume_m5, volume_m15, volume_m30,
                volume_h1, volume_h6, volume_h24,
                price_change_m5, price_change_m15, price_change_m30,
                price_change_h1, price_change_h6, price_change_h24,
                txns_m5_buys, txns_m5_sells,
                txns_m15_buys, txns_m15_sells,
                txns_m30_buys, txns_m30_sells,
                txns_h1_buys, txns_h1_sells,
                txns_h6_buys, txns_h6_sells,
                txns_h24_buys, txns_h24_sells,
                pool_created_at,
                fetched_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40
            )
            "#,
            params![
                mint, pool.pool_address, pool.pool_name, pool.dex_id,
                pool.base_token_id, pool.quote_token_id,
                pool.base_token_price_usd, pool.base_token_price_native, pool.base_token_price_quote,
                pool.quote_token_price_usd, pool.quote_token_price_native, pool.quote_token_price_base,
                pool.token_price_usd,
                pool.fdv_usd, pool.market_cap_usd, pool.reserve_usd,
                pool.volume_m5, pool.volume_m15, pool.volume_m30,
                pool.volume_h1, pool.volume_h6, pool.volume_h24,
                pool.price_change_m5, pool.price_change_m15, pool.price_change_m30,
                pool.price_change_h1, pool.price_change_h6, pool.price_change_h24,
                pool.txns_m5_buys, pool.txns_m5_sells,
                pool.txns_m15_buys, pool.txns_m15_sells,
                pool.txns_m30_buys, pool.txns_m30_sells,
                pool.txns_h1_buys, pool.txns_h1_sells,
                pool.txns_h6_buys, pool.txns_h6_sells,
                pool.txns_h24_buys, pool.txns_h24_sells,
                pool.pool_created_at,
                now
            ],
        ).map_err(|e| format!("Failed to insert GeckoTerminal pool: {}", e))?;
    }
    
    debug!("[TOKENS_NEW] Saved {} GeckoTerminal pools for mint={}", pools.len(), mint);
    
    Ok(())
}

/// Save Rugcheck info (replaces existing data for this mint)
pub fn save_rugcheck_info(
    db: &Database,
    mint: &str,
    info: &RugcheckInfo,
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    
    let conn = db.get_connection();
    let conn = conn.lock()
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
            None::<f64>, // market_top_holders_percentage - not in current schema
            serde_json::to_string(&info.risks).ok(),
            serde_json::to_string(&info.top_holders).ok(),
            now
        ],
    ).map_err(|e| format!("Failed to save Rugcheck info: {}", e))?;
    
    debug!("[TOKENS_NEW] Saved Rugcheck info for mint={}", mint);
    
    Ok(())
}

/// Log an API fetch attempt
pub fn log_api_fetch(
    db: &Database,
    mint: &str,
    source: DataSource,
    success: bool,
    error_message: Option<&str>,
    records_fetched: Option<usize>,
) -> Result<(), String> {
    let now = Utc::now().timestamp();
    
    let conn = db.get_connection();
    let conn = conn.lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    
    conn.execute(
        r#"
        INSERT INTO api_fetch_log (mint, source, success, error_message, records_fetched, fetched_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            mint,
            source.as_str(),
            if success { 1 } else { 0 },
            error_message,
            records_fetched.map(|n| n as i64),
            now
        ],
    ).map_err(|e| format!("Failed to log API fetch: {}", e))?;
    
    Ok(())
}

/// Get token metadata from database
pub fn get_token_metadata(db: &Database, mint: &str) -> Result<Option<TokenMetadata>, String> {
    let conn = db.get_connection();
    let conn = conn.lock()
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

// TokenMetadata defined in query module, re-use that type
use crate::tokens_new::provider::query::TokenMetadata;

/// Get DexScreener pools for a token
pub fn get_dexscreener_pools(db: &Database, mint: &str) -> Result<Vec<DexScreenerPool>, String> {
    let conn = db.get_connection();
    let conn = conn.lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    
    let mut stmt = conn.prepare(
        "SELECT * FROM data_dexscreener_pools WHERE mint = ?1 ORDER BY liquidity_usd DESC"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let rows = stmt.query_map(params![mint], |row| {
        parse_dexscreener_row(row)
    }).map_err(|e| format!("Failed to query DexScreener pools: {}", e))?;
    
    let mut pools = Vec::new();
    for row_result in rows {
        match row_result {
            Ok(pool) => pools.push(pool),
            Err(e) => warn!("[TOKENS_NEW] Failed to parse DexScreener pool row: {}", e),
        }
    }
    
    Ok(pools)
}

/// Get GeckoTerminal pools for a token
pub fn get_geckoterminal_pools(db: &Database, mint: &str) -> Result<Vec<GeckoTerminalPool>, String> {
    let conn = db.get_connection();
    let conn = conn.lock()
        .map_err(|e| format!("Failed to lock connection: {}", e))?;
    
    let mut stmt = conn.prepare(
        "SELECT * FROM data_geckoterminal_pools WHERE mint = ?1 ORDER BY reserve_in_usd DESC"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
    let rows = stmt.query_map(params![mint], |row| {
        parse_geckoterminal_row(row)
    }).map_err(|e| format!("Failed to query GeckoTerminal pools: {}", e))?;
    
    let mut pools = Vec::new();
    for row_result in rows {
        match row_result {
            Ok(pool) => pools.push(pool),
            Err(e) => warn!("[TOKENS_NEW] Failed to parse GeckoTerminal pool row: {}", e),
        }
    }
    
    Ok(pools)
}

/// Get Rugcheck info for a token
pub fn get_rugcheck_info(db: &Database, mint: &str) -> Result<Option<RugcheckInfo>, String> {
    let conn = db.get_connection();
    let conn = conn.lock()
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

// Row parsing helpers (simplified for brevity - in production would have full field mapping)
fn parse_dexscreener_row(row: &Row) -> SqliteResult<DexScreenerPool> {
    // Parse all 50+ fields from row - simplified here
    Ok(DexScreenerPool {
        chain_id: row.get(1)?,
        dex_id: row.get(2)?,
        pair_address: row.get(3)?,
        base_token_address: row.get(4)?,
        base_token_name: row.get(5)?,
        base_token_symbol: row.get(6)?,
        quote_token_address: row.get(7)?,
        quote_token_name: row.get(8)?,
        quote_token_symbol: row.get(9)?,
        price_native: row.get(10)?,
        price_usd: row.get(11)?,
        liquidity_usd: row.get(12)?,
        // ... rest of fields
        ..Default::default()
    })
}

fn parse_geckoterminal_row(row: &Row) -> SqliteResult<GeckoTerminalPool> {
    Ok(GeckoTerminalPool {
        mint: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        pool_address: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        pool_name: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        dex_id: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        base_token_id: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        quote_token_id: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
        base_token_price_usd: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        base_token_price_native: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
        base_token_price_quote: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
        // Use defaults for remaining fields
        ..Default::default()
    })
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
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))
        .map_err(|e| format!("Failed to query blacklist: {}", e))?;
    let mut out = Vec::new();
    for r in rows { out.push(r.map_err(|e| e.to_string())?); }
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
        risks: row.get::<_, Option<String>>(10)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        top_holders: row.get::<_, Option<String>>(11)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
        // Use defaults for unparsed fields
        ..Default::default()
    })
}
