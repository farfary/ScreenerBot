// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Actions database module.
//!
//! High-performance SQLite database for persistent action storage.
//! Follows EventsDatabase pattern with split read/write pools.
//!
//! Submodules:
//! - `queries`: Read operations, filtered listing, pagination, cleanup

mod queries;

use super::failure::ActionFailure;
use super::types::{Action, ActionState, ActionStep, ActionType, StepStatus};
use crate::actions::{Error, Result};
use crate::database;
use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::database::WriteTransaction;
use crate::errors::{DataError, DatabaseError, InternalError};
use crate::logger::{self, LogTag};
use crate::utils::get_wallet_address;
use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};

/// The actions store's schema.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: 1,
    min_reader_version: 1,
    baseline: 1,
    renames: None,
    step: create_schema,
    legacy_stamp: None,
};

fn create_schema(
    tx: &Transaction<'_>,
    _: &SchemaContext,
) -> std::result::Result<(), DatabaseError> {
    ddl::run(
        tx,
        "create actions table",
        "CREATE TABLE IF NOT EXISTS actions (
            id TEXT PRIMARY KEY,
            action_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            wallet_address TEXT NOT NULL,
            state TEXT NOT NULL,
            state_data TEXT,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            duration_ms INTEGER,
            read_at TEXT,
            dismissed_at TEXT,
            metadata TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            chain_id TEXT NOT NULL DEFAULT 'solana'
        )",
    )?;
    // Chain scope: the shared file gains a chain column. Historical rows are Solana's by
    // construction, so the migration backfills 'solana'.
    ddl::add_column(tx, "actions", "chain_id", "TEXT NOT NULL DEFAULT 'solana'")?;
    ddl::run(
        tx,
        "create action steps table",
        "CREATE TABLE IF NOT EXISTS action_steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action_id TEXT NOT NULL,
            step_index INTEGER NOT NULL,
            step_id TEXT NOT NULL,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT,
            duration_ms INTEGER,
            error TEXT,
            metadata TEXT,
            FOREIGN KEY (action_id) REFERENCES actions(id),
            UNIQUE(action_id, step_index)
        )",
    )?;
    ddl::add_column(tx, "actions", "read_at", "TEXT")?;
    ddl::add_column(tx, "actions", "dismissed_at", "TEXT")?;
    ddl::run_batch(
        tx,
        "create actions indexes",
        "CREATE INDEX IF NOT EXISTS idx_actions_action_type ON actions(action_type);
         CREATE INDEX IF NOT EXISTS idx_actions_entity_id ON actions(entity_id);
         CREATE INDEX IF NOT EXISTS idx_actions_state ON actions(state);
         CREATE INDEX IF NOT EXISTS idx_actions_started_at ON actions(started_at DESC);
         CREATE INDEX IF NOT EXISTS idx_actions_wallet_address ON actions(wallet_address);
         CREATE INDEX IF NOT EXISTS idx_actions_completed_at ON actions(completed_at DESC) WHERE completed_at IS NOT NULL;
         CREATE INDEX IF NOT EXISTS idx_actions_read_at ON actions(read_at);
         CREATE INDEX IF NOT EXISTS idx_actions_dismissed_at ON actions(dismissed_at);
         CREATE INDEX IF NOT EXISTS idx_action_steps_action_id ON action_steps(action_id);",
    )
}

// =============================================================================
// DATABASE STRUCTURE
// =============================================================================

/// High-performance actions database with split connection pools
pub struct ActionsDatabase {
    write_pool: Pool<SqliteConnectionManager>,
    read_pool: Pool<SqliteConnectionManager>,
    database_path: String,
}

impl ActionsDatabase {
    /// Open the actions store with its writer and reader pools.
    pub async fn new() -> Result<Self> {
        let database_path = crate::paths::get_actions_db_path();
        let write_pool = database::open::store(&database::catalog::ACTIONS)?;
        let read_pool = database::open::reader(&database::catalog::ACTIONS, &database_path)?;
        let database_path = database_path.to_string_lossy().to_string();
        logger::info(
            LogTag::System,
            &format!("Actions database initialized at {database_path}"),
        );
        Ok(ActionsDatabase {
            write_pool,
            read_pool,
            database_path,
        })
    }

    /// Get a write connection from the pool
    pub(crate) fn get_write_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.write_pool.get().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "get actions write connection".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Get a read connection from the pool
    pub(crate) fn get_read_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.read_pool.get().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "get actions read connection".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Insert a new action into the database
    pub async fn insert_action(&self, action: &Action) -> Result<()> {
        let mut conn = self.get_write_connection()?;

        let wallet_address = get_wallet_address().map_err(|e| {
            Error::Internal(InternalError::InvariantViolation {
                message: format!("failed to get wallet address: {e}"),
            })
        })?;
        let action_type_str = format!("{:?}", action.action_type).to_lowercase();
        let state_str = match &action.state {
            ActionState::InProgress { .. } => "in_progress",
            ActionState::Completed => "completed",
            ActionState::Failed { .. } => "failed",
            ActionState::Cancelled => "cancelled",
        };
        let state_data = serde_json::to_string(&action.state).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "action state".to_owned(),
                error: e.to_string(),
            })
        })?;
        let metadata = serde_json::to_string(&action.metadata).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "action metadata".to_owned(),
                error: e.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();
        let chain_id = crate::chains::legacy_row_chain().as_str();

        // Use transaction to ensure atomicity of action + steps insertion
        let tx = conn.write_tx().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "begin action insert transaction".to_owned(),
                message: e.to_string(),
            })
        })?;

        tx.execute(
            r#"
            INSERT INTO actions (
                id, action_type, entity_id, wallet_address, state, state_data,
                started_at, completed_at, duration_ms, read_at, dismissed_at,
                metadata, created_at, updated_at, chain_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, ?10, ?11, ?12, ?13)
            "#,
            params![
                action.id,
                action_type_str,
                action.entity_id,
                wallet_address,
                state_str,
                state_data,
                action.started_at.to_rfc3339(),
                action.completed_at.map(|dt| dt.to_rfc3339()),
                action
                    .completed_at
                    .map(|end| (end - action.started_at).num_milliseconds()),
                metadata,
                now,
                now,
                chain_id,
            ],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "insert action".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Insert all steps within the same transaction
        for (index, step) in action.steps.iter().enumerate() {
            self.insert_step_internal_tx(&tx, &action.id, index, step)?;
        }

        tx.commit().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "commit action insert transaction".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Insert a step (internal helper for transaction)
    fn insert_step_internal_tx(
        &self,
        tx: &rusqlite::Transaction,
        action_id: &str,
        step_index: usize,
        step: &ActionStep,
    ) -> Result<()> {
        let status_str = format!("{:?}", step.status).to_lowercase();
        let metadata = serde_json::to_string(&step.metadata).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "action step metadata".to_owned(),
                error: e.to_string(),
            })
        })?;

        tx.execute(
            r#"
            INSERT INTO action_steps (
                action_id, step_index, step_id, name, status,
                started_at, completed_at, duration_ms, error, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                action_id,
                step_index as i64,
                step.step_id,
                step.name.as_str(),
                status_str,
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.and_then(|end| step
                    .started_at
                    .map(|start| (end - start).num_milliseconds())),
                step.error.as_ref().map(ActionFailure::to_stored),
                metadata,
            ],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "insert action step".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Insert a step (internal helper)
    fn insert_step_internal(
        &self,
        conn: &Connection,
        action_id: &str,
        step_index: usize,
        step: &ActionStep,
    ) -> Result<()> {
        let status_str = format!("{:?}", step.status).to_lowercase();
        let metadata = serde_json::to_string(&step.metadata).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "action step metadata".to_owned(),
                error: e.to_string(),
            })
        })?;

        conn.execute(
            r#"
            INSERT INTO action_steps (
                action_id, step_index, step_id, name, status,
                started_at, completed_at, duration_ms, error, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                action_id,
                step_index as i64,
                step.step_id,
                step.name.as_str(),
                status_str,
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.and_then(|end| step
                    .started_at
                    .map(|start| (end - start).num_milliseconds())),
                step.error.as_ref().map(ActionFailure::to_stored),
                metadata,
            ],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "insert action step".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Update action state
    pub async fn update_action_state(
        &self,
        action_id: &str,
        state: &ActionState,
        completed_at: Option<DateTime<Utc>>,
        started_at: DateTime<Utc>,
    ) -> Result<()> {
        let conn = self.get_write_connection()?;

        let state_str = match state {
            ActionState::InProgress { .. } => "in_progress",
            ActionState::Completed => "completed",
            ActionState::Failed { .. } => "failed",
            ActionState::Cancelled => "cancelled",
        };
        let state_data = serde_json::to_string(&state).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "action state".to_owned(),
                error: e.to_string(),
            })
        })?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            UPDATE actions
            SET state = ?1, state_data = ?2, completed_at = ?3, 
                duration_ms = ?4, updated_at = ?5
            WHERE id = ?6
            "#,
            params![
                state_str,
                state_data,
                completed_at.map(|dt| dt.to_rfc3339()),
                completed_at.map(|end| (end - started_at).num_milliseconds()),
                now,
                action_id,
            ],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "update action state".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Update a step
    pub async fn update_step(
        &self,
        action_id: &str,
        step_index: usize,
        status: StepStatus,
        error: Option<&ActionFailure>,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let conn = self.get_write_connection()?;

        let status_str = format!("{:?}", status).to_lowercase();
        let now = Utc::now().to_rfc3339();

        let metadata_str = metadata.and_then(|m| serde_json::to_string(&m).ok());

        // Atomic UPDATE using COALESCE to prevent race conditions
        // - Set started_at if transitioning to InProgress and not already set
        // - Set completed_at if transitioning to terminal state and not already set
        // - Calculate duration_ms from timestamps
        let affected = conn.execute(
            r#"
            UPDATE action_steps
            SET status = ?1,
                started_at = CASE 
                    WHEN ?2 = 'inprogress' AND started_at IS NULL THEN ?3
                    ELSE started_at 
                END,
                completed_at = CASE 
                    WHEN ?2 IN ('completed', 'failed', 'skipped') AND completed_at IS NULL THEN ?3
                    ELSE completed_at 
                END,
                duration_ms = CASE
                    WHEN completed_at IS NOT NULL AND started_at IS NOT NULL THEN
                        CAST((julianday(completed_at) - julianday(started_at)) * 86400000 AS INTEGER)
                    ELSE NULL
                END,
                error = ?4,
                metadata = COALESCE(?5, metadata)
            WHERE action_id = ?6 AND step_index = ?7
            "#,
            params![
                status_str,
                status_str,
                now,
                error.map(ActionFailure::to_stored),
                metadata_str,
                action_id,
                step_index as i64,
            ],
        )
        .map_err(|e| Error::Database(DatabaseError::Query {
            operation: "update action step".to_owned(),
            message: e.to_string(),
        }))?;

        // Validate that the step was found and updated
        if affected == 0 {
            return Err(Error::Internal(InternalError::InvariantViolation {
                message: format!(
                    "step not found or not updated: action_id={action_id}, step_index={step_index}"
                ),
            }));
        }

        Ok(())
    }
}

/// Filters for querying actions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionFilters {
    /// Matches any of these types; empty matches every type.
    pub action_types: Vec<ActionType>,
    pub entity_id: Option<String>,
    pub state: Option<Vec<String>>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
