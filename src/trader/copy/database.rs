//! SQLite repository for copy tasks, spend, outcomes, activity, and position links.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::Utc;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};

use crate::database;
use crate::trader::error::Error;

use super::types::{
    confirm_mode_transition, CopyActivityRow, CopyMode, CopyOutcome, CopySkip, CopyTask, SpendState,
};

#[path = "database/rows.rs"]
mod rows;
#[path = "database/schema.rs"]
mod schema;

use crate::database::WriteTransaction;
use rows::{json_error, parse_datetime, row_to_task};
use schema::{migrate, SCHEMA, SCHEMA_VERSION};

#[derive(Clone)]
pub struct CopyDatabase {
    pool: Pool<SqliteConnectionManager>,
    chain: crate::chains::ChainId,
}

static SHARED_COPY_DATABASE: OnceLock<CopyDatabase> = OnceLock::new();

impl CopyDatabase {
    pub fn new(chain: crate::chains::ChainId) -> crate::trader::Result<Self> {
        Self::open(crate::paths::get_copy_trading_db_path(), chain)
    }

    /// Process-wide pool for runtime consumers. Exit evaluation runs every few
    /// seconds, so opening a new r2d2 pool for each policy lookup would churn WAL
    /// connections and defeat the centralized connection configuration.
    pub fn shared(chain: crate::chains::ChainId) -> crate::trader::Result<Self> {
        if let Some(database) = SHARED_COPY_DATABASE.get() {
            return (database.chain == chain)
                .then(|| database.clone())
                .ok_or_else(|| Error::CopyValidation {
                    detail: "already bound to another chain".to_owned(),
                });
        }
        let database = Self::new(chain)?;
        let _ = SHARED_COPY_DATABASE.set(database);
        let database =
            SHARED_COPY_DATABASE
                .get()
                .cloned()
                .ok_or_else(|| Error::CopyDatabaseUnavailable {
                    detail: "failed to initialize shared copy database".to_owned(),
                })?;
        (database.chain == chain)
            .then_some(database)
            .ok_or_else(|| Error::CopyValidation {
                detail: "initialized for another chain".to_owned(),
            })
    }

    /// Explicit-path constructor for isolated tests and offline tools.
    /// Opens one chain-bound repository against the shared copy-trading database.
    pub fn open(
        path: impl AsRef<Path>,
        chain: crate::chains::ChainId,
    ) -> crate::trader::Result<Self> {
        let path = PathBuf::from(path.as_ref());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::CopyDatabaseUnavailable {
                detail: format!("failed to create copy database directory: {e}"),
            })?;
        }
        let manager = SqliteConnectionManager::file(path).with_init(|connection| {
            database::configure_connection(connection, database::COPY_TRADING_DB)
        });
        let pool = Pool::builder()
            .max_size(3)
            .idle_timeout(None)
            .max_lifetime(None)
            .build(manager)
            .map_err(crate::errors::DatabaseError::from)?;
        let db = Self { pool, chain };
        db.initialize()?;
        Ok(db)
    }

    fn connection(&self) -> crate::trader::Result<PooledConnection<SqliteConnectionManager>> {
        Ok(self
            .pool
            .get()
            .map_err(crate::errors::DatabaseError::from)?)
    }

    fn initialize(&self) -> crate::trader::Result<()> {
        let connection = self.connection()?;
        connection
            .execute_batch(SCHEMA)
            .map_err(crate::errors::DatabaseError::from)?;
        migrate(&connection)?;
        connection
            .execute(
                "INSERT INTO copy_metadata (key, value) VALUES ('schema_version', ?1) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![SCHEMA_VERSION.to_string()],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        Ok(())
    }

    pub async fn insert_task(&self, task: CopyTask) -> crate::trader::Result<CopyTask> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.insert_task_sync(task))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn insert_task_sync(&self, mut task: CopyTask) -> crate::trader::Result<CopyTask> {
        if task.chain != self.chain {
            return Err(Error::CopyValidation {
                detail: "task chain does not match this repository".to_owned(),
            });
        }
        if task.mode != CopyMode::Paper {
            return Err(Error::CopyValidation {
                detail:
                    "new copy tasks must start in paper mode; use the confirmed mode transition"
                        .to_owned(),
            });
        }
        let connection = self.connection()?;
        let mode = serde_json::to_string(&task.mode).map_err(|e| Error::CopySerialize {
            field: "mode",
            detail: e.to_string(),
        })?;
        let sizing = serde_json::to_string(&task.sizing).map_err(|e| Error::CopySerialize {
            field: "sizing",
            detail: e.to_string(),
        })?;
        let exit_mode =
            serde_json::to_string(&task.exit_mode).map_err(|e| Error::CopySerialize {
                field: "exit_mode",
                detail: e.to_string(),
            })?;
        let exit_policy = serde_json::to_string(&task.exit_policy_overrides).map_err(|e| {
            Error::CopySerialize {
                field: "exit_policy",
                detail: e.to_string(),
            }
        })?;
        connection
            .execute(
                "INSERT INTO copy_tasks (chain_id, target_address, label, enabled, mode_json, sizing_json, \
                 exit_mode_json, exit_policy_json, max_sol_per_trade, max_sol_per_token, total_budget_sol, \
                 min_target_trade_sol, max_target_trade_sol, buy_once_per_token, slippage_pct, \
                 created_at, updated_at) VALUES \
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    self.chain.as_str(), task.target_address,
                    task.label,
                    task.enabled,
                    mode,
                    sizing,
                    exit_mode,
                    exit_policy,
                    task.max_sol_per_trade,
                    task.max_sol_per_token,
                    task.total_budget_sol,
                    task.min_target_trade_sol,
                    task.max_target_trade_sol,
                    task.buy_once_per_token,
                    task.slippage_pct,
                    task.created_at.to_rfc3339(),
                    task.updated_at.to_rfc3339(),
                ],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        task.id = connection.last_insert_rowid();
        Ok(task)
    }

    pub async fn enabled_tasks_for_subject(
        &self,
        target_address: &str,
    ) -> crate::trader::Result<Vec<CopyTask>> {
        let db = self.clone();
        let address = target_address.to_owned();
        tokio::task::spawn_blocking(move || db.enabled_tasks_for_subject_sync(&address))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    pub async fn list_tasks(&self) -> crate::trader::Result<Vec<CopyTask>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.list_tasks_sync())
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn list_tasks_sync(&self) -> crate::trader::Result<Vec<CopyTask>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, chain_id, target_address, label, enabled, mode_json, sizing_json, exit_mode_json, \
                 exit_policy_json, max_sol_per_trade, max_sol_per_token, total_budget_sol, min_target_trade_sol, \
                 max_target_trade_sol, buy_once_per_token, slippage_pct, created_at, updated_at \
                 FROM copy_tasks WHERE chain_id=?1 ORDER BY id DESC",
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map(params![self.chain.as_str()], row_to_task)
            .map_err(crate::errors::DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::errors::DatabaseError::from)?;
        Ok(rows)
    }

    pub async fn get_task(&self, id: i64) -> crate::trader::Result<Option<CopyTask>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.get_task_sync(id))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn get_task_sync(&self, id: i64) -> crate::trader::Result<Option<CopyTask>> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, chain_id, target_address, label, enabled, mode_json, sizing_json, exit_mode_json, \
                 exit_policy_json, max_sol_per_trade, max_sol_per_token, total_budget_sol, min_target_trade_sol, \
                 max_target_trade_sol, buy_once_per_token, slippage_pct, created_at, updated_at \
                 FROM copy_tasks WHERE id = ?1 AND chain_id=?2",
                params![id, self.chain.as_str()],
                row_to_task,
            )
            .optional()
            .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
    }

    pub async fn update_task(&self, task: CopyTask) -> crate::trader::Result<CopyTask> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.update_task_sync(task))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn update_task_sync(&self, mut task: CopyTask) -> crate::trader::Result<CopyTask> {
        let connection = self.connection()?;
        let (current_mode, current_address): (String, String) = connection
            .query_row(
                "SELECT mode_json, target_address FROM copy_tasks WHERE id=?1 AND chain_id=?2",
                params![task.id, self.chain.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?
            .ok_or_else(|| Error::CopyTaskNotFound { task_id: task.id })?;
        let current_mode: CopyMode =
            serde_json::from_str(&current_mode).map_err(|e| Error::CopyTaskDecode {
                task_id: task.id,
                field: "mode",
                detail: e.to_string(),
            })?;
        if task.mode != current_mode {
            return Err(Error::CopyValidation {
                detail: "mode changes require the dedicated confirmation endpoint".to_owned(),
            });
        }
        task.updated_at = Utc::now();
        let affected = connection
            .execute(
                "UPDATE copy_tasks SET target_address=?1, label=?2, enabled=?3, mode_json=?4, \
                 sizing_json=?5, exit_mode_json=?6, exit_policy_json=?7, max_sol_per_trade=?8, max_sol_per_token=?9, \
                 total_budget_sol=?10, min_target_trade_sol=?11, max_target_trade_sol=?12, \
                 buy_once_per_token=?13, slippage_pct=?14, updated_at=?15 WHERE id=?16 AND chain_id=?17",
                params![
                    task.target_address,
                    task.label,
                    task.enabled,
                    serde_json::to_string(&task.mode)
                        .map_err(|e| Error::CopySerialize { field: "mode", detail: e.to_string() })?,
                    serde_json::to_string(&task.sizing)
                        .map_err(|e| Error::CopySerialize { field: "sizing", detail: e.to_string() })?,
                    serde_json::to_string(&task.exit_mode)
                        .map_err(|e| Error::CopySerialize { field: "exit_mode", detail: e.to_string() })?,
                    serde_json::to_string(&task.exit_policy_overrides)
                        .map_err(|e| Error::CopySerialize { field: "exit_policy", detail: e.to_string() })?,
                    task.max_sol_per_trade,
                    task.max_sol_per_token,
                    task.total_budget_sol,
                    task.min_target_trade_sol,
                    task.max_target_trade_sol,
                    task.buy_once_per_token,
                    task.slippage_pct,
                    task.updated_at.to_rfc3339(),
                    task.id, self.chain.as_str(),
                ],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        if affected == 0 {
            return Err(Error::CopyTaskNotFound { task_id: task.id });
        }
        if current_address != task.target_address {
            connection
                .execute(
                    "DELETE FROM copy_target_events WHERE task_id=?1",
                    params![task.id],
                )
                .map_err(crate::errors::DatabaseError::from)?;
            connection
                .execute(
                    "DELETE FROM copy_target_holdings WHERE task_id=?1",
                    params![task.id],
                )
                .map_err(crate::errors::DatabaseError::from)?;
        }
        Ok(task)
    }

    pub async fn set_task_mode(
        &self,
        id: i64,
        mode: CopyMode,
        confirmation: Option<String>,
    ) -> crate::trader::Result<CopyTask> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.set_task_mode_sync(id, mode, confirmation.as_deref())
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    fn set_task_mode_sync(
        &self,
        id: i64,
        requested: CopyMode,
        confirmation: Option<&str>,
    ) -> crate::trader::Result<CopyTask> {
        let connection = self.connection()?;
        let current_json: String = connection
            .query_row(
                "SELECT mode_json FROM copy_tasks WHERE id=?1 AND chain_id=?2",
                params![id, self.chain.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?
            .ok_or_else(|| Error::CopyTaskNotFound { task_id: id })?;
        let current: CopyMode =
            serde_json::from_str(&current_json).map_err(|e| Error::CopyTaskDecode {
                task_id: id,
                field: "mode",
                detail: e.to_string(),
            })?;
        let mode = confirm_mode_transition(current, requested, confirmation).map_err(|reason| {
            Error::CopyValidation {
                detail: format!("mode transition rejected: {reason:?}"),
            }
        })?;
        let affected = connection
            .execute(
                "UPDATE copy_tasks SET mode_json=?2, updated_at=?3 WHERE id=?1 AND chain_id=?4",
                params![
                    id,
                    serde_json::to_string(&mode).map_err(|e| Error::CopySerialize {
                        field: "mode",
                        detail: e.to_string()
                    })?,
                    Utc::now().to_rfc3339(),
                    self.chain.as_str()
                ],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        if affected == 0 {
            return Err(Error::CopyTaskNotFound { task_id: id });
        }
        drop(connection);
        self.get_task_sync(id)?
            .ok_or_else(|| Error::CopyTaskNotFound { task_id: id })
    }

    pub async fn delete_task(&self, id: i64) -> crate::trader::Result<bool> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .execute(
                    "DELETE FROM copy_tasks WHERE id = ?1 AND chain_id=?2",
                    params![id, db.chain.as_str()],
                )
                .map(|affected| affected > 0)
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    pub async fn pause_task(&self, id: i64) -> crate::trader::Result<bool> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .execute(
                    "UPDATE copy_tasks SET enabled=0, updated_at=?2 WHERE id=?1 AND chain_id=?3 AND enabled=1",
                    params![id, Utc::now().to_rfc3339(), db.chain.as_str()],
                )
                .map(|affected| affected > 0)
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable { detail: e.to_string() })?
    }

    pub async fn list_activity(&self, limit: usize) -> crate::trader::Result<Vec<CopyActivityRow>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.list_activity_sync(limit.clamp(1, 1_000)))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    pub async fn list_task_activity(
        &self,
        task_id: i64,
        limit: usize,
    ) -> crate::trader::Result<Vec<CopyActivityRow>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.list_task_activity_sync(task_id, limit.clamp(1, 10_000))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    pub async fn list_unconfirmed_live_entries(
        &self,
    ) -> crate::trader::Result<Vec<super::types::LiveDecision>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            let mut statement = connection
                .prepare("SELECT outcome_json FROM copy_decisions ORDER BY id")
                .map_err(crate::errors::DatabaseError::from)?;
            let outcomes = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(crate::errors::DatabaseError::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(crate::errors::DatabaseError::from)?;
            Ok(outcomes
                .into_iter()
                .filter_map(|json| serde_json::from_str::<CopyOutcome>(&json).ok())
                .filter_map(|outcome| match outcome {
                    CopyOutcome::LiveSubmitted(decision) => Some(decision),
                    _ => None,
                })
                .collect())
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    fn list_task_activity_sync(
        &self,
        task_id: i64,
        limit: usize,
    ) -> crate::trader::Result<Vec<CopyActivityRow>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, task_id, kind, details_json, created_at FROM copy_activity \
                 WHERE task_id=?1 ORDER BY id DESC LIMIT ?2",
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map(params![task_id, limit], |row| {
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

    fn list_activity_sync(&self, limit: usize) -> crate::trader::Result<Vec<CopyActivityRow>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, task_id, kind, details_json, created_at FROM copy_activity \
                 ORDER BY id DESC LIMIT ?1",
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map(params![limit], |row| {
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

    fn enabled_tasks_for_subject_sync(
        &self,
        address: &str,
    ) -> crate::trader::Result<Vec<CopyTask>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, chain_id, target_address, label, enabled, mode_json, sizing_json, exit_mode_json, \
                 exit_policy_json, max_sol_per_trade, max_sol_per_token, total_budget_sol, min_target_trade_sol, \
                 max_target_trade_sol, buy_once_per_token, slippage_pct, created_at, updated_at \
                 FROM copy_tasks WHERE target_address = ?1 AND chain_id=?2 AND enabled = 1 ORDER BY id",
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map(params![address, self.chain.as_str()], row_to_task)
            .map_err(crate::errors::DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::errors::DatabaseError::from)?;
        Ok(rows)
    }

    pub async fn spend_state(&self, task_id: i64, mint: &str) -> crate::trader::Result<SpendState> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || db.spend_state_sync(task_id, &mint))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    pub async fn task_total_spent(&self, task_id: i64) -> crate::trader::Result<f64> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT COALESCE(SUM(spent_sol), 0) FROM copy_spend WHERE task_id = ?1",
                    params![task_id],
                    |row| row.get(0),
                )
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Atomically claim a target activity before any live admission/submission work.
    /// A false result means another delivery already owns it and must never spend again.
    pub async fn claim_live_activity(
        &self,
        task_id: i64,
        signature: &str,
    ) -> crate::trader::Result<bool> {
        let db = self.clone();
        let signature = signature.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .execute(
                    "INSERT INTO copy_live_claims (task_id, signature, claimed_at, state, updated_at) VALUES (?1, ?2, ?3, 'claimed', ?3) ON CONFLICT(task_id, signature) DO NOTHING",
                    params![task_id, signature, Utc::now().to_rfc3339()],
                )
                .map(|affected| affected > 0)
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable { detail: e.to_string() })?
    }

    pub async fn target_holding(&self, task_id: i64, mint: &str) -> crate::trader::Result<f64> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT token_amount FROM copy_target_holdings WHERE task_id=?1 AND mint=?2",
                    params![task_id, mint],
                    |row| row.get(0),
                )
                .optional()
                .map(|value| value.unwrap_or_default())
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Idempotently apply one observed target token delta and return the holding
    /// immediately before it. This is observation state, independent of whether our
    /// copy decision fills, skips, or fails.
    pub async fn observe_target_inventory(
        &self,
        task_id: i64,
        signature: &str,
        mint: &str,
        token_delta: f64,
    ) -> crate::trader::Result<f64> {
        let db = self.clone();
        let signature = signature.to_owned();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut connection = db.connection()?;
            let transaction = connection.write_tx().map_err(|e| Error::CopyReconciliation { detail: format!("failed to begin target inventory update: {e}") })?;
            let before = transaction.query_row(
                "SELECT token_amount FROM copy_target_holdings WHERE task_id=?1 AND mint=?2",
                params![task_id, mint],
                |row| row.get::<_, f64>(0),
            ).optional().map_err(|e| Error::CopyReconciliation { detail: format!("failed to read target inventory: {e}") })?.unwrap_or_default();
            let inserted = transaction.execute(
                "INSERT INTO copy_target_events (task_id, signature, mint, token_delta, observed_at) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(task_id, signature) DO NOTHING",
                params![task_id, signature, mint, token_delta, Utc::now().to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to record target inventory event: {e}") })? > 0;
            if inserted {
                update_target_holding(&transaction, task_id, &mint, token_delta)?;
            }
            transaction.commit().map_err(|e| Error::CopyReconciliation { detail: format!("failed to commit target inventory update: {e}") })?;
            Ok(before)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable { detail: e.to_string() })?
    }

    /// Fail closed after a crash: stale claims are marked abandoned and surfaced as
    /// activity, but are never made spendable again because submission may have happened.
    pub async fn reconcile_stale_claims(&self, grace_seconds: u64) -> crate::trader::Result<usize> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.reconcile_stale_claims_sync(grace_seconds))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn reconcile_stale_claims_sync(&self, grace_seconds: u64) -> crate::trader::Result<usize> {
        let mut connection = self.connection()?;
        let transaction = connection
            .write_tx()
            .map_err(|e| Error::CopyReconciliation {
                detail: format!("failed to begin claim reconciliation: {e}"),
            })?;
        transaction.execute(
            "UPDATE copy_live_claims SET state='settled', updated_at=?1 WHERE state='claimed' AND EXISTS (SELECT 1 FROM copy_decisions d WHERE d.task_id=copy_live_claims.task_id AND d.signature=copy_live_claims.signature)",
            params![Utc::now().to_rfc3339()],
        ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to settle legacy copy claims: {e}") })?;
        let cutoff =
            Utc::now() - chrono::Duration::seconds(grace_seconds.min(i64::MAX as u64) as i64);
        let stale = {
            let mut statement = transaction
                .prepare("SELECT task_id, signature FROM copy_live_claims WHERE state='claimed' AND datetime(claimed_at) <= datetime(?1)")
                .map_err(|e| Error::CopyReconciliation { detail: format!("failed to prepare stale claim query: {e}") })?;
            let rows = statement
                .query_map(params![cutoff.to_rfc3339()], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| Error::CopyReconciliation {
                    detail: format!("failed to query stale claims: {e}"),
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::CopyReconciliation {
                    detail: format!("failed to decode stale claims: {e}"),
                })?;
            rows
        };
        for (task_id, signature) in &stale {
            let now = Utc::now();
            let outcome = CopyOutcome::Skipped {
                task_id: *task_id,
                signature: signature.clone(),
                mint: None,
                reason: CopySkip::ClaimReconciledAbandoned,
                decided_at: now,
                telemetry: None,
            };
            let json = serde_json::to_string(&outcome).map_err(|e| Error::CopySerialize {
                field: "reconciled_claim",
                detail: e.to_string(),
            })?;
            transaction.execute(
                "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) VALUES (?1, ?2, NULL, ?3, ?4) ON CONFLICT(task_id, signature) DO NOTHING",
                params![task_id, signature, json, now.to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to record reconciled claim decision: {e}") })?;
            transaction.execute(
                "INSERT INTO copy_activity (task_id, kind, details_json, created_at) VALUES (?1, 'claim_abandoned', ?2, ?3)",
                params![task_id, json, now.to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to record reconciled claim activity: {e}") })?;
            transaction.execute(
                "UPDATE copy_live_claims SET state='abandoned', updated_at=?3 WHERE task_id=?1 AND signature=?2",
                params![task_id, signature, now.to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to abandon stale claim: {e}") })?;
        }
        transaction
            .commit()
            .map_err(|e| Error::CopyReconciliation {
                detail: format!("failed to commit claim reconciliation: {e}"),
            })?;
        Ok(stale.len())
    }

    fn spend_state_sync(&self, task_id: i64, mint: &str) -> crate::trader::Result<SpendState> {
        let connection = self.connection()?;
        let total_spent_sol: f64 = connection
            .query_row(
                "SELECT COALESCE(SUM(spent_sol), 0) FROM copy_spend WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let token = connection
            .query_row(
                "SELECT spent_sol, buy_count FROM copy_spend WHERE task_id = ?1 AND mint = ?2",
                params![task_id, mint],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, u64>(1)?)),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?
            .unwrap_or_default();
        Ok(SpendState {
            total_spent_sol,
            token_spent_sol: token.0,
            token_buy_count: token.1,
        })
    }

    /// Persist one outcome and commit spend atomically once money may have left.
    pub async fn record_outcome(&self, outcome: CopyOutcome) -> crate::trader::Result<()> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.record_outcome_sync(outcome))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn record_outcome_sync(&self, outcome: CopyOutcome) -> crate::trader::Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection
            .write_tx()
            .map_err(crate::errors::DatabaseError::from)?;
        let (task_id, signature, mint, decided_at, kind) = match &outcome {
            CopyOutcome::PaperFilled(decision) => (
                decision.task_id,
                decision.signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "paper_filled",
            ),
            CopyOutcome::LiveSubmitted(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_submitted",
            ),
            CopyOutcome::LiveConfirmed(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_confirmed",
            ),
            CopyOutcome::LiveFailed(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_failed",
            ),
            CopyOutcome::PaperSellObserved(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "paper_sell_observed",
            ),
            CopyOutcome::LiveSellSubmitted(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_sell_submitted",
            ),
            CopyOutcome::LiveSellFailed(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_sell_failed",
            ),
            CopyOutcome::Skipped {
                task_id,
                signature,
                mint,
                decided_at,
                ..
            } => (
                *task_id,
                signature.as_str(),
                mint.as_deref(),
                *decided_at,
                "skipped",
            ),
        };
        let json = serde_json::to_string(&outcome).map_err(|e| Error::CopySerialize {
            field: "outcome",
            detail: e.to_string(),
        })?;
        let inserted = transaction
            .execute(
                "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(task_id, signature) DO NOTHING",
                params![task_id, signature, mint, json, decided_at.to_rfc3339()],
            )
            .map_err(crate::errors::DatabaseError::from)?
            > 0;
        let existing = if inserted {
            None
        } else {
            transaction
                .query_row(
                    "SELECT outcome_json FROM copy_decisions WHERE task_id=?1 AND signature=?2",
                    params![task_id, signature],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(crate::errors::DatabaseError::from)?
                .and_then(|value| serde_json::from_str::<CopyOutcome>(&value).ok())
        };
        let confirmation_upgrade = matches!(
            (&existing, &outcome),
            (
                Some(CopyOutcome::LiveSubmitted(_)),
                CopyOutcome::LiveConfirmed(_)
            )
        );
        if confirmation_upgrade {
            transaction
                .execute(
                    "UPDATE copy_decisions SET outcome_json=?3, decided_at=?4 WHERE task_id=?1 AND signature=?2",
                    params![task_id, signature, json, decided_at.to_rfc3339()],
                )
                .map_err(crate::errors::DatabaseError::from)?;
        }
        if inserted || confirmation_upgrade {
            transaction
                .execute(
                    "INSERT INTO copy_activity (task_id, kind, details_json, created_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![task_id, kind, json, Utc::now().to_rfc3339()],
                )
                .map_err(crate::errors::DatabaseError::from)?;
            let spend = match &outcome {
                CopyOutcome::PaperFilled(decision) => {
                    Some((decision.mint.as_str(), decision.sized_sol))
                }
                CopyOutcome::LiveSubmitted(decision) | CopyOutcome::LiveConfirmed(decision) => {
                    Some((decision.mint.as_str(), decision.sized_sol))
                }
                _ => None,
            };
            if inserted {
                if let Some((spend_mint, spend_sol)) = spend {
                    transaction
                    .execute(
                        "INSERT INTO copy_spend (task_id, mint, spent_sol, buy_count, updated_at) \
                         VALUES (?1, ?2, ?3, 1, ?4) \
                         ON CONFLICT(task_id, mint) DO UPDATE SET \
                         spent_sol = spent_sol + excluded.spent_sol, \
                         buy_count = buy_count + 1, updated_at = excluded.updated_at",
                        params![
                            task_id,
                            spend_mint,
                            spend_sol,
                            Utc::now().to_rfc3339()
                        ],
                    )
                    .map_err(crate::errors::DatabaseError::from)?;
                }
            }
        }
        let claim_state = match &outcome {
            CopyOutcome::LiveSubmitted(_) | CopyOutcome::LiveSellSubmitted(_) => Some("submitted"),
            CopyOutcome::LiveConfirmed(_)
            | CopyOutcome::LiveFailed(_)
            | CopyOutcome::LiveSellFailed(_)
            | CopyOutcome::Skipped { .. } => Some("settled"),
            _ => None,
        };
        if let Some(state) = claim_state {
            transaction.execute(
                "UPDATE copy_live_claims SET state=?3, updated_at=?4 WHERE task_id=?1 AND signature=?2",
                params![task_id, signature, state, Utc::now().to_rfc3339()],
            ).map_err(crate::errors::DatabaseError::from)?;
        }
        transaction
            .commit()
            .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
    }
}

fn update_target_holding(
    transaction: &rusqlite::Transaction<'_>,
    task_id: i64,
    mint: &str,
    delta: f64,
) -> crate::trader::Result<()> {
    if !delta.is_finite() || delta == 0.0 {
        return Ok(());
    }
    transaction.execute(
        "INSERT INTO copy_target_holdings (task_id, mint, token_amount, updated_at) VALUES (?1, ?2, 0, ?3) ON CONFLICT(task_id, mint) DO NOTHING",
        params![task_id, mint, Utc::now().to_rfc3339()],
    ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to initialize target holding: {e}") })?;
    transaction.execute(
        "UPDATE copy_target_holdings SET token_amount=MAX(0, token_amount + ?3), updated_at=?4 WHERE task_id=?1 AND mint=?2",
        params![task_id, mint, delta, Utc::now().to_rfc3339()],
    ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to update target holding: {e}") })?;
    Ok(())
}

#[cfg(test)]
#[path = "database/tests.rs"]
mod tests;
