use crate::core::{
    BotResult,
    BotError,
    CacheConfig,
    CacheEntry,
    TradeResult,
    TokenBalance,
    WalletTransaction,
    MarketData,
};
use rusqlite::{ Connection, params, Row };
use serde::{ Serialize, de::DeserializeOwned };
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;

pub mod database;
pub mod storage;

// Re-export public interfaces

/// Cache manager for storing and retrieving data
#[derive(Debug, Clone)]
pub struct CacheManager {
    db: Arc<Mutex<Connection>>,
    config: CacheConfig,
    memory_cache: Arc<Mutex<HashMap<String, CacheEntry<String>>>>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(bot_config: &crate::core::BotConfig) -> BotResult<Self> {
        let config = &bot_config.cache_settings;
        let db = Connection::open(&config.database_path).map_err(|e|
            BotError::Cache(format!("Failed to open database: {}", e))
        )?;

        let cache = Self {
            db: Arc::new(Mutex::new(db)),
            config: config.clone(),
            memory_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        Ok(cache)
    }

    /// Initialize the cache system
    pub async fn initialize(&self) -> BotResult<()> {
        log::info!("🗄️ Initializing cache system...");

        // Create database tables
        self.create_tables().await?;

        // Clean up old entries
        self.cleanup_expired_entries().await?;

        log::info!("✅ Cache system initialized");
        Ok(())
    }

    /// Create database tables
    async fn create_tables(&self) -> BotResult<()> {
        let db = self.db.lock().unwrap();

        // Transactions table
        db
            .execute(
                "CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signature TEXT UNIQUE NOT NULL,
                wallet_address TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                tokens_involved TEXT NOT NULL,
                sol_change INTEGER NOT NULL,
                token_changes TEXT NOT NULL,
                fees INTEGER NOT NULL,
                status TEXT NOT NULL,
                block_time INTEGER,
                slot INTEGER NOT NULL,
                parsed_data TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
                []
            )
            .map_err(|e| BotError::Database(e))?;

        // Token metadata table
        db
            .execute(
                "CREATE TABLE IF NOT EXISTS token_metadata (
                mint TEXT PRIMARY KEY,
                symbol TEXT,
                name TEXT,
                decimals INTEGER NOT NULL,
                logo_uri TEXT,
                verified BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
                []
            )
            .map_err(|e| BotError::Database(e))?;

        // Market data table
        db
            .execute(
                "CREATE TABLE IF NOT EXISTS market_data (
                mint TEXT PRIMARY KEY,
                price_usd REAL NOT NULL,
                volume_24h REAL NOT NULL,
                liquidity REAL NOT NULL,
                market_cap REAL,
                price_change_24h REAL,
                data_source TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
                []
            )
            .map_err(|e| BotError::Database(e))?;

        // Trade results table
        db
            .execute(
                "CREATE TABLE IF NOT EXISTS trade_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                transaction_id TEXT UNIQUE NOT NULL,
                trade_type TEXT NOT NULL,
                token_mint TEXT NOT NULL,
                amount_sol REAL NOT NULL,
                amount_token INTEGER NOT NULL,
                price_per_token REAL NOT NULL,
                slippage_actual REAL NOT NULL,
                fees_paid INTEGER NOT NULL,
                success BOOLEAN NOT NULL,
                error_message TEXT,
                executed_at DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
                []
            )
            .map_err(|e| BotError::Database(e))?;

        // Wallet balances table (for portfolio tracking)
        db
            .execute(
                "CREATE TABLE IF NOT EXISTS wallet_balances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_address TEXT NOT NULL,
                mint TEXT NOT NULL,
                amount INTEGER NOT NULL,
                ui_amount REAL NOT NULL,
                value_usd REAL,
                snapshot_time DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(wallet_address, mint, snapshot_time)
            )",
                []
            )
            .map_err(|e| BotError::Database(e))?;

        // Create indexes for better performance
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_transactions_wallet ON transactions(wallet_address)",
            []
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_transactions_signature ON transactions(signature)",
            []
        )?;
        db.execute("CREATE INDEX IF NOT EXISTS idx_market_data_mint ON market_data(mint)", [])?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_trade_results_token ON trade_results(token_mint)",
            []
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_wallet_balances_wallet ON wallet_balances(wallet_address)",
            []
        )?;

        Ok(())
    }

    /// Store a trade result
    pub async fn store_trade_result(&self, result: &TradeResult) -> BotResult<()> {
        let db = self.db.lock().unwrap();

        db
            .execute(
                "INSERT OR REPLACE INTO trade_results 
            (transaction_id, trade_type, token_mint, amount_sol, amount_token, 
             price_per_token, slippage_actual, fees_paid, success, error_message, executed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    result.transaction_id,
                    serde_json::to_string(&result.trade_type)?,
                    result.token.to_string(),
                    result.amount_sol,
                    result.amount_token as i64,
                    result.price_per_token,
                    result.slippage_actual,
                    result.fees_paid as i64,
                    result.success,
                    result.error_message,
                    result.executed_at.format("%Y-%m-%d %H:%M:%S").to_string()
                ]
            )
            .map_err(|e| BotError::Database(e))?;

        Ok(())
    }

    /// Cache a transaction
    pub async fn cache_transaction(
        &self,
        wallet: &Pubkey,
        transaction: &WalletTransaction
    ) -> BotResult<()> {
        if !self.config.cache_transactions {
            return Ok(());
        }

        let db = self.db.lock().unwrap();

        db
            .execute(
                "INSERT OR REPLACE INTO transactions 
            (signature, wallet_address, transaction_type, tokens_involved, sol_change, 
             token_changes, fees, status, block_time, slot, parsed_data)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    transaction.signature,
                    wallet.to_string(),
                    serde_json::to_string(&transaction.transaction_type)?,
                    serde_json::to_string(&transaction.tokens_involved)?,
                    transaction.sol_change,
                    serde_json::to_string(&transaction.token_changes)?,
                    transaction.fees as i64,
                    serde_json::to_string(&transaction.status)?,
                    transaction.block_time,
                    transaction.slot as i64,
                    transaction.parsed_data
                        .as_ref()
                        .map(|d| serde_json::to_string(d).unwrap_or_default())
                ]
            )
            .map_err(|e| BotError::Database(e))?;

        Ok(())
    }

    /// Get cached transactions for a wallet
    pub async fn get_cached_transactions(
        &self,
        wallet: &Pubkey,
        limit: usize
    ) -> BotResult<Vec<WalletTransaction>> {
        let db = self.db.lock().unwrap();

        let mut stmt = db
            .prepare(
                "SELECT signature, transaction_type, tokens_involved, sol_change, token_changes, 
                    fees, status, block_time, slot, parsed_data, created_at
             FROM transactions 
             WHERE wallet_address = ?1 
             ORDER BY created_at DESC 
             LIMIT ?2"
            )
            .map_err(|e| BotError::Database(e))?;

        let transaction_iter = stmt
            .query_map(params![wallet.to_string(), limit], |row| {
                Ok((
                    row.get::<_, String>(0)?, // signature
                    row.get::<_, String>(1)?, // transaction_type
                    row.get::<_, String>(2)?, // tokens_involved
                    row.get::<_, f64>(3)?, // sol_change
                    row.get::<_, String>(4)?, // token_changes
                    row.get::<_, f64>(5)?, // fees
                    row.get::<_, String>(6)?, // status
                    row.get::<_, i64>(7)?, // block_time
                    row.get::<_, i64>(8)?, // slot
                    row.get::<_, String>(9)?, // parsed_data
                ))
            })
            .map_err(|e| BotError::Database(e))?;

        let mut transactions = Vec::new();
        for transaction_result in transaction_iter {
            match transaction_result {
                Ok(
                    (
                        signature,
                        transaction_type,
                        tokens_involved,
                        sol_change,
                        token_changes,
                        fees,
                        status,
                        block_time,
                        slot,
                        parsed_data,
                    ),
                ) => {
                    match
                        self.create_transaction_from_data(
                            signature,
                            block_time,
                            transaction_type,
                            tokens_involved,
                            sol_change,
                            token_changes,
                            fees,
                            status,
                            slot,
                            parsed_data
                        )
                    {
                        Ok(tx) => transactions.push(tx),
                        Err(e) => log::warn!("Failed to parse cached transaction: {}", e),
                    }
                }
                Err(e) => log::warn!("Database error reading transaction: {}", e),
            }
        }

        Ok(transactions)
    }

    /// Convert database row to WalletTransaction
    fn row_to_transaction(&self, row: &Row) -> BotResult<WalletTransaction> {
        Ok(WalletTransaction {
            signature: row.get(0)?,
            transaction_type: serde_json::from_str(&row.get::<_, String>(1)?)?,
            tokens_involved: serde_json::from_str(&row.get::<_, String>(2)?)?,
            sol_change: row.get(3)?,
            token_changes: serde_json::from_str(&row.get::<_, String>(4)?)?,
            fees: row.get::<_, i64>(5)? as u64,
            status: serde_json::from_str(&row.get::<_, String>(6)?)?,
            block_time: row.get(7)?,
            slot: row.get::<_, i64>(8)? as u64,
            parsed_data: {
                let data_str: Option<String> = row.get(9)?;
                match data_str {
                    Some(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
                    _ => None,
                }
            },
        })
    }

    /// Cache market data for a token
    pub async fn cache_market_data(&self, data: &MarketData) -> BotResult<()> {
        let db = self.db.lock().unwrap();

        db
            .execute(
                "INSERT OR REPLACE INTO market_data 
            (mint, price_usd, volume_24h, liquidity, market_cap, price_change_24h, data_source, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
                params![
                    data.mint.to_string(),
                    data.price_usd,
                    data.volume_24h,
                    data.liquidity,
                    data.market_cap,
                    data.price_change_24h,
                    data.data_source
                ]
            )
            .map_err(|e| BotError::Database(e))?;

        Ok(())
    }

    /// Get cached market data
    pub async fn get_market_data(&self, mint: &Pubkey) -> BotResult<Option<MarketData>> {
        let db = self.db.lock().unwrap();

        let mut stmt = db
            .prepare(
                "SELECT mint, price_usd, volume_24h, liquidity, market_cap, price_change_24h, 
                    data_source, updated_at
             FROM market_data 
             WHERE mint = ?1 AND updated_at > datetime('now', '-1 hour')"
            )
            .map_err(|e| BotError::Database(e))?;

        let result = stmt.query_row(params![mint.to_string()], |row| {
            Ok(MarketData {
                mint: row.get::<_, String>(0)?.parse().unwrap(),
                symbol: "".to_string(), // Will be filled from metadata
                price_usd: row.get(1)?,
                volume_24h: row.get(2)?,
                liquidity: row.get(3)?,
                market_cap: row.get(4)?,
                price_change_1h: None,
                price_change_24h: row.get(5)?,
                price_change_7d: None,
                all_time_high: None,
                all_time_low: None,
                last_updated: Utc::now(),
                data_source: row.get(6)?,
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BotError::Database(e)),
        }
    }

    /// Store wallet balance snapshot
    pub async fn store_balance_snapshot(
        &self,
        wallet: &Pubkey,
        balances: &[TokenBalance]
    ) -> BotResult<()> {
        let db = self.db.lock().unwrap();
        let snapshot_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        for balance in balances {
            db
                .execute(
                    "INSERT OR REPLACE INTO wallet_balances 
                (wallet_address, mint, amount, ui_amount, value_usd, snapshot_time)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        wallet.to_string(),
                        balance.mint.to_string(),
                        balance.amount as i64,
                        balance.ui_amount,
                        balance.value_usd,
                        snapshot_time
                    ]
                )
                .map_err(|e| BotError::Database(e))?;
        }

        Ok(())
    }

    /// Generic cache storage for JSON data
    pub async fn store_json<T: Serialize>(
        &self,
        key: &str,
        data: &T,
        ttl_hours: u64
    ) -> BotResult<()> {
        let json_data = serde_json::to_string(data)?;
        let entry = CacheEntry::new(json_data, ttl_hours);

        let mut memory_cache = self.memory_cache.lock().unwrap();
        memory_cache.insert(key.to_string(), entry);

        Ok(())
    }

    /// Generic cache retrieval for JSON data
    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> BotResult<Option<T>> {
        let mut memory_cache = self.memory_cache.lock().unwrap();

        if let Some(entry) = memory_cache.get_mut(key) {
            if !entry.is_expired() {
                let data = entry.access();
                return Ok(Some(serde_json::from_str(data)?));
            } else {
                memory_cache.remove(key);
            }
        }

        Ok(None)
    }

    /// Helper method to create transaction from raw data
    fn create_transaction_from_data(
        &self,
        signature: String,
        block_time: i64,
        transaction_type: String,
        tokens_involved: String,
        sol_change: f64,
        token_changes: String,
        fees: f64,
        status: String,
        slot: i64,
        parsed_data: String
    ) -> BotResult<WalletTransaction> {
        use crate::core::{ TransactionType, TransactionStatus };
        use std::str::FromStr;

        // Parse transaction type
        let transaction_type: TransactionType = serde_json
            ::from_str(&transaction_type)
            .unwrap_or(TransactionType::Unknown);

        // Parse tokens involved
        let tokens_involved: Vec<Pubkey> = serde_json
            ::from_str(&tokens_involved)
            .unwrap_or_default();

        // Parse token changes
        let token_changes: HashMap<Pubkey, i64> = serde_json
            ::from_str(&token_changes)
            .unwrap_or_default();

        // Parse status
        let status: TransactionStatus = serde_json
            ::from_str(&status)
            .unwrap_or(TransactionStatus::Success);

        // Parse optional data
        let parsed_data: Option<crate::core::ParsedTransactionData> = if parsed_data.is_empty() {
            None
        } else {
            serde_json::from_str(&parsed_data).ok()
        };

        Ok(WalletTransaction {
            signature,
            transaction_type,
            tokens_involved,
            sol_change: sol_change as i64,
            token_changes,
            fees: fees as u64,
            status,
            block_time: Some(block_time),
            slot: slot as u64,
            parsed_data,
        })
    }

    /// Clean up expired cache entries
    pub async fn cleanup_expired_entries(&self) -> BotResult<()> {
        let db = self.db.lock().unwrap();

        // Clean up old transactions (older than configured TTL)
        let ttl_hours = crate::core::constants::TRANSACTION_CACHE_TTL_HOURS;
        db.execute(
            "DELETE FROM transactions WHERE created_at < datetime('now', ?1)",
            params![format!("-{} hours", ttl_hours)]
        )?;

        // Clean up old market data
        let market_ttl_hours = crate::core::constants::DEFAULT_CACHE_TTL_HOURS;
        db.execute(
            "DELETE FROM market_data WHERE updated_at < datetime('now', ?1)",
            params![format!("-{} hours", market_ttl_hours)]
        )?;

        // Clean up old balance snapshots
        db.execute(
            "DELETE FROM wallet_balances WHERE created_at < datetime('now', '-7 days')",
            []
        )?;

        log::debug!("🧹 Cleaned up expired cache entries");
        Ok(())
    }

    /// Get transaction count for a wallet
    pub async fn get_transaction_count(&self, wallet: &Pubkey) -> BotResult<usize> {
        let db = self.db.lock().unwrap();

        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE wallet_address = ?1",
                params![wallet.to_string()],
                |row| row.get(0)
            )
            .unwrap_or(0);

        Ok(count as usize)
    }

    /// Check if cache is enabled for transactions
    pub fn is_transaction_cache_enabled(&self) -> bool {
        self.config.cache_transactions
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> BotResult<CacheStats> {
        let db = self.db.lock().unwrap();

        let transaction_count: i64 = db
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap_or(0);

        let market_data_count: i64 = db
            .query_row("SELECT COUNT(*) FROM market_data", [], |row| row.get(0))
            .unwrap_or(0);

        let trade_results_count: i64 = db
            .query_row("SELECT COUNT(*) FROM trade_results", [], |row| row.get(0))
            .unwrap_or(0);

        let memory_cache_size = {
            let cache = self.memory_cache.lock().unwrap();
            cache.len()
        };

        Ok(CacheStats {
            transaction_count: transaction_count as usize,
            market_data_count: market_data_count as usize,
            trade_results_count: trade_results_count as usize,
            memory_cache_size,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub transaction_count: usize,
    pub market_data_count: usize,
    pub trade_results_count: usize,
    pub memory_cache_size: usize,
}
