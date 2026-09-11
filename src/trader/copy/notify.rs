//! Announcements for copy decisions worth attention -- a copied fill, an exit, a
//! failed live trade, an auto-pause. Each lands in the event log, in the in-app
//! notice feed the dashboard header polls, and on Telegram when enabled. Skips
//! are recorded but never announced: a busy wallet skips far more than it trades.

use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;

use crate::events::Severity;
use crate::telegram::{Notification, NotificationType};

use super::insights::exit_label;
use super::{CopyDatabase, CopyOutcome, CopyPauseReason, CopyTask, PaperExitRule};

/// Notices the header keeps for its poll; older ones have been shown already.
const NOTICE_CAPACITY: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct CopyNotice {
    /// Monotonic per process; the dashboard shows each sequence number once.
    pub seq: u64,
    pub task_id: i64,
    pub task: String,
    pub title: String,
    pub detail: String,
    pub mint: Option<String>,
    pub paper: bool,
    pub warning: bool,
    pub at: DateTime<Utc>,
}

#[derive(Default)]
struct NoticeFeed {
    next_seq: u64,
    notices: VecDeque<CopyNotice>,
}

static FEED: LazyLock<Mutex<NoticeFeed>> = LazyLock::new(Default::default);

/// The newest notices, oldest first.
pub fn recent_notices() -> Vec<CopyNotice> {
    FEED.lock()
        .map(|feed| feed.notices.iter().cloned().collect())
        .unwrap_or_default()
}

struct Announcement {
    task_id: i64,
    subtype: &'static str,
    warning: bool,
    mint: Option<String>,
    title: String,
    detail: String,
    paper: bool,
}

pub fn task_name(task: &CopyTask) -> String {
    task.label.clone().unwrap_or_else(|| {
        let address = &task.target_address;
        format!(
            "{}...{}",
            &address[..address.len().min(6)],
            &address[address.len().saturating_sub(4)..]
        )
    })
}

fn exit_title(rule: Option<PaperExitRule>) -> String {
    match rule {
        None => "Paper copy sell".to_owned(),
        Some(PaperExitRule::Manual) => "Paper holding closed".to_owned(),
        Some(rule) => format!("Paper exit: {}", exit_label(Some(rule)).replace('_', " ")),
    }
}

fn announcement(outcome: &CopyOutcome) -> Option<Announcement> {
    let entry = |task_id, mint: &str, title: &str, detail: String, paper, warning| Announcement {
        task_id,
        subtype: if warning { "copy_failed" } else { "copy_trade" },
        warning,
        mint: Some(mint.to_owned()),
        title: title.to_owned(),
        detail,
        paper,
    };
    Some(match outcome {
        CopyOutcome::PaperFilled(d) => entry(
            d.task_id,
            &d.mint,
            "Paper copy buy",
            format!("Bought for {:.4} SOL", d.fill.total_cost_sol),
            true,
            false,
        ),
        CopyOutcome::LiveSubmitted(d) => entry(
            d.task_id,
            &d.mint,
            "Live copy buy submitted",
            format!("{:.4} SOL", d.sized_sol),
            false,
            false,
        ),
        CopyOutcome::LiveConfirmed(d) => entry(
            d.task_id,
            &d.mint,
            "Live copy buy confirmed",
            format!("{:.4} SOL", d.sized_sol),
            false,
            false,
        ),
        CopyOutcome::LiveFailed(d) => entry(
            d.task_id,
            &d.mint,
            "Live copy buy failed",
            d.error.clone().unwrap_or_else(|| "Swap failed".to_owned()),
            false,
            true,
        ),
        CopyOutcome::PaperSellObserved(d) => {
            let fill = d.paper_fill.as_ref()?;
            entry(
                d.task_id,
                &d.mint,
                &exit_title(d.exit_rule),
                format!("Sold for {:.4} SOL", fill.net_proceeds_sol),
                true,
                false,
            )
        }
        CopyOutcome::LiveSellSubmitted(d) => entry(
            d.task_id,
            &d.mint,
            "Live copy sell submitted",
            match d.exit_percentage {
                Some(pct) => format!("{pct:.1}% of the holding"),
                None => "Full close".to_owned(),
            },
            false,
            false,
        ),
        CopyOutcome::LiveSellFailed(d) => entry(
            d.task_id,
            &d.mint,
            "Live copy sell failed",
            d.error.clone().unwrap_or_else(|| "Swap failed".to_owned()),
            false,
            true,
        ),
        CopyOutcome::Skipped { .. } => return None,
    })
}

/// Record an outcome, announcing it when it is new. Every runtime path records
/// through here so a decision is announced exactly once.
pub(super) async fn record(
    database: &CopyDatabase,
    outcome: CopyOutcome,
) -> crate::trader::Result<()> {
    let announcement = announcement(&outcome);
    if database.record_outcome(outcome).await? {
        if let Some(announcement) = announcement {
            let task = database.get_task(announcement.task_id).await.ok().flatten();
            let name = task
                .as_ref()
                .map(task_name)
                .unwrap_or_else(|| format!("Task {}", announcement.task_id));
            publish(name, announcement).await;
        }
    }
    Ok(())
}

/// Announce a guard pausing a task.
pub(super) async fn announce_pause(task: &CopyTask, reason: &CopyPauseReason) {
    let detail = match reason {
        CopyPauseReason::User => "Paused by the user".to_owned(),
        CopyPauseReason::LatencyKillSwitch {
            average_ms,
            threshold_ms,
        } => format!(
            "Target trades arrived {:.1}s late on average (limit {:.1}s)",
            *average_ms as f64 / 1000.0,
            *threshold_ms as f64 / 1000.0
        ),
        CopyPauseReason::WatchDetached => "The target wallet is no longer watched".to_owned(),
    };
    publish(
        task_name(task),
        Announcement {
            task_id: task.id,
            subtype: "copy_task_paused",
            warning: true,
            mint: None,
            title: "Copy task auto-paused".to_owned(),
            detail,
            paper: false,
        },
    )
    .await;
}

async fn publish(task: String, announcement: Announcement) {
    let Announcement {
        task_id,
        subtype,
        warning,
        mint,
        title,
        detail,
        paper,
    } = announcement;
    crate::events::record_trader_event(
        subtype,
        if warning { Severity::Warn } else { Severity::Info },
        mint.as_deref(),
        Some(&task_id.to_string()),
        json!({ "task_id": task_id, "task": task, "title": title, "detail": detail, "paper": paper }),
    )
    .await;
    if let Ok(mut feed) = FEED.lock() {
        feed.next_seq += 1;
        let seq = feed.next_seq;
        feed.notices.push_back(CopyNotice {
            seq,
            task_id,
            task: task.clone(),
            title: title.clone(),
            detail: detail.clone(),
            mint: mint.clone(),
            paper,
            warning,
            at: Utc::now(),
        });
        while feed.notices.len() > NOTICE_CAPACITY {
            feed.notices.pop_front();
        }
    }
    let token_symbol = match &mint {
        Some(mint) => crate::tokens::get_token_info_batch_async(vec![mint.clone()])
            .await
            .ok()
            .and_then(|mut info| info.remove(mint))
            .and_then(|(symbol, _, _)| symbol),
        None => None,
    };
    crate::telegram::notifier::queue_notification(Notification::new(
        NotificationType::CopyTrading {
            task,
            title,
            token_symbol,
            token_mint: mint,
            detail,
            paper,
        },
    ));
}
