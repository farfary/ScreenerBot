use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ params, Connection };
use std::path::Path;
use std::sync::Mutex;

use crate::trader::types::*;

pub struct TraderDatabase {
    conn: Mutex<Connection>,
}

impl TraderDatabase {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open trader database")?;

        let db = Self { conn: Mutex::new(conn) };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Positions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS positions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_address TEXT NOT NULL UNIQUE,
                token_symbol TEXT NOT NULL,
                total_invested_sol REAL NOT NULL DEFAULT 0.0,
                original_entry_price REAL NOT NULL DEFAULT 0.0,
                average_buy_price REAL NOT NULL DEFAULT 0.0,
                current_price REAL NOT NULL DEFAULT 0.0,
                total_tokens REAL NOT NULL DEFAULT 0.0,
                unrealized_pnl_sol REAL NOT NULL DEFAULT 0.0,
                unrealized_pnl_percent REAL NOT NULL DEFAULT 0.0,
                realized_pnl_sol REAL NOT NULL DEFAULT 0.0,
                dca_count INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'Active',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                peak_price REAL NOT NULL DEFAULT 0.0,
                lowest_price REAL NOT NULL DEFAULT 0.0
            )",
            []
        )?;

        // Trades table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                position_id INTEGER NOT NULL,
                token_address TEXT NOT NULL,
                trade_type TEXT NOT NULL,
                amount_sol REAL NOT NULL,
                amount_tokens REAL NOT NULL,
                price_per_token REAL NOT NULL,
                fees REAL NOT NULL DEFAULT 0.0,
                slippage REAL NOT NULL DEFAULT 0.0,
                transaction_hash TEXT,
                success BOOLEAN NOT NULL DEFAULT 0,
                error TEXT,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (position_id) REFERENCES positions (id)
            )",
            []
        )?;

        // Trade signals table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trade_signals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_address TEXT NOT NULL,
                signal_type TEXT NOT NULL,
                current_price REAL NOT NULL,
                trigger_price REAL NOT NULL,
                volume_24h REAL NOT NULL DEFAULT 0.0,
                liquidity REAL NOT NULL DEFAULT 0.0,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            []
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_positions_token_address ON positions(token_address)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_position_id ON trades(position_id)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_token_address ON trades(token_address)",
            []
        )?;

        // Add new columns if they don't exist (migration)
        let _ = conn.execute("ALTER TABLE positions ADD COLUMN peak_price REAL DEFAULT 0.0", []);
        let _ = conn.execute("ALTER TABLE positions ADD COLUMN lowest_price REAL DEFAULT 0.0", []);
        let _ = conn.execute("ALTER TABLE positions ADD COLUMN total_opens INTEGER DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE positions ADD COLUMN total_closes INTEGER DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE positions ADD COLUMN total_dca INTEGER DEFAULT 0", []);

        Ok(())
    }

    pub fn create_position(&self, token_address: &str, token_symbol: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT INTO positions (token_address, token_symbol) VALUES (?1, ?2)"
        )?;

        stmt.execute(params![token_address, token_symbol])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_position(&self, position_id: i64, summary: &PositionSummary) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE positions SET 
                token_symbol = ?1, total_invested_sol = ?2, original_entry_price = ?3,
                average_buy_price = ?4, current_price = ?5, total_tokens = ?6,
                unrealized_pnl_sol = ?7, unrealized_pnl_percent = ?8, realized_pnl_sol = ?9,
                dca_count = ?10, status = ?11, peak_price = ?12, lowest_price = ?13, 
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?14",
            params![
                summary.token_symbol,
                summary.total_invested_sol,
                summary.original_entry_price,
                summary.average_buy_price,
                summary.current_price,
                summary.total_tokens,
                summary.unrealized_pnl_sol,
                summary.unrealized_pnl_percent,
                summary.realized_pnl_sol,
                summary.dca_count,
                format!("{:?}", summary.status),
                summary.peak_price,
                summary.lowest_price,
                position_id
            ]
        )?;
        Ok(())
    }

    pub fn get_position(&self, token_address: &str) -> Result<Option<(i64, PositionSummary)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, token_address, token_symbol, total_invested_sol, original_entry_price,
                    average_buy_price, current_price, total_tokens, unrealized_pnl_sol, 
                    unrealized_pnl_percent, realized_pnl_sol, dca_count, status, created_at, 
                    updated_at, COALESCE(peak_price, 0.0), COALESCE(lowest_price, 0.0)
             FROM positions WHERE token_address = ?1"
        )?;

        let mut rows = stmt.query_map(params![token_address], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                PositionSummary {
                    token_address: row.get(1)?,
                    token_symbol: row.get(2)?,
                    total_invested_sol: row.get(3)?,
                    original_entry_price: row.get(4)?,
                    average_buy_price: row.get(5)?,
                    current_price: row.get(6)?,
                    total_tokens: row.get(7)?,
                    unrealized_pnl_sol: row.get(8)?,
                    unrealized_pnl_percent: row.get(9)?,
                    realized_pnl_sol: row.get(10)?,
                    dca_count: row.get::<_, u32>(11)?,
                    status: match row.get::<_, String>(12)?.as_str() {
                        "Active" => PositionStatus::Active,
                        "Closed" => PositionStatus::Closed,
                        "StopLoss" => PositionStatus::StopLoss,
                        "TakeProfit" => PositionStatus::TakeProfit,
                        _ => PositionStatus::Active,
                    },
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(14)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    peak_price: row.get::<_, f64>(15)?,
                    lowest_price: row.get::<_, f64>(16)?,
                },
            ))
        })?;

        match rows.next() {
            Some(result) => Ok(Some(result?)),
            None => Ok(None),
        }
    }

    pub fn get_active_positions(&self) -> Result<Vec<(i64, PositionSummary)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, token_address, token_symbol, total_invested_sol, original_entry_price,
                    average_buy_price, current_price, total_tokens, unrealized_pnl_sol, 
                    unrealized_pnl_percent, realized_pnl_sol, dca_count, status, created_at, 
                    updated_at, COALESCE(peak_price, 0.0), COALESCE(lowest_price, 0.0)
             FROM positions WHERE status = 'Active'"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                PositionSummary {
                    token_address: row.get(1)?,
                    token_symbol: row.get(2)?,
                    total_invested_sol: row.get(3)?,
                    original_entry_price: row.get(4)?,
                    average_buy_price: row.get(5)?,
                    current_price: row.get(6)?,
                    total_tokens: row.get(7)?,
                    unrealized_pnl_sol: row.get(8)?,
                    unrealized_pnl_percent: row.get(9)?,
                    realized_pnl_sol: row.get(10)?,
                    dca_count: row.get::<_, u32>(11)?,
                    status: match row.get::<_, String>(12)?.as_str() {
                        "Active" => PositionStatus::Active,
                        "Closed" => PositionStatus::Closed,
                        "StopLoss" => PositionStatus::StopLoss,
                        "TakeProfit" => PositionStatus::TakeProfit,
                        _ => PositionStatus::Active,
                    },
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(14)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    peak_price: row.get::<_, f64>(15)?,
                    lowest_price: row.get::<_, f64>(16)?,
                },
            ))
        })?;

        let mut positions = Vec::new();
        for row in rows {
            positions.push(row?);
        }
        Ok(positions)
    }

    pub fn record_trade(
        &self,
        position_id: i64,
        token_address: &str,
        trade_type: &str,
        result: &TradeResult
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trades (position_id, token_address, trade_type, amount_sol, amount_tokens,
                               price_per_token, fees, slippage, transaction_hash, success, error, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                position_id,
                token_address,
                trade_type,
                result.amount_sol,
                result.amount_tokens,
                result.price_per_token,
                result.fees,
                result.slippage,
                result.transaction_hash,
                result.success,
                result.error,
                result.timestamp.to_rfc3339()
            ]
        )?;
        Ok(())
    }

    pub fn record_signal(&self, signal: &TradeSignal) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trade_signals (token_address, signal_type, current_price, trigger_price, volume_24h, liquidity)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                signal.token_address,
                format!("{:?}", signal.signal_type),
                signal.current_price,
                signal.trigger_price,
                signal.volume_24h,
                signal.liquidity
            ]
        )?;
        Ok(())
    }

    pub fn get_closed_positions(&self, limit: u32) -> Result<Vec<(i64, PositionSummary)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, token_address, token_symbol, total_invested_sol, original_entry_price,
                    average_buy_price, current_price, total_tokens, unrealized_pnl_sol, 
                    unrealized_pnl_percent, realized_pnl_sol, dca_count, status, created_at, 
                    updated_at, COALESCE(peak_price, 0.0), COALESCE(lowest_price, 0.0)
             FROM positions 
             WHERE status != 'Active' 
             ORDER BY updated_at DESC 
             LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                PositionSummary {
                    token_address: row.get(1)?,
                    token_symbol: row.get(2)?,
                    total_invested_sol: row.get(3)?,
                    original_entry_price: row.get(4)?,
                    average_buy_price: row.get(5)?,
                    current_price: row.get(6)?,
                    total_tokens: row.get(7)?,
                    unrealized_pnl_sol: row.get(8)?,
                    unrealized_pnl_percent: row.get(9)?,
                    realized_pnl_sol: row.get(10)?,
                    dca_count: row.get::<_, u32>(11)?,
                    status: match row.get::<_, String>(12)?.as_str() {
                        "Active" => PositionStatus::Active,
                        "Closed" => PositionStatus::Closed,
                        "StopLoss" => PositionStatus::StopLoss,
                        "TakeProfit" => PositionStatus::TakeProfit,
                        _ => PositionStatus::Closed,
                    },
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(14)?)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    peak_price: row.get::<_, f64>(15)?,
                    lowest_price: row.get::<_, f64>(16)?,
                },
            ))
        })?;

        let mut positions = Vec::new();
        for row in rows {
            positions.push(row?);
        }
        Ok(positions)
    }

    pub fn get_trader_stats(&self) -> Result<TraderStats> {
        let conn = self.conn.lock().unwrap();

        // Get trade execution stats (successful/failed executions)
        let mut stmt = conn.prepare(
            "SELECT 
                COUNT(*) as total_trades,
                COALESCE(SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END), 0) as successful_trades,
                COALESCE(SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END), 0) as failed_trades,
                COALESCE(AVG(amount_sol), 0.0) as avg_trade_size
             FROM trades"
        )?;

        let trade_stats: (u32, u32, u32, f64) = stmt.query_row([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        // Get position stats including actual win/loss calculation
        let mut position_stmt = conn.prepare(
            "SELECT 
                COALESCE(SUM(total_invested_sol), 0.0) as total_invested,
                COALESCE(SUM(realized_pnl_sol), 0.0) as total_realized_pnl,
                COALESCE(SUM(unrealized_pnl_sol), 0.0) as total_unrealized_pnl,
                COUNT(CASE WHEN status = 'Active' THEN 1 END) as active_positions,
                COUNT(CASE WHEN status != 'Active' THEN 1 END) as closed_positions,
                COUNT(CASE WHEN status != 'Active' AND realized_pnl_sol > 0 THEN 1 END) as winning_positions,
                COUNT(CASE WHEN status != 'Active' AND realized_pnl_sol <= 0 THEN 1 END) as losing_positions
             FROM positions"
        )?;

        let position_stats: (f64, f64, f64, u32, u32, u32, u32) = position_stmt.query_row(
            [],
            |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, u32>(3)?,
                    row.get::<_, u32>(4)?,
                    row.get::<_, u32>(5)?,
                    row.get::<_, u32>(6)?,
                ))
            }
        )?;

        // Get largest win/loss
        let mut pnl_stmt = conn.prepare(
            "SELECT COALESCE(MAX(realized_pnl_sol), 0.0) as largest_win, 
                    COALESCE(MIN(realized_pnl_sol), 0.0) as largest_loss
             FROM positions WHERE status != 'Active'"
        )?;

        let (largest_win, largest_loss): (f64, f64) = pnl_stmt.query_row([], |row| {
            Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?))
        })?;

        // Calculate actual win rate based on profitable vs losing positions
        let total_closed_positions = position_stats.4;
        let winning_positions = position_stats.5;
        let actual_win_rate = if total_closed_positions > 0 {
            ((winning_positions as f64) / (total_closed_positions as f64)) * 100.0
        } else {
            0.0
        };

        Ok(TraderStats {
            total_trades: trade_stats.0,
            successful_trades: trade_stats.1,
            failed_trades: trade_stats.2,
            total_invested_sol: position_stats.0,
            total_realized_pnl_sol: position_stats.1,
            total_unrealized_pnl_sol: position_stats.2,
            win_rate: actual_win_rate, // Fixed: now based on profitable positions, not trade execution success
            average_trade_size_sol: trade_stats.3,
            largest_win_sol: largest_win,
            largest_loss_sol: largest_loss,
            active_positions: position_stats.3,
            closed_positions: position_stats.4,
        })
    }
}
