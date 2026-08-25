//! Actions database module.
//!
//! High-performance SQLite database for persistent action storage.
//! Follows EventsDatabase pattern with split read/write pools.
//!
//! Submodules:
//! - `queries`: Read operations, filtered listing, pagination, cleanup

mod queries;

use super::types::{Action, ActionState, ActionStep, ActionType, StepStatus};
use crate::actions::{Error, Result};
use crate::database;
use crate::database::WriteTransaction;
use crate::errors::{DataError, DatabaseError, InternalError};
use crate::logger::{self, LogTag};
use crate::utils::get_wallet_address;
use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum age for actions (30 days)
const MAX_ACTION_AGE_DAYS: i64 = 30;

/// Connection pool configuration
const WRITE_POOL_MAX_SIZE: u32 = 2;
const READ_POOL_MAX_SIZE: u32 = 4;
const POOL_MIN_IDLE: u32 = 1;
const CONNECTION_TIMEOUT_MS: u64 = 30_000;

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
    /// Create new ActionsDatabase with connection pooling
    pub async fn new() -> Result<Self> {
        let database_path = crate::paths::get_actions_db_path();
        let database_path_str = database_path.to_string_lossy().to_string();

        // Configure connection managers with centralized PRAGMAs
        let write_manager = SqliteConnectionManager::file(&database_path)
            .with_init(|c| database::configure_connection(c, database::ACTIONS_WRITE_DB));
        let read_manager = SqliteConnectionManager::file(&database_path)
            .with_init(|c| database::configure_connection(c, database::ACTIONS_READ_DB));

        // Create write pool
        let write_pool = Pool::builder()
            .max_size(WRITE_POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(write_manager)
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "create actions write pool".to_owned(),
                    message: e.to_string(),
                })
            })?;

        // Create read pool
        let read_pool = Pool::builder()
            .max_size(READ_POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(read_manager)
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "create actions read pool".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut db = ActionsDatabase {
            write_pool,
            read_pool,
            database_path: database_path_str.clone(),
        };

        // Initialize database schema
        db.initialize_schema().await?;

        logger::info(
            LogTag::System,
            &format!("Actions database initialized at {database_path_str}"),
        );

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<()> {
        let conn = self.get_write_connection()?;

        // Create main actions table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS actions (
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
                updated_at TEXT NOT NULL
            )
            "#,
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create actions table".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Create action steps table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS action_steps (
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
            )
            "#,
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action steps table".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_action_type ON actions(action_type)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action type index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_entity_id ON actions(entity_id)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action entity id index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_state ON actions(state)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action state index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_started_at ON actions(started_at DESC)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action started at index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_wallet_address ON actions(wallet_address)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action wallet address index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_completed_at ON actions(completed_at DESC) WHERE completed_at IS NOT NULL",
            [],
        )
        .map_err(|e| Error::Database(DatabaseError::Query {
            operation: "create action completed at index".to_owned(),
            message: e.to_string(),
        }))?;

        self.ensure_actions_column(&conn, "read_at", "TEXT")?;
        self.ensure_actions_column(&conn, "dismissed_at", "TEXT")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_read_at ON actions(read_at)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action read at index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_dismissed_at ON actions(dismissed_at)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action dismissed at index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_action_steps_action_id ON action_steps(action_id)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create action steps index".to_owned(),
                message: e.to_string(),
            })
        })?;

        logger::info(LogTag::System, "Actions database schema initialized");

        Ok(())
    }

    fn ensure_actions_column(
        &self,
        conn: &Connection,
        column_name: &str,
        column_type: &str,
    ) -> Result<()> {
        let mut stmt = conn.prepare("PRAGMA table_info(actions)").map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "inspect actions schema".to_owned(),
                message: e.to_string(),
            })
        })?;

        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "read actions schema".to_owned(),
                    message: e.to_string(),
                })
            })?;

        for column in columns {
            let column = column.map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "read actions schema row".to_owned(),
                    message: e.to_string(),
                })
            })?;
            if column == column_name {
                return Ok(());
            }
        }

        conn.execute(
            &format!("ALTER TABLE actions ADD COLUMN {column_name} {column_type}"),
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: format!("add actions.{column_name}"),
                message: e.to_string(),
            })
        })?;

        logger::info(
            LogTag::System,
            &format!("Added actions.{column_name} column"),
        );

        Ok(())
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
                metadata, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, ?10, ?11, ?12)
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
                step.name,
                status_str,
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.and_then(|end| step
                    .started_at
                    .map(|start| (end - start).num_milliseconds())),
                step.error,
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
                step.name,
                status_str,
                step.started_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.map(|dt| dt.to_rfc3339()),
                step.completed_at.and_then(|end| step
                    .started_at
                    .map(|start| (end - start).num_milliseconds())),
                step.error,
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
        error: Option<String>,
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
                error,
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
    pub action_type: Option<ActionType>,
    pub entity_id: Option<String>,
    pub state: Option<Vec<String>>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
