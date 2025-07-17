use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ Connection, params };
use serde::{ Deserialize, Serialize };
use std::sync::Mutex;

/// Simple discovered token entry - only mint address and discovery date
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredToken {
    pub mint: String,
    pub discovered_at: DateTime<Utc>,
}

/// Discovery statistics for tracking module performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_tokens_discovered: u64,
    pub active_tokens: u64,
    pub last_discovery_run: DateTime<Utc>,
    pub discovery_rate_per_hour: f64,
}

/// Database connection for discovery module
pub struct DiscoveryDatabase {
    conn: Mutex<Connection>,
}

impl DiscoveryDatabase {
    /// Create a new discovery database connection
    pub fn new() -> Result<Self> {
        let conn = Connection::open("cache_discovery.db").context(
            "Failed to open discovery database"
        )?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.initialize_tables()?;
        Ok(db)
    }

    /// Initialize discovery database tables
    fn initialize_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create tokens table - only mint address and discovery date
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                discovered_at TEXT NOT NULL
            )",
            []
        )?;

        // Create index for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_discovered_at ON tokens(discovered_at)",
            []
        )?;

        Ok(())
    }

    /// Save a discovered token (mint address only)
    pub fn save_token(&self, mint: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        // Check if token already exists
        let exists: bool = conn
            .query_row("SELECT 1 FROM tokens WHERE mint = ?1", params![mint], |_| Ok(true))
            .unwrap_or(false);

        if exists {
            return Ok(false); // Token already exists
        }

        // Insert new token
        conn.execute(
            "INSERT INTO tokens (mint, discovered_at) VALUES (?1, ?2)",
            params![mint, Utc::now().to_rfc3339()]
        )?;

        Ok(true) // New token was added
    }

    /// Check if a token exists in the database
    pub fn token_exists(&self, mint: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tokens WHERE mint = ?1",
            params![mint],
            |row| row.get(0)
        )?;
        Ok(count > 0)
    }

    /// Get all discovered tokens
    pub fn get_all_tokens(&self) -> Result<Vec<DiscoveredToken>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT mint, discovered_at FROM tokens ORDER BY discovered_at DESC"
        )?;
        let token_iter = stmt.query_map([], |row| {
            Ok(DiscoveredToken {
                mint: row.get(0)?,
                discovered_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get recent tokens (last N hours)
    pub fn get_recent_tokens(&self, hours: u64) -> Result<Vec<DiscoveredToken>> {
        let conn = self.conn.lock().unwrap();
        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
        let mut stmt = conn.prepare(
            "SELECT mint, discovered_at FROM tokens WHERE discovered_at >= ?1 ORDER BY discovered_at DESC"
        )?;

        let token_iter = stmt.query_map(params![cutoff_time.to_rfc3339()], |row| {
            Ok(DiscoveredToken {
                mint: row.get(0)?,
                discovered_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get tokens with pagination
    pub fn get_tokens_paginated(&self, limit: u32, offset: u32) -> Result<Vec<DiscoveredToken>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT mint, discovered_at FROM tokens ORDER BY discovered_at DESC LIMIT ?1 OFFSET ?2"
        )?;

        let token_iter = stmt.query_map(params![limit, offset], |row| {
            Ok(DiscoveredToken {
                mint: row.get(0)?,
                discovered_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get total token count
    pub fn get_token_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Get discovery statistics
    pub fn get_stats(&self) -> Result<DiscoveryStats> {
        let conn = self.conn.lock().unwrap();

        let total_tokens: u64 = conn.query_row("SELECT COUNT(*) FROM tokens", [], |row|
            row.get(0)
        )?;

        // Calculate discovery rate for last 24 hours
        let cutoff_time = Utc::now() - chrono::Duration::hours(24);
        let recent_tokens: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tokens WHERE discovered_at >= ?1",
            params![cutoff_time.to_rfc3339()],
            |row| row.get(0)
        )?;

        let discovery_rate = recent_tokens as f64; // tokens per 24 hours

        Ok(DiscoveryStats {
            total_tokens_discovered: total_tokens,
            active_tokens: total_tokens, // All tokens are considered active in discovery
            last_discovery_run: Utc::now(),
            discovery_rate_per_hour: discovery_rate / 24.0,
        })
    }

    /// Delete a token from the database
    pub fn delete_token(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tokens WHERE mint = ?1", params![mint])?;
        Ok(())
    }

    /// Clear all tokens (for testing/reset purposes)
    pub fn clear_all_tokens(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tokens", [])?;
        Ok(())
    }
}

// Thread safety
unsafe impl Send for DiscoveryDatabase {}
unsafe impl Sync for DiscoveryDatabase {}
