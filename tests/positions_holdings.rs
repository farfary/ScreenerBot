// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: checked token-holding bookkeeping on a position.

mod common;

use common::test_position;
use screenerbot::chains::RawAmount;
use screenerbot::positions::{Error, Position};

fn raw<T: From<u64>>(value: u64) -> T {
    T::from(value)
}

fn holding(remaining: Option<u64>, exited: u64) -> Position {
    let mut position = test_position(0.01, 1.0);
    position.token_amount = Some(raw(100));
    position.remaining_token_amount = remaining.map(raw);
    position.total_exited_amount = raw(exited);
    position
}

#[test]
fn held_amount_prefers_the_remaining_amount() {
    assert_eq!(
        holding(Some(40), 60).held_amount(),
        Some(RawAmount::from(40u64))
    );
    assert_eq!(
        holding(None, 0).held_amount(),
        Some(RawAmount::from(100u64))
    );

    let mut empty = holding(None, 0);
    empty.token_amount = None;
    assert_eq!(empty.held_amount(), None);
}

#[test]
fn an_exit_moves_tokens_from_held_to_exited() {
    let mut position = holding(Some(100), 0);
    assert_eq!(
        position.book_exit(RawAmount::from(30u64)).unwrap(),
        RawAmount::from(30u64)
    );
    assert_eq!(position.remaining_token_amount, Some(raw(70)));
    assert_eq!(position.total_exited_amount, raw(30));
}

#[test]
fn an_oversell_floors_the_remaining_amount_at_zero() {
    let mut position = holding(Some(10), 0);
    assert_eq!(
        position.book_exit(RawAmount::from(40u64)).unwrap(),
        RawAmount::from(40u64)
    );
    assert_eq!(position.remaining_token_amount, Some(raw(0)));
    assert_eq!(position.total_exited_amount, raw(40));
}

#[test]
fn an_exit_without_a_remaining_amount_only_books_the_exit() {
    let mut position = holding(None, 5);
    assert_eq!(
        position.book_exit(RawAmount::from(10u64)).unwrap(),
        RawAmount::from(15u64)
    );
    assert_eq!(position.remaining_token_amount, None);
    assert_eq!(position.total_exited_amount, raw(15));
}

#[test]
fn an_acquisition_adds_to_the_remaining_amount() {
    let mut position = holding(Some(70), 30);
    assert_eq!(
        position.book_acquisition(RawAmount::from(50u64)).unwrap(),
        RawAmount::from(120u64)
    );
    assert_eq!(position.remaining_token_amount, Some(raw(120)));
    assert_eq!(position.total_exited_amount, raw(30));

    let mut fresh = holding(None, 0);
    assert_eq!(
        fresh.book_acquisition(RawAmount::from(50u64)).unwrap(),
        RawAmount::from(50u64)
    );
    assert_eq!(fresh.remaining_token_amount, Some(raw(50)));
    assert_eq!(fresh.total_exited_amount, raw(0));
}

#[test]
fn closing_books_what_is_held_as_exited() {
    let mut position = holding(Some(70), 30);
    assert_eq!(
        position.book_remaining_as_exited().unwrap(),
        RawAmount::from(70u64)
    );
    assert_eq!(position.remaining_token_amount, Some(raw(0)));
    assert_eq!(position.total_exited_amount, raw(100));

    let mut unrecorded = holding(None, 30);
    assert_eq!(
        unrecorded.book_remaining_as_exited().unwrap(),
        RawAmount::ZERO
    );
    assert_eq!(unrecorded.remaining_token_amount, None);
    assert_eq!(unrecorded.total_exited_amount, raw(30));
}

#[test]
fn acquired_amount_is_held_plus_exited() {
    assert_eq!(holding(Some(70), 30).acquired_amount(), Some(raw(100)));
    assert_eq!(holding(None, 30).acquired_amount(), Some(raw(30)));

    let mut full = holding(Some(0), 1);
    full.remaining_token_amount = Some(RawAmount::MAX);
    assert_eq!(full.acquired_amount(), None);
}

#[test]
fn an_overflow_changes_nothing() {
    let mut position = holding(Some(1), 0);
    position.total_exited_amount = RawAmount::MAX;
    assert!(matches!(
        position.book_exit(RawAmount::from(1u64)),
        Err(Error::AmountOverflow { .. })
    ));
    assert_eq!(position.remaining_token_amount, Some(raw(1)));
    assert_eq!(position.total_exited_amount, RawAmount::MAX);

    assert!(matches!(
        position.book_remaining_as_exited(),
        Err(Error::AmountOverflow { .. })
    ));
    assert_eq!(position.remaining_token_amount, Some(raw(1)));
    assert_eq!(position.total_exited_amount, RawAmount::MAX);

    let mut full = holding(Some(0), 0);
    full.remaining_token_amount = Some(RawAmount::MAX);
    assert!(matches!(
        full.book_acquisition(RawAmount::from(1u64)),
        Err(Error::AmountOverflow { .. })
    ));
    assert_eq!(full.remaining_token_amount, Some(RawAmount::MAX));
    assert_eq!(full.total_exited_amount, raw(0));
}
