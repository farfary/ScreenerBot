// transactions/cache.rs - SQLite database caching implementation
use super::types::*;
use rusqlite::{ Connection, OptionalExtension, params };
use std::path::Path;
use std::error::Error;
use chrono::{ DateTime, Utc };
use serde_json;
use crate::logger::{ log, LogTag };

/// SQLite database path for transaction cache
const DB_PATH: &str = "transactions.db";

/// Transaction database cache with SQLite backend
pub struct TransactionDatabase {
    conn: Connection,
}

impl TransactionDatabase {
    /// Create a new database connection and initialize tables
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let conn = Connection::open(DB_PATH)?;

        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                signature TEXT PRIMARY KEY,
                slot INTEGER NOT NULL,
                block_time INTEGER,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL,
                UNIQUE(signature)
            )",
            []
        )?;

        // Create indexes for better performance
        conn.execute("CREATE INDEX IF NOT EXISTS idx_slot ON transactions(slot)", [])?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_block_time ON transactions(block_time)", [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_last_accessed ON transactions(last_accessed)",
            []
        )?;

        // Create sync status table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_status (
                wallet_address TEXT PRIMARY KEY,
                last_sync_slot INTEGER NOT NULL,
                last_sync_time TEXT NOT NULL,
                total_transactions INTEGER DEFAULT 0,
                pending_transactions INTEGER DEFAULT 0
            )",
            []
        )?;

        log(LogTag::System, "SUCCESS", &format!("Initialized transaction database at {}", DB_PATH));

        Ok(Self { conn })
    }

    /// Get transaction by signature
    pub fn get_transaction(
        &self,
        signature: &str
    ) -> Result<Option<TransactionResult>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT data, last_accessed FROM transactions WHERE signature = ?1"
        )?;

        let result: Result<Option<TransactionResult>, _> = stmt
            .query_row(params![signature], |row| {
                let data: String = row.get(0)?;
                let transaction: TransactionResult = serde_json
                    ::from_str(&data)
                    .map_err(|e|
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "JSON parse error".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?;

                // Update last accessed time asynchronously
                let now = Utc::now().to_rfc3339();
                let _ = self.conn.execute(
                    "UPDATE transactions SET last_accessed = ?1 WHERE signature = ?2",
                    params![now, signature]
                );

                Ok(transaction)
            })
            .optional();

        match result {
            Ok(Some(transaction)) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Retrieved transaction {} from database", signature)
                );
                Ok(Some(transaction))
            }
            Ok(None) => Ok(None),
            Err(e) =>
                Err(format!("Database error retrieving transaction {}: {}", signature, e).into()),
        }
    }

    /// Add or update transaction in database
    pub fn upsert_transaction(
        &self,
        signature: &str,
        transaction: &TransactionResult
    ) -> Result<(), Box<dyn Error>> {
        let data = serde_json::to_string(transaction)?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO transactions 
             (signature, slot, block_time, data, created_at, last_accessed) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                signature,
                transaction.slot as i64,
                transaction.block_time.map(|t| t as i64),
                data,
                now,
                now
            ]
        )?;

        Ok(())
    }

    /// Batch insert transactions for better performance
    pub fn batch_upsert_transactions(
        &self,
        transactions: &[(String, TransactionResult)]
    ) -> Result<(), Box<dyn Error>> {
        let tx = self.conn.unchecked_transaction()?;
        let now = Utc::now().to_rfc3339();

        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO transactions 
                 (signature, slot, block_time, data, created_at, last_accessed) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
            )?;

            for (signature, transaction) in transactions {
                let data = serde_json::to_string(transaction)?;
                stmt.execute(
                    params![
                        signature,
                        transaction.slot as i64,
                        transaction.block_time.map(|t| t as i64),
                        data,
                        now,
                        now
                    ]
                )?;
            }
        }

        tx.commit()?;

        log(
            LogTag::System,
            "SUCCESS",
            &format!("Batch inserted {} transactions to database", transactions.len())
        );
        Ok(())
    }

    /// Check if transaction exists in database
    pub fn contains(&self, signature: &str) -> Result<bool, Box<dyn Error>> {
        let mut stmt = self.conn.prepare("SELECT 1 FROM transactions WHERE signature = ?1")?;
        let exists = stmt.exists(params![signature])?;
        Ok(exists)
    }

    /// Get transactions by slot range
    pub fn get_transactions_by_slot_range(
        &self,
        min_slot: u64,
        max_slot: u64
    ) -> Result<Vec<TransactionRecord>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT signature, slot, block_time, data, created_at, last_accessed 
             FROM transactions 
             WHERE slot BETWEEN ?1 AND ?2 
             ORDER BY slot DESC"
        )?;

        let rows = stmt.query_map(params![min_slot as i64, max_slot as i64], |row| {
            Ok(TransactionRecord {
                signature: row.get(0)?,
                slot: row.get::<_, i64>(1)? as u64,
                block_time: row.get::<_, Option<i64>>(2)?.map(|t| t as u64),
                data: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get recent transactions with limit
    pub fn get_recent_transactions(
        &self,
        limit: usize
    ) -> Result<Vec<TransactionRecord>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT signature, slot, block_time, data, created_at, last_accessed 
             FROM transactions 
             ORDER BY slot DESC 
             LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok(TransactionRecord {
                signature: row.get(0)?,
                slot: row.get::<_, i64>(1)? as u64,
                block_time: row.get::<_, Option<i64>>(2)?.map(|t| t as u64),
                data: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<(usize, u64, u64), Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*), MIN(slot), MAX(slot) FROM transactions"
        )?;

        let (count, min_slot, max_slot): (usize, Option<i64>, Option<i64>) = stmt.query_row(
            [],
            |row| { Ok((row.get::<_, i64>(0)? as usize, row.get(1)?, row.get(2)?)) }
        )?;

        Ok((count, min_slot.unwrap_or(0) as u64, max_slot.unwrap_or(0) as u64))
    }

    /// Clean up old transactions to maintain database size
    pub fn cleanup_old_transactions(&self, keep_count: usize) -> Result<usize, Box<dyn Error>> {
        // Get total count
        let (total_count, _, _) = self.get_stats()?;

        if total_count <= keep_count {
            return Ok(0);
        }

        // Delete oldest transactions, keeping the specified count
        let deleted = self.conn.execute(
            "DELETE FROM transactions 
             WHERE signature NOT IN (
                 SELECT signature FROM transactions 
                 ORDER BY slot DESC 
                 LIMIT ?1
             )",
            params![keep_count]
        )?;

        if deleted > 0 {
            log(
                LogTag::System,
                "INFO",
                &format!("Cleaned up {} old transactions from database", deleted)
            );
        }

        Ok(deleted)
    }

    /// Get sync status for a wallet
    pub fn get_sync_status(
        &self,
        wallet_address: &str
    ) -> Result<Option<SyncStatus>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT last_sync_slot, last_sync_time, total_transactions, pending_transactions 
             FROM sync_status WHERE wallet_address = ?1"
        )?;

        let result = stmt
            .query_row(params![wallet_address], |row| {
                Ok(SyncStatus {
                    last_sync_slot: row.get::<_, i64>(0)? as u64,
                    last_sync_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                    total_transactions: row.get::<_, i64>(2)? as u64,
                    pending_transactions: row.get::<_, i64>(3)? as u64,
                })
            })
            .optional()?;

        Ok(result)
    }

    /// Update sync status for a wallet
    pub fn update_sync_status(
        &self,
        wallet_address: &str,
        status: &SyncStatus
    ) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sync_status 
             (wallet_address, last_sync_slot, last_sync_time, total_transactions, pending_transactions) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                wallet_address,
                status.last_sync_slot as i64,
                status.last_sync_time.to_rfc3339(),
                status.total_transactions as i64,
                status.pending_transactions as i64
            ]
        )?;

        Ok(())
    }

    /// Get signatures that need to be fetched (not in database)
    pub fn get_missing_signatures(
        &self,
        signatures: &[String]
    ) -> Result<Vec<String>, Box<dyn Error>> {
        if signatures.is_empty() {
            return Ok(Vec::new());
        }

        // Create placeholders for IN clause
        let placeholders = signatures
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query =
            format!("SELECT signature FROM transactions WHERE signature IN ({})", placeholders);

        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> = signatures
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let existing_signatures: std::collections::HashSet<String> = stmt
            .query_map(params.as_slice(), |row| Ok(row.get::<_, String>(0)?))?
            .collect::<Result<_, _>>()?;

        let missing: Vec<String> = signatures
            .iter()
            .filter(|sig| !existing_signatures.contains(*sig))
            .cloned()
            .collect();

        Ok(missing)
    }

    /// Optimize database performance
    pub fn optimize(&self) -> Result<(), Box<dyn Error>> {
        // Analyze tables for better query planning
        self.conn.execute("ANALYZE", [])?;

        // Vacuum to defragment and reclaim space
        self.conn.execute("VACUUM", [])?;

        log(LogTag::System, "SUCCESS", "Database optimization completed");
        Ok(())
    }

    /// Get transaction count for statistics
    pub fn get_transaction_count(&self) -> Result<usize, Box<dyn Error>> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM transactions")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get all transactions as raw records (for migration)
    pub fn get_all_transactions_raw(&self) -> Result<Vec<TransactionRecord>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT signature, slot, block_time, data, created_at, last_accessed 
             FROM transactions ORDER BY slot DESC"
        )?;

        let records = stmt.query_map([], |row| {
            Ok(TransactionRecord {
                signature: row.get(0)?,
                slot: row.get::<_, i64>(1)? as u64,
                block_time: row.get::<_, Option<i64>>(2)?.map(|t| t as u64),
                data: row.get(3)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            4,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            5,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc),
            })
        })?;

        let mut result = Vec::new();
        for record in records {
            result.push(record?);
        }
        Ok(result)
    }

    /// Store a transaction in the database
    pub fn store_transaction(&self, transaction: &TransactionResult) -> Result<(), Box<dyn Error>> {
        let signature = transaction.transaction.signatures
            .get(0)
            .ok_or("Transaction has no signatures")?;
        let slot = transaction.slot as i64;
        let block_time = transaction.block_time.map(|t| t as i64);
        let data = serde_json::to_string(transaction)?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO transactions 
             (signature, slot, block_time, data, created_at, last_accessed) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![signature, slot, block_time, data, now, now]
        )?;

        log(LogTag::System, "DEBUG", &format!("Stored transaction in database: {}", signature));
        Ok(())
    }

    /// Clean up old transactions beyond the retention period
    pub fn cleanup_transactions_by_timestamp(
        &self,
        cutoff_timestamp: i64
    ) -> Result<usize, Box<dyn Error>> {
        let rows_affected = self.conn.execute(
            "DELETE FROM transactions WHERE block_time IS NOT NULL AND block_time < ?1",
            params![cutoff_timestamp]
        )?;

        if rows_affected > 0 {
            log(LogTag::System, "INFO", &format!("Cleaned up {} old transactions", rows_affected));
        }

        Ok(rows_affected)
    }

    /// Get all signature strings from database (for populating in-memory cache)
    pub fn get_all_signatures(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut stmt = self.conn.prepare("SELECT signature FROM transactions")?;
        let signatures = stmt
            .query_map([], |row| { Ok(row.get::<_, String>(0)?) })?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(signatures)
    }
}

/// Legacy JSON cache compatibility layer
pub struct TransactionCache {
    db: TransactionDatabase,
}

impl TransactionCache {
    /// Load cache (creates new database connection)
    pub fn load() -> Self {
        match TransactionDatabase::new() {
            Ok(db) => {
                log(LogTag::System, "SUCCESS", "Loaded transaction database cache");
                Self { db }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed to initialize database: {}", e));
                panic!("Cannot initialize transaction database: {}", e);
            }
        }
    }

    /// Get transaction from cache
    pub fn get_transaction(&self, signature: &str) -> Option<TransactionResult> {
        match self.db.get_transaction(signature) {
            Ok(transaction) => transaction,
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Error retrieving transaction {}: {}", signature, e)
                );
                None
            }
        }
    }

    /// Add transaction to cache
    pub fn add_transaction(&mut self, signature: String, transaction: TransactionResult) {
        if let Err(e) = self.db.upsert_transaction(&signature, &transaction) {
            log(
                LogTag::System,
                "WARNING",
                &format!("Error caching transaction {}: {}", signature, e)
            );
        }
    }

    /// Check if transaction exists in cache
    pub fn contains(&self, signature: &str) -> bool {
        self.db.contains(signature).unwrap_or(false)
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, u64) {
        match self.db.get_stats() {
            Ok((count, _, max_slot)) => (count, max_slot),
            Err(_) => (0, 0),
        }
    }

    /// Save cache (no-op for database backend)
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        // No-op for database backend - data is already persisted
        Ok(())
    }

    /// Get all transactions (for compatibility)
    pub fn get_all_transactions(&self) -> std::collections::HashMap<String, TransactionResult> {
        let mut map = std::collections::HashMap::new();

        if let Ok(records) = self.db.get_recent_transactions(10000) {
            // Limit to avoid memory issues
            for record in records {
                if let Ok(transaction) = serde_json::from_str::<TransactionResult>(&record.data) {
                    map.insert(record.signature, transaction);
                }
            }
        }

        map
    }
}

// Implement backward compatibility
impl TransactionCache {
    /// Legacy property access for backward compatibility
    pub fn transactions(&self) -> std::collections::HashMap<String, TransactionResult> {
        self.get_all_transactions()
    }
}
