// Database utilities and helpers for the cache system

use crate::core::{ BotResult, BotError };
use rusqlite::{ Connection, params };
use chrono::{ DateTime, Utc };

/// Database utilities for cache operations
pub struct DatabaseUtils;

impl DatabaseUtils {
    /// Get database statistics
    pub fn get_stats(db: &Connection) -> BotResult<DatabaseStats> {
        let transactions_count: i64 = db
            .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
            .unwrap_or(0);

        let market_data_count: i64 = db
            .query_row("SELECT COUNT(*) FROM market_data", [], |row| row.get(0))
            .unwrap_or(0);

        let trade_results_count: i64 = db
            .query_row("SELECT COUNT(*) FROM trade_results", [], |row| row.get(0))
            .unwrap_or(0);

        let wallet_balances_count: i64 = db
            .query_row("SELECT COUNT(*) FROM wallet_balances", [], |row| row.get(0))
            .unwrap_or(0);

        Ok(DatabaseStats {
            transactions_count: transactions_count as u64,
            market_data_count: market_data_count as u64,
            trade_results_count: trade_results_count as u64,
            wallet_balances_count: wallet_balances_count as u64,
        })
    }

    /// Vacuum database to reclaim space
    pub fn vacuum(db: &Connection) -> BotResult<()> {
        db.execute("VACUUM", []).map_err(|e| BotError::Database(e))?;
        Ok(())
    }

    /// Analyze database for query optimization
    pub fn analyze(db: &Connection) -> BotResult<()> {
        db.execute("ANALYZE", []).map_err(|e| BotError::Database(e))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub transactions_count: u64,
    pub market_data_count: u64,
    pub trade_results_count: u64,
    pub wallet_balances_count: u64,
}
