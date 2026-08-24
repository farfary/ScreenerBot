//! Token market data storage — persists price, volume, and market cap from APIs.

use crate::errors::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::tokens::store;
use crate::tokens::types::{
    DexScreenerData, GeckoTerminalData, Priority, RugcheckData, Token, TokenMetadata, TokenResult,
};
use crate::tokens::Error;

use super::helpers::assemble_token_without_market_data;
use super::TokenDatabase;

impl TokenDatabase {
    /// Count tokens that have no market data from any source
    pub fn count_tokens_no_market(&self) -> TokenResult<usize> {
        let conn = self.conn()?;

        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens t \
                 LEFT JOIN market_dexscreener d ON t.chain_id = d.chain_id AND t.mint = d.mint \
                 LEFT JOIN market_geckoterminal g ON t.chain_id = g.chain_id AND t.mint = g.mint \
                 WHERE t.chain_id = ?1 AND d.mint IS NULL AND g.mint IS NULL",
                params![self.chain_id()],
                |row| row.get(0),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Count no-market failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(count)
    }

    /// Fetch tokens missing market data, ordered by priority
    pub fn get_tokens_no_market(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_direction: Option<&str>,
    ) -> TokenResult<Vec<Token>> {
        let conn = self.conn()?;

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
                    LEFT JOIN security_rugcheck sr ON t.chain_id = sr.chain_id AND t.mint = sr.mint \
                    LEFT JOIN blacklist bl ON t.chain_id = bl.chain_id AND t.mint = bl.mint \
                    LEFT JOIN update_tracking ut ON t.chain_id = ut.chain_id AND t.mint = ut.mint \
                    LEFT JOIN market_dexscreener d ON t.chain_id = d.chain_id AND t.mint = d.mint \
                    LEFT JOIN market_geckoterminal g ON t.chain_id = g.chain_id AND t.mint = g.mint \
                    WHERE t.chain_id = ?1 AND d.mint IS NULL AND g.mint IS NULL";

        let query = if limit == 0 {
            format!("{base} ORDER BY {order_column} {direction}")
        } else {
            format!(
                "{} ORDER BY {} {} LIMIT {} OFFSET {}",
                base, order_column, direction, limit, offset
            )
        };

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare".to_owned(),
                message: e.to_string(),
            })
        })?;

        let rows = stmt
            .query_map(params![self.chain_id()], |row| {
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
                    .unwrap_or_default();
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
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query no-market failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

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
            ) = row.map_err(|e| Error::RowDecode {
                detail: e.to_string(),
            })?;

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
                self.chain(),
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
    // DEXSCREENER DATA OPERATIONS
    // ========================================================================

    /// Insert or update DexScreener market data for a token
    pub fn upsert_dexscreener_data(&self, mint: &str, data: &DexScreenerData) -> TokenResult<()> {
        let conn = self.conn()?;

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM market_dexscreener WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
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
                "SELECT market_data_first_fetched_at FROM market_dexscreener WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        conn.execute(
            "INSERT INTO market_dexscreener (
                chain_id, mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                txns_5m_buys, txns_5m_sells, txns_1h_buys, txns_1h_sells,
                txns_6h_buys, txns_6h_sells, txns_24h_buys, txns_24h_sells,
                pair_address, provider_chain_id, dex_id, url, pair_blockchain_created_at, image_url, header_image_url,
                market_data_last_fetched_at, market_data_first_fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                       ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33)
             ON CONFLICT(chain_id, mint) DO UPDATE SET
                price_usd = excluded.price_usd, price_sol = excluded.price_sol, price_native = excluded.price_native,
                price_change_5m = excluded.price_change_5m, price_change_1h = excluded.price_change_1h, price_change_6h = excluded.price_change_6h, price_change_24h = excluded.price_change_24h,
                market_cap = excluded.market_cap, fdv = excluded.fdv, liquidity_usd = excluded.liquidity_usd,
                volume_5m = excluded.volume_5m, volume_1h = excluded.volume_1h, volume_6h = excluded.volume_6h, volume_24h = excluded.volume_24h,
                txns_5m_buys = excluded.txns_5m_buys, txns_5m_sells = excluded.txns_5m_sells, txns_1h_buys = excluded.txns_1h_buys, txns_1h_sells = excluded.txns_1h_sells,
                txns_6h_buys = excluded.txns_6h_buys, txns_6h_sells = excluded.txns_6h_sells, txns_24h_buys = excluded.txns_24h_buys, txns_24h_sells = excluded.txns_24h_sells,
                pair_address = excluded.pair_address, provider_chain_id = excluded.provider_chain_id, dex_id = excluded.dex_id, url = excluded.url, pair_blockchain_created_at = excluded.pair_blockchain_created_at,
                image_url = excluded.image_url, header_image_url = excluded.header_image_url, market_data_last_fetched_at = excluded.market_data_last_fetched_at",
            params![
                self.chain_id(), mint, data.price_usd, data.price_sol, &data.price_native,
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
        ).map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to upsert DexScreener data".to_owned(), message: e.to_string() }))?;

        // Update in-memory cache
        store::store_dexscreener(self.chain(), mint, data);

        Ok(())
    }

    /// Get DexScreener market data
    pub fn get_dexscreener_data(&self, mint: &str) -> TokenResult<Option<DexScreenerData>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT price_usd, price_sol, price_native,
                    price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                    market_cap, fdv, liquidity_usd,
                    volume_5m, volume_1h, volume_6h, volume_24h,
                    txns_5m_buys, txns_5m_sells, txns_1h_buys, txns_1h_sells,
                    txns_6h_buys, txns_6h_sells, txns_24h_buys, txns_24h_sells,
                    pair_address, provider_chain_id, dex_id, url, pair_blockchain_created_at, image_url, header_image_url,
                    market_data_last_fetched_at, market_data_first_fetched_at
             FROM market_dexscreener WHERE chain_id = ?1 AND mint = ?2",
            )
            .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to prepare".to_owned(), message: e.to_string() }))?;

        let result = stmt.query_row(params![self.chain_id(), mint], |row| {
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
                price_usd: row.get::<_, Option<f64>>(0)?.unwrap_or_default(),
                price_sol: row.get::<_, Option<f64>>(1)?.unwrap_or_default(),
                price_native: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "0".to_owned()),
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
            Err(e) => Err(Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })),
        }
    }

    // ========================================================================
    // GECKOTERMINAL DATA OPERATIONS
    // ========================================================================

    /// Insert or update GeckoTerminal market data for a token
    pub fn upsert_geckoterminal_data(
        &self,
        mint: &str,
        data: &GeckoTerminalData,
    ) -> TokenResult<()> {
        let conn = self.conn()?;

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM market_geckoterminal WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
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
                "SELECT market_data_first_fetched_at FROM market_geckoterminal WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        // Clean schema insert (image_url column included)
        let insert_result = conn.execute(
            "INSERT INTO market_geckoterminal (
                chain_id, mint, price_usd, price_sol, price_native,
                price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                market_cap, fdv, liquidity_usd,
                volume_5m, volume_1h, volume_6h, volume_24h,
                pool_count, top_pool_address, reserve_in_usd, image_url,
                market_data_last_fetched_at, market_data_first_fetched_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)
             ON CONFLICT(chain_id, mint) DO UPDATE SET
                price_usd = excluded.price_usd, price_sol = excluded.price_sol, price_native = excluded.price_native,
                price_change_5m = excluded.price_change_5m, price_change_1h = excluded.price_change_1h, price_change_6h = excluded.price_change_6h, price_change_24h = excluded.price_change_24h,
                market_cap = excluded.market_cap, fdv = excluded.fdv, liquidity_usd = excluded.liquidity_usd,
                volume_5m = excluded.volume_5m, volume_1h = excluded.volume_1h, volume_6h = excluded.volume_6h, volume_24h = excluded.volume_24h,
                pool_count = excluded.pool_count, top_pool_address = excluded.top_pool_address, reserve_in_usd = excluded.reserve_in_usd, image_url = excluded.image_url, market_data_last_fetched_at = excluded.market_data_last_fetched_at",
            params![
                self.chain_id(), mint, data.price_usd, data.price_sol, &data.price_native,
                data.price_change_5m, data.price_change_1h, data.price_change_6h, data.price_change_24h,
                data.market_cap, data.fdv, data.liquidity_usd,
                data.volume_5m, data.volume_1h, data.volume_6h, data.volume_24h,
                data.pool_count.map(|c| c as i64), &data.top_pool_address, data.reserve_in_usd,
                &data.image_url, now_ts, first_fetched_ts,
            ],
        );

        insert_result.map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to upsert GeckoTerminal data".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Update in-memory cache
        store::store_geckoterminal(self.chain(), mint, data);

        Ok(())
    }

    /// Get GeckoTerminal market data
    pub fn get_geckoterminal_data(&self, mint: &str) -> TokenResult<Option<GeckoTerminalData>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT price_usd, price_sol, price_native,
                    price_change_5m, price_change_1h, price_change_6h, price_change_24h,
                    market_cap, fdv, liquidity_usd,
                    volume_5m, volume_1h, volume_6h, volume_24h,
                    pool_count, top_pool_address, reserve_in_usd, image_url,
                    market_data_last_fetched_at, market_data_first_fetched_at
             FROM market_geckoterminal WHERE chain_id = ?1 AND mint = ?2",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let result = stmt.query_row(params![self.chain_id(), mint], |row| {
            let last_fetched_ts: i64 = row.get(18)?;
            let first_fetched_ts: i64 = row.get(19)?;

            Ok(GeckoTerminalData {
                // Some historical rows may have NULLs; treat missing numeric/text values as defaults
                price_usd: row.get::<_, Option<f64>>(0)?.unwrap_or_default(),
                price_sol: row.get::<_, Option<f64>>(1)?.unwrap_or_default(),
                price_native: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "0".to_owned()),
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
            Err(e) => Err(Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })),
        }
    }

    // ========================================================================
    // MARKET DATA FRESHNESS & ERRORS
    // ========================================================================

    /// Check if a token's market data is older than the given threshold
    pub fn is_market_data_stale(&self, mint: &str, threshold_seconds: i64) -> TokenResult<bool> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare("SELECT market_data_last_updated_at FROM update_tracking WHERE chain_id = ?1 AND mint = ?2")
            .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to prepare".to_owned(), message: e.to_string() }))?;

        let result: Result<i64, rusqlite::Error> =
            stmt.query_row(params![self.chain_id(), mint], |row| row.get(0));

        match result {
            Ok(last_update) => {
                let now = chrono::Utc::now().timestamp();
                let age = now - last_update;
                Ok(age > threshold_seconds)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true), // No update tracking = stale
            Err(e) => Err(Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })),
        }
    }

    /// Fetch token mints that have no market data records
    pub fn get_tokens_without_market_data(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
                 INNER JOIN update_tracking u ON t.chain_id = u.chain_id AND t.mint = u.mint
                 LEFT JOIN market_dexscreener md ON t.chain_id = md.chain_id AND t.mint = md.mint
                 LEFT JOIN market_geckoterminal mg ON t.chain_id = mg.chain_id AND t.mint = mg.mint
                 WHERE t.chain_id = ?1 AND u.market_data_update_count = 0
                 AND md.mint IS NULL
                 AND mg.mint IS NULL
                 AND (u.last_error_at IS NULL OR u.last_error_at < strftime('%s','now') - 180)
                 AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
                 ORDER BY u.priority DESC, t.first_discovered_at ASC
                 LIMIT ?2",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mints = stmt
            .query_map(params![self.chain_id(), limit], |row| row.get(0))
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        mints.collect::<Result<Vec<_>, _>>().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to collect".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Count tokens with permanent market data failure (not listed on any exchange)
    /// These tokens are excluded from market data update attempts
    pub fn count_permanent_market_failures(&self) -> TokenResult<u64> {
        let conn = self.conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM update_tracking WHERE chain_id = ?1 AND market_error_type = 'permanent'",
                params![self.chain_id()],
                |row| row.get(0),
            )
            .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to count".to_owned(), message: e.to_string() }))?;

        Ok(count as u64)
    }

    /// Record a market data fetch error for tracking and retry logic
    pub fn record_market_error(
        &self,
        mint: &str,
        message: &str,
        error_type: &str,
    ) -> TokenResult<u32> {
        let conn = self.conn()?;

        let now = Utc::now().timestamp();

        // Update error tracking with type and increment count
        conn.execute(
            "UPDATE update_tracking SET 
                last_error = ?1, 
                last_error_at = ?2, 
                market_error_count = market_error_count + 1,
                market_error_type = ?3
             WHERE chain_id = ?4 AND mint = ?5",
            params![message, now, error_type, self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to record market error".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Get the new error count
        let error_count: u32 = conn
            .query_row(
                "SELECT market_error_count FROM update_tracking WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |row| row.get(0),
            )
            .unwrap_or(1);

        Ok(error_count)
    }

    /// Mark a token as permanently failed for market data updates
    /// This only updates the error_type without incrementing the error count
    /// Used when a token has hit the failure threshold and should be excluded from updates
    pub fn mark_market_permanent(&self, mint: &str) -> TokenResult<()> {
        let conn = self.conn()?;

        conn.execute(
            "UPDATE update_tracking SET market_error_type = 'permanent' WHERE chain_id = ?1 AND mint = ?2",
            params![self.chain_id(), mint],
        )
        .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to mark market permanent".to_owned(), message: e.to_string() }))?;

        Ok(())
    }

    /// Mark market data as updated (called after successful DexScreener or GeckoTerminal fetch)
    pub fn mark_market_data_updated(&self, mint: &str) -> TokenResult<()> {
        let conn = self.conn()?;

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
             WHERE chain_id = ?2 AND mint = ?3",
            params![now, self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to mark market data updated".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }
}
