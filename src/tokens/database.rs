/// Unified database operations for tokens system
/// All SQL operations in one place with proper error handling
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

use crate::tokens::types::{
    DataSource, DexScreenerData, GeckoTerminalData, Priority, RugcheckData, SecurityRisk,
    SocialLink, Token, TokenError, TokenHolder, TokenMetadata, TokenResult, UpdateTrackingInfo,
    WebsiteLink,
};

// Global database instance for easy access
static GLOBAL_DB: OnceCell<Arc<TokenDatabase>> = OnceCell::new();

/// Initialize global database (called by service)
pub fn init_global_database(db: Arc<TokenDatabase>) -> Result<(), String> {
    GLOBAL_DB
        .set(db)
        .map_err(|_| "Global database already initialized".to_string())
}

/// Get global database instance
pub fn get_global_database() -> Option<Arc<TokenDatabase>> {
    GLOBAL_DB.get().cloned()
}

/// Token database with connection pool
pub struct TokenDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl TokenDatabase {
    /// Create new database instance
    pub fn new(path: &str) -> TokenResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| TokenError::Database(format!("Failed to open database: {}", e)))?;

        // Initialize schema
        crate::tokens::schema::initialize_schema(&conn).map_err(|e| TokenError::Database(e))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get connection for external schema operations
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    // ========================================================================
    // TOKEN METADATA OPERATIONS
    // ========================================================================

    /// Create or update token metadata
    pub fn upsert_token(
        &self,
        mint: &str,
        symbol: Option<&str>,
        name: Option<&str>,
        decimals: Option<u8>,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO tokens (mint, symbol, name, decimals, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(mint) DO UPDATE SET
                symbol = COALESCE(?2, symbol),
                name = COALESCE(?3, name),
                decimals = COALESCE(?4, decimals),
                updated_at = ?5",
            params![mint, symbol, name, decimals.map(|d| d as i64), now],
        )
        .map_err(|e| TokenError::Database(format!("Failed to upsert token: {}", e)))?;

        // Ensure tracking entry exists
        conn.execute(
            "INSERT OR IGNORE INTO update_tracking (mint, priority) VALUES (?1, 10)",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to create tracking: {}", e)))?;

        Ok(())
    }

    /// Get token metadata
    pub fn get_token(&self, mint: &str) -> TokenResult<Option<TokenMetadata>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT mint, symbol, name, decimals, created_at, updated_at FROM tokens WHERE mint = ?1"
        ).map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            Ok(TokenMetadata {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get::<_, Option<i64>>(3)?.map(|d| d as u8),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        });

        match result {
            Ok(token) => Ok(Some(token)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    /// Check if token exists
    pub fn token_exists(&self, mint: &str) -> TokenResult<bool> {
        Ok(self.get_token(mint)?.is_some())
    }

    /// List all tokens with limit
    pub fn list_tokens(&self, limit: usize) -> TokenResult<Vec<TokenMetadata>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, symbol, name, decimals, created_at, updated_at 
             FROM tokens 
             ORDER BY updated_at DESC 
             LIMIT ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let tokens = stmt
            .query_map(params![limit], |row| {
                Ok(TokenMetadata {
                    mint: row.get(0)?,
                    symbol: row.get(1)?,
                    name: row.get(2)?,
                    decimals: row.get::<_, Option<i64>>(3)?.map(|d| d as u8),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        tokens
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))
    }

    // ========================================================================
    // DEXSCREENER DATA OPERATIONS
    // ========================================================================

    /// Store DexScreener market data
    pub fn upsert_dexscreener_data(&self, mint: &str, data: &DexScreenerData) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "INSERT INTO market_dexscreener (
                mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                txns_5m_buys, txns_5m_sells, txns_1h_buys, txns_1h_sells,
                txns_6h_buys, txns_6h_sells, txns_24h_buys, txns_24h_sells,
                pair_address, chain_id, dex_id, url, fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                       ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)
             ON CONFLICT(mint) DO UPDATE SET
                price_usd = ?2, price_sol = ?3, price_native = ?4,
                price_change_5m = ?5, price_change_1h = ?6, price_change_6h = ?7, price_change_24h = ?8,
                market_cap = ?9, fdv = ?10, liquidity_usd = ?11,
                volume_5m = ?12, volume_1h = ?13, volume_6h = ?14, volume_24h = ?15,
                txns_5m_buys = ?16, txns_5m_sells = ?17, txns_1h_buys = ?18, txns_1h_sells = ?19,
                txns_6h_buys = ?20, txns_6h_sells = ?21, txns_24h_buys = ?22, txns_24h_sells = ?23,
                pair_address = ?24, chain_id = ?25, dex_id = ?26, url = ?27, fetched_at = ?28",
            params![
                mint, data.price_usd, data.price_sol, &data.price_native,
                data.price_change_5m, data.price_change_1h, data.price_change_6h, data.price_change_24h,
                data.market_cap, data.fdv, data.liquidity_usd,
                data.volume_5m, data.volume_1h, data.volume_6h, data.volume_24h,
                data.txns_5m.map(|t| t.0 as i64), data.txns_5m.map(|t| t.1 as i64),
                data.txns_1h.map(|t| t.0 as i64), data.txns_1h.map(|t| t.1 as i64),
                data.txns_6h.map(|t| t.0 as i64), data.txns_6h.map(|t| t.1 as i64),
                data.txns_24h.map(|t| t.0 as i64), data.txns_24h.map(|t| t.1 as i64),
                &data.pair_address, &data.chain_id, &data.dex_id, &data.url,
                data.fetched_at.timestamp(),
            ],
        ).map_err(|e| TokenError::Database(format!("Failed to upsert DexScreener data: {}", e)))?;

        Ok(())
    }

    /// Get DexScreener market data
    pub fn get_dexscreener_data(&self, mint: &str) -> TokenResult<Option<DexScreenerData>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT price_usd, price_sol, price_native,
                    price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                    market_cap, fdv, liquidity_usd,
                    volume_5m, volume_1h, volume_6h, volume_24h,
                    txns_5m_buys, txns_5m_sells, txns_1h_buys, txns_1h_sells,
                    txns_6h_buys, txns_6h_sells, txns_24h_buys, txns_24h_sells,
                    pair_address, chain_id, dex_id, url, fetched_at
             FROM market_dexscreener WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            let txns_5m_buys: Option<i64> = row.get(14)?;
            let txns_5m_sells: Option<i64> = row.get(15)?;
            let txns_1h_buys: Option<i64> = row.get(16)?;
            let txns_1h_sells: Option<i64> = row.get(17)?;
            let txns_6h_buys: Option<i64> = row.get(18)?;
            let txns_6h_sells: Option<i64> = row.get(19)?;
            let txns_24h_buys: Option<i64> = row.get(20)?;
            let txns_24h_sells: Option<i64> = row.get(21)?;
            // fetched_at is the 27th selected column (0-based index 26)
            let fetched_ts: i64 = row.get(26)?;

            Ok(DexScreenerData {
                price_usd: row.get(0)?,
                price_sol: row.get(1)?,
                price_native: row.get(2)?,
                price_change_5m: row.get(3)?,
                price_change_1h: row.get(4)?,
                price_change_6h: row.get(5)?,
                price_change_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                liquidity_usd: row.get(9)?,
                volume_5m: row.get(10)?,
                volume_1h: row.get(11)?,
                volume_6h: row.get(12)?,
                volume_24h: row.get(13)?,
                txns_5m: txns_5m_buys.and_then(|b| txns_5m_sells.map(|s| (b as u32, s as u32))),
                txns_1h: txns_1h_buys.and_then(|b| txns_1h_sells.map(|s| (b as u32, s as u32))),
                txns_6h: txns_6h_buys.and_then(|b| txns_6h_sells.map(|s| (b as u32, s as u32))),
                txns_24h: txns_24h_buys.and_then(|b| txns_24h_sells.map(|s| (b as u32, s as u32))),
                pair_address: row.get(22)?,
                chain_id: row.get(23)?,
                dex_id: row.get(24)?,
                url: row.get(25)?,
                fetched_at: DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now()),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    // ========================================================================
    // GECKOTERMINAL DATA OPERATIONS
    // ========================================================================

    /// Store GeckoTerminal market data
    pub fn upsert_geckoterminal_data(
        &self,
        mint: &str,
        data: &GeckoTerminalData,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "INSERT INTO market_geckoterminal (
                mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                pool_count, top_pool_address, reserve_in_usd, fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
             ON CONFLICT(mint) DO UPDATE SET
                price_usd = ?2, price_sol = ?3, price_native = ?4,
                price_change_5m = ?5, price_change_1h = ?6, price_change_6h = ?7, price_change_24h = ?8,
                market_cap = ?9, fdv = ?10, liquidity_usd = ?11,
                volume_5m = ?12, volume_1h = ?13, volume_6h = ?14, volume_24h = ?15,
                pool_count = ?16, top_pool_address = ?17, reserve_in_usd = ?18, fetched_at = ?19",
            params![
                mint, data.price_usd, data.price_sol, &data.price_native,
                data.price_change_5m, data.price_change_1h, data.price_change_6h, data.price_change_24h,
                data.market_cap, data.fdv, data.liquidity_usd,
                data.volume_5m, data.volume_1h, data.volume_6h, data.volume_24h,
                data.pool_count.map(|c| c as i64), &data.top_pool_address, data.reserve_in_usd,
                data.fetched_at.timestamp(),
            ],
        ).map_err(|e| TokenError::Database(format!("Failed to upsert GeckoTerminal data: {}", e)))?;

        Ok(())
    }

    /// Get GeckoTerminal market data
    pub fn get_geckoterminal_data(&self, mint: &str) -> TokenResult<Option<GeckoTerminalData>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT price_usd, price_sol, price_native,
                    price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                    market_cap, fdv, liquidity_usd,
                    volume_5m, volume_1h, volume_6h, volume_24h,
                    pool_count, top_pool_address, reserve_in_usd, fetched_at
             FROM market_geckoterminal WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            // fetched_at is the last selected column for geckoterminal (0-based index 17)
            let fetched_ts: i64 = row.get(17)?;

            Ok(GeckoTerminalData {
                price_usd: row.get(0)?,
                price_sol: row.get(1)?,
                price_native: row.get(2)?,
                price_change_5m: row.get(3)?,
                price_change_1h: row.get(4)?,
                price_change_6h: row.get(5)?,
                price_change_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                liquidity_usd: row.get(9)?,
                volume_5m: row.get(10)?,
                volume_1h: row.get(11)?,
                volume_6h: row.get(12)?,
                volume_24h: row.get(13)?,
                pool_count: row.get::<_, Option<i64>>(14)?.map(|c| c as u32),
                top_pool_address: row.get(15)?,
                reserve_in_usd: row.get(16)?,
                fetched_at: DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now()),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    // ========================================================================
    // RUGCHECK SECURITY DATA OPERATIONS
    // ========================================================================

    /// Store Rugcheck security data
    pub fn upsert_rugcheck_data(&self, mint: &str, data: &RugcheckData) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let risks_json = serde_json::to_string(&data.risks)
            .map_err(|e| TokenError::Database(format!("Failed to serialize risks: {}", e)))?;
        let holders_json = serde_json::to_string(&data.top_holders)
            .map_err(|e| TokenError::Database(format!("Failed to serialize holders: {}", e)))?;
        let markets_json = data
            .markets
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()
            .map_err(|e| TokenError::Database(format!("Failed to serialize markets: {}", e)))?;

        conn.execute(
            "INSERT INTO security_rugcheck (
                mint, token_type, score, score_description,
                mint_authority, freeze_authority, top_10_holders_pct, total_supply,
                risks, top_holders, markets, fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(mint) DO UPDATE SET
                token_type = ?2, score = ?3, score_description = ?4,
                mint_authority = ?5, freeze_authority = ?6, top_10_holders_pct = ?7, total_supply = ?8,
                risks = ?9, top_holders = ?10, markets = ?11, fetched_at = ?12",
            params![
                mint, &data.token_type, data.score, &data.score_description,
                &data.mint_authority, &data.freeze_authority, data.top_10_holders_pct, &data.total_supply,
                risks_json, holders_json, markets_json,
                data.fetched_at.timestamp(),
            ],
        ).map_err(|e| TokenError::Database(format!("Failed to upsert Rugcheck data: {}", e)))?;

        Ok(())
    }

    /// Get Rugcheck security data
    pub fn get_rugcheck_data(&self, mint: &str) -> TokenResult<Option<RugcheckData>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT token_type, score, score_description,
                    mint_authority, freeze_authority, top_10_holders_pct, total_supply,
                    risks, top_holders, markets, fetched_at
             FROM security_rugcheck WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            let risks_json: String = row.get(7)?;
            let holders_json: String = row.get(8)?;
            let markets_json: Option<String> = row.get(9)?;
            let fetched_ts: i64 = row.get(10)?;

            let risks: Vec<SecurityRisk> = serde_json::from_str(&risks_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let holders: Vec<TokenHolder> = serde_json::from_str(&holders_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let markets = markets_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(RugcheckData {
                token_type: row.get(0)?,
                score: row.get(1)?,
                score_description: row.get(2)?,
                mint_authority: row.get(3)?,
                freeze_authority: row.get(4)?,
                top_10_holders_pct: row.get(5)?,
                total_supply: row.get(6)?,
                risks,
                top_holders: holders,
                markets,
                fetched_at: DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now()),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    // ========================================================================
    // UPDATE TRACKING OPERATIONS
    // ========================================================================

    /// Get tokens by priority with limit
    pub fn get_tokens_by_priority(&self, priority: i32, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint FROM update_tracking 
                 WHERE priority = ?1
                 AND (last_error_at IS NULL OR last_error_at < strftime('%s','now') - 180)
                 ORDER BY last_market_update ASC NULLS FIRST 
                 LIMIT ?2",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mints = stmt
            .query_map(params![priority, limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))
    }

    /// Get oldest non-blacklisted tokens
    pub fn get_oldest_non_blacklisted(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
             LEFT JOIN blacklist b ON t.mint = b.mint
             LEFT JOIN update_tracking u ON t.mint = u.mint
             WHERE b.mint IS NULL
             ORDER BY COALESCE(u.last_market_update, 0) ASC
             LIMIT ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mints = stmt
            .query_map(params![limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))
    }

    /// Update priority for a token
    pub fn update_priority(&self, mint: &str, priority: i32) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET priority = ?1 WHERE mint = ?2",
            params![priority, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to update priority: {}", e)))?;

        Ok(())
    }

    /// Get tokens that have never received market data
    pub fn get_tokens_without_market_data(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
             INNER JOIN update_tracking u ON t.mint = u.mint
             LEFT JOIN market_dexscreener md ON t.mint = md.mint
             LEFT JOIN market_geckoterminal mg ON t.mint = mg.mint
             WHERE u.market_update_count = 0
             AND md.mint IS NULL
             AND mg.mint IS NULL
             AND (u.last_error_at IS NULL OR u.last_error_at < strftime('%s','now') - 180)
             ORDER BY u.priority DESC, t.created_at ASC
             LIMIT ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mints = stmt
            .query_map(params![limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))
    }

    /// Record a failed market update attempt (used to throttle retries)
    pub fn record_market_error(&self, mint: &str, message: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET last_error = ?1, last_error_at = ?2 WHERE mint = ?3",
            params![message, now, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to record market error: {}", e)))?;

        Ok(())
    }

    /// Mark token as updated
    pub fn mark_updated(&self, mint: &str, had_errors: bool) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        if had_errors {
            conn.execute(
                "UPDATE update_tracking SET 
                    last_market_update = ?1,
                    market_update_count = market_update_count + 1,
                    last_error_at = ?1
                 WHERE mint = ?2",
                params![now, mint],
            )
            .map_err(|e| TokenError::Database(format!("Failed to mark updated: {}", e)))?;
        } else {
            conn.execute(
                "UPDATE update_tracking SET 
                    last_market_update = ?1,
                    market_update_count = market_update_count + 1,
                    last_error = NULL,
                    last_error_at = NULL
                 WHERE mint = ?2",
                params![now, mint],
            )
            .map_err(|e| TokenError::Database(format!("Failed to mark updated: {}", e)))?;
        }

        // Also update tokens table
        conn.execute(
            "UPDATE tokens SET updated_at = ?1 WHERE mint = ?2",
            params![now, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to update tokens: {}", e)))?;

        Ok(())
    }

    // ========================================================================
    // BLACKLIST OPERATIONS
    // ========================================================================

    /// Add token to blacklist
    pub fn add_to_blacklist(&self, mint: &str, reason: &str, source: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO blacklist (mint, reason, source, added_at) 
             VALUES (?1, ?2, ?3, ?4)",
            params![mint, reason, source, now],
        )
        .map_err(|e| TokenError::Database(format!("Failed to add to blacklist: {}", e)))?;

        Ok(())
    }

    /// Check if token is blacklisted
    pub fn is_blacklisted(&self, mint: &str) -> TokenResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT 1 FROM blacklist WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let exists = stmt
            .exists(params![mint])
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        Ok(exists)
    }

    /// Remove token from blacklist
    pub fn remove_from_blacklist(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute("DELETE FROM blacklist WHERE mint = ?1", params![mint])
            .map_err(|e| TokenError::Database(format!("Failed to remove from blacklist: {}", e)))?;

        Ok(())
    }

    /// Get blacklist reason
    pub fn get_blacklist_reason(&self, mint: &str) -> TokenResult<Option<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT reason, source FROM blacklist WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| Ok((row.get(0)?, row.get(1)?)));

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    // ========================================================================
    // AGGREGATE & DEBUG HELPERS
    // ========================================================================

    /// Count total tokens stored in the tokens table
    pub fn count_tokens(&self) -> TokenResult<u64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Failed to count tokens: {}", e)))?;

        Ok(count.max(0) as u64)
    }

    /// Count tokens currently tracked for updates
    pub fn count_tracked_tokens(&self) -> TokenResult<u64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM update_tracking", [], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Failed to count tracked tokens: {}", e)))?;

        Ok(count.max(0) as u64)
    }

    /// Count blacklisted tokens
    pub fn count_blacklisted(&self) -> TokenResult<u64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM blacklist", [], |row| row.get(0))
            .map_err(|e| {
                TokenError::Database(format!("Failed to count blacklisted tokens: {}", e))
            })?;

        Ok(count.max(0) as u64)
    }

    /// Retrieve update tracking information for a specific token
    pub fn get_update_tracking_info(&self, mint: &str) -> TokenResult<Option<UpdateTrackingInfo>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, priority, last_market_update, last_security_update, last_decimals_update,
                        market_update_count, security_update_count, last_error, last_error_at
                 FROM update_tracking
                 WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| map_tracking_row(row));

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    /// List update tracking entries with optional priority filter
    pub fn list_update_tracking(
        &self,
        limit: usize,
        priority: Option<i32>,
    ) -> TokenResult<Vec<UpdateTrackingInfo>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let records = if let Some(priority) = priority {
            let mut stmt = conn
                .prepare(
                    "SELECT mint, priority, last_market_update, last_security_update, last_decimals_update,
                            market_update_count, security_update_count, last_error, last_error_at
                     FROM update_tracking
                     WHERE priority = ?1
                     ORDER BY COALESCE(last_market_update, 0) ASC, mint ASC
                     LIMIT ?2",
                )
                .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

            let rows = stmt
                .query_map(params![priority, limit as i64], |row| map_tracking_row(row))
                .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

            rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
                TokenError::Database(format!("Failed to collect tracking entries: {}", e))
            })?
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT mint, priority, last_market_update, last_security_update, last_decimals_update,
                            market_update_count, security_update_count, last_error, last_error_at
                     FROM update_tracking
                     ORDER BY priority DESC, COALESCE(last_market_update, 0) ASC, mint ASC
                     LIMIT ?1",
                )
                .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

            let rows = stmt
                .query_map(params![limit as i64], |row| map_tracking_row(row))
                .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

            rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
                TokenError::Database(format!("Failed to collect tracking entries: {}", e))
            })?
        };

        Ok(records)
    }

    /// Summarize tracked tokens by their priority value
    pub fn summarize_priorities(&self) -> TokenResult<Vec<(i32, u64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT priority, COUNT(*) FROM update_tracking GROUP BY priority ORDER BY priority DESC",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let priority: i32 = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((priority, count.max(0) as u64))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect priority summary: {}", e)))
    }

    // ========================================================================
    // FULL TOKEN ASSEMBLY (for external code)
    // ========================================================================

    /// Assemble complete Token struct from all data sources
    ///
    /// This is the bridge function that external code should use when they need
    /// full token data (market + security). It assembles data from:
    /// - TokenMetadata (basic info)
    /// - DexScreenerData or GeckoTerminalData (market data, based on config)
    /// - RugcheckData (security data)
    /// - Blacklist status
    ///
    /// Returns None if token doesn't exist or has no market data from preferred source.
    pub fn get_full_token(&self, mint: &str) -> TokenResult<Option<Token>> {
        // Get basic metadata
        let metadata = match self.get_token(mint)? {
            Some(m) => m,
            None => return Ok(None),
        };

        // Determine preferred source from config
        let preferred_source =
            crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());

        // Get market data from preferred source
        let (market_data, data_source) = if preferred_source == "geckoterminal" {
            match self.get_geckoterminal_data(mint)? {
                Some(data) => (
                    MarketDataType::GeckoTerminal(data),
                    DataSource::GeckoTerminal,
                ),
                None => match self.get_dexscreener_data(mint)? {
                    Some(data) => (MarketDataType::DexScreener(data), DataSource::DexScreener),
                    None => return Ok(None), // No market data available
                },
            }
        } else {
            match self.get_dexscreener_data(mint)? {
                Some(data) => (MarketDataType::DexScreener(data), DataSource::DexScreener),
                None => match self.get_geckoterminal_data(mint)? {
                    Some(data) => (
                        MarketDataType::GeckoTerminal(data),
                        DataSource::GeckoTerminal,
                    ),
                    None => return Ok(None), // No market data available
                },
            }
        };

        // Get security data
        let security = self.get_rugcheck_data(mint)?;

        // Get blacklist status
        let is_blacklisted = self.is_blacklisted(mint)?;

        // Get priority
        let priority = self.get_priority(mint)?;

        // Assemble Token struct
        let token = assemble_token(
            metadata,
            market_data,
            data_source,
            security,
            is_blacklisted,
            priority,
        );

        Ok(Some(token))
    }

    /// Get priority for a token
    fn get_priority(&self, mint: &str) -> TokenResult<Priority> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT priority FROM update_tracking WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let priority: i32 = stmt
            .query_row(params![mint], |row| row.get(0))
            .unwrap_or(10); // Default to Low priority

        Ok(Priority::from_value(priority))
    }
}

// ============================================================================
// HELPER TYPES AND FUNCTIONS
// ============================================================================

/// Market data type wrapper
enum MarketDataType {
    DexScreener(DexScreenerData),
    GeckoTerminal(GeckoTerminalData),
}

/// Assemble Token from components
fn assemble_token(
    metadata: TokenMetadata,
    market_data: MarketDataType,
    data_source: DataSource,
    security: Option<RugcheckData>,
    is_blacklisted: bool,
    priority: Priority,
) -> Token {
    // Extract market data fields based on source
    let (
        price_usd,
        price_sol,
        price_native,
        price_changes,
        market_metrics,
        volumes,
        txns,
        fetched_at,
    ) = match market_data {
        MarketDataType::DexScreener(data) => {
            let txns = (
                data.txns_5m.map(|(b, s)| (b as i64, s as i64)),
                data.txns_1h.map(|(b, s)| (b as i64, s as i64)),
                data.txns_6h.map(|(b, s)| (b as i64, s as i64)),
                data.txns_24h.map(|(b, s)| (b as i64, s as i64)),
            );

            (
                data.price_usd,
                data.price_sol,
                data.price_native,
                (
                    data.price_change_5m,
                    data.price_change_1h,
                    data.price_change_6h,
                    data.price_change_24h,
                ),
                (data.market_cap, data.fdv, data.liquidity_usd),
                (
                    data.volume_5m,
                    data.volume_1h,
                    data.volume_6h,
                    data.volume_24h,
                ),
                txns,
                data.fetched_at,
            )
        }
        MarketDataType::GeckoTerminal(data) => {
            let txns = (None, None, None, None); // GeckoTerminal doesn't provide txn data

            (
                data.price_usd,
                data.price_sol,
                data.price_native,
                (
                    data.price_change_5m,
                    data.price_change_1h,
                    data.price_change_6h,
                    data.price_change_24h,
                ),
                (data.market_cap, data.fdv, data.liquidity_usd),
                (
                    data.volume_5m,
                    data.volume_1h,
                    data.volume_6h,
                    data.volume_24h,
                ),
                txns,
                data.fetched_at,
            )
        }
    };

    // Extract security data
    let (
        mint_authority,
        freeze_authority,
        security_score,
        is_rugged,
        security_risks,
        top_holders,
        total_holders,
        creator_balance_pct,
        transfer_fee_pct,
    ) = if let Some(sec) = security {
        let is_rugged = sec.score.map(|s| s < 20).unwrap_or(false);
        let total_holders = sec.top_holders.len() as i64;
        let creator_pct = sec.top_10_holders_pct;

        (
            sec.mint_authority,
            sec.freeze_authority,
            sec.score,
            is_rugged,
            sec.risks,
            sec.top_holders,
            Some(total_holders),
            creator_pct,
            None, // Transfer fee not in rugcheck data
        )
    } else {
        (None, None, None, false, vec![], vec![], None, None, None)
    };

    Token {
        // Core identity
        mint: metadata.mint.clone(),
        symbol: metadata.symbol.unwrap_or_else(|| "UNKNOWN".to_string()),
        name: metadata.name.unwrap_or_else(|| "Unknown Token".to_string()),
        decimals: metadata.decimals.unwrap_or(9),
        description: None,
        image_url: None,
        header_image_url: None,
        supply: None,

        // Data source
        data_source,
        fetched_at,
        updated_at: DateTime::from_timestamp(metadata.updated_at, 0).unwrap_or_else(|| Utc::now()),

        // Price information
        price_usd,
        price_sol,
        price_native,
        price_change_m5: price_changes.0,
        price_change_h1: price_changes.1,
        price_change_h6: price_changes.2,
        price_change_h24: price_changes.3,

        // Market metrics
        market_cap: market_metrics.0,
        fdv: market_metrics.1,
        liquidity_usd: market_metrics.2,

        // Volume data
        volume_m5: volumes.0,
        volume_h1: volumes.1,
        volume_h6: volumes.2,
        volume_h24: volumes.3,

        // Transaction activity
        txns_m5_buys: txns.0.map(|(b, _)| b),
        txns_m5_sells: txns.0.map(|(_, s)| s),
        txns_h1_buys: txns.1.map(|(b, _)| b),
        txns_h1_sells: txns.1.map(|(_, s)| s),
        txns_h6_buys: txns.2.map(|(b, _)| b),
        txns_h6_sells: txns.2.map(|(_, s)| s),
        txns_h24_buys: txns.3.map(|(b, _)| b),
        txns_h24_sells: txns.3.map(|(_, s)| s),

        // Social & links (not available from current APIs)
        websites: vec![],
        socials: vec![],

        // Security information
        mint_authority,
        freeze_authority,
        security_score,
        is_rugged,
        security_risks,
        total_holders,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,

        // Bot-specific state
        is_blacklisted,
        priority,
        first_seen_at: DateTime::from_timestamp(metadata.created_at, 0)
            .unwrap_or_else(|| Utc::now()),
        last_price_update: fetched_at,
    }
}

// ============================================================================
// ASYNC WRAPPERS (for external code)
// ============================================================================

/// Async wrapper for get_token (returns TokenMetadata)
pub async fn get_token_async(mint: &str) -> TokenResult<Option<TokenMetadata>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    let mint = mint.to_string();
    tokio::task::spawn_blocking(move || db.get_token(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for get_full_token (returns complete Token)
pub async fn get_full_token_async(mint: &str) -> TokenResult<Option<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    let mint = mint.to_string();
    tokio::task::spawn_blocking(move || db.get_full_token(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for list_tokens (returns Vec<TokenMetadata>)
pub async fn list_tokens_async(limit: usize) -> TokenResult<Vec<TokenMetadata>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || db.list_tokens(limit))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

fn map_tracking_row(row: &rusqlite::Row) -> rusqlite::Result<UpdateTrackingInfo> {
    let mint: String = row.get(0)?;
    let priority: i32 = row.get(1)?;
    let last_market = ts_to_datetime(row.get::<_, Option<i64>>(2)?);
    let last_security = ts_to_datetime(row.get::<_, Option<i64>>(3)?);
    let last_decimals = ts_to_datetime(row.get::<_, Option<i64>>(4)?);
    let market_update_count = row.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0) as u64;
    let security_update_count = row.get::<_, Option<i64>>(6)?.unwrap_or(0).max(0) as u64;
    let last_error: Option<String> = row.get(7)?;
    let last_error_at = ts_to_datetime(row.get::<_, Option<i64>>(8)?);

    Ok(UpdateTrackingInfo {
        mint,
        priority,
        last_market_update: last_market,
        last_security_update: last_security,
        last_decimals_update: last_decimals,
        market_update_count,
        security_update_count,
        last_error,
        last_error_at,
    })
}

fn ts_to_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.and_then(|value| DateTime::from_timestamp(value, 0))
}
