/// Events Database Module
///
/// High-performance SQLite database for persistent event storage.
/// Fresh schema (no migrations), split read/write pools, batched writes,
/// and keyset-optimized queries.
use crate::events::types::{ Event, EventCategory, Severity };
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use r2d2::{ Pool, PooledConnection };
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{ params, Connection, OptionalExtension, Result as SqliteResult };
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{ AtomicBool, Ordering };

// =============================================================================
// CONSTANTS
// =============================================================================

/// Database file path
const EVENTS_DB_PATH: &str = "data/events.db";

/// Maximum age for events (30 days)
const MAX_EVENT_AGE_DAYS: i64 = 30;

/// Connection pool configuration
const WRITE_POOL_MAX_SIZE: u32 = 2;
const READ_POOL_MAX_SIZE: u32 = 10;
const POOL_MIN_IDLE: u32 = 1;
const CONNECTION_TIMEOUT_MS: u64 = 30_000;

// =============================================================================
// DATABASE STRUCTURE
// =============================================================================

/// High-performance events database with split connection pools
pub struct EventsDatabase {
    write_pool: Pool<SqliteConnectionManager>,
    read_pool: Pool<SqliteConnectionManager>,
    database_path: String,
}

impl EventsDatabase {
    /// Create new EventsDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        // Database should be at data/events.db
        let data_dir = std::path::PathBuf::from("data");

        // Ensure data directory exists
        if !data_dir.exists() {
            std::fs
                ::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("events.db");
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
            .map_err(|e| format!("Failed to create events write pool: {}", e))?;

        // Create read pool
        let read_pool = Pool::builder()
            .max_size(READ_POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .build(read_manager)
            .map_err(|e| format!("Failed to create events read pool: {}", e))?;

        let mut db = EventsDatabase {
            write_pool,
            read_pool,
            database_path: database_path_str.clone(),
        };

        // Initialize database schema
        db.initialize_schema().await?;

        log(
            LogTag::System,
            "READY",
            &format!("Events database initialized at {}", database_path_str)
        );

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        // Use a write connection for initialization
        let conn = self.get_write_connection()?;

        // Configure connection for optimal performance
        conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set journal mode: {}", e))?;
        conn
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
        conn
            .pragma_update(None, "cache_size", 10000)
            .map_err(|e| format!("Failed to set cache size: {}", e))?;
        conn
            .pragma_update(None, "temp_store", "memory")
            .map_err(|e| format!("Failed to set temp store: {}", e))?;
        conn
            .busy_timeout(std::time::Duration::from_millis(30_000))
            .map_err(|e| format!("Failed to set busy timeout: {}", e))?;

        // Create main events table (fresh schema)
        conn
            .execute(
                "CREATE TABLE IF NOT EXISTS events (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                event_time      TEXT    NOT NULL,
                category        TEXT    NOT NULL,
                subtype         TEXT,
                severity        TEXT    NOT NULL,
                mint            TEXT,
                reference_id    TEXT,
                message_short   TEXT,
                json_payload    TEXT    NOT NULL,
                created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
            )",
                []
            )
            .map_err(|e| format!("Failed to create events table: {}", e))?;

        // Create optimized indexes
        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_category_time 
             ON events(category, event_time DESC)",
                []
            )
            .map_err(|e| format!("Failed to create category-time index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_reference_id 
             ON events(reference_id)",
                []
            )
            .map_err(|e| format!("Failed to create reference_id index: {}", e))?;

        conn
            .execute("CREATE INDEX IF NOT EXISTS idx_events_mint 
             ON events(mint)", [])
            .map_err(|e| format!("Failed to create mint index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_severity_time 
             ON events(severity, event_time DESC)",
                []
            )
            .map_err(|e| format!("Failed to create severity-time index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_created_at 
             ON events(created_at)",
                []
            )
            .map_err(|e| format!("Failed to create created_at index: {}", e))?;

        // Keyset and composite indexes for pagination and filters
        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_id_desc 
             ON events(id DESC)",
                []
            )
            .map_err(|e| format!("Failed to create id desc index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_category_severity_id 
             ON events(category, severity, id DESC)",
                []
            )
            .map_err(|e| format!("Failed to create category-severity-id index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_events_mint_id 
             ON events(mint, id DESC)",
                []
            )
            .map_err(|e| format!("Failed to create mint-id index: {}", e))?;

        Ok(())
    }

    /// Get write connection from pool
    fn get_write_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        let conn = self.write_pool
            .get()
            .map_err(|e| format!("Failed to get events write connection: {}", e))?;
        // Write-optimized PRAGMAs (database-level WAL already set during init)
        conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set journal mode: {}", e))?;
        conn
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
        conn
            .pragma_update(None, "cache_size", 10000)
            .map_err(|e| format!("Failed to set cache size: {}", e))?;
        conn
            .pragma_update(None, "temp_store", "memory")
            .map_err(|e| format!("Failed to set temp store: {}", e))?;
        conn
            .busy_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .map_err(|e| format!("Failed to set busy timeout: {}", e))?;
        Ok(conn)
    }

    /// Get read connection from pool
    fn get_read_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        let conn = self.read_pool
            .get()
            .map_err(|e| format!("Failed to get events read connection: {}", e))?;
        // Read-optimized PRAGMAs
        conn
            .pragma_update(None, "query_only", "1")
            .map_err(|e| format!("Failed to set query_only: {}", e))?;
        conn
            .pragma_update(None, "cache_size", 20000)
            .map_err(|e| format!("Failed to set cache size: {}", e))?;
        // 256MB mmap if supported
        let _ = conn.pragma_update(None, "mmap_size", 268435456i64);
        conn
            .busy_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .map_err(|e| format!("Failed to set busy timeout: {}", e))?;
        Ok(conn)
    }

    /// Insert a single event
    pub async fn insert_event(&self, event: &Event) -> Result<i64, String> {
        let conn = self.get_write_connection()?;

        let event_time_str = event.event_time.to_rfc3339();
        let category_str = event.category.to_string();
        let severity_str = event.severity.to_string();
        let payload_str = serde_json
            ::to_string(&event.payload)
            .map_err(|e| format!("Failed to serialize event payload: {}", e))?;
        let message_short: Option<String> = event.payload
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| {
                let mut m = s.to_string();
                if m.len() > 240 {
                    m.truncate(240);
                }
                m
            });

        let id = conn
            .execute(
                "INSERT INTO events (
                    event_time, category, subtype, severity, 
                    mint, reference_id, message_short, json_payload
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    event_time_str,
                    category_str,
                    event.subtype,
                    severity_str,
                    event.mint,
                    event.reference_id,
                    message_short,
                    payload_str
                ]
            )
            .map_err(|e| format!("Failed to insert event: {}", e))?;

        Ok(conn.last_insert_rowid())
    }

    /// Insert multiple events in a batch (more efficient)
    pub async fn insert_events(&self, events: &[Event]) -> Result<(), String> {
        if events.is_empty() {
            return Ok(());
        }

        let conn = self.get_write_connection()?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO events (
                        event_time, category, subtype, severity, 
                        mint, reference_id, message_short, json_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
                )
                .map_err(|e| format!("Failed to prepare insert statement: {}", e))?;

            for event in events {
                let event_time_str = event.event_time.to_rfc3339();
                let category_str = event.category.to_string();
                let severity_str = event.severity.to_string();
                let payload_str = serde_json
                    ::to_string(&event.payload)
                    .map_err(|e| format!("Failed to serialize event payload: {}", e))?;
                let message_short: Option<String> = event.payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        let mut m = s.to_string();
                        if m.len() > 240 {
                            m.truncate(240);
                        }
                        m
                    });

                stmt
                    .execute(
                        params![
                            event_time_str,
                            category_str,
                            event.subtype,
                            severity_str,
                            event.mint,
                            event.reference_id,
                            message_short,
                            payload_str
                        ]
                    )
                    .map_err(|e| format!("Failed to execute insert: {}", e))?;
            }
        }

        tx.commit().map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Get recent events, optionally filtered by category
    pub async fn get_recent_events(
        &self,
        category: Option<EventCategory>,
        limit: usize
    ) -> Result<Vec<Event>, String> {
        let conn = self.get_read_connection()?;

        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match category {
            Some(cat) =>
                (
                    "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events WHERE category = ?1 ORDER BY id DESC LIMIT ?2".to_string(),
                    vec![Box::new(cat.to_string()), Box::new(limit as i64)],
                ),
            None =>
                (
                    "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events ORDER BY id DESC LIMIT ?1".to_string(),
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
                                    rusqlite::types::Type::Text
                                )
                            })?
                            .with_timezone(&Utc),
                        category: EventCategory::from_string(&row.get::<_, String>(2)?),
                        subtype: row.get(3)?,
                        severity: Severity::from_string(&row.get::<_, String>(4)?),
                        mint: row.get(5)?,
                        reference_id: row.get(6)?,
                        payload: serde_json
                            ::from_str(&row.get::<_, String>(7)?)
                            .map_err(|_| {
                                rusqlite::Error::InvalidColumnType(
                                    7,
                                    "json_payload".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            })?,
                        created_at: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                    })
                }
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
        limit: usize
    ) -> Result<Vec<Event>, String> {
        let conn = self.get_read_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
              FROM events WHERE reference_id = ?1 ORDER BY id DESC LIMIT ?2"
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
                                rusqlite::types::Type::Text
                            )
                        })?
                        .with_timezone(&Utc),
                    category: EventCategory::from_string(&row.get::<_, String>(2)?),
                    subtype: row.get(3)?,
                    severity: Severity::from_string(&row.get::<_, String>(4)?),
                    mint: row.get(5)?,
                    reference_id: row.get(6)?,
                    payload: serde_json
                        ::from_str(&row.get::<_, String>(7)?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                7,
                                "json_payload".to_string(),
                                rusqlite::types::Type::Text
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
        let conn = self.get_read_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
              FROM events WHERE mint = ?1 ORDER BY id DESC LIMIT ?2"
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
                                rusqlite::types::Type::Text
                            )
                        })?
                        .with_timezone(&Utc),
                    category: EventCategory::from_string(&row.get::<_, String>(2)?),
                    subtype: row.get(3)?,
                    severity: Severity::from_string(&row.get::<_, String>(4)?),
                    mint: row.get(5)?,
                    reference_id: row.get(6)?,
                    payload: serde_json
                        ::from_str(&row.get::<_, String>(7)?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                7,
                                "json_payload".to_string(),
                                rusqlite::types::Type::Text
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
        since_hours: u64
    ) -> Result<HashMap<String, u64>, String> {
        let conn = self.get_read_connection()?;

        let cutoff_time = Utc::now() - chrono::Duration::hours(since_hours as i64);
        let cutoff_str = cutoff_time.to_rfc3339();

        let mut stmt = conn
            .prepare(
                "SELECT category, COUNT(*) as count 
                 FROM events 
                 WHERE event_time >= ?1 
                 GROUP BY category"
            )
            .map_err(|e| format!("Failed to prepare count query: {}", e))?;

        let count_iter = stmt
            .query_map(params![cutoff_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .map_err(|e| format!("Failed to execute count query: {}", e))?;

        let mut counts = HashMap::new();
        for count_result in count_iter {
            let (category, count) = count_result.map_err(|e|
                format!("Failed to parse count row: {}", e)
            )?;
            counts.insert(category, count);
        }

        Ok(counts)
    }

    /// Cleanup old events (older than MAX_EVENT_AGE_DAYS)
    pub async fn cleanup_old_events(&self) -> Result<usize, String> {
        let conn = self.get_write_connection()?;

        let cutoff_time = Utc::now() - chrono::Duration::days(MAX_EVENT_AGE_DAYS);
        let cutoff_str = cutoff_time.to_rfc3339();

        let deleted_count = conn
            .execute("DELETE FROM events WHERE event_time < ?1", params![cutoff_str])
            .map_err(|e| format!("Failed to delete old events: {}", e))?;

        if deleted_count > 0 {
            log(LogTag::System, "CLEANUP", &format!("Cleaned up {} old events", deleted_count));
        }

        Ok(deleted_count)
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>, String> {
        let conn = self.get_read_connection()?;

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
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to get 24h event count: {}", e))?;
        stats.insert("events_24h".to_string(), events_24h);

        Ok(stats)
    }
}

impl EventsDatabase {
    /// Get events with ID greater than cursor (keyset forward)
    pub async fn get_events_since(
        &self,
        after_id: i64,
        limit: usize,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>
    ) -> Result<Vec<Event>, String> {
        let conn = self.get_read_connection()?;
        let mut query = String::from(
            "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at FROM events WHERE id > ?1"
        );
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(after_id)];
        let mut idx = 2;
        if let Some(cat) = category {
            query.push_str(&format!(" AND category = ?{}", idx));
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(&format!(" AND severity = ?{}", idx));
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(&format!(" AND mint = ?{}", idx));
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(&format!(" AND reference_id = ?{}", idx));
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        query.push_str(" ORDER BY id ASC LIMIT ?");
        bind.push(Box::new(limit as i64));

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare since query: {}", e))?;
        let rows = stmt
            .query_map(
                bind
                    .iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| {
                    Ok(Event {
                        id: Some(row.get(0)?),
                        event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                            .map_err(|_|
                                rusqlite::Error::InvalidColumnType(
                                    1,
                                    "event_time".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            )?
                            .with_timezone(&Utc),
                        category: EventCategory::from_string(&row.get::<_, String>(2)?),
                        subtype: row.get(3)?,
                        severity: Severity::from_string(&row.get::<_, String>(4)?),
                        mint: row.get(5)?,
                        reference_id: row.get(6)?,
                        payload: serde_json
                            ::from_str(&row.get::<_, String>(7)?)
                            .map_err(|_|
                                rusqlite::Error::InvalidColumnType(
                                    7,
                                    "json_payload".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            )?,
                        created_at: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                    })
                }
            )
            .map_err(|e| format!("Failed to execute since query: {}", e))?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r.map_err(|e| format!("Failed to parse row: {}", e))?);
        }
        Ok(events)
    }

    /// Get events with ID less than cursor (keyset backward)
    pub async fn get_events_before(
        &self,
        before_id: i64,
        limit: usize,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>
    ) -> Result<Vec<Event>, String> {
        let conn = self.get_read_connection()?;
        let mut query = String::from(
            "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at FROM events WHERE id < ?1"
        );
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(before_id)];
        let mut idx = 2;
        if let Some(cat) = category {
            query.push_str(&format!(" AND category = ?{}", idx));
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(&format!(" AND severity = ?{}", idx));
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(&format!(" AND mint = ?{}", idx));
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(&format!(" AND reference_id = ?{}", idx));
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        query.push_str(" ORDER BY id DESC LIMIT ?");
        bind.push(Box::new(limit as i64));

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare before query: {}", e))?;
        let rows = stmt
            .query_map(
                bind
                    .iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| {
                    Ok(Event {
                        id: Some(row.get(0)?),
                        event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                            .map_err(|_|
                                rusqlite::Error::InvalidColumnType(
                                    1,
                                    "event_time".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            )?
                            .with_timezone(&Utc),
                        category: EventCategory::from_string(&row.get::<_, String>(2)?),
                        subtype: row.get(3)?,
                        severity: Severity::from_string(&row.get::<_, String>(4)?),
                        mint: row.get(5)?,
                        reference_id: row.get(6)?,
                        payload: serde_json
                            ::from_str(&row.get::<_, String>(7)?)
                            .map_err(|_|
                                rusqlite::Error::InvalidColumnType(
                                    7,
                                    "json_payload".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            )?,
                        created_at: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                    })
                }
            )
            .map_err(|e| format!("Failed to execute before query: {}", e))?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r.map_err(|e| format!("Failed to parse row: {}", e))?);
        }
        Ok(events)
    }

    /// Get latest N events and return also the max id
    pub async fn get_events_head(
        &self,
        limit: usize,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>
    ) -> Result<(Vec<Event>, i64), String> {
        let conn = self.get_read_connection()?;
        let mut query = String::from(
            "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at FROM events"
        );
        let mut where_added = false;
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1;
        if let Some(cat) = category {
            query.push_str(
                &format!(
                    "{} category = ?{}",
                    if where_added {
                        " AND"
                    } else {
                        where_added = true;
                        " WHERE"
                    },
                    idx
                )
            );
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(
                &format!(
                    "{} severity = ?{}",
                    if where_added {
                        " AND"
                    } else {
                        where_added = true;
                        " WHERE"
                    },
                    idx
                )
            );
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(
                &format!(
                    "{} mint = ?{}",
                    if where_added {
                        " AND"
                    } else {
                        where_added = true;
                        " WHERE"
                    },
                    idx
                )
            );
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(
                &format!(
                    "{} reference_id = ?{}",
                    if where_added {
                        " AND"
                    } else {
                        where_added = true;
                        " WHERE"
                    },
                    idx
                )
            );
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        query.push_str(" ORDER BY id DESC LIMIT ?");
        bind.push(Box::new(limit as i64));

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare head query: {}", e))?;
        let rows = stmt
            .query_map(
                bind
                    .iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| {
                    Ok(Event {
                        id: Some(row.get(0)?),
                        event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                            .map_err(|_|
                                rusqlite::Error::InvalidColumnType(
                                    1,
                                    "event_time".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            )?
                            .with_timezone(&Utc),
                        category: EventCategory::from_string(&row.get::<_, String>(2)?),
                        subtype: row.get(3)?,
                        severity: Severity::from_string(&row.get::<_, String>(4)?),
                        mint: row.get(5)?,
                        reference_id: row.get(6)?,
                        payload: serde_json
                            ::from_str(&row.get::<_, String>(7)?)
                            .map_err(|_|
                                rusqlite::Error::InvalidColumnType(
                                    7,
                                    "json_payload".to_string(),
                                    rusqlite::types::Type::Text
                                )
                            )?,
                        created_at: row
                            .get::<_, Option<String>>(8)?
                            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                            .map(|dt| dt.with_timezone(&Utc)),
                    })
                }
            )
            .map_err(|e| format!("Failed to execute head query: {}", e))?;

        let mut events = Vec::new();
        let mut max_id: i64 = 0;
        for r in rows {
            let e = r.map_err(|e| format!("Failed to parse row: {}", e))?;
            if let Some(id) = e.id {
                if id > max_id {
                    max_id = id;
                }
            }
            events.push(e);
        }
        Ok((events, max_id))
    }
}
