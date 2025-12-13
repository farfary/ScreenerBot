//! SQLite database operations for RPC statistics

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::path::{Path, PathBuf};

use super::types::*;
use crate::rpc::types::{CircuitState, ProviderKind};

/// Database path for RPC stats
pub fn get_rpc_stats_db_path() -> PathBuf {
    crate::paths::get_data_directory().join("rpc_stats.db")
}

/// RPC statistics database
pub struct RpcStatsDatabase {
    pool: Pool<SqliteConnectionManager>,
}

impl RpcStatsDatabase {
    /// Create/open database
    pub fn new() -> Result<Self, String> {
        let db_path = get_rpc_stats_db_path();
        Self::open(&db_path)
    }

    /// Open database at specific path
    pub fn open(path: &Path) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

        let db = Self { pool };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Get connection from pool
    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self.conn()?;

        conn.execute_batch(
            r#"
            -- Enable WAL mode for better concurrency
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = 10000;
            PRAGMA temp_store = MEMORY;
            PRAGMA busy_timeout = 30000;

            -- Sessions table
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                total_calls INTEGER DEFAULT 0,
                total_errors INTEGER DEFAULT 0,
                is_current INTEGER DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_current ON sessions(is_current);
            CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC);

            -- Providers table
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                url_masked TEXT NOT NULL,
                kind TEXT NOT NULL,
                priority INTEGER DEFAULT 100,
                enabled INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_providers_kind ON providers(kind);

            -- RPC calls table (time-series)
            CREATE TABLE IF NOT EXISTS calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                method TEXT NOT NULL,
                success INTEGER NOT NULL,
                latency_ms INTEGER NOT NULL,
                error_code INTEGER,
                error_message TEXT,
                was_retried INTEGER DEFAULT 0,
                retry_count INTEGER DEFAULT 0,
                was_rate_limited INTEGER DEFAULT 0,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
            CREATE INDEX IF NOT EXISTS idx_calls_session_time ON calls(session_id, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_calls_provider_time ON calls(provider_id, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_calls_method ON calls(method);
            CREATE INDEX IF NOT EXISTS idx_calls_timestamp ON calls(timestamp DESC);

            -- Minute buckets table (aggregated)
            CREATE TABLE IF NOT EXISTS minute_buckets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                provider_id TEXT,
                minute_start TEXT NOT NULL,
                call_count INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                error_count INTEGER DEFAULT 0,
                rate_limit_count INTEGER DEFAULT 0,
                latency_sum_ms INTEGER DEFAULT 0,
                latency_min_ms INTEGER,
                latency_max_ms INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(id),
                UNIQUE (session_id, provider_id, minute_start)
            );
            CREATE INDEX IF NOT EXISTS idx_minute_buckets_time ON minute_buckets(minute_start DESC);

            -- Provider health table
            CREATE TABLE IF NOT EXISTS provider_health (
                provider_id TEXT PRIMARY KEY,
                circuit_state TEXT NOT NULL DEFAULT 'closed',
                consecutive_failures INTEGER DEFAULT 0,
                consecutive_successes INTEGER DEFAULT 0,
                last_success TEXT,
                last_failure TEXT,
                last_error TEXT,
                avg_latency_ms REAL DEFAULT 0,
                current_rate_limit INTEGER,
                base_rate_limit INTEGER,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (provider_id) REFERENCES providers(id)
            );
            "#,
        )
        .map_err(|e| format!("Failed to initialize schema: {}", e))?;

        Ok(())
    }

    /// Start new session
    pub fn start_session(&self, session_id: &str) -> Result<(), String> {
        let conn = self.conn()?;

        // Mark all previous sessions as not current
        conn.execute("UPDATE sessions SET is_current = 0", [])
            .map_err(|e| format!("Failed to update sessions: {}", e))?;

        // Insert new session
        conn.execute(
            "INSERT INTO sessions (id, started_at, is_current) VALUES (?1, ?2, 1)",
            params![session_id, Utc::now().to_rfc3339()],
        )
        .map_err(|e| format!("Failed to insert session: {}", e))?;

        Ok(())
    }

    /// End current session
    pub fn end_session(&self, session_id: &str) -> Result<(), String> {
        let conn = self.conn()?;

        conn.execute(
            "UPDATE sessions SET ended_at = ?1, is_current = 0 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), session_id],
        )
        .map_err(|e| format!("Failed to end session: {}", e))?;

        Ok(())
    }

    /// Get current session ID
    pub fn get_current_session(&self) -> Result<Option<String>, String> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT id FROM sessions WHERE is_current = 1 LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to get session: {}", e))
    }

    /// Register or update provider
    pub fn upsert_provider(
        &self,
        id: &str,
        url_masked: &str,
        kind: ProviderKind,
        priority: u8,
    ) -> Result<(), String> {
        let conn = self.conn()?;

        conn.execute(
            r#"
            INSERT INTO providers (id, url_masked, kind, priority, enabled, updated_at)
            VALUES (?1, ?2, ?3, ?4, 1, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                url_masked = excluded.url_masked,
                kind = excluded.kind,
                priority = excluded.priority,
                updated_at = datetime('now')
            "#,
            params![id, url_masked, kind.to_string(), priority as i64],
        )
        .map_err(|e| format!("Failed to upsert provider: {}", e))?;

        Ok(())
    }

    /// Record RPC call
    pub fn record_call(&self, session_id: &str, record: &RpcCallRecord) -> Result<(), String> {
        let conn = self.conn()?;

        conn.execute(
            r#"
            INSERT INTO calls (
                session_id, provider_id, method, success, latency_ms,
                error_code, error_message, was_retried, retry_count,
                was_rate_limited, timestamp
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                session_id,
                record.provider_id,
                record.method,
                record.success as i32,
                record.latency_ms as i64,
                record.error_code,
                record.error_message,
                record.was_retried as i32,
                record.retry_count as i32,
                record.was_rate_limited as i32,
                record.timestamp.to_rfc3339(),
            ],
        )
        .map_err(|e| format!("Failed to record call: {}", e))?;

        // Update session totals
        if record.success {
            conn.execute(
                "UPDATE sessions SET total_calls = total_calls + 1 WHERE id = ?1",
                params![session_id],
            )
            .ok();
        } else {
            conn.execute(
                "UPDATE sessions SET total_calls = total_calls + 1, total_errors = total_errors + 1 WHERE id = ?1",
                params![session_id],
            )
            .ok();
        }

        Ok(())
    }

    /// Batch record calls (more efficient)
    pub fn record_calls(&self, session_id: &str, records: &[RpcCallRecord]) -> Result<(), String> {
        if records.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    r#"
                INSERT INTO calls (
                    session_id, provider_id, method, success, latency_ms,
                    error_code, error_message, was_retried, retry_count,
                    was_rate_limited, timestamp
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                )
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            for record in records {
                stmt.execute(params![
                    session_id,
                    record.provider_id,
                    record.method,
                    record.success as i32,
                    record.latency_ms as i64,
                    record.error_code,
                    record.error_message,
                    record.was_retried as i32,
                    record.retry_count as i32,
                    record.was_rate_limited as i32,
                    record.timestamp.to_rfc3339(),
                ])
                .map_err(|e| format!("Failed to insert record: {}", e))?;
            }
        }

        // Update session totals
        let success_count = records.iter().filter(|r| r.success).count();
        let error_count = records.len() - success_count;

        tx.execute(
            "UPDATE sessions SET total_calls = total_calls + ?1, total_errors = total_errors + ?2 WHERE id = ?3",
            params![records.len() as i64, error_count as i64, session_id],
        )
        .ok();

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Update provider health
    pub fn update_provider_health(
        &self,
        provider_id: &str,
        circuit_state: CircuitState,
        consecutive_failures: u32,
        consecutive_successes: u32,
        avg_latency_ms: f64,
        current_rate_limit: u32,
        base_rate_limit: u32,
        last_error: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn()?;

        conn.execute(
            r#"
            INSERT INTO provider_health (
                provider_id, circuit_state, consecutive_failures, consecutive_successes,
                avg_latency_ms, current_rate_limit, base_rate_limit, last_error, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))
            ON CONFLICT(provider_id) DO UPDATE SET
                circuit_state = excluded.circuit_state,
                consecutive_failures = excluded.consecutive_failures,
                consecutive_successes = excluded.consecutive_successes,
                avg_latency_ms = excluded.avg_latency_ms,
                current_rate_limit = excluded.current_rate_limit,
                base_rate_limit = excluded.base_rate_limit,
                last_error = excluded.last_error,
                updated_at = datetime('now')
            "#,
            params![
                provider_id,
                circuit_state.to_string(),
                consecutive_failures as i64,
                consecutive_successes as i64,
                avg_latency_ms,
                current_rate_limit as i64,
                base_rate_limit as i64,
                last_error,
            ],
        )
        .map_err(|e| format!("Failed to update health: {}", e))?;

        Ok(())
    }

    /// Get session stats
    pub fn get_session_stats(&self, session_id: &str) -> Result<Option<SessionStats>, String> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, started_at, ended_at, total_calls, total_errors
            FROM sessions WHERE id = ?1
            "#,
            params![session_id],
            |row| {
                let started_str: String = row.get(1)?;
                let ended_str: Option<String> = row.get(2)?;
                let started_at = DateTime::parse_from_rfc3339(&started_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let ended_at = ended_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok()
                });

                Ok(SessionStats {
                    session_id: row.get(0)?,
                    started_at,
                    ended_at,
                    total_calls: row.get::<_, i64>(3)? as u64,
                    total_errors: row.get::<_, i64>(4)? as u64,
                    duration_secs: (Utc::now() - started_at).num_seconds().max(0) as u64,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Failed to get session stats: {}", e))
    }

    /// Get calls per minute for last N minutes
    pub fn get_calls_per_minute(
        &self,
        session_id: &str,
        minutes: u32,
    ) -> Result<Vec<TimeBucketStats>, String> {
        let conn = self.conn()?;
        let cutoff = Utc::now() - ChronoDuration::minutes(minutes as i64);

        let mut stmt = conn
            .prepare(
                r#"
                SELECT 
                    strftime('%Y-%m-%dT%H:%M:00Z', timestamp) as minute,
                    COUNT(*) as call_count,
                    SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                    SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as error_count,
                    SUM(CASE WHEN was_rate_limited = 1 THEN 1 ELSE 0 END) as rate_limit_count,
                    SUM(latency_ms) as latency_sum,
                    MIN(latency_ms) as latency_min,
                    MAX(latency_ms) as latency_max
                FROM calls
                WHERE session_id = ?1 AND timestamp >= ?2
                GROUP BY minute
                ORDER BY minute DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map(params![session_id, cutoff.to_rfc3339()], |row| {
                let minute_str: String = row.get(0)?;
                let bucket_start = DateTime::parse_from_rfc3339(&minute_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(TimeBucketStats {
                    bucket_start,
                    call_count: row.get::<_, i64>(1)? as u64,
                    success_count: row.get::<_, i64>(2)? as u64,
                    error_count: row.get::<_, i64>(3)? as u64,
                    rate_limit_count: row.get::<_, i64>(4)? as u64,
                    latency_sum_ms: row.get::<_, i64>(5)? as u64,
                    latency_min_ms: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
                    latency_max_ms: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                })
            })
            .map_err(|e| format!("Failed to query: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(bucket) = row {
                results.push(bucket);
            }
        }

        Ok(results)
    }

    /// Get method stats
    pub fn get_method_stats(
        &self,
        session_id: &str,
        limit: u32,
    ) -> Result<Vec<MethodStats>, String> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT 
                    method,
                    COUNT(*) as total_calls,
                    SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as total_errors,
                    AVG(latency_ms) as avg_latency
                FROM calls
                WHERE session_id = ?1
                GROUP BY method
                ORDER BY total_calls DESC
                LIMIT ?2
                "#,
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map(params![session_id, limit], |row| {
                let total_calls: i64 = row.get(1)?;
                let total_errors: i64 = row.get(2)?;
                let success_rate = if total_calls > 0 {
                    100.0 * (total_calls - total_errors) as f64 / total_calls as f64
                } else {
                    100.0
                };

                Ok(MethodStats {
                    method: row.get(0)?,
                    total_calls: total_calls as u64,
                    total_errors: total_errors as u64,
                    avg_latency_ms: row.get(3)?,
                    success_rate,
                })
            })
            .map_err(|e| format!("Failed to query: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(stats) = row {
                results.push(stats);
            }
        }

        Ok(results)
    }

    /// Cleanup old data (retention)
    pub fn cleanup(&self, retention_hours: u64) -> Result<u64, String> {
        let conn = self.conn()?;
        let cutoff = Utc::now() - ChronoDuration::hours(retention_hours as i64);

        let deleted = conn
            .execute(
                "DELETE FROM calls WHERE timestamp < ?1",
                params![cutoff.to_rfc3339()],
            )
            .map_err(|e| format!("Failed to cleanup: {}", e))?;

        // Also cleanup old minute buckets
        conn.execute(
            "DELETE FROM minute_buckets WHERE minute_start < ?1",
            params![cutoff.to_rfc3339()],
        )
        .ok();

        Ok(deleted as u64)
    }

    /// Get total call count for session
    pub fn get_total_calls(&self, session_id: &str) -> Result<u64, String> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT total_calls FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|v| v as u64)
        .map_err(|e| format!("Failed to get total calls: {}", e))
    }

    /// Get average latency for session
    pub fn get_avg_latency(&self, session_id: &str) -> Result<f64, String> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT AVG(latency_ms) FROM calls WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, Option<f64>>(0),
        )
        .map(|v| v.unwrap_or(0.0))
        .map_err(|e| format!("Failed to get avg latency: {}", e))
    }
}

impl std::fmt::Debug for RpcStatsDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcStatsDatabase")
            .field("path", &get_rpc_stats_db_path())
            .finish()
    }
}
