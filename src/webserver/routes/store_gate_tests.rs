// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Every feature route group whose store is not open answers the typed `STORE_UNAVAILABLE` error (503) naming the store, before any handler reaches the store.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::database::catalog::{self, StoreSpec};
use crate::database::health::{self, StoreHealth};
use crate::database::recovery::RecoveryAction;
use crate::errors::DatabaseError;
use crate::webserver::state::AppState;

/// One route of each feature family and the store it reads.
const FAMILIES: [(&StoreSpec, &str, &str); 5] = [
    (&catalog::POSITIONS, "GET", "/api/positions"),
    (&catalog::COPY_TRADING, "GET", "/api/copy-trading/tasks"),
    (&catalog::STRATEGIES, "GET", "/api/strategies"),
    (&catalog::TRANSACTIONS, "POST", "/api/transactions/list"),
    (&catalog::WALLET, "GET", "/api/wallet/current"),
];

#[tokio::test]
async fn a_route_whose_store_is_not_open_answers_store_unavailable() {
    let too_new = DatabaseError::StoreTooNew {
        store: "test".to_owned(),
        version: 9,
        min_reader_version: 9,
        supported_version: 1,
        written_by: "99.0.0".to_owned(),
    };
    for (store, _, _) in FAMILIES {
        for path in store.paths() {
            health::publish(
                store,
                &path,
                StoreHealth::unavailable(&too_new, RecoveryAction::Isolate),
            );
        }
    }

    let router = super::create_router(Arc::new(AppState::new()));
    for (store, method, uri) in FAMILIES {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .expect("build request");
        let response = router
            .clone()
            .oneshot(request)
            .await
            .expect("router responded");
        assert_eq!(
            response.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "{method} {uri}"
        );
        let body: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 64 * 1024)
                .await
                .expect("read response body"),
        )
        .expect("decode error envelope");
        let error = &body["error"];
        assert_eq!(error["code"], "STORE_UNAVAILABLE", "{uri}");
        assert_eq!(error["text"]["id"], "errors-store-unavailable", "{uri}");
        assert_eq!(error["text"]["args"]["store"]["value"], store.id, "{uri}");
        assert!(
            error["message"]
                .as_str()
                .is_some_and(|message| message.contains(store.id)),
            "{uri}: {error}"
        );
        assert!(
            error["details"]
                .as_str()
                .is_some_and(|details| details.starts_with("store_too_new")),
            "{uri}: {error}"
        );
    }
}
