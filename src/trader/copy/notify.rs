// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
use crate::i18n::{ids, UiArg, UiText};
use crate::telegram::{Notification, NotificationType};

use super::{CopyDatabase, CopyOutcome, CopyPauseReason, CopyTask, PaperExitRule};

/// Notices the header keeps for its poll; older ones have been shown already.
const NOTICE_CAPACITY: usize = 20;

#[derive(Debug, Clone, Serialize)]
pub struct CopyNotice {
    /// Monotonic per process; the dashboard shows each sequence number once.
    pub seq: u64,
    pub task_id: i64,
    /// The task's label or shortened address; `None` when the task no longer
    /// exists, in which case renderers show [`task_arg`].
    pub task: Option<String>,
    pub title: UiText,
    pub detail: UiText,
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
    title: UiText,
    detail: UiText,
    paper: bool,
}

/// Message argument naming a task: its name, or the localized "Task #id".
pub fn task_arg(task: Option<&str>, task_id: i64) -> UiArg {
    match task {
        Some(name) => UiArg::Text(name.to_owned()),
        None => UiArg::Nested(Box::new(
            UiText::new(ids::COPY_NOTICE_TASK_UNNAMED).arg("id", UiArg::Text(task_id.to_string())),
        )),
    }
}

/// A task's name as the dashboard writes it: its label, or the wallet address
/// shortened to its first five and last four characters.
pub fn task_name(task: &CopyTask) -> String {
    task.label.clone().unwrap_or_else(|| {
        let address = &task.target_address;
        if address.len() <= 12 || !address.is_ascii() {
            return address.clone();
        }
        format!("{}…{}", &address[..5], &address[address.len() - 4..])
    })
}

fn exit_title(rule: Option<PaperExitRule>) -> UiText {
    match rule {
        None => UiText::new(ids::COPY_NOTICE_TITLE_PAPER_SELL),
        Some(PaperExitRule::Manual) => UiText::new(ids::COPY_NOTICE_TITLE_PAPER_CLOSED),
        Some(rule) => UiText::new(ids::COPY_NOTICE_TITLE_PAPER_EXIT)
            .arg("rule", UiArg::Nested(Box::new(rule.label_text()))),
    }
}

/// SOL amount at the four-decimal precision the notices have always shown.
fn sol(amount: f64) -> UiArg {
    UiArg::Sol(format!("{amount:.4}"))
}

fn failure_detail(error: &Option<String>) -> UiText {
    match error {
        Some(error) => {
            UiText::new(ids::COPY_NOTICE_DETAIL_ERROR).arg("error", UiArg::Text(error.clone()))
        }
        None => UiText::new(ids::COPY_NOTICE_DETAIL_SWAP_FAILED),
    }
}

fn announcement(outcome: &CopyOutcome) -> Option<Announcement> {
    let entry = |task_id, mint: &str, title: UiText, detail: UiText, paper, warning| Announcement {
        task_id,
        subtype: if warning { "copy_failed" } else { "copy_trade" },
        warning,
        mint: Some(mint.to_owned()),
        title,
        detail,
        paper,
    };
    let sized = |amount: f64| UiText::new(ids::COPY_NOTICE_DETAIL_SIZED).arg("amount", sol(amount));
    Some(match outcome {
        CopyOutcome::PaperFilled(d) => entry(
            d.task_id,
            &d.mint,
            UiText::new(ids::COPY_NOTICE_TITLE_PAPER_BUY),
            UiText::new(ids::COPY_NOTICE_DETAIL_BOUGHT).arg("amount", sol(d.fill.total_cost_sol)),
            true,
            false,
        ),
        CopyOutcome::LiveSubmitted(d) => entry(
            d.task_id,
            &d.mint,
            UiText::new(ids::COPY_NOTICE_TITLE_LIVE_BUY_SUBMITTED),
            sized(d.sized_sol),
            false,
            false,
        ),
        CopyOutcome::LiveConfirmed(d) => entry(
            d.task_id,
            &d.mint,
            UiText::new(ids::COPY_NOTICE_TITLE_LIVE_BUY_CONFIRMED),
            sized(d.sized_sol),
            false,
            false,
        ),
        CopyOutcome::LiveFailed(d) => entry(
            d.task_id,
            &d.mint,
            UiText::new(ids::COPY_NOTICE_TITLE_LIVE_BUY_FAILED),
            failure_detail(&d.error),
            false,
            true,
        ),
        CopyOutcome::PaperSellObserved(d) => {
            let fill = d.paper_fill.as_ref()?;
            entry(
                d.task_id,
                &d.mint,
                exit_title(d.exit_rule),
                UiText::new(ids::COPY_NOTICE_DETAIL_SOLD).arg("amount", sol(fill.net_proceeds_sol)),
                true,
                false,
            )
        }
        CopyOutcome::LiveSellSubmitted(d) => entry(
            d.task_id,
            &d.mint,
            UiText::new(ids::COPY_NOTICE_TITLE_LIVE_SELL_SUBMITTED),
            match d.exit_percentage {
                Some(pct) => UiText::new(ids::COPY_NOTICE_DETAIL_PARTIAL_CLOSE)
                    .arg("percent", UiArg::Text(format!("{pct:.1}"))),
                None => UiText::new(ids::COPY_NOTICE_DETAIL_FULL_CLOSE),
            },
            false,
            false,
        ),
        CopyOutcome::LiveSellFailed(d) => entry(
            d.task_id,
            &d.mint,
            UiText::new(ids::COPY_NOTICE_TITLE_LIVE_SELL_FAILED),
            failure_detail(&d.error),
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
            publish(task.as_ref().map(task_name), announcement).await;
        }
    }
    Ok(())
}

/// Announce a guard pausing a task.
pub(super) async fn announce_pause(task: &CopyTask, reason: &CopyPauseReason) {
    publish(
        Some(task_name(task)),
        Announcement {
            task_id: task.id,
            subtype: "copy_task_paused",
            warning: true,
            mint: None,
            title: UiText::new(ids::COPY_NOTICE_TITLE_AUTO_PAUSED),
            detail: reason.ui_text(),
            paper: false,
        },
    )
    .await;
}

async fn publish(task: Option<String>, announcement: Announcement) {
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
        crate::events::with_text(
            json!({ "task_id": task_id, "task": task, "title": title, "detail": detail, "paper": paper }),
            &UiText::new(ids::COPY_NOTICE_EVENT)
                .arg("task", task_arg(task.as_deref(), task_id))
                .arg("title", UiArg::Nested(Box::new(title.clone())))
                .arg("detail", UiArg::Nested(Box::new(detail.clone()))),
        ),
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
        Some(mint) => match crate::chains::chain_for_address(mint) {
            Ok(chain) => crate::tokens::get_token_info_batch_async(chain, vec![mint.clone()])
                .await
                .ok(),
            Err(_) => None,
        }
        .and_then(|mut info| info.remove(mint))
        .and_then(|(symbol, _, _)| symbol),
        None => None,
    };
    crate::telegram::notifier::queue_notification(Notification::new(
        NotificationType::CopyTrading {
            task,
            task_id,
            title,
            token_symbol,
            token_mint: mint,
            detail,
            paper,
        },
    ));
}
