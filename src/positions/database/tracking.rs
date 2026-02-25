//! Position database tracking — delete, state tracking, snapshots, and maintenance operations.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::logger::{self, LogTag};

use super::types::*;

impl PositionsDatabase {
    /// Delete position by ID
    pub async fn delete_position(&self, id: i64) -> Result<bool, String> {
        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute("DELETE FROM positions WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete position: {e}"))?;

        Ok(rows_affected > 0)
    }

    /// Delete position by entry signature
    pub async fn delete_position_by_entry_signature(
        &self,
        signature: &str,
    ) -> Result<bool, String> {
        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                "DELETE FROM positions WHERE entry_transaction_signature = ?1",
                params![signature],
            )
            .map_err(|e| format!("Failed to delete position by entry signature: {e}"))?;

        if rows_affected > 0 {
            logger::info(
                LogTag::Positions,
                &format!("Deleted position with entry signature: {signature}"),
            );
        }

        Ok(rows_affected > 0)
    }

    /// Record state change for position
    pub async fn record_state_change(
        &self,
        position_id: i64,
        state: PositionState,
        reason: Option<&str>,
    ) -> Result<(), String> {
        let changed_at = Utc::now().to_rfc3339();

        logger::debug(
            LogTag::Positions,
            &format!(
                "Recording state change for position ID {}: {} (reason: {}) at {}",
                position_id,
                state,
                reason.unwrap_or("None"),
                changed_at
            ),
        );

        let conn = self.get_connection()?;

        conn.execute(
      "INSERT INTO position_states (position_id, state, changed_at, reason) VALUES (?1, ?2, ?3, ?4)",
      params![position_id, state.to_string(), changed_at, reason],
    )
    .map_err(|e| format!("Failed to record state change: {e}"))?;

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully recorded state change for position ID {}: {}",
                position_id, state
            ),
        );

        Ok(())
    }

    /// Get position state history
    pub async fn get_position_state_history(
        &self,
        position_id: i64,
    ) -> Result<Vec<PositionStateHistory>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
      .prepare(
        "SELECT position_id, state, changed_at, reason FROM position_states WHERE position_id = ?1 ORDER BY changed_at DESC"
      )
      .map_err(|e| format!("Failed to prepare state history query: {e}"))?;

        let history_iter = stmt
            .query_map(params![position_id], |row| {
                let state_str: String = row.get(1)?;
                let state = state_str.parse::<PositionState>().map_err(|e| {
                    rusqlite::Error::InvalidColumnType(
                        1,
                        "Invalid state".to_owned(),
                        rusqlite::types::Type::Text,
                    )
                })?;

                let changed_at_str: String = row.get(2)?;
                let changed_at = DateTime::parse_from_rfc3339(&changed_at_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid datetime".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(PositionStateHistory {
                    position_id: row.get(0)?,
                    state,
                    changed_at,
                    reason: row.get(3)?,
                })
            })
            .map_err(|e| format!("Failed to execute state history query: {e}"))?;

        let mut history = Vec::new();
        for history_result in history_iter {
            history.push(
                history_result.map_err(|e| format!("Failed to parse state history row: {e}"))?,
            );
        }

        Ok(history)
    }

    /// Record position tracking data
    pub async fn record_position_tracking(
        &self,
        tracking: &PositionTracking,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
      .execute(
        r#"
      INSERT INTO position_tracking (position_id, price, price_source, pool_type, pool_address, api_price, tracked_at)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
      "#,
        params![
          tracking.position_id,
          tracking.price,
          tracking.price_source,
          tracking.pool_type,
          tracking.pool_address,
          tracking.api_price,
          tracking.tracked_at.to_rfc3339()
        ]
      )
      .map_err(|e| format!("Failed to record position tracking: {e}"))?;

        Ok(())
    }

    /// Get recent position tracking data
    pub async fn get_recent_position_tracking(
        &self,
        position_id: i64,
        limit: usize,
    ) -> Result<Vec<PositionTracking>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT position_id, price, price_source, pool_type, pool_address, api_price, tracked_at
      FROM position_tracking 
      WHERE position_id = ?1 
      ORDER BY tracked_at DESC 
      LIMIT ?2
      "#,
            )
            .map_err(|e| format!("Failed to prepare tracking query: {e}"))?;

        let tracking_iter = stmt
            .query_map(params![position_id, limit], |row| {
                let tracked_at_str: String = row.get(6)?;
                let tracked_at = DateTime::parse_from_rfc3339(&tracked_at_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            6,
                            "Invalid datetime".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(PositionTracking {
                    position_id: row.get(0)?,
                    price: row.get(1)?,
                    price_source: row.get(2)?,
                    pool_type: row.get(3)?,
                    pool_address: row.get(4)?,
                    api_price: row.get(5)?,
                    tracked_at,
                })
            })
            .map_err(|e| format!("Failed to execute tracking query: {e}"))?;

        let mut tracking_data = Vec::new();
        for tracking_result in tracking_iter {
            tracking_data
                .push(tracking_result.map_err(|e| format!("Failed to parse tracking row: {e}"))?);
        }

        Ok(tracking_data)
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<PositionsDatabaseStats, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let total_positions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM positions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count total positions: {e}"))?;

        let open_positions: i64 = conn
      .query_row(
        "SELECT COUNT(*) FROM positions WHERE wallet_address = ?1 AND transaction_exit_verified = 0",
        params![wallet_address],
        |row| row.get(0),
      )
      .map_err(|e| format!("Failed to count open positions: {e}"))?;

        let closed_positions: i64 = conn
      .query_row(
        "SELECT COUNT(*) FROM positions WHERE wallet_address = ?1 AND transaction_exit_verified = 1",
        params![wallet_address],
        |row| row.get(0),
      )
      .map_err(|e| format!("Failed to count closed positions: {e}"))?;

        let phantom_positions: i64 = conn
      .query_row(
        "SELECT COUNT(*) FROM positions WHERE wallet_address = ?1 AND phantom_confirmations > 0",
        params![wallet_address],
        |row| row.get(0),
      )
      .map_err(|e| format!("Failed to count phantom positions: {e}"))?;

        let total_state_history: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM position_states WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count state history: {e}"))?;

        let total_tracking_records: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM position_tracking WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count tracking records: {e}"))?;

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or_default();

        Ok(PositionsDatabaseStats {
            total_positions: total_positions as u64,
            open_positions: open_positions as u64,
            closed_positions: closed_positions as u64,
            phantom_positions: phantom_positions as u64,
            total_state_history: total_state_history as u64,
            total_tracking_records: total_tracking_records as u64,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Vacuum database to reclaim space and optimize performance
    pub async fn vacuum_database(&self) -> Result<(), String> {
        logger::info(
            LogTag::Positions,
            "Starting positions database vacuum operation...",
        );

        let conn = self.get_connection()?;
        conn.execute("VACUUM", [])
            .map_err(|e| format!("Failed to vacuum positions database: {e}"))?;

        logger::info(
            LogTag::Positions,
            "Positions database vacuum completed successfully",
        );
        Ok(())
    }

    /// Analyze database for query optimization
    pub async fn analyze_database(&self) -> Result<(), String> {
        logger::info(
            LogTag::Positions,
            "Running positions database analysis for optimization...",
        );

        let conn = self.get_connection()?;
        conn.execute("ANALYZE", [])
            .map_err(|e| format!("Failed to analyze positions database: {e}"))?;

        logger::info(
            LogTag::Positions,
            "Positions database analysis completed successfully",
        );
        Ok(())
    }

    /// Save token snapshot to database
    pub async fn save_token_snapshot(&self, snapshot: &TokenSnapshot) -> Result<i64, String> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Saving token snapshot for position ID {} (type: {}) with price {:.6} SOL",
                snapshot.position_id,
                snapshot.snapshot_type,
                snapshot.price_sol.unwrap_or_default()
            ),
        );

        let conn = self.get_connection()?;

        let result = conn
      .execute(
        r#"
      INSERT INTO token_snapshots (
        position_id, snapshot_type, mint, symbol, name, price_sol, price_usd, price_native,
        dex_id, pair_address, pair_url, fdv, market_cap, pair_created_at,
        liquidity_usd, liquidity_base, liquidity_quote,
        volume_h24, volume_h6, volume_h1, volume_m5,
        txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
        txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
        price_change_h24, price_change_h6, price_change_h1, price_change_m5,
        token_uri, token_description, token_image, token_website, token_twitter, token_telegram,
        snapshot_time, api_fetch_time, data_freshness_score
      ) VALUES (
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
        ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40,
        ?41, ?42
      )
      "#,
        params![
          snapshot.position_id,
          snapshot.snapshot_type,
          snapshot.mint,
          snapshot.symbol,
          snapshot.name,
          snapshot.price_sol,
          snapshot.price_usd,
          snapshot.price_native,
          snapshot.dex_id,
          snapshot.pair_address,
          snapshot.pair_url,
          snapshot.fdv,
          snapshot.market_cap,
          snapshot.pair_created_at,
          snapshot.liquidity_usd,
          snapshot.liquidity_base,
          snapshot.liquidity_quote,
          snapshot.volume_h24,
          snapshot.volume_h6,
          snapshot.volume_h1,
          snapshot.volume_m5,
          snapshot.txns_h24_buys,
          snapshot.txns_h24_sells,
          snapshot.txns_h6_buys,
          snapshot.txns_h6_sells,
          snapshot.txns_h1_buys,
          snapshot.txns_h1_sells,
          snapshot.txns_m5_buys,
          snapshot.txns_m5_sells,
          snapshot.price_change_h24,
          snapshot.price_change_h6,
          snapshot.price_change_h1,
          snapshot.price_change_m5,
          snapshot.token_uri,
          snapshot.token_description,
          snapshot.token_image,
          snapshot.token_website,
          snapshot.token_twitter,
          snapshot.token_telegram,
          snapshot.snapshot_time.to_rfc3339(),
          snapshot.api_fetch_time.to_rfc3339(),
          snapshot.data_freshness_score
        ]
      )
      .map_err(|e| format!("Failed to insert token snapshot: {e}"))?;

        let snapshot_id = conn.last_insert_rowid();

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully saved token snapshot ID {} for position ID {} (type: {})",
                snapshot_id, snapshot.position_id, snapshot.snapshot_type
            ),
        );

        Ok(snapshot_id)
    }

    /// Get token snapshots for a position
    pub async fn get_token_snapshots(
        &self,
        position_id: i64,
    ) -> Result<Vec<TokenSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT id, position_id, snapshot_type, mint, symbol, name, price_sol, price_usd, price_native,
          dex_id, pair_address, pair_url, fdv, market_cap, pair_created_at,
          liquidity_usd, liquidity_base, liquidity_quote,
          volume_h24, volume_h6, volume_h1, volume_m5,
          txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
          txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
          price_change_h24, price_change_h6, price_change_h1, price_change_m5,
          token_uri, token_description, token_image, token_website, token_twitter, token_telegram,
          snapshot_time, api_fetch_time, data_freshness_score
      FROM token_snapshots 
      WHERE position_id = ?1 
      ORDER BY snapshot_time ASC
      "#,
            )
            .map_err(|e| format!("Failed to prepare token snapshots query: {e}"))?;

        let snapshot_iter = stmt
            .query_map(params![position_id], |row| self.row_to_token_snapshot(row))
            .map_err(|e| format!("Failed to execute token snapshots query: {e}"))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots.push(
                snapshot_result.map_err(|e| format!("Failed to parse token snapshot row: {e}"))?,
            );
        }

        Ok(snapshots)
    }

    /// Get specific token snapshot by type
    pub async fn get_token_snapshot(
        &self,
        position_id: i64,
        snapshot_type: &str,
    ) -> Result<Option<TokenSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT id, position_id, snapshot_type, mint, symbol, name, price_sol, price_usd, price_native,
          dex_id, pair_address, pair_url, fdv, market_cap, pair_created_at,
          liquidity_usd, liquidity_base, liquidity_quote,
          volume_h24, volume_h6, volume_h1, volume_m5,
          txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
          txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
          price_change_h24, price_change_h6, price_change_h1, price_change_m5,
          token_uri, token_description, token_image, token_website, token_twitter, token_telegram,
          snapshot_time, api_fetch_time, data_freshness_score
      FROM token_snapshots 
      WHERE position_id = ?1 AND snapshot_type = ?2
      ORDER BY snapshot_time DESC
      LIMIT 1
      "#,
            )
            .map_err(|e| format!("Failed to prepare token snapshot query: {e}"))?;

        let result = stmt
            .query_row(params![position_id, snapshot_type], |row| {
                self.row_to_token_snapshot(row)
            })
            .optional()
            .map_err(|e| format!("Failed to execute token snapshot query: {e}"))?;

        Ok(result)
    }

    /// Helper function to convert database row to TokenSnapshot struct
    fn row_to_token_snapshot(&self, row: &rusqlite::Row) -> rusqlite::Result<TokenSnapshot> {
        let snapshot_time_str: String = row.get("snapshot_time")?;
        let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "Invalid snapshot_time".to_owned(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        let api_fetch_time_str: String = row.get("api_fetch_time")?;
        let api_fetch_time = DateTime::parse_from_rfc3339(&api_fetch_time_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "Invalid api_fetch_time".to_owned(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        Ok(TokenSnapshot {
            id: Some(row.get("id")?),
            position_id: row.get("position_id")?,
            snapshot_type: row.get("snapshot_type")?,
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            price_sol: row.get("price_sol")?,
            price_usd: row.get("price_usd")?,
            price_native: row.get("price_native")?,
            dex_id: row.get("dex_id")?,
            pair_address: row.get("pair_address")?,
            pair_url: row.get("pair_url")?,
            fdv: row.get("fdv")?,
            market_cap: row.get("market_cap")?,
            pair_created_at: row.get("pair_created_at")?,
            liquidity_usd: row.get("liquidity_usd")?,
            liquidity_base: row.get("liquidity_base")?,
            liquidity_quote: row.get("liquidity_quote")?,
            volume_h24: row.get("volume_h24")?,
            volume_h6: row.get("volume_h6")?,
            volume_h1: row.get("volume_h1")?,
            volume_m5: row.get("volume_m5")?,
            txns_h24_buys: row.get("txns_h24_buys")?,
            txns_h24_sells: row.get("txns_h24_sells")?,
            txns_h6_buys: row.get("txns_h6_buys")?,
            txns_h6_sells: row.get("txns_h6_sells")?,
            txns_h1_buys: row.get("txns_h1_buys")?,
            txns_h1_sells: row.get("txns_h1_sells")?,
            txns_m5_buys: row.get("txns_m5_buys")?,
            txns_m5_sells: row.get("txns_m5_sells")?,
            price_change_h24: row.get("price_change_h24")?,
            price_change_h6: row.get("price_change_h6")?,
            price_change_h1: row.get("price_change_h1")?,
            price_change_m5: row.get("price_change_m5")?,
            token_uri: row.get("token_uri")?,
            token_description: row.get("token_description")?,
            token_image: row.get("token_image")?,
            token_website: row.get("token_website")?,
            token_twitter: row.get("token_twitter")?,
            token_telegram: row.get("token_telegram")?,
            snapshot_time,
            api_fetch_time,
            data_freshness_score: row.get("data_freshness_score")?,
        })
    }
}
