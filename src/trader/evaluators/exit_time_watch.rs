// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Which time exit watches an open position and when it comes due, from the policy the exit monitor resolves for it.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::positions::{Position, PositionManagement};
use crate::trader::evaluators::{exit_time, exit_time_stagnation};
use crate::trader::policy::ExitPolicy;

/// A time exit rule, named as the exit monitor names the sale it makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeExitRule {
    /// Held past the time limit at or below the loss threshold.
    TimeLoss,
    /// Held past the stagnation time with profit or loss inside the flat band.
    TimeStagnation,
}

/// What the time exits do with one open position as it stands now.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TimeExitWatch {
    /// The exit monitor runs no policy exit on this position; `management` says who
    /// owns its exits.
    NotManaged { management: PositionManagement },
    /// Neither time rule is enabled in the position's policy.
    Off,
    /// The position has no usable entry or mark to measure its profit or loss from.
    Unpriced,
    /// A time rule is enabled, but the position is outside the condition of each one.
    Clear,
    /// `rule` sells the position at `due_at` (Unix seconds) if its condition still holds
    /// then; a past `due_at` means on the next check. `pnl_pct` is the profit or loss from
    /// the average entry the rule measures.
    Due {
        rule: TimeExitRule,
        due_at: i64,
        pnl_pct: f64,
    },
}

/// The time exit that applies to `position` under `policy` at `now`.
///
/// Each rule's own predicate decides whether the position meets its price condition,
/// asked as of the moment its holding time is reached, so the watch and the exit monitor
/// share one definition of each rule. When both rules apply, the one due first is
/// reported, the loss rule on a tie, which is the order the monitor checks them in. A
/// rule whose configuration is impossible never fires, so it never applies here either.
pub fn time_exit_watch(
    position: &Position,
    policy: &ExitPolicy,
    now: DateTime<Utc>,
) -> TimeExitWatch {
    if !position.management.allows_policy_exits() {
        return TimeExitWatch::NotManaged {
            management: position.management,
        };
    }
    if !policy.time.enabled && !policy.time_stagnation.enabled {
        return TimeExitWatch::Off;
    }
    let usable = |price: f64| price.is_finite() && price > 0.0;
    let entry = position.average_entry_price;
    let Some(current) = position.current_price.filter(|price| usable(*price)) else {
        return TimeExitWatch::Unpriced;
    };
    if !usable(entry) {
        return TimeExitWatch::Unpriced;
    }
    let held = (now - position.entry_time).num_seconds();

    let loss = holding_seconds(policy.time.duration_seconds).filter(|&due| {
        exit_time::time_override_triggered(entry, current, held.max(due), &policy.time)
            .unwrap_or(false)
    });
    let flat = holding_seconds(policy.time_stagnation.duration_seconds).filter(|&due| {
        exit_time_stagnation::stagnation_triggered(
            entry,
            current,
            held.max(due),
            &policy.time_stagnation,
        )
        .unwrap_or(false)
    });
    // A due moment past what a timestamp can hold is never reached.
    let due_at = |due: i64| {
        Duration::try_seconds(due)
            .and_then(|span| position.entry_time.checked_add_signed(span))
            .map(|at| at.timestamp())
    };
    // `min_by_key` keeps the first of equal keys: the loss rule wins a tie.
    let Some((rule, due_at)) = [
        (TimeExitRule::TimeLoss, loss),
        (TimeExitRule::TimeStagnation, flat),
    ]
    .into_iter()
    .filter_map(|(rule, due)| due.and_then(due_at).map(|at| (rule, at)))
    .min_by_key(|&(_, at)| at) else {
        return TimeExitWatch::Clear;
    };
    TimeExitWatch::Due {
        rule,
        due_at,
        pnl_pct: (current / entry - 1.0) * 100.0,
    }
}

/// The first whole second of holding at which a rule with `duration_seconds` acts; `None`
/// for a duration no rule accepts.
fn holding_seconds(duration_seconds: f64) -> Option<i64> {
    (duration_seconds.is_finite() && duration_seconds > 0.0).then(|| duration_seconds.ceil() as i64)
}
