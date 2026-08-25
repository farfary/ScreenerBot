//! Multi-consumer source ownership for persisted watch targets.

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use super::database::WatchDatabase;
use super::{WatchSource, WatchTarget};
use crate::database::WriteTransaction;
use crate::errors::{DatabaseError, InternalError};
use crate::wallets::Error;

impl WatchDatabase {
    pub async fn upsert_source(
        &self,
        address: &str,
        label: Option<&str>,
        source: WatchSource,
    ) -> Result<WatchTarget, Error> {
        let db = self.clone();
        let address = address.to_owned();
        let label = label.map(str::to_owned);
        tokio::task::spawn_blocking(move || {
            let mut conn = db.conn()?;
            let tx = conn.write_tx().map_err(DatabaseError::from)?;
            let existing: Option<(i64, String)> = tx
                .query_row(
                    "SELECT id, sources FROM watch_targets WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(DatabaseError::from)?;
            let now = Utc::now().to_rfc3339();
            let id = if let Some((id, json)) = existing {
                let mut sources: Vec<WatchSource> =
                    serde_json::from_str(&json).map_err(|e| Error::WatchSourcesDecode {
                        detail: e.to_string(),
                    })?;
                if !sources.contains(&source) {
                    sources.push(source);
                }
                let sources_json =
                    serde_json::to_string(&sources).map_err(|e| Error::Internal(InternalError::InvariantViolation {
                        message: format!("could not serialize watch sources: {e}"),
                    }))?;
                tx.execute("UPDATE watch_targets SET sources=?1, label=COALESCE(?2,label), enabled=1, updated_at=?3 WHERE chain_id = ?4 AND id=?5", params![sources_json, label, now, db.chain.as_str(), id]).map_err(DatabaseError::from)?;
                id
            } else {
                let sources_json =
                    serde_json::to_string(&vec![source]).map_err(|e| Error::Internal(InternalError::InvariantViolation {
                        message: format!("could not serialize watch source: {e}"),
                    }))?;
                tx.execute("INSERT INTO watch_targets (chain_id,address,label,sources,enabled,created_at,updated_at) VALUES (?1,?2,?3,?4,1,?5,?5)", params![db.chain.as_str(), address, label, sources_json, now]).map_err(DatabaseError::from)?;
                tx.last_insert_rowid()
            };
            tx.commit().map_err(DatabaseError::from)?;
            drop(conn);
            db.get_target_sync(id)?
                .ok_or_else(|| Error::Internal(InternalError::InvariantViolation {
                    message: format!("watch target {id} missing after source update"),
                }))
        })
        .await
        .map_err(|e| Error::Internal(InternalError::from(e)))?
    }

    /// Remove one consumer; delete the target/cursor only after its final source.
    pub async fn remove_source(&self, address: &str, source: WatchSource) -> Result<(), Error> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut conn = db.conn()?;
            let tx = conn.write_tx().map_err(DatabaseError::from)?;
            let json: Option<String> = tx
                .query_row(
                    "SELECT sources FROM watch_targets WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                    |row| row.get(0),
                )
                .optional()
                .map_err(DatabaseError::from)?;
            let Some(json) = json else {
                return Ok(());
            };
            let mut sources: Vec<WatchSource> =
                serde_json::from_str(&json).map_err(|e| Error::WatchSourcesDecode {
                    detail: e.to_string(),
                })?;
            sources.retain(|candidate| *candidate != source);
            if sources.is_empty() {
                tx.execute(
                    "DELETE FROM watch_targets WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                )
                .map_err(DatabaseError::from)?;
                tx.execute(
                    "DELETE FROM watch_cursors WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                )
                .map_err(DatabaseError::from)?;
            } else {
                let sources_json =
                    serde_json::to_string(&sources).map_err(|e| Error::Internal(InternalError::InvariantViolation {
                        message: format!("could not serialize watch sources: {e}"),
                    }))?;
                tx.execute(
                    "UPDATE watch_targets SET sources=?1, updated_at=?2 WHERE chain_id = ?3 AND address=?4",
                    params![sources_json, Utc::now().to_rfc3339(), db.chain.as_str(), address],
                )
                .map_err(DatabaseError::from)?;
            }
            tx.commit().map_err(DatabaseError::from).map_err(Error::from)
        })
        .await
        .map_err(|e| Error::Internal(InternalError::from(e)))?
    }
}
