// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Every refused trading-wallet replacement answers the error code and message of its reason, and the setup routes refuse after a full setup.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use super::handlers::replacement_error;
use crate::version::RestartBlocker;
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
                detail: "disk is full".to_owned(),
            },
            "INTERNAL",
            "errors-wallets-replace-interrupted",
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

#[tokio::test]
async fn the_setup_routes_refuse_after_a_full_setup() {
    crate::global::INITIALIZATION_COMPLETE.store(true, Ordering::SeqCst);
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
