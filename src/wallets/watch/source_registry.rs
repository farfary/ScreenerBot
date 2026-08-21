//! Multi-consumer source ownership for persisted watch targets.

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use super::database::WatchDatabase;
use super::{WatchSource, WatchTarget};

impl WatchDatabase {
    pub async fn upsert_source(
        &self,
        address: &str,
        label: Option<&str>,
        source: WatchSource,
    ) -> Result<WatchTarget, String> {
        let db = self.clone();
        let address = address.to_owned();
        let label = label.map(str::to_owned);
        tokio::task::spawn_blocking(move || {
            let mut conn = db.conn()?;
            let tx = conn.transaction().map_err(|e| format!("Failed to begin watch-source update: {e}"))?;
            let existing: Option<(i64, String)> = tx.query_row("SELECT id, sources FROM watch_targets WHERE chain_id = ?1 AND address = ?2", params![db.chain.as_str(), address], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|e| format!("Failed to read watch sources: {e}"))?;
            let now = Utc::now().to_rfc3339();
            let id = if let Some((id, json)) = existing {
                let mut sources: Vec<WatchSource> = serde_json::from_str(&json)
                    .map_err(|e| format!("Failed to decode persisted watch sources: {e}"))?;
                if !sources.contains(&source) { sources.push(source); }
                tx.execute("UPDATE watch_targets SET sources=?1, label=COALESCE(?2,label), enabled=1, updated_at=?3 WHERE chain_id = ?4 AND id=?5", params![serde_json::to_string(&sources).map_err(|e| format!("Failed to serialize watch sources: {e}"))?, label, now, db.chain.as_str(), id]).map_err(|e| format!("Failed to update watch sources: {e}"))?;
                id
            } else {
                tx.execute("INSERT INTO watch_targets (chain_id,address,label,sources,enabled,created_at,updated_at) VALUES (?1,?2,?3,?4,1,?5,?5)", params![db.chain.as_str(), address, label, serde_json::to_string(&vec![source]).map_err(|e| format!("Failed to serialize watch source: {e}"))?, now]).map_err(|e| format!("Failed to insert watch source: {e}"))?;
                tx.last_insert_rowid()
            };
            tx.commit().map_err(|e| format!("Failed to commit watch-source update: {e}"))?;
            drop(conn);
            db.get_target_sync(id)?.ok_or_else(|| "Watch target missing after source update".to_owned())
        }).await.map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Remove one consumer; delete the target/cursor only after its final source.
    pub async fn remove_source(&self, address: &str, source: WatchSource) -> Result<(), String> {
        let db = self.clone();
        let address = address.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut conn = db.conn()?;
            let tx = conn
                .transaction()
                .map_err(|e| format!("Failed to begin watch-source removal: {e}"))?;
            let json: Option<String> = tx
                .query_row(
                    "SELECT sources FROM watch_targets WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| format!("Failed to read watch sources: {e}"))?;
            let Some(json) = json else {
                return Ok(());
            };
            let mut sources: Vec<WatchSource> = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to decode persisted watch sources: {e}"))?;
            sources.retain(|candidate| *candidate != source);
            if sources.is_empty() {
                tx.execute(
                    "DELETE FROM watch_targets WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                )
                .map_err(|e| format!("Failed to delete watch target: {e}"))?;
                tx.execute(
                    "DELETE FROM watch_cursors WHERE chain_id = ?1 AND address = ?2",
                    params![db.chain.as_str(), address],
                )
                .map_err(|e| format!("Failed to delete watch cursor: {e}"))?;
            } else {
                tx.execute(
                    "UPDATE watch_targets SET sources=?1, updated_at=?2 WHERE chain_id = ?3 AND address=?4",
                    params![
                        serde_json::to_string(&sources)
                            .map_err(|e| format!("Failed to serialize watch sources: {e}"))?,
                        Utc::now().to_rfc3339(),
                        db.chain.as_str(), address
                    ],
                )
                .map_err(|e| format!("Failed to update watch sources: {e}"))?;
            }
            tx.commit()
                .map_err(|e| format!("Failed to commit watch-source removal: {e}"))
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }
}
