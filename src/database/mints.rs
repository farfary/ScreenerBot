use crate::database::connection::Database;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

impl Database {
    /// Save a mint address to the database
    pub fn save_mint(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO mints (mint, discovered_at, is_active) VALUES (?1, ?2, ?3)",
            params![mint, Utc::now().to_rfc3339(), 1]
        )?;
        Ok(())
    }

    /// Get all mint addresses from the database
    pub fn get_all_mints(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT mint FROM mints WHERE is_active = 1")?;
        let mint_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut mints = Vec::new();
        for mint in mint_iter {
            mints.push(mint?);
        }

        Ok(mints)
    }

    /// Check if a mint exists in the database
    pub fn mint_exists(&self, mint: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM mints WHERE mint = ?1",
            params![mint],
            |row| row.get(0)
        )?;
        Ok(count > 0)
    }

    /// Get total mint count
    pub fn get_mint_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM mints WHERE is_active = 1", [], |row|
            row.get(0)
        )?;
        Ok(count)
    }

    /// Mark a mint as inactive
    pub fn mark_mint_inactive(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE mints SET is_active = 0 WHERE mint = ?1", params![mint])?;
        Ok(())
    }

    /// Mark a mint as active
    pub fn mark_mint_active(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE mints SET is_active = 1 WHERE mint = ?1", params![mint])?;
        Ok(())
    }

    /// Delete a mint from the database
    pub fn delete_mint(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM mints WHERE mint = ?1", params![mint])?;
        Ok(())
    }

    /// Get recently discovered mints (last N hours)
    pub fn get_recent_mints(&self, hours: u64) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
        let mut stmt = conn.prepare(
            "SELECT mint FROM mints WHERE discovered_at >= ?1 AND is_active = 1 ORDER BY discovered_at DESC"
        )?;

        let mint_iter = stmt.query_map(params![cutoff_time.to_rfc3339()], |row|
            row.get::<_, String>(0)
        )?;

        let mut mints = Vec::new();
        for mint in mint_iter {
            mints.push(mint?);
        }

        Ok(mints)
    }

    /// Get mints with pagination
    pub fn get_mints_paginated(&self, limit: u32, offset: u32) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT mint FROM mints WHERE is_active = 1 ORDER BY discovered_at DESC LIMIT ?1 OFFSET ?2"
        )?;

        let mint_iter = stmt.query_map(params![limit, offset], |row| row.get::<_, String>(0))?;

        let mut mints = Vec::new();
        for mint in mint_iter {
            mints.push(mint?);
        }

        Ok(mints)
    }
}
