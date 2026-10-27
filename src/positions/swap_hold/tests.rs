// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the partial-exit hold rules: what a settled holding proves about a held sale,
//! when the hold limit releases it without proof, and that a hold leaves only by its own
//! identity.

use chrono::{Duration as ChronoDuration, TimeZone, Utc};

use super::*;

fn hold(position_id: i64, created_at: DateTime<Utc>) -> PartialExitHold {
    PartialExitHold {
        mint: "mint".to_owned(),
        position_id,
        exit_amount: RawAmount::new(250),
        requested_exit_percentage: 25.0,
        holding_before: RawAmount::new(1_000),
        take_profit_level: Some(2),
        expiry_height: Some(500),
        created_at,
    }
}

fn registered() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap()
}

fn limit() -> ChronoDuration {
    ChronoDuration::from_std(UNPROVEN_HOLD_LIMIT).unwrap()
}

#[test]
fn a_settled_holding_proves_the_outcome_only_by_exact_amounts() {
    let held = hold(1, registered());
    let now = registered() + ChronoDuration::seconds(90);
    let cases = [
        (1_000, HoldOutcome::NotSold),
        (750, HoldOutcome::Sold),
        (751, HoldOutcome::Unknown),
        (749, HoldOutcome::Unknown),
        (500, HoldOutcome::Unknown),
        (1_001, HoldOutcome::Unknown),
        (5_000, HoldOutcome::Unknown),
        (0, HoldOutcome::Unknown),
    ];
    for (after, outcome) in cases {
        assert_eq!(
            hold_outcome(&held, Some(RawAmount::new(after)), now),
            outcome,
            "after={after}"
        );
    }
}

#[test]
fn a_settled_holding_decides_even_past_the_limit() {
    let held = hold(1, registered());
    let late = registered() + limit() + ChronoDuration::minutes(30);
    assert_eq!(
        hold_outcome(&held, Some(RawAmount::new(1_000)), late),
        HoldOutcome::NotSold
    );
    assert_eq!(
        hold_outcome(&held, Some(RawAmount::new(750)), late),
        HoldOutcome::Sold
    );
}

#[test]
fn no_settled_reading_waits_until_the_limit_and_then_releases_unknown() {
    let held = hold(1, registered());
    assert_eq!(hold_outcome(&held, None, registered()), HoldOutcome::Wait);
    assert_eq!(
        hold_outcome(
            &held,
            None,
            registered() + limit() - ChronoDuration::seconds(1)
        ),
        HoldOutcome::Wait
    );
    assert_eq!(
        hold_outcome(&held, None, registered() + limit()),
        HoldOutcome::Unknown
    );
    assert_eq!(
        hold_outcome(
            &held,
            None,
            registered() + limit() + ChronoDuration::hours(5)
        ),
        HoldOutcome::Unknown
    );
}

/// A clock behind the registration time is not time spent waiting.
#[test]
fn a_clock_before_the_registration_keeps_waiting() {
    let held = hold(1, registered());
    assert_eq!(
        hold_outcome(&held, None, registered() - ChronoDuration::hours(1)),
        HoldOutcome::Wait
    );
}

#[test]
fn a_hold_is_removed_only_by_its_own_position_and_registration_time() {
    let first = hold(1, registered());
    let later_same_position = hold(1, registered() + ChronoDuration::seconds(1));
    let other_position = hold(2, registered());
    let mut holds = vec![
        first.clone(),
        later_same_position.clone(),
        other_position.clone(),
    ];

    assert_eq!(remove_hold(&mut holds, &first), Some(first.clone()));
    assert_eq!(
        holds,
        vec![later_same_position.clone(), other_position.clone()]
    );
    assert_eq!(remove_hold(&mut holds, &first), None, "a hold leaves once");

    let mut renamed = other_position.clone();
    renamed.exit_amount = RawAmount::new(1);
    assert_eq!(
        remove_hold(&mut holds, &renamed),
        Some(other_position),
        "the identity is the position and the registration time, not the amounts"
    );
    assert_eq!(holds, vec![later_same_position]);
}

#[test]
fn a_hold_round_trips_through_its_stored_form() {
    let held = hold(7, registered());
    let stored = serde_json::to_string(&vec![held.clone()]).unwrap();
    let restored: Vec<PartialExitHold> = serde_json::from_str(&stored).unwrap();
    assert_eq!(restored, vec![held]);
}
