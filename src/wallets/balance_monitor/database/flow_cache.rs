//! Balance flow cache — tracks SOL inflow/outflow for wallet performance metrics.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::super::types::WalletFlowCacheStats;
use super::WalletDatabase;

impl WalletDatabase {
    /// Aggregate pre-cached SOL flows for a given time window
    pub fn aggregate_cached_flows(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), String> {
        let conn = self.get_connection()?;
        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN sol_delta > 0 THEN sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN sol_delta < 0 THEN -sol_delta ELSE 0 END), 0), \
                COUNT(signature) \
             FROM sol_flow_cache \
             WHERE chain_id = ?1 AND wallet_address = ?2 AND timestamp >= ?3",
        );

        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {e}"))?;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(self.chain.as_str().to_owned()),
            Box::new(wallet_address),
            Box::new(from.to_rfc3339()),
        ];
        if let Some(to_ts) = to {
            query.push_str(&format!(" AND timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare cached flow aggregation query: {e}"))?;
        let (inflow, outflow, count) = stmt
            .query_row(params_refs.as_slice(), |row| {
                let inflow = row.get::<_, Option<f64>>(0)?.unwrap_or_default();
                let outflow = row.get::<_, Option<f64>>(1)?.unwrap_or_default();
                let count = row.get::<_, i64>(2)?.max(0) as usize;
                Ok((inflow, outflow, count))
            })
            .map_err(|e| format!("Failed to aggregate cached SOL flows: {e}"))?;
        Ok((inflow, outflow, count))
    }

    /// Upsert a batch of flow rows into cache
    pub fn upsert_flow_rows(&self, rows: &[(String, DateTime<Utc>, f64)]) -> Result<usize, String> {
        if rows.is_empty() {
            return Ok(0);
        }
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start flow cache transaction: {e}"))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO sol_flow_cache(chain_id, wallet_address, signature, timestamp, sol_delta) VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .map_err(|e| format!("Failed to prepare flow cache upsert: {e}"))?;
            for (sig, ts, delta) in rows.iter() {
                stmt.execute(params![
                    self.chain.as_str(),
                    crate::utils::get_wallet_address()
                        .map_err(|e| format!("Failed to get wallet address: {e}"))?,
                    sig,
                    ts.to_rfc3339(),
                    *delta
                ])
                .map_err(|e| format!("Failed to upsert flow row: {e}"))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit flow cache upserts: {e}"))?;
        Ok(rows.len())
    }

    /// Get the max timestamp present in the flow cache
    pub fn get_flow_cache_max_ts(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MAX(timestamp) FROM sol_flow_cache WHERE chain_id = ?1 AND wallet_address = ?2")
            .map_err(|e| format!("Failed to prepare max timestamp query: {e}"))?;
        let ts: Option<String> = stmt
            .query_row(
                params![
                    self.chain.as_str(),
                    crate::utils::get_wallet_address()
                        .map_err(|e| format!("Failed to get wallet address: {e}"))?
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query max timestamp: {e}"))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached max timestamp: {e}"))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get the minimum timestamp present in the flow cache (earliest record)
    pub fn get_flow_cache_min_ts(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MIN(timestamp) FROM sol_flow_cache WHERE chain_id = ?1 AND wallet_address = ?2")
            .map_err(|e| format!("Failed to prepare min timestamp query: {e}"))?;
        let ts: Option<String> = stmt
            .query_row(
                params![
                    self.chain.as_str(),
                    crate::utils::get_wallet_address()
                        .map_err(|e| format!("Failed to get wallet address: {e}"))?
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query min timestamp: {e}"))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached min timestamp: {e}"))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get flow cache stats (row count and latest timestamp)
    pub fn get_flow_cache_stats(&self) -> Result<WalletFlowCacheStats, String> {
        let conn = self.get_connection()?;
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sol_flow_cache WHERE chain_id = ?1 AND wallet_address = ?2",
                params![
                    self.chain.as_str(),
                    crate::utils::get_wallet_address()
                        .map_err(|e| format!("Failed to get wallet address: {e}"))?
                ],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let max_ts = self.get_flow_cache_max_ts()?.map(|dt| dt.to_rfc3339());
        Ok(WalletFlowCacheStats {
            rows: rows.max(0) as u64,
            max_timestamp: max_ts,
        })
    }
}
