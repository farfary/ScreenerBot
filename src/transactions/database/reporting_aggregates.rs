//! Aggregate and export queries for SOL flow reporting.
//
// Split from reporting.rs — contains aggregate_sol_flows_since,
// aggregate_daily_flows, and export_processed_for_wallet_flow.

use chrono::{DateTime, Utc};
use rusqlite::params;
use std::collections::HashMap;

use crate::logger::{self, LogTag};

use crate::transactions::error::Error;

use super::operations::TransactionDatabase;
use super::types::WalletFlowExportRow;

impl TransactionDatabase {
    /// Aggregate SOL inflow/outflow metrics within a time window for wallet dashboard usage
    pub async fn aggregate_sol_flows_since(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), Error> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        // Check if this is "all time" query (from epoch = no time filter)
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let is_all_time = from == epoch;

        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) > 0 THEN p.sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) < 0 THEN -p.sol_delta ELSE 0 END), 0), \
                COUNT(r.signature) \
             FROM raw_transactions r \
             LEFT JOIN processed_transactions p ON r.chain_id = p.chain_id AND r.signature = p.signature AND p.wallet_address = ?2 \
             WHERE r.chain_id = ?1 AND r.wallet_address = ?2 AND r.status IN ('Confirmed', 'Finalized')",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        params_vec.push(Box::new(self.chain.as_str().to_owned()));
        params_vec.push(Box::new(wallet_address.clone()));

        // Only add timestamp filter if NOT all-time query
        if !is_all_time {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to_ts) = to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|value| value.as_ref()).collect();

        logger::debug(
            LogTag::Transactions,
            &format!(
                "Aggregating SOL flows for wallet {} from {}",
                wallet_address,
                from.to_rfc3339()
            ),
        );

        // Change query to get all rows so we can parse JSON
        let row_query = query.replace(
            "SELECT \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) > 0 THEN p.sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) < 0 THEN -p.sol_delta ELSE 0 END), 0), \
                COUNT(r.signature)",
            "SELECT r.signature, r.timestamp, p.sol_balance_change",
        );

        let mut stmt = conn
            .prepare(&row_query)
            .map_err(crate::errors::DatabaseError::from)?;

        let mut rows = stmt
            .query(params_refs.as_slice())
            .map_err(crate::errors::DatabaseError::from)?;

        let mut inflow = 0.0;
        let mut outflow = 0.0;
        let mut count = 0;
        let mut parsed_count = 0;
        let mut no_json_count = 0;
        let mut parse_error_count = 0;
        let mut no_wallet_account_count = 0;

        while let Some(row) = rows.next().map_err(crate::errors::DatabaseError::from)? {
            count += 1;
            let signature: String = row.get(0).unwrap_or_default();
            let sol_balance_change_json: Option<String> = row.get(2).ok();

            if let Some(json_str) = sol_balance_change_json {
                // Parse JSON array of balance changes
                match serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    Ok(changes) => {
                        let mut found_wallet = false;
                        let changes_len = changes.len();
                        for change_obj in &changes {
                            if let Some(account) =
                                change_obj.get("account").and_then(|v| v.as_str())
                            {
                                if account == wallet_address {
                                    found_wallet = true;
                                    if let Some(change) =
                                        change_obj.get("change").and_then(|v| v.as_f64())
                                    {
                                        parsed_count += 1;
                                        if count <= 5 {
                                            logger::debug(
                                                LogTag::Transactions,
                                                &format!(
                                                    "TX {}: wallet change={:.6} SOL",
                                                    &signature[..8],
                                                    change
                                                ),
                                            );
                                        }
                                        if change > 0.0 {
                                            inflow += change;
                                        } else if change < 0.0 {
                                            outflow += change.abs();
                                        }
                                    }
                                    break; // Found wallet, no need to check other accounts
                                }
                            }
                        }
                        if !found_wallet {
                            no_wallet_account_count += 1;
                            if no_wallet_account_count <= 3 {
                                logger::debug(
                                    LogTag::Transactions,
                                    &format!(
                                        "TX {}: no wallet account in {} balance changes",
                                        &signature[..8],
                                        changes_len
                                    ),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        parse_error_count += 1;
                        if parse_error_count <= 3 {
                            logger::debug(
                                LogTag::Transactions,
                                &format!("TX {}: JSON parse error: {}", &signature[..8], e),
                            );
                        }
                    }
                }
            } else {
                no_json_count += 1;
            }
        }

        logger::debug(
        LogTag::Transactions,
                &format!(
                    "Aggregated {} txs: parsed={} with wallet, no_json={}, parse_errors={}, no_wallet_account={} | inflow={:.6} SOL, outflow={:.6} SOL, net={:.6} SOL",
                    count,
                    parsed_count,
                    no_json_count,
                    parse_error_count,
                    no_wallet_account_count,
                    inflow,
                    outflow,
                    inflow - outflow
                ),
            );

        Ok((inflow, outflow, count))
    }

    /// Get daily flow aggregation for time-series chart
    pub async fn aggregate_daily_flows(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, f64, f64, usize)>, Error> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        // Check if this is "all time" query
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let is_all_time = from == epoch;

        // Query to get daily aggregated flows
        let mut query = String::from(
            "SELECT \
                DATE(r.timestamp) as day, \
                r.signature, \
                p.sol_balance_change \
             FROM raw_transactions r \
             LEFT JOIN processed_transactions p ON r.chain_id = p.chain_id AND r.signature = p.signature AND p.wallet_address = ?2 \
             WHERE r.chain_id = ?1 AND r.wallet_address = ?2 AND r.status IN ('Confirmed', 'Finalized')",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        params_vec.push(Box::new(self.chain.as_str().to_owned()));
        params_vec.push(Box::new(wallet_address.clone()));

        if !is_all_time {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to_ts) = to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }

        query.push_str(" ORDER BY day ASC, r.timestamp ASC");

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|value| value.as_ref()).collect();

        let mut stmt = conn
            .prepare(&query)
            .map_err(crate::errors::DatabaseError::from)?;

        let mut rows = stmt
            .query(params_refs.as_slice())
            .map_err(crate::errors::DatabaseError::from)?;

        // Group by day manually
        let mut daily_data: HashMap<String, (f64, f64, usize)> = HashMap::new();

        while let Some(row) = rows.next().map_err(crate::errors::DatabaseError::from)? {
            let day: String = row.get(0).unwrap_or_default();
            let sol_balance_change_json: Option<String> = row.get(2).ok();

            if let Some(json_str) = sol_balance_change_json {
                if let Ok(changes) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    for change_obj in &changes {
                        if let Some(account) = change_obj.get("account").and_then(|v| v.as_str()) {
                            if account == wallet_address {
                                if let Some(change) =
                                    change_obj.get("change").and_then(|v| v.as_f64())
                                {
                                    let entry =
                                        daily_data.entry(day.clone()).or_insert((0.0, 0.0, 0));
                                    if change > 0.0 {
                                        entry.0 += change; // inflow
                                    } else if change < 0.0 {
                                        entry.1 += change.abs(); // outflow
                                    }
                                    entry.2 += 1; // tx count
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Convert to sorted vec
        let mut result: Vec<(String, f64, f64, usize)> = daily_data
            .into_iter()
            .map(|(day, (inflow, outflow, count))| (day, inflow, outflow, count))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(result)
    }

    /// Lightweight export of processed transactions for wallet flow cache
    pub async fn export_processed_for_wallet_flow(
        &self,
        from: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<WalletFlowExportRow>, Error> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let mut stmt = conn
            .prepare(
                "SELECT r.signature, r.timestamp, COALESCE(p.sol_delta, 0) as sol_delta \
                 FROM raw_transactions r \
                 LEFT JOIN processed_transactions p ON r.chain_id = p.chain_id AND r.signature = p.signature AND p.wallet_address = ?2 \
                 WHERE r.chain_id = ?1 AND r.wallet_address = ?2 AND r.timestamp >= ?3 AND r.status IN ('Confirmed', 'Finalized') \
                 ORDER BY r.timestamp ASC, r.signature ASC \
                 LIMIT ?4",
            )
            .map_err(crate::errors::DatabaseError::from)?;

        let mut rows = stmt
            .query(params![
                self.chain.as_str(),
                wallet_address,
                from.to_rfc3339(),
                (limit as i64).max(1)
            ])
            .map_err(crate::errors::DatabaseError::from)?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().map_err(crate::errors::DatabaseError::from)? {
            let signature: String = row.get(0).map_err(|e| Error::RowDecode {
                column: "signature",
                detail: e.to_string(),
            })?;
            let ts_str: String = row.get(1).map_err(|e| Error::RowDecode {
                column: "timestamp",
                detail: e.to_string(),
            })?;
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| Error::RowDecode {
                    column: "timestamp",
                    detail: e.to_string(),
                })?;
            let sol_delta: f64 = row
                .get::<_, Option<f64>>(2)
                .unwrap_or(Some(0.0))
                .unwrap_or_default();
            results.push(WalletFlowExportRow {
                signature,
                timestamp,
                sol_delta,
            });
        }

        Ok(results)
    }
}
