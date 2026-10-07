// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! ApiError unit tests - error-code ordering, serde and HTTP status mapping, and the error envelope shape.

use super::*;
use crate::i18n::{format_en, ids};
use fluent_bundle::FluentArgs;

/// Position of each variant; the exhaustive match makes a new variant a
/// compile error until it is listed here and in `ALL`.
fn ordinal(code: ApiErrorCode) -> usize {
    match code {
        ApiErrorCode::InvalidInput => 0,
        ApiErrorCode::InvalidPassword => 1,
        ApiErrorCode::InvalidTotp => 2,
        ApiErrorCode::AuthDisabled => 3,
        ApiErrorCode::PasswordNotSet => 4,
        ApiErrorCode::CurrentPasswordRequired => 5,
        ApiErrorCode::Unauthorized => 6,
        ApiErrorCode::AuthenticationRequired => 7,
        ApiErrorCode::Forbidden => 8,
        ApiErrorCode::InvalidLocalRequest => 9,
        ApiErrorCode::InvalidToken => 10,
        ApiErrorCode::MissingToken => 11,
        ApiErrorCode::NotFound => 12,
        ApiErrorCode::Conflict => 13,
        ApiErrorCode::StaleRequest => 14,
        ApiErrorCode::NoUpdateAvailable => 15,
        ApiErrorCode::SetupValidationRequired => 16,
        ApiErrorCode::TelegramDisabled => 17,
        ApiErrorCode::NotConfigured => 18,
        ApiErrorCode::ConfigError => 19,
        ApiErrorCode::DatabaseError => 20,
        ApiErrorCode::Internal => 21,
        ApiErrorCode::BrowserOpenFailed => 22,
        ApiErrorCode::ServiceUnavailable => 23,
        ApiErrorCode::InitializationRequired => 24,
        ApiErrorCode::UpstreamError => 25,
        ApiErrorCode::IntegrityFailed => 26,
        ApiErrorCode::NotImplemented => 27,
        ApiErrorCode::AgentControlDisabled => 28,
        ApiErrorCode::LiveConfirmationRequired => 29,
        ApiErrorCode::InvalidTask => 30,
        ApiErrorCode::TaskLimit => 31,
        ApiErrorCode::WatchRejected => 32,
        ApiErrorCode::LiveUnavailable => 33,
        ApiErrorCode::TaskLive => 34,
        ApiErrorCode::OpenPositions => 35,
        ApiErrorCode::CopyError => 36,
        ApiErrorCode::RateLimited => 37,
        ApiErrorCode::UpstreamTimeout => 38,
        ApiErrorCode::PayloadTooLarge => 39,
        ApiErrorCode::RouteNotOffered => 40,
    }
}

const ALL: [ApiErrorCode; 41] = [
    ApiErrorCode::InvalidInput,
    ApiErrorCode::InvalidPassword,
    ApiErrorCode::InvalidTotp,
    ApiErrorCode::AuthDisabled,
    ApiErrorCode::PasswordNotSet,
    ApiErrorCode::CurrentPasswordRequired,
    ApiErrorCode::Unauthorized,
    ApiErrorCode::AuthenticationRequired,
    ApiErrorCode::Forbidden,
    ApiErrorCode::InvalidLocalRequest,
    ApiErrorCode::InvalidToken,
    ApiErrorCode::MissingToken,
    ApiErrorCode::NotFound,
    ApiErrorCode::Conflict,
    ApiErrorCode::StaleRequest,
    ApiErrorCode::NoUpdateAvailable,
    ApiErrorCode::SetupValidationRequired,
    ApiErrorCode::TelegramDisabled,
    ApiErrorCode::NotConfigured,
    ApiErrorCode::ConfigError,
    ApiErrorCode::DatabaseError,
    ApiErrorCode::Internal,
    ApiErrorCode::BrowserOpenFailed,
    ApiErrorCode::ServiceUnavailable,
    ApiErrorCode::InitializationRequired,
    ApiErrorCode::UpstreamError,
    ApiErrorCode::IntegrityFailed,
    ApiErrorCode::NotImplemented,
    ApiErrorCode::AgentControlDisabled,
    ApiErrorCode::LiveConfirmationRequired,
    ApiErrorCode::InvalidTask,
    ApiErrorCode::TaskLimit,
    ApiErrorCode::WatchRejected,
    ApiErrorCode::LiveUnavailable,
    ApiErrorCode::TaskLive,
    ApiErrorCode::OpenPositions,
    ApiErrorCode::CopyError,
    ApiErrorCode::RateLimited,
    ApiErrorCode::UpstreamTimeout,
    ApiErrorCode::PayloadTooLarge,
    ApiErrorCode::RouteNotOffered,
];

#[test]
fn every_code_is_listed_once_in_order() {
    for (index, code) in ALL.iter().enumerate() {
        assert_eq!(ordinal(*code), index, "{code:?}");
    }
}

#[test]
fn as_str_matches_serde_and_status_is_an_error_status() {
    for code in ALL {
        assert_eq!(
            serde_json::to_value(code).unwrap(),
            Value::String(code.as_str().to_owned()),
            "{code:?}"
        );
        let status = code.status();
        assert!(
            status.is_client_error() || status.is_server_error(),
            "{code:?}"
        );
    }
}

#[test]
fn envelope_has_the_documented_shape() {
    let error = ApiError::new(
        ApiErrorCode::NotFound,
        ids::ERRORS_STRATEGIES_ALREADY_EXISTS,
    )
    .text_arg("id", "alpha")
    .details("raw cause");
    let body = error.body();
    let object = body["error"].as_object().expect("error object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["code", "details", "message", "text", "timestamp"]);
    assert_eq!(object["code"], "NOT_FOUND");
    assert_eq!(object["details"], "raw cause");

    let mut args = FluentArgs::new();
    args.set("id", "alpha");
    let expected: String = format_en("errors-strategies-already-exists", Some(&args))
        .chars()
        .filter(|c| !matches!(c, '\u{2068}' | '\u{2069}'))
        .collect();
    assert_eq!(expected, "Strategy with ID 'alpha' already exists");
    assert_eq!(object["message"], expected.as_str());

    let text: UiText = serde_json::from_value(object["text"].clone()).unwrap();
    assert_eq!(text.id, "errors-strategies-already-exists");
    assert_eq!(text, error.text);
}

#[test]
fn details_are_null_when_absent() {
    let body = ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_CONFIG_SAVE_FAILED).body();
    assert!(body["error"]["details"].is_null());
    assert!(body["error"].as_object().unwrap().contains_key("details"));
}

#[tokio::test]
async fn response_uses_the_status_of_the_code() {
    let response =
        ApiError::new(ApiErrorCode::Conflict, ids::ERRORS_CONFIG_SAVE_FAILED).into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn for_status_preserves_the_status() {
    for status in [400u16, 401, 403, 404, 409, 422, 429, 502, 503, 504] {
        assert_eq!(
            ApiErrorCode::for_status(status).status().as_u16(),
            status,
            "{status}"
        );
    }
    assert_eq!(ApiErrorCode::for_status(500), ApiErrorCode::Internal);
    assert_eq!(ApiErrorCode::for_status(418), ApiErrorCode::Internal);
}
