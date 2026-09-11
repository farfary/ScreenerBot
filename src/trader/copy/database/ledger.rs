//! Mode-scoped spend, the paper position book, and atomic outcome recording.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::database::WriteTransaction;
use crate::trader::copy::types::{CopyMode, CopyOutcome, PaperDecision, PaperPosition, SpendState};
use crate::trader::error::Error;

use super::rows::parse_datetime;
use super::CopyDatabase;

/// Remaining holdings below this fraction of the position count as fully sold,
/// so float residue never keeps a closed paper position open.
const CLOSE_RESIDUE_FRACTION: f64 = 1e-9;

fn mode_key(mode: CopyMode) -> &'static str {
    match mode {
        CopyMode::Paper => "paper",
        CopyMode::Live => "live",
    }
}

const PAPER_POSITION_COLUMNS: &str = "task_id, mint, token_amount, cost_basis_sol, invested_sol, \
     realized_proceeds_sol, realized_cost_sol, buys, sells, last_price_sol, last_price_at, \
     opened_at, closed_at, peak_price_sol";

impl CopyDatabase {
    /// Spend in one execution mode. Paper and live budgets are separate ledgers:
    /// simulated fills never consume the budget a live task may spend.
    pub async fn spend_state(
        &self,
        task_id: i64,
        mode: CopyMode,
        mint: &str,
    ) -> crate::trader::Result<SpendState> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || db.spend_state_sync(task_id, mode, &mint))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    pub async fn task_total_spent(
        &self,
        task_id: i64,
        mode: CopyMode,
    ) -> crate::trader::Result<f64> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT COALESCE(SUM(spent_sol), 0) FROM copy_spend WHERE task_id = ?1 AND mode = ?2",
                    params![task_id, mode_key(mode)],
                    |row| row.get(0),
                )
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    fn spend_state_sync(
        &self,
        task_id: i64,
        mode: CopyMode,
        mint: &str,
    ) -> crate::trader::Result<SpendState> {
        let connection = self.connection()?;
        let total_spent_sol: f64 = connection
            .query_row(
                "SELECT COALESCE(SUM(spent_sol), 0) FROM copy_spend WHERE task_id = ?1 AND mode = ?2",
                params![task_id, mode_key(mode)],
                |row| row.get(0),
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let token = connection
            .query_row(
                "SELECT spent_sol, buy_count FROM copy_spend WHERE task_id = ?1 AND mode = ?2 AND mint = ?3",
                params![task_id, mode_key(mode), mint],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, u64>(1)?)),
            )
            .optional()
            .map_err(crate::errors::DatabaseError::from)?
            .unwrap_or_default();
        Ok(SpendState {
            total_spent_sol,
            token_spent_sol: token.0,
            token_buy_count: token.1,
        })
    }

    /// Every paper position a task has held, open ones first.
    pub async fn paper_positions(&self, task_id: i64) -> crate::trader::Result<Vec<PaperPosition>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            let mut statement = connection
                .prepare(&format!(
                    "SELECT {PAPER_POSITION_COLUMNS} FROM copy_paper_positions \
                     WHERE task_id = ?1 ORDER BY closed_at IS NOT NULL, opened_at DESC"
                ))
                .map_err(crate::errors::DatabaseError::from)?;
            let rows = statement
                .query_map(params![task_id], row_to_paper_position)
                .map_err(crate::errors::DatabaseError::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(crate::errors::DatabaseError::from)?;
            Ok(rows)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Raise an open paper holding's running peak to `price_sol` when it is higher.
    pub async fn raise_paper_peak(
        &self,
        task_id: i64,
        mint: &str,
        price_sol: f64,
    ) -> crate::trader::Result<()> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .execute(
                    "UPDATE copy_paper_positions \
                     SET peak_price_sol = MAX(COALESCE(peak_price_sol, ?3), ?3) \
                     WHERE task_id = ?1 AND mint = ?2 AND closed_at IS NULL",
                    params![task_id, mint, price_sol],
                )
                .map(|_| ())
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    pub async fn paper_position(
        &self,
        task_id: i64,
        mint: &str,
    ) -> crate::trader::Result<Option<PaperPosition>> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    &format!(
                        "SELECT {PAPER_POSITION_COLUMNS} FROM copy_paper_positions \
                         WHERE task_id = ?1 AND mint = ?2"
                    ),
                    params![task_id, mint],
                    row_to_paper_position,
                )
                .optional()
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

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
                CopyMode::Paper,
                &decision.mint,
                decision.sized_sol,
            )?;
            apply_paper_buy(transaction, decision)
        }
        CopyOutcome::LiveSubmitted(decision) | CopyOutcome::LiveConfirmed(decision) => add_spend(
            transaction,
            decision.task_id,
            CopyMode::Live,
            &decision.mint,
            decision.sized_sol,
        ),
        CopyOutcome::PaperSellObserved(decision) => match &decision.paper_fill {
            Some(fill) => apply_paper_sell(
                transaction,
                decision.task_id,
                &decision.mint,
                fill.token_amount,
                fill.net_proceeds_sol,
                fill.market_price_sol,
                decided_at,
            ),
            None => Ok(()),
        },
        _ => Ok(()),
    }
}

fn add_spend(
    transaction: &Transaction<'_>,
    task_id: i64,
    mode: CopyMode,
    mint: &str,
    spent_sol: f64,
) -> crate::trader::Result<()> {
    transaction
        .execute(
            "INSERT INTO copy_spend (task_id, mode, mint, spent_sol, buy_count, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 1, ?5) \
             ON CONFLICT(task_id, mode, mint) DO UPDATE SET \
             spent_sol = spent_sol + excluded.spent_sol, \
             buy_count = buy_count + 1, updated_at = excluded.updated_at",
            params![
                task_id,
                mode_key(mode),
                mint,
                spent_sol,
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}

/// Book a paper fill: the tokens and their full cost (input plus fees) join the
/// position, reopening it when an earlier round was sold out.
pub(super) fn apply_paper_buy(
    transaction: &Transaction<'_>,
    decision: &PaperDecision,
) -> crate::trader::Result<()> {
    let at = decision.telemetry.decided_at.to_rfc3339();
    transaction
        .execute(
            "INSERT INTO copy_paper_positions (task_id, mint, token_amount, cost_basis_sol, invested_sol, \
             buys, last_price_sol, last_price_at, opened_at, updated_at, peak_price_sol) \
             VALUES (?1, ?2, ?3, ?4, ?4, 1, ?5, ?6, ?6, ?6, ?5) \
             ON CONFLICT(task_id, mint) DO UPDATE SET \
             token_amount = token_amount + excluded.token_amount, \
             cost_basis_sol = cost_basis_sol + excluded.cost_basis_sol, \
             invested_sol = invested_sol + excluded.invested_sol, \
             buys = buys + 1, \
             last_price_sol = excluded.last_price_sol, last_price_at = excluded.last_price_at, \
             opened_at = CASE WHEN closed_at IS NULL THEN opened_at ELSE excluded.opened_at END, \
             peak_price_sol = CASE WHEN closed_at IS NULL \
                 THEN MAX(COALESCE(peak_price_sol, excluded.peak_price_sol), excluded.peak_price_sol) \
                 ELSE excluded.peak_price_sol END, \
             closed_at = NULL, updated_at = excluded.updated_at",
            params![
                decision.task_id,
                decision.mint,
                decision.fill.token_amount,
                decision.fill.total_cost_sol,
                decision.fill.market_price_sol,
                at,
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}

/// Book a paper sell: the sold share of the cost basis moves to realized cost,
/// the net proceeds to realized proceeds, and a position sold down to float
/// residue closes.
fn apply_paper_sell(
    transaction: &Transaction<'_>,
    task_id: i64,
    mint: &str,
    token_amount: f64,
    net_proceeds_sol: f64,
    price_sol: f64,
    at: DateTime<Utc>,
) -> crate::trader::Result<()> {
    let Some((held, cost_basis)) = transaction
        .query_row(
            "SELECT token_amount, cost_basis_sol FROM copy_paper_positions \
             WHERE task_id = ?1 AND mint = ?2 AND closed_at IS NULL",
            params![task_id, mint],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
        )
        .optional()
        .map_err(crate::errors::DatabaseError::from)?
    else {
        return Ok(());
    };
    if held <= 0.0 {
        return Ok(());
    }
    let sold = token_amount.min(held);
    let cost_sold = cost_basis * sold / held;
    let remaining = held - sold;
    let closes = remaining <= held * CLOSE_RESIDUE_FRACTION;
    let at = at.to_rfc3339();
    transaction
        .execute(
            "UPDATE copy_paper_positions SET \
             token_amount = ?3, cost_basis_sol = ?4, \
             realized_proceeds_sol = realized_proceeds_sol + ?5, \
             realized_cost_sol = realized_cost_sol + ?6, \
             sells = sells + 1, last_price_sol = ?7, last_price_at = ?8, \
             closed_at = ?9, updated_at = ?8 \
             WHERE task_id = ?1 AND mint = ?2",
            params![
                task_id,
                mint,
                if closes { 0.0 } else { remaining },
                if closes { 0.0 } else { cost_basis - cost_sold },
                net_proceeds_sol,
                if closes { cost_basis } else { cost_sold },
                price_sol,
                at,
                closes.then_some(at.as_str()),
            ],
        )
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}

fn row_to_paper_position(row: &rusqlite::Row<'_>) -> rusqlite::Result<PaperPosition> {
    let optional_time = |index: usize| -> rusqlite::Result<Option<DateTime<Utc>>> {
        row.get::<_, Option<String>>(index)?
            .map(|value| parse_datetime(&value, index))
            .transpose()
    };
    Ok(PaperPosition {
        task_id: row.get(0)?,
        mint: row.get(1)?,
        token_amount: row.get(2)?,
        cost_basis_sol: row.get(3)?,
        invested_sol: row.get(4)?,
        realized_proceeds_sol: row.get(5)?,
        realized_cost_sol: row.get(6)?,
        buys: row.get(7)?,
        sells: row.get(8)?,
        last_price_sol: row.get(9)?,
        last_price_at: optional_time(10)?,
        opened_at: parse_datetime(&row.get::<_, String>(11)?, 11)?,
        closed_at: optional_time(12)?,
        peak_price_sol: row.get(13)?,
    })
}
