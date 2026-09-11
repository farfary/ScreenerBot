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
    confirm_mode_transition, CopyActivityRow, CopyMode, CopyOutcome, CopyPauseReason, CopyTask,
};

#[path = "database/claims.rs"]
mod claims;
#[path = "database/ledger.rs"]
mod ledger;
#[path = "database/maintenance.rs"]
mod maintenance;

pub use maintenance::{ActivityQuery, TargetObservations};
#[path = "database/rows.rs"]
mod rows;
#[path = "database/schema.rs"]
mod schema;

use rows::{json_error, parse_datetime, row_to_task, TASK_COLUMNS};
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
                 created_at, updated_at, require_filter_pass, pause_reason_json, paused_at) VALUES \
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                    task.require_filter_pass,
                    pause_reason_json(&task)?,
                    task.paused_at.map(|at| at.to_rfc3339()),
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
            .prepare(&format!(
                "SELECT {TASK_COLUMNS} FROM copy_tasks WHERE chain_id=?1 ORDER BY id DESC"
            ))
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
                &format!("SELECT {TASK_COLUMNS} FROM copy_tasks WHERE id = ?1 AND chain_id=?2"),
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

    /// Write a task's editable fields. The target wallet is the task's identity,
    /// not a setting, so it is never rewritten; another wallet is a new task.
    fn update_task_sync(&self, mut task: CopyTask) -> crate::trader::Result<CopyTask> {
        let connection = self.connection()?;
        let current_mode: String = connection
            .query_row(
                "SELECT mode_json FROM copy_tasks WHERE id=?1 AND chain_id=?2",
                params![task.id, self.chain.as_str()],
                |row| row.get(0),
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
        let serialize = |field: &'static str, value: Result<String, serde_json::Error>| {
            value.map_err(|e| Error::CopySerialize {
                field,
                detail: e.to_string(),
            })
        };
        let affected = connection
            .execute(
                "UPDATE copy_tasks SET label=?1, enabled=?2, mode_json=?3, sizing_json=?4, \
                 exit_mode_json=?5, exit_policy_json=?6, max_sol_per_trade=?7, max_sol_per_token=?8, \
                 total_budget_sol=?9, min_target_trade_sol=?10, max_target_trade_sol=?11, \
                 buy_once_per_token=?12, slippage_pct=?13, updated_at=?14, require_filter_pass=?15, \
                 pause_reason_json=?16, paused_at=?17 WHERE id=?18 AND chain_id=?19",
                params![
                    task.label,
                    task.enabled,
                    serialize("mode", serde_json::to_string(&task.mode))?,
                    serialize("sizing", serde_json::to_string(&task.sizing))?,
                    serialize("exit_mode", serde_json::to_string(&task.exit_mode))?,
                    serialize("exit_policy", serde_json::to_string(&task.exit_policy_overrides))?,
                    task.max_sol_per_trade,
                    task.max_sol_per_token,
                    task.total_budget_sol,
                    task.min_target_trade_sol,
                    task.max_target_trade_sol,
                    task.buy_once_per_token,
                    task.slippage_pct,
                    task.updated_at.to_rfc3339(),
                    task.require_filter_pass,
                    pause_reason_json(&task)?,
                    task.paused_at.map(|at| at.to_rfc3339()),
                    task.id,
                    self.chain.as_str(),
                ],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        if affected == 0 {
            return Err(Error::CopyTaskNotFound { task_id: task.id });
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

    /// Stand an enabled task down with the reason a guard gave; `false` when it
    /// was not enabled.
    pub async fn pause_task(
        &self,
        id: i64,
        reason: CopyPauseReason,
    ) -> crate::trader::Result<bool> {
        let db = self.clone();
        let reason = serde_json::to_string(&reason).map_err(|e| Error::CopySerialize {
            field: "pause_reason",
            detail: e.to_string(),
        })?;
        tokio::task::spawn_blocking(move || {
            let now = Utc::now().to_rfc3339();
            db.connection()?
                .execute(
                    "UPDATE copy_tasks SET enabled=0, updated_at=?2, paused_at=?2, pause_reason_json=?4 \
                     WHERE id=?1 AND chain_id=?3 AND enabled=1",
                    params![id, now, db.chain.as_str(), reason],
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
                &format!("SELECT {TASK_COLUMNS} FROM copy_tasks WHERE target_address = ?1 AND chain_id=?2 AND enabled = 1 ORDER BY id"),
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map(params![address, self.chain.as_str()], row_to_task)
            .map_err(crate::errors::DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::errors::DatabaseError::from)?;
        Ok(rows)
    }
}

fn pause_reason_json(task: &CopyTask) -> crate::trader::Result<Option<String>> {
    task.pause_reason
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| Error::CopySerialize {
            field: "pause_reason",
            detail: e.to_string(),
        })
}

#[cfg(test)]
#[path = "database/tests.rs"]
mod tests;
