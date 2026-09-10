//! Live-submission claims and observed target inventory.

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::database::WriteTransaction;
use crate::trader::copy::types::{CopyOutcome, CopySkip};
use crate::trader::error::Error;

use super::CopyDatabase;

impl CopyDatabase {
    /// Atomically claim a target activity before any live admission/submission work.
    /// A false result means another delivery already owns it and must never spend again.
    pub async fn claim_live_activity(
        &self,
        task_id: i64,
        signature: &str,
    ) -> crate::trader::Result<bool> {
        let db = self.clone();
        let signature = signature.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .execute(
                    "INSERT INTO copy_live_claims (task_id, signature, claimed_at, state, updated_at) VALUES (?1, ?2, ?3, 'claimed', ?3) ON CONFLICT(task_id, signature) DO NOTHING",
                    params![task_id, signature, Utc::now().to_rfc3339()],
                )
                .map(|affected| affected > 0)
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable { detail: e.to_string() })?
    }

    pub async fn target_holding(&self, task_id: i64, mint: &str) -> crate::trader::Result<f64> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT token_amount FROM copy_target_holdings WHERE task_id=?1 AND mint=?2",
                    params![task_id, mint],
                    |row| row.get(0),
                )
                .optional()
                .map(|value| value.unwrap_or_default())
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Idempotently apply one observed target token delta and return the holding
    /// immediately before it. This is observation state, independent of whether our
    /// copy decision fills, skips, or fails.
    pub async fn observe_target_inventory(
        &self,
        task_id: i64,
        signature: &str,
        mint: &str,
        token_delta: f64,
    ) -> crate::trader::Result<f64> {
        let db = self.clone();
        let signature = signature.to_owned();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut connection = db.connection()?;
            let transaction = connection.write_tx().map_err(|e| Error::CopyReconciliation { detail: format!("failed to begin target inventory update: {e}") })?;
            let before = transaction.query_row(
                "SELECT token_amount FROM copy_target_holdings WHERE task_id=?1 AND mint=?2",
                params![task_id, mint],
                |row| row.get::<_, f64>(0),
            ).optional().map_err(|e| Error::CopyReconciliation { detail: format!("failed to read target inventory: {e}") })?.unwrap_or_default();
            let inserted = transaction.execute(
                "INSERT INTO copy_target_events (task_id, signature, mint, token_delta, observed_at) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(task_id, signature) DO NOTHING",
                params![task_id, signature, mint, token_delta, Utc::now().to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to record target inventory event: {e}") })? > 0;
            if inserted {
                update_target_holding(&transaction, task_id, &mint, token_delta)?;
            }
            transaction.commit().map_err(|e| Error::CopyReconciliation { detail: format!("failed to commit target inventory update: {e}") })?;
            Ok(before)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable { detail: e.to_string() })?
    }

    /// Fail closed after a crash: stale claims are marked abandoned and surfaced as
    /// activity, but are never made spendable again because submission may have happened.
    pub async fn reconcile_stale_claims(&self, grace_seconds: u64) -> crate::trader::Result<usize> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.reconcile_stale_claims_sync(grace_seconds))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn reconcile_stale_claims_sync(&self, grace_seconds: u64) -> crate::trader::Result<usize> {
        let mut connection = self.connection()?;
        let transaction = connection
            .write_tx()
            .map_err(|e| Error::CopyReconciliation {
                detail: format!("failed to begin claim reconciliation: {e}"),
            })?;
        transaction.execute(
            "UPDATE copy_live_claims SET state='settled', updated_at=?1 WHERE state='claimed' AND EXISTS (SELECT 1 FROM copy_decisions d WHERE d.task_id=copy_live_claims.task_id AND d.signature=copy_live_claims.signature)",
            params![Utc::now().to_rfc3339()],
        ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to settle legacy copy claims: {e}") })?;
        let cutoff =
            Utc::now() - chrono::Duration::seconds(grace_seconds.min(i64::MAX as u64) as i64);
        let stale = {
            let mut statement = transaction
                .prepare("SELECT task_id, signature FROM copy_live_claims WHERE state='claimed' AND datetime(claimed_at) <= datetime(?1)")
                .map_err(|e| Error::CopyReconciliation { detail: format!("failed to prepare stale claim query: {e}") })?;
            let rows = statement
                .query_map(params![cutoff.to_rfc3339()], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| Error::CopyReconciliation {
                    detail: format!("failed to query stale claims: {e}"),
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::CopyReconciliation {
                    detail: format!("failed to decode stale claims: {e}"),
                })?;
            rows
        };
        for (task_id, signature) in &stale {
            let now = Utc::now();
            let outcome = CopyOutcome::Skipped {
                task_id: *task_id,
                signature: signature.clone(),
                mint: None,
                reason: CopySkip::ClaimReconciledAbandoned,
                decided_at: now,
                telemetry: None,
            };
            let json = serde_json::to_string(&outcome).map_err(|e| Error::CopySerialize {
                field: "reconciled_claim",
                detail: e.to_string(),
            })?;
            transaction.execute(
                "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) VALUES (?1, ?2, NULL, ?3, ?4) ON CONFLICT(task_id, signature) DO NOTHING",
                params![task_id, signature, json, now.to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to record reconciled claim decision: {e}") })?;
            transaction.execute(
                "INSERT INTO copy_activity (task_id, kind, details_json, created_at) VALUES (?1, 'claim_abandoned', ?2, ?3)",
                params![task_id, json, now.to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to record reconciled claim activity: {e}") })?;
            transaction.execute(
                "UPDATE copy_live_claims SET state='abandoned', updated_at=?3 WHERE task_id=?1 AND signature=?2",
                params![task_id, signature, now.to_rfc3339()],
            ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to abandon stale claim: {e}") })?;
        }
        transaction
            .commit()
            .map_err(|e| Error::CopyReconciliation {
                detail: format!("failed to commit claim reconciliation: {e}"),
            })?;
        Ok(stale.len())
    }
}

fn update_target_holding(
    transaction: &rusqlite::Transaction<'_>,
    task_id: i64,
    mint: &str,
    delta: f64,
) -> crate::trader::Result<()> {
    if !delta.is_finite() || delta == 0.0 {
        return Ok(());
    }
    transaction.execute(
        "INSERT INTO copy_target_holdings (task_id, mint, token_amount, updated_at) VALUES (?1, ?2, 0, ?3) ON CONFLICT(task_id, mint) DO NOTHING",
        params![task_id, mint, Utc::now().to_rfc3339()],
    ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to initialize target holding: {e}") })?;
    transaction.execute(
        "UPDATE copy_target_holdings SET token_amount=MAX(0, token_amount + ?3), updated_at=?4 WHERE task_id=?1 AND mint=?2",
        params![task_id, mint, delta, Utc::now().to_rfc3339()],
    ).map_err(|e| Error::CopyReconciliation { detail: format!("failed to update target holding: {e}") })?;
    Ok(())
}
