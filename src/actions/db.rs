/// Actions Database Module
///
/// High-performance SQLite database for persistent action storage.
/// Follows EventsDatabase pattern with split read/write pools.
use super::types::{Action, ActionId, ActionState, ActionStep, ActionType, StepStatus};
use crate::logger::{self, LogTag};
use crate::utils::get_wallet_address;
use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum age for actions (30 days)
const MAX_ACTION_AGE_DAYS: i64 = 30;

/// Connection pool configuration
const WRITE_POOL_MAX_SIZE: u32 = 2;
const READ_POOL_MAX_SIZE: u32 = 10;
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
    pub async fn new() -> Result<Self, String> {
        let database_path = crate::paths::get_actions_db_path();
        let database_path_str = database_path.to_string_lossy().to_string();

        // Configure connection managers (same file for both pools)
        let write_manager = SqliteConnectionManager::file(&database_path);
        let read_manager = SqliteConnectionManager::file(&database_path);

        // Create write pool
        let write_pool = Pool::builder()
            .max_size(WRITE_POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .build(write_manager)
            .map_err(|e| format!("Failed to create actions write pool: {}", e))?;

        // Create read pool
        let read_pool = Pool::builder()
            .max_size(READ_POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .build(read_manager)
            .map_err(|e| format!("Failed to create actions read pool: {}", e))?;

        let mut db = ActionsDatabase {
            write_pool,
            read_pool,
            database_path: database_path_str.clone(),
        };

        // Initialize database schema
        db.initialize_schema().await?;

        logger::info(
            LogTag::System,
            &format!("Actions database initialized at {}", database_path_str),
        );

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let conn = self.get_write_connection()?;

        // Configure connection for optimal performance
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set journal mode: {}", e))?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
        conn.pragma_update(None, "cache_size", 10000)
            .map_err(|e| format!("Failed to set cache size: {}", e))?;
        conn.pragma_update(None, "temp_store", "memory")
            .map_err(|e| format!("Failed to set temp store: {}", e))?;
        conn.busy_timeout(std::time::Duration::from_millis(30_000))
            .map_err(|e| format!("Failed to set busy timeout: {}", e))?;

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
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
            [],
        )
        .map_err(|e| format!("Failed to create actions table: {}", e))?;

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
        .map_err(|e| format!("Failed to create action_steps table: {}", e))?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_action_type ON actions(action_type)",
            [],
        )
        .map_err(|e| format!("Failed to create action_type index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_entity_id ON actions(entity_id)",
            [],
        )
        .map_err(|e| format!("Failed to create entity_id index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_state ON actions(state)",
            [],
        )
        .map_err(|e| format!("Failed to create state index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_started_at ON actions(started_at DESC)",
            [],
        )
        .map_err(|e| format!("Failed to create started_at index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_wallet_address ON actions(wallet_address)",
            [],
        )
        .map_err(|e| format!("Failed to create wallet_address index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_actions_completed_at ON actions(completed_at DESC) WHERE completed_at IS NOT NULL",
            [],
        )
        .map_err(|e| format!("Failed to create completed_at index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_action_steps_action_id ON action_steps(action_id)",
            [],
        )
        .map_err(|e| format!("Failed to create action_steps index: {}", e))?;

        logger::info(LogTag::System, "Actions database schema initialized");

        Ok(())
    }

    /// Get a write connection from the pool
    fn get_write_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.write_pool
            .get()
            .map_err(|e| format!("Failed to get write connection: {}", e))
    }

    /// Get a read connection from the pool
    fn get_read_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.read_pool
            .get()
            .map_err(|e| format!("Failed to get read connection: {}", e))
    }

    /// Insert a new action into the database
    pub async fn insert_action(&self, action: &Action) -> Result<(), String> {
        let mut conn = self.get_write_connection()?;

        let wallet_address =
            get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;
        let action_type_str = format!("{:?}", action.action_type).to_lowercase();
        let state_str = match &action.state {
            ActionState::InProgress { .. } => "in_progress",
            ActionState::Completed => "completed",
            ActionState::Failed { .. } => "failed",
            ActionState::Cancelled => "cancelled",
        };
        let state_data = serde_json::to_string(&action.state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        let metadata = serde_json::to_string(&action.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        let now = Utc::now().to_rfc3339();

        // Use transaction to ensure atomicity of action + steps insertion
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        tx.execute(
            r#"
            INSERT INTO actions (
                id, action_type, entity_id, wallet_address, state, state_data,
                started_at, completed_at, duration_ms, metadata, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
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
        .map_err(|e| format!("Failed to insert action: {}", e))?;

        // Insert all steps within the same transaction
        for (index, step) in action.steps.iter().enumerate() {
            self.insert_step_internal_tx(&tx, &action.id, index, step)?;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Insert a step (internal helper for transaction)
    fn insert_step_internal_tx(
        &self,
        tx: &rusqlite::Transaction,
        action_id: &str,
        step_index: usize,
        step: &ActionStep,
    ) -> Result<(), String> {
        let status_str = format!("{:?}", step.status).to_lowercase();
        let metadata = serde_json::to_string(&step.metadata)
            .map_err(|e| format!("Failed to serialize step metadata: {}", e))?;

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
        .map_err(|e| format!("Failed to insert step: {}", e))?;

        Ok(())
    }

    /// Insert a step (internal helper)
    fn insert_step_internal(
        &self,
        conn: &Connection,
        action_id: &str,
        step_index: usize,
        step: &ActionStep,
    ) -> Result<(), String> {
        let status_str = format!("{:?}", step.status).to_lowercase();
        let metadata = serde_json::to_string(&step.metadata)
            .map_err(|e| format!("Failed to serialize step metadata: {}", e))?;

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
        .map_err(|e| format!("Failed to insert step: {}", e))?;

        Ok(())
    }

    /// Update action state
    pub async fn update_action_state(
        &self,
        action_id: &str,
        state: &ActionState,
        completed_at: Option<DateTime<Utc>>,
        started_at: DateTime<Utc>,
    ) -> Result<(), String> {
        let conn = self.get_write_connection()?;

        let state_str = match state {
            ActionState::InProgress { .. } => "in_progress",
            ActionState::Completed => "completed",
            ActionState::Failed { .. } => "failed",
            ActionState::Cancelled => "cancelled",
        };
        let state_data = serde_json::to_string(&state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
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
        .map_err(|e| format!("Failed to update action state: {}", e))?;

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
    ) -> Result<(), String> {
        let conn = self.get_write_connection()?;

        let status_str = format!("{:?}", status).to_lowercase();
        let now = Utc::now().to_rfc3339();

        let metadata_str = metadata.map(|m| serde_json::to_string(&m).ok()).flatten();

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
        .map_err(|e| format!("Failed to update step: {}", e))?;

        // Validate that the step was found and updated
        if affected == 0 {
            return Err(format!(
                "Step not found or not updated: action_id={}, step_index={}",
                action_id, step_index
            ));
        }

        Ok(())
    }

    /// Get a single action by ID
    pub async fn get_action(&self, action_id: &str) -> Result<Option<Action>, String> {
        let conn = self.get_read_connection()?;

        let action_row: Option<(
            String,         // id
            String,         // action_type
            String,         // entity_id
            String,         // state
            String,         // state_data
            String,         // started_at
            Option<String>, // completed_at
            String,         // metadata
            String,         // updated_at
        )> = conn
            .query_row(
                r#"
                SELECT id, action_type, entity_id, state, state_data,
                       started_at, completed_at, metadata, updated_at
                FROM actions
                WHERE id = ?1
                "#,
                params![action_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("Failed to query action: {}", e))?;

        if action_row.is_none() {
            return Ok(None);
        }

        let (
            id,
            action_type_str,
            entity_id,
            _state_str,
            state_data,
            started_at_str,
            completed_at_str,
            metadata_str,
            _updated_at,
        ) = action_row.unwrap();

        // Parse action type
        let action_type = self.parse_action_type(&action_type_str)?;

        // Parse state
        let state: ActionState = serde_json::from_str(&state_data)
            .map_err(|e| format!("Failed to parse state: {}", e))?;

        // Parse timestamps
        let started_at = DateTime::parse_from_rfc3339(&started_at_str)
            .map_err(|e| format!("Failed to parse started_at: {}", e))?
            .with_timezone(&Utc);

        let completed_at = if let Some(s) = completed_at_str {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        } else {
            None
        };

        // Parse metadata
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
            .map_err(|e| format!("Failed to parse metadata: {}", e))?;

        // Get steps
        let mut stmt = conn
            .prepare(
                r#"
                SELECT step_index, step_id, name, status, started_at, completed_at, error, metadata
                FROM action_steps
                WHERE action_id = ?1
                ORDER BY step_index ASC
                "#,
            )
            .map_err(|e| format!("Failed to prepare step query: {}", e))?;

        let steps = stmt
            .query_map(params![action_id], |row| {
                let status_str: String = row.get(3)?;
                let status = self.parse_step_status(&status_str);

                let started_at_str: Option<String> = row.get(4)?;
                let started_at = started_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let completed_at_str: Option<String> = row.get(5)?;
                let completed_at = completed_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let metadata_str: Option<String> = row.get(7)?;
                let metadata = metadata_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::Value::Null);

                Ok(ActionStep {
                    step_id: row.get(1)?,
                    name: row.get(2)?,
                    status,
                    started_at,
                    completed_at,
                    error: row.get(6)?,
                    metadata,
                })
            })
            .map_err(|e| format!("Failed to query steps: {}", e))?
            .collect::<Result<Vec<ActionStep>, _>>()
            .map_err(|e| format!("Failed to collect steps: {}", e))?;

        let current_step_index = match &state {
            ActionState::InProgress {
                current_step_index, ..
            } => *current_step_index,
            _ => 0,
        };

        Ok(Some(Action {
            id,
            action_type,
            entity_id,
            state,
            steps,
            current_step_index,
            started_at,
            completed_at,
            metadata,
        }))
    }

    /// Get actions with filters (optimized with batch fetching)
    pub async fn get_actions(&self, filters: &ActionFilters) -> Result<Vec<Action>, String> {
        let conn = self.get_read_connection()?;

        // Build query for action IDs
        let mut query = String::from(
            r#"
            SELECT id, action_type, entity_id, state, state_data,
                   started_at, completed_at, metadata, updated_at
            FROM actions
            WHERE 1=1
            "#,
        );

        let mut params: Vec<String> = Vec::new();

        if let Some(action_type) = filters.action_type {
            query.push_str(" AND action_type = ?");
            params.push(format!("{:?}", action_type).to_lowercase());
        }

        if let Some(ref entity_id) = filters.entity_id {
            query.push_str(" AND entity_id = ?");
            params.push(entity_id.clone());
        }

        if let Some(ref states) = filters.state {
            if !states.is_empty() {
                let placeholders = states.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND state IN ({})", placeholders));
                for state in states {
                    params.push(state.clone());
                }
            }
        }

        if let Some(started_after) = filters.started_after {
            query.push_str(" AND started_at >= ?");
            params.push(started_after.to_rfc3339());
        }

        if let Some(started_before) = filters.started_before {
            query.push_str(" AND started_at <= ?");
            params.push(started_before.to_rfc3339());
        }

        query.push_str(" ORDER BY started_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filters.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

        // Fetch all actions in one query
        let actions_data: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
        )> = stmt
            .query_map(&params_refs[..], |row| {
                Ok((
                    row.get(0)?, // id
                    row.get(1)?, // action_type
                    row.get(2)?, // entity_id
                    row.get(3)?, // state
                    row.get(4)?, // state_data
                    row.get(5)?, // started_at
                    row.get(6)?, // completed_at
                    row.get(7)?, // metadata
                ))
            })
            .map_err(|e| format!("Failed to query actions: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect actions: {}", e))?;

        if actions_data.is_empty() {
            return Ok(Vec::new());
        }

        // Collect action IDs for batch step fetch
        let action_ids: Vec<String> = actions_data.iter().map(|(id, ..)| id.clone()).collect();

        // Batch fetch all steps for these actions in ONE query
        let placeholders = action_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let steps_query = format!(
            r#"
            SELECT action_id, step_index, step_id, name, status, 
                   started_at, completed_at, error, metadata
            FROM action_steps
            WHERE action_id IN ({})
            ORDER BY action_id, step_index ASC
            "#,
            placeholders
        );

        let mut steps_stmt = conn
            .prepare(&steps_query)
            .map_err(|e| format!("Failed to prepare steps query: {}", e))?;

        let action_id_refs: Vec<&dyn rusqlite::ToSql> = action_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let steps_rows = steps_stmt
            .query_map(&action_id_refs[..], |row| {
                let action_id: String = row.get(0)?;
                let status_str: String = row.get(4)?;
                let status = self.parse_step_status(&status_str);

                let started_at_str: Option<String> = row.get(5)?;
                let started_at = started_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let completed_at_str: Option<String> = row.get(6)?;
                let completed_at = completed_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let metadata_str: Option<String> = row.get(8)?;
                let metadata = metadata_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or(serde_json::Value::Null);

                Ok((
                    action_id,
                    ActionStep {
                        step_id: row.get(2)?,
                        name: row.get(3)?,
                        status,
                        started_at,
                        completed_at,
                        error: row.get(7)?,
                        metadata,
                    },
                ))
            })
            .map_err(|e| format!("Failed to query steps: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect steps: {}", e))?;

        // Build a map of action_id -> Vec<ActionStep>
        let mut steps_map: HashMap<String, Vec<ActionStep>> = HashMap::new();
        for (action_id, step) in steps_rows {
            steps_map
                .entry(action_id)
                .or_insert_with(Vec::new)
                .push(step);
        }

        // Assemble actions with their steps
        let mut actions = Vec::new();
        for (
            id,
            action_type_str,
            entity_id,
            _state_str,
            state_data,
            started_at_str,
            completed_at_str,
            metadata_str,
        ) in actions_data
        {
            let action_type = self.parse_action_type(&action_type_str)?;

            let state: ActionState = serde_json::from_str(&state_data)
                .map_err(|e| format!("Failed to parse state for action {}: {}", id, e))?;

            let started_at = DateTime::parse_from_rfc3339(&started_at_str)
                .map_err(|e| format!("Failed to parse started_at for action {}: {}", id, e))?
                .with_timezone(&Utc);

            let completed_at = if let Some(s) = completed_at_str {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            } else {
                None
            };

            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)
                .map_err(|e| format!("Failed to parse metadata for action {}: {}", id, e))?;

            let steps = steps_map.remove(&id).unwrap_or_default();

            let current_step_index = match &state {
                ActionState::InProgress {
                    current_step_index, ..
                } => *current_step_index,
                _ => 0,
            };

            actions.push(Action {
                id,
                action_type,
                entity_id,
                state,
                steps,
                current_step_index,
                started_at,
                completed_at,
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get action history with pagination
    pub async fn get_action_history(
        &self,
        limit: usize,
        offset: usize,
        filters: &ActionFilters,
    ) -> Result<(Vec<Action>, usize), String> {
        // Get total count in a scope to drop conn and params early
        let total = {
            let conn = self.get_read_connection()?;

            let mut count_query = String::from("SELECT COUNT(*) FROM actions WHERE 1=1");
            let mut params: Vec<String> = Vec::new();

            if let Some(action_type) = filters.action_type {
                count_query.push_str(" AND action_type = ?");
                params.push(format!("{:?}", action_type).to_lowercase());
            }

            if let Some(ref entity_id) = filters.entity_id {
                count_query.push_str(" AND entity_id = ?");
                params.push(entity_id.clone());
            }

            if let Some(ref states) = filters.state {
                if !states.is_empty() {
                    let placeholders = states.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                    count_query.push_str(&format!(" AND state IN ({})", placeholders));
                    for state in states {
                        params.push(state.clone());
                    }
                }
            }

            if let Some(started_after) = filters.started_after.as_ref() {
                count_query.push_str(" AND started_at >= ?");
                params.push(started_after.to_rfc3339());
            }

            if let Some(started_before) = filters.started_before.as_ref() {
                count_query.push_str(" AND started_at <= ?");
                params.push(started_before.to_rfc3339());
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

            let total: i64 = conn
                .query_row(&count_query, &params_refs[..], |row| row.get(0))
                .map_err(|e| format!("Failed to count actions: {}", e))?;

            total as usize
        };

        // Get actions (conn and params are now dropped)
        let mut filters_with_pagination = filters.clone();
        filters_with_pagination.limit = Some(limit);
        filters_with_pagination.offset = Some(offset);

        let actions = self.get_actions(&filters_with_pagination).await?;

        Ok((actions, total))
    }

    /// Cleanup old actions
    pub async fn cleanup_old_actions(&self, days: i64) -> Result<usize, String> {
        let mut conn = self.get_write_connection()?;

        let cutoff = Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        // Use transaction to ensure both deletes succeed or both roll back
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let deleted = tx
            .execute(
                "DELETE FROM actions WHERE completed_at < ?1 AND completed_at IS NOT NULL",
                params![cutoff_str],
            )
            .map_err(|e| format!("Failed to cleanup old actions: {}", e))?;

        // Cleanup orphaned steps
        tx.execute(
            "DELETE FROM action_steps WHERE action_id NOT IN (SELECT id FROM actions)",
            [],
        )
        .map_err(|e| format!("Failed to cleanup orphaned steps: {}", e))?;

        tx.commit()
            .map_err(|e| format!("Failed to commit cleanup transaction: {}", e))?;

        if deleted > 0 {
            logger::info(
                LogTag::System,
                &format!(
                    "Cleaned up {} old actions (older than {} days)",
                    deleted, days
                ),
            );
        }

        Ok(deleted)
    }

    /// Get recent incomplete actions for startup sync
    pub async fn get_recent_incomplete_actions(&self) -> Result<Vec<Action>, String> {
        // NOTE: We intentionally do not time-box this query. Any action that is still
        // running must be restored after a restart so the in-memory cache and SSE
        // stream remain consistent with the database.
        let filters = ActionFilters {
            state: Some(vec!["in_progress".to_string()]),
            limit: Some(500),
            ..Default::default()
        };

        self.get_actions(&filters).await
    }

    /// Parse action type from string
    fn parse_action_type(&self, s: &str) -> Result<ActionType, String> {
        match s {
            "swapbuy" => Ok(ActionType::SwapBuy),
            "swapsell" => Ok(ActionType::SwapSell),
            "positionopen" => Ok(ActionType::PositionOpen),
            "positionclose" => Ok(ActionType::PositionClose),
            "positiondca" => Ok(ActionType::PositionDca),
            "positionpartialexit" => Ok(ActionType::PositionPartialExit),
            "manualorder" => Ok(ActionType::ManualOrder),
            _ => Err(format!("Unknown action type: {}", s)),
        }
    }

    /// Parse step status from string
    fn parse_step_status(&self, s: &str) -> StepStatus {
        match s {
            "pending" => StepStatus::Pending,
            "inprogress" => StepStatus::InProgress,
            "completed" => StepStatus::Completed,
            "failed" => StepStatus::Failed,
            "skipped" => StepStatus::Skipped,
            _ => StepStatus::Pending,
        }
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
