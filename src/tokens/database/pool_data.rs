use chrono::{DateTime, Utc};
use rusqlite::params;
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::tokens::pools;
use crate::tokens::types::{
    TokenError, TokenPoolInfo, TokenPoolSources, TokenPoolsSnapshot, TokenResult,
};

use super::assembly::read_row_value;
use super::TokenDatabase;

impl TokenDatabase {
    pub fn replace_token_pools(&self, snapshot: &TokenPoolsSnapshot) -> TokenResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Failed to start transaction: {e}")))?;

        // Query existing first_seen_ts values BEFORE delete to preserve them
        let mut existing_first_seen: HashMap<String, i64> = HashMap::new();
        {
            let mut stmt = tx
                .prepare(
                    "SELECT pool_address, pool_data_first_seen_at FROM token_pools WHERE mint = ?1",
                )
                .map_err(|e| TokenError::Database(format!("Failed to prepare query: {e}")))?;

            let rows = stmt
                .query_map(params![&snapshot.mint], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|e| {
                    TokenError::Database(format!("Failed to query existing pools: {e}"))
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
        .map_err(|e| TokenError::Database(format!("Failed to clear token pools: {e}")))?;

        for pool in snapshot.pools.iter() {
            let sources_json = serde_json::to_string(&pool.sources).map_err(|e| {
                TokenError::Database(format!("Failed to serialize pool sources: {e}"))
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
            .map_err(|e| TokenError::Database(format!("Failed to insert token pool: {e}")))?;
        }

        tx.commit().map_err(|e| {
            TokenError::Database(format!("Failed to commit pool transaction: {e}"))
        })?;

        Ok(())
    }

    /// Load pool snapshot for a token (if any pools stored)
    pub fn get_token_pools(&self, mint: &str) -> TokenResult<Option<TokenPoolsSnapshot>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, dex, base_mint, quote_mint, is_sol_pair,
                        liquidity_usd, liquidity_token, liquidity_sol, volume_h24,
                        price_usd, price_sol, price_native, sources_json,
                        pool_data_last_fetched_at, pool_data_first_seen_at
                 FROM token_pools WHERE mint = ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mut rows = stmt
            .query(params![mint])
            .map_err(|e| TokenError::Database(format!("Failed to query pools: {e}")))?;

        let mut pools: Vec<TokenPoolInfo> = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| TokenError::Database(format!("Failed to read row: {e}")))?
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
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, pool_address, dex, base_mint, quote_mint, is_sol_pair,
                        liquidity_usd, liquidity_token, liquidity_sol, volume_h24,
                        price_usd, price_sol, price_native, sources_json,
                        pool_data_last_fetched_at, pool_data_first_seen_at
                 FROM token_pools ORDER BY mint",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mut rows = stmt
            .query([])
            .map_err(|e| TokenError::Database(format!("Failed to query pools: {e}")))?;

        let mut snapshots: Vec<TokenPoolsSnapshot> = Vec::new();
        let mut current_mint: Option<String> = None;
        let mut current_pools: Vec<TokenPoolInfo> = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| TokenError::Database(format!("Failed to read row: {e}")))?
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
}
