//! Position database queries — read-only methods for fetching positions.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::positions::types::Position;

use super::types::*;

impl PositionsDatabase {
    /// Get position by ID
    pub async fn get_position_by_id(&self, id: i64) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
            "SELECT {} FROM positions WHERE id = ?1 AND wallet_address = ?2",
            POSITION_SELECT_COLUMNS
        );
        let result = conn
            .query_row(&query, params![id, wallet_address], |row| {
                self.row_to_position(row)
            })
            .optional()
            .map_err(|e| format!("Failed to get position by ID: {e}"))?;

        Ok(result)
    }

    /// Get position by mint
    pub async fn get_position_by_mint(&self, mint: &str) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
      "SELECT {} FROM positions WHERE mint = ?1 AND wallet_address = ?2 ORDER BY entry_time DESC LIMIT 1",
      POSITION_SELECT_COLUMNS
    );
        let result = conn
            .query_row(&query, params![mint, wallet_address], |row| {
                self.row_to_position(row)
            })
            .optional()
            .map_err(|e| format!("Failed to get position by mint: {e}"))?;

        Ok(result)
    }

    /// Get position by entry transaction signature
    pub async fn get_position_by_entry_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let result = conn
            .query_row(
                r#"
      SELECT id, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
          position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
          entry_transaction_signature, exit_transaction_signature, token_amount,
          effective_entry_price, effective_exit_price, sol_received,
          profit_target_min, profit_target_max, liquidity_tier,
          transaction_entry_verified, transaction_exit_verified,
          entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
          phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
          remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
          dca_count, average_entry_price, last_dca_time
      FROM positions WHERE entry_transaction_signature = ?1 AND wallet_address = ?2
      "#,
                params![signature, wallet_address],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(|e| format!("Failed to get position by entry signature: {e}"))?;

        Ok(result)
    }

    /// Get position by exit transaction signature
    pub async fn get_position_by_exit_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
      "SELECT {} FROM positions WHERE exit_transaction_signature = ?1 AND wallet_address = ?2",
      POSITION_SELECT_COLUMNS
    );

        let result = conn
            .query_row(&query, params![signature, wallet_address], |row| {
                self.row_to_position(row)
            })
            .optional()
            .map_err(|e| format!("Failed to get position by exit signature: {e}"))?;

        Ok(result)
    }

    /// Get all positions with optional filtering
    pub async fn get_positions(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let mut query = format!(
            "SELECT {} FROM positions ORDER BY entry_time DESC",
            POSITION_SELECT_COLUMNS
        );

        if let Some(limit) = limit {
            query.push_str(&format!("LIMIT {limit}"));
            if let Some(offset) = offset {
                query.push_str(&format!("OFFSET {offset}"));
            }
        }

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare positions query: {e}"))?;

        let position_iter = stmt
            .query_map([], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute positions query: {e}"))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {e}"))?);
        }

        Ok(positions)
    }

    /// Get open positions (not archived, no exit recorded, exit tx not verified).
    /// Mirrors the in-memory filter in state::get_open_positions().
    pub async fn get_open_positions(&self) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND archived = 0 AND exit_time IS NULL AND transaction_exit_verified = 0 ORDER BY entry_time DESC",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare open positions query: {e}"))?;

        let position_iter = stmt
            .query_map(params![wallet_address], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute open positions query: {e}"))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {e}"))?);
        }

        Ok(positions)
    }

    /// Get closed positions (exit verified, not archived).
    /// Mirrors the in-memory filter in state::get_closed_positions().
    pub async fn get_closed_positions(&self) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND archived = 0 AND transaction_exit_verified = 1 ORDER BY exit_time DESC",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare closed positions query: {e}"))?;

        let position_iter = stmt
            .query_map(params![wallet_address], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute closed positions query: {e}"))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {e}"))?);
        }

        Ok(positions)
    }

    /// Get closed positions since a specific date (have exit_time >= since)
    pub async fn get_closed_positions_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND archived = 0 AND transaction_exit_verified = 1 AND datetime(exit_time) >= datetime(?2) ORDER BY exit_time DESC",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare closed positions since query: {e}"))?;

        let since_str = since.to_rfc3339();
        let position_iter = stmt
            .query_map(params![wallet_address, since_str], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| format!("Failed to execute closed positions since query: {e}"))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {e}"))?);
        }

        Ok(positions)
    }

    /// Count closed positions since the provided timestamp
    pub async fn count_closed_positions_since(&self, since: DateTime<Utc>) -> Result<i64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT COUNT(1)
      FROM positions
      WHERE wallet_address = ?1
       AND archived = 0
       AND transaction_exit_verified = 1
       AND exit_time IS NOT NULL
       AND datetime(exit_time) >= datetime(?2)
      "#,
            )
            .map_err(|e| format!("Failed to prepare closed position count query: {e}"))?;

        let since_str = since.to_rfc3339();
        let count: i64 = stmt
            .query_row(params![wallet_address, since_str], |row| row.get(0))
            .map_err(|e| format!("Failed to execute closed position count query: {e}"))?;

        Ok(count)
    }

    /// Get aggregated trading statistics for a time period (OPTIMIZED - SQL aggregation)
    /// This replaces fetching all positions and calculating in Rust
    pub async fn get_period_trading_stats(
        &self,
        period_start: DateTime<Utc>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<PeriodTradingStats, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = if period_end.is_some() {
            r#"
      SELECT 
        COUNT(*) as trade_count,
        SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
        COALESCE(SUM(CASE WHEN pnl > 0 THEN pnl ELSE 0 END), 0) as profit,
        COALESCE(SUM(CASE WHEN pnl < 0 THEN ABS(pnl) ELSE 0 END), 0) as loss,
        COALESCE(SUM(pnl), 0) as total_pnl,
        COALESCE(SUM(1 + dca_count), 0) as total_buys,
        COALESCE(SUM(CASE 
          WHEN partial_exit_count > 0 
          THEN partial_exit_count + 1 
          ELSE 1 
        END), 0) as total_sells,
        COALESCE(MAX(CASE WHEN pnl_percent < 0 THEN ABS(pnl_percent) ELSE 0 END), 0) as max_dd
      FROM positions 
      WHERE wallet_address = ?1 
        AND transaction_exit_verified = 1
        AND exit_time IS NOT NULL
        AND datetime(exit_time) >= datetime(?2)
        AND datetime(exit_time) < datetime(?3)
      "#
        } else {
            r#"
      SELECT 
        COUNT(*) as trade_count,
        SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
        COALESCE(SUM(CASE WHEN pnl > 0 THEN pnl ELSE 0 END), 0) as profit,
        COALESCE(SUM(CASE WHEN pnl < 0 THEN ABS(pnl) ELSE 0 END), 0) as loss,
        COALESCE(SUM(pnl), 0) as total_pnl,
        COALESCE(SUM(1 + dca_count), 0) as total_buys,
        COALESCE(SUM(CASE 
          WHEN partial_exit_count > 0 
          THEN partial_exit_count + 1 
          ELSE 1 
        END), 0) as total_sells,
        COALESCE(MAX(CASE WHEN pnl_percent < 0 THEN ABS(pnl_percent) ELSE 0 END), 0) as max_dd
      FROM positions 
      WHERE wallet_address = ?1 
        AND transaction_exit_verified = 1
        AND exit_time IS NOT NULL
        AND datetime(exit_time) >= datetime(?2)
      "#
        };

        let start_str = period_start.to_rfc3339();

        let stats = if let Some(end) = period_end {
            let end_str = end.to_rfc3339();
            conn.query_row(query, params![wallet_address, start_str, end_str], |row| {
                let trade_count: i64 = row.get(0)?;
                let wins: Option<i64> = row.get(1)?;
                let profit: f64 = row.get(2)?;
                let loss: f64 = row.get(3)?;
                let total_pnl: f64 = row.get(4)?;
                let total_buys: i64 = row.get(5)?;
                let total_sells: i64 = row.get(6)?;
                let max_dd: f64 = row.get(7)?;

                let win_rate = if trade_count > 0 {
                    (wins.unwrap_or_default() as f64 / trade_count as f64) * 100.0
                } else {
                    0.0
                };

                Ok(PeriodTradingStats {
                    buys: total_buys,
                    sells: total_sells,
                    profit_sol: profit,
                    loss_sol: loss,
                    net_pnl_sol: total_pnl,
                    drawdown_percent: max_dd,
                    win_rate,
                })
            })
            .map_err(|e| format!("Failed to execute period stats query: {e}"))?
        } else {
            conn.query_row(query, params![wallet_address, start_str], |row| {
                let trade_count: i64 = row.get(0)?;
                let wins: Option<i64> = row.get(1)?;
                let profit: f64 = row.get(2)?;
                let loss: f64 = row.get(3)?;
                let total_pnl: f64 = row.get(4)?;
                let total_buys: i64 = row.get(5)?;
                let total_sells: i64 = row.get(6)?;
                let max_dd: f64 = row.get(7)?;

                let win_rate = if trade_count > 0 {
                    (wins.unwrap_or_default() as f64 / trade_count as f64) * 100.0
                } else {
                    0.0
                };

                Ok(PeriodTradingStats {
                    buys: total_buys,
                    sells: total_sells,
                    profit_sol: profit,
                    loss_sol: loss,
                    net_pnl_sol: total_pnl,
                    drawdown_percent: max_dd,
                    win_rate,
                })
            })
            .map_err(|e| format!("Failed to execute period stats query: {e}"))?
        };

        Ok(stats)
    }

    /// Get realized trading statistics grouped by calendar day (UTC) for a period.
    /// Used by the home portfolio calendar. Only exit-verified, closed positions count,
    /// keyed by the day their exit was recorded (`exit_time`).
    pub async fn get_daily_trading_stats(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<DailyTradingStats>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                r#"
        SELECT
          strftime('%Y-%m-%d', exit_time) as day,
          COUNT(*) as trades,
          SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
          COALESCE(SUM(CASE WHEN pnl > 0 THEN pnl ELSE 0 END), 0) as profit,
          COALESCE(SUM(CASE WHEN pnl < 0 THEN ABS(pnl) ELSE 0 END), 0) as loss,
          COALESCE(SUM(pnl), 0) as total_pnl
        FROM positions
        WHERE wallet_address = ?1
          AND transaction_exit_verified = 1
          AND exit_time IS NOT NULL
          AND datetime(exit_time) >= datetime(?2)
          AND datetime(exit_time) < datetime(?3)
        GROUP BY day
        ORDER BY day ASC
        "#,
            )
            .map_err(|e| format!("Failed to prepare daily stats query: {e}"))?;

        let rows = stmt
            .query_map(
                params![
                    wallet_address,
                    period_start.to_rfc3339(),
                    period_end.to_rfc3339()
                ],
                |row| {
                    Ok(DailyTradingStats {
                        date: row.get(0)?,
                        trades: row.get(1)?,
                        wins: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                        profit_sol: row.get(3)?,
                        loss_sol: row.get(4)?,
                        net_pnl_sol: row.get(5)?,
                    })
                },
            )
            .map_err(|e| format!("Failed to execute daily stats query: {e}"))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("Failed to read daily stats row: {e}"))?);
        }
        Ok(result)
    }

    /// Get recent closed & verified positions for a specific mint (exit verified)
    /// Ordered by most recent exit_time DESC. Used for adaptive re-entry profit capping.
    pub async fn get_recent_closed_positions_for_mint(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let query = format!(
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND mint = ?2 AND transaction_exit_verified = 1 AND exit_price IS NOT NULL AND exit_time IS NOT NULL ORDER BY exit_time DESC LIMIT ?3",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare recent closed positions query: {e}"))?;

        let rows = stmt
            .query_map(params![wallet_address, mint, limit as i64], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| format!("Failed to execute recent closed positions query: {e}"))?;

        let mut positions = Vec::new();
        for row in rows {
            if let Ok(p) = row {
                positions.push(p);
            }
        }
        Ok(positions)
    }

    /// Lightweight variant: only fetch (exit_price, effective_exit_price) for recent verified exits
    /// to reduce row size & parsing overhead for re-entry heuristics.
    pub async fn get_recent_closed_exit_prices_for_mint(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<(Option<f64>, Option<f64>)>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT exit_price, effective_exit_price
      FROM positions
      WHERE wallet_address = ?1 AND mint = ?2 AND transaction_exit_verified = 1
       AND exit_price IS NOT NULL AND exit_time IS NOT NULL
      ORDER BY datetime(exit_time) DESC
      LIMIT ?3
      "#,
            )
            .map_err(|e| format!("Failed to prepare recent closed exit prices query: {e}"))?;

        let mut out: Vec<(Option<f64>, Option<f64>)> = Vec::new();
        let rows = stmt
            .query_map(params![wallet_address, mint, limit as i64], |row| {
                let exit_p: Option<f64> = row.get(0).ok();
                let eff_exit_p: Option<f64> = row.get(1).ok();
                Ok((exit_p, eff_exit_p))
            })
            .map_err(|e| format!("Failed to execute recent closed exit prices query: {e}"))?;
        for r in rows {
            if let Ok(v) = r {
                out.push(v);
            }
        }
        Ok(out)
    }

    /// Get positions by state
    pub async fn get_positions_by_state(
        &self,
        state: &PositionState,
    ) -> Result<Vec<Position>, String> {
        // This requires joining with position_states to get current state
        let conn = self.get_connection()?;

        let mut stmt = conn
      .prepare(
        r#"
      SELECT p.id, p.mint, p.symbol, p.name, p.entry_price, p.entry_time, p.exit_price, p.exit_time,
          p.position_type, p.entry_size_sol, p.total_size_sol, p.price_highest, p.price_lowest,
          p.entry_transaction_signature, p.exit_transaction_signature, p.token_amount,
          p.effective_entry_price, p.effective_exit_price, p.sol_received,
          p.profit_target_min, p.profit_target_max, p.liquidity_tier,
          p.transaction_entry_verified, p.transaction_exit_verified,
          p.entry_fee_lamports, p.exit_fee_lamports, p.current_price, p.current_price_updated,
          p.phantom_confirmations, p.phantom_first_seen, p.synthetic_exit, p.closed_reason,
          p.remaining_token_amount, p.total_exited_amount, p.average_exit_price, p.partial_exit_count,
          p.dca_count, p.average_entry_price, p.last_dca_time
      FROM positions p
      WHERE EXISTS (
        SELECT 1 FROM position_states ps
        WHERE ps.position_id = p.id
         AND ps.state = ?1
         AND ps.changed_at = (
           SELECT MAX(ps2.changed_at)
           FROM position_states ps2
           WHERE ps2.position_id = p.id
         )
      )
      ORDER BY p.entry_time DESC
      "#
      )
      .map_err(|e| format!("Failed to prepare positions by state query: {e}"))?;

        let position_iter = stmt
            .query_map(params![state.to_string()], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute positions by state query: {e}"))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {e}"))?);
        }

        Ok(positions)
    }

    /// Get positions with unverified transactions
    pub async fn get_unverified_positions(&self) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT id, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
          position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
          entry_transaction_signature, exit_transaction_signature, token_amount,
          effective_entry_price, effective_exit_price, sol_received,
          profit_target_min, profit_target_max, liquidity_tier,
          transaction_entry_verified, transaction_exit_verified,
          entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
          phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
          remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
          dca_count, average_entry_price, last_dca_time
      FROM positions 
      WHERE transaction_entry_verified = false 
        OR (exit_transaction_signature IS NOT NULL AND transaction_exit_verified = false)
      ORDER BY entry_time DESC
      "#,
            )
            .map_err(|e| format!("Failed to prepare unverified positions query: {e}"))?;

        let position_iter = stmt
            .query_map([], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute unverified positions query: {e}"))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {e}"))?);
        }

        Ok(positions)
    }
}
