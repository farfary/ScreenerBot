// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Atomic outcome recording and settlement of copy spend and paper-book mutations.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::chains::RawAmount;
use crate::database::WriteTransaction;
use crate::trader::copy::types::{CopyOutcome, PaperDecision};
use crate::trader::error::Error;
use crate::trader::paper::PaperModel;
use crate::trader::paper::TradingMode;

use super::ledger::{mode_key, read_paper_epoch, spend_state_on};
use super::rows::task_on;
use super::CopyDatabase;

impl CopyDatabase {
    /// Persist one outcome and, in the same transaction, commit its spend and its
    /// paper-book effect -- exactly once per decision, never on a status upgrade.
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
        let paper_epoch = match &outcome {
            CopyOutcome::PaperFilled(decision) => {
                let metadata = decision
                    .fill
                    .quote
                    .as_ref()
                    .filter(|_| decision.fill.has_quote_provenance());
                let Some(metadata) = metadata else {
                    return Err(Error::CopyValidation {
                        detail: "runtime paper fills require quote provenance".to_owned(),
                    });
                };
                Some((decision.task_id, metadata.epoch))
            }
            CopyOutcome::PaperSellObserved(decision) => {
                let Some(metadata) = decision
                    .paper_fill
                    .as_ref()
                    .filter(|fill| fill.has_quote_provenance())
                    .and_then(|fill| fill.quote.as_ref())
                else {
                    return Err(Error::CopyValidation {
                        detail: "runtime paper sells require quote provenance".to_owned(),
                    });
                };
                Some((decision.task_id, metadata.epoch))
            }
            CopyOutcome::Skipped {
                task_id,
                paper: Some(context),
                ..
            } if context.model == PaperModel::Quote => Some((*task_id, context.epoch)),
            _ => None,
        };
        if let Some((task_id, queued_epoch)) = paper_epoch {
            let current_epoch = read_paper_epoch(&transaction, task_id)?;
            if current_epoch != queued_epoch {
                transaction
                    .commit()
                    .map_err(crate::errors::DatabaseError::from)?;
                return Ok(false);
            }
        }
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
        if inserted {
            if let CopyOutcome::PaperFilled(decision) = &outcome {
                validate_paper_buy_settlement(&transaction, self.chain, decision)?;
            }
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
                book_outcome(&transaction, &outcome, decided_at)?;
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

/// Apply a newly recorded decision to the spend ledger and the paper book.
fn book_outcome(
    transaction: &Transaction<'_>,
    outcome: &CopyOutcome,
    decided_at: DateTime<Utc>,
) -> crate::trader::Result<()> {
    match outcome {
        CopyOutcome::PaperFilled(decision) => {
            add_spend(
                transaction,
                decision.task_id,
                TradingMode::Paper,
                &decision.mint,
                decision.fill.total_cost_sol,
            )?;
            super::paper_book::apply_paper_buy(transaction, decision)
        }
        CopyOutcome::LiveSubmitted(decision) | CopyOutcome::LiveConfirmed(decision) => add_spend(
            transaction,
            decision.task_id,
            TradingMode::Live,
            &decision.mint,
            decision.sized_sol,
        ),
        CopyOutcome::PaperSellObserved(decision) => {
            let Some(fill) = &decision.paper_fill else {
                return Err(Error::CopyValidation {
                    detail: "runtime paper sells require quote provenance".to_owned(),
                });
            };
            super::paper_book::apply_paper_sell(
                transaction,
                decision.task_id,
                &decision.mint,
                fill,
                decision.take_profit_level,
                decided_at,
            )
        }
        _ => Ok(()),
    }
}

fn add_spend(
    transaction: &Transaction<'_>,
    task_id: i64,
    mode: TradingMode,
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
                mode_key(mode),
                mint,
                spent_native,
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}

fn validate_paper_buy_settlement(
    transaction: &rusqlite::Transaction<'_>,
    chain: crate::chains::ChainId,
    decision: &PaperDecision,
) -> crate::trader::Result<()> {
    let task = task_on(transaction, decision.task_id, chain.as_str())
        .map_err(crate::errors::DatabaseError::from)?
        .ok_or(Error::CopyTaskNotFound {
            task_id: decision.task_id,
        })?;
    if !task.enabled || task.mode != TradingMode::Paper {
        return Err(Error::CopyTaskRejected {
            reason: crate::trader::copy::CopySkip::TaskDisabled,
        });
    }

    let fill = &decision.fill;
    let metadata = fill
        .quote
        .as_ref()
        .filter(|_| fill.has_quote_provenance())
        .ok_or_else(|| Error::CopyValidation {
            detail: "runtime paper fills require quote provenance".to_owned(),
        })?;
    if !fill.input_sol.is_finite()
        || fill.input_sol <= 0.0
        || !fill.total_cost_sol.is_finite()
        || fill.total_cost_sol <= 0.0
    {
        return Err(Error::CopyValidation {
            detail: "paper buy input and full cost must be finite and positive".to_owned(),
        });
    }
    let raw_input = metadata
        .input_amount
        .filter(|amount| *amount != RawAmount::ZERO)
        .ok_or_else(|| Error::CopyValidation {
            detail: "paper buy quote has no positive raw native input".to_owned(),
        })?;
    let adapter = crate::chains::adapter_for(chain);
    let quote_input = raw_input.to_whole_units(adapter.native_asset_decimals());
    if !quote_input.is_finite() || (quote_input - fill.input_sol).abs() > f64::EPSILON {
        return Err(Error::CopyValidation {
            detail: "paper buy input does not match quote provenance".to_owned(),
        });
    }

    let spent = spend_state_on(
        transaction,
        decision.task_id,
        TradingMode::Paper,
        &decision.mint,
    )?;
    if !spent.total_spent_native.is_finite()
        || spent.total_spent_native + fill.total_cost_sol > task.total_budget_native
    {
        return Err(Error::CopyTaskRejected {
            reason: crate::trader::copy::CopySkip::BudgetExhausted,
        });
    }
    if !spent.token_spent_native.is_finite()
        || spent.token_spent_native + fill.total_cost_sol > task.max_native_per_token
    {
        return Err(Error::CopyTaskRejected {
            reason: crate::trader::copy::CopySkip::TokenCapReached,
        });
    }
    if quote_input > task.max_native_per_trade {
        return Err(Error::CopyTaskRejected {
            reason: crate::trader::copy::CopySkip::PaperExecutionUnavailable,
        });
    }
    Ok(())
}
