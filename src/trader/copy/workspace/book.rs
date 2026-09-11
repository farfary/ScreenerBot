//! Paper-book maintenance behind the workspace: clone a task, reset its paper
//! book, and close (or write off) one paper holding by hand.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::super::control::{self, open_database};
use super::super::paper_exits::paper_exit_outcome;
use super::super::service::paper_costs;
use super::super::{
    notify, CopyMode, CopyOutcome, CopySellDecision, CopyTask, CopyTaskInput, CopyTelemetry,
    PaperExitRule, PaperPosition, PaperSellFill,
};
use crate::trader::{Error, Result};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloneRequest {
    /// Another wallet to copy with the same rules; the same wallet when absent.
    #[serde(default)]
    pub target_address: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    /// Clones start paused unless asked otherwise.
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// A new paper task with this task's rules, through the normal create path.
pub async fn clone_task(id: i64, request: CloneRequest) -> Result<CopyTask> {
    let original = control::get_task(id).await?;
    let mut input = CopyTaskInput::from(&original);
    input.mode = CopyMode::Paper;
    if let Some(address) = request
        .target_address
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty())
    {
        input.target_address = address;
    }
    input.label = request
        .label
        .map(|label| label.trim().to_owned())
        .filter(|label| !label.is_empty())
        .or_else(|| Some(format!("{} (copy)", notify::task_name(&original))));
    input.enabled = request.enabled.unwrap_or(false);
    control::create_task(input).await
}

#[derive(Debug, Serialize)]
pub struct ResetResult {
    pub task_id: i64,
    pub removed_decisions: usize,
}

/// Start a paper task's evaluation over. Refused for a live task, whose history
/// is the record of real trades.
pub async fn reset_paper_book(id: i64) -> Result<ResetResult> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.mode != CopyMode::Paper {
        return Err(Error::CopyValidation {
            detail: "return the task to paper before resetting its paper book".to_owned(),
        });
    }
    let removed_decisions = db.reset_paper_book(id).await?;
    Ok(ResetResult {
        task_id: id,
        removed_decisions,
    })
}

#[derive(Debug, Serialize)]
pub struct ClosedHolding {
    pub task_id: i64,
    pub mint: String,
    /// No pool price: the holding was closed at zero proceeds.
    pub written_off: bool,
    pub mark_price_sol: Option<f64>,
}

/// An unpriced holding cannot be sold at a price that does not exist, so the
/// book closes it at zero and realizes its whole cost basis as the loss.
fn write_off(task: &CopyTask, position: &PaperPosition, now: DateTime<Utc>) -> CopyOutcome {
    CopyOutcome::PaperSellObserved(CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: format!(
            "paper-writeoff:{}:{}:{}",
            position.mint,
            position.opened_at.timestamp(),
            position.sells
        ),
        mint: position.mint.clone(),
        target_token_amount: 0.0,
        target_sol_amount: 0.0,
        exit_percentage: None,
        transaction_signature: None,
        error: None,
        telemetry: CopyTelemetry {
            target_block_time: None,
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: None,
            backfill: false,
        },
        paper_fill: Some(PaperSellFill {
            token_amount: position.token_amount,
            market_price_sol: 0.0,
            fill_price_sol: 0.0,
            gross_sol: 0.0,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            net_proceeds_sol: 0.0,
        }),
        exit_rule: Some(PaperExitRule::Manual),
    })
}

/// Sell a paper holding by hand at the pool price, like an exit rule would; a
/// holding without a pool price is written off.
pub async fn close_paper_holding(id: i64, mint: &str) -> Result<ClosedHolding> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let position = db
        .paper_position(id, mint)
        .await?
        .filter(PaperPosition::is_open)
        .ok_or_else(|| Error::CopyHoldingNotFound {
            task_id: id,
            mint: mint.to_owned(),
        })?;
    let now = Utc::now();
    let mark = control::paper_mark(&position).filter(|price| price.is_finite() && *price > 0.0);
    let outcome = match mark {
        Some(mark) => paper_exit_outcome(
            &task,
            &position,
            PaperExitRule::Manual,
            None,
            mark,
            paper_costs(),
            now,
        )
        .map_err(|reason| Error::CopyTaskRejected { reason })?,
        None => write_off(&task, &position, now),
    };
    notify::record(&db, outcome).await?;
    Ok(ClosedHolding {
        task_id: id,
        mint: mint.to_owned(),
        written_off: mark.is_none(),
        mark_price_sol: mark,
    })
}
