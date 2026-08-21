//! Transaction database bootstrap — state tracking for historical backfill.
//
// Bootstrap state and reconciliation operations

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::operations::TransactionDatabase;

// =============================================================================
// IMPLEMENTATION - BOOTSTRAP STATE AND RECONCILIATION
// =============================================================================

/// Bootstrap state structure for resuming backfill across restarts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapState {
    pub backfill_before_cursor: Option<String>,
    pub full_history_completed: bool,
}

impl TransactionDatabase {
    /// Get the current bootstrap state
    pub async fn get_bootstrap_state(&self) -> Result<BootstrapState, String> {
        let conn = self.get_connection()?;
        let mut state = BootstrapState::default();

        let result = conn
            .query_row(
                "SELECT backfill_before_cursor, full_history_completed FROM bootstrap_state WHERE chain_id = ?1 AND id = 1",
                params![self.chain.as_str()],
                |row| {
                    let cursor: Option<String> = row.get(0)?;
                    let completed_i: i64 = row.get(1)?;
                    Ok((cursor, completed_i))
                }
            )
            .optional()
            .map_err(|e| format!("Failed to load bootstrap_state: {e}"))?;

        if let Some((cursor, completed_i)) = result {
            state.backfill_before_cursor = cursor;
            state.full_history_completed = completed_i != 0;
        }

        Ok(state)
    }

    /// Update the backfill cursor (the `before` parameter for next page)
    pub async fn set_backfill_cursor(&self, cursor: Option<&str>) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (chain_id, id, full_history_completed) VALUES (?1, 1, 0)",
            params![self.chain.as_str()],
        )
        .map_err(|e| format!("Failed to ensure bootstrap_state row: {e}"))?;

        conn
            .execute(
                "UPDATE bootstrap_state SET backfill_before_cursor = ?1, updated_at = datetime('now') WHERE chain_id = ?2 AND id = 1",
                params![cursor, self.chain.as_str()]
            )
            .map_err(|e| format!("Failed to update backfill cursor: {e}"))?;
        Ok(())
    }

    /// Clear the backfill cursor
    pub async fn clear_backfill_cursor(&self) -> Result<(), String> {
        self.set_backfill_cursor(None).await
    }

    /// Mark the full history as completed
    pub async fn mark_full_history_completed(&self) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn
            .execute(
                "UPDATE bootstrap_state SET full_history_completed = 1, updated_at = datetime('now') WHERE chain_id = ?1 AND id = 1",
                params![self.chain.as_str()]
            )
            .map_err(|e| format!("Failed to mark full history completed: {e}"))?;
        Ok(())
    }

    /// Reconcile known_signatures with already processed transactions
    /// Ensures no processed transaction is missing from known_signatures, for any
    /// subject -- both columns of the composite key must be carried across, or every
    /// row would violate the `wallet_address NOT NULL` constraint and silently
    /// reconcile nothing (`INSERT OR IGNORE` swallows constraint failures).
    pub async fn reconcile_known_with_processed(&self) -> Result<usize, String> {
        let conn = self.get_connection()?;
        let affected = conn
            .execute(
                "INSERT OR IGNORE INTO known_signatures(chain_id, signature, wallet_address) SELECT chain_id, signature, wallet_address FROM processed_transactions WHERE chain_id = ?1",
                params![self.chain.as_str()]
            )
            .map_err(|e| format!("Failed to reconcile known signatures: {e}"))?;
        Ok(affected as usize)
    }
}
