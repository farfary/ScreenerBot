use chrono::Utc;
use rusqlite::{params, params_from_iter};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::tokens::types::{TokenError, TokenMetadata, TokenResult};

use super::TokenDatabase;

impl TokenDatabase {
    // ========================================================================
    // TOKEN METADATA OPERATIONS
    // ========================================================================

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
    // TOKEN INFO BATCH QUERIES
    // ========================================================================

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

}
