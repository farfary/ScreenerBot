// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Transaction database bootstrap — state tracking for historical backfill.
//
// Bootstrap state and reconciliation operations

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::transactions::error::Error;

use super::operations::TransactionDatabase;

// =============================================================================
// IMPLEMENTATION - BOOTSTRAP STATE AND RECONCILIATION
// =============================================================================

/// Bootstrap state persisted across restarts: whether the full history was walked to the
/// chain's end, and where a bootstrap limited to the newest signatures stopped. An interrupted
/// walk is never resumed from that cursor: it restarts from the newest signature and skips the
/// known ones, so no signature collected but never processed is passed over. The cursor is read
/// only by the on-demand load of older history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapState {
    pub full_history_completed: bool,
    /// The oldest signature a limited bootstrap or older-history load reached; older history
    /// continues before it.
    pub history_before: Option<String>,
}

impl BootstrapState {
    /// Whether history older than the stored signatures can still be loaded on demand.
    pub fn older_history_available(&self) -> bool {
        !self.full_history_completed && self.history_before.is_some()
    }
}

impl TransactionDatabase {
    /// Get the current bootstrap state
    pub async fn get_bootstrap_state(&self) -> Result<BootstrapState, Error> {
        let conn = self.get_connection()?;
        let mut state = BootstrapState::default();

        let result = conn
            .query_row(
                "SELECT full_history_completed, backfill_before_cursor FROM bootstrap_state WHERE chain_id = ?1 AND id = 1",
                params![self.chain.as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?;

        if let Some((completed_i, history_before)) = result {
            state.full_history_completed = completed_i != 0;
            state.history_before = history_before;
        }

        Ok(state)
    }

    /// Mark the full history as completed, creating the chain's state row when none exists
    pub async fn mark_full_history_completed(&self) -> Result<(), Error> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO bootstrap_state (chain_id, id, full_history_completed) VALUES (?1, 1, 1)
             ON CONFLICT (chain_id, id) DO UPDATE SET full_history_completed = 1, updated_at = datetime('now')",
            params![self.chain.as_str()],
        )
        .map_err(crate::errors::DatabaseError::from)?;
        Ok(())
    }

    /// Record where older history continues, or clear it once the chain's end is reached,
    /// creating the chain's state row when none exists
    pub async fn set_history_cursor(&self, before: Option<&str>) -> Result<(), Error> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO bootstrap_state (chain_id, id, backfill_before_cursor) VALUES (?1, 1, ?2)
             ON CONFLICT (chain_id, id) DO UPDATE SET backfill_before_cursor = excluded.backfill_before_cursor, updated_at = datetime('now')",
            params![self.chain.as_str(), before],
        )
        .map_err(crate::errors::DatabaseError::from)?;
        Ok(())
    }

    /// Reconcile known_signatures with already processed transactions
    /// Ensures no processed transaction is missing from known_signatures, for any
    /// subject -- both columns of the composite key must be carried across, or every
    /// row would violate the `wallet_address NOT NULL` constraint and silently
    /// reconcile nothing (`INSERT OR IGNORE` swallows constraint failures).
    pub async fn reconcile_known_with_processed(&self) -> Result<usize, Error> {
        let conn = self.get_connection()?;
        let affected = conn
            .execute(
                "INSERT OR IGNORE INTO known_signatures(chain_id, signature, wallet_address) SELECT chain_id, signature, wallet_address FROM processed_transactions WHERE chain_id = ?1",
                params![self.chain.as_str()]
            )
            .map_err(crate::errors::DatabaseError::from)?;
        Ok(affected as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;

    /// A walk that reached the chain's end is recorded on a database that never had a state
    /// row, and is recorded once however often it is marked.
    #[tokio::test]
    async fn full_history_completion_is_recorded_without_an_earlier_state_row() {
        let dir = tempfile::tempdir().unwrap();
        let db =
            TransactionDatabase::new_with_path(dir.path().join("transactions.db"), ChainId::Solana)
                .await
                .unwrap();
        assert!(
            !db.get_bootstrap_state()
                .await
                .unwrap()
                .full_history_completed
        );

        db.mark_full_history_completed().await.unwrap();
        db.mark_full_history_completed().await.unwrap();

        assert!(
            db.get_bootstrap_state()
                .await
                .unwrap()
                .full_history_completed
        );
        let rows: i64 = db
            .get_connection()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM bootstrap_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    /// The older-history cursor is recorded, moved and cleared on one state row, and older
    /// history is offered only while the full history is incomplete.
    #[tokio::test]
    async fn the_history_cursor_marks_where_older_history_continues() {
        let dir = tempfile::tempdir().unwrap();
        let db =
            TransactionDatabase::new_with_path(dir.path().join("transactions.db"), ChainId::Solana)
                .await
                .unwrap();
        assert!(!db
            .get_bootstrap_state()
            .await
            .unwrap()
            .older_history_available());

        db.set_history_cursor(Some("sig-200")).await.unwrap();
        let state = db.get_bootstrap_state().await.unwrap();
        assert_eq!(state.history_before.as_deref(), Some("sig-200"));
        assert!(state.older_history_available());

        db.set_history_cursor(Some("sig-400")).await.unwrap();
        assert_eq!(
            db.get_bootstrap_state()
                .await
                .unwrap()
                .history_before
                .as_deref(),
            Some("sig-400")
        );

        db.mark_full_history_completed().await.unwrap();
        assert!(!db
            .get_bootstrap_state()
            .await
            .unwrap()
            .older_history_available());

        db.set_history_cursor(None).await.unwrap();
        assert_eq!(db.get_bootstrap_state().await.unwrap().history_before, None);
        let rows: i64 = db
            .get_connection()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM bootstrap_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }
}
