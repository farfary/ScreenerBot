//! Persistence for `tokens::media`: the data service's resolved logo and banner.

use std::collections::HashMap;

use rusqlite::params;

use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::tokens::media::{MediaOverride, MediaUpdate};
use crate::tokens::types::TokenResult;
use crate::tokens::Error;

use super::TokenDatabase;

impl TokenDatabase {
    /// Every stored override that carries a logo or a banner.
    pub fn load_media_overrides(&self) -> TokenResult<HashMap<String, MediaOverride>> {
        let conn = self.conn()?;
        let mut statement = conn
            .prepare(
                "SELECT mint, logo_url, banner_url FROM token_media
                 WHERE chain_id = ?1 AND (logo_url IS NOT NULL OR banner_url IS NOT NULL)",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare token media load".to_owned(),
                    message: e.to_string(),
                })
            })?;
        let rows = statement
            .query_map(params![self.chain_id()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    MediaOverride {
                        logo_url: row.get(1)?,
                        banner_url: row.get(2)?,
                    },
                ))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to load token media".to_owned(),
                    message: e.to_string(),
                })
            })?;
        rows.collect::<Result<HashMap<_, _>, _>>().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to read token media row".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Known tokens whose media was never fetched or is due again, newest first.
    pub fn mints_due_for_media(&self, now: i64, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self.conn()?;
        let mut statement = conn
            .prepare(
                "SELECT t.mint FROM tokens t
                 LEFT JOIN token_media m ON m.chain_id = t.chain_id AND m.mint = t.mint
                 WHERE t.chain_id = ?1 AND (m.mint IS NULL OR m.media_next_fetch_at <= ?2)
                 ORDER BY t.first_discovered_at DESC
                 LIMIT ?3",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare token media due query".to_owned(),
                    message: e.to_string(),
                })
            })?;
        let rows = statement
            .query_map(params![self.chain_id(), now, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to query tokens due for media".to_owned(),
                    message: e.to_string(),
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to read token media due row".to_owned(),
                message: e.to_string(),
            })
        })
    }

    pub fn store_media_updates(&self, updates: &[MediaUpdate]) -> TokenResult<()> {
        if updates.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn()?;
        let tx = conn.write_tx().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to start token media transaction".to_owned(),
                message: e.to_string(),
            })
        })?;
        {
            let mut statement = tx
                .prepare_cached(
                    "INSERT INTO token_media
                        (chain_id, mint, logo_url, banner_url, media_last_fetched_at, media_next_fetch_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(chain_id, mint) DO UPDATE SET
                        logo_url = excluded.logo_url,
                        banner_url = excluded.banner_url,
                        media_last_fetched_at = excluded.media_last_fetched_at,
                        media_next_fetch_at = excluded.media_next_fetch_at",
                )
                .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare token media upsert".to_owned(),
                    message: e.to_string(),
                })
            })?;
            for update in updates {
                statement
                    .execute(params![
                        self.chain_id(),
                        update.mint,
                        update.media.logo_url,
                        update.media.banner_url,
                        update.fetched_at,
                        update.next_fetch_at,
                    ])
                    .map_err(|e| {
                        Error::Database(DatabaseError::Query {
                            operation: "Failed to store token media".to_owned(),
                            message: e.to_string(),
                        })
                    })?;
            }
        }
        tx.commit().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to commit token media".to_owned(),
                message: e.to_string(),
            })
        })?;
        Ok(())
    }
}
