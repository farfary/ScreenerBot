// tokens/database/authority.rs
// Database operations for the authority_reputation table.
// Supports the auto-discovery system: persists reputation scores,
// loads blocked authorities on startup, and upserts from analysis tasks.

use crate::tokens::database::TokenDatabase;
use crate::tokens::types::{TokenError, TokenResult};

impl TokenDatabase {
    /// Load all blocked authority addresses from the database.
    /// Called on startup and periodically by the discovery task.
    pub fn load_blocked_authorities(&self) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT address FROM authority_reputation WHERE is_blocked = 1")
            .map_err(|e| TokenError::Database(format!("Prepare error: {}", e)))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| TokenError::Database(format!("Query error: {}", e)))?;

        let mut addresses = Vec::new();
        for row in rows {
            if let Ok(addr) = row {
                addresses.push(addr);
            }
        }
        Ok(addresses)
    }

    /// Upsert authority reputation record. Uses ON CONFLICT to update existing entries.
    pub fn upsert_authority_reputation(
        &self,
        address: &str,
        authority_type: &str,
        total_token_count: u32,
        flagged_token_count: u32,
        confidence: f64,
        is_blocked: bool,
        source: &str,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock error: {}", e)))?;

        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO authority_reputation (address, authority_type, total_token_count, flagged_token_count, confidence, is_blocked, source, first_seen_at, last_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(address) DO UPDATE SET
                 authority_type = ?2,
                 total_token_count = ?3,
                 flagged_token_count = ?4,
                 confidence = ?5,
                 is_blocked = ?6,
                 source = ?7,
                 last_updated_at = ?8",
            rusqlite::params![
                address,
                authority_type,
                total_token_count,
                flagged_token_count,
                confidence,
                is_blocked as i32,
                source,
                now,
            ],
        )
        .map_err(|e| TokenError::Database(format!("Upsert error: {}", e)))?;

        Ok(())
    }

    /// Get count of blocked authorities (for stats/logging)
    pub fn count_blocked_authorities(&self) -> TokenResult<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock error: {}", e)))?;

        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM authority_reputation WHERE is_blocked = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| TokenError::Database(format!("Query error: {}", e)))?;

        Ok(count)
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock error: {}", e)))?;

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
            LEFT JOIN update_tracking ut ON sr.mint = ut.mint
            WHERE sr.freeze_authority IS NOT NULL AND sr.freeze_authority != ''
            GROUP BY sr.freeze_authority
            HAVING total_tokens >= ?1

            UNION ALL

            SELECT
                sr.update_authority AS authority,
                'update' AS authority_type,
                COUNT(DISTINCT sr.mint) AS total_tokens,
                COUNT(DISTINCT CASE WHEN ut.last_rejection_at IS NOT NULL THEN sr.mint END) AS flagged_tokens
            FROM security_rugcheck sr
            LEFT JOIN update_tracking ut ON sr.mint = ut.mint
            WHERE sr.update_authority IS NOT NULL AND sr.update_authority != ''
            GROUP BY sr.update_authority
            HAVING total_tokens >= ?1

            UNION ALL

            SELECT
                sr.mint_authority AS authority,
                'mint' AS authority_type,
                COUNT(DISTINCT sr.mint) AS total_tokens,
                COUNT(DISTINCT CASE WHEN ut.last_rejection_at IS NOT NULL THEN sr.mint END) AS flagged_tokens
            FROM security_rugcheck sr
            LEFT JOIN update_tracking ut ON sr.mint = ut.mint
            WHERE sr.mint_authority IS NOT NULL AND sr.mint_authority != ''
            GROUP BY sr.mint_authority
            HAVING total_tokens >= ?1
        "#;

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| TokenError::Database(format!("Prepare discovery SQL error: {}", e)))?;

        let rows = stmt
            .query_map(rusqlite::params![min_tokens], |row| {
                Ok((
                    row.get::<_, String>(0)?, // authority
                    row.get::<_, String>(1)?, // authority_type
                    row.get::<_, u32>(2)?,    // total_tokens
                    row.get::<_, u32>(3)?,    // flagged_tokens
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query discovery error: {}", e)))?;

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
                    "INSERT INTO authority_reputation (address, authority_type, total_token_count, flagged_token_count, confidence, is_blocked, source, first_seen_at, last_updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'auto', ?7, ?7)
                     ON CONFLICT(address) DO UPDATE SET
                         authority_type = ?2,
                         total_token_count = ?3,
                         flagged_token_count = ?4,
                         confidence = ?5,
                         is_blocked = MAX(is_blocked, ?6),
                         last_updated_at = ?7",
                    rusqlite::params![
                        authority,
                        authority_type,
                        total,
                        flagged,
                        confidence,
                        should_block as i32,
                        now,
                    ],
                )
                .map_err(|e| TokenError::Database(format!("Upsert discovery error: {}", e)))?;

                if should_block {
                    newly_blocked += 1;
                }
            }
        }

        Ok(newly_blocked)
    }
}
