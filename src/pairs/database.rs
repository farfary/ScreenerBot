use crate::pairs::types::*;
use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ params, Connection, Row };
use serde::{ Deserialize, Serialize };
use std::sync::Mutex;

/// Database for caching DexScreener API responses
pub struct PairsDatabase {
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTokenPairs {
    pub token_address: String,
    pub pairs: Vec<TokenPair>,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl PairsDatabase {
    pub fn new() -> Result<Self> {
        let conn = Connection::open("cache_pairs.db").context(
            "Failed to open pairs cache database"
        )?;

        let db = Self {
            connection: Mutex::new(conn),
        };

        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        // Token pairs cache table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_pairs_cache (
                token_address TEXT PRIMARY KEY,
                pairs_json TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
            []
        )?;

        // Individual pair info cache for quick lookups
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pair_info_cache (
                pair_address TEXT PRIMARY KEY,
                token_address TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                dex_id TEXT NOT NULL,
                pair_json TEXT NOT NULL,
                price_usd REAL,
                liquidity_usd REAL,
                volume_24h REAL,
                cached_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                FOREIGN KEY (token_address) REFERENCES token_pairs_cache(token_address)
            )",
            []
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_pairs_expires_at ON token_pairs_cache(expires_at)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pair_info_token_address ON pair_info_cache(token_address)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pair_info_expires_at ON pair_info_cache(expires_at)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pair_info_dex_id ON pair_info_cache(dex_id)",
            []
        )?;

        Ok(())
    }

    /// Cache token pairs data
    pub fn cache_token_pairs(
        &self,
        token_address: &str,
        pairs: &[TokenPair],
        cache_duration_hours: i64
    ) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(cache_duration_hours);

        let pairs_json = serde_json::to_string(pairs).context("Failed to serialize pairs to JSON")?;

        // Insert or replace token pairs cache
        conn.execute(
            "INSERT OR REPLACE INTO token_pairs_cache (token_address, pairs_json, cached_at, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![token_address, pairs_json, now.to_rfc3339(), expires_at.to_rfc3339()]
        )?;

        // Cache individual pair info for quick lookups
        for pair in pairs {
            let pair_json = serde_json
                ::to_string(pair)
                .context("Failed to serialize pair to JSON")?;

            conn.execute(
                "INSERT OR REPLACE INTO pair_info_cache 
                 (pair_address, token_address, chain_id, dex_id, pair_json, price_usd, liquidity_usd, volume_24h, cached_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    pair.pair_address,
                    token_address,
                    pair.chain_id,
                    pair.dex_id,
                    pair_json,
                    pair.price_usd.parse::<f64>().ok(),
                    Some(pair.liquidity.usd),
                    Some(pair.volume.h24),
                    now.to_rfc3339(),
                    expires_at.to_rfc3339()
                ]
            )?;
        }

        Ok(())
    }

    /// Get cached token pairs if not expired
    pub fn get_cached_token_pairs(&self, token_address: &str) -> Result<Option<CachedTokenPairs>> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();

        let result = conn.query_row(
            "SELECT pairs_json, cached_at, expires_at FROM token_pairs_cache 
             WHERE token_address = ?1 AND expires_at > ?2",
            params![token_address, now.to_rfc3339()],
            |row| {
                let pairs_json: String = row.get(0)?;
                let cached_at_str: String = row.get(1)?;
                let expires_at_str: String = row.get(2)?;

                let pairs: Vec<TokenPair> = serde_json
                    ::from_str(&pairs_json)
                    .map_err(|e|
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "Invalid JSON".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?;

                let cached_at = DateTime::parse_from_rfc3339(&cached_at_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            1,
                            "Invalid datetime".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                let expires_at = DateTime::parse_from_rfc3339(&expires_at_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid datetime".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok(CachedTokenPairs {
                    token_address: token_address.to_string(),
                    pairs,
                    cached_at,
                    expires_at,
                })
            }
        );

        match result {
            Ok(cached) => Ok(Some(cached)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get pairs by DEX ID from cache
    pub fn get_cached_pairs_by_dex(&self, dex_id: &str) -> Result<Vec<TokenPair>> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();

        let mut stmt = conn.prepare(
            "SELECT pair_json FROM pair_info_cache 
             WHERE dex_id = ?1 AND expires_at > ?2
             ORDER BY liquidity_usd DESC NULLS LAST"
        )?;

        let pair_iter = stmt.query_map(params![dex_id, now.to_rfc3339()], |row| {
            let pair_json: String = row.get(0)?;
            let pair: TokenPair = serde_json
                ::from_str(&pair_json)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "Invalid JSON".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?;
            Ok(pair)
        })?;

        let mut pairs = Vec::new();
        for pair in pair_iter {
            pairs.push(pair?);
        }

        Ok(pairs)
    }

    /// Get top pairs by liquidity from cache
    pub fn get_top_pairs_by_liquidity(&self, limit: usize) -> Result<Vec<TokenPair>> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();

        let mut stmt = conn.prepare(
            "SELECT pair_json FROM pair_info_cache 
             WHERE expires_at > ?1 AND liquidity_usd IS NOT NULL
             ORDER BY liquidity_usd DESC
             LIMIT ?2"
        )?;

        let pair_iter = stmt.query_map(params![now.to_rfc3339(), limit], |row| {
            let pair_json: String = row.get(0)?;
            let pair: TokenPair = serde_json
                ::from_str(&pair_json)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "Invalid JSON".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?;
            Ok(pair)
        })?;

        let mut pairs = Vec::new();
        for pair in pair_iter {
            pairs.push(pair?);
        }

        Ok(pairs)
    }

    /// Clean expired cache entries
    pub fn clean_expired_cache(&self) -> Result<usize> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();

        let pairs_deleted = conn.execute(
            "DELETE FROM token_pairs_cache WHERE expires_at <= ?1",
            params![now.to_rfc3339()]
        )?;

        let pair_info_deleted = conn.execute(
            "DELETE FROM pair_info_cache WHERE expires_at <= ?1",
            params![now.to_rfc3339()]
        )?;

        Ok(pairs_deleted + pair_info_deleted)
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> Result<CacheStats> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();

        let total_tokens: u64 = conn.query_row("SELECT COUNT(*) FROM token_pairs_cache", [], |row|
            row.get(0)
        )?;

        let active_tokens: u64 = conn.query_row(
            "SELECT COUNT(*) FROM token_pairs_cache WHERE expires_at > ?1",
            params![now.to_rfc3339()],
            |row| row.get(0)
        )?;

        let total_pairs: u64 = conn.query_row("SELECT COUNT(*) FROM pair_info_cache", [], |row|
            row.get(0)
        )?;

        let active_pairs: u64 = conn.query_row(
            "SELECT COUNT(*) FROM pair_info_cache WHERE expires_at > ?1",
            params![now.to_rfc3339()],
            |row| row.get(0)
        )?;

        Ok(CacheStats {
            total_tokens,
            active_tokens,
            total_pairs,
            active_pairs,
            last_cleanup: now,
        })
    }

    /// Get cached pairs for a specific token
    pub fn get_cached_pairs_for_token(&self, token_address: &str) -> Result<Vec<TokenPair>> {
        if let Some(cached) = self.get_cached_token_pairs(token_address)? {
            Ok(cached.pairs)
        } else {
            Ok(Vec::new())
        }
    }

    /// Store individual pair in cache
    pub fn store_pair(&self, pair: &TokenPair, cache_duration_hours: i64) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(cache_duration_hours);

        let pair_json = serde_json::to_string(pair).context("Failed to serialize pair to JSON")?;

        conn.execute(
            "INSERT OR REPLACE INTO pair_info_cache 
             (pair_address, dex_id, pair_json, cached_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                pair.pair_address,
                pair.dex_id,
                pair_json,
                now.to_rfc3339(),
                expires_at.to_rfc3339()
            ]
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_tokens: u64,
    pub active_tokens: u64,
    pub total_pairs: u64,
    pub active_pairs: u64,
    pub last_cleanup: DateTime<Utc>,
}
