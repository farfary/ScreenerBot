use crate::logger::{self, LogTag};
use crate::strategies::types::{
    EvaluationResult, RiskLevel, Strategy, StrategyPerformance, StrategyTemplate, StrategyType,
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

// Static flag to track if database has been initialized
static STRATEGIES_DB_INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Database schema version
const STRATEGIES_SCHEMA_VERSION: u32 = 1;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_STRATEGIES: &str = r#"
CREATE TABLE IF NOT EXISTS strategies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    type TEXT NOT NULL, -- 'ENTRY' or 'EXIT'
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 10,
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_strategies_type ON strategies(type);
CREATE INDEX IF NOT EXISTS idx_strategies_enabled ON strategies(enabled);
CREATE INDEX IF NOT EXISTS idx_strategies_priority ON strategies(priority);
"#;

const SCHEMA_STRATEGY_PERFORMANCE: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id TEXT NOT NULL,
    execution_time_ms INTEGER NOT NULL,
    result INTEGER NOT NULL, -- 0 or 1
    confidence REAL NOT NULL,
    details_json TEXT,
    token_mint TEXT,
    execution_timestamp TEXT NOT NULL,
    trade_id TEXT,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_performance_strategy ON strategy_performance(strategy_id);
CREATE INDEX IF NOT EXISTS idx_performance_timestamp ON strategy_performance(execution_timestamp);
CREATE INDEX IF NOT EXISTS idx_performance_token ON strategy_performance(token_mint);
"#;

const SCHEMA_STRATEGY_ASSIGNMENTS: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_assignments (
    position_id TEXT NOT NULL,
    strategy_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    PRIMARY KEY (position_id, strategy_id)
);

CREATE INDEX IF NOT EXISTS idx_assignments_position ON strategy_assignments(position_id);
CREATE INDEX IF NOT EXISTS idx_assignments_strategy ON strategy_assignments(strategy_id);
"#;

const SCHEMA_STRATEGY_TEMPLATES: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    risk_level TEXT NOT NULL, -- 'LOW', 'MEDIUM', 'HIGH'
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT
);

CREATE INDEX IF NOT EXISTS idx_templates_category ON strategy_templates(category);
CREATE INDEX IF NOT EXISTS idx_templates_risk ON strategy_templates(risk_level);
"#;

const SCHEMA_STRATEGY_BACKTESTS: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_backtests (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    total_trades INTEGER NOT NULL,
    win_trades INTEGER NOT NULL,
    loss_trades INTEGER NOT NULL,
    total_profit_sol REAL NOT NULL,
    results_json TEXT NOT NULL,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_backtests_strategy ON strategy_backtests(strategy_id);
CREATE INDEX IF NOT EXISTS idx_backtests_start ON strategy_backtests(start_time);
"#;

const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
"#;

// =============================================================================
// CONNECTION POOL
// =============================================================================

static DB_POOL: Lazy<Pool<SqliteConnectionManager>> = Lazy::new(|| {
    let db_path = crate::paths::get_strategies_db_path();
    let manager = SqliteConnectionManager::file(&db_path);
    Pool::builder()
        .max_size(10)
        .build(manager)
        .expect("Failed to create strategies database pool")
});

/// Get a connection from the pool
fn get_connection() -> Result<PooledConnection<SqliteConnectionManager>, String> {
    DB_POOL
        .get()
        .map_err(|e| format!("Failed to get database connection: {}", e))
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize the strategies database with all schemas
pub fn init_strategies_db() -> Result<(), String> {
    if STRATEGIES_DB_INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let conn = get_connection()?;

    // Enable WAL mode for better concurrency
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA cache_size = 10000;
        PRAGMA temp_store = memory;
        PRAGMA busy_timeout = 30000;
    ",
    )
    .map_err(|e| format!("Failed to set pragmas: {}", e))?;

    // Create version table first
    conn.execute_batch(SCHEMA_VERSION_TABLE)
        .map_err(|e| format!("Failed to create version table: {}", e))?;

    // Check current schema version
    let current_version: Option<u32> = conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to check schema version: {}", e))?;

    if current_version.is_none() || current_version.unwrap() < STRATEGIES_SCHEMA_VERSION {
        // Create all tables
        conn.execute_batch(SCHEMA_STRATEGIES)
            .map_err(|e| format!("Failed to create strategies table: {}", e))?;

        conn.execute_batch(SCHEMA_STRATEGY_PERFORMANCE)
            .map_err(|e| format!("Failed to create performance table: {}", e))?;

        conn.execute_batch(SCHEMA_STRATEGY_ASSIGNMENTS)
            .map_err(|e| format!("Failed to create assignments table: {}", e))?;

        conn.execute_batch(SCHEMA_STRATEGY_TEMPLATES)
            .map_err(|e| format!("Failed to create templates table: {}", e))?;

        conn.execute_batch(SCHEMA_STRATEGY_BACKTESTS)
            .map_err(|e| format!("Failed to create backtests table: {}", e))?;

        // Update version
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![STRATEGIES_SCHEMA_VERSION, Utc::now().to_rfc3339()],
        )
        .map_err(|e| format!("Failed to update schema version: {}", e))?;

        logger::info(
            LogTag::System,
            &format!(
                "Strategies database initialized with schema version {}",
                STRATEGIES_SCHEMA_VERSION
            ),
        );
    }

    STRATEGIES_DB_INITIALIZED.store(true, Ordering::Relaxed);
    Ok(())
}

// =============================================================================
// STRATEGY CRUD OPERATIONS
// =============================================================================

/// Insert a new strategy
pub fn insert_strategy(strategy: &Strategy) -> Result<(), String> {
    let conn = get_connection()?;

    let rules_json = serde_json::to_string(&strategy.rules)
        .map_err(|e| format!("Failed to serialize rules: {}", e))?;

    let parameters_json = serde_json::to_string(&strategy.parameters)
        .map_err(|e| format!("Failed to serialize parameters: {}", e))?;

    conn.execute(
        "INSERT INTO strategies (id, name, description, type, enabled, priority, rules_json, parameters_json, created_at, updated_at, author, version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            strategy.id,
            strategy.name,
            strategy.description,
            strategy.strategy_type.to_string(),
            strategy.enabled,
            strategy.priority,
            rules_json,
            parameters_json,
            strategy.created_at.to_rfc3339(),
            strategy.updated_at.to_rfc3339(),
            strategy.author,
            strategy.version,
        ],
    )
    .map_err(|e| format!("Failed to insert strategy: {}", e))?;

    logger::info(
        LogTag::System,
        &format!(
            "Inserted strategy: id={}, name={}, type={}",
            strategy.id, strategy.name, strategy.strategy_type
        ),
    );

    Ok(())
}

/// Update an existing strategy
pub fn update_strategy(strategy: &Strategy) -> Result<(), String> {
    let conn = get_connection()?;

    let rules_json = serde_json::to_string(&strategy.rules)
        .map_err(|e| format!("Failed to serialize rules: {}", e))?;

    let parameters_json = serde_json::to_string(&strategy.parameters)
        .map_err(|e| format!("Failed to serialize parameters: {}", e))?;

    let rows_affected = conn
        .execute(
            "UPDATE strategies 
             SET name = ?2, description = ?3, type = ?4, enabled = ?5, priority = ?6, 
                 rules_json = ?7, parameters_json = ?8, updated_at = ?9, author = ?10, version = ?11
             WHERE id = ?1",
            params![
                strategy.id,
                strategy.name,
                strategy.description,
                strategy.strategy_type.to_string(),
                strategy.enabled,
                strategy.priority,
                rules_json,
                parameters_json,
                strategy.updated_at.to_rfc3339(),
                strategy.author,
                strategy.version,
            ],
        )
        .map_err(|e| format!("Failed to update strategy: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Strategy not found: {}", strategy.id));
    }

    logger::info(
        LogTag::System,
        &format!(
            "Updated strategy: id={}, name={}",
            strategy.id, strategy.name
        ),
    );

    Ok(())
}

/// Delete a strategy
pub fn delete_strategy(strategy_id: &str) -> Result<(), String> {
    let conn = get_connection()?;

    let rows_affected = conn
        .execute("DELETE FROM strategies WHERE id = ?1", params![strategy_id])
        .map_err(|e| format!("Failed to delete strategy: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Strategy not found: {}", strategy_id));
    }

    logger::info(
        LogTag::System,
        &format!("Deleted strategy: id={}", strategy_id),
    );

    Ok(())
}

/// Get a strategy by ID
pub fn get_strategy(strategy_id: &str) -> Result<Option<Strategy>, String> {
    let conn = get_connection()?;

    let result = conn
        .query_row(
            "SELECT id, name, description, type, enabled, priority, rules_json, parameters_json, created_at, updated_at, author, version
             FROM strategies WHERE id = ?1",
            params![strategy_id],
            |row| {
                let rules_json: String = row.get(6)?;
                let parameters_json: String = row.get(7)?;
                let type_str: String = row.get(3)?;
                let created_at_str: String = row.get(8)?;
                let updated_at_str: String = row.get(9)?;

                Ok((rules_json, parameters_json, type_str, created_at_str, updated_at_str, row.get(0)?, row.get(1)?, row.get(2)?, row.get(4)?, row.get(5)?, row.get(10)?, row.get(11)?))
            },
        )
        .optional()
        .map_err(|e| format!("Failed to get strategy: {}", e))?;

    match result {
        Some((
            rules_json,
            parameters_json,
            type_str,
            created_at_str,
            updated_at_str,
            id,
            name,
            description,
            enabled,
            priority,
            author,
            version,
        )) => {
            let rules = serde_json::from_str(&rules_json)
                .map_err(|e| format!("Failed to deserialize rules: {}", e))?;
            let parameters = serde_json::from_str(&parameters_json)
                .map_err(|e| format!("Failed to deserialize parameters: {}", e))?;
            let strategy_type = match type_str.as_str() {
                "ENTRY" => StrategyType::Entry,
                "EXIT" => StrategyType::Exit,
                _ => return Err(format!("Invalid strategy type: {}", type_str)),
            };
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| format!("Failed to parse created_at: {}", e))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| format!("Failed to parse updated_at: {}", e))?
                .with_timezone(&Utc);

            Ok(Some(Strategy {
                id,
                name,
                description,
                strategy_type,
                enabled,
                priority,
                rules,
                parameters,
                created_at,
                updated_at,
                author,
                version,
            }))
        }
        None => Ok(None),
    }
}

/// Get all strategies
pub fn get_all_strategies() -> Result<Vec<Strategy>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, type, enabled, priority, rules_json, parameters_json, created_at, updated_at, author, version
             FROM strategies ORDER BY priority ASC, name ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let strategies = stmt
        .query_map([], |row| {
            let rules_json: String = row.get(6)?;
            let parameters_json: String = row.get(7)?;
            let type_str: String = row.get(3)?;
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;

            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                type_str,
                row.get(4)?,
                row.get(5)?,
                rules_json,
                parameters_json,
                created_at_str,
                updated_at_str,
                row.get(10)?,
                row.get(11)?,
            ))
        })
        .map_err(|e| format!("Failed to query strategies: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect strategies: {}", e))?;

    let mut result = Vec::new();
    for (
        id,
        name,
        description,
        type_str,
        enabled,
        priority,
        rules_json,
        parameters_json,
        created_at_str,
        updated_at_str,
        author,
        version,
    ) in strategies
    {
        let rules = serde_json::from_str(&rules_json)
            .map_err(|e| format!("Failed to deserialize rules for {}: {}", id, e))?;
        let parameters = serde_json::from_str(&parameters_json)
            .map_err(|e| format!("Failed to deserialize parameters for {}: {}", id, e))?;
        let strategy_type = match type_str.as_str() {
            "ENTRY" => StrategyType::Entry,
            "EXIT" => StrategyType::Exit,
            _ => continue,
        };
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| format!("Failed to parse created_at for {}: {}", id, e))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| format!("Failed to parse updated_at for {}: {}", id, e))?
            .with_timezone(&Utc);

        result.push(Strategy {
            id,
            name,
            description,
            strategy_type,
            enabled,
            priority,
            rules,
            parameters,
            created_at,
            updated_at,
            author,
            version,
        });
    }

    Ok(result)
}

/// Get enabled strategies by type
pub fn get_enabled_strategies(strategy_type: StrategyType) -> Result<Vec<Strategy>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, type, enabled, priority, rules_json, parameters_json, created_at, updated_at, author, version
             FROM strategies WHERE type = ?1 AND enabled = 1 ORDER BY priority ASC, name ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let strategies = stmt
        .query_map(params![strategy_type.to_string()], |row| {
            let rules_json: String = row.get(6)?;
            let parameters_json: String = row.get(7)?;
            let type_str: String = row.get(3)?;
            let created_at_str: String = row.get(8)?;
            let updated_at_str: String = row.get(9)?;

            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                type_str,
                row.get(4)?,
                row.get(5)?,
                rules_json,
                parameters_json,
                created_at_str,
                updated_at_str,
                row.get(10)?,
                row.get(11)?,
            ))
        })
        .map_err(|e| format!("Failed to query strategies: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect strategies: {}", e))?;

    let mut result = Vec::new();
    for (
        id,
        name,
        description,
        type_str,
        enabled,
        priority,
        rules_json,
        parameters_json,
        created_at_str,
        updated_at_str,
        author,
        version,
    ) in strategies
    {
        let rules = serde_json::from_str(&rules_json)
            .map_err(|e| format!("Failed to deserialize rules for {}: {}", id, e))?;
        let parameters = serde_json::from_str(&parameters_json)
            .map_err(|e| format!("Failed to deserialize parameters for {}: {}", id, e))?;
        let strategy_type = match type_str.as_str() {
            "ENTRY" => StrategyType::Entry,
            "EXIT" => StrategyType::Exit,
            _ => continue,
        };
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| format!("Failed to parse created_at for {}: {}", id, e))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| format!("Failed to parse updated_at for {}: {}", id, e))?
            .with_timezone(&Utc);

        result.push(Strategy {
            id,
            name,
            description,
            strategy_type,
            enabled,
            priority,
            rules,
            parameters,
            created_at,
            updated_at,
            author,
            version,
        });
    }

    Ok(result)
}

// =============================================================================
// PERFORMANCE TRACKING
// =============================================================================

/// Record strategy evaluation result
pub fn record_evaluation(result: &EvaluationResult, token_mint: &str) -> Result<(), String> {
    let conn = get_connection()?;

    let details_json = serde_json::to_string(&result.details)
        .map_err(|e| format!("Failed to serialize details: {}", e))?;

    conn.execute(
        "INSERT INTO strategy_performance (strategy_id, execution_time_ms, result, confidence, details_json, token_mint, execution_timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            result.strategy_id,
            result.execution_time_ms,
            result.result,
            result.confidence,
            details_json,
            token_mint,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| format!("Failed to record evaluation: {}", e))?;

    Ok(())
}

/// Get performance statistics for a strategy
pub fn get_strategy_performance(strategy_id: &str) -> Result<Option<StrategyPerformance>, String> {
    let conn = get_connection()?;

    let result = conn
        .query_row(
            "SELECT 
                COUNT(*) as total_evaluations,
                SUM(CASE WHEN result = 1 THEN 1 ELSE 0 END) as successful_signals,
                AVG(execution_time_ms) as avg_execution_time_ms,
                MAX(execution_timestamp) as last_evaluation
             FROM strategy_performance
             WHERE strategy_id = ?1",
            params![strategy_id],
            |row| {
                let total: u64 = row.get(0)?;
                let successful: u64 = row.get(1)?;
                let avg_time: f64 = row.get(2)?;
                let last_eval_str: String = row.get(3)?;
                Ok((total, successful, avg_time, last_eval_str))
            },
        )
        .optional()
        .map_err(|e| format!("Failed to get performance: {}", e))?;

    match result {
        Some((total_evaluations, successful_signals, avg_execution_time_ms, last_eval_str)) => {
            if total_evaluations == 0 {
                return Ok(None);
            }

            let last_evaluation = DateTime::parse_from_rfc3339(&last_eval_str)
                .map_err(|e| format!("Failed to parse timestamp: {}", e))?
                .with_timezone(&Utc);

            Ok(Some(StrategyPerformance {
                strategy_id: strategy_id.to_string(),
                total_evaluations,
                successful_signals,
                avg_execution_time_ms,
                last_evaluation,
            }))
        }
        None => Ok(None),
    }
}

// =============================================================================
// STRATEGY ASSIGNMENTS (Position to Strategy mapping)
// =============================================================================

/// Assign a strategy to a position
pub fn assign_strategy_to_position(position_id: &str, strategy_id: &str) -> Result<(), String> {
    let conn = get_connection()?;

    conn.execute(
        "INSERT OR REPLACE INTO strategy_assignments (position_id, strategy_id, assigned_at)
         VALUES (?1, ?2, ?3)",
        params![position_id, strategy_id, Utc::now().to_rfc3339()],
    )
    .map_err(|e| format!("Failed to assign strategy: {}", e))?;

    Ok(())
}

/// Get strategies assigned to a position
pub fn get_position_strategies(position_id: &str) -> Result<Vec<String>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare("SELECT strategy_id FROM strategy_assignments WHERE position_id = ?1")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let strategies = stmt
        .query_map(params![position_id], |row| row.get(0))
        .map_err(|e| format!("Failed to query assignments: {}", e))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| format!("Failed to collect assignments: {}", e))?;

    Ok(strategies)
}
