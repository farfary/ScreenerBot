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
/// chain's end. An interrupted walk is never resumed from a cursor: it restarts from the
/// newest signature and skips the known ones, so no signature collected but never processed
/// is passed over.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapState {
    pub full_history_completed: bool,
}

impl TransactionDatabase {
    /// Get the current bootstrap state
    pub async fn get_bootstrap_state(&self) -> Result<BootstrapState, Error> {
        let conn = self.get_connection()?;
        let mut state = BootstrapState::default();

        let result = conn
            .query_row(
                "SELECT full_history_completed FROM bootstrap_state WHERE chain_id = ?1 AND id = 1",
                params![self.chain.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?;

        if let Some(completed_i) = result {
            state.full_history_completed = completed_i != 0;
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
}
