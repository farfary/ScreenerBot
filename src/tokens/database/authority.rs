// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token authority database — stores mint/freeze authority and upgrade status.

// tokens/database/authority.rs
// Database operations for the authority_reputation table:
// loads blocked authorities on startup and removes automatically written rows.

use crate::errors::DatabaseError;
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;
use crate::tokens::Error;

impl TokenDatabase {
    /// Load all blocked authority addresses from the database.
    /// Called on startup by the tokens service.
    pub fn load_blocked_authorities(&self) -> TokenResult<Vec<String>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT address FROM authority_reputation WHERE chain_id = ?1 AND is_blocked = 1",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Prepare error".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let rows = stmt
            .query_map(rusqlite::params![self.chain_id()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query error".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut addresses = Vec::new();
        for row in rows {
            if let Ok(addr) = row {
                addresses.push(addr);
            }
        }
        Ok(addresses)
    }

    /// Delete every automatically written reputation row of this chain.
    ///
    /// Rows written by the retired discovery count any current rejection as
    /// evidence against an authority, so they are not reliable; only rows from
    /// another source are kept.
    ///
    /// Returns the number of removed rows.
    pub fn remove_auto_reputation(&self) -> TokenResult<usize> {
        let conn = self.conn()?;

        conn.execute(
            "DELETE FROM authority_reputation WHERE chain_id = ?1 AND source = 'auto'",
            rusqlite::params![self.chain_id()],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Remove automatic authority reputation error".to_owned(),
                message: e.to_string(),
            })
        })
    }
}
