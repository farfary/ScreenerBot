//! Paged activity reads, target observations and paper-book resets for the task
//! workspace.

use chrono::{DateTime, Utc};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter};
use serde::Serialize;

use crate::database::WriteTransaction;
use crate::trader::copy::types::CopyActivityRow;
use crate::trader::error::Error;

use super::rows::{json_error, parse_datetime};
use super::CopyDatabase;

/// Activity kinds a paper reset removes: everything the paper book was built
/// from plus the skips of the same evaluation.
const PAPER_KINDS: &str = "'paper_filled', 'paper_sell_observed', 'skipped'";

/// One page of decisions, newest first. Every filter is optional.
#[derive(Debug, Clone, Default)]
pub struct ActivityQuery {
    pub task_id: Option<i64>,
    /// Keyset cursor: only rows older than this id.
    pub before_id: Option<i64>,
    pub kinds: Vec<&'static str>,
    pub mint: Option<String>,
    pub limit: usize,
}

/// What the copy service has seen a target wallet do, across every task that
/// watches it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TargetObservations {
    pub swaps: u64,
    pub buys: u64,
    pub sells: u64,
    pub tokens: u64,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
}

impl CopyDatabase {
    pub async fn activity_page(
        &self,
        query: ActivityQuery,
    ) -> crate::trader::Result<Vec<CopyActivityRow>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.activity_page_sync(&query))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn activity_page_sync(
        &self,
        query: &ActivityQuery,
    ) -> crate::trader::Result<Vec<CopyActivityRow>> {
        let mut clauses = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        if let Some(task_id) = query.task_id {
            values.push(Value::Integer(task_id));
            clauses.push(format!("task_id = ?{}", values.len()));
        }
        if let Some(before_id) = query.before_id {
            values.push(Value::Integer(before_id));
            clauses.push(format!("id < ?{}", values.len()));
        }
        if !query.kinds.is_empty() {
            let mut marks = Vec::with_capacity(query.kinds.len());
            for kind in &query.kinds {
                values.push(Value::Text((*kind).to_owned()));
                marks.push(format!("?{}", values.len()));
            }
            clauses.push(format!("kind IN ({})", marks.join(", ")));
        }
        if let Some(mint) = &query.mint {
            values.push(Value::Text(mint.clone()));
            clauses.push(format!(
                "json_extract(details_json, '$.mint') = ?{}",
                values.len()
            ));
        }
        values.push(Value::Integer(query.limit.clamp(1, 1_000) as i64));
        let filter = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT id, task_id, kind, details_json, created_at FROM copy_activity \
                 {filter} ORDER BY id DESC LIMIT ?{}",
                values.len()
            ))
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                let json: String = row.get(3)?;
                let created: String = row.get(4)?;
                Ok(CopyActivityRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    kind: row.get(2)?,
                    outcome: serde_json::from_str(&json).map_err(json_error)?,
                    created_at: parse_datetime(&created, 4)?,
                })
            })
            .map_err(crate::errors::DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::errors::DatabaseError::from)?;
        Ok(rows)
    }

    /// Distinct target swaps recorded for these tasks' inventory tracking.
    pub async fn target_observations(
        &self,
        task_ids: Vec<i64>,
    ) -> crate::trader::Result<TargetObservations> {
        if task_ids.is_empty() {
            return Ok(TargetObservations::default());
        }
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let marks = (1..=task_ids.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            let connection = db.connection()?;
            let (swaps, buys, sells, tokens, first, last) = connection
                .query_row(
                    &format!(
                        "SELECT COUNT(DISTINCT signature), \
                         COUNT(DISTINCT CASE WHEN token_delta > 0 THEN signature END), \
                         COUNT(DISTINCT CASE WHEN token_delta < 0 THEN signature END), \
                         COUNT(DISTINCT mint), MIN(observed_at), MAX(observed_at) \
                         FROM copy_target_events WHERE task_id IN ({marks})"
                    ),
                    params_from_iter(task_ids),
                    |row| {
                        Ok((
                            row.get::<_, u64>(0)?,
                            row.get::<_, u64>(1)?,
                            row.get::<_, u64>(2)?,
                            row.get::<_, u64>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    },
                )
                .map_err(crate::errors::DatabaseError::from)?;
            let time = |value: Option<String>, index| {
                value
                    .map(|value| parse_datetime(&value, index))
                    .transpose()
                    .map_err(crate::errors::DatabaseError::from)
            };
            Ok(TargetObservations {
                swaps,
                buys,
                sells,
                tokens,
                first_seen: time(first, 4)?,
                last_seen: time(last, 5)?,
            })
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Start a task's paper evaluation over: its paper book, paper spend and the
    /// paper decisions and skips that built them are removed in one transaction.
    /// Live decisions, live spend and the target inventory are kept. Returns the
    /// number of activity rows removed.
    pub async fn reset_paper_book(&self, task_id: i64) -> crate::trader::Result<usize> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = db.connection()?;
            let transaction = connection
                .write_tx()
                .map_err(crate::errors::DatabaseError::from)?;
            let execute = |sql: &str| {
                transaction
                    .execute(sql, params![task_id])
                    .map_err(crate::errors::DatabaseError::from)
            };
            execute("DELETE FROM copy_paper_positions WHERE task_id = ?1")?;
            execute("DELETE FROM copy_spend WHERE task_id = ?1 AND mode = 'paper'")?;
            execute(&format!(
                "DELETE FROM copy_decisions WHERE task_id = ?1 \
                 AND json_extract(outcome_json, '$.outcome') IN ({PAPER_KINDS})"
            ))?;
            let removed = execute(&format!(
                "DELETE FROM copy_activity WHERE task_id = ?1 AND kind IN ({PAPER_KINDS})"
            ))?;
            transaction
                .execute(
                    "UPDATE copy_tasks SET updated_at = ?2 WHERE id = ?1",
                    params![task_id, Utc::now().to_rfc3339()],
                )
                .map_err(crate::errors::DatabaseError::from)?;
            transaction
                .commit()
                .map_err(crate::errors::DatabaseError::from)?;
            Ok(removed)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }
}
