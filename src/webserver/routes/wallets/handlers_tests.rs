// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Every refused trading-wallet replacement answers the error code and message of its reason, the setup routes refuse after a full setup, and the replacement routes refuse before a full setup and without the GUI token or a session.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::Value;
use tower::ServiceExt;

use super::handlers::replacement_error;
use crate::process::restart_safety::RestartBlocker;
use crate::wallets::Error;
use crate::webserver::state::AppState;

fn code_and_id(error: &Error) -> (String, String) {
    let body = replacement_error(error).body();
    (
        body["error"]["code"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        body["error"]["text"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    )
}

#[test]
fn every_refusal_maps_to_the_code_and_message_of_its_reason() {
    let address = "NewWalletAddress11111111111111111111111111".to_owned();
    let cases = [
        (
            Error::ReplacementBlocked {
                reason: RestartBlocker::OpenPositions { count: 2 },
            },
            "OPEN_POSITIONS",
            "errors-wallets-replace-open-positions",
        ),
        (
            Error::ReplacementBlocked {
                reason: RestartBlocker::TradeInFlight,
            },
            "CONFLICT",
            "errors-wallets-replace-trade-in-flight",
        ),
        (
            Error::ReplacementBlocked {
                reason: RestartBlocker::ToolInFlight,
            },
            "CONFLICT",
            "errors-wallets-replace-tool-in-flight",
        ),
        (
            Error::ReplacementBlocked {
                reason: RestartBlocker::PositionsLoading,
            },
            "CONFLICT",
            "errors-wallets-replace-positions-loading",
        ),
        (
            Error::ReplacementBlocked {
                reason: RestartBlocker::PositionsUnreadable,
            },
            "CONFLICT",
            "errors-wallets-replace-positions-unreadable",
        ),
        (
            Error::ReplacementRestartInProgress,
            "CONFLICT",
            "errors-wallets-replace-restart-in-progress",
        ),
        (
            Error::ReplacementPending,
            "CONFLICT",
            "errors-wallets-replace-pending",
        ),
        (
            Error::ReplacementNoTradingWallet,
            "CONFLICT",
            "errors-wallets-replace-no-trading-wallet",
        ),
        (
            Error::ReplacementSameWallet {
                address: address.clone(),
            },
            "INVALID_INPUT",
            "errors-wallets-replace-same-wallet",
        ),
        (
            Error::ReplacementWalletIsWatched {
                address: address.clone(),
            },
            "INVALID_INPUT",
            "errors-wallets-replace-watched",
        ),
        (
            Error::ReplacementAddressMismatch {
                expected: address.clone(),
                actual: "Other".to_owned(),
            },
            "INVALID_INPUT",
            "errors-wallets-replace-address-mismatch",
        ),
        (
            Error::ReplacementInterrupted {
                step: "add the new key to the vault",
                copy_trading_off: true,
                detail: "disk is full".to_owned(),
            },
            "INTERNAL",
            "errors-wallets-replace-interrupted",
        ),
        (
            Error::ReplacementInterrupted {
                step: "switch Copy Trading off",
                copy_trading_off: false,
                detail: "disk is full".to_owned(),
            },
            "INTERNAL",
            "errors-wallets-replace-interrupted-copy-trading-on",
        ),
        (
            Error::ReplacementVaultChanged {
                detail: "a second trading wallet".to_owned(),
            },
            "INTERNAL",
            "errors-wallets-replace-failed",
        ),
        (
            Error::ReplacementMarker {
                operation: "write",
                message: "disk is full".to_owned(),
            },
            "INTERNAL",
            "errors-wallets-replace-failed",
        ),
    ];
    for (error, code, id) in cases {
        assert_eq!(
            code_and_id(&error),
            (code.to_owned(), id.to_owned()),
            "{error}"
        );
    }
}

#[test]
fn an_open_position_refusal_carries_the_count() {
    let body = replacement_error(&Error::ReplacementBlocked {
        reason: RestartBlocker::OpenPositions { count: 3 },
    })
    .body();
    assert_eq!(body["error"]["text"]["args"]["count"]["value"], 3);
}

/// Holds a process-wide flag at a value for one test and restores the earlier value
/// when dropped, so no later test in the same process inherits it.
struct FlagGuard {
    flag: &'static AtomicBool,
    previous: bool,
}

impl FlagGuard {
    fn set(flag: &'static AtomicBool, value: bool) -> Self {
        Self {
            flag,
            previous: flag.swap(value, Ordering::SeqCst),
        }
    }
}

impl Drop for FlagGuard {
    fn drop(&mut self) {
        self.flag.store(self.previous, Ordering::SeqCst);
    }
}

/// Send `method uri` with `headers` through `router` and return the status and error code.
async fn refusal(
    router: &Router,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, String) {
    let mut request = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let body = if method == "POST" {
        Body::from(r#"{"validation_id":"x","wallet_private_key":"x","rpc_urls":[]}"#)
    } else {
        Body::empty()
    };
    let response = router
        .clone()
        .oneshot(
            request
                .header("content-type", "application/json")
                .body(body)
                .expect("build request"),
        )
        .await
        .expect("router responded");
    let status = response.status();
    let body: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("read response body"),
    )
    .expect("decode error envelope");
    (
        status,
        body["error"]["code"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    )
}

const REPLACEMENT_ROUTES: [(&str, &str); 3] = [
    ("POST", "/api/wallets/replace"),
    ("GET", "/api/wallets/backups"),
    ("POST", "/api/wallets/replacement/dismiss"),
];

#[tokio::test]
async fn the_replacement_routes_refuse_before_a_full_setup() {
    let _initialized = FlagGuard::set(&crate::global::INITIALIZATION_COMPLETE, false);
    crate::global::set_explore_mode(true);
    let router = crate::webserver::routes::create_router(Arc::new(AppState::new()));

    for (method, uri) in REPLACEMENT_ROUTES {
        let (status, code) = refusal(&router, method, uri, &[]).await;
        assert_eq!(code, "INITIALIZATION_REQUIRED", "{method} {uri} ({status})");
    }
    crate::global::set_explore_mode(false);
}

#[tokio::test]
async fn the_replacement_routes_refuse_a_request_without_the_gui_token() {
    let _ = crate::config::utils::CONFIG
        .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
    let _initialized = FlagGuard::set(&crate::global::INITIALIZATION_COMPLETE, true);
    crate::global::set_gui_mode(true);
    crate::global::set_webserver_port(8080);
    let router = crate::webserver::server::build_app(Arc::new(AppState::new()));
    let local = [
        ("host", "127.0.0.1:8080"),
        ("origin", "http://127.0.0.1:8080"),
    ];

    for (method, uri) in REPLACEMENT_ROUTES {
        let (status, code) = refusal(&router, method, uri, &local).await;
        assert_eq!(code, "MISSING_TOKEN", "{method} {uri} ({status})");
    }
    crate::global::set_gui_mode(false);
}

#[tokio::test]
async fn the_replacement_routes_refuse_a_headless_request_without_a_session() {
    let _ = crate::config::utils::CONFIG
        .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
    crate::config::update_config_section(|cfg| cfg.webserver.auth_enabled = true, false)
        .expect("enable the dashboard password");
    let _initialized = FlagGuard::set(&crate::global::INITIALIZATION_COMPLETE, true);
    crate::global::set_gui_mode(false);
    crate::global::set_webserver_port(8080);
    let router = crate::webserver::server::build_app(Arc::new(AppState::new()));

    for (method, uri) in REPLACEMENT_ROUTES {
        let (status, code) = refusal(&router, method, uri, &[("host", "127.0.0.1:8080")]).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(code, "AUTHENTICATION_REQUIRED", "{method} {uri}");
    }
}

#[tokio::test]
async fn the_setup_routes_refuse_after_a_full_setup() {
    let _initialized = FlagGuard::set(&crate::global::INITIALIZATION_COMPLETE, true);
    let router = crate::webserver::routes::create_router(Arc::new(AppState::new()));
    for uri in [
        "/api/initialization/complete",
        "/api/initialization/explore",
    ] {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"validation_id":"x","wallet_private_key":"x","rpc_urls":[]}"#,
            ))
            .expect("build request");
        let response = router
            .clone()
            .oneshot(request)
            .await
            .expect("router responded");
        assert_eq!(response.status(), StatusCode::CONFLICT, "{uri}");
        let body: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 64 * 1024)
                .await
                .expect("read response body"),
        )
        .expect("decode error envelope");
        assert_eq!(body["error"]["code"], "CONFLICT", "{uri}");
        assert_eq!(
            body["error"]["text"]["id"], "errors-initialization-already-complete",
            "{uri}"
        );
    }
}
