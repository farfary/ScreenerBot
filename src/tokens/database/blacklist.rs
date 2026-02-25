//! Token blacklist database — persists permanently blocked token addresses.

use chrono::Utc;
use rusqlite::params;

use crate::tokens::types::{TokenError, TokenResult};

use super::{TokenBlacklistRecord, TokenDatabase};

impl TokenDatabase {
    /// Add a token to the blacklist with a reason and source
    pub fn add_to_blacklist(&self, mint: &str, reason: &str, source: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO blacklist (mint, reason, source, added_at) 
             VALUES (?1, ?2, ?3, ?4)",
            params![mint, reason, source, now],
        )
        .map_err(|e| TokenError::Database(format!("Failed to add to blacklist: {e}")))?;

        Ok(())
    }

    /// List all blacklist entries with metadata for diagnostics/analytics
    pub fn list_blacklisted_tokens(&self) -> TokenResult<Vec<TokenBlacklistRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, reason, source, added_at \
                 FROM blacklist \
                 ORDER BY added_at DESC",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare blacklist query: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TokenBlacklistRecord {
                    mint: row.get(0)?,
                    reason: row.get(1)?,
                    source: row.get(2)?,
                    added_at: row.get(3)?,
                })
            })
            .map_err(|e| TokenError::Database(format!("Failed to query blacklist: {e}")))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(
                row.map_err(|e| {
                    TokenError::Database(format!("Failed to read blacklist row: {e}"))
                })?,
            );
        }

        Ok(records)
    }

    /// Check if token is blacklisted
    pub fn is_blacklisted(&self, mint: &str) -> TokenResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT 1 FROM blacklist WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let exists = stmt
            .exists(params![mint])
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        Ok(exists)
    }

    /// Remove token from blacklist
    pub fn remove_from_blacklist(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        conn.execute("DELETE FROM blacklist WHERE mint = ?1", params![mint])
            .map_err(|e| TokenError::Database(format!("Failed to remove from blacklist: {e}")))?;

        Ok(())
    }

    /// Get blacklist reason
    pub fn get_blacklist_reason(&self, mint: &str) -> TokenResult<Option<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT reason, source FROM blacklist WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let result = stmt.query_row(params![mint], |row| Ok((row.get(0)?, row.get(1)?)));

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(TokenError::Database(format!("Query failed: {e}"))),
        }
    }
}
