// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Atomic outcome recording with the live spend it settles.

use chrono::Utc;
use rusqlite::{params, OptionalExtension, Transaction};

use crate::database::WriteTransaction;
use crate::trader::copy::types::CopyOutcome;
use crate::trader::error::Error;

use super::ledger::LIVE_SPEND;
use super::CopyDatabase;

impl CopyDatabase {
    /// Persist one outcome and, in the same transaction, commit its live spend --
    /// exactly once per decision, never on a status upgrade.
    /// `true` when the outcome is new (a first decision or a confirmation upgrade).
    pub async fn record_outcome(&self, outcome: CopyOutcome) -> crate::trader::Result<bool> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.record_outcome_sync(outcome))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    fn record_outcome_sync(&self, outcome: CopyOutcome) -> crate::trader::Result<bool> {
        let mut connection = self.connection()?;
        let transaction = connection
            .write_tx()
            .map_err(crate::errors::DatabaseError::from)?;
        let (task_id, signature, mint, decided_at, kind) = match &outcome {
            CopyOutcome::PaperFilled(decision) => (
                decision.task_id,
                decision.signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "paper_filled",
            ),
            CopyOutcome::LiveSubmitted(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_submitted",
            ),
            CopyOutcome::LiveConfirmed(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_confirmed",
            ),
            CopyOutcome::LiveFailed(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_failed",
            ),
            CopyOutcome::PaperSellObserved(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "paper_sell_observed",
            ),
            CopyOutcome::LiveSellSubmitted(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_sell_submitted",
            ),
            CopyOutcome::LiveSellFailed(decision) => (
                decision.task_id,
                decision.target_signature.as_str(),
                Some(decision.mint.as_str()),
                decision.telemetry.decided_at,
                "live_sell_failed",
            ),
            CopyOutcome::Skipped {
                task_id,
                signature,
                mint,
                decided_at,
                ..
            } => (
                *task_id,
                signature.as_str(),
                mint.as_deref(),
                *decided_at,
                "skipped",
            ),
        };
        let json = serde_json::to_string(&outcome).map_err(|e| Error::CopySerialize {
            field: "outcome",
            detail: e.to_string(),
        })?;
        let inserted = transaction
            .execute(
                "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(task_id, signature) DO NOTHING",
                params![task_id, signature, mint, json, decided_at.to_rfc3339()],
            )
            .map_err(crate::errors::DatabaseError::from)?
            > 0;
        let existing = if inserted {
            None
        } else {
            transaction
                .query_row(
                    "SELECT outcome_json FROM copy_decisions WHERE task_id=?1 AND signature=?2",
                    params![task_id, signature],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(crate::errors::DatabaseError::from)?
                .and_then(|value| serde_json::from_str::<CopyOutcome>(&value).ok())
        };
        let confirmation_upgrade = matches!(
            (&existing, &outcome),
            (
                Some(CopyOutcome::LiveSubmitted(_)),
                CopyOutcome::LiveConfirmed(_)
            )
        );
        if confirmation_upgrade {
            transaction
                .execute(
                    "UPDATE copy_decisions SET outcome_json=?3, decided_at=?4 WHERE task_id=?1 AND signature=?2",
                    params![task_id, signature, json, decided_at.to_rfc3339()],
                )
                .map_err(crate::errors::DatabaseError::from)?;
        }
        if inserted || confirmation_upgrade {
            transaction
                .execute(
                    "INSERT INTO copy_activity (task_id, kind, details_json, created_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![task_id, kind, json, Utc::now().to_rfc3339()],
                )
                .map_err(crate::errors::DatabaseError::from)?;
            if inserted {
                book_outcome(&transaction, &outcome)?;
            }
        }
        let claim_state = match &outcome {
            CopyOutcome::LiveSubmitted(_) | CopyOutcome::LiveSellSubmitted(_) => Some("submitted"),
            CopyOutcome::LiveConfirmed(_)
            | CopyOutcome::LiveFailed(_)
            | CopyOutcome::LiveSellFailed(_)
            | CopyOutcome::Skipped { .. } => Some("settled"),
            _ => None,
        };
        if let Some(state) = claim_state {
            transaction.execute(
                "UPDATE copy_live_claims SET state=?3, updated_at=?4 WHERE task_id=?1 AND signature=?2",
                params![task_id, signature, state, Utc::now().to_rfc3339()],
            ).map_err(crate::errors::DatabaseError::from)?;
        }
        transaction
            .commit()
            .map(|()| inserted || confirmation_upgrade)
            .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
    }
}

/// Apply a newly recorded decision to the live spend ledger. A paper decision books
/// nothing here: its round is already in the shared paper book, which its spend is read
/// from.
fn book_outcome(transaction: &Transaction<'_>, outcome: &CopyOutcome) -> crate::trader::Result<()> {
    match outcome {
        CopyOutcome::LiveSubmitted(decision) | CopyOutcome::LiveConfirmed(decision) => {
            add_live_spend(
                transaction,
                decision.task_id,
                &decision.mint,
                decision.sized_sol,
            )
        }
        _ => Ok(()),
    }
}

fn add_live_spend(
    transaction: &Transaction<'_>,
    task_id: i64,
    mint: &str,
    spent_native: f64,
) -> crate::trader::Result<()> {
    transaction
        .execute(
            "INSERT INTO copy_spend (task_id, mode, mint, spent_native, buy_count, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 1, ?5) \
             ON CONFLICT(task_id, mode, mint) DO UPDATE SET \
             spent_native = spent_native + excluded.spent_native, \
             buy_count = buy_count + 1, updated_at = excluded.updated_at",
            params![
                task_id,
                LIVE_SPEND,
                mint,
                spent_native,
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}
