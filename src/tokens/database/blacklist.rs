//! Token blacklist database — persists permanently blocked token addresses.

use crate::errors::DatabaseError;
use chrono::Utc;
use rusqlite::params;

use crate::tokens::types::TokenResult;
use crate::tokens::Error;

use super::{TokenBlacklistRecord, TokenDatabase};

impl TokenDatabase {
    /// Add a token to the blacklist with a reason and source
    pub fn add_to_blacklist(&self, mint: &str, reason: &str, source: &str) -> TokenResult<()> {
        let conn = self.conn()?;

        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT OR REPLACE INTO blacklist (chain_id, mint, reason, source, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![self.chain_id(), mint, reason, source, now],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to add to blacklist".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// List all blacklist entries with metadata for diagnostics/analytics
    pub fn list_blacklisted_tokens(&self) -> TokenResult<Vec<TokenBlacklistRecord>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, reason, source, added_at \
                 FROM blacklist WHERE chain_id = ?1 \
                 ORDER BY added_at DESC",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare blacklist query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let rows = stmt
            .query_map(params![self.chain_id()], |row| {
                Ok(TokenBlacklistRecord {
                    mint: row.get(0)?,
                    reason: row.get(1)?,
                    source: row.get(2)?,
                    added_at: row.get(3)?,
                })
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to query blacklist".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to read blacklist row".to_owned(),
                    message: e.to_string(),
                })
            })?);
        }

        Ok(records)
    }

    /// Check if token is blacklisted
    pub fn is_blacklisted(&self, mint: &str) -> TokenResult<bool> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare("SELECT 1 FROM blacklist WHERE chain_id = ?1 AND mint = ?2")
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let exists = stmt.exists(params![self.chain_id(), mint]).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(exists)
    }

    /// Remove token from blacklist
    pub fn remove_from_blacklist(&self, mint: &str) -> TokenResult<()> {
        let conn = self.conn()?;

        conn.execute(
            "DELETE FROM blacklist WHERE chain_id = ?1 AND mint = ?2",
            params![self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to remove from blacklist".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Get blacklist reason
    pub fn get_blacklist_reason(&self, mint: &str) -> TokenResult<Option<(String, String)>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare("SELECT reason, source FROM blacklist WHERE chain_id = ?1 AND mint = ?2")
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let result = stmt.query_row(params![self.chain_id(), mint], |row| {
            Ok((row.get(0)?, row.get(1)?))
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })),
        }
    }
}
