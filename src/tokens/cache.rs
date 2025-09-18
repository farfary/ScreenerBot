// Token database persistence module.
use crate::global::{ is_debug_monitor_enabled, TOKENS_DATABASE };
use crate::logger::{ log, LogTag };
use crate::tokens::types::*;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{ Arc, Mutex };

// =============================================================================
// TOKEN DATABASE (SQLite)
// =============================================================================

use crate::tokens::types::ApiToken;
use rusqlite::{ params, Connection, Result as SqliteResult };

/// SQLite database for token storage and caching
#[derive(Clone)]
pub struct TokenDatabase {
    connection: Arc<Mutex<Connection>>,
}

// Manually implement Send and Sync since Arc<Mutex<Connection>> is Send + Sync
unsafe impl Send for TokenDatabase {}
unsafe impl Sync for TokenDatabase {}

/// Configure database connection for optimal performance and concurrency
fn configure_database_connection(connection: &Connection) -> Result<(), rusqlite::Error> {
    // Use rusqlite pragma_update APIs to avoid statements that return rows
    // Set Write-Ahead Logging for better concurrency
    connection.pragma_update(None, "journal_mode", "WAL")?;
    // Reasonable durability/perf tradeoff
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    // Use memory for temp storage
    connection.pragma_update(None, "temp_store", "memory")?;
    // Increase cache size (number of pages if positive; SQLite also accepts negative for KB)
    connection.pragma_update(None, "cache_size", 10000)?;
    // Set busy timeout for lock contention
    connection.busy_timeout(std::time::Duration::from_millis(30_000))?;
    Ok(())
}

/// Create a properly configured database connection
pub fn create_configured_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    let connection = Connection::open(TOKENS_DATABASE)?;
    configure_database_connection(&connection)?;
    Ok(connection)
}

impl TokenDatabase {
    /// Create new token database instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = create_configured_connection()?;

        // Create tables if they don't exist
        connection.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                dex_id TEXT,
                pair_address TEXT,
                pair_url TEXT,
                price_native REAL NOT NULL,
                price_usd REAL NOT NULL,
                price_sol REAL,
                liquidity_usd REAL,
                liquidity_base REAL,
                liquidity_quote REAL,
                volume_h24 REAL,
                volume_h6 REAL,
                volume_h1 REAL,
                volume_m5 REAL,
                txns_h24_buys INTEGER,
                txns_h24_sells INTEGER,
                txns_h6_buys INTEGER,
                txns_h6_sells INTEGER,
                txns_h1_buys INTEGER,
                txns_h1_sells INTEGER,
                txns_m5_buys INTEGER,
                txns_m5_sells INTEGER,
                price_change_h24 REAL,
                price_change_h6 REAL,
                price_change_h1 REAL,
                price_change_m5 REAL,
                fdv REAL,
                market_cap REAL,
                pair_created_at INTEGER,
                boosts_active INTEGER,
                info_image_url TEXT,
                labels TEXT,
                last_updated TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            []
        )?;

        // Create indexes for better performance
        connection.execute("CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol)", [])?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_liquidity ON tokens(liquidity_usd DESC)",
            []
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_last_updated ON tokens(last_updated)",
            []
        )?;

        // Helpful index for boost selection: pair_created_at recency
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_pair_created_at ON tokens(pair_created_at)",
            []
        )?;

        // Only log on first initialization - reduce log spam
        static DATABASE_INITIALIZED: std::sync::Once = std::sync::Once::new();
        DATABASE_INITIALIZED.call_once(|| {
            log(LogTag::Cache, "DATABASE", "Token database initialized");
        });

        let database = Self {
            connection: Arc::new(Mutex::new(connection)),
        };

        // Run database schema migrations on startup
        if let Err(e) = database.migrate_database_schemas() {
            log(LogTag::Cache, "MIGRATION_ERROR", &format!("Database migration failed: {}", e));
        }

        Ok(database)
    }

    /// Add new tokens to database
    pub async fn add_tokens(&self, tokens: &[ApiToken]) -> Result<(), Box<dyn std::error::Error>> {
        for token in tokens {
            self.insert_or_update_token(token)?;
        }

        if is_debug_monitor_enabled() {
            log(LogTag::Cache, "DATABASE", &format!("Added/updated {} tokens", tokens.len()));
        }

        Ok(())
    }

    /// Update existing tokens in database
    pub async fn update_tokens(&self, tokens: &[ApiToken]) -> Result<(), String> {
        for token in tokens {
            self
                .insert_or_update_token(token)
                .map_err(|e| format!("Failed to update token: {}", e))?;
        }

        // Only log on errors or significant updates (> 50 tokens)
        if tokens.len() > 50 {
            log(LogTag::Cache, "DATABASE", &format!("Updated {} tokens", tokens.len()));
        }
        Ok(())
    }

    /// Delete tokens from database by mint addresses
    /// This also deletes related records to handle foreign key constraints
    pub async fn delete_tokens(&self, mints: &[String]) -> Result<usize, String> {
        if mints.is_empty() {
            return Ok(0);
        }

        let placeholders = mints
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let connection = self.connection.lock().map_err(|e| format!("Database lock error: {}", e))?;

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        for mint in mints {
            params.push(mint);
        }

        // Start transaction to ensure atomicity
        connection
            .execute("BEGIN TRANSACTION", [])
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        // First delete from tables that have foreign key references to tokens
        // Delete from liquidity_tracking
        let liquidity_query =
            format!("DELETE FROM liquidity_tracking WHERE mint IN ({})", placeholders);
        let liquidity_deleted = connection
            .prepare(&liquidity_query)
            .map_err(|e| format!("Failed to prepare liquidity_tracking delete: {}", e))?
            .execute(&params[..])
            .map_err(|e| format!("Failed to delete from liquidity_tracking: {}", e))?;

        // Delete from route_failure_tracking
        let route_query =
            format!("DELETE FROM route_failure_tracking WHERE mint IN ({})", placeholders);
        let route_deleted = connection
            .prepare(&route_query)
            .map_err(|e| format!("Failed to prepare route_failure_tracking delete: {}", e))?
            .execute(&params[..])
            .map_err(|e| format!("Failed to delete from route_failure_tracking: {}", e))?;

        // Finally delete from tokens table
        let token_query = format!("DELETE FROM tokens WHERE mint IN ({})", placeholders);
        let deleted_count = connection
            .prepare(&token_query)
            .map_err(|e| format!("Failed to prepare tokens delete: {}", e))?
            .execute(&params[..])
            .map_err(|e| format!("Failed to delete from tokens: {}", e))?;

        // Commit transaction
        connection
            .execute("COMMIT", [])
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        if is_debug_monitor_enabled() && (liquidity_deleted > 0 || route_deleted > 0) {
            log(
                LogTag::Monitor,
                "CLEANUP",
                &format!(
                    "Deleted {} liquidity_tracking + {} route_failure_tracking records for {} tokens",
                    liquidity_deleted,
                    route_deleted,
                    deleted_count
                )
            );
        }

        if deleted_count > 0 {
            log(
                LogTag::Cache,
                "DATABASE",
                &format!("Deleted {} stale tokens from database", deleted_count)
            );
        }

        Ok(deleted_count)
    }

    /// Small helper for monitor: get a few very new tokens that are stale enough for a quick recheck
    /// Returns up to `limit` mint addresses for tokens whose pair_created_at is within `max_age_minutes`
    /// and whose last_updated is at least `min_stale_minutes` ago.
    pub async fn get_new_tokens_needing_boost(
        &self,
        max_age_minutes: i64,
        min_stale_minutes: i64,
        limit: usize
    ) -> Result<Vec<String>, String> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let connection = self.connection.lock().map_err(|e| format!("Database lock error: {}", e))?;

        let now = chrono::Utc::now();
        let min_created_at = now - chrono::Duration::minutes(max_age_minutes);
        let max_last_updated = now - chrono::Duration::minutes(min_stale_minutes);

        // pair_created_at is stored as INTEGER (epoch ms) in DexScreener data; some entries may be NULL
        let min_created_ms = min_created_at.timestamp_millis();
        let max_last_updated_str = max_last_updated.to_rfc3339();

        let mut stmt = connection
            .prepare(
                "SELECT mint FROM tokens 
                 WHERE pair_created_at IS NOT NULL 
                   AND pair_created_at >= ?1
                   AND last_updated <= ?2
                 ORDER BY pair_created_at DESC
                 LIMIT ?3"
            )
            .map_err(|e| format!("Failed to prepare boost query: {}", e))?;

        let rows = stmt
            .query_map(
                rusqlite::params![min_created_ms, max_last_updated_str, limit as i64],
                |row| { Ok(row.get::<_, String>("mint")?) }
            )
            .map_err(|e| format!("Failed to execute boost query: {}", e))?;

        let mut mints = Vec::new();
        for r in rows {
            mints.push(r.map_err(|e| format!("Failed to parse mint: {}", e))?);
        }
        Ok(mints)
    }

    /// Get all tokens from database
    pub async fn get_all_tokens(&self) -> Result<Vec<ApiToken>, String> {
        let connection = self.connection.lock().map_err(|e| format!("Database lock error: {}", e))?;

        let mut stmt = connection
            .prepare("SELECT * FROM tokens ORDER BY liquidity_usd DESC")
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let token_iter = stmt
            .query_map([], |row| Ok(self.row_to_token(row)?))
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token.map_err(|e| format!("Failed to parse token: {}", e))?);
        }

        Ok(tokens)
    }

    /// Get tokens by mints
    pub async fn get_tokens_by_mints(
        &self,
        mints: &[String]
    ) -> Result<Vec<ApiToken>, Box<dyn std::error::Error>> {
        let mut tokens = Vec::new();

        for mint in mints {
            if let Some(token) = self.get_token_by_mint(mint)? {
                tokens.push(token);
            }
        }

        Ok(tokens)
    }

    /// Get single token by mint
    pub fn get_token_by_mint(
        &self,
        mint: &str
    ) -> Result<Option<ApiToken>, Box<dyn std::error::Error>> {
        let connection = self.connection
            .lock()
            .map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database lock error: {}", e)
                    )
                ) as Box<dyn std::error::Error>
            })?;
        let mut stmt = connection.prepare("SELECT * FROM tokens WHERE mint = ?1")?;

        let mut rows = stmt.query_map(params![mint], |row| Ok(self.row_to_token(row)?))?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Get tokens by liquidity threshold for new entry detection
    pub async fn get_tokens_by_liquidity_threshold(
        &self,
        threshold: f64
    ) -> Result<Vec<ApiToken>, Box<dyn std::error::Error>> {
        let connection = self.connection
            .lock()
            .map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database lock error: {}", e)
                    )
                ) as Box<dyn std::error::Error>
            })?;

        let mut stmt = connection.prepare(
            "SELECT * FROM tokens 
             WHERE liquidity_usd >= ?1 
             ORDER BY liquidity_usd DESC"
        )?;

        let rows = stmt.query_map(params![threshold], |row| Ok(self.row_to_token(row)?))?;

        let mut tokens = Vec::new();
        for row in rows {
            tokens.push(row?);
        }

        Ok(tokens)
    }

    /// Insert or update token in database
    fn insert_or_update_token(&self, token: &ApiToken) -> Result<(), Box<dyn std::error::Error>> {
        let labels_json = token.labels
            .as_ref()
            .map(|labels| serde_json::to_string(labels).unwrap_or_default())
            .unwrap_or_default();

        let connection = self.connection
            .lock()
            .map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database lock error: {}", e)
                    )
                ) as Box<dyn std::error::Error>
            })?;
        connection.execute(
            "INSERT OR REPLACE INTO tokens (
                mint, symbol, name, chain_id, dex_id, pair_address, pair_url,
                price_native, price_usd, price_sol,
                liquidity_usd, liquidity_base, liquidity_quote,
                volume_h24, volume_h6, volume_h1, volume_m5,
                txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
                txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
                price_change_h24, price_change_h6, price_change_h1, price_change_m5,
                fdv, market_cap, pair_created_at, boosts_active,
                info_image_url, labels, last_updated
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36
            )",
            params![
                token.mint,
                token.symbol,
                token.name,
                token.chain_id,
                token.dex_id,
                token.pair_address,
                token.pair_url,
                token.price_native,
                token.price_usd,
                token.price_sol,
                token.liquidity.as_ref().and_then(|l| l.usd),
                token.liquidity.as_ref().and_then(|l| l.base),
                token.liquidity.as_ref().and_then(|l| l.quote),
                token.volume.as_ref().and_then(|v| v.h24),
                token.volume.as_ref().and_then(|v| v.h6),
                token.volume.as_ref().and_then(|v| v.h1),
                token.volume.as_ref().and_then(|v| v.m5),
                token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.sells)),
                token.price_change.as_ref().and_then(|p| p.h24),
                token.price_change.as_ref().and_then(|p| p.h6),
                token.price_change.as_ref().and_then(|p| p.h1),
                token.price_change.as_ref().and_then(|p| p.m5),
                token.fdv,
                token.market_cap,
                token.pair_created_at,
                token.boosts.as_ref().and_then(|b| b.active),
                token.info.as_ref().and_then(|i| i.image_url.clone()),
                labels_json,
                token.last_updated.to_rfc3339()
            ]
        )?;

        Ok(())
    }

    /// Convert database row to ApiToken
    fn row_to_token(&self, row: &rusqlite::Row) -> SqliteResult<ApiToken> {
        let labels_json: String = row.get("labels")?;
        let labels = if labels_json.is_empty() {
            None
        } else {
            serde_json::from_str(&labels_json).ok()
        };

        let last_updated_str: String = row.get("last_updated")?;
        let last_updated = chrono::DateTime
            ::parse_from_rfc3339(&last_updated_str)
            .map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "last_updated".to_string(),
                    rusqlite::types::Type::Text
                )
            })?
            .with_timezone(&chrono::Utc);

        Ok(ApiToken {
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            // decimals removed - only use decimal_cache.json
            chain_id: row.get("chain_id")?,
            dex_id: row.get("dex_id")?,
            pair_address: row.get("pair_address")?,
            pair_url: row.get("pair_url")?,
            price_native: row.get("price_native")?,
            price_usd: row.get("price_usd")?,
            price_sol: row.get("price_sol")?,
            liquidity: Some(crate::tokens::types::LiquidityInfo {
                usd: row.get("liquidity_usd")?,
                base: row.get("liquidity_base")?,
                quote: row.get("liquidity_quote")?,
            }),
            volume: Some(crate::tokens::types::VolumeStats {
                h24: row.get("volume_h24")?,
                h6: row.get("volume_h6")?,
                h1: row.get("volume_h1")?,
                m5: row.get("volume_m5")?,
            }),
            txns: Some(crate::tokens::types::TxnStats {
                h24: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_h24_buys")?,
                    sells: row.get("txns_h24_sells")?,
                }),
                h6: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_h6_buys")?,
                    sells: row.get("txns_h6_sells")?,
                }),
                h1: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_h1_buys")?,
                    sells: row.get("txns_h1_sells")?,
                }),
                m5: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_m5_buys")?,
                    sells: row.get("txns_m5_sells")?,
                }),
            }),
            price_change: Some(crate::tokens::types::PriceChangeStats {
                h24: row.get("price_change_h24")?,
                h6: row.get("price_change_h6")?,
                h1: row.get("price_change_h1")?,
                m5: row.get("price_change_m5")?,
            }),
            fdv: row.get("fdv")?,
            market_cap: row.get("market_cap")?,
            pair_created_at: row.get("pair_created_at")?,
            boosts: Some(crate::tokens::types::BoostInfo {
                active: row.get("boosts_active")?,
            }),
            info: Some(crate::tokens::types::TokenInfo {
                address: row.get::<_, String>("mint")?,
                name: row.get::<_, String>("name")?,
                symbol: row.get::<_, String>("symbol")?,
                image_url: row.get("info_image_url")?,
                websites: None, // Not stored in simplified schema
                socials: None, // Not stored in simplified schema
            }),
            labels,
            last_updated,
        })
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        let connection = self.connection
            .lock()
            .map_err(|e| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database lock error: {}", e)
                    )
                ) as Box<dyn std::error::Error>
            })?;
        let mut stmt = connection.prepare("SELECT COUNT(*) FROM tokens")?;
        let total_tokens: i64 = stmt.query_row([], |row| row.get(0))?;

        let mut stmt = connection.prepare("SELECT COUNT(*) FROM tokens WHERE liquidity_usd > 100")?;
        let tokens_with_liquidity: i64 = stmt.query_row([], |row| row.get(0))?;

        Ok(DatabaseStats {
            total_tokens: total_tokens as usize,
            tokens_with_liquidity: tokens_with_liquidity as usize,
            last_updated: chrono::Utc::now(),
        })
    }

    /// Check if a token has security issues that warrant removal
    /// Returns Some(reason) if token should be removed, None otherwise
    /// Get all tokens with their last update times for monitoring
    /// Returns tokens ordered by liquidity (highest first) with update time information
    pub async fn get_all_tokens_with_update_time(
        &self
    ) -> Result<Vec<(String, String, DateTime<Utc>, f64)>, String> {
        let connection = self.connection.lock().map_err(|e| format!("Database lock error: {}", e))?;

        let mut stmt = connection
            .prepare(
                "SELECT mint, symbol, last_updated, COALESCE(liquidity_usd, 0.0) as liquidity
                 FROM tokens 
                 ORDER BY liquidity_usd DESC NULLS LAST"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let token_iter = stmt
            .query_map([], |row| {
                let last_updated_str: String = row.get("last_updated")?;
                let last_updated = chrono::DateTime
                    ::parse_from_rfc3339(&last_updated_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "last_updated".to_string(),
                            rusqlite::types::Type::Text
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                Ok((
                    row.get::<_, String>("mint")?,
                    row.get::<_, String>("symbol")?,
                    last_updated,
                    row.get::<_, f64>("liquidity")?,
                ))
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token.map_err(|e| format!("Failed to parse token: {}", e))?);
        }

        Ok(tokens)
    }

    /// Get tokens that need updating based on time criteria
    /// Returns tokens that haven't been updated within the specified hours
    pub async fn get_tokens_needing_update(
        &self,
        min_hours_since_update: i64
    ) -> Result<Vec<(String, String, DateTime<Utc>, f64)>, String> {
        let connection = self.connection.lock().map_err(|e| format!("Database lock error: {}", e))?;

        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(min_hours_since_update);
        let cutoff_str = cutoff_time.to_rfc3339();

        let mut stmt = connection
            .prepare(
                "SELECT mint, symbol, last_updated, COALESCE(liquidity_usd, 0.0) as liquidity
                 FROM tokens 
                 WHERE last_updated < ?1
                 ORDER BY liquidity_usd DESC NULLS LAST"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let token_iter = stmt
            .query_map([&cutoff_str], |row| {
                let last_updated_str: String = row.get("last_updated")?;
                let last_updated = chrono::DateTime
                    ::parse_from_rfc3339(&last_updated_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "last_updated".to_string(),
                            rusqlite::types::Type::Text
                        )
                    })?
                    .with_timezone(&chrono::Utc);

                Ok((
                    row.get::<_, String>("mint")?,
                    row.get::<_, String>("symbol")?,
                    last_updated,
                    row.get::<_, f64>("liquidity")?,
                ))
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token.map_err(|e| format!("Failed to parse token: {}", e))?);
        }

        Ok(tokens)
    }

    /// Ensure database schemas are up to date - run this at startup
    pub fn migrate_database_schemas(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Use a separate configured connection for migration to avoid long-held locks
        let migration_conn = create_configured_connection()?;

        // Check if failed_decimals table has retry_count column
        let has_retry_count = {
            match migration_conn.prepare("SELECT retry_count FROM failed_decimals LIMIT 1") {
                Ok(mut stmt) => {
                    // Try to execute the query to see if the column exists
                    stmt.query_map([], |_| Ok(())).is_ok()
                }
                Err(_) => false,
            }
        };

        if !has_retry_count {
            log(
                LogTag::Cache,
                "MIGRATION",
                "Migrating failed_decimals table to add retry_count columns"
            );

            // Add missing columns to failed_decimals table
            migration_conn
                .execute(
                    "ALTER TABLE failed_decimals ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
                    []
                )
                .unwrap_or_else(|e| {
                    log(
                        LogTag::Cache,
                        "MIGRATION_WARN",
                        &format!("Could not add retry_count column (may already exist): {}", e)
                    );
                    0
                });

            migration_conn
                .execute(
                    "ALTER TABLE failed_decimals ADD COLUMN max_retries INTEGER NOT NULL DEFAULT 3",
                    []
                )
                .unwrap_or_else(|e| {
                    log(
                        LogTag::Cache,
                        "MIGRATION_WARN",
                        &format!("Could not add max_retries column (may already exist): {}", e)
                    );
                    0
                });

            log(LogTag::Cache, "MIGRATION", "Failed_decimals table migration completed");
        }

        // Close migration connection explicitly
        drop(migration_conn);
        Ok(())
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_tokens: usize,
    pub tokens_with_liquidity: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}
