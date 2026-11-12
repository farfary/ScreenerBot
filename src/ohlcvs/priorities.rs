// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV fetch priorities — scoring and scheduling for token candle updates, and the ranking
//! of the monitor's token slots.

use crate::ohlcvs::types::{Priority, TokenOhlcvConfig};
use chrono::{DateTime, Utc};
use std::collections::{HashSet, VecDeque};
use std::time::Duration;

pub struct PriorityManager;

impl PriorityManager {
    /// Calculate priority score for a token
    /// Higher score = higher priority
    pub fn calculate_priority_score(
        is_open_position: bool,
        recent_views: u32,
        recent_trades: u32,
        hours_since_activity: f64,
    ) -> u32 {
        let mut score = 0u32;

        // Open positions get maximum priority
        if is_open_position {
            score += 100;
        }

        // Recent views contribute to priority
        score += recent_views * 10;

        // Recent trades contribute more
        score += recent_trades * 50;

        // Decay based on inactivity
        let decay_factor = 1.0 / (1.0 + hours_since_activity / 24.0);
        score = ((score as f64) * decay_factor) as u32;

        score
    }

    /// Determine priority level from score
    pub fn priority_from_score(score: u32) -> Priority {
        match score {
            100.. => Priority::Critical,
            50..=99 => Priority::High,
            10..=49 => Priority::Medium,
            _ => Priority::Low,
        }
    }

    /// Calculate adjusted fetch interval based on activity
    pub fn calculate_fetch_interval(config: &TokenOhlcvConfig) -> Duration {
        config.calculate_adjusted_interval()
    }

    /// Should throttle based on empty fetches
    pub fn should_throttle(config: &TokenOhlcvConfig) -> bool {
        // Throttle if we've had 5+ consecutive empty fetches
        config.consecutive_empty_fetches >= 5
    }

    /// Calculate throttle multiplier
    pub fn throttle_multiplier(consecutive_empty_fetches: u32) -> f64 {
        // Exponential backoff: 1.0, 1.5, 2.0, 2.5, 3.0 (capped at 3x)
        (1.0 + (consecutive_empty_fetches as f64) * 0.5).min(3.0)
    }

    /// Get recommended action for a token
    pub fn get_recommended_action(config: &TokenOhlcvConfig) -> RecommendedAction {
        let hours_inactive = (Utc::now() - config.last_activity).num_hours().max(0) as f64;

        // Critical priority - always fetch
        if config.priority == Priority::Critical {
            return RecommendedAction::FetchNow;
        }

        // Too many empty fetches - pause
        if config.consecutive_empty_fetches >= 10 {
            return RecommendedAction::Pause;
        }

        // Recently active - fetch normally
        if hours_inactive < 1.0 {
            return RecommendedAction::FetchNow;
        }

        // Moderately inactive - throttle
        if hours_inactive < 24.0 {
            return RecommendedAction::Throttle(Duration::from_secs(
                ((config.fetch_frequency.as_secs() as f64) * 2.0) as u64,
            ));
        }

        // Very inactive - pause
        if hours_inactive > 168.0 {
            // 1 week
            return RecommendedAction::Pause;
        }

        // Default - throttle moderately
        RecommendedAction::Throttle(Duration::from_secs(
            ((config.fetch_frequency.as_secs() as f64) * 1.5) as u64,
        ))
    }

    /// Update priority based on new activity
    pub fn update_priority_on_activity(
        current_priority: Priority,
        activity_type: ActivityType,
    ) -> Priority {
        match activity_type {
            ActivityType::PositionOpened => Priority::Critical,
            ActivityType::PositionClosed => {
                // Downgrade from critical but keep high
                if current_priority == Priority::Critical {
                    Priority::High
                } else {
                    current_priority
                }
            }
            ActivityType::TokenViewed => {
                // Upgrade if low, maintain if higher
                match current_priority {
                    Priority::Low => Priority::Medium,
                    _ => current_priority,
                }
            }
            ActivityType::ChartViewed => {
                // Significant interest, upgrade
                match current_priority {
                    Priority::Low => Priority::Medium,
                    Priority::Medium => Priority::High,
                    _ => current_priority,
                }
            }
            ActivityType::DataRequested => Priority::High, // Immediate data need
        }
    }

    /// Calculate optimal batch size for fetching
    pub fn calculate_batch_size(priority: Priority) -> usize {
        match priority {
            Priority::Critical => 1000, // Max candles
            Priority::High => 500,
            Priority::Medium => 200,
            Priority::Low => 100,
        }
    }

    /// Get fetch timeout based on priority
    pub fn get_fetch_timeout(priority: Priority) -> Duration {
        match priority {
            Priority::Critical => Duration::from_secs(60),
            Priority::High => Duration::from_secs(45),
            Priority::Medium => Duration::from_secs(30),
            Priority::Low => Duration::from_secs(15),
        }
    }

    /// Should retry on failure
    pub fn should_retry(priority: Priority, attempt: u32) -> bool {
        let max_retries = match priority {
            Priority::Critical => 5,
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
        };

        attempt < max_retries
    }

    /// Calculate retry delay with exponential backoff
    pub fn retry_delay(attempt: u32) -> Duration {
        let base_delay = Duration::from_secs(2);
        let multiplier = (2u32).pow(attempt.min(5)); // Cap at 2^5 = 32
        base_delay * multiplier
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityType {
    PositionOpened,
    PositionClosed,
    TokenViewed,
    ChartViewed,
    DataRequested,
}

impl ActivityType {
    /// Whether this activity needs the token's data now. The monitor fetches such a token at
    /// once, and only such an activity clears its empty-fetch backoff, because that fetch
    /// re-tests the token. Every other activity (a view, a chart poll, a closed position)
    /// refreshes the activity time and priority and leaves the backoff to the fetches.
    pub fn fetches_now(self) -> bool {
        matches!(self, Self::PositionOpened | Self::DataRequested)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecommendedAction {
    FetchNow,
    Throttle(Duration),
    Pause,
}

/// A monitored token as the slot planner sees it.
#[derive(Debug, Clone)]
pub(super) struct SlotHolder {
    pub mint: String,
    pub priority: Priority,
    pub last_activity: DateTime<Utc>,
    pub consecutive_empty_fetches: u32,
}

/// What one sync of the monitor's token slots is planned from.
pub(super) struct SlotDemand<'a> {
    /// Mints the pool service prices, in its order: the only tokens that can be monitored.
    pub available: &'a [String],
    pub monitored: &'a [SlotHolder],
    /// Mints the live and paper books hold. They always get a slot, cap or not.
    pub held: &'a HashSet<String>,
    /// Mints that currently pass filtering: the candidates of the entry strategies.
    pub passed: &'a HashSet<String>,
    /// `ohlcv.max_monitored_tokens`; `usize::MAX` when it is 0 (unlimited).
    pub max_tokens: usize,
}

/// The changes one sync makes to the monitored tokens.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct SlotPlan {
    /// Mints to start monitoring, held first, then passed, then the rest.
    pub add: Vec<String>,
    /// Monitored mints to stop monitoring: over the cap, or releasing a slot to a passed token.
    pub remove: Vec<String>,
    /// Priced tokens that pass filtering and still have no slot after the plan.
    pub passed_unmonitored: usize,
}

/// Plan the monitored tokens under `max_tokens`. Held mints come first, then the tokens that
/// pass filtering, then the rest. A token over the cap leaves in the order of
/// [`drops_before`]; a passed token that finds no free slot takes the slot of a monitored token
/// that only holds a low-priority place (`is_releasable`), so the entry strategies' candidates
/// are not starved of candles by tokens nothing is waiting on.
pub(super) fn plan_monitor_slots(demand: &SlotDemand<'_>) -> SlotPlan {
    let monitored_mints: HashSet<&str> = demand
        .monitored
        .iter()
        .map(|holder| holder.mint.as_str())
        .collect();
    let (mut held_new, mut passed_new, mut rest_new) = (Vec::new(), Vec::new(), Vec::new());
    for mint in demand.available {
        if monitored_mints.contains(mint.as_str()) {
            continue;
        }
        if demand.held.contains(mint) {
            held_new.push(mint);
        } else if demand.passed.contains(mint) {
            passed_new.push(mint);
        } else {
            rest_new.push(mint);
        }
    }

    // Holders that may leave, the first to leave first. Held mints never leave.
    let mut leavers: Vec<&SlotHolder> = demand
        .monitored
        .iter()
        .filter(|holder| !demand.held.contains(&holder.mint))
        .collect();
    leavers.sort_by(|a, b| drops_before(a, b, demand.passed));
    let mut leavers: VecDeque<&SlotHolder> = leavers.into();

    let mut plan = SlotPlan::default();
    let mut len = demand.monitored.len() + held_new.len();
    while len > demand.max_tokens {
        let Some(leaver) = leavers.pop_front() else {
            break;
        };
        plan.remove.push(leaver.mint.clone());
        len -= 1;
    }

    plan.add.extend(held_new.into_iter().cloned());
    let mut free = demand.max_tokens.saturating_sub(len);
    for mint in passed_new {
        if free > 0 {
            free -= 1;
            plan.add.push(mint.clone());
        } else if let Some(at) = leavers
            .iter()
            .position(|holder| is_releasable(holder, demand.held, demand.passed))
        {
            let released = leavers.remove(at).expect("position is within the queue");
            plan.remove.push(released.mint.clone());
            plan.add.push(mint.clone());
        } else {
            plan.passed_unmonitored += 1;
        }
    }
    for mint in rest_new {
        if free == 0 {
            break;
        }
        free -= 1;
        plan.add.push(mint.clone());
    }
    plan
}

/// Order in which monitored tokens leave when the cap is exceeded: lowest priority first, a token
/// nothing waits on before a passed one, then the longest inactive, then the most empty fetches.
fn drops_before(a: &SlotHolder, b: &SlotHolder, passed: &HashSet<String>) -> std::cmp::Ordering {
    a.priority
        .cmp(&b.priority)
        .then_with(|| passed.contains(&a.mint).cmp(&passed.contains(&b.mint)))
        .then_with(|| a.last_activity.cmp(&b.last_activity))
        .then_with(|| {
            b.consecutive_empty_fetches
                .cmp(&a.consecutive_empty_fetches)
        })
}

/// Whether a monitored token gives up its slot for a waiting passed token: low priority, not
/// held and not passed itself. A medium or higher priority token is one somebody viewed, requested
/// or traded.
fn is_releasable(holder: &SlotHolder, held: &HashSet<String>, passed: &HashSet<String>) -> bool {
    holder.priority == Priority::Low
        && !held.contains(&holder.mint)
        && !passed.contains(&holder.mint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    /// A fixed instant, so two holders idle for the same hours tie on activity.
    fn at_rest() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("valid instant")
    }

    fn holder(mint: &str, priority: Priority, idle_hours: i64, empty_fetches: u32) -> SlotHolder {
        SlotHolder {
            mint: mint.to_string(),
            priority,
            last_activity: at_rest() - ChronoDuration::hours(idle_hours),
            consecutive_empty_fetches: empty_fetches,
        }
    }

    fn mints(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn plan(
        available: &[&str],
        monitored: &[SlotHolder],
        held: &[&str],
        passed: &[&str],
        max_tokens: usize,
    ) -> SlotPlan {
        plan_monitor_slots(&SlotDemand {
            available: &mints(available),
            monitored,
            held: &set(held),
            passed: &set(passed),
            max_tokens,
        })
    }

    #[test]
    fn free_slots_go_to_held_then_passed_then_the_rest() {
        let result = plan(
            &["rest", "passed", "held", "passed2", "rest2"],
            &[],
            &["held"],
            &["passed", "passed2"],
            3,
        );
        assert_eq!(result.add, mints(&["held", "passed", "passed2"]));
        assert!(result.remove.is_empty());
        assert_eq!(result.passed_unmonitored, 0);
    }

    #[test]
    fn a_held_mint_gets_a_slot_beyond_the_cap() {
        let monitored = [
            holder("a", Priority::Low, 1, 0),
            holder("b", Priority::Low, 2, 0),
        ];
        let result = plan(&["a", "b", "held"], &monitored, &["held"], &[], 2);
        assert_eq!(result.add, mints(&["held"]));
        // The cap is exceeded by the held mint, so the longest idle token leaves.
        assert_eq!(result.remove, mints(&["b"]));
    }

    #[test]
    fn a_waiting_passed_token_takes_the_slot_of_a_plain_low_token() {
        let monitored = [
            holder("plain-recent", Priority::Low, 1, 0),
            holder("plain-idle", Priority::Low, 9, 0),
            holder("passed-low", Priority::Low, 20, 0),
        ];
        let result = plan(
            &["plain-recent", "plain-idle", "passed-low", "waiting"],
            &monitored,
            &[],
            &["passed-low", "waiting"],
            3,
        );
        assert_eq!(result.remove, mints(&["plain-idle"]));
        assert_eq!(result.add, mints(&["waiting"]));
        assert_eq!(result.passed_unmonitored, 0);
    }

    #[test]
    fn only_a_plain_low_token_is_released_for_a_passed_one() {
        let monitored = [
            holder("held", Priority::Low, 50, 0),
            holder("viewed", Priority::Medium, 50, 0),
            holder("requested", Priority::High, 50, 0),
            holder("traded", Priority::Critical, 50, 0),
            holder("passed-low", Priority::Low, 50, 0),
        ];
        let result = plan(
            &[
                "held",
                "viewed",
                "requested",
                "traded",
                "passed-low",
                "waiting",
            ],
            &monitored,
            &["held"],
            &["passed-low", "waiting"],
            5,
        );
        assert!(result.remove.is_empty());
        assert!(result.add.is_empty());
        assert_eq!(result.passed_unmonitored, 1);
    }

    #[test]
    fn each_waiting_passed_token_releases_one_slot() {
        let monitored = [
            holder("plain-a", Priority::Low, 3, 0),
            holder("plain-b", Priority::Low, 2, 0),
            holder("plain-c", Priority::Low, 1, 0),
        ];
        let result = plan(
            &[
                "plain-a", "plain-b", "plain-c", "wait-1", "wait-2", "wait-3", "wait-4",
            ],
            &monitored,
            &[],
            &["wait-1", "wait-2", "wait-3", "wait-4"],
            3,
        );
        assert_eq!(result.remove, mints(&["plain-a", "plain-b", "plain-c"]));
        assert_eq!(result.add, mints(&["wait-1", "wait-2", "wait-3"]));
        assert_eq!(result.passed_unmonitored, 1);
    }

    #[test]
    fn the_cap_trims_plain_tokens_before_passed_ones_within_a_priority() {
        let monitored = [
            holder("passed-idle", Priority::Low, 30, 0),
            holder("plain-recent", Priority::Low, 1, 0),
            holder("medium", Priority::Medium, 40, 0),
            holder("plain-idle", Priority::Low, 20, 0),
        ];
        let result = plan(
            &["passed-idle", "plain-recent", "medium", "plain-idle"],
            &monitored,
            &[],
            &["passed-idle"],
            2,
        );
        // Lowest priority first, plain before passed, then the longest idle.
        assert_eq!(result.remove, mints(&["plain-idle", "plain-recent"]));
        assert!(result.add.is_empty());
    }

    #[test]
    fn a_token_with_more_empty_fetches_leaves_first_among_equals() {
        let monitored = [
            holder("quiet", Priority::Low, 5, 0),
            holder("empty", Priority::Low, 5, 7),
        ];
        let result = plan(&["quiet", "empty"], &monitored, &[], &[], 1);
        assert_eq!(result.remove, mints(&["empty"]));
    }

    #[test]
    fn rest_tokens_fill_only_what_passed_ones_leave_free() {
        let monitored = [holder("plain", Priority::Low, 1, 0)];
        let result = plan(
            &["plain", "other", "waiting"],
            &monitored,
            &[],
            &["waiting"],
            2,
        );
        assert_eq!(result.add, mints(&["waiting"]));
        assert!(result.remove.is_empty());
    }

    #[test]
    fn a_cap_of_zero_means_unlimited() {
        let result = plan(&["a", "b", "c"], &[], &[], &["b"], usize::MAX);
        assert_eq!(result.add, mints(&["b", "a", "c"]));
        assert!(result.remove.is_empty());
        assert_eq!(result.passed_unmonitored, 0);
    }

    #[test]
    fn a_synced_set_plans_nothing() {
        let monitored = [
            holder("a", Priority::Low, 1, 0),
            holder("b", Priority::Low, 1, 0),
        ];
        let result = plan(&["a", "b"], &monitored, &[], &["a"], 2);
        assert_eq!(result, SlotPlan::default());
    }
}
