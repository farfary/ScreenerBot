// Token database and rugcheck persistence module.
// NOTE: Legacy in-memory PriceCache removed in favor of TokenPriceService (price.rs).
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

impl TokenDatabase {
    /// Create new token database instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::open(TOKENS_DATABASE)?;

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

        // Only log on first initialization - reduce log spam
        static DATABASE_INITIALIZED: std::sync::Once = std::sync::Once::new();
        DATABASE_INITIALIZED.call_once(|| {
            log(LogTag::System, "DATABASE", "Token database initialized");
        });

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Add new tokens to database
    pub async fn add_tokens(&self, tokens: &[ApiToken]) -> Result<(), Box<dyn std::error::Error>> {
        for token in tokens {
            self.insert_or_update_token(token)?;
        }

        if is_debug_monitor_enabled() {
            log(LogTag::System, "DATABASE", &format!("Added/updated {} tokens", tokens.len()));
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
            log(LogTag::System, "DATABASE", &format!("Updated {} tokens", tokens.len()));
        }
        Ok(())
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

    /// Cleanup tokens with zero liquidity from the database
    /// Only removes tokens that have zero liquidity AND are older than 1 hour
    /// This should only be called after fetching and updating latest token data
    pub async fn cleanup_zero_liquidity_tokens(&self) -> Result<usize, Box<dyn std::error::Error>> {
        // First, collect the candidate tokens (with database lock)
        let tokens_to_check = {
            let connection = self.connection
                .lock()
                .map_err(|_e| {
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Database lock error".to_string()
                        )
                    ) as Box<dyn std::error::Error>
                })?;

            // Calculate cutoff time (1 hour ago)
            let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
            let one_hour_ago_str = one_hour_ago.to_rfc3339();

            // Get tokens with zero liquidity that are older than 1 hour
            let mut stmt = connection.prepare(
                "SELECT mint, symbol, last_updated FROM tokens 
                 WHERE (liquidity_usd IS NULL OR liquidity_usd <= 0.0)
                 AND last_updated < ?1
                 ORDER BY last_updated ASC"
            )?;

            let token_rows = stmt.query_map([&one_hour_ago_str], |row| {
                Ok((
                    row.get::<_, String>("mint")?,
                    row.get::<_, String>("symbol")?,
                    row.get::<_, String>("last_updated")?,
                ))
            })?;

            let mut tokens_to_check = Vec::new();
            for row in token_rows {
                tokens_to_check.push(row?);
            }
            tokens_to_check
        }; // connection lock released here

        if tokens_to_check.is_empty() {
            return Ok(0);
        }

        log(
            LogTag::System,
            "CLEANUP",
            &format!("Found {} tokens with zero liquidity older than 1 hour", tokens_to_check.len())
        );

        // Check which tokens have open positions - we must not delete these
        // (this is done outside the database lock to avoid holding it across async calls)
        let mut tokens_to_delete = Vec::new();
        for (mint, symbol, last_updated) in tokens_to_check {
            // Check if this token has an open position
            if !self.has_open_position(&mint).await {
                tokens_to_delete.push((mint, symbol, last_updated));
            }
        }

        if tokens_to_delete.is_empty() {
            log(
                LogTag::System,
                "CLEANUP",
                "No tokens eligible for deletion (all have open positions)"
            );
            return Ok(0);
        }

        // Finally, delete the eligible tokens (with database lock)
        let deleted_count = {
            let connection = self.connection
                .lock()
                .map_err(|_e| {
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Database lock error".to_string()
                        )
                    ) as Box<dyn std::error::Error>
                })?;

            let mut deleted_count = 0;
            for (mint, symbol, last_updated) in &tokens_to_delete {
                match connection.execute("DELETE FROM tokens WHERE mint = ?1", params![mint]) {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            deleted_count += 1;

                            // Also delete rugcheck data for this token
                            if
                                let Err(e) = connection.execute(
                                    "DELETE FROM rugcheck_data WHERE mint = ?1",
                                    params![mint]
                                )
                            {
                                log(
                                    LogTag::System,
                                    "ERROR",
                                    &format!(
                                        "Failed to delete rugcheck data for token {}: {}",
                                        mint,
                                        e
                                    )
                                );
                            }

                            log(
                                LogTag::System,
                                "CLEANUP",
                                &format!(
                                    "Deleted stale zero liquidity token: {} ({}) - last updated: {}",
                                    symbol,
                                    mint,
                                    last_updated
                                )
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to delete token {}: {}", mint, e)
                        );
                    }
                }
            }
            deleted_count
        }; // connection lock released here

        if deleted_count > 0 {
            log(
                LogTag::System,
                "CLEANUP",
                &format!("Database cleanup: Removed {} stale tokens with zero liquidity (>1h old)", deleted_count)
            );
        } else {
            log(LogTag::System, "CLEANUP", "Database cleanup: No stale tokens removed");
        }

        Ok(deleted_count)
    }

    /// Cleanup tokens with near-zero liquidity AND security issues from the database
    /// For security issues: Moves tokens to blacklist table (preserves blacklist)
    /// For low liquidity: Removes tokens from main table after proper evaluation
    /// This should only be called after fetching and updating latest token data
    pub async fn cleanup_near_zero_liquidity_tokens(
        &self,
        threshold_usd: f64
    ) -> Result<usize, Box<dyn std::error::Error>> {
        use crate::tokens::blacklist::{ add_to_blacklist_db, is_token_blacklisted_db, BlacklistReason };
        
        // First, collect the candidate tokens (with database lock)
        let (liquidity_tokens, security_tokens) = {
            let connection = self.connection
                .lock()
                .map_err(|_e| {
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Database lock error".to_string()
                        )
                    ) as Box<dyn std::error::Error>
                })?;

            // Calculate cutoff time (1 hour ago)
            let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
            let one_hour_ago_str = one_hour_ago.to_rfc3339();

            // Get tokens with near-zero liquidity that are older than 1 hour
            let mut stmt = connection.prepare(
                "SELECT mint, symbol, last_updated FROM tokens 
                 WHERE (liquidity_usd IS NULL OR liquidity_usd < ?2)
                 AND last_updated < ?1
                 ORDER BY last_updated ASC"
            )?;

            let token_rows = stmt.query_map(params![one_hour_ago_str, threshold_usd], |row| {
                Ok((
                    row.get::<_, String>("mint")?,
                    row.get::<_, String>("symbol")?,
                    row.get::<_, String>("last_updated")?,
                ))
            })?;

            let mut liquidity_tokens = Vec::new();
            for row in token_rows {
                liquidity_tokens.push(row?);
            }

            // Also collect tokens with security issues regardless of age
            // Join with rugcheck_data to find tokens with problematic authorities
            let mut security_stmt = connection.prepare(
                "SELECT t.mint, t.symbol, t.last_updated FROM tokens t
                 INNER JOIN rugcheck_data r ON t.mint = r.mint
                 WHERE (
                     r.token_mint_authority IS NOT NULL OR 
                     r.token_freeze_authority IS NOT NULL OR
                     r.freeze_authority_json IS NOT NULL OR
                     r.mint_authority_json IS NOT NULL OR
                     r.risks_json LIKE '%mint%' OR 
                     r.risks_json LIKE '%freeze%' OR
                     r.risks_json LIKE '%authority%'
                 )"
            )?;

            let security_rows = security_stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>("mint")?,
                    row.get::<_, String>("symbol")?,
                    row.get::<_, String>("last_updated")?,
                ))
            })?;

            let mut security_tokens = Vec::new();
            for row in security_rows {
                security_tokens.push(row?);
            }

            (liquidity_tokens, security_tokens)
        }; // connection lock released here

        if liquidity_tokens.is_empty() && security_tokens.is_empty() {
            return Ok(0);
        }

        log(
            LogTag::System,
            "CLEANUP",
            &format!(
                "Found {} tokens with low liquidity (<${:.1}) and {} tokens with security issues",
                liquidity_tokens.len(),
                threshold_usd,
                security_tokens.len()
            )
        );

        // Process candidates: blacklist security issues, delete low liquidity
        // (this is done outside the database lock to avoid holding it across async calls)
        let mut tokens_to_blacklist = Vec::new();
        let mut tokens_to_delete = Vec::new();
        let mut blacklist_count = 0;
        let mut delete_count = 0;

        // Process security-based candidates - these go to BLACKLIST (not deleted)
        for (mint, symbol, last_updated) in security_tokens {
            if !self.has_open_position(&mint).await {
                // Skip tokens already blacklisted
                if !is_token_blacklisted_db(&mint) {
                    // Check if token has specific security issues
                    if let Some(reason) = self.check_security_issues(&mint).await {
                        let blacklist_reason = match reason.as_str() {
                            s if s.contains("mint") => BlacklistReason::ApiError, // Use existing variant
                            s if s.contains("freeze") => BlacklistReason::ApiError,
                            s if s.contains("authority") => BlacklistReason::ApiError,
                            _ => BlacklistReason::ApiError,
                        };
                        tokens_to_blacklist.push((mint, symbol, last_updated, blacklist_reason));
                        blacklist_count += 1;
                    }
                }
            }
        }

        // Process liquidity-based candidates - these get DELETED from main table
        for (mint, symbol, last_updated) in liquidity_tokens {
            if !self.has_open_position(&mint).await {
                // Skip tokens that are blacklisted (keep them in blacklist, don't delete)
                if !is_token_blacklisted_db(&mint) {
                    tokens_to_delete.push((mint, symbol, last_updated, "low_liquidity".to_string()));
                    delete_count += 1;
                }
            }
        }

        if tokens_to_blacklist.is_empty() && tokens_to_delete.is_empty() {
            log(
                LogTag::System,
                "CLEANUP",
                "No tokens eligible for cleanup (all have open positions or already processed)"
            );
            return Ok(0);
        }

        log(
            LogTag::System,
            "CLEANUP",
            &format!(
                "Selected for cleanup: {} tokens to blacklist (security), {} tokens to delete (low liquidity)",
                blacklist_count,
                delete_count
            )
        );

        // First, handle blacklisting (security issues) - keep tokens but mark as blacklisted
        let mut blacklisted_count = 0;
        for (mint, symbol, _last_updated, reason) in tokens_to_blacklist {
            if add_to_blacklist_db(&mint, &symbol, reason.clone()) {
                blacklisted_count += 1;
                if is_debug_monitor_enabled() {
                    log(
                        LogTag::System,
                        "BLACKLIST",
                        &format!("Blacklisted token: {} ({}) - reason: {:?}", symbol, &mint[..8], reason)
                    );
                }
            } else {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to blacklist token {}: {}", mint, "Database error")
                );
            }
        }

        // Then, handle deletion (low liquidity tokens) - remove from main table
        let (deleted_count, deleted_mints) = {
            let connection = self.connection
                .lock()
                .map_err(|_e| {
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Database lock error".to_string()
                        )
                    ) as Box<dyn std::error::Error>
                })?;

            let mut deleted_count = 0;
            let mut deleted_mints = Vec::new();
            for (mint, symbol, last_updated, reason) in &tokens_to_delete {
                match connection.execute("DELETE FROM tokens WHERE mint = ?1", params![mint]) {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            deleted_count += 1;
                            deleted_mints.push(mint.clone());

                            // Also delete rugcheck data for this token
                            if
                                let Err(e) = connection.execute(
                                    "DELETE FROM rugcheck_data WHERE mint = ?1",
                                    params![mint]
                                )
                            {
                                log(
                                    LogTag::System,
                                    "ERROR",
                                    &format!(
                                        "Failed to delete rugcheck data for token {}: {}",
                                        mint,
                                        e
                                    )
                                );
                            }

                            if is_debug_monitor_enabled() {
                                log(
                                    LogTag::System,
                                    "CLEANUP",
                                    &format!(
                                        "Deleted token: {} ({}) - reason: {} - last updated: {}",
                                        symbol,
                                        mint,
                                        reason,
                                        last_updated
                                    )
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to delete token {}: {}", mint, e)
                        );
                    }
                }
            }
            (deleted_count, deleted_mints)
        }; // connection lock released here

        // Clear deleted tokens from in-memory caches (outside database lock)
        for mint in &deleted_mints {
            self.clear_token_from_caches(mint);
        }

        let total_processed = blacklisted_count + deleted_count;
        if total_processed > 0 {
            log(
                LogTag::System,
                "CLEANUP",
                &format!(
                    "Database cleanup completed: {} tokens blacklisted (security), {} tokens deleted (low liquidity <${:.1})",
                    blacklisted_count,
                    deleted_count,
                    threshold_usd
                )
            );
        } else {
            log(LogTag::System, "CLEANUP", "Database cleanup: No problematic tokens processed");
        }

        Ok(total_processed)
    }

    /// Check if a token has an open position
    /// This prevents deletion of tokens that we currently hold
    async fn has_open_position(&self, mint: &str) -> bool {
        use crate::positions::is_open_position;

        // Use the async positions manager API to check for open positions
        is_open_position(mint).await
    }

    /// Clear a token from all in-memory caches
    /// Call this when a token is deleted from the database
    fn clear_token_from_caches(&self, mint: &str) {
        // Note: We cannot directly access private static caches from other modules
        // Instead, we'll add public cache clearing functions to those modules if needed
        // For now, we just log the cache clearing operation

        log(
            LogTag::System,
            "CACHE_CLEAR", 
            &format!("Token {} deleted - caches will be cleared on next access", &mint[..8])
        );

        // The caches will naturally clear on next access when the token is not found in database:
        // - DECIMAL_CACHE: get_cached_decimals will not find the token and return None
        // - TOKEN_BLACKLIST_CACHE: refresh_blacklist_cache will reload from database  
        // - TOKEN_CACHE: DexScreener cache has TTL and will refresh
        // - PRICE_CACHE: Has TTL and will naturally expire
    }

    /// Check if a token has security issues that warrant removal
    /// Returns Some(reason) if token should be removed, None otherwise
    async fn check_security_issues(&self, mint: &str) -> Option<String> {
        // Get rugcheck data for this token
        match self.get_rugcheck_data_instance(mint) {
            Ok(Some(rugcheck_data)) => {
                // Check for active mint authority (dangerous - can mint unlimited tokens)
                // First check the JSON field, then the basic text field
                if let Some(mint_authority) = &rugcheck_data.mint_authority {
                    match mint_authority {
                        serde_json::Value::String(s) if !s.is_empty() => {
                            return Some("mint_authority_enabled".to_string());
                        }
                        serde_json::Value::Null => {
                            // Null is good - mint authority disabled
                        }
                        _ => {
                            // Any other value indicates active mint authority
                            return Some("mint_authority_enabled".to_string());
                        }
                    }
                }

                // Also check the basic token mint authority field
                if let Some(token) = &rugcheck_data.token {
                    if let Some(mint_auth) = &token.mint_authority {
                        if !mint_auth.is_empty() {
                            return Some("token_mint_authority_enabled".to_string());
                        }
                    }

                    // Check token freeze authority
                    if let Some(freeze_auth) = &token.freeze_authority {
                        if !freeze_auth.is_empty() {
                            return Some("token_freeze_authority_enabled".to_string());
                        }
                    }
                }

                // Check for active freeze authority (dangerous - can freeze user accounts)
                if let Some(freeze_authority) = &rugcheck_data.freeze_authority {
                    match freeze_authority {
                        serde_json::Value::String(s) if !s.is_empty() => {
                            return Some("freeze_authority_enabled".to_string());
                        }
                        serde_json::Value::Null => {
                            // Null is good - freeze authority disabled
                        }
                        _ => {
                            // Any other value indicates active freeze authority
                            return Some("freeze_authority_enabled".to_string());
                        }
                    }
                }

                // Check for specific risks in rugcheck data
                if let Some(risks) = &rugcheck_data.risks {
                    for risk in risks {
                        let risk_name = risk.name.to_lowercase();
                        // Look for critical security risks
                        if risk_name.contains("mint") && risk_name.contains("authority") {
                            return Some(format!("security_risk_{}", risk_name.replace(' ', "_")));
                        }
                        if risk_name.contains("freeze") && risk_name.contains("authority") {
                            return Some(format!("security_risk_{}", risk_name.replace(' ', "_")));
                        }
                        if
                            risk_name.contains("ownership") &&
                            risk.level.as_ref().map_or(false, |l| l == "high")
                        {
                            return Some(format!("security_risk_{}", risk_name.replace(' ', "_")));
                        }
                    }
                }

                None // No security issues found
            }
            _ => None, // No rugcheck data or error - don't delete based on security
        }
    }

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
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_tokens: usize,
    pub tokens_with_liquidity: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl TokenDatabase {
    /// Initialize rugcheck table in the database
    pub fn initialize_rugcheck_table(&self) -> Result<(), rusqlite::Error> {
        let connection = self.connection
            .lock()
            .map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    Some("Failed to acquire database lock".to_string())
                )
            })?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS rugcheck_data (
                mint TEXT PRIMARY KEY,
                token_program TEXT,
                creator TEXT,
                creator_balance TEXT, -- Changed from INTEGER to TEXT to handle large numbers
                
                -- Token Info
                token_mint_authority TEXT,
                token_supply TEXT, -- Changed from INTEGER to TEXT to handle large numbers
                token_decimals INTEGER,
                token_is_initialized BOOLEAN,
                token_freeze_authority TEXT,
                
                -- Token Meta
                token_meta_name TEXT,
                token_meta_symbol TEXT,
                token_meta_uri TEXT,
                token_meta_mutable BOOLEAN,
                token_meta_update_authority TEXT,
                
                -- Risk Analysis
                score INTEGER,
                score_normalised INTEGER,
                rugged BOOLEAN,
                token_type TEXT,
                
                -- File Meta
                file_meta_description TEXT,
                file_meta_name TEXT,
                file_meta_symbol TEXT,
                file_meta_image TEXT,
                
                -- Market Data
                total_market_liquidity REAL,
                total_stable_liquidity REAL,
                total_lp_providers INTEGER,
                total_holders INTEGER,
                price REAL,
                
                -- Transfer Fee
                transfer_fee_pct REAL,
                transfer_fee_max_amount TEXT, -- Changed from INTEGER to TEXT to handle large numbers
                transfer_fee_authority TEXT,
                
                -- Analysis Info
                graph_insiders_detected INTEGER,
                detected_at TEXT,
                
                -- JSON Fields (for complex nested data)
                token_extensions TEXT,
                top_holders_json TEXT,
                freeze_authority_json TEXT,
                mint_authority_json TEXT,
                risks_json TEXT,
                locker_owners_json TEXT,
                lockers_json TEXT,
                markets_json TEXT,
                known_accounts_json TEXT,
                events_json TEXT,
                verification_json TEXT,
                insider_networks_json TEXT,
                creator_tokens_json TEXT,
                launchpad_json TEXT,
                
                -- Metadata
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            []
        )?;

        // Create indexes for better performance
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_rugcheck_score ON rugcheck_data(score DESC)",
            []
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_rugcheck_rugged ON rugcheck_data(rugged)",
            []
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_rugcheck_updated ON rugcheck_data(updated_at)",
            []
        )?;

        Ok(())
    }

    /// Store rugcheck data in the database
    pub fn store_rugcheck_data(
        &self,
        data: &crate::tokens::rugcheck::RugcheckResponse
    ) -> Result<(), rusqlite::Error> {
        let connection = self.connection
            .lock()
            .map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    Some("Failed to acquire database lock".to_string())
                )
            })?;

        // Serialize complex fields to JSON
        let token_extensions_json = data.token_extensions
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let top_holders_json = data.top_holders
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let freeze_authority_json = data.freeze_authority
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let mint_authority_json = data.mint_authority
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let risks_json = data.risks
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let locker_owners_json = data.locker_owners
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let lockers_json = data.lockers
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let markets_json = data.markets
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let known_accounts_json = data.known_accounts
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let events_json = data.events
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let verification_json = data.verification
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let insider_networks_json = data.insider_networks
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let creator_tokens_json = data.creator_tokens
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        let launchpad_json = data.launchpad
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();

        connection.execute(
            "INSERT OR REPLACE INTO rugcheck_data (
                mint, token_program, creator, creator_balance,
                token_mint_authority, token_supply, token_decimals, token_is_initialized, token_freeze_authority,
                token_meta_name, token_meta_symbol, token_meta_uri, token_meta_mutable, token_meta_update_authority,
                score, score_normalised, rugged, token_type,
                file_meta_description, file_meta_name, file_meta_symbol, file_meta_image,
                total_market_liquidity, total_stable_liquidity, total_lp_providers, total_holders, price,
                transfer_fee_pct, transfer_fee_max_amount, transfer_fee_authority,
                graph_insiders_detected, detected_at,
                token_extensions, top_holders_json, freeze_authority_json, mint_authority_json,
                risks_json, locker_owners_json, lockers_json, markets_json, known_accounts_json,
                events_json, verification_json, insider_networks_json, creator_tokens_json, launchpad_json,
                updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
                ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34,
                ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, datetime('now')
            )",
            params![
                data.mint,
                data.token_program,
                data.creator,
                data.creator_balance,

                // Token info
                data.token.as_ref().and_then(|t| t.mint_authority.as_ref()),
                data.token.as_ref().and_then(|t| t.supply.as_ref()),
                data.token.as_ref().and_then(|t| t.decimals),
                data.token.as_ref().and_then(|t| t.is_initialized),
                data.token.as_ref().and_then(|t| t.freeze_authority.as_ref()),

                // Token meta
                data.token_meta.as_ref().and_then(|m| m.name.as_ref()),
                data.token_meta.as_ref().and_then(|m| m.symbol.as_ref()),
                data.token_meta.as_ref().and_then(|m| m.uri.as_ref()),
                data.token_meta.as_ref().and_then(|m| m.mutable),
                data.token_meta.as_ref().and_then(|m| m.update_authority.as_ref()),

                // Risk analysis
                data.score,
                data.score_normalised,
                data.rugged,
                data.token_type,

                // File meta
                data.file_meta.as_ref().and_then(|f| f.description.as_ref()),
                data.file_meta.as_ref().and_then(|f| f.name.as_ref()),
                data.file_meta.as_ref().and_then(|f| f.symbol.as_ref()),
                data.file_meta.as_ref().and_then(|f| f.image.as_ref()),

                // Market data
                data.total_market_liquidity,
                data.total_stable_liquidity,
                data.total_lp_providers,
                data.total_holders,
                data.price,

                // Transfer fee
                data.transfer_fee.as_ref().and_then(|f| f.pct),
                data.transfer_fee.as_ref().and_then(|f| f.max_amount.as_ref()),
                data.transfer_fee.as_ref().and_then(|f| f.authority.as_ref()),

                // Analysis info
                data.graph_insiders_detected,
                data.detected_at,

                // JSON fields
                token_extensions_json,
                top_holders_json,
                freeze_authority_json,
                mint_authority_json,
                risks_json,
                locker_owners_json,
                lockers_json,
                markets_json,
                known_accounts_json,
                events_json,
                verification_json,
                insider_networks_json,
                creator_tokens_json,
                launchpad_json
            ]
        )?;

        Ok(())
    }

    /// Get rugcheck data for a specific token with timestamp
    pub fn get_rugcheck_data_with_timestamp(
        &self,
        mint: &str
    ) -> Result<
        Option<(crate::tokens::rugcheck::RugcheckResponse, chrono::DateTime<chrono::Utc>)>,
        rusqlite::Error
    > {
        let connection = self.connection
            .lock()
            .map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    Some("Failed to acquire database lock".to_string())
                )
            })?;

        let mut stmt = connection.prepare(
            "SELECT *, updated_at FROM rugcheck_data WHERE mint = ?1"
        )?;

        let mut rows = stmt.query_map(params![mint], |row| {
            let rugcheck_response = self.row_to_rugcheck_response(row)?;
            let updated_at_str: String = row.get("updated_at")?;

            // Parse SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
            let updated_at = chrono::NaiveDateTime
                ::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "updated_at".to_string(),
                        rusqlite::types::Type::Text
                    )
                })?
                .and_utc(); // Convert to UTC DateTime

            Ok((rugcheck_response, updated_at))
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Get rugcheck data for a specific token (static method)
    pub fn get_rugcheck_data(
        mint: &str
    ) -> Result<Option<crate::tokens::rugcheck::RugcheckResponse>, String> {
        use std::sync::Arc;

        let connection = Arc::new(
            std::sync::Mutex::new(
                Connection::open(&*TOKENS_DATABASE).map_err(|e|
                    format!("Database connection failed: {}", e)
                )?
            )
        );

        let conn = connection.lock().map_err(|_| "Failed to acquire database lock".to_string())?;

        let mut stmt = conn
            .prepare("SELECT * FROM rugcheck_data WHERE mint = ?1")
            .map_err(|e| format!("Failed to prepare rugcheck query: {}", e))?;

        let mut rows = stmt
            .query_map(params![mint], |row| { Self::row_to_rugcheck_response_static(row) })
            .map_err(|e| format!("Failed to execute rugcheck query: {}", e))?;

        if let Some(row) = rows.next() {
            Ok(Some(row.map_err(|e| format!("Failed to parse rugcheck row: {}", e))?))
        } else {
            Ok(None)
        }
    }

    /// Get rugcheck data for a specific token (instance method)
    pub fn get_rugcheck_data_instance(
        &self,
        mint: &str
    ) -> Result<Option<crate::tokens::rugcheck::RugcheckResponse>, rusqlite::Error> {
        let connection = self.connection
            .lock()
            .map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    Some("Failed to acquire database lock".to_string())
                )
            })?;

        let mut stmt = connection.prepare("SELECT * FROM rugcheck_data WHERE mint = ?1")?;

        let mut rows = stmt.query_map(params![mint], |row|
            Ok(self.row_to_rugcheck_response(row)?)
        )?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Convert database row to RugcheckResponse
    /// Static version of row_to_rugcheck_response for static methods
    fn row_to_rugcheck_response_static(
        row: &rusqlite::Row
    ) -> Result<crate::tokens::rugcheck::RugcheckResponse, rusqlite::Error> {
        use crate::tokens::rugcheck::*;

        // Parse JSON fields
        let token_extensions: Option<serde_json::Value> = row
            .get::<_, Option<String>>("token_extensions")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let top_holders: Option<Vec<Holder>> = row
            .get::<_, Option<String>>("top_holders_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let freeze_authority: Option<serde_json::Value> = row
            .get::<_, Option<String>>("freeze_authority_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let mint_authority: Option<serde_json::Value> = row
            .get::<_, Option<String>>("mint_authority_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let risks: Option<Vec<Risk>> = row.get::<_, Option<String>>("risks_json")?.and_then(|s| {
            if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
        });

        let markets: Option<Vec<Market>> = row
            .get::<_, Option<String>>("markets_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let known_accounts: Option<Vec<KnownAccount>> = row
            .get::<_, Option<String>>("known_accounts_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let events: Option<Vec<Event>> = row.get::<_, Option<String>>("events_json")?.and_then(|s| {
            if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
        });

        let verification: Option<Verification> = row
            .get::<_, Option<String>>("verification_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let insider_networks: Option<serde_json::Value> = row
            .get::<_, Option<String>>("insider_networks_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let creator_tokens: Option<serde_json::Value> = row
            .get::<_, Option<String>>("creator_tokens_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        let launchpad: Option<serde_json::Value> = row
            .get::<_, Option<String>>("launchpad_json")?
            .and_then(|s| {
                if s.is_empty() { None } else { serde_json::from_str(&s).ok() }
            });

        Ok(RugcheckResponse {
            mint: row.get("mint")?,
            token_program: row.get("token_program")?,
            creator: row.get("creator")?,
            creator_balance: row.get("creator_balance")?,
            token: Some(TokenInfo {
                mint_authority: row.get("token_mint_authority")?,
                supply: row.get("token_supply")?,
                decimals: row.get("token_decimals")?,
                is_initialized: row.get("token_is_initialized")?,
                freeze_authority: row.get("token_freeze_authority")?,
            }),
            token_extensions,
            token_meta: Some(TokenMeta {
                name: row.get("token_meta_name")?,
                symbol: row.get("token_meta_symbol")?,
                uri: row.get("token_meta_uri")?,
                mutable: row.get("token_meta_mutable")?,
                update_authority: row.get("token_meta_update_authority")?,
            }),
            top_holders,
            freeze_authority,
            mint_authority,
            risks,
            score: row.get("score")?,
            score_normalised: row.get("score_normalised")?,
            file_meta: Some(FileMeta {
                description: row.get("file_meta_description")?,
                name: row.get("file_meta_name")?,
                symbol: row.get("file_meta_symbol")?,
                image: row.get("file_meta_image")?,
            }),
            locker_owners: None, // Not stored in database yet
            lockers: None, // Not stored in database yet
            markets,
            total_market_liquidity: row.get("total_market_liquidity")?,
            total_stable_liquidity: row.get("total_stable_liquidity")?,
            total_lp_providers: row.get("total_lp_providers")?,
            total_holders: row.get("total_holders")?,
            price: row.get("price")?,
            rugged: row.get("rugged")?,
            token_type: row.get("token_type")?,
            transfer_fee: Some(TransferFee {
                pct: row.get("transfer_fee_pct")?,
                max_amount: row.get("transfer_fee_max_amount")?,
                authority: row.get("transfer_fee_authority")?,
            }),
            known_accounts: None, // Will be populated from JSON
            events,
            verification,
            graph_insiders_detected: row.get("graph_insiders_detected")?,
            insider_networks,
            detected_at: row.get("detected_at")?,
            creator_tokens,
            launchpad,
        })
    }

    fn row_to_rugcheck_response(
        &self,
        row: &rusqlite::Row
    ) -> Result<crate::tokens::rugcheck::RugcheckResponse, rusqlite::Error> {
        Self::row_to_rugcheck_response_static(row)
    }

    /// Delete rugcheck data for a specific token
    pub fn delete_rugcheck_data(&self, mint: &str) -> Result<bool, rusqlite::Error> {
        let connection = self.connection
            .lock()
            .map_err(|_| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                    Some("Failed to acquire database lock".to_string())
                )
            })?;

        let rows_affected = connection.execute(
            "DELETE FROM rugcheck_data WHERE mint = ?1",
            params![mint]
        )?;
        Ok(rows_affected > 0)
    }

    /// Populate tokens with rugcheck_data and decimals from database
    pub async fn populate_tokens_with_cached_data(
        &self,
        tokens: &mut [crate::tokens::types::Token]
    ) -> Result<(), String> {
        use crate::tokens::get_token_decimals;

        for token in tokens.iter_mut() {
            // Populate rugcheck_data from database
            if token.rugcheck_data.is_none() {
                match self.get_rugcheck_data_instance(&token.mint) {
                    Ok(Some(rugcheck_data)) => {
                        token.rugcheck_data = Some(rugcheck_data);
                    }
                    Ok(None) => {
                        // No rugcheck data found - leave as None
                    }
                    Err(e) => {
                        // Log error but continue with other tokens
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to get rugcheck data for {}: {}", token.mint, e)
                        );
                    }
                }
            }

            // Populate decimals from cache and blockchain if needed
            if token.decimals.is_none() {
                if let Some(decimals) = get_token_decimals(&token.mint).await {
                    token.decimals = Some(decimals);
                }
            }
        }

        Ok(())
    }


}
