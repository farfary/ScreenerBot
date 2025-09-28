/// Events Database Module
///
/// High-performance SQLite database for persistent event storage.
/// Uses connection pooling, batched writes, and optimized schemas
/// following project patterns.
use crate::events::types::{Event, EventCategory, Severity};
use crate::logger::{log, LogTag};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Database file path
const EVENTS_DB_PATH: &str = "data/events.db";

/// Database schema version for migrations
const EVENTS_SCHEMA_VERSION: u32 = 1;

/// Maximum age for events (30 days)
const MAX_EVENT_AGE_DAYS: i64 = 30;

/// Connection pool configuration
const POOL_MAX_SIZE: u32 = 5;
const POOL_MIN_IDLE: u32 = 1;
const CONNECTION_TIMEOUT_MS: u64 = 30_000;

/// Static flag to track initialization
static EVENTS_DB_INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// =============================================================================
// DATABASE STRUCTURE
// =============================================================================

/// High-performance events database with connection pooling
pub struct EventsDatabase {
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
}

impl EventsDatabase {
    /// Create new EventsDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        // Database should be at data/events.db
        let data_dir = std::path::PathBuf::from("data");

        // Ensure data directory exists
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("events.db");
        let database_path_str = database_path.to_string_lossy().to_string();

        // Only log detailed initialization on first database creation
        let is_first_init = !EVENTS_DB_INITIALIZED.load(Ordering::Relaxed);
        if is_first_init {
            log(
                LogTag::System,
                "INIT",
                &format!("Initializing events database at: {}", database_path_str),
            );
        }

        // Configure connection manager
        let manager = SqliteConnectionManager::file(&database_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .build(manager)
            .map_err(|e| format!("Failed to create events connection pool: {}", e))?;

        let mut db = EventsDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: EVENTS_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema(is_first_init).await?;

        if is_first_init {
            log(
                LogTag::System,
                "READY",
                "Events database initialized successfully",
            );
            EVENTS_DB_INITIALIZED.store(true, Ordering::Relaxed);
        }

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self, log_initialization: bool) -> Result<(), String> {
        let conn = self.get_connection()?;

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

        // Create main events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                event_time      TEXT    NOT NULL,
                category        TEXT    NOT NULL,
                subtype         TEXT,
                severity        TEXT    NOT NULL,
                mint            TEXT,
                reference_id    TEXT,
                json_payload    TEXT    NOT NULL,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(|e| format!("Failed to create events table: {}", e))?;

        // Create optimized indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_category_time 
             ON events(category, event_time DESC)",
            [],
        )
        .map_err(|e| format!("Failed to create category-time index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_reference_id 
             ON events(reference_id)",
            [],
        )
        .map_err(|e| format!("Failed to create reference_id index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_mint 
             ON events(mint)",
            [],
        )
        .map_err(|e| format!("Failed to create mint index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_severity_time 
             ON events(severity, event_time DESC)",
            [],
        )
        .map_err(|e| format!("Failed to create severity-time index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_created_at 
             ON events(created_at)",
            [],
        )
        .map_err(|e| format!("Failed to create created_at index: {}", e))?;

        // Create schema version table for future migrations
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events_schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(|e| format!("Failed to create schema version table: {}", e))?;

        // Insert or update schema version
        conn.execute(
            "INSERT OR REPLACE INTO events_schema_version (version) VALUES (?1)",
            params![self.schema_version],
        )
        .map_err(|e| format!("Failed to update schema version: {}", e))?;

        if log_initialization {
            log(
                LogTag::System,
                "DB_SCHEMA",
                &format!("Events database schema v{} ready", self.schema_version),
            );
        }

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get events database connection: {}", e))
    }

    /// Insert a single event
    pub async fn insert_event(&self, event: &Event) -> Result<i64, String> {
        let conn = self.get_connection()?;

        let event_time_str = event.event_time.to_rfc3339();
        let category_str = event.category.to_string();
        let severity_str = event.severity.to_string();
        let payload_str = serde_json::to_string(&event.payload)
            .map_err(|e| format!("Failed to serialize event payload: {}", e))?;

        let id = conn
            .execute(
                "INSERT INTO events (
                    event_time, category, subtype, severity, 
                    mint, reference_id, json_payload
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    event_time_str,
                    category_str,
                    event.subtype,
                    severity_str,
                    event.mint,
                    event.reference_id,
                    payload_str
                ],
            )
            .map_err(|e| format!("Failed to insert event: {}", e))?;

        Ok(conn.last_insert_rowid())
    }

    /// Insert multiple events in a batch (more efficient)
    pub async fn insert_events(&self, events: &[Event]) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }

        let conn = self.get_connection()?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO events (
                        event_time, category, subtype, severity, 
                        mint, reference_id, json_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| format!("Failed to prepare insert statement: {}", e))?;

            for event in events {
                let event_time_str = event.event_time.to_rfc3339();
                let category_str = event.category.to_string();
                let severity_str = event.severity.to_string();
                let payload_str = serde_json::to_string(&event.payload)
                    .map_err(|e| format!("Failed to serialize event payload: {}", e))?;

                stmt.execute(params![
                    event_time_str,
                    category_str,
                    event.subtype,
                    severity_str,
                    event.mint,
                    event.reference_id,
                    payload_str
                ])
                .map_err(|e| format!("Failed to execute insert: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Get recent events, optionally filtered by category
    pub async fn get_recent_events(
        &self,
        category: Option<EventCategory>,
        limit: usize,
    ) -> Result<Vec<Event>, String> {
        let conn = self.get_connection()?;

        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match category {
            Some(cat) =>
                (
                    "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events WHERE category = ?1 ORDER BY event_time DESC LIMIT ?2".to_string(),
                    vec![Box::new(cat.to_string()), Box::new(limit as i64)],
                ),
            None =>
                (
                    "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events ORDER BY event_time DESC LIMIT ?1".to_string(),
                    vec![Box::new(limit as i64)],
                ),
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

        let event_iter = stmt
            .query_map(
                params
                    .iter()
                    .map(|p| p.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| {
                    Ok(Event {
                        id: Some(row.get(0)?),
                        event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                            .map_err(|_| {
                                rusqlite::Error::InvalidColumnType(
                                    1,
                                    "event_time".to_string(),
                                    rusqlite::types::Type::Text,
                                )
                            })?
                            .with_timezone(&Utc),
                        category: EventCategory::from_string(&row.get::<_, String>(2)?),
                        subtype: row.get(3)?,
                        severity: Severity::from_string(&row.get::<_, String>(4)?),
                        mint: row.get(5)?,
                        reference_id: row.get(6)?,
                        payload: serde_json::from_str(&row.get::<_, String>(7)?).map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                7,
                                "json_payload".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?,
                        created_at: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                    })
                },
            )
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut events = Vec::new();
        for event_result in event_iter {
            events.push(event_result.map_err(|e| format!("Failed to parse event row: {}", e))?);
        }

        Ok(events)
    }

    /// Get events by reference ID (tx signature, pool address, etc.)
    pub async fn get_events_by_reference(
        &self,
        reference_id: &str,
        limit: usize,
    ) -> Result<Vec<Event>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events WHERE reference_id = ?1 ORDER BY event_time DESC LIMIT ?2"
            )
            .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

        let event_iter = stmt
            .query_map(params![reference_id, limit as i64], |row| {
                Ok(Event {
                    id: Some(row.get(0)?),
                    event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                1,
                                "event_time".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?
                        .with_timezone(&Utc),
                    category: EventCategory::from_string(&row.get::<_, String>(2)?),
                    subtype: row.get(3)?,
                    severity: Severity::from_string(&row.get::<_, String>(4)?),
                    mint: row.get(5)?,
                    reference_id: row.get(6)?,
                    payload: serde_json::from_str(&row.get::<_, String>(7)?).map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "json_payload".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    created_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut events = Vec::new();
        for event_result in event_iter {
            events.push(event_result.map_err(|e| format!("Failed to parse event row: {}", e))?);
        }

        Ok(events)
    }

    /// Get events by token mint
    pub async fn get_events_by_mint(&self, mint: &str, limit: usize) -> Result<Vec<Event>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events WHERE mint = ?1 ORDER BY event_time DESC LIMIT ?2"
            )
            .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

        let event_iter = stmt
            .query_map(params![mint, limit as i64], |row| {
                Ok(Event {
                    id: Some(row.get(0)?),
                    event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                1,
                                "event_time".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?
                        .with_timezone(&Utc),
                    category: EventCategory::from_string(&row.get::<_, String>(2)?),
                    subtype: row.get(3)?,
                    severity: Severity::from_string(&row.get::<_, String>(4)?),
                    mint: row.get(5)?,
                    reference_id: row.get(6)?,
                    payload: serde_json::from_str(&row.get::<_, String>(7)?).map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "json_payload".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    created_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        let mut events = Vec::new();
        for event_result in event_iter {
            events.push(event_result.map_err(|e| format!("Failed to parse event row: {}", e))?);
        }

        Ok(events)
    }

    /// Get event counts by category for the last N hours
    pub async fn get_event_counts_by_category(
        &self,
        since_hours: u64,
    ) -> Result<HashMap<String, u64>, String> {
        let conn = self.get_connection()?;

        let cutoff_time = Utc::now() - chrono::Duration::hours(since_hours as i64);
        let cutoff_str = cutoff_time.to_rfc3339();

        let mut stmt = conn
            .prepare(
                "SELECT category, COUNT(*) as count 
                 FROM events 
                 WHERE event_time >= ?1 
                 GROUP BY category",
            )
            .map_err(|e| format!("Failed to prepare count query: {}", e))?;

        let count_iter = stmt
            .query_map(params![cutoff_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .map_err(|e| format!("Failed to execute count query: {}", e))?;

        let mut counts = HashMap::new();
        for count_result in count_iter {
            let (category, count) =
                count_result.map_err(|e| format!("Failed to parse count row: {}", e))?;
            counts.insert(category, count);
        }

        Ok(counts)
    }

    /// Cleanup old events (older than MAX_EVENT_AGE_DAYS)
    pub async fn cleanup_old_events(&self) -> Result<usize, String> {
        let conn = self.get_connection()?;

        let cutoff_time = Utc::now() - chrono::Duration::days(MAX_EVENT_AGE_DAYS);
        let cutoff_str = cutoff_time.to_rfc3339();

        let deleted_count = conn
            .execute(
                "DELETE FROM events WHERE event_time < ?1",
                params![cutoff_str],
            )
            .map_err(|e| format!("Failed to delete old events: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::System,
                "CLEANUP",
                &format!("Cleaned up {} old events", deleted_count),
            );
        }

        Ok(deleted_count)
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>, String> {
        let conn = self.get_connection()?;

        let mut stats = HashMap::new();

        // Total event count
        let total_events: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .map_err(|e| format!("Failed to get total event count: {}", e))?;
        stats.insert("total_events".to_string(), total_events);

        // Database file size
        if let Ok(metadata) = std::fs::metadata(&self.database_path) {
            stats.insert("db_size_bytes".to_string(), metadata.len() as i64);
        }

        // Events in last 24 hours
        let cutoff_24h = Utc::now() - chrono::Duration::hours(24);
        let cutoff_24h_str = cutoff_24h.to_rfc3339();
        let events_24h: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE event_time >= ?1",
                params![cutoff_24h_str],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get 24h event count: {}", e))?;
        stats.insert("events_24h".to_string(), events_24h);

        Ok(stats)
    }
}
