//! Events database module.
//!
//! High-performance SQLite database for persistent event storage.
//! Fresh schema (no migrations), split read/write pools, batched writes,
//! and keyset-optimized queries.
//!
//! Submodules:
//! - `pagination`: Keyset cursor-based pagination and filtered counting

mod pagination;

use crate::database;
use crate::errors::{DataError, DatabaseError};
use crate::events::types::{Event, EventCategory, Severity};
use crate::events::{Error, Result};
use crate::logger::{self, LogTag};
use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::collections::HashMap;
use std::time::Duration;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum age for events (30 days)
const MAX_EVENT_AGE_DAYS: i64 = 30;

/// Connection pool configuration
const WRITE_POOL_MAX_SIZE: u32 = 2;
const READ_POOL_MAX_SIZE: u32 = 4;
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
    pub async fn new() -> Result<Self> {
        let database_path = crate::paths::get_events_db_path();
        let database_path_str = database_path.to_string_lossy().to_string();

        // Configure connection managers with centralized PRAGMAs
        let write_manager = SqliteConnectionManager::file(&database_path)
            .with_init(|c| database::configure_connection(c, database::EVENTS_WRITE_DB));
        let read_manager = SqliteConnectionManager::file(&database_path)
            .with_init(|c| database::configure_connection(c, database::EVENTS_READ_DB));

        // Create write pool
        let write_pool = Pool::builder()
            .max_size(WRITE_POOL_MAX_SIZE)
            .min_idle(Some(POOL_MIN_IDLE))
            .connection_timeout(Duration::from_millis(CONNECTION_TIMEOUT_MS))
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(write_manager)
            .map_err(|e| {
                Error::Database(DatabaseError::Connection {
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
                Error::Database(DatabaseError::Connection {
                    message: e.to_string(),
                })
            })?;

        let mut db = EventsDatabase {
            write_pool,
            read_pool,
            database_path: database_path_str.clone(),
        };

        // Initialize database schema
        db.initialize_schema().await?;

        logger::info(
            LogTag::System,
            &format!("Events database initialized at {database_path_str}"),
        );

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<()> {
        // Use a write connection for initialization
        let conn = self.get_write_connection()?;

        // Create main events table (fresh schema)
        conn.execute(
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
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events table".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Create optimized indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_category_time 
             ON events(category, event_time DESC)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events category-time index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_reference_id 
             ON events(reference_id)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events reference id index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_mint 
             ON events(mint)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events mint index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_severity_time 
             ON events(severity, event_time DESC)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events severity-time index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_created_at 
             ON events(created_at)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events created at index".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Keyset and composite indexes for pagination and filters
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_id_desc 
             ON events(id DESC)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events id descending index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_category_severity_id 
             ON events(category, severity, id DESC)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events category severity id index".to_owned(),
                message: e.to_string(),
            })
        })?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_mint_id 
             ON events(mint, id DESC)",
            [],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "create events mint id index".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Get write connection from pool
    fn get_write_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.write_pool.get().map_err(|e| {
            Error::Database(DatabaseError::Connection {
                message: e.to_string(),
            })
        })
    }

    /// Get read connection from pool
    pub(crate) fn get_read_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.read_pool.get().map_err(|e| {
            Error::Database(DatabaseError::Connection {
                message: e.to_string(),
            })
        })
    }

    /// Insert a single event
    pub async fn insert_event(&self, event: &Event) -> Result<i64> {
        let conn = self.get_write_connection()?;

        let event_time_str = event.event_time.to_rfc3339();
        let category_str = event.category.to_string();
        let severity_str = event.severity.to_string();
        let payload_str = serde_json::to_string(&event.payload).map_err(|e| {
            Error::Data(DataError::ParseError {
                data_type: "event payload".to_owned(),
                error: e.to_string(),
            })
        })?;
        let message_short: Option<String> = event
            .payload
            .get("message")
            .and_then(|v| v.as_str())
            .map(|s| {
                if s.len() > 240 {
                    // Find a valid UTF-8 boundary at or before 240 bytes
                    let mut end = 240;
                    while end > 0 && !s.is_char_boundary(end) {
                        end -= 1;
                    }
                    s[..end].to_string()
                } else {
                    s.to_string()
                }
            });

        let _id = conn
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
                ],
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "insert event".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(conn.last_insert_rowid())
    }

    /// Insert multiple events in a batch (more efficient)
    pub async fn insert_events(&self, events: &mut [Event]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let conn = self.get_write_connection()?;

        let tx = conn.unchecked_transaction().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "start events insert transaction".to_owned(),
                message: e.to_string(),
            })
        })?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO events (
                        event_time, category, subtype, severity, 
                        mint, reference_id, message_short, json_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .map_err(|e| {
                    Error::Database(DatabaseError::Query {
                        operation: "prepare event insert".to_owned(),
                        message: e.to_string(),
                    })
                })?;

            for event in events.iter_mut() {
                let event_time_str = event.event_time.to_rfc3339();
                let category_str = event.category.to_string();
                let severity_str = event.severity.to_string();
                let payload_str = serde_json::to_string(&event.payload).map_err(|e| {
                    Error::Data(DataError::ParseError {
                        data_type: "event payload".to_owned(),
                        error: e.to_string(),
                    })
                })?;
                let message_short: Option<String> = event
                    .payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        if s.len() > 240 {
                            let mut end = 240;
                            while end > 0 && !s.is_char_boundary(end) {
                                end -= 1;
                            }
                            s[..end].to_string()
                        } else {
                            s.to_string()
                        }
                    });

                stmt.execute(params![
                    event_time_str,
                    category_str,
                    event.subtype.clone(),
                    severity_str,
                    event.mint.clone(),
                    event.reference_id.clone(),
                    message_short,
                    payload_str
                ])
                .map_err(|e| {
                    Error::Database(DatabaseError::Query {
                        operation: "insert event batch item".to_owned(),
                        message: e.to_string(),
                    })
                })?;

                let inserted_id = tx.last_insert_rowid();
                event.id = Some(inserted_id);
                if event.created_at.is_none() {
                    event.created_at = Some(Utc::now());
                }
            }
        }

        tx.commit().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "commit events insert transaction".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Get recent events, optionally filtered by category
    pub async fn get_recent_events(
        &self,
        category: Option<EventCategory>,
        limit: usize,
    ) -> Result<Vec<Event>> {
        let conn = self.get_read_connection()?;

        let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match category {
            Some(cat) =>
                (
                    "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events WHERE category = ?1 ORDER BY id DESC LIMIT ?2".to_owned(),
                    vec![Box::new(cat.to_string()), Box::new(limit as i64)],
                ),
            None =>
                (
                    "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
                 FROM events ORDER BY id DESC LIMIT ?1".to_owned(),
                    vec![Box::new(limit as i64)],
                ),
        };

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "prepare recent events query".to_owned(),
                message: e.to_string(),
            })
        })?;

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
                                    "event_time".to_owned(),
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
                                "json_payload".to_owned(),
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
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute recent events query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut events = Vec::new();
        for event_result in event_iter {
            events.push(event_result.map_err(|e| Error::RowDecode {
                column: "event row",
                detail: e.to_string(),
            })?);
        }

        Ok(events)
    }

    /// Get events by reference ID (tx signature, pool address, etc.)
    pub async fn get_events_by_reference(
        &self,
        reference_id: &str,
        limit: usize,
    ) -> Result<Vec<Event>> {
        let conn = self.get_read_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
              FROM events WHERE reference_id = ?1 ORDER BY id DESC LIMIT ?2"
            )
            .map_err(|e| Error::Database(DatabaseError::Query {
                operation: "prepare events by reference query".to_owned(),
                message: e.to_string(),
            }))?;

        let event_iter = stmt
            .query_map(params![reference_id, limit as i64], |row| {
                Ok(Event {
                    id: Some(row.get(0)?),
                    event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                1,
                                "event_time".to_owned(),
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
                            "json_payload".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    created_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute events by reference query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut events = Vec::new();
        for event_result in event_iter {
            events.push(event_result.map_err(|e| Error::RowDecode {
                column: "event row",
                detail: e.to_string(),
            })?);
        }

        Ok(events)
    }

    /// Get events by token mint
    pub async fn get_events_by_mint(&self, mint: &str, limit: usize) -> Result<Vec<Event>> {
        let conn = self.get_read_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at
              FROM events WHERE mint = ?1 ORDER BY id DESC LIMIT ?2"
            )
            .map_err(|e| Error::Database(DatabaseError::Query {
                operation: "prepare events by mint query".to_owned(),
                message: e.to_string(),
            }))?;

        let event_iter = stmt
            .query_map(params![mint, limit as i64], |row| {
                Ok(Event {
                    id: Some(row.get(0)?),
                    event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                1,
                                "event_time".to_owned(),
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
                            "json_payload".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    created_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute events by mint query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut events = Vec::new();
        for event_result in event_iter {
            events.push(event_result.map_err(|e| Error::RowDecode {
                column: "event row",
                detail: e.to_string(),
            })?);
        }

        Ok(events)
    }

    /// Get event counts by category for the last N hours
    pub async fn get_event_counts_by_category(
        &self,
        since_hours: u64,
    ) -> Result<HashMap<String, u64>> {
        let conn = self.get_read_connection()?;

        let cutoff_time = Utc::now() - chrono::Duration::hours(since_hours as i64);
        let cutoff_str = cutoff_time.to_rfc3339();

        let mut stmt = conn
            .prepare(
                "SELECT category, COUNT(*) as count 
                 FROM events 
                 WHERE event_time >= ?1 
                 GROUP BY category",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "prepare event category count query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let count_iter = stmt
            .query_map(params![cutoff_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute event category count query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut counts = HashMap::new();
        for count_result in count_iter {
            let (category, count) = count_result.map_err(|e| Error::RowDecode {
                column: "category count",
                detail: e.to_string(),
            })?;
            counts.insert(category, count);
        }

        Ok(counts)
    }

    /// Cleanup old events (older than MAX_EVENT_AGE_DAYS)
    pub async fn cleanup_old_events(&self) -> Result<usize> {
        let conn = self.get_write_connection()?;

        let cutoff_time = Utc::now() - chrono::Duration::days(MAX_EVENT_AGE_DAYS);
        let cutoff_str = cutoff_time.to_rfc3339();

        let deleted_count = conn
            .execute(
                "DELETE FROM events WHERE event_time < ?1",
                params![cutoff_str],
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "delete old events".to_owned(),
                    message: e.to_string(),
                })
            })?;

        if deleted_count > 0 {
            logger::info(
                LogTag::System,
                &format!("Cleaned up {deleted_count} old events"),
            );
        }

        Ok(deleted_count)
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, i64>> {
        let conn = self.get_read_connection()?;

        let mut stats = HashMap::new();

        // Total event count
        let total_events: i64 = conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "get total event count".to_owned(),
                    message: e.to_string(),
                })
            })?;
        stats.insert("total_events".to_owned(), total_events);

        // Database file size
        if let Ok(metadata) = std::fs::metadata(&self.database_path) {
            stats.insert("db_size_bytes".to_owned(), metadata.len() as i64);
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
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "get 24 hour event count".to_owned(),
                    message: e.to_string(),
                })
            })?;
        stats.insert("events_24h".to_owned(), events_24h);

        Ok(stats)
    }
}
