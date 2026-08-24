//! Token authority database — stores mint/freeze authority and upgrade status.

// tokens/database/authority.rs
// Database operations for the authority_reputation table.
// Supports the auto-discovery system: persists reputation scores,
// loads blocked authorities on startup, and upserts from analysis tasks.

use crate::errors::DatabaseError;
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;
use crate::tokens::Error;

impl TokenDatabase {
    /// Load all blocked authority addresses from the database.
    /// Called on startup and periodically by the discovery task.
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

    /// Run auto-discovery: analyze rejection data to find scam authorities.
    /// Groups tokens by their freeze/mint/update authority addresses,
    /// cross-references with rejection history to compute confidence scores.
    ///
    /// Returns the number of newly blocked authorities.
    pub fn run_authority_discovery(
        &self,
        min_tokens: u32,
        min_confidence: f64,
    ) -> TokenResult<u32> {
        let conn = self.conn()?;

        let now = chrono::Utc::now().timestamp();
        let mut newly_blocked = 0u32;

        // Analyze freeze authorities from security_rugcheck
        // Cross-reference with rejection history to find authorities whose tokens get rejected
        let sql = r#"
            SELECT
                sr.freeze_authority AS authority,
                'freeze' AS authority_type,
                COUNT(DISTINCT sr.mint) AS total_tokens,
                COUNT(DISTINCT CASE WHEN ut.last_rejection_at IS NOT NULL THEN sr.mint END) AS flagged_tokens
            FROM security_rugcheck sr
            LEFT JOIN update_tracking ut ON sr.chain_id = ut.chain_id AND sr.mint = ut.mint
            WHERE sr.chain_id = ?1 AND sr.freeze_authority IS NOT NULL AND sr.freeze_authority != ''
            GROUP BY sr.freeze_authority
            HAVING total_tokens >= ?2

            UNION ALL

            SELECT
                sr.update_authority AS authority,
                'update' AS authority_type,
                COUNT(DISTINCT sr.mint) AS total_tokens,
                COUNT(DISTINCT CASE WHEN ut.last_rejection_at IS NOT NULL THEN sr.mint END) AS flagged_tokens
            FROM security_rugcheck sr
            LEFT JOIN update_tracking ut ON sr.chain_id = ut.chain_id AND sr.mint = ut.mint
            WHERE sr.chain_id = ?1 AND sr.update_authority IS NOT NULL AND sr.update_authority != ''
            GROUP BY sr.update_authority
            HAVING total_tokens >= ?2

            UNION ALL

            SELECT
                sr.mint_authority AS authority,
                'mint' AS authority_type,
                COUNT(DISTINCT sr.mint) AS total_tokens,
                COUNT(DISTINCT CASE WHEN ut.last_rejection_at IS NOT NULL THEN sr.mint END) AS flagged_tokens
            FROM security_rugcheck sr
            LEFT JOIN update_tracking ut ON sr.chain_id = ut.chain_id AND sr.mint = ut.mint
            WHERE sr.chain_id = ?1 AND sr.mint_authority IS NOT NULL AND sr.mint_authority != ''
            GROUP BY sr.mint_authority
            HAVING total_tokens >= ?2
        "#;

        let mut stmt = conn.prepare(sql).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Prepare discovery SQL error".to_owned(),
                message: e.to_string(),
            })
        })?;

        let rows = stmt
            .query_map(rusqlite::params![self.chain_id(), min_tokens], |row| {
                Ok((
                    row.get::<_, String>(0)?, // authority
                    row.get::<_, String>(1)?, // authority_type
                    row.get::<_, u32>(2)?,    // total_tokens
                    row.get::<_, u32>(3)?,    // flagged_tokens
                ))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query discovery error".to_owned(),
                    message: e.to_string(),
                })
            })?;

        for row in rows {
            if let Ok((authority, authority_type, total, flagged)) = row {
                let confidence = if total > 0 {
                    flagged as f64 / total as f64
                } else {
                    0.0
                };

                let should_block = confidence >= min_confidence && total >= min_tokens;

                // Upsert using the same connection (no lock reentry needed)
                conn.execute(
                    "INSERT INTO authority_reputation (chain_id, address, authority_type, total_token_count, flagged_token_count, confidence, is_blocked, source, first_seen_at, last_updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'auto', ?8, ?8)
                     ON CONFLICT(chain_id, address) DO UPDATE SET
                         authority_type = excluded.authority_type,
                         total_token_count = excluded.total_token_count,
                         flagged_token_count = excluded.flagged_token_count,
                         confidence = excluded.confidence,
                         is_blocked = MAX(is_blocked, excluded.is_blocked),
                         last_updated_at = excluded.last_updated_at",
                    rusqlite::params![
                        self.chain_id(), authority,
                        authority_type,
                        total,
                        flagged,
                        confidence,
                        should_block as i32,
                        now,
                    ],
                )
                .map_err(|e| Error::Database(DatabaseError::Query { operation: "Upsert discovery error".to_owned(), message: e.to_string() }))?;

                if should_block {
                    newly_blocked += 1;
                }
            }
        }

        Ok(newly_blocked)
    }
}
