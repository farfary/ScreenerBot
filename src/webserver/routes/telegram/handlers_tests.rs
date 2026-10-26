// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for what a saved Telegram settings change asks of the running service, and for
//! the bot token check that runs before a new token is stored.

use super::handlers::{
    check_new_bot_token, requested_new_token, required_service_action, RequiredServiceAction,
    TelegramSettingsChange,
};
use crate::i18n::ids;
use crate::telegram::Error as TelegramError;

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

#[test]
fn only_a_new_unmasked_token_is_checked_and_stored() {
    assert_eq!(requested_new_token(Some("123:abc"), ""), Some("123:abc"));
    assert_eq!(
        requested_new_token(Some("123:abc"), "123:old"),
        Some("123:abc")
    );
    assert_eq!(requested_new_token(Some("123:abc"), "123:abc"), None);
    assert_eq!(requested_new_token(Some("123:ab..."), "123:abc"), None);
    assert_eq!(requested_new_token(Some(""), "123:abc"), None);
    assert_eq!(requested_new_token(None, "123:abc"), None);
}

fn error_code_and_id(error: crate::webserver::api_error::ApiError) -> (String, String) {
    let body = error.body();
    (
        body["error"]["code"].as_str().expect("code").to_string(),
        body["error"]["text"]["id"]
            .as_str()
            .expect("id")
            .to_string(),
    )
}

#[tokio::test]
async fn an_accepted_token_passes_the_check() {
    let checked = check_new_bot_token("123:abc", |token| async move {
        assert_eq!(token, "123:abc");
        Ok(())
    })
    .await;
    assert!(checked.is_ok());
}

#[tokio::test]
async fn a_refused_token_is_invalid_input() {
    let error = check_new_bot_token("123:abc", |_| async {
        Err(TelegramError::InvalidBotToken {
            detail: "Unauthorized".to_string(),
        })
    })
    .await
    .expect_err("a refused token is an error");
    assert_eq!(
        error_code_and_id(error),
        (
            "INVALID_INPUT".to_string(),
            ids::ERRORS_TELEGRAM_BOT_TOKEN_INVALID.to_string()
        )
    );
}

#[tokio::test]
async fn an_unreachable_check_is_an_upstream_error() {
    let error = check_new_bot_token("123:abc", |_| async {
        Err(TelegramError::TokenCheckUnavailable {
            detail: "connection refused".to_string(),
        })
    })
    .await
    .expect_err("an unreachable check is an error");
    assert_eq!(
        error_code_and_id(error),
        (
            "UPSTREAM_ERROR".to_string(),
            ids::ERRORS_TELEGRAM_BOT_TOKEN_CHECK_FAILED.to_string()
        )
    );
}
