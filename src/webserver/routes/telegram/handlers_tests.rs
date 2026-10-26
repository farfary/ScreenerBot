// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for what a saved Telegram settings change asks of the running service.

use super::handlers::{required_service_action, RequiredServiceAction, TelegramSettingsChange};

fn change(enabled: bool, has_token: bool, connection_changed: bool) -> TelegramSettingsChange {
    TelegramSettingsChange {
        enabled,
        has_token,
        connection_changed,
    }
}

#[test]
fn a_stopped_service_starts_once_telegram_is_on_with_a_token() {
    assert_eq!(
        required_service_action(change(true, true, false), false),
        RequiredServiceAction::Start
    );
    assert_eq!(
        required_service_action(change(true, true, true), false),
        RequiredServiceAction::Start
    );
}

#[test]
fn a_stopped_service_stays_off_without_a_token_or_while_telegram_is_off() {
    assert_eq!(
        required_service_action(change(true, false, true), false),
        RequiredServiceAction::None
    );
    assert_eq!(
        required_service_action(change(false, true, true), false),
        RequiredServiceAction::None
    );
}

#[test]
fn turning_telegram_off_stops_a_running_service_at_once() {
    for (has_token, connection_changed) in [(true, false), (true, true), (false, true)] {
        assert_eq!(
            required_service_action(change(false, has_token, connection_changed), true),
            RequiredServiceAction::Stop
        );
    }
}

#[test]
fn new_credentials_restart_a_running_service() {
    assert_eq!(
        required_service_action(change(true, true, true), true),
        RequiredServiceAction::Restart
    );
}

#[test]
fn a_running_service_with_the_same_credentials_is_left_alone() {
    assert_eq!(
        required_service_action(change(true, true, false), true),
        RequiredServiceAction::None
    );
}
