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

    /// Store Rugcheck security data (clean schema)
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

        let rugged_flag = if data.rugged { 1 } else { 0 };

        conn.execute(
            "INSERT INTO security_rugcheck (
                mint,
                token_type,
                token_decimals,
                score,
                score_description,
                mint_authority,
                freeze_authority,
                top_10_holders_pct,
                total_supply,
                total_holders,
                total_lp_providers,
                graph_insiders_detected,
                total_market_liquidity,
                total_stable_liquidity,
                creator_balance_pct,
                transfer_fee_pct,
                transfer_fee_max_amount,
                transfer_fee_authority,
                rugged,
                risks,
                top_holders,
                markets,
                fetched_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
             )
             ON CONFLICT(mint) DO UPDATE SET
                token_type = excluded.token_type,
                token_decimals = excluded.token_decimals,
                score = excluded.score,
                score_description = excluded.score_description,
                mint_authority = excluded.mint_authority,
                freeze_authority = excluded.freeze_authority,
                top_10_holders_pct = excluded.top_10_holders_pct,
                total_supply = excluded.total_supply,
                total_holders = excluded.total_holders,
                total_lp_providers = excluded.total_lp_providers,
                graph_insiders_detected = excluded.graph_insiders_detected,
                total_market_liquidity = excluded.total_market_liquidity,
                total_stable_liquidity = excluded.total_stable_liquidity,
                creator_balance_pct = excluded.creator_balance_pct,
                transfer_fee_pct = excluded.transfer_fee_pct,
                transfer_fee_max_amount = excluded.transfer_fee_max_amount,
                transfer_fee_authority = excluded.transfer_fee_authority,
                rugged = excluded.rugged,
                risks = excluded.risks,
                top_holders = excluded.top_holders,
                markets = excluded.markets,
                fetched_at = excluded.fetched_at",
            params![
                mint,
                &data.token_type,
                data.token_decimals,
                data.score,
                &data.score_description,
                &data.mint_authority,
                &data.freeze_authority,
                data.top_10_holders_pct,
                &data.total_supply,
                data.total_holders,
                data.total_lp_providers,
                data.graph_insiders_detected,
                data.total_market_liquidity,
                data.total_stable_liquidity,
                data.creator_balance_pct,
                data.transfer_fee_pct,
                data.transfer_fee_max_amount,
                &data.transfer_fee_authority,
                rugged_flag,
                risks_json,
                holders_json,
                markets_json,
                data.fetched_at.timestamp(),
            ],
        )
        .map_err(|e| TokenError::Database(format!("Failed to upsert Rugcheck data: {}", e)))?;

        Ok(())
    }
    /// Get connection for external schema operations
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// Count tokens with NO market data in both DexScreener and GeckoTerminal
    pub fn count_tokens_no_market(&self) -> TokenResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens t \
                 LEFT JOIN market_dexscreener d ON t.mint = d.mint \
                 LEFT JOIN market_geckoterminal g ON t.mint = g.mint \
                 WHERE d.mint IS NULL AND g.mint IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|e| TokenError::Database(format!("Count no-market failed: {}", e)))?;

        Ok(count)
    }

    /// Get tokens WITHOUT any market data (assemble minimal tokens)
    pub fn get_tokens_no_market(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_direction: Option<&str>,
    ) -> TokenResult<Vec<Token>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // Only support sorting by metadata/security fields for this view
        let order_column = match sort_by {
            Some("symbol") => "t.symbol",
            Some("updated_at") => "COALESCE(ut.last_market_update, t.created_at)",
            Some("first_seen_at") => "t.created_at",
            Some("metadata_updated_at") => "COALESCE(ut.last_decimals_update, t.updated_at)",
            Some("token_birth_at") => "COALESCE(ut.last_decimals_update, t.created_at)",
            Some("mint") => "t.mint",
            Some("risk_score") => "sr.score",
            _ => "COALESCE(ut.last_market_update, t.created_at)",
        };
        let direction = match sort_direction {
            Some("asc") => "ASC",
            Some("desc") => "DESC",
            _ => "DESC",
        };

        let base = "SELECT \
                        t.mint, t.symbol, t.name, t.decimals, t.created_at, \
                        COALESCE(ut.last_decimals_update, t.updated_at) AS metadata_updated_at, \
                        ut.last_market_update, \
                        sr.score, sr.rugged, \
                        bl.reason as blacklist_reason, \
                        ut.priority \
                    FROM tokens t \
                    LEFT JOIN security_rugcheck sr ON t.mint = sr.mint \
                    LEFT JOIN blacklist bl ON t.mint = bl.mint \
                    LEFT JOIN update_tracking ut ON t.mint = ut.mint \
                    LEFT JOIN market_dexscreener d ON t.mint = d.mint \
                    LEFT JOIN market_geckoterminal g ON t.mint = g.mint \
                    WHERE d.mint IS NULL AND g.mint IS NULL";

        let query = if limit == 0 {
            format!("{} ORDER BY {} {}", base, order_column, direction)
        } else {
            format!(
                "{} ORDER BY {} {} LIMIT {} OFFSET {}",
                base, order_column, direction, limit, offset
            )
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let rows = stmt
            .query_map(params![], |row| {
                let metadata = TokenMetadata {
                    mint: row.get::<_, String>(0)?,
                    symbol: row.get::<_, Option<String>>(1)?,
                    name: row.get::<_, Option<String>>(2)?,
                    decimals: row.get::<_, Option<i64>>(3)?.map(|v| v as u8),
                    created_at: row.get::<_, i64>(4)?,
                    updated_at: row.get::<_, i64>(5)?,
                };
                let last_market_update: Option<i64> = row.get(6)?;
                let security_score: Option<i32> = row.get(7)?;
                let is_rugged: bool = row
                    .get::<_, Option<i64>>(8)?
                    .map(|v| v != 0)
                    .unwrap_or(false);
                let is_blacklisted = row.get::<_, Option<String>>(9)?.is_some();
                let priority_value: Option<i32> = row.get(10)?;

                Ok((
                    metadata,
                    last_market_update,
                    security_score,
                    is_rugged,
                    is_blacklisted,
                    priority_value,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query no-market failed: {}", e)))?;

        let mut tokens = Vec::new();
        for row in rows {
            let (
                meta,
                last_market_update,
                security_score,
                is_rugged,
                is_blacklisted,
                priority_value,
            ) = row.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;

            // Build a RugcheckData-lite only for values we expose directly; we can avoid it and set fields below
            let security = if security_score.is_some() || is_rugged {
                Some(RugcheckData {
                    token_type: None,
                    token_decimals: None,
                    score: security_score,
                    score_description: None,
                    mint_authority: None,
                    freeze_authority: None,
                    top_10_holders_pct: None,
                    total_holders: None,
                    total_lp_providers: None,
                    graph_insiders_detected: None,
                    total_market_liquidity: None,
                    total_stable_liquidity: None,
                    total_supply: None,
                    creator_balance_pct: None,
                    transfer_fee_pct: None,
                    transfer_fee_max_amount: None,
                    transfer_fee_authority: None,
                    rugged: is_rugged,
                    risks: vec![],
                    top_holders: vec![],
                    markets: None,
                    fetched_at: Utc::now(),
                })
            } else {
                None
            };

            let priority = priority_value
                .map(Priority::from_value)
                .unwrap_or(Priority::Medium);

            let token = assemble_token_without_market_data(
                meta,
                security,
                is_blacklisted,
                priority,
                None,
                last_market_update.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                None,
            );
            tokens.push(token);
        }

        Ok(tokens)
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
                pair_address, chain_id, dex_id, url, pair_created_at, image_url, header_image_url, fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                       ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31)
             ON CONFLICT(mint) DO UPDATE SET
                price_usd = ?2, price_sol = ?3, price_native = ?4,
                price_change_5m = ?5, price_change_1h = ?6, price_change_6h = ?7, price_change_24h = ?8,
                market_cap = ?9, fdv = ?10, liquidity_usd = ?11,
                volume_5m = ?12, volume_1h = ?13, volume_6h = ?14, volume_24h = ?15,
                txns_5m_buys = ?16, txns_5m_sells = ?17, txns_1h_buys = ?18, txns_1h_sells = ?19,
                txns_6h_buys = ?20, txns_6h_sells = ?21, txns_24h_buys = ?22, txns_24h_sells = ?23,
                pair_address = ?24, chain_id = ?25, dex_id = ?26, url = ?27, pair_created_at = ?28, image_url = ?29, header_image_url = ?30, fetched_at = ?31",
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
                data.pair_created_at.map(|dt| dt.timestamp()),
                &data.image_url,
                &data.header_image_url,
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
                    pair_address, chain_id, dex_id, url, pair_created_at, image_url, header_image_url, fetched_at
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
            // fetched_at is now the 30th selected column (0-based index 29)
            let pair_created_ts: Option<i64> = row.get(26)?;
            let image_url: Option<String> = row.get(27)?;
            let header_image_url: Option<String> = row.get(28)?;
            let fetched_ts: i64 = row.get(29)?;

            Ok(DexScreenerData {
                // Some historical rows may have NULLs; treat missing numeric/text values as defaults
                price_usd: row.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                price_sol: row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                price_native: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "0".to_string()),
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
                pair_created_at: pair_created_ts.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                image_url,
                header_image_url,
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

        // Clean schema insert (image_url column included)
        let insert_result = conn.execute(
            "INSERT INTO market_geckoterminal (
                mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                pool_count, top_pool_address, reserve_in_usd, image_url, fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
             ON CONFLICT(mint) DO UPDATE SET
                price_usd = ?2, price_sol = ?3, price_native = ?4,
                price_change_5m = ?5, price_change_1h = ?6, price_change_6h = ?7, price_change_24h = ?8,
                market_cap = ?9, fdv = ?10, liquidity_usd = ?11,
                volume_5m = ?12, volume_1h = ?13, volume_6h = ?14, volume_24h = ?15,
                pool_count = ?16, top_pool_address = ?17, reserve_in_usd = ?18, image_url = ?19, fetched_at = ?20",
            params![
                mint, data.price_usd, data.price_sol, &data.price_native,
                data.price_change_5m, data.price_change_1h, data.price_change_6h, data.price_change_24h,
                data.market_cap, data.fdv, data.liquidity_usd,
                data.volume_5m, data.volume_1h, data.volume_6h, data.volume_24h,
                data.pool_count.map(|c| c as i64), &data.top_pool_address, data.reserve_in_usd,
                &data.image_url, data.fetched_at.timestamp(),
            ],
        );

        insert_result.map_err(|e| {
            TokenError::Database(format!("Failed to upsert GeckoTerminal data: {}", e))
        })?;

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
                    pool_count, top_pool_address, reserve_in_usd, image_url, fetched_at
             FROM market_geckoterminal WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            // fetched_at is the last selected column for geckoterminal (0-based index 18)
            let fetched_ts: i64 = row.get(18)?;

            Ok(GeckoTerminalData {
                // Some historical rows may have NULLs; treat missing numeric/text values as defaults
                price_usd: row.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                price_sol: row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                price_native: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "0".to_string()),
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
                image_url: row.get(17)?,
                fetched_at: DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now()),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    /// Get Rugcheck security data
    pub fn get_rugcheck_data(&self, mint: &str) -> TokenResult<Option<RugcheckData>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT
                    token_type,
                    token_decimals,
                    score,
                    score_description,
                    mint_authority,
                    freeze_authority,
                    top_10_holders_pct,
                    total_supply,
                    total_holders,
                    total_lp_providers,
                    graph_insiders_detected,
                    total_market_liquidity,
                    total_stable_liquidity,
                    creator_balance_pct,
                    transfer_fee_pct,
                    transfer_fee_max_amount,
                    transfer_fee_authority,
                    rugged,
                    risks,
                    top_holders,
                    markets,
                    fetched_at
                 FROM security_rugcheck WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            let risks_json: String = row.get(18)?;
            let holders_json: String = row.get(19)?;
            let markets_json: Option<String> = row.get(20)?;
            let fetched_ts: i64 = row.get(21)?;
            let rugged_flag: Option<i64> = row.get(17)?;
            let is_rugged = rugged_flag.unwrap_or(0) != 0;

            let risks: Vec<SecurityRisk> = serde_json::from_str(&risks_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let holders: Vec<TokenHolder> = serde_json::from_str(&holders_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let markets = markets_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(RugcheckData {
                token_type: row.get(0)?,
                token_decimals: row.get(1)?,
                score: row.get(2)?,
                score_description: row.get(3)?,
                mint_authority: row.get(4)?,
                freeze_authority: row.get(5)?,
                top_10_holders_pct: row.get(6)?,
                total_supply: row.get(7)?,
                total_holders: row.get(8)?,
                total_lp_providers: row.get(9)?,
                graph_insiders_detected: row.get(10)?,
                total_market_liquidity: row.get(11)?,
                total_stable_liquidity: row.get(12)?,
                creator_balance_pct: row.get(13)?,
                transfer_fee_pct: row.get(14)?,
                transfer_fee_max_amount: row.get(15)?,
                transfer_fee_authority: row.get(16)?,
                rugged: is_rugged,
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

    /// Get tokens without security (Rugcheck) data with exponential backoff for errors
    pub fn get_tokens_without_security_data(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        // Base backoff interval: 2 minutes (120 seconds)
        // Max backoff: 24 hours (86400 seconds)
        // Formula: min(120 * 2^(error_count - 1), 86400)
        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
             LEFT JOIN security_rugcheck sr ON t.mint = sr.mint
             LEFT JOIN blacklist b ON t.mint = b.mint
             LEFT JOIN update_tracking ut ON t.mint = ut.mint
             WHERE sr.mint IS NULL
             AND b.mint IS NULL
             AND (
                 -- Never tried
                 ut.security_error_type IS NULL
                 -- Temporary errors with exponential backoff
                 OR (ut.security_error_type = 'temporary' 
                     AND ut.last_security_error_at < ?1 - (120 * (1 << MIN(ut.security_error_count - 1, 10))))
                 -- Permanent errors retry after 7 days
                 OR (ut.security_error_type = 'permanent' 
                     AND ut.last_security_error_at < ?1 - 604800)
             )
             ORDER BY 
                 CASE 
                     -- Priority 1: New tokens (created in last 24h, no errors)
                     WHEN ut.security_error_type IS NULL AND t.created_at > ?1 - 86400 THEN 1
                     -- Priority 2: Tokens without errors
                     WHEN ut.security_error_type IS NULL THEN 2
                     -- Priority 3: Temporary errors (with backoff)
                     WHEN ut.security_error_type = 'temporary' THEN 3
                     -- Priority 4: Permanent errors (very rare retry)
                     ELSE 4
                 END,
                 t.created_at ASC
             LIMIT ?2",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mints = stmt
            .query_map(params![now, limit], |row| row.get(0))
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

    /// Record a security fetch error with exponential backoff tracking
    pub fn record_security_error(
        &self,
        mint: &str,
        message: &str,
        error_type: &str,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                security_error_count = security_error_count + 1,
                last_security_error = ?1,
                last_security_error_at = ?2,
                security_error_type = ?3
             WHERE mint = ?4",
            params![message, now, error_type, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to record security error: {}", e)))?;

        Ok(())
    }

    /// Clear security error tracking (called after successful fetch)
    pub fn clear_security_error(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET 
                security_error_count = 0,
                last_security_error = NULL,
                last_security_error_at = NULL,
                security_error_type = NULL
             WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to clear security error: {}", e)))?;

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

    /// Assemble complete Token using a specific market data source
    pub fn get_full_token_for_source(
        &self,
        mint: &str,
        source: DataSource,
    ) -> TokenResult<Option<Token>> {
        let metadata = match self.get_token(mint)? {
            Some(m) => m,
            None => return Ok(None),
        };

        let (market_data, data_source) = match source {
            DataSource::DexScreener => match self.get_dexscreener_data(mint)? {
                Some(data) => (MarketDataType::DexScreener(data), DataSource::DexScreener),
                None => return Ok(None),
            },
            DataSource::GeckoTerminal => match self.get_geckoterminal_data(mint)? {
                Some(data) => (
                    MarketDataType::GeckoTerminal(data),
                    DataSource::GeckoTerminal,
                ),
                None => return Ok(None),
            },
            _ => return Ok(None),
        };

        let security = self.get_rugcheck_data(mint)?;
        let is_blacklisted = self.is_blacklisted(mint)?;
        let priority = self.get_priority(mint)?;

        // Prepare fallback images: when using GeckoTerminal, try DexScreener images from DB
        let (fallback_img, fallback_header) = match data_source {
            DataSource::GeckoTerminal => match self.get_dexscreener_data(mint)? {
                Some(ds) => (ds.image_url, ds.header_image_url),
                None => (None, None),
            },
            _ => (None, None),
        };

        let token = assemble_token(
            metadata,
            market_data,
            data_source,
            security,
            is_blacklisted,
            priority,
            fallback_img,
            fallback_header,
        );

        Ok(Some(token))
    }

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
        // Determine preferred source from config
        let preferred_source =
            crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());
        let primary_source = if preferred_source.eq_ignore_ascii_case("geckoterminal") {
            DataSource::GeckoTerminal
        } else {
            DataSource::DexScreener
        };

        if let Some(token) = self.get_full_token_for_source(mint, primary_source)? {
            return Ok(Some(token));
        }

        let fallback_source = match primary_source {
            DataSource::DexScreener => DataSource::GeckoTerminal,
            DataSource::GeckoTerminal => DataSource::DexScreener,
            _ => return Ok(None),
        };

        self.get_full_token_for_source(mint, fallback_source)
    }

    /// Get all tokens from database with optional market data.
    /// Unlike get_full_token(), this returns tokens EVEN WITHOUT market data,
    /// using default/null values for missing fields.
    ///
    /// Use this for "All Tokens" view to show complete database contents.
    ///
    /// If limit=0, returns ALL tokens. Otherwise returns limit tokens with offset.
    ///
    /// PERFORMANCE: Uses LEFT JOINs to fetch all data in a single query, avoiding N+1 problem.
    pub fn get_all_tokens_optional_market(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_direction: Option<&str>,
    ) -> TokenResult<Vec<Token>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // Map sort_by to SQL column with table prefix
        let order_column = match sort_by {
            Some("symbol") => "t.symbol",
            Some("updated_at") =>
                "COALESCE(ut.last_market_update, COALESCE(d.fetched_at, g.fetched_at, t.updated_at))",
            Some("first_seen_at") => "t.created_at",
            Some("metadata_updated_at") => "COALESCE(ut.last_decimals_update, t.updated_at)",
            Some("token_birth_at") => "COALESCE(d.pair_created_at, t.created_at)",
            Some("mint") => "t.mint",
            Some("risk_score") => "sr.score",
            Some("price_sol") => "COALESCE(d.price_sol, g.price_sol)",
            Some("liquidity_usd") => "COALESCE(d.liquidity_usd, g.liquidity_usd)",
            Some("volume_24h") => "COALESCE(d.volume_24h, g.volume_24h)",
            Some("fdv") => "COALESCE(d.fdv, g.fdv)",
            Some("market_cap") => "COALESCE(d.market_cap, g.market_cap)",
            Some("price_change_h1") => "COALESCE(d.price_change_1h, g.price_change_1h)",
            Some("price_change_h24") => "COALESCE(d.price_change_24h, g.price_change_24h)",
            _ => "t.updated_at", // default
        };

        let direction = match sort_direction {
            Some("asc") => "ASC",
            Some("desc") => "DESC",
            _ => "DESC", // default
        };

        // Build query (always join market tables so we can populate Token fields consistently)
        let select_base = r#"
            SELECT
                t.mint, t.symbol, t.name, t.decimals, t.created_at, t.updated_at,
                sr.score, sr.rugged,
                bl.reason as blacklist_reason,
                ut.priority,
                d.price_usd, d.price_sol, d.price_native,
                d.price_change_5m, d.price_change_1h, d.price_change_6h, d.price_change_24h,
                d.market_cap, d.fdv, d.liquidity_usd,
                d.volume_5m, d.volume_1h, d.volume_6h, d.volume_24h,
                d.txns_5m_buys, d.txns_5m_sells, d.txns_1h_buys, d.txns_1h_sells,
                d.txns_6h_buys, d.txns_6h_sells, d.txns_24h_buys, d.txns_24h_sells,
                d.fetched_at as d_fetched_at,
                g.price_usd, g.price_sol, g.price_native,
                g.price_change_5m, g.price_change_1h, g.price_change_6h, g.price_change_24h,
                g.market_cap, g.fdv, g.liquidity_usd,
                g.volume_5m, g.volume_1h, g.volume_6h, g.volume_24h,
                g.pool_count, g.reserve_in_usd,
                g.fetched_at as g_fetched_at,
                ut.last_market_update,
                ut.last_decimals_update,
                d.pair_created_at
            FROM tokens t
            LEFT JOIN security_rugcheck sr ON t.mint = sr.mint
            LEFT JOIN blacklist bl ON t.mint = bl.mint
            LEFT JOIN update_tracking ut ON t.mint = ut.mint
            LEFT JOIN market_dexscreener d ON t.mint = d.mint
            LEFT JOIN market_geckoterminal g ON t.mint = g.mint
        "#;

        let query = if limit == 0 {
            format!("{} ORDER BY {} {}", select_base, order_column, direction)
        } else {
            format!(
                "{} ORDER BY {} {} LIMIT {} OFFSET {}",
                select_base, order_column, direction, limit, offset
            )
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        // Parse row data
        let tokens_iter = stmt
            .query_map(params![], |row| {
                let mint: String = row.get(0)?;
                let symbol: Option<String> = row.get(1)?;
                let name: Option<String> = row.get(2)?;
                let decimals: Option<i64> = row.get(3)?;
                let created_at: i64 = row.get(4)?;
                let updated_at: i64 = row.get(5)?;

                // Security data (optional)
                let security_score: Option<i32> = row.get(6)?;
                let is_rugged: bool = row
                    .get::<_, Option<i64>>(7)?
                    .map(|v| v != 0)
                    .unwrap_or(false);

                // Blacklist status
                let is_blacklisted = row.get::<_, Option<String>>(8)?.is_some();

                // Priority
                let priority_value: Option<i32> = row.get(9)?;

                // DexScreener fields 10..=30
                let d_price_usd: Option<f64> = row.get(10)?;
                let d_price_sol: Option<f64> = row.get(11)?;
                let d_price_native: Option<String> = row.get(12)?;
                let d_change_5m: Option<f64> = row.get(13)?;
                let d_change_1h: Option<f64> = row.get(14)?;
                let d_change_6h: Option<f64> = row.get(15)?;
                let d_change_24h: Option<f64> = row.get(16)?;
                let d_market_cap: Option<f64> = row.get(17)?;
                let d_fdv: Option<f64> = row.get(18)?;
                let d_liquidity_usd: Option<f64> = row.get(19)?;
                let d_vol_5m: Option<f64> = row.get(20)?;
                let d_vol_1h: Option<f64> = row.get(21)?;
                let d_vol_6h: Option<f64> = row.get(22)?;
                let d_vol_24h: Option<f64> = row.get(23)?;
                let d_txn_5m_buys: Option<i64> = row.get(24)?;
                let d_txn_5m_sells: Option<i64> = row.get(25)?;
                let d_txn_1h_buys: Option<i64> = row.get(26)?;
                let d_txn_1h_sells: Option<i64> = row.get(27)?;
                let d_txn_6h_buys: Option<i64> = row.get(28)?;
                let d_txn_6h_sells: Option<i64> = row.get(29)?;
                let d_txn_24h_buys: Option<i64> = row.get(30)?;
                let d_txn_24h_sells: Option<i64> = row.get(31)?;
                let d_fetched_at: Option<i64> = row.get(32)?;

                // GeckoTerminal fields 33..=45
                let g_price_usd: Option<f64> = row.get(33)?;
                let g_price_sol: Option<f64> = row.get(34)?;
                let g_price_native: Option<String> = row.get(35)?;
                let g_change_5m: Option<f64> = row.get(36)?;
                let g_change_1h: Option<f64> = row.get(37)?;
                let g_change_6h: Option<f64> = row.get(38)?;
                let g_change_24h: Option<f64> = row.get(39)?;
                let g_market_cap: Option<f64> = row.get(40)?;
                let g_fdv: Option<f64> = row.get(41)?;
                let g_liquidity_usd: Option<f64> = row.get(42)?;
                let g_vol_5m: Option<f64> = row.get(43)?;
                let g_vol_1h: Option<f64> = row.get(44)?;
                let g_vol_6h: Option<f64> = row.get(45)?;
                let g_vol_24h: Option<f64> = row.get(46)?;
                let g_pool_count: Option<i64> = row.get(47)?;
                let g_reserve_in_usd: Option<f64> = row.get(48)?;
                let g_fetched_at: Option<i64> = row.get(49)?;

                // Update tracking and pair creation timestamps
                let last_market_update_ts: Option<i64> = row.get(50)?;
                let last_decimals_update_ts: Option<i64> = row.get(51)?;
                let d_pair_created_at: Option<i64> = row.get(52)?;

                Ok((
                    mint,
                    symbol,
                    name,
                    decimals.map(|d| d as u8),
                    created_at,
                    updated_at,
                    security_score,
                    is_rugged,
                    is_blacklisted,
                    priority_value,
                    // Dex (match SELECT order)
                    d_price_usd,
                    d_price_sol,
                    d_price_native,
                    d_change_5m,
                    d_change_1h,
                    d_change_6h,
                    d_change_24h,
                    d_market_cap,
                    d_fdv,
                    d_liquidity_usd,
                    d_vol_5m,
                    d_vol_1h,
                    d_vol_6h,
                    d_vol_24h,
                    d_txn_5m_buys,
                    d_txn_5m_sells,
                    d_txn_1h_buys,
                    d_txn_1h_sells,
                    d_txn_6h_buys,
                    d_txn_6h_sells,
                    d_txn_24h_buys,
                    d_txn_24h_sells,
                    d_fetched_at,
                    // Gecko (match SELECT order)
                    g_price_usd,
                    g_price_sol,
                    g_price_native,
                    g_change_5m,
                    g_change_1h,
                    g_change_6h,
                    g_change_24h,
                    g_market_cap,
                    g_fdv,
                    g_liquidity_usd,
                    g_vol_5m,
                    g_vol_1h,
                    g_vol_6h,
                    g_vol_24h,
                    g_pool_count,
                    g_reserve_in_usd,
                    g_fetched_at,
                    last_market_update_ts,
                    last_decimals_update_ts,
                    d_pair_created_at,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut tokens = Vec::new();
        for row_result in tokens_iter {
            let (
                mint,
                symbol,
                name,
                decimals,
                created_at,
                updated_at,
                security_score,
                is_rugged,
                is_blacklisted,
                priority_value,
                // Dex fields
                d_price_usd,
                d_price_sol,
                d_price_native,
                d_change_5m,
                d_change_1h,
                d_change_6h,
                d_change_24h,
                d_market_cap,
                d_fdv,
                d_liquidity_usd,
                d_vol_5m,
                d_vol_1h,
                d_vol_6h,
                d_vol_24h,
                d_txn_5m_buys,
                d_txn_5m_sells,
                d_txn_1h_buys,
                d_txn_1h_sells,
                d_txn_6h_buys,
                d_txn_6h_sells,
                d_txn_24h_buys,
                d_txn_24h_sells,
                d_fetched_at,
                // Gecko fields
                g_price_usd,
                g_price_sol,
                g_price_native,
                g_change_5m,
                g_change_1h,
                g_change_6h,
                g_change_24h,
                g_market_cap,
                g_fdv,
                g_liquidity_usd,
                g_vol_5m,
                g_vol_1h,
                g_vol_6h,
                g_vol_24h,
                g_pool_count,
                g_reserve_in_usd,
                g_fetched_at,
                last_market_update_ts,
                last_decimals_update_ts,
                d_pair_created_at,
            ) = row_result.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;

            let created_dt = DateTime::from_timestamp(created_at, 0).unwrap_or_else(|| Utc::now());
            let metadata_updated_dt = last_decimals_update_ts
                .and_then(|ts| DateTime::from_timestamp(ts, 0))
                .or_else(|| DateTime::from_timestamp(updated_at, 0));
            let fallback_fetch_dt = metadata_updated_dt.unwrap_or(created_dt);
            let last_market_update_dt =
                last_market_update_ts.and_then(|ts| DateTime::from_timestamp(ts, 0));
            let dex_pair_created_dt =
                d_pair_created_at.and_then(|ts| DateTime::from_timestamp(ts, 0));

            let priority = priority_value
                .map(Priority::from_value)
                .unwrap_or(Priority::Medium);
            // Determine chosen market source based on config preference then fallback
            let preferred_source =
                crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());
            let dex_available = d_price_sol.is_some() || d_price_usd.is_some();
            let gecko_available = g_price_sol.is_some() || g_price_usd.is_some();

            let (
                data_source,
                fetched_at_dt,
                price_usd,
                price_sol,
                price_native,
                change_5m,
                change_1h,
                change_6h,
                change_24h,
                market_cap,
                fdv,
                liquidity_usd,
                vol_5m,
                vol_1h,
                vol_6h,
                vol_24h,
                tx5b,
                tx5s,
                tx1b,
                tx1s,
                tx6b,
                tx6s,
                tx24b,
                tx24s,
            ) = if preferred_source == "geckoterminal" {
                if gecko_available {
                    (
                        DataSource::GeckoTerminal,
                        g_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(fallback_fetch_dt),
                        g_price_usd.unwrap_or(0.0),
                        g_price_sol.unwrap_or(0.0),
                        g_price_native.unwrap_or_else(|| "0".to_string()),
                        g_change_5m,
                        g_change_1h,
                        g_change_6h,
                        g_change_24h,
                        g_market_cap,
                        g_fdv,
                        g_liquidity_usd,
                        g_vol_5m,
                        g_vol_1h,
                        g_vol_6h,
                        g_vol_24h,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                } else if dex_available {
                    (
                        DataSource::DexScreener,
                        d_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(fallback_fetch_dt),
                        d_price_usd.unwrap_or(0.0),
                        d_price_sol.unwrap_or(0.0),
                        d_price_native.unwrap_or_else(|| "0".to_string()),
                        d_change_5m,
                        d_change_1h,
                        d_change_6h,
                        d_change_24h,
                        d_market_cap,
                        d_fdv,
                        d_liquidity_usd,
                        d_vol_5m,
                        d_vol_1h,
                        d_vol_6h,
                        d_vol_24h,
                        d_txn_5m_buys,
                        d_txn_5m_sells,
                        d_txn_1h_buys,
                        d_txn_1h_sells,
                        d_txn_6h_buys,
                        d_txn_6h_sells,
                        d_txn_24h_buys,
                        d_txn_24h_sells,
                    )
                } else {
                    (
                        DataSource::Unknown,
                        fallback_fetch_dt,
                        0.0,
                        0.0,
                        "0".to_string(),
                        // price changes
                        None,
                        None,
                        None,
                        None,
                        // market metrics
                        None,
                        None,
                        None,
                        // volumes
                        None,
                        None,
                        None,
                        None,
                        // txns (all None for Unknown)
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
            } else {
                if dex_available {
                    (
                        DataSource::DexScreener,
                        d_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(fallback_fetch_dt),
                        d_price_usd.unwrap_or(0.0),
                        d_price_sol.unwrap_or(0.0),
                        d_price_native.unwrap_or_else(|| "0".to_string()),
                        d_change_5m,
                        d_change_1h,
                        d_change_6h,
                        d_change_24h,
                        d_market_cap,
                        d_fdv,
                        d_liquidity_usd,
                        d_vol_5m,
                        d_vol_1h,
                        d_vol_6h,
                        d_vol_24h,
                        d_txn_5m_buys,
                        d_txn_5m_sells,
                        d_txn_1h_buys,
                        d_txn_1h_sells,
                        d_txn_6h_buys,
                        d_txn_6h_sells,
                        d_txn_24h_buys,
                        d_txn_24h_sells,
                    )
                } else if gecko_available {
                    (
                        DataSource::GeckoTerminal,
                        g_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(fallback_fetch_dt),
                        g_price_usd.unwrap_or(0.0),
                        g_price_sol.unwrap_or(0.0),
                        g_price_native.unwrap_or_else(|| "0".to_string()),
                        g_change_5m,
                        g_change_1h,
                        g_change_6h,
                        g_change_24h,
                        g_market_cap,
                        g_fdv,
                        g_liquidity_usd,
                        g_vol_5m,
                        g_vol_1h,
                        g_vol_6h,
                        g_vol_24h,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                } else {
                    (
                        DataSource::Unknown,
                        fallback_fetch_dt,
                        0.0,
                        0.0,
                        "0".to_string(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
            };

            let pool_count = if data_source == DataSource::GeckoTerminal {
                g_pool_count.map(|value| value as u32)
            } else {
                None
            };

            let reserve_in_usd = if data_source == DataSource::GeckoTerminal {
                g_reserve_in_usd
            } else {
                None
            };

            let market_updated_dt = last_market_update_dt.unwrap_or(fetched_at_dt);

            let token = Token {
                // Core Identity & Metadata
                mint: mint.clone(),
                symbol: symbol.unwrap_or_else(|| "UNKNOWN".to_string()),
                name: name.unwrap_or_else(|| "Unknown Token".to_string()),
                decimals: decimals.unwrap_or(9),
                description: None,
                image_url: None,
                header_image_url: None,
                supply: None,

                // Data source & timestamps
                data_source,
                fetched_at: fetched_at_dt,
                updated_at: market_updated_dt,
                created_at: created_dt,
                metadata_updated_at: metadata_updated_dt,
                token_birth_at: dex_pair_created_dt,

                // Price Information
                price_usd,
                price_sol,
                price_native,
                price_change_m5: change_5m,
                price_change_h1: change_1h,
                price_change_h6: change_6h,
                price_change_h24: change_24h,

                // Market Metrics
                market_cap,
                fdv,
                liquidity_usd,

                // Volume Data
                volume_m5: vol_5m,
                volume_h1: vol_1h,
                volume_h6: vol_6h,
                volume_h24: vol_24h,

                // Pool metrics
                pool_count,
                reserve_in_usd,

                // Transaction Activity (only available for DexScreener)
                txns_m5_buys: tx5b,
                txns_m5_sells: tx5s,
                txns_h1_buys: tx1b,
                txns_h1_sells: tx1s,
                txns_h6_buys: tx6b,
                txns_h6_sells: tx6s,
                txns_h24_buys: tx24b,
                txns_h24_sells: tx24s,

                // Social & Links
                websites: vec![],
                socials: vec![],

                // Security Information
                mint_authority: None,
                freeze_authority: None,
                security_score,
                is_rugged,
                token_type: None,
                graph_insiders_detected: None,
                lp_provider_count: None,
                security_risks: vec![],
                total_holders: None,
                top_holders: vec![],
                creator_balance_pct: None,
                transfer_fee_pct: None,
                transfer_fee_max_amount: None,
                transfer_fee_authority: None,

                // Bot-Specific State
                is_blacklisted,
                priority,
                first_seen_at: created_dt,
                last_price_update: fetched_at_dt,
            };

            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Get tokens that have NO market data in either DexScreener or GeckoTerminal
    /// Returns minimal Token objects (Unknown data_source; market fields empty/defaults)
    pub fn get_tokens_without_market_data_paginated(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_direction: Option<&str>,
    ) -> TokenResult<Vec<Token>> {
        self.get_tokens_no_market(limit, offset, sort_by, sort_direction)
    }

    /// Helper to get market data from either source (returns None if not available)
    fn get_optional_market_data(
        &self,
        mint: &str,
    ) -> TokenResult<(Option<MarketDataType>, DataSource)> {
        let preferred_source =
            crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());

        if preferred_source == "geckoterminal" {
            if let Some(data) = self.get_geckoterminal_data(mint)? {
                return Ok((
                    Some(MarketDataType::GeckoTerminal(data)),
                    DataSource::GeckoTerminal,
                ));
            }
            if let Some(data) = self.get_dexscreener_data(mint)? {
                return Ok((
                    Some(MarketDataType::DexScreener(data)),
                    DataSource::DexScreener,
                ));
            }
        } else {
            if let Some(data) = self.get_dexscreener_data(mint)? {
                return Ok((
                    Some(MarketDataType::DexScreener(data)),
                    DataSource::DexScreener,
                ));
            }
            if let Some(data) = self.get_geckoterminal_data(mint)? {
                return Ok((
                    Some(MarketDataType::GeckoTerminal(data)),
                    DataSource::GeckoTerminal,
                ));
            }
        }

        Ok((None, DataSource::Unknown))
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
    fallback_image_url: Option<String>,
    fallback_header_url: Option<String>,
) -> Token {
    let created_dt = DateTime::from_timestamp(metadata.created_at, 0).unwrap_or_else(|| Utc::now());
    let metadata_updated_dt = DateTime::from_timestamp(metadata.updated_at, 0);

    // Capture primary source images (DexScreener provides them) without moving market_data
    let (primary_image_url, primary_header_url) = match &market_data {
        MarketDataType::DexScreener(data) => {
            (data.image_url.clone(), data.header_image_url.clone())
        }
        MarketDataType::GeckoTerminal(data) => (data.image_url.clone(), None),
    };

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
        token_birth_at,
        pool_metrics,
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
                data.pair_created_at,
                (None, None),
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
                None,
                (data.pool_count, data.reserve_in_usd),
            )
        }
    };

    // Extract security data
    let security_ref = security.as_ref();

    let token_type = security_ref.and_then(|sec| sec.token_type.clone());
    let mint_authority = security_ref.and_then(|sec| sec.mint_authority.clone());
    let freeze_authority = security_ref.and_then(|sec| sec.freeze_authority.clone());
    let security_score = security_ref.and_then(|sec| sec.score);
    let is_rugged = security_ref.map(|sec| sec.rugged).unwrap_or(false);
    let security_risks = security_ref
        .map(|sec| sec.risks.clone())
        .unwrap_or_else(Vec::new);
    let top_holders = security_ref
        .map(|sec| sec.top_holders.clone())
        .unwrap_or_else(Vec::new);
    let total_holders = security_ref.and_then(|sec| sec.total_holders);
    let creator_balance_pct = security_ref.and_then(|sec| sec.creator_balance_pct);
    let transfer_fee_pct = security_ref.and_then(|sec| sec.transfer_fee_pct);
    let transfer_fee_max_amount = security_ref.and_then(|sec| sec.transfer_fee_max_amount);
    let transfer_fee_authority = security_ref.and_then(|sec| sec.transfer_fee_authority.clone());
    let graph_insiders_detected = security_ref.and_then(|sec| sec.graph_insiders_detected);
    let lp_provider_count = security_ref.and_then(|sec| sec.total_lp_providers);

    let resolved_decimals = metadata
        .decimals
        .or_else(|| security_ref.and_then(|data| data.token_decimals))
        .unwrap_or(9);

    let token_birth_dt = token_birth_at
        .or(metadata_updated_dt.clone())
        .or(Some(created_dt));

    // For now, only use primary source-provided images. Fallbacks can be added upstream where DB is available.
    let resolved_image_url = primary_image_url.or(fallback_image_url);
    let resolved_header_url = primary_header_url.or(fallback_header_url);

    Token {
        // Core identity
        mint: metadata.mint.clone(),
        symbol: metadata.symbol.unwrap_or_else(|| "UNKNOWN".to_string()),
        name: metadata.name.unwrap_or_else(|| "Unknown Token".to_string()),
        decimals: resolved_decimals,
        description: None,
        image_url: resolved_image_url,
        header_image_url: resolved_header_url,
        supply: None,

        // Data source
        data_source,
        fetched_at,
        updated_at: fetched_at,
        created_at: created_dt,
        metadata_updated_at: metadata_updated_dt.clone(),
        token_birth_at: token_birth_dt,

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

        // Pool metrics
        pool_count: pool_metrics.0,
        reserve_in_usd: pool_metrics.1,

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
        token_type,
        graph_insiders_detected,
        lp_provider_count,
        security_risks,
        total_holders,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,
        transfer_fee_max_amount,
        transfer_fee_authority,

        // Bot-specific state
        is_blacklisted,
        priority,
        first_seen_at: created_dt,
        last_price_update: fetched_at,
    }
}

/// Assemble Token without market data (for tokens discovered but not yet enriched)
fn assemble_token_without_market_data(
    metadata: TokenMetadata,
    security: Option<RugcheckData>,
    is_blacklisted: bool,
    priority: Priority,
    metadata_updated_at_override: Option<DateTime<Utc>>,
    last_market_update: Option<DateTime<Utc>>,
    token_birth_at_override: Option<DateTime<Utc>>,
) -> Token {
    // Extract security data
    let security_ref = security.as_ref();

    let token_type = security_ref.and_then(|sec| sec.token_type.clone());
    let mint_authority = security_ref.and_then(|sec| sec.mint_authority.clone());
    let freeze_authority = security_ref.and_then(|sec| sec.freeze_authority.clone());
    let security_score = security_ref.and_then(|sec| sec.score);
    let is_rugged = security_ref.map(|sec| sec.rugged).unwrap_or(false);
    let security_risks = security_ref
        .map(|sec| sec.risks.clone())
        .unwrap_or_else(Vec::new);
    let top_holders = security_ref
        .map(|sec| sec.top_holders.clone())
        .unwrap_or_else(Vec::new);
    let total_holders = security_ref.and_then(|sec| sec.total_holders);
    let creator_balance_pct = security_ref.and_then(|sec| sec.creator_balance_pct);
    let transfer_fee_pct = security_ref.and_then(|sec| sec.transfer_fee_pct);
    let transfer_fee_max_amount = security_ref.and_then(|sec| sec.transfer_fee_max_amount);
    let transfer_fee_authority = security_ref.and_then(|sec| sec.transfer_fee_authority.clone());
    let graph_insiders_detected = security_ref.and_then(|sec| sec.graph_insiders_detected);
    let lp_provider_count = security_ref.and_then(|sec| sec.total_lp_providers);

    let created_at = DateTime::from_timestamp(metadata.created_at, 0).unwrap_or_else(|| Utc::now());
    let metadata_updated_dt =
        metadata_updated_at_override.or_else(|| DateTime::from_timestamp(metadata.updated_at, 0));
    let market_updated_dt = last_market_update
        .or_else(|| metadata_updated_dt.clone())
        .unwrap_or(created_at);
    let token_birth_dt = token_birth_at_override
        .or_else(|| metadata_updated_dt.clone())
        .or(Some(created_at));

    let resolved_decimals = metadata
        .decimals
        .or_else(|| security_ref.and_then(|data| data.token_decimals))
        .unwrap_or(9);

    Token {
        // Core Identity & Metadata
        mint: metadata.mint.clone(),
        symbol: metadata.symbol.unwrap_or_else(|| "UNKNOWN".to_string()),
        name: metadata.name.unwrap_or_else(|| "Unknown Token".to_string()),
        decimals: resolved_decimals, // Default to 9 if unknown
        description: None,
        image_url: None,
        header_image_url: None,
        supply: None,

        // Data source
        data_source: DataSource::Unknown,
        fetched_at: market_updated_dt,
        updated_at: market_updated_dt,
        created_at,
        metadata_updated_at: metadata_updated_dt,
        token_birth_at: token_birth_dt,

        // Price Information (defaults for missing market data)
        price_usd: 0.0,
        price_sol: 0.0,
        price_native: "0".to_string(),
        price_change_m5: None,
        price_change_h1: None,
        price_change_h6: None,
        price_change_h24: None,

        // Market Metrics
        market_cap: None,
        fdv: None,
        liquidity_usd: None,

        // Volume Data
        volume_m5: None,
        volume_h1: None,
        volume_h6: None,
        volume_h24: None,

        // Pool metrics
        pool_count: None,
        reserve_in_usd: None,

        // Transaction Activity
        txns_m5_buys: None,
        txns_m5_sells: None,
        txns_h1_buys: None,
        txns_h1_sells: None,
        txns_h6_buys: None,
        txns_h6_sells: None,
        txns_h24_buys: None,
        txns_h24_sells: None,

        // Social & Links
        websites: vec![],
        socials: vec![],

        // Security Information
        mint_authority,
        freeze_authority,
        security_score,
        is_rugged,
        token_type,
        graph_insiders_detected,
        lp_provider_count,
        security_risks,
        total_holders,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,
        transfer_fee_max_amount,
        transfer_fee_authority,

        // Bot-Specific State
        is_blacklisted,
        priority,
        first_seen_at: created_at,
        last_price_update: created_at,
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
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || db_clone.get_full_token(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for get_full_token_for_source (returns complete Token with specific source)
pub async fn get_full_token_for_source_async(
    mint: &str,
    source: DataSource,
) -> TokenResult<Option<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    let mint = mint.to_string();
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || db_clone.get_full_token_for_source(&mint, source))
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

/// Async wrapper to count total tokens in database (fast, no data loading)
pub async fn count_tokens_async() -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || {
        let conn = db
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let count: usize = conn
            .query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Count query failed: {}", e)))?;

        Ok(count)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for get_all_tokens_optional_market (returns Vec<Token> with optional market data)
pub async fn get_all_tokens_optional_market_async(
    limit: usize,
    offset: usize,
    sort_by: Option<String>,
    sort_direction: Option<String>,
) -> TokenResult<Vec<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || {
        db.get_all_tokens_optional_market(
            limit,
            offset,
            sort_by.as_deref(),
            sort_direction.as_deref(),
        )
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: count tokens with no market
pub async fn count_tokens_no_market_async() -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.count_tokens_no_market())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: get tokens with no market
pub async fn get_tokens_no_market_async(
    limit: usize,
    offset: usize,
    sort_by: Option<String>,
    sort_direction: Option<String>,
) -> TokenResult<Vec<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || {
        db.get_tokens_no_market(limit, offset, sort_by.as_deref(), sort_direction.as_deref())
    })
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
