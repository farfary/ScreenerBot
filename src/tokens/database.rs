/// Unified database operations for tokens system
/// All SQL operations in one place with proper error handling
use chrono::{DateTime, Utc};
use rusqlite::types::FromSql;
use rusqlite::{params, params_from_iter, Connection, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::logger::{self, LogTag};
use crate::tokens::pools;
use crate::tokens::store;
use crate::tokens::types::{
    DataSource, DexScreenerData, GeckoTerminalData, Priority, RugcheckData, SecurityRisk,
    SocialLink, Token, TokenError, TokenHolder, TokenMetadata, TokenPoolInfo, TokenPoolSources,
    TokenPoolsSnapshot, TokenResult, UpdateTrackingInfo, WebsiteLink,
};

// Global database instance for easy access
static GLOBAL_DB: std::sync::Mutex<Option<Arc<TokenDatabase>>> = std::sync::Mutex::new(None);

/// Initialize global database (called by service)
pub fn init_global_database(db: Arc<TokenDatabase>) -> Result<(), String> {
    let mut guard = GLOBAL_DB
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))?;
    *guard = Some(db);
    Ok(())
}

/// Get global database instance
pub fn get_global_database() -> Option<Arc<TokenDatabase>> {
    GLOBAL_DB.lock().ok()?.clone()
}

/// Clear global database (called on service restart)
pub fn clear_global_database() {
    if let Ok(mut guard) = GLOBAL_DB.lock() {
        *guard = None;
    }
}

/// Token database with connection pool
pub struct TokenDatabase {
    conn: Arc<Mutex<Connection>>,
}

/// Token-level blacklist entry with metadata for diagnostics and UI
#[derive(Debug, Clone)]
pub struct TokenBlacklistRecord {
    pub mint: String,
    pub reason: String,
    pub source: String,
    pub added_at: i64,
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
        let mut conn = self
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

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM security_rugcheck WHERE mint = ?1",
                params![mint],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count == 0)
                },
            )
            .unwrap_or(true);

        let now_ts = data.security_data_last_fetched_at.timestamp();
        let first_fetched_ts = if is_first_insert {
            now_ts
        } else {
            // Preserve existing first_fetched_at on updates
            conn.query_row(
                "SELECT security_data_first_fetched_at FROM security_rugcheck WHERE mint = ?1",
                params![mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        let is_mutable_flag = data.is_mutable.map(|b| if b { 1 } else { 0 });

        conn.execute(
            "INSERT INTO security_rugcheck (
                mint,
                token_type,
                token_decimals,
                score,
                score_normalised,
                score_description,
                mint_authority,
                freeze_authority,
                update_authority,
                is_mutable,
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
                security_data_last_fetched_at,
                security_data_first_fetched_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27
             )
             ON CONFLICT(mint) DO UPDATE SET
                token_type = excluded.token_type,
                token_decimals = excluded.token_decimals,
                score = excluded.score,
                score_normalised = excluded.score_normalised,
                score_description = excluded.score_description,
                mint_authority = excluded.mint_authority,
                freeze_authority = excluded.freeze_authority,
                update_authority = excluded.update_authority,
                is_mutable = excluded.is_mutable,
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
                security_data_last_fetched_at = excluded.security_data_last_fetched_at",
            params![
                mint,
                &data.token_type,
                data.token_decimals,
                data.score,
                data.score_normalised,
                &data.score_description,
                &data.mint_authority,
                &data.freeze_authority,
                &data.update_authority,
                is_mutable_flag,
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
                now_ts,
                first_fetched_ts,
            ],
        )
        .map_err(|e| TokenError::Database(format!("Failed to upsert Rugcheck data: {}", e)))?;

        // Update in-memory cache
        store::store_rugcheck(mint, data);

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
            Some("market_data_last_fetched_at") => {
                "COALESCE(ut.market_data_last_updated_at, t.metadata_last_fetched_at)"
            }
            Some("first_discovered_at") => "t.first_discovered_at",
            Some("metadata_last_fetched_at") => "t.metadata_last_fetched_at",
            Some("blockchain_created_at") => {
                "COALESCE(t.blockchain_created_at, t.first_discovered_at)"
            }
            Some("pool_price_last_calculated_at") => {
                "COALESCE(ut.pool_price_last_calculated_at, t.metadata_last_fetched_at)"
            }
            Some("mint") => "t.mint",
            Some("risk_score") => "sr.score",
            _ => "COALESCE(ut.market_data_last_updated_at, t.metadata_last_fetched_at)",
        };
        let direction = match sort_direction {
            Some("asc") => "ASC",
            Some("desc") => "DESC",
            _ => "DESC",
        };

        let base = "SELECT \
                        t.mint, t.symbol, t.name, t.decimals, t.first_discovered_at, \
                        t.metadata_last_fetched_at, \
                        ut.market_data_last_updated_at, \
                        sr.score, sr.rugged, \
                        bl.reason as blacklist_reason, \
                        ut.priority, \
                        t.blockchain_created_at, \
                        sr.security_data_last_fetched_at, \
                        ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at \
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
                    first_discovered_at: row.get::<_, i64>(4)?,
                    metadata_last_fetched_at: row.get::<_, i64>(5)?,
                };
                let last_market_update: Option<i64> = row.get(6)?;
                let security_score: Option<i32> = row.get(7)?;
                let is_rugged: bool = row
                    .get::<_, Option<i64>>(8)?
                    .map(|v| v != 0)
                    .unwrap_or(false);
                let is_blacklisted = row.get::<_, Option<String>>(9)?.is_some();
                let priority_value: Option<i32> = row.get(10)?;
                let blockchain_created_at: Option<i64> = row.get(11)?;
                let security_data_last_fetched_at: Option<i64> = row.get(12)?;

                // Rejection tracking fields
                let last_rejection_reason: Option<String> = row.get(13)?;
                let last_rejection_source: Option<String> = row.get(14)?;
                let last_rejection_at: Option<i64> = row.get(15)?;

                Ok((
                    metadata,
                    last_market_update,
                    security_score,
                    is_rugged,
                    is_blacklisted,
                    priority_value,
                    blockchain_created_at,
                    security_data_last_fetched_at,
                    last_rejection_reason,
                    last_rejection_source,
                    last_rejection_at,
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
                blockchain_created_at,
                security_data_last_fetched_at,
                last_rejection_reason,
                last_rejection_source,
                last_rejection_at,
            ) = row.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;

            // Parse rejection timestamp
            let last_rejection_at_dt =
                last_rejection_at.and_then(|ts| DateTime::from_timestamp(ts, 0));

            // Build a RugcheckData-lite only for values we expose directly
            let security = if security_score.is_some() || is_rugged {
                let security_ts = security_data_last_fetched_at
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .unwrap_or_else(|| Utc::now());

                Some(RugcheckData {
                    token_type: None,
                    token_decimals: None,
                    score: security_score,
                    score_normalised: None, // Not loaded in this lite version
                    score_description: None,
                    mint_authority: None,
                    freeze_authority: None,
                    update_authority: None,
                    is_mutable: None,
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
                    security_data_last_fetched_at: security_ts,
                    security_data_first_fetched_at: security_ts, // Same for this fallback case
                })
            } else {
                None
            };

            let priority = priority_value
                .map(Priority::from_value)
                .unwrap_or(Priority::Standard);

            let blockchain_created_dt =
                blockchain_created_at.and_then(|ts| DateTime::from_timestamp(ts, 0));

            let token = assemble_token_without_market_data(
                meta,
                security,
                is_blacklisted,
                priority,
                None,
                last_market_update.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                blockchain_created_dt,
                last_rejection_reason,
                last_rejection_source,
                last_rejection_at_dt,
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
            "INSERT INTO tokens (mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?5)
             ON CONFLICT(mint) DO UPDATE SET
                symbol = COALESCE(?2, symbol),
                name = COALESCE(?3, name),
                decimals = COALESCE(?4, decimals),
                metadata_last_fetched_at = ?5,
                decimals_last_fetched_at = CASE WHEN ?4 IS NOT NULL THEN ?5 ELSE decimals_last_fetched_at END",
            params![mint, symbol, name, decimals.map(|d| d as i64), now],
        )
        .map_err(|e| TokenError::Database(format!("Failed to upsert token: {}", e)))?;

        // Ensure tracking entry exists
        conn.execute(
            "INSERT OR IGNORE INTO update_tracking (mint, priority) VALUES (?1, 10)",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to create tracking: {}", e)))?;

        // CRITICAL: Update in-memory cache immediately after successful DB write
        // This ensures the cache stays synchronized with the database
        // Pool decoders rely on cached decimals being available
        if let Some(d) = decimals {
            if d > 0 {
                crate::tokens::decimals::cache(mint, d);
            }
        }

        Ok(())
    }

    /// Get token metadata
    pub fn get_token(&self, mint: &str) -> TokenResult<Option<TokenMetadata>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at FROM tokens WHERE mint = ?1"
        ).map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            Ok(TokenMetadata {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get::<_, Option<i64>>(3)?.map(|d| d as u8),
                first_discovered_at: row.get(4)?,
                metadata_last_fetched_at: row.get(5)?,
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
                "SELECT mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at 
             FROM tokens 
             ORDER BY metadata_last_fetched_at DESC 
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
                    first_discovered_at: row.get(4)?,
                    metadata_last_fetched_at: row.get(5)?,
                })
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        tokens
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))
    }

    /// Get all tokens with valid decimals for cache preloading
    /// Used at startup to populate in-memory decimals cache
    pub fn get_all_tokens_with_decimals(&self) -> TokenResult<Vec<(String, u8)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // First check how many tokens exist
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens WHERE decimals IS NOT NULL AND decimals > 0",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        crate::logger::debug(
            crate::logger::LogTag::Tokens,
            &format!(
                "[PRELOAD] Database query found {} tokens with decimals",
                count
            ),
        );

        let mut stmt = conn
            .prepare(
                "SELECT mint, decimals FROM tokens WHERE decimals IS NOT NULL AND decimals > 0",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let mint: String = row.get(0)?;
                let decimals: i64 = row.get(1)?;
                Ok((mint, decimals as u8))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let result = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {}", e)))?;

        crate::logger::debug(
            crate::logger::LogTag::Tokens,
            &format!(
                "[PRELOAD] Successfully collected {} decimals from database",
                result.len()
            ),
        );

        Ok(result)
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

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM market_dexscreener WHERE mint = ?1",
                params![mint],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count == 0)
                },
            )
            .unwrap_or(true);

        let now_ts = data.market_data_last_fetched_at.timestamp();
        let first_fetched_ts = if is_first_insert {
            now_ts
        } else {
            // Preserve existing first_fetched_at on updates
            conn.query_row(
                "SELECT market_data_first_fetched_at FROM market_dexscreener WHERE mint = ?1",
                params![mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        conn.execute(
            "INSERT INTO market_dexscreener (
                mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                txns_5m_buys, txns_5m_sells, txns_1h_buys, txns_1h_sells,
                txns_6h_buys, txns_6h_sells, txns_24h_buys, txns_24h_sells,
                pair_address, chain_id, dex_id, url, pair_blockchain_created_at, image_url, header_image_url,
                market_data_last_fetched_at, market_data_first_fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                       ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32)
             ON CONFLICT(mint) DO UPDATE SET
                price_usd = ?2, price_sol = ?3, price_native = ?4,
                price_change_5m = ?5, price_change_1h = ?6, price_change_6h = ?7, price_change_24h = ?8,
                market_cap = ?9, fdv = ?10, liquidity_usd = ?11,
                volume_5m = ?12, volume_1h = ?13, volume_6h = ?14, volume_24h = ?15,
                txns_5m_buys = ?16, txns_5m_sells = ?17, txns_1h_buys = ?18, txns_1h_sells = ?19,
                txns_6h_buys = ?20, txns_6h_sells = ?21, txns_24h_buys = ?22, txns_24h_sells = ?23,
                pair_address = ?24, chain_id = ?25, dex_id = ?26, url = ?27, pair_blockchain_created_at = ?28,
                image_url = ?29, header_image_url = ?30, market_data_last_fetched_at = ?31",
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
                data.pair_blockchain_created_at.map(|dt| dt.timestamp()),
                &data.image_url,
                &data.header_image_url,
                now_ts,
                first_fetched_ts,
            ],
        ).map_err(|e| TokenError::Database(format!("Failed to upsert DexScreener data: {}", e)))?;

        // Update in-memory cache
        store::store_dexscreener(mint, data);

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
                    pair_address, chain_id, dex_id, url, pair_blockchain_created_at, image_url, header_image_url,
                    market_data_last_fetched_at, market_data_first_fetched_at
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
            let pair_blockchain_created_ts: Option<i64> = row.get(26)?;
            let image_url: Option<String> = row.get(27)?;
            let header_image_url: Option<String> = row.get(28)?;
            let last_fetched_ts: i64 = row.get(29)?;
            let first_fetched_ts: i64 = row.get(30)?;

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
                pair_blockchain_created_at: pair_blockchain_created_ts
                    .and_then(|ts| DateTime::from_timestamp(ts, 0)),
                image_url,
                header_image_url,
                market_data_last_fetched_at: DateTime::from_timestamp(last_fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                market_data_first_fetched_at: DateTime::from_timestamp(first_fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
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

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM market_geckoterminal WHERE mint = ?1",
                params![mint],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count == 0)
                },
            )
            .unwrap_or(true);

        let now_ts = data.market_data_last_fetched_at.timestamp();
        let first_fetched_ts = if is_first_insert {
            now_ts
        } else {
            // Preserve existing first_fetched_at on updates
            conn.query_row(
                "SELECT market_data_first_fetched_at FROM market_geckoterminal WHERE mint = ?1",
                params![mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        // Clean schema insert (image_url column included)
        let insert_result = conn.execute(
            "INSERT INTO market_geckoterminal (
                mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                pool_count, top_pool_address, reserve_in_usd, image_url,
                market_data_last_fetched_at, market_data_first_fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
             ON CONFLICT(mint) DO UPDATE SET
                price_usd = ?2, price_sol = ?3, price_native = ?4,
                price_change_5m = ?5, price_change_1h = ?6, price_change_6h = ?7, price_change_24h = ?8,
                market_cap = ?9, fdv = ?10, liquidity_usd = ?11,
                volume_5m = ?12, volume_1h = ?13, volume_6h = ?14, volume_24h = ?15,
                pool_count = ?16, top_pool_address = ?17, reserve_in_usd = ?18, image_url = ?19, market_data_last_fetched_at = ?20",
            params![
                mint, data.price_usd, data.price_sol, &data.price_native,
                data.price_change_5m, data.price_change_1h, data.price_change_6h, data.price_change_24h,
                data.market_cap, data.fdv, data.liquidity_usd,
                data.volume_5m, data.volume_1h, data.volume_6h, data.volume_24h,
                data.pool_count.map(|c| c as i64), &data.top_pool_address, data.reserve_in_usd,
                &data.image_url, now_ts, first_fetched_ts,
            ],
        );

        insert_result.map_err(|e| {
            TokenError::Database(format!("Failed to upsert GeckoTerminal data: {}", e))
        })?;

        // Update in-memory cache
        store::store_geckoterminal(mint, data);

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
                    pool_count, top_pool_address, reserve_in_usd, image_url,
                    market_data_last_fetched_at, market_data_first_fetched_at
             FROM market_geckoterminal WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            let last_fetched_ts: i64 = row.get(18)?;
            let first_fetched_ts: i64 = row.get(19)?;

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
                market_data_last_fetched_at: DateTime::from_timestamp(last_fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                market_data_first_fetched_at: DateTime::from_timestamp(first_fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    // ========================================================================
    // BATCH IMAGE OPERATIONS (performance-optimized)
    // ========================================================================

    /// Get image URLs for multiple tokens in a single query
    /// Returns HashMap<mint, image_url> - only includes mints that have images
    /// Prioritizes DexScreener images, falls back to GeckoTerminal
    pub fn get_token_images_batch(&self, mints: &[String]) -> TokenResult<HashMap<String, String>> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // Build placeholders for IN clause
        let placeholders: String = mints.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        // Query: DexScreener images first, then GeckoTerminal for any missing
        // Uses UNION to combine results, with DexScreener taking priority
        let query = format!(
            r#"
            SELECT mint, image_url FROM market_dexscreener 
            WHERE mint IN ({}) AND image_url IS NOT NULL AND image_url != ''
            UNION ALL
            SELECT g.mint, g.image_url FROM market_geckoterminal g
            WHERE g.mint IN ({}) 
              AND g.image_url IS NOT NULL AND g.image_url != ''
              AND g.mint NOT IN (
                SELECT mint FROM market_dexscreener 
                WHERE mint IN ({}) AND image_url IS NOT NULL AND image_url != ''
              )
            "#,
            placeholders, placeholders, placeholders
        );

        let mut stmt = conn.prepare(&query).map_err(|e| {
            TokenError::Database(format!("Failed to prepare batch image query: {}", e))
        })?;

        // Build params: mints repeated 3 times for the 3 IN clauses
        let all_mints: Vec<&str> = mints
            .iter()
            .chain(mints.iter())
            .chain(mints.iter())
            .map(|s| s.as_str())
            .collect();

        let rows = stmt
            .query_map(params_from_iter(all_mints), |row| {
                let mint: String = row.get(0)?;
                let image_url: String = row.get(1)?;
                Ok((mint, image_url))
            })
            .map_err(|e| TokenError::Database(format!("Batch image query failed: {}", e)))?;

        let mut result = HashMap::with_capacity(mints.len());
        for row in rows {
            let (mint, image_url) =
                row.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;
            result.insert(mint, image_url);
        }

        Ok(result)
    }

    /// Get basic token info (symbol, name, image_url) for multiple tokens in a single query
    /// Returns HashMap<mint, (symbol, name, image_url)> - optimized for display purposes
    pub fn get_token_info_batch(
        &self,
        mints: &[String],
    ) -> TokenResult<HashMap<String, (Option<String>, Option<String>, Option<String>)>> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let placeholders: String = mints.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        // Join tokens table with market data to get symbol, name, and image
        // Priority: DexScreener image > GeckoTerminal image
        let query = format!(
            r#"
            SELECT 
                t.mint,
                t.symbol,
                t.name,
                COALESCE(d.image_url, g.image_url) as image_url
            FROM tokens t
            LEFT JOIN market_dexscreener d ON t.mint = d.mint
            LEFT JOIN market_geckoterminal g ON t.mint = g.mint
            WHERE t.mint IN ({})
            "#,
            placeholders
        );

        let mut stmt = conn.prepare(&query).map_err(|e| {
            TokenError::Database(format!("Failed to prepare batch token info query: {}", e))
        })?;

        let mint_refs: Vec<&str> = mints.iter().map(|s| s.as_str()).collect();

        let rows = stmt
            .query_map(params_from_iter(mint_refs), |row| {
                let mint: String = row.get(0)?;
                let symbol: Option<String> = row.get(1)?;
                let name: Option<String> = row.get(2)?;
                let image_url: Option<String> = row.get(3)?;
                Ok((mint, symbol, name, image_url))
            })
            .map_err(|e| TokenError::Database(format!("Batch token info query failed: {}", e)))?;

        let mut result = HashMap::with_capacity(mints.len());
        for row in rows {
            let (mint, symbol, name, image_url) =
                row.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;
            result.insert(mint, (symbol, name, image_url));
        }

        Ok(result)
    }

    // ========================================================================
    // POOL DATA OPERATIONS
    // ========================================================================

    /// Replace all stored pool records for a token with the provided snapshot
    pub fn replace_token_pools(&self, snapshot: &TokenPoolsSnapshot) -> TokenResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Failed to start transaction: {}", e)))?;

        // Query existing first_seen_ts values BEFORE delete to preserve them
        let mut existing_first_seen: HashMap<String, i64> = HashMap::new();
        {
            let mut stmt = tx
                .prepare(
                    "SELECT pool_address, pool_data_first_seen_at FROM token_pools WHERE mint = ?1",
                )
                .map_err(|e| TokenError::Database(format!("Failed to prepare query: {}", e)))?;

            let rows = stmt
                .query_map(params![&snapshot.mint], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|e| {
                    TokenError::Database(format!("Failed to query existing pools: {}", e))
                })?;

            for row in rows {
                if let Ok((pool_addr, ts)) = row {
                    existing_first_seen.insert(pool_addr, ts);
                }
            }
        }

        tx.execute(
            "DELETE FROM token_pools WHERE mint = ?1",
            params![&snapshot.mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to clear token pools: {}", e)))?;

        for pool in snapshot.pools.iter() {
            let sources_json = serde_json::to_string(&pool.sources).map_err(|e| {
                TokenError::Database(format!("Failed to serialize pool sources: {}", e))
            })?;

            // Use preserved first_seen_ts or fall back to current timestamp
            let first_seen_ts = existing_first_seen
                .get(&pool.pool_address)
                .copied()
                .unwrap_or_else(|| pool.pool_data_last_fetched_at.timestamp());

            tx.execute(
                "INSERT INTO token_pools (
                    mint, pool_address, dex, base_mint, quote_mint, is_sol_pair,
                    liquidity_usd, liquidity_token, liquidity_sol, volume_h24,
                    price_usd, price_sol, price_native, sources_json,
                    pool_data_last_fetched_at, pool_data_first_seen_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    &snapshot.mint,
                    &pool.pool_address,
                    &pool.dex,
                    &pool.base_mint,
                    &pool.quote_mint,
                    if pool.is_sol_pair { 1 } else { 0 },
                    pool.liquidity_usd,
                    pool.liquidity_token,
                    pool.liquidity_sol,
                    pool.volume_h24,
                    pool.price_usd,
                    pool.price_sol,
                    &pool.price_native,
                    sources_json,
                    pool.pool_data_last_fetched_at.timestamp(),
                    first_seen_ts,
                ],
            )
            .map_err(|e| TokenError::Database(format!("Failed to insert token pool: {}", e)))?;
        }

        tx.commit().map_err(|e| {
            TokenError::Database(format!("Failed to commit pool transaction: {}", e))
        })?;

        Ok(())
    }

    /// Load pool snapshot for a token (if any pools stored)
    pub fn get_token_pools(&self, mint: &str) -> TokenResult<Option<TokenPoolsSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, dex, base_mint, quote_mint, is_sol_pair,
                        liquidity_usd, liquidity_token, liquidity_sol, volume_h24,
                        price_usd, price_sol, price_native, sources_json,
                        pool_data_last_fetched_at, pool_data_first_seen_at
                 FROM token_pools WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mut rows = stmt
            .query(params![mint])
            .map_err(|e| TokenError::Database(format!("Failed to query pools: {}", e)))?;

        let mut pools: Vec<TokenPoolInfo> = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| TokenError::Database(format!("Failed to read row: {}", e)))?
        {
            let sources_json: Option<String> = read_row_value(&row, 12, "sources_json")?;
            let sources = match sources_json {
                Some(json) if !json.is_empty() => {
                    serde_json::from_str::<TokenPoolSources>(&json).unwrap_or_default()
                }
                _ => TokenPoolSources::default(),
            };
            let last_fetched_ts: i64 = read_row_value(&row, 13, "pool_data_last_fetched_at")?;
            let first_seen_ts: i64 = read_row_value(&row, 14, "pool_data_first_seen_at")?;
            let pool_address: String = read_row_value(&row, 0, "pool_address")?;
            let dex: Option<String> = read_row_value(&row, 1, "dex")?;
            let base_mint: String = read_row_value(&row, 2, "base_mint")?;
            let quote_mint: String = read_row_value(&row, 3, "quote_mint")?;
            let is_sol_pair_flag: i64 = read_row_value(&row, 4, "is_sol_pair")?;
            let liquidity_usd: Option<f64> = read_row_value(&row, 5, "liquidity_usd")?;
            let liquidity_token: Option<f64> = read_row_value(&row, 6, "liquidity_token")?;
            let liquidity_sol: Option<f64> = read_row_value(&row, 7, "liquidity_sol")?;
            let volume_h24: Option<f64> = read_row_value(&row, 8, "volume_h24")?;
            let price_usd: Option<f64> = read_row_value(&row, 9, "price_usd")?;
            let price_sol: Option<f64> = read_row_value(&row, 10, "price_sol")?;
            let price_native: Option<String> = read_row_value(&row, 11, "price_native")?;

            pools.push(TokenPoolInfo {
                pool_address,
                dex,
                base_mint,
                quote_mint,
                is_sol_pair: is_sol_pair_flag != 0,
                liquidity_usd,
                liquidity_token,
                liquidity_sol,
                volume_h24,
                price_usd,
                price_sol,
                price_native,
                sources,
                pool_data_last_fetched_at: DateTime::from_timestamp(last_fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                pool_data_first_seen_at: DateTime::from_timestamp(first_seen_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
            });
        }

        if pools.is_empty() {
            return Ok(None);
        }

        let pool_data_last_fetched_at = pools
            .iter()
            .map(|p| p.pool_data_last_fetched_at)
            .max()
            .unwrap_or_else(|| Utc::now());
        let canonical_pool_address = pools::choose_canonical_pool(&pools);

        Ok(Some(TokenPoolsSnapshot {
            mint: mint.to_string(),
            pools,
            canonical_pool_address,
            pool_data_last_fetched_at,
        }))
    }

    /// Load all token pool snapshots (used for cache warmup at startup)
    pub fn get_all_token_pools(&self) -> TokenResult<Vec<TokenPoolsSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, pool_address, dex, base_mint, quote_mint, is_sol_pair,
                        liquidity_usd, liquidity_token, liquidity_sol, volume_h24,
                        price_usd, price_sol, price_native, sources_json,
                        pool_data_last_fetched_at, pool_data_first_seen_at
                 FROM token_pools ORDER BY mint",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mut rows = stmt
            .query([])
            .map_err(|e| TokenError::Database(format!("Failed to query pools: {}", e)))?;

        let mut snapshots: Vec<TokenPoolsSnapshot> = Vec::new();
        let mut current_mint: Option<String> = None;
        let mut current_pools: Vec<TokenPoolInfo> = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| TokenError::Database(format!("Failed to read row: {}", e)))?
        {
            let mint: String = read_row_value(&row, 0, "mint")?;
            if current_mint.as_ref() != Some(&mint) && !current_pools.is_empty() {
                let pool_data_last_fetched_at = current_pools
                    .iter()
                    .map(|p| p.pool_data_last_fetched_at)
                    .max()
                    .unwrap_or_else(|| Utc::now());
                let canonical_pool_address = pools::choose_canonical_pool(&current_pools);

                snapshots.push(TokenPoolsSnapshot {
                    mint: current_mint.take().unwrap(),
                    pools: std::mem::take(&mut current_pools),
                    canonical_pool_address,
                    pool_data_last_fetched_at,
                });
            }

            current_mint = Some(mint.clone());

            let sources_json: Option<String> = read_row_value(&row, 13, "sources_json")?;
            let sources = match sources_json {
                Some(json) if !json.is_empty() => {
                    serde_json::from_str::<TokenPoolSources>(&json).unwrap_or_default()
                }
                _ => TokenPoolSources::default(),
            };
            let last_fetched_ts: i64 = read_row_value(&row, 14, "pool_data_last_fetched_at")?;
            let first_seen_ts: i64 = read_row_value(&row, 15, "pool_data_first_seen_at")?;

            current_pools.push(TokenPoolInfo {
                pool_address: read_row_value(&row, 1, "pool_address")?,
                dex: read_row_value(&row, 2, "dex")?,
                base_mint: read_row_value(&row, 3, "base_mint")?,
                quote_mint: read_row_value(&row, 4, "quote_mint")?,
                is_sol_pair: read_row_value::<i64>(&row, 5, "is_sol_pair")? != 0,
                liquidity_usd: read_row_value(&row, 6, "liquidity_usd")?,
                liquidity_token: read_row_value(&row, 7, "liquidity_token")?,
                liquidity_sol: read_row_value(&row, 8, "liquidity_sol")?,
                volume_h24: read_row_value(&row, 9, "volume_h24")?,
                price_usd: read_row_value(&row, 10, "price_usd")?,
                price_sol: read_row_value(&row, 11, "price_sol")?,
                price_native: read_row_value(&row, 12, "price_native")?,
                sources,
                pool_data_last_fetched_at: DateTime::from_timestamp(last_fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                pool_data_first_seen_at: DateTime::from_timestamp(first_seen_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
            });
        }

        if let Some(mint) = current_mint.take() {
            if !current_pools.is_empty() {
                let pool_data_last_fetched_at = current_pools
                    .iter()
                    .map(|p| p.pool_data_last_fetched_at)
                    .max()
                    .unwrap_or_else(|| Utc::now());
                let canonical_pool_address = pools::choose_canonical_pool(&current_pools);

                snapshots.push(TokenPoolsSnapshot {
                    mint,
                    pools: std::mem::take(&mut current_pools),
                    canonical_pool_address,
                    pool_data_last_fetched_at,
                });
            }
        }

        Ok(snapshots)
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
                    score_normalised,
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
                    security_data_last_fetched_at,
                    security_data_first_fetched_at,
                    update_authority,
                    is_mutable
                 FROM security_rugcheck WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result = stmt.query_row(params![mint], |row| {
            let risks_json: String = row.get(19)?;
            let holders_json: String = row.get(20)?;
            let markets_json: Option<String> = row.get(21)?;
            let fetched_ts: i64 = row.get(22)?;
            let first_fetched_ts: i64 = row.get(23)?;
            let rugged_flag: Option<i64> = row.get(18)?;
            let is_rugged = rugged_flag.unwrap_or(0) != 0;
            let is_mutable_flag: Option<i64> = row.get(25)?;
            let is_mutable = is_mutable_flag.map(|f| f != 0);

            let risks: Vec<SecurityRisk> = serde_json::from_str(&risks_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let holders: Vec<TokenHolder> = serde_json::from_str(&holders_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let markets = markets_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(RugcheckData {
                token_type: row.get(0)?,
                token_decimals: row.get(1)?,
                score: row.get(2)?,
                score_normalised: row.get(3)?,
                score_description: row.get(4)?,
                mint_authority: row.get(5)?,
                freeze_authority: row.get(6)?,
                update_authority: row.get(24)?,
                is_mutable,
                top_10_holders_pct: row.get(7)?,
                total_supply: row.get(8)?,
                total_holders: row.get(9)?,
                total_lp_providers: row.get(10)?,
                graph_insiders_detected: row.get(11)?,
                total_market_liquidity: row.get(12)?,
                total_stable_liquidity: row.get(13)?,
                creator_balance_pct: row.get(14)?,
                transfer_fee_pct: row.get(15)?,
                transfer_fee_max_amount: row.get(16)?,
                transfer_fee_authority: row.get(17)?,
                rugged: is_rugged,
                risks,
                top_holders: holders,
                markets,
                security_data_last_fetched_at: DateTime::from_timestamp(fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                security_data_first_fetched_at: DateTime::from_timestamp(first_fetched_ts, 0)
                    .unwrap_or_else(|| {
                        DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now())
                    }),
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

    /// Get tokens by priority with limit (excludes permanently failed market data tokens)
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
                 AND (market_error_type IS NULL OR market_error_type != 'permanent')
                 ORDER BY market_data_last_updated_at ASC NULLS FIRST 
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

    /// Get oldest non-blacklisted tokens (excludes permanently failed market data tokens)
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
             AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
             ORDER BY COALESCE(u.market_data_last_updated_at, 0) ASC
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
        // Validate priority value (Bug #29 fix)
        let valid_priorities = [10, 25, 40, 55, 60, 75, 100];
        if !valid_priorities.contains(&priority) {
            return Err(TokenError::Database(format!(
                "Invalid priority value: {}. Must be one of: 10, 25, 40, 55, 60, 75, 100",
                priority
            )));
        }

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

    /// Update rejection status for a token (called by filtering engine)
    pub fn update_rejection_status(
        &self,
        mint: &str,
        reason: &str,
        source: &str,
        rejected_at: i64,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET 
                last_rejection_reason = ?1, 
                last_rejection_source = ?2, 
                last_rejection_at = ?3 
             WHERE mint = ?4",
            params![reason, source, rejected_at, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to update rejection status: {}", e)))?;

        Ok(())
    }

    /// Clear rejection status for a token that passed filtering
    pub fn clear_rejection_status(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET 
                last_rejection_reason = NULL, 
                last_rejection_source = NULL, 
                last_rejection_at = NULL 
             WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to clear rejection status: {}", e)))?;

        Ok(())
    }

    /// Batch clear rejection status for multiple tokens (PERF optimization)
    /// Uses a single transaction instead of spawning individual tasks
    pub fn batch_clear_rejection_status(&self, mints: &[String]) -> TokenResult<usize> {
        if mints.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {}", e)))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE update_tracking SET 
                        last_rejection_reason = NULL, 
                        last_rejection_source = NULL, 
                        last_rejection_at = NULL 
                     WHERE mint = ?1",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {}", e)))?;

            for mint in mints {
                match stmt.execute(params![mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        // Log but continue - don't fail entire batch
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_clear_rejection_status error for {}: {}", mint, e),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {}", e)))?;

        Ok(updated)
    }

    /// Batch update priority for multiple tokens (PERF optimization)
    /// Uses a single transaction instead of spawning individual tasks
    pub fn batch_update_priority(&self, mints: &[String], priority: i32) -> TokenResult<usize> {
        if mints.is_empty() {
            return Ok(0);
        }

        // Validate priority value (Bug #29 fix)
        let valid_priorities = [10, 25, 40, 55, 60, 75, 100];
        if !valid_priorities.contains(&priority) {
            return Err(TokenError::Database(format!(
                "Invalid priority value: {}. Must be one of: 10, 25, 40, 55, 60, 75, 100",
                priority
            )));
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {}", e)))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached("UPDATE update_tracking SET priority = ?1 WHERE mint = ?2")
                .map_err(|e| TokenError::Database(format!("Prepare failed: {}", e)))?;

            for mint in mints {
                match stmt.execute(params![priority, mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_update_priority error for {}: {}", mint, e),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {}", e)))?;

        Ok(updated)
    }

    /// Batch update rejection status for multiple tokens (PERF optimization)
    /// updates: Vec of (mint, reason, source, rejected_at)
    pub fn batch_update_rejection_status(
        &self,
        updates: &[(String, String, String, i64)],
    ) -> TokenResult<usize> {
        if updates.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {}", e)))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE update_tracking SET 
                        last_rejection_reason = ?1, 
                        last_rejection_source = ?2, 
                        last_rejection_at = ?3 
                     WHERE mint = ?4",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {}", e)))?;

            for (mint, reason, source, rejected_at) in updates {
                match stmt.execute(params![reason, source, rejected_at, mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_update_rejection_status error for {}: {}", mint, e),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {}", e)))?;

        Ok(updated)
    }

    /// Batch upsert rejection stats (PERF optimization)
    /// stats: Vec of (reason, source, timestamp)
    pub fn batch_upsert_rejection_stats(
        &self,
        stats: &[(String, String, i64)],
    ) -> TokenResult<usize> {
        if stats.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {}", e)))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO rejection_stats (bucket_hour, reason, source, rejection_count, unique_tokens, first_seen, last_seen)
                     VALUES (?1, ?2, ?3, 1, 1, ?4, ?4)
                     ON CONFLICT(bucket_hour, reason, source) DO UPDATE SET
                         rejection_count = rejection_count + 1,
                         last_seen = ?4",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {}", e)))?;

            for (reason, source, timestamp) in stats {
                // Round timestamp to hour bucket
                let bucket_hour = (timestamp / 3600) * 3600;
                match stmt.execute(params![bucket_hour, reason, source, timestamp]) {
                    Ok(_) => updated += 1,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_upsert_rejection_stats error: {}", e),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {}", e)))?;

        Ok(updated)
    }

    /// Get rejection statistics grouped by reason
    pub fn get_rejection_stats(&self) -> TokenResult<Vec<(String, String, i64)>> {
        self.get_rejection_stats_with_time_filter(None, None)
    }

    /// Get rejection statistics grouped by reason with optional time filter
    /// Queries update_tracking table for UNIQUE tokens rejected in time range
    /// This is the correct semantic - counting unique tokens, not cumulative events
    pub fn get_rejection_stats_with_time_filter(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // Build query with optional time filters on last_rejection_at
        let mut query = "SELECT 
                    last_rejection_reason, 
                    last_rejection_source, 
                    COUNT(*) as count 
                 FROM update_tracking 
                 WHERE last_rejection_reason IS NOT NULL"
            .to_string();

        if start_time.is_some() {
            query.push_str(" AND last_rejection_at >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND last_rejection_at <= :end_time");
        }

        query
            .push_str(" GROUP BY last_rejection_reason, last_rejection_source ORDER BY count DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        // Bind parameters
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
        if let Some(ref start) = start_time {
            params.push((":start_time", start));
        }
        if let Some(ref end) = end_time {
            params.push((":end_time", end));
        }

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Get list of rejected tokens with pagination and optional filtering
    pub fn get_recent_rejections(
        &self,
        limit: usize,
    ) -> TokenResult<Vec<(String, String, String, i64, Option<String>)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let query = "SELECT ut.mint, ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at, t.symbol 
                     FROM update_tracking ut 
                     LEFT JOIN tokens t ON ut.mint = t.mint 
                     WHERE ut.last_rejection_reason IS NOT NULL 
                     ORDER BY ut.last_rejection_at DESC LIMIT :limit";

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let limit_i64 = limit as i64;
        let rows = stmt
            .query_map(&[(":limit", &limit_i64)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| TokenError::Database(format!("Row failed: {}", e)))?);
        }

        Ok(results)
    }

    pub fn get_rejected_tokens(
        &self,
        reason_filter: Option<String>,
        source_filter: Option<String>,
        search_filter: Option<String>,
        limit: usize,
        offset: usize,
    ) -> TokenResult<Vec<(String, String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut query = if search_filter.is_some() {
            "SELECT ut.mint, ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at 
             FROM update_tracking ut 
             LEFT JOIN tokens t ON ut.mint = t.mint 
             WHERE ut.last_rejection_reason IS NOT NULL".to_string()
        } else {
            "SELECT mint, last_rejection_reason, last_rejection_source, last_rejection_at 
             FROM update_tracking 
             WHERE last_rejection_reason IS NOT NULL"
                .to_string()
        };

        if reason_filter.is_some() {
            query.push_str(if search_filter.is_some() {
                " AND ut.last_rejection_reason = :reason"
            } else {
                " AND last_rejection_reason = :reason"
            });
        }

        if source_filter.is_some() {
            query.push_str(if search_filter.is_some() {
                " AND ut.last_rejection_source = :source"
            } else {
                " AND last_rejection_source = :source"
            });
        }

        if search_filter.is_some() {
            query.push_str(
                " AND (ut.mint LIKE :search OR t.symbol LIKE :search OR t.name LIKE :search)",
            );
        }

        query.push_str(if search_filter.is_some() {
            " ORDER BY ut.last_rejection_at DESC LIMIT :limit OFFSET :offset"
        } else {
            " ORDER BY last_rejection_at DESC LIMIT :limit OFFSET :offset"
        });

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        // Build params dynamically - only include params that are in the query
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
        if let Some(ref reason) = reason_filter {
            params.push((":reason", reason));
        }
        if let Some(ref source) = source_filter {
            params.push((":source", source));
        }

        let search_pattern;
        if let Some(ref search) = search_filter {
            search_pattern = format!("%{}%", search);
            params.push((":search", &search_pattern));
        }

        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;
        params.push((":limit", &limit_i64));
        params.push((":offset", &offset_i64));

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Insert rejection event into history table (for time-range analytics)
    pub fn insert_rejection_history(
        &self,
        mint: &str,
        reason: &str,
        source: &str,
        rejected_at: i64,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "INSERT INTO rejection_history (mint, reason, source, rejected_at) VALUES (?1, ?2, ?3, ?4)",
            params![mint, reason, source, rejected_at],
        )
        .map_err(|e| TokenError::Database(format!("Failed to insert rejection history: {}", e)))?;

        Ok(())
    }

    /// Get rejection statistics for a specific time range
    pub fn get_rejection_stats_for_range(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // If no time range specified, fall back to current rejection stats (update_tracking table)
        if start_time.is_none() && end_time.is_none() {
            return self.get_rejection_stats();
        }

        // Query rejection_history table for time-range stats
        let mut query =
            "SELECT reason, source, COUNT(*) as count FROM rejection_history WHERE 1=1".to_string();

        if start_time.is_some() {
            query.push_str(" AND rejected_at >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND rejected_at <= :end_time");
        }

        query.push_str(" GROUP BY reason, source ORDER BY count DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
        if let Some(ref start) = start_time {
            params.push((":start_time", start));
        }
        if let Some(ref end) = end_time {
            params.push((":end_time", end));
        }

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Cleanup old rejection history entries (keep last N hours)
    /// This is critical for database size management - rejection history grows ~5GB/day
    pub fn cleanup_rejection_history(&self, hours_to_keep: i64) -> TokenResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let cutoff = chrono::Utc::now().timestamp() - (hours_to_keep * 60 * 60);

        let deleted = conn
            .execute(
                "DELETE FROM rejection_history WHERE rejected_at < ?1",
                params![cutoff],
            )
            .map_err(|e| {
                TokenError::Database(format!("Failed to cleanup rejection history: {}", e))
            })?;

        Ok(deleted)
    }

    /// Upsert rejection stat into aggregated hourly bucket table
    /// This replaces per-event logging with O(1) aggregation
    pub fn upsert_rejection_stat(
        &self,
        reason: &str,
        source: &str,
        timestamp: i64,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // Round timestamp to hour bucket
        let bucket_hour = (timestamp / 3600) * 3600;

        conn.execute(
            "INSERT INTO rejection_stats (bucket_hour, reason, source, rejection_count, unique_tokens, first_seen, last_seen)
             VALUES (?1, ?2, ?3, 1, 1, ?4, ?4)
             ON CONFLICT(bucket_hour, reason, source) DO UPDATE SET
                 rejection_count = rejection_count + 1,
                 last_seen = ?4",
            params![bucket_hour, reason, source, timestamp],
        )
        .map_err(|e| TokenError::Database(format!("Upsert rejection stat failed: {}", e)))?;

        Ok(())
    }

    /// Get rejection statistics from aggregated table for a time range
    pub fn get_rejection_stats_aggregated(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut query =
            "SELECT reason, source, SUM(rejection_count) as total FROM rejection_stats WHERE 1=1"
                .to_string();

        if start_time.is_some() {
            query.push_str(" AND bucket_hour >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND bucket_hour <= :end_time");
        }

        query.push_str(" GROUP BY reason, source ORDER BY total DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Prepare failed: {}", e)))?;

        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
        if let Some(ref start) = start_time {
            params.push((":start_time", start));
        }
        if let Some(ref end) = end_time {
            params.push((":end_time", end));
        }

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }
        Ok(results)
    }

    /// Cleanup old aggregated rejection stats (keep last N hours)
    pub fn cleanup_rejection_stats(&self, hours_to_keep: i64) -> TokenResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let cutoff = chrono::Utc::now().timestamp() - (hours_to_keep * 3600);

        let deleted = conn
            .execute(
                "DELETE FROM rejection_stats WHERE bucket_hour < ?1",
                params![cutoff],
            )
            .map_err(|e| TokenError::Database(format!("Delete rejection stats failed: {}", e)))?;

        Ok(deleted)
    }

    /// Check if token has stale market data (>2 minutes old or missing)
    /// Reserved for future use in health monitoring/diagnostics (Bug #27)
    #[allow(dead_code)]
    pub fn is_market_data_stale(&self, mint: &str, threshold_seconds: i64) -> TokenResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT market_data_last_updated_at FROM update_tracking WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let result: Result<i64, rusqlite::Error> = stmt.query_row(params![mint], |row| row.get(0));

        match result {
            Ok(last_update) => {
                let now = chrono::Utc::now().timestamp();
                let age = now - last_update;
                Ok(age > threshold_seconds)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true), // No update tracking = stale
            Err(e) => Err(TokenError::Database(format!("Query failed: {}", e))),
        }
    }

    /// Get priority mapping for specific tokens
    pub fn get_priorities_for_tokens(&self, mints: &[String]) -> TokenResult<HashMap<String, i32>> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut placeholders = String::new();
        for (idx, _) in mints.iter().enumerate() {
            if idx > 0 {
                placeholders.push(',');
            }
            placeholders.push('?');
        }

        let query = format!(
            "SELECT mint, priority FROM update_tracking WHERE mint IN ({})",
            placeholders
        );

        let mint_refs: Vec<&str> = mints.iter().map(|mint| mint.as_str()).collect();

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {}", e)))?;

        let rows = stmt
            .query_map(params_from_iter(mint_refs.into_iter()), |row| {
                let mint: String = row.get(0)?;
                let priority: i32 = row.get(1)?;
                Ok((mint, priority))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {}", e)))?;

        let mut result = HashMap::new();
        for row in rows {
            let (mint, priority) =
                row.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;
            result.insert(mint, priority);
        }

        Ok(result)
    }

    /// Get tokens that have never received market data (excludes permanently failed)
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
                 WHERE u.market_data_update_count = 0
                 AND md.mint IS NULL
                 AND mg.mint IS NULL
                 AND (u.last_error_at IS NULL OR u.last_error_at < strftime('%s','now') - 180)
                 AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
                 ORDER BY u.priority DESC, t.first_discovered_at ASC
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

    /// Count tokens with permanent market data failure (not listed on any exchange)
    /// These tokens are excluded from market data update attempts
    pub fn count_permanent_market_failures(&self) -> TokenResult<u64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM update_tracking WHERE market_error_type = 'permanent'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| TokenError::Database(format!("Failed to count: {}", e)))?;

        Ok(count as u64)
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
                     -- Priority 1: New tokens (discovered in last 24h, no errors)
                     WHEN ut.security_error_type IS NULL AND t.first_discovered_at > ?1 - 86400 THEN 1
                     -- Priority 2: Tokens without errors
                     WHEN ut.security_error_type IS NULL THEN 2
                     -- Priority 3: Temporary errors (with backoff)
                     WHEN ut.security_error_type = 'temporary' THEN 3
                     -- Priority 4: Permanent errors (very rare retry)
                     ELSE 4
                 END,
                 t.first_discovered_at ASC
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

    /// Record a failed market update attempt with error type tracking
    ///
    /// Error types:
    /// - "temporary": Transient errors (rate limit, network issues) - retry with backoff
    /// - "permanent": Token not listed on any exchange - stop retrying after threshold
    ///
    /// Returns the new error count for the token
    pub fn record_market_error(
        &self,
        mint: &str,
        message: &str,
        error_type: &str,
    ) -> TokenResult<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        // Update error tracking with type and increment count
        conn.execute(
            "UPDATE update_tracking SET 
                last_error = ?1, 
                last_error_at = ?2, 
                market_error_count = market_error_count + 1,
                market_error_type = ?3
             WHERE mint = ?4",
            params![message, now, error_type, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to record market error: {}", e)))?;

        // Get the new error count
        let error_count: u32 = conn
            .query_row(
                "SELECT market_error_count FROM update_tracking WHERE mint = ?1",
                params![mint],
                |row| row.get(0),
            )
            .unwrap_or(1);

        Ok(error_count)
    }

    /// Clear market error tracking (called after successful market data fetch)
    pub fn clear_market_error(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET 
                market_error_count = 0,
                last_error = NULL,
                last_error_at = NULL,
                market_error_type = NULL
             WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to clear market error: {}", e)))?;

        Ok(())
    }

    /// Mark a token as permanently failed for market data updates
    /// This only updates the error_type without incrementing the error count
    /// Used when a token has hit the failure threshold and should be excluded from updates
    pub fn mark_market_permanent(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        conn.execute(
            "UPDATE update_tracking SET market_error_type = 'permanent' WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to mark market permanent: {}", e)))?;

        Ok(())
    }

    /// Mark market data as updated (called after successful DexScreener or GeckoTerminal fetch)
    pub fn mark_market_data_updated(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        // Also clear any market error state on success
        conn.execute(
            "UPDATE update_tracking SET 
                market_data_last_updated_at = ?1,
                market_data_update_count = market_data_update_count + 1,
                last_error = NULL,
                last_error_at = NULL,
                market_error_count = 0,
                market_error_type = NULL
             WHERE mint = ?2",
            params![now, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to mark market data updated: {}", e)))?;

        Ok(())
    }

    /// Mark security data as updated (called after successful Rugcheck fetch)
    pub fn mark_security_data_updated(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                security_data_last_updated_at = ?1,
                security_data_update_count = security_data_update_count + 1,
                last_security_error = NULL,
                last_security_error_at = NULL,
                security_error_type = NULL
             WHERE mint = ?2",
            params![now, mint],
        )
        .map_err(|e| {
            TokenError::Database(format!("Failed to mark security data updated: {}", e))
        })?;

        Ok(())
    }

    /// Mark metadata as updated (called after symbol/name/decimals change)
    pub fn mark_metadata_updated(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET metadata_last_updated_at = ?1 WHERE mint = ?2",
            params![now, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to mark metadata updated: {}", e)))?;

        Ok(())
    }

    /// Mark decimals as updated (called after decimals fetch from chain)
    pub fn mark_decimals_updated(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET decimals_last_updated_at = ?1 WHERE mint = ?2",
            params![now, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to mark decimals updated: {}", e)))?;

        Ok(())
    }

    /// Mark pool price as calculated (called after Pool Service calculation)
    pub fn mark_pool_price_calculated(&self, mint: &str, pool_address: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                pool_price_last_calculated_at = ?1,
                pool_price_last_used_pool_address = ?2
             WHERE mint = ?3",
            params![now, pool_address, mint],
        )
        .map_err(|e| {
            TokenError::Database(format!("Failed to mark pool price calculated: {}", e))
        })?;

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

    /// List all blacklist entries with metadata for diagnostics/analytics
    pub fn list_blacklisted_tokens(&self) -> TokenResult<Vec<TokenBlacklistRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, reason, source, added_at \
                 FROM blacklist \
                 ORDER BY added_at DESC",
            )
            .map_err(|e| {
                TokenError::Database(format!("Failed to prepare blacklist query: {}", e))
            })?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TokenBlacklistRecord {
                    mint: row.get(0)?,
                    reason: row.get(1)?,
                    source: row.get(2)?,
                    added_at: row.get(3)?,
                })
            })
            .map_err(|e| TokenError::Database(format!("Failed to query blacklist: {}", e)))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| {
                TokenError::Database(format!("Failed to read blacklist row: {}", e))
            })?);
        }

        Ok(records)
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
                "SELECT mint, priority,
                        market_data_last_updated_at, market_data_update_count,
                        security_data_last_updated_at, security_data_update_count,
                        metadata_last_updated_at, decimals_last_updated_at,
                        pool_price_last_calculated_at, pool_price_last_used_pool_address,
                        last_error, last_error_at, market_error_count, market_error_type,
                        last_security_error, last_security_error_at, security_error_count
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
                    "SELECT mint, priority,
                            market_data_last_updated_at, market_data_update_count,
                            security_data_last_updated_at, security_data_update_count,
                            metadata_last_updated_at, decimals_last_updated_at,
                            pool_price_last_calculated_at, pool_price_last_used_pool_address,
                            last_error, last_error_at, market_error_count, market_error_type,
                            last_security_error, last_security_error_at, security_error_count
                     FROM update_tracking
                     WHERE priority = ?1
                       AND (market_error_type IS NULL OR market_error_type != 'permanent')
                     ORDER BY COALESCE(market_data_last_updated_at, 0) ASC, mint ASC
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
                    "SELECT mint, priority,
                            market_data_last_updated_at, market_data_update_count,
                            security_data_last_updated_at, security_data_update_count,
                            metadata_last_updated_at, decimals_last_updated_at,
                            pool_price_last_calculated_at, pool_price_last_used_pool_address,
                            last_error, last_error_at, market_error_count, market_error_type,
                            last_security_error, last_security_error_at, security_error_count
                     FROM update_tracking
                     WHERE (market_error_type IS NULL OR market_error_type != 'permanent')
                     ORDER BY priority DESC, COALESCE(market_data_last_updated_at, 0) ASC, mint ASC
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

        // Prepare fallback images from alternate source when primary source is missing them
        let (fallback_img, fallback_header) = match (&data_source, &market_data) {
            // When using GeckoTerminal, try DexScreener images
            (DataSource::GeckoTerminal, _) => match self.get_dexscreener_data(mint)? {
                Some(ds) => (ds.image_url, ds.header_image_url),
                None => (None, None),
            },
            // When using DexScreener without image, try GeckoTerminal
            (DataSource::DexScreener, MarketDataType::DexScreener(ds))
                if ds.image_url.is_none() =>
            {
                match self.get_geckoterminal_data(mint)? {
                    Some(gt) => (gt.image_url, None),
                    None => (None, None),
                }
            }
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
    /// If require_market_data=true, only returns tokens that have DexScreener OR GeckoTerminal data.
    /// This significantly reduces memory usage for filtering (144k -> ~56k tokens).
    ///
    /// PERFORMANCE: Uses LEFT JOINs to fetch all data in a single query, avoiding N+1 problem.
    pub fn get_all_tokens_optional_market(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_direction: Option<&str>,
        require_market_data: bool,
    ) -> TokenResult<Vec<Token>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {}", e)))?;

        // Map sort_by to SQL column with table prefix
        let order_column = match sort_by {
            Some("symbol") => "t.symbol",
            Some("market_data_last_fetched_at") =>
                "COALESCE(ut.market_data_last_updated_at, d.market_data_last_fetched_at, g.market_data_last_fetched_at, t.metadata_last_fetched_at)",
            Some("first_discovered_at") => "t.first_discovered_at",
            Some("metadata_last_fetched_at") => "COALESCE(ut.metadata_last_updated_at, t.metadata_last_fetched_at)",
            Some("blockchain_created_at") =>
                "COALESCE(d.pair_blockchain_created_at, t.blockchain_created_at, t.first_discovered_at)",
            Some("pool_price_last_calculated_at") =>
                "COALESCE(ut.pool_price_last_calculated_at, t.first_discovered_at)",
            Some("mint") => "t.mint",
            Some("risk_score") => "sr.score",
            Some("price_sol") => "COALESCE(d.price_sol, g.price_sol)",
            Some("liquidity_usd") => "COALESCE(d.liquidity_usd, g.liquidity_usd)",
            Some("volume_24h") => "COALESCE(d.volume_24h, g.volume_24h)",
            Some("fdv") => "COALESCE(d.fdv, g.fdv)",
            Some("market_cap") => "COALESCE(d.market_cap, g.market_cap)",
            Some("price_change_h1") => "COALESCE(d.price_change_1h, g.price_change_1h)",
            Some("price_change_h24") => "COALESCE(d.price_change_24h, g.price_change_24h)",
            Some("txns_5m") => "COALESCE(d.txns_5m_buys, 0) + COALESCE(d.txns_5m_sells, 0)",
            Some("txns_1h") => "COALESCE(d.txns_1h_buys, 0) + COALESCE(d.txns_1h_sells, 0)",
            Some("txns_6h") => "COALESCE(d.txns_6h_buys, 0) + COALESCE(d.txns_6h_sells, 0)",
            Some("txns_24h") => "COALESCE(d.txns_24h_buys, 0) + COALESCE(d.txns_24h_sells, 0)",
            _ =>
                "COALESCE(ut.market_data_last_updated_at, d.market_data_last_fetched_at, g.market_data_last_fetched_at, t.metadata_last_fetched_at)",
        };

        let direction = match sort_direction {
            Some("asc") => "ASC",
            Some("desc") => "DESC",
            _ => "DESC", // default
        };

        // Build query (always join market tables so we can populate Token fields consistently)
        // PERF: This single query with JOINs avoids N+1 problem for filtering
        let select_base = r#"
            SELECT
                t.mint, t.symbol, t.name, t.decimals,
                t.first_discovered_at, t.blockchain_created_at,
                t.metadata_last_fetched_at, t.decimals_last_fetched_at,
                sr.score, sr.rugged, sr.security_data_last_fetched_at,
                sr.mint_authority, sr.freeze_authority,
                bl.reason as blacklist_reason,
                ut.priority, ut.pool_price_last_calculated_at, ut.pool_price_last_used_pool_address,
                d.price_usd, d.price_sol, d.price_native,
                d.price_change_5m, d.price_change_1h, d.price_change_6h, d.price_change_24h,
                d.market_cap, d.fdv, d.liquidity_usd,
                d.volume_5m, d.volume_1h, d.volume_6h, d.volume_24h,
                d.txns_5m_buys, d.txns_5m_sells, d.txns_1h_buys, d.txns_1h_sells,
                d.txns_6h_buys, d.txns_6h_sells, d.txns_24h_buys, d.txns_24h_sells,
                d.market_data_last_fetched_at as d_market_data_last_fetched_at,
                d.image_url as d_image_url, d.header_image_url as d_header_image_url,
                d.pair_blockchain_created_at,
                g.price_usd, g.price_sol, g.price_native,
                g.price_change_5m, g.price_change_1h, g.price_change_6h, g.price_change_24h,
                g.market_cap, g.fdv, g.liquidity_usd,
                g.volume_5m, g.volume_1h, g.volume_6h, g.volume_24h,
                g.pool_count, g.reserve_in_usd,
                g.market_data_last_fetched_at as g_market_data_last_fetched_at,
                g.image_url as g_image_url,
                ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at,
                sr.update_authority, sr.is_mutable
            FROM tokens t
            LEFT JOIN security_rugcheck sr ON t.mint = sr.mint
            LEFT JOIN blacklist bl ON t.mint = bl.mint
            LEFT JOIN update_tracking ut ON t.mint = ut.mint
            LEFT JOIN market_dexscreener d ON t.mint = d.mint
            LEFT JOIN market_geckoterminal g ON t.mint = g.mint
        "#;

        // PERF: When require_market_data=true, only load tokens with market data
        // This reduces initial load from 144k to ~56k tokens (60% reduction)
        let where_clause = if require_market_data {
            " WHERE (d.mint IS NOT NULL OR g.mint IS NOT NULL)"
        } else {
            ""
        };

        let query = if limit == 0 {
            format!(
                "{}{} ORDER BY {} {}",
                select_base, where_clause, order_column, direction
            )
        } else {
            format!(
                "{}{} ORDER BY {} {} LIMIT {} OFFSET {}",
                select_base, where_clause, order_column, direction, limit, offset
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
                let first_discovered_at: i64 = row.get(4)?;
                let blockchain_created_at: Option<i64> = row.get(5)?;
                let metadata_last_fetched_at: i64 = row.get(6)?;
                let decimals_last_fetched_at: i64 = row.get(7)?;

                // Security data (optional) - includes authority fields for filtering
                let security_score: Option<i32> = row.get(8)?;
                let is_rugged: bool = row
                    .get::<_, Option<i64>>(9)?
                    .map(|v| v != 0)
                    .unwrap_or(false);
                let security_data_last_fetched_at: Option<i64> = row.get(10)?;
                let mint_authority: Option<String> = row.get(11)?;
                let freeze_authority: Option<String> = row.get(12)?;

                // Blacklist status
                let is_blacklisted = row.get::<_, Option<String>>(13)?.is_some();

                // Priority and pool price tracking
                let priority_value: Option<i32> = row.get(14)?;
                let pool_price_last_calculated_at: Option<i64> = row.get(15)?;
                let pool_price_last_used_pool: Option<String> = row.get(16)?;

                // DexScreener fields 17..=37
                let d_price_usd: Option<f64> = row.get(17)?;
                let d_price_sol: Option<f64> = row.get(18)?;
                let d_price_native: Option<String> = row.get(19)?;
                let d_change_5m: Option<f64> = row.get(20)?;
                let d_change_1h: Option<f64> = row.get(21)?;
                let d_change_6h: Option<f64> = row.get(22)?;
                let d_change_24h: Option<f64> = row.get(23)?;
                let d_market_cap: Option<f64> = row.get(24)?;
                let d_fdv: Option<f64> = row.get(25)?;
                let d_liquidity_usd: Option<f64> = row.get(26)?;
                let d_vol_5m: Option<f64> = row.get(27)?;
                let d_vol_1h: Option<f64> = row.get(28)?;
                let d_vol_6h: Option<f64> = row.get(29)?;
                let d_vol_24h: Option<f64> = row.get(30)?;
                let d_txn_5m_buys: Option<i64> = row.get(31)?;
                let d_txn_5m_sells: Option<i64> = row.get(32)?;
                let d_txn_1h_buys: Option<i64> = row.get(33)?;
                let d_txn_1h_sells: Option<i64> = row.get(34)?;
                let d_txn_6h_buys: Option<i64> = row.get(35)?;
                let d_txn_6h_sells: Option<i64> = row.get(36)?;
                let d_txn_24h_buys: Option<i64> = row.get(37)?;
                let d_txn_24h_sells: Option<i64> = row.get(38)?;
                let d_market_data_last_fetched_at: Option<i64> = row.get(39)?;
                let d_image_url: Option<String> = row.get(40)?;
                let d_header_image_url: Option<String> = row.get(41)?;
                let d_pair_blockchain_created_at: Option<i64> = row.get(42)?;

                // GeckoTerminal fields 43..=60
                let g_price_usd: Option<f64> = row.get(43)?;
                let g_price_sol: Option<f64> = row.get(44)?;
                let g_price_native: Option<String> = row.get(45)?;
                let g_change_5m: Option<f64> = row.get(46)?;
                let g_change_1h: Option<f64> = row.get(47)?;
                let g_change_6h: Option<f64> = row.get(48)?;
                let g_change_24h: Option<f64> = row.get(49)?;
                let g_market_cap: Option<f64> = row.get(50)?;
                let g_fdv: Option<f64> = row.get(51)?;
                let g_liquidity_usd: Option<f64> = row.get(52)?;
                let g_vol_5m: Option<f64> = row.get(53)?;
                let g_vol_1h: Option<f64> = row.get(54)?;
                let g_vol_6h: Option<f64> = row.get(55)?;
                let g_vol_24h: Option<f64> = row.get(56)?;
                let g_pool_count: Option<i64> = row.get(57)?;
                let g_reserve_in_usd: Option<f64> = row.get(58)?;
                let g_market_data_last_fetched_at: Option<i64> = row.get(59)?;
                let g_image_url: Option<String> = row.get(60)?;

                // Rejection tracking fields 61..=63
                let last_rejection_reason: Option<String> = row.get(61)?;
                let last_rejection_source: Option<String> = row.get(62)?;
                let last_rejection_at: Option<i64> = row.get(63)?;

                // New security fields 64..=65
                let update_authority: Option<String> = row.get(64)?;
                let is_mutable: Option<bool> = row.get::<_, Option<i64>>(65)?.map(|v| v != 0);

                Ok((
                    mint,
                    symbol,
                    name,
                    decimals.map(|d| d as u8),
                    first_discovered_at,
                    blockchain_created_at,
                    metadata_last_fetched_at,
                    decimals_last_fetched_at,
                    security_score,
                    is_rugged,
                    security_data_last_fetched_at,
                    mint_authority,
                    freeze_authority,
                    is_blacklisted,
                    priority_value,
                    pool_price_last_calculated_at,
                    pool_price_last_used_pool,
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
                    d_market_data_last_fetched_at,
                    d_image_url,
                    d_header_image_url,
                    d_pair_blockchain_created_at,
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
                    g_market_data_last_fetched_at,
                    g_image_url,
                    // Rejection tracking
                    last_rejection_reason,
                    last_rejection_source,
                    last_rejection_at,
                    update_authority,
                    is_mutable,
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
                first_discovered_at,
                blockchain_created_at,
                metadata_last_fetched_at,
                decimals_last_fetched_at,
                security_score,
                is_rugged,
                security_data_last_fetched_at,
                mint_authority,
                freeze_authority,
                is_blacklisted,
                priority_value,
                pool_price_last_calculated_at,
                pool_price_last_used_pool,
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
                d_market_data_last_fetched_at,
                d_image_url,
                d_header_image_url,
                d_pair_blockchain_created_at,
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
                g_market_data_last_fetched_at,
                g_image_url,
                // Rejection tracking
                last_rejection_reason,
                last_rejection_source,
                last_rejection_at,
                update_authority,
                is_mutable,
            ) = row_result.map_err(|e| TokenError::Database(format!("Row parse failed: {}", e)))?;

            // Parse all timestamps
            let first_discovered_dt =
                DateTime::from_timestamp(first_discovered_at, 0).unwrap_or_else(|| Utc::now());
            let blockchain_created_dt =
                blockchain_created_at.and_then(|ts| DateTime::from_timestamp(ts, 0));
            let metadata_last_fetched_dt =
                DateTime::from_timestamp(metadata_last_fetched_at, 0).unwrap_or_else(|| Utc::now());
            let decimals_last_fetched_dt =
                DateTime::from_timestamp(decimals_last_fetched_at, 0).unwrap_or_else(|| Utc::now());
            let security_data_last_fetched_dt =
                security_data_last_fetched_at.and_then(|ts| DateTime::from_timestamp(ts, 0));
            let pool_price_last_calculated_dt = pool_price_last_calculated_at
                .and_then(|ts| DateTime::from_timestamp(ts, 0))
                .unwrap_or(metadata_last_fetched_dt); // Fallback

            // Parse rejection timestamp
            let last_rejection_at_dt =
                last_rejection_at.and_then(|ts| DateTime::from_timestamp(ts, 0));

            let priority = priority_value
                .map(Priority::from_value)
                .unwrap_or(Priority::Standard);
            // Determine chosen market source based on config preference then fallback
            let preferred_source =
                crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());
            let dex_available = d_price_sol.is_some() || d_price_usd.is_some();
            let gecko_available = g_price_sol.is_some() || g_price_usd.is_some();

            let (
                data_source,
                market_data_last_fetched_dt,
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
                        g_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
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
                        d_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
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
                        metadata_last_fetched_dt,
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
                        d_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
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
                        g_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
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
                        metadata_last_fetched_dt,
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

            // Determine image_url and header_image_url with cross-source fallback
            // If primary source doesn't have image, try the other source
            let (resolved_image_url, resolved_header_image_url) = match data_source {
                DataSource::DexScreener => {
                    // Use DexScreener image, fallback to GeckoTerminal if missing
                    let img = d_image_url.clone().or(g_image_url.clone());
                    (img, d_header_image_url.clone())
                }
                DataSource::GeckoTerminal => {
                    // Use GeckoTerminal image, fallback to DexScreener if missing
                    let img = g_image_url.clone().or(d_image_url.clone());
                    (img, d_header_image_url.clone())
                }
                DataSource::Unknown => {
                    // Try any available image
                    let img = d_image_url.clone().or(g_image_url.clone());
                    (img, d_header_image_url.clone())
                }
                _ => (None, None),
            };

            let token = Token {
                // Core Identity & Metadata
                mint: mint.clone(),
                symbol: symbol.unwrap_or_else(|| "UNKNOWN".to_string()),
                name: name.unwrap_or_else(|| "Unknown Token".to_string()),
                decimals: decimals.unwrap_or(9),
                description: None,
                image_url: resolved_image_url,
                header_image_url: resolved_header_image_url,
                supply: None,

                // Data source
                data_source,

                // Discovery & Creation timestamps
                first_discovered_at: first_discovered_dt,
                blockchain_created_at: d_pair_blockchain_created_at
                    .and_then(|ts| DateTime::from_timestamp(ts, 0)),

                // Metadata timestamps
                metadata_last_fetched_at: metadata_last_fetched_dt,
                decimals_last_fetched_at: decimals_last_fetched_dt,

                // Market data timestamps
                market_data_last_fetched_at: market_data_last_fetched_dt,

                // Security data timestamp
                security_data_last_fetched_at: security_data_last_fetched_dt,

                // Pool price timestamps
                pool_price_last_calculated_at: pool_price_last_calculated_dt,
                pool_price_last_used_pool: pool_price_last_used_pool,

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

                // Security Information - authority fields loaded for filtering
                mint_authority,
                freeze_authority,
                update_authority,
                is_mutable,
                security_score,
                security_score_normalised: None, // Not loaded in this query
                is_rugged,
                token_type: None,
                graph_insiders_detected: None,
                lp_provider_count: None,
                security_risks: vec![],
                total_holders: None,
                top_10_holders_pct: None,
                top_holders: vec![],
                creator_balance_pct: None,
                transfer_fee_pct: None,
                transfer_fee_max_amount: None,
                transfer_fee_authority: None,

                // Bot-Specific State
                is_blacklisted,
                priority,

                // Filtering State
                last_rejection_reason,
                last_rejection_source,
                last_rejection_at: last_rejection_at_dt,
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
    pub fn get_priority(&self, mint: &str) -> TokenResult<Priority> {
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
    // Extract timestamps from metadata
    let first_discovered_dt =
        DateTime::from_timestamp(metadata.first_discovered_at, 0).unwrap_or_else(|| Utc::now());
    let metadata_last_fetched_dt = DateTime::from_timestamp(metadata.metadata_last_fetched_at, 0)
        .unwrap_or_else(|| Utc::now());

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
        market_data_last_fetched_at,
        pair_blockchain_created_at,
        pool_metrics,
        market_data_first_fetched_at,
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
                data.market_data_last_fetched_at,
                data.pair_blockchain_created_at,
                (None, None),
                data.market_data_first_fetched_at,
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
                data.market_data_last_fetched_at,
                None,
                (data.pool_count, data.reserve_in_usd),
                data.market_data_first_fetched_at,
            )
        }
    };

    // Extract security data
    let security_ref = security.as_ref();

    let token_type = security_ref.and_then(|sec| sec.token_type.clone());
    let mint_authority = security_ref.and_then(|sec| sec.mint_authority.clone());
    let freeze_authority = security_ref.and_then(|sec| sec.freeze_authority.clone());
    let update_authority = security_ref.and_then(|sec| sec.update_authority.clone());
    let is_mutable = security_ref.and_then(|sec| sec.is_mutable);
    let security_score = security_ref.and_then(|sec| sec.score);
    let security_score_normalised = security_ref.and_then(|sec| sec.score_normalised);
    let is_rugged = security_ref.map(|sec| sec.rugged).unwrap_or(false);
    let security_risks = security_ref
        .map(|sec| sec.risks.clone())
        .unwrap_or_else(Vec::new);
    let top_holders = security_ref
        .map(|sec| sec.top_holders.clone())
        .unwrap_or_else(Vec::new);
    let total_holders = security_ref.and_then(|sec| sec.total_holders);
    let top_10_holders_pct = security_ref.and_then(|sec| sec.top_10_holders_pct);
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

    // For now, only use primary source-provided images. Fallbacks can be added upstream where DB is available.
    let resolved_image_url = primary_image_url.or(fallback_image_url);
    let resolved_header_url = primary_header_url.or(fallback_header_url);

    // Security data timestamp (if available)
    let security_data_last_fetched_dt = security_ref.map(|sec| sec.security_data_last_fetched_at);

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

        // Discovery & Creation timestamps
        first_discovered_at: first_discovered_dt,
        blockchain_created_at: pair_blockchain_created_at,

        // Metadata timestamps
        metadata_last_fetched_at: metadata_last_fetched_dt,
        decimals_last_fetched_at: metadata_last_fetched_dt, // Same as metadata for now

        // Market data timestamps
        market_data_last_fetched_at: market_data_last_fetched_at,

        // Security data timestamp
        security_data_last_fetched_at: security_data_last_fetched_dt,

        // Pool price timestamps (defaults - will be updated by pool service)
        pool_price_last_calculated_at: market_data_last_fetched_at, // Fallback to market fetch
        pool_price_last_used_pool: None,

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
        update_authority,
        is_mutable,
        security_score,
        security_score_normalised,
        is_rugged,
        token_type,
        graph_insiders_detected,
        lp_provider_count,
        security_risks,
        total_holders,
        top_10_holders_pct,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,
        transfer_fee_max_amount,
        transfer_fee_authority,

        // Bot-specific state
        is_blacklisted,
        priority,

        // Filtering State
        last_rejection_reason: None,
        last_rejection_source: None,
        last_rejection_at: None,
    }
}

fn read_row_value<T: FromSql>(row: &Row<'_>, index: usize, field: &str) -> TokenResult<T> {
    row.get(index)
        .map_err(|e| TokenError::Database(format!("Failed to read {}: {}", field, e)))
}

/// Assemble Token without market data (for tokens discovered but not yet enriched)
fn assemble_token_without_market_data(
    metadata: TokenMetadata,
    security: Option<RugcheckData>,
    is_blacklisted: bool,
    priority: Priority,
    metadata_updated_at_override: Option<DateTime<Utc>>,
    last_market_update: Option<DateTime<Utc>>,
    blockchain_created_at_override: Option<DateTime<Utc>>,
    last_rejection_reason: Option<String>,
    last_rejection_source: Option<String>,
    last_rejection_at: Option<DateTime<Utc>>,
) -> Token {
    // Extract security data
    let security_ref = security.as_ref();

    let token_type = security_ref.and_then(|sec| sec.token_type.clone());
    let mint_authority = security_ref.and_then(|sec| sec.mint_authority.clone());
    let freeze_authority = security_ref.and_then(|sec| sec.freeze_authority.clone());
    let update_authority = security_ref.and_then(|sec| sec.update_authority.clone());
    let is_mutable = security_ref.and_then(|sec| sec.is_mutable);
    let security_score = security_ref.and_then(|sec| sec.score);
    let security_score_normalised = security_ref.and_then(|sec| sec.score_normalised);
    let is_rugged = security_ref.map(|sec| sec.rugged).unwrap_or(false);
    let security_risks = security_ref
        .map(|sec| sec.risks.clone())
        .unwrap_or_else(Vec::new);
    let top_holders = security_ref
        .map(|sec| sec.top_holders.clone())
        .unwrap_or_else(Vec::new);
    let total_holders = security_ref.and_then(|sec| sec.total_holders);
    let top_10_holders_pct = security_ref.and_then(|sec| sec.top_10_holders_pct);
    let creator_balance_pct = security_ref.and_then(|sec| sec.creator_balance_pct);
    let transfer_fee_pct = security_ref.and_then(|sec| sec.transfer_fee_pct);
    let transfer_fee_max_amount = security_ref.and_then(|sec| sec.transfer_fee_max_amount);
    let transfer_fee_authority = security_ref.and_then(|sec| sec.transfer_fee_authority.clone());
    let graph_insiders_detected = security_ref.and_then(|sec| sec.graph_insiders_detected);
    let lp_provider_count = security_ref.and_then(|sec| sec.total_lp_providers);

    // Parse timestamps from metadata
    let first_discovered_dt =
        DateTime::from_timestamp(metadata.first_discovered_at, 0).unwrap_or_else(|| Utc::now());
    let metadata_last_fetched_dt = DateTime::from_timestamp(metadata.metadata_last_fetched_at, 0)
        .unwrap_or_else(|| Utc::now());

    // Override if provided, otherwise use metadata timestamp
    let final_metadata_updated = metadata_updated_at_override.unwrap_or(metadata_last_fetched_dt);

    // Market data last fetched fallback
    let market_data_last_fetched_dt = last_market_update.unwrap_or(final_metadata_updated);

    // Security timestamp (if available)
    let security_data_last_fetched_dt = security_ref.map(|sec| sec.security_data_last_fetched_at);

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

        // Discovery & Creation timestamps
        first_discovered_at: first_discovered_dt,
        blockchain_created_at: blockchain_created_at_override,

        // Metadata timestamps
        metadata_last_fetched_at: final_metadata_updated,
        decimals_last_fetched_at: final_metadata_updated, // Same as metadata

        // Market data timestamps (defaults since no market data)
        market_data_last_fetched_at: market_data_last_fetched_dt,

        // Security data timestamp
        security_data_last_fetched_at: security_data_last_fetched_dt,

        // Pool price timestamps (defaults)
        pool_price_last_calculated_at: market_data_last_fetched_dt, // Fallback
        pool_price_last_used_pool: None,

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
        update_authority,
        is_mutable,
        security_score,
        security_score_normalised,
        is_rugged,
        token_type,
        graph_insiders_detected,
        lp_provider_count,
        security_risks,
        total_holders,
        top_10_holders_pct,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,
        transfer_fee_max_amount,
        transfer_fee_authority,

        // Bot-Specific State
        is_blacklisted,
        priority,

        // Filtering State
        last_rejection_reason,
        last_rejection_source,
        last_rejection_at,
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

/// Async wrapper for get_token_images_batch (returns HashMap<mint, image_url>)
/// Fetches image URLs for multiple tokens in a single query - use for batch operations
pub async fn get_token_images_batch_async(
    mints: Vec<String>,
) -> TokenResult<HashMap<String, String>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || db.get_token_images_batch(&mints))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for get_token_info_batch (returns HashMap<mint, (symbol, name, image_url)>)
/// Fetches basic token info for multiple tokens in a single query - use for display purposes
pub async fn get_token_info_batch_async(
    mints: Vec<String>,
) -> TokenResult<HashMap<String, (Option<String>, Option<String>, Option<String>)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || db.get_token_info_batch(&mints))
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

/// Async wrapper for get_token_pools (returns aggregated pool snapshot)
pub async fn get_token_pools_async(mint: &str) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    let mint = mint.to_string();
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || db_clone.get_token_pools(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for replace_token_pools (persist aggregated pool snapshot)
pub async fn replace_token_pools_async(snapshot: TokenPoolsSnapshot) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    let mint = snapshot.mint.clone();

    tokio::task::spawn_blocking(move || db.replace_token_pools(&snapshot))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))??;

    // Invalidate cache after successful pool replacement
    store::invalidate_token_snapshot(&mint);

    Ok(())
}

/// Async wrapper for list_tokens (returns Vec<TokenMetadata>)
pub async fn list_tokens_async(limit: usize) -> TokenResult<Vec<TokenMetadata>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || db.list_tokens(limit))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async wrapper for listing all token blacklist entries
pub async fn list_blacklisted_tokens_async() -> TokenResult<Vec<TokenBlacklistRecord>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || db.list_blacklisted_tokens())
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
            false, // Load all tokens including those without market data
        )
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: Load tokens for filtering with market data optimization.
/// PERF: Uses efficient JOINs to avoid N+1 query problem.
/// PERF: Only loads tokens with DexScreener OR GeckoTerminal data (reduces 144k -> ~56k tokens).
/// Returns tokens with market data and security fields needed for filtering.
pub async fn get_all_tokens_for_filtering_async() -> TokenResult<Vec<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || {
        // PERF: require_market_data=true reduces initial load by ~60%
        // Tokens without market data are immediately rejected anyway (dex_data_missing)
        db.get_all_tokens_optional_market(0, 0, None, None, true)
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

/// Async: update token priority
pub async fn update_token_priority_async(mint: &str, priority: i32) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    let mint_owned = mint.to_string();
    tokio::task::spawn_blocking(move || db.update_priority(&mint_owned, priority))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: check if token market data is stale
/// Reserved for future use in health monitoring/diagnostics (Bug #27)
#[allow(dead_code)]
pub async fn is_market_data_stale_async(mint: &str, threshold_seconds: i64) -> TokenResult<bool> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    let mint_owned = mint.to_string();
    tokio::task::spawn_blocking(move || db.is_market_data_stale(&mint_owned, threshold_seconds))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: count tokens with permanent market data failure
pub async fn count_permanent_market_failures_async() -> TokenResult<u64> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.count_permanent_market_failures())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: update token rejection status
pub async fn update_rejection_status_async(
    mint: &str,
    reason: &str,
    source: &str,
    rejected_at: i64,
) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    let mint_owned = mint.to_string();
    let reason_owned = reason.to_string();
    let source_owned = source.to_string();
    tokio::task::spawn_blocking(move || {
        db.update_rejection_status(&mint_owned, &reason_owned, &source_owned, rejected_at)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: clear token rejection status (when token passes)
pub async fn clear_rejection_status_async(mint: &str) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    let mint_owned = mint.to_string();
    tokio::task::spawn_blocking(move || db.clear_rejection_status(&mint_owned))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: batch clear rejection status for multiple tokens (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
pub async fn batch_clear_rejection_status_async(mints: Vec<String>) -> TokenResult<usize> {
    if mints.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.batch_clear_rejection_status(&mints))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: batch update priority for multiple tokens (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
pub async fn batch_update_priority_async(mints: Vec<String>, priority: i32) -> TokenResult<usize> {
    if mints.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.batch_update_priority(&mints, priority))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: batch update rejection status for multiple tokens (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
/// updates: Vec of (mint, reason, source, rejected_at)
pub async fn batch_update_rejection_status_async(
    updates: Vec<(String, String, String, i64)>,
) -> TokenResult<usize> {
    if updates.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.batch_update_rejection_status(&updates))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: batch upsert rejection stats (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
/// stats: Vec of (reason, source, timestamp)
pub async fn batch_upsert_rejection_stats_async(
    stats: Vec<(String, String, i64)>,
) -> TokenResult<usize> {
    if stats.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.batch_upsert_rejection_stats(&stats))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: get rejection statistics grouped by reason
pub async fn get_rejection_stats_async() -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.get_rejection_stats())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: get rejection statistics with optional time filter
/// Counts UNIQUE tokens rejected in the time range (not cumulative events)
pub async fn get_rejection_stats_with_time_filter_async(
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || {
        db.get_rejection_stats_with_time_filter(start_time, end_time)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: get rejected tokens list
pub async fn get_recent_rejections_async(
    limit: usize,
) -> TokenResult<Vec<(String, String, String, i64, Option<String>)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || db.get_recent_rejections(limit))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

pub async fn get_rejected_tokens_async(
    reason_filter: Option<String>,
    source_filter: Option<String>,
    search_filter: Option<String>,
    limit: usize,
    offset: usize,
) -> TokenResult<Vec<(String, String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;

    tokio::task::spawn_blocking(move || {
        db.get_rejected_tokens(reason_filter, source_filter, search_filter, limit, offset)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: insert rejection history event (for time-range analytics)
pub async fn insert_rejection_history_async(
    mint: &str,
    reason: &str,
    source: &str,
    rejected_at: i64,
) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    let mint_owned = mint.to_string();
    let reason_owned = reason.to_string();
    let source_owned = source.to_string();
    tokio::task::spawn_blocking(move || {
        db.insert_rejection_history(&mint_owned, &reason_owned, &source_owned, rejected_at)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: get rejection statistics for a specific time range
pub async fn get_rejection_stats_for_range_async(
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.get_rejection_stats_for_range(start_time, end_time))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: cleanup old rejection history entries (keep last N hours)
pub async fn cleanup_rejection_history_async(hours_to_keep: i64) -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.cleanup_rejection_history(hours_to_keep))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: upsert rejection stat into aggregated hourly bucket
pub async fn upsert_rejection_stat_async(
    reason: &str,
    source: &str,
    timestamp: i64,
) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    let reason = reason.to_string();
    let source = source.to_string();
    tokio::task::spawn_blocking(move || db.upsert_rejection_stat(&reason, &source, timestamp))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: get rejection statistics from aggregated table
pub async fn get_rejection_stats_aggregated_async(
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.get_rejection_stats_aggregated(start_time, end_time))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

/// Async: cleanup old aggregated rejection stats
pub async fn cleanup_rejection_stats_async(hours_to_keep: i64) -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_string()))?;
    tokio::task::spawn_blocking(move || db.cleanup_rejection_stats(hours_to_keep))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {}", e)))?
}

fn map_tracking_row(row: &rusqlite::Row) -> rusqlite::Result<UpdateTrackingInfo> {
    let mint: String = row.get(0)?;
    let priority: i32 = row.get(1)?;
    let market_data_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(2)?);
    let market_data_update_count = row.get::<_, Option<i64>>(3)?.unwrap_or(0).max(0) as u64;
    let security_data_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(4)?);
    let security_data_update_count = row.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0) as u64;
    let metadata_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(6)?);
    let decimals_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(7)?);
    let pool_price_last_calculated = ts_to_datetime(row.get::<_, Option<i64>>(8)?);
    let pool_price_last_used_pool_address: Option<String> = row.get(9)?;
    let last_error: Option<String> = row.get(10)?;
    let last_error_at = ts_to_datetime(row.get::<_, Option<i64>>(11)?);
    let market_error_count = row.get::<_, Option<i64>>(12)?.unwrap_or(0).max(0) as u64;
    let market_error_type: Option<String> = row.get(13)?;
    let last_security_error: Option<String> = row.get(14)?;
    let last_security_error_at = ts_to_datetime(row.get::<_, Option<i64>>(15)?);
    let security_error_count = row.get::<_, Option<i64>>(16)?.unwrap_or(0).max(0) as u64;

    Ok(UpdateTrackingInfo {
        mint,
        priority,
        market_data_last_updated_at: market_data_last_updated,
        market_data_update_count,
        security_data_last_updated_at: security_data_last_updated,
        security_data_update_count,
        metadata_last_updated_at: metadata_last_updated,
        decimals_last_updated_at: decimals_last_updated,
        pool_price_last_calculated_at: pool_price_last_calculated,
        pool_price_last_used_pool_address,
        market_error_count,
        market_error_type,
        security_error_count,
        last_error,
        last_error_at,
    })
}

fn ts_to_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.and_then(|value| DateTime::from_timestamp(value, 0))
}
