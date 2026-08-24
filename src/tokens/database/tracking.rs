//! Token tracking database — stores discovery timestamps and processing state.

use crate::errors::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::tokens::types::{TokenResult, UpdateTrackingInfo};
use crate::tokens::Error;

use super::TokenDatabase;

impl TokenDatabase {
    /// Mark pool price as calculated (called after Pool Service calculation)

    pub fn mark_pool_price_calculated(&self, mint: &str, pool_address: &str) -> TokenResult<()> {
        let conn = self.conn()?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                pool_price_last_calculated_at = ?1,
                pool_price_last_used_pool_address = ?2
             WHERE chain_id = ?3 AND mint = ?4",
            params![now, pool_address, self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to mark pool price calculated".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    // ========================================================================
    // COUNTS & TRACKING
    // ========================================================================

    /// Count total tokens in the database
    pub fn count_tokens(&self) -> TokenResult<u64> {
        let conn = self.conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens WHERE chain_id = ?1",
                params![self.chain_id()],
                |row| row.get(0),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to count tokens".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(count.max(0) as u64)
    }

    /// Count tokens currently tracked for updates
    pub fn count_tracked_tokens(&self) -> TokenResult<u64> {
        let conn = self.conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM update_tracking WHERE chain_id = ?1",
                params![self.chain_id()],
                |row| row.get(0),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to count tracked tokens".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(count.max(0) as u64)
    }

    /// Count blacklisted tokens
    pub fn count_blacklisted(&self) -> TokenResult<u64> {
        let conn = self.conn()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blacklist WHERE chain_id = ?1",
                params![self.chain_id()],
                |row| row.get(0),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to count blacklisted tokens".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(count.max(0) as u64)
    }

    /// Retrieve update tracking information for a specific token
    pub fn get_update_tracking_info(&self, mint: &str) -> TokenResult<Option<UpdateTrackingInfo>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, priority,
                        market_data_last_updated_at, market_data_update_count,
                        security_data_last_updated_at, security_data_update_count,
                        metadata_last_updated_at, decimals_last_updated_at,
                        pool_price_last_calculated_at, pool_price_last_used_pool_address,
                        last_error, last_error_at, market_error_count, market_error_type,
                        last_security_error, last_security_error_at, security_error_count
                 FROM update_tracking
                 WHERE chain_id = ?1 AND mint = ?2",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let result = stmt.query_row(params![self.chain_id(), mint], |row| map_tracking_row(row));

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })),
        }
    }

    /// List update tracking entries with optional priority filter
    pub fn list_update_tracking(
        &self,
        limit: usize,
        priority: Option<i32>,
    ) -> TokenResult<Vec<UpdateTrackingInfo>> {
        let conn = self.conn()?;

        let records = if let Some(priority) = priority {
            let mut stmt = conn
                .prepare(
                    "SELECT mint, priority,
                            market_data_last_updated_at, market_data_update_count,
                            security_data_last_updated_at, security_data_update_count,
                            metadata_last_updated_at, decimals_last_updated_at,
                            pool_price_last_calculated_at, pool_price_last_used_pool_address,
                            last_error, last_error_at, market_error_count, market_error_type,
                            last_security_error, last_security_error_at, security_error_count
                     FROM update_tracking
                     WHERE chain_id = ?1 AND priority = ?2
                       AND (market_error_type IS NULL OR market_error_type != 'permanent')
                     ORDER BY COALESCE(market_data_last_updated_at, 0) ASC, mint ASC
                     LIMIT ?3",
                )
                .map_err(|e| {
                    Error::Database(DatabaseError::Query {
                        operation: "Failed to prepare".to_owned(),
                        message: e.to_string(),
                    })
                })?;

            let rows = stmt
                .query_map(params![self.chain_id(), priority, limit as i64], |row| {
                    map_tracking_row(row)
                })
                .map_err(|e| {
                    Error::Database(DatabaseError::Query {
                        operation: "Query failed".to_owned(),
                        message: e.to_string(),
                    })
                })?;

            rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to collect tracking entries".to_owned(),
                    message: e.to_string(),
                })
            })?
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT mint, priority,
                            market_data_last_updated_at, market_data_update_count,
                            security_data_last_updated_at, security_data_update_count,
                            metadata_last_updated_at, decimals_last_updated_at,
                            pool_price_last_calculated_at, pool_price_last_used_pool_address,
                            last_error, last_error_at, market_error_count, market_error_type,
                            last_security_error, last_security_error_at, security_error_count
                     FROM update_tracking
                     WHERE chain_id = ?1 AND (market_error_type IS NULL OR market_error_type != 'permanent')
                     ORDER BY priority DESC, COALESCE(market_data_last_updated_at, 0) ASC, mint ASC
                     LIMIT ?2",
                )
                .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to prepare".to_owned(), message: e.to_string() }))?;

            let rows = stmt
                .query_map(params![self.chain_id(), limit as i64], |row| {
                    map_tracking_row(row)
                })
                .map_err(|e| {
                    Error::Database(DatabaseError::Query {
                        operation: "Query failed".to_owned(),
                        message: e.to_string(),
                    })
                })?;

            rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to collect tracking entries".to_owned(),
                    message: e.to_string(),
                })
            })?
        };

        Ok(records)
    }
}

fn map_tracking_row(row: &rusqlite::Row) -> rusqlite::Result<UpdateTrackingInfo> {
    let mint: String = row.get(0)?;
    let priority: i32 = row.get(1)?;
    let market_data_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(2)?);
    let market_data_update_count = row.get::<_, Option<i64>>(3)?.unwrap_or_default().max(0) as u64;
    let security_data_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(4)?);
    let security_data_update_count =
        row.get::<_, Option<i64>>(5)?.unwrap_or_default().max(0) as u64;
    let metadata_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(6)?);
    let decimals_last_updated = ts_to_datetime(row.get::<_, Option<i64>>(7)?);
    let pool_price_last_calculated = ts_to_datetime(row.get::<_, Option<i64>>(8)?);
    let pool_price_last_used_pool_address: Option<String> = row.get(9)?;
    let last_error: Option<String> = row.get(10)?;
    let last_error_at = ts_to_datetime(row.get::<_, Option<i64>>(11)?);
    let market_error_count = row.get::<_, Option<i64>>(12)?.unwrap_or_default().max(0) as u64;
    let market_error_type: Option<String> = row.get(13)?;
    let _last_security_error: Option<String> = row.get(14)?;
    let _last_security_error_at = ts_to_datetime(row.get::<_, Option<i64>>(15)?);
    let security_error_count = row.get::<_, Option<i64>>(16)?.unwrap_or_default().max(0) as u64;

    Ok(UpdateTrackingInfo {
        mint,
        priority,
        market_data_last_updated_at: market_data_last_updated,
        market_data_update_count,
        security_data_last_updated_at: security_data_last_updated,
        security_data_update_count,
        metadata_last_updated_at: metadata_last_updated,
        decimals_last_updated_at: decimals_last_updated,
        pool_price_last_calculated_at: pool_price_last_calculated,
        pool_price_last_used_pool_address,
        market_error_count,
        market_error_type,
        security_error_count,
        last_error,
        last_error_at,
    })
}

fn ts_to_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.and_then(|value| DateTime::from_timestamp(value, 0))
}
