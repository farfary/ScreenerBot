// token_cache.rs - SQLite database caching for tokens
use crate::global::Token;
use rusqlite::{ Connection, OptionalExtension, params };
use std::error::Error;
use chrono::{ DateTime, Utc };
use serde_json;
use crate::logger::{ log, LogTag };

/// SQLite database path for token cache
const TOKEN_DB_PATH: &str = "tokens.db";

/// Token database cache with SQLite backend
pub struct TokenDatabase {
    conn: Connection,
}

impl TokenDatabase {
    /// Create a new database connection and initialize tables
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let conn = Connection::open(TOKEN_DB_PATH)?;

        // Create tokens table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                decimals INTEGER NOT NULL,
                chain TEXT NOT NULL,
                data TEXT NOT NULL,
                first_seen TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                discovery_source TEXT NOT NULL,
                UNIQUE(mint)
            )",
            []
        )?;

        // Create indexes for better performance
        conn.execute("CREATE INDEX IF NOT EXISTS idx_symbol ON tokens(symbol)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_name ON tokens(name)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_first_seen ON tokens(first_seen)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_last_updated ON tokens(last_updated)", [])?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_discovery_source ON tokens(discovery_source)",
            []
        )?;

        // Create token stats table for usage tracking
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_stats (
                mint TEXT PRIMARY KEY,
                first_seen TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                access_count INTEGER DEFAULT 0,
                discovery_count INTEGER DEFAULT 0,
                FOREIGN KEY (mint) REFERENCES tokens(mint)
            )",
            []
        )?;

        log(LogTag::System, "SUCCESS", &format!("Initialized token database at {}", TOKEN_DB_PATH));

        Ok(Self { conn })
    }

    /// Add or update a token in the database
    pub fn add_or_update_token(
        &self,
        token: &Token,
        discovery_source: &str
    ) -> Result<bool, Box<dyn Error>> {
        let now = Utc::now().to_rfc3339();
        let token_data = serde_json::to_string(token)?;

        // Check if token exists
        let exists = self.get_token(&token.mint)?.is_some();

        if exists {
            // Update existing token
            self.conn.execute(
                "UPDATE tokens SET 
                    symbol = ?1, name = ?2, decimals = ?3, chain = ?4, 
                    data = ?5, last_updated = ?6, discovery_source = ?7
                WHERE mint = ?8",
                params![
                    token.symbol,
                    token.name,
                    token.decimals,
                    token.chain,
                    token_data,
                    now,
                    discovery_source,
                    token.mint
                ]
            )?;

            // Update stats
            self.conn.execute(
                "UPDATE token_stats SET 
                    last_updated = ?1, discovery_count = discovery_count + 1
                WHERE mint = ?2",
                params![now, token.mint]
            )?;

            log(
                LogTag::System,
                "UPDATE",
                &format!(
                    "Updated token {} ({}) from {} - Liquidity USD: {}",
                    token.symbol,
                    token.mint,
                    discovery_source,
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .map(|usd| format!("{:.2}", usd))
                        .unwrap_or_else(|| "None".to_string())
                )
            );
        } else {
            // Insert new token
            self.conn.execute(
                "INSERT INTO tokens (mint, symbol, name, decimals, chain, data, first_seen, last_updated, discovery_source)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    token.mint,
                    token.symbol,
                    token.name,
                    token.decimals,
                    token.chain,
                    token_data,
                    now,
                    now,
                    discovery_source
                ]
            )?;

            // Insert stats
            self.conn.execute(
                "INSERT INTO token_stats (mint, first_seen, last_updated, access_count, discovery_count)
                VALUES (?1, ?2, ?3, 0, 1)",
                params![token.mint, now, now]
            )?;

            log(
                LogTag::System,
                "CACHE",
                &format!(
                    "Cached new token {} ({}) from {} - Liquidity USD: {}",
                    token.symbol,
                    token.mint,
                    discovery_source,
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .map(|usd| format!("{:.2}", usd))
                        .unwrap_or_else(|| "None".to_string())
                )
            );
        }

        Ok(!exists) // Return true if it was a new token
    }

    /// Get token by mint address
    pub fn get_token(&self, mint: &str) -> Result<Option<Token>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare("SELECT data FROM tokens WHERE mint = ?1")?;

        let result: Result<Option<Token>, _> = stmt
            .query_row(params![mint], |row| {
                let data: String = row.get(0)?;
                let token: Token = serde_json
                    ::from_str(&data)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "JSON parse error".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?;

                // Update access count
                let _ = self.conn.execute(
                    "UPDATE token_stats SET access_count = access_count + 1 WHERE mint = ?1",
                    params![mint]
                );

                Ok(token)
            })
            .optional();

        match result {
            Ok(Some(token)) => Ok(Some(token)),
            Ok(None) => Ok(None),
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed to get token {}: {}", mint, e));
                Ok(None)
            }
        }
    }

    /// Get all tokens from database (for statistics or bulk operations)
    pub fn get_all_tokens(&self) -> Result<Vec<Token>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare("SELECT data FROM tokens")?;
        let token_iter = stmt.query_map([], |row| {
            let data: String = row.get(0)?;
            let token: Token = serde_json
                ::from_str(&data)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "JSON parse error".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?;
            Ok(token)
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            match token {
                Ok(t) => tokens.push(t),
                Err(e) =>
                    log(LogTag::System, "WARN", &format!("Failed to parse token from DB: {}", e)),
            }
        }

        Ok(tokens)
    }

    /// Get tokens discovered after a specific timestamp
    pub fn get_tokens_since(&self, since: DateTime<Utc>) -> Result<Vec<Token>, Box<dyn Error>> {
        let since_str = since.to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT data FROM tokens WHERE first_seen > ?1 ORDER BY first_seen DESC"
        )?;

        let token_iter = stmt.query_map(params![since_str], |row| {
            let data: String = row.get(0)?;
            let token: Token = serde_json
                ::from_str(&data)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "JSON parse error".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?;
            Ok(token)
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            match token {
                Ok(t) => tokens.push(t),
                Err(e) =>
                    log(LogTag::System, "WARN", &format!("Failed to parse token from DB: {}", e)),
            }
        }

        Ok(tokens)
    }

    /// Search tokens by symbol or name (for swap detection)
    pub fn search_tokens(&self, query: &str) -> Result<Vec<Token>, Box<dyn Error>> {
        let search_pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = self.conn.prepare(
            "SELECT data FROM tokens 
             WHERE LOWER(symbol) LIKE ?1 OR LOWER(name) LIKE ?1 
             ORDER BY last_updated DESC LIMIT 50"
        )?;

        let token_iter = stmt.query_map(params![search_pattern], |row| {
            let data: String = row.get(0)?;
            let token: Token = serde_json
                ::from_str(&data)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "JSON parse error".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?;
            Ok(token)
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            match token {
                Ok(t) => tokens.push(t),
                Err(e) =>
                    log(LogTag::System, "WARN", &format!("Failed to parse token from DB: {}", e)),
            }
        }

        Ok(tokens)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<TokenDatabaseStats, Box<dyn Error>> {
        let total_tokens: i64 = self.conn.query_row("SELECT COUNT(*) FROM tokens", [], |row|
            row.get(0)
        )?;

        let unique_sources: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT discovery_source) FROM tokens",
            [],
            |row| row.get(0)
        )?;

        let total_accesses: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(access_count), 0) FROM token_stats",
            [],
            |row| row.get(0)
        )?;

        // Get most recent token
        let most_recent: Option<String> = self.conn
            .query_row("SELECT first_seen FROM tokens ORDER BY first_seen DESC LIMIT 1", [], |row|
                row.get(0)
            )
            .optional()?;

        Ok(TokenDatabaseStats {
            total_tokens,
            unique_sources,
            total_accesses,
            most_recent_token: most_recent,
        })
    }

    /// Clean up old tokens (optional maintenance)
    pub fn cleanup_old_tokens(&self, older_than_days: i64) -> Result<usize, Box<dyn Error>> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days);
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self.conn.execute(
            "DELETE FROM tokens WHERE last_updated < ?1",
            params![cutoff_str]
        )?;

        if deleted > 0 {
            log(
                LogTag::System,
                "CLEANUP",
                &format!("Cleaned up {} old tokens older than {} days", deleted, older_than_days)
            );
        }

        Ok(deleted)
    }
}

/// Token database statistics
#[derive(Debug)]
pub struct TokenDatabaseStats {
    pub total_tokens: i64,
    pub unique_sources: i64,
    pub total_accesses: i64,
    pub most_recent_token: Option<String>,
}

impl std::fmt::Display for TokenDatabaseStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Token DB Stats: {} tokens, {} sources, {} accesses, latest: {}",
            self.total_tokens,
            self.unique_sources,
            self.total_accesses,
            self.most_recent_token.as_deref().unwrap_or("None")
        )
    }
}
