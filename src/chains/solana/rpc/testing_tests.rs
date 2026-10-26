// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the endpoint checks of setup: the node checks and the wallet reads.

use super::testing::probe_endpoint;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use tokio::net::TcpListener;

const WALLET: &str = "So11111111111111111111111111111111111111112";

/// A mainnet node at a local address that answers `blocked_method` with HTTP 403
/// "Request blocked", as a public endpoint does for the reads it does not serve.
async fn serve_node(blocked_method: Option<&'static str>) -> String {
    let app = Router::new().route(
        "/",
        post(move |Json(request): Json<serde_json::Value>| async move {
            let method = request["method"].as_str().unwrap_or_default().to_owned();
            if Some(method.as_str()) == blocked_method {
                return (StatusCode::FORBIDDEN, "Request blocked").into_response();
            }
            let result = match method.as_str() {
                "getHealth" => serde_json::json!("ok"),
                "getGenesisHash" => {
                    serde_json::json!("5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d")
                }
                "getVersion" => serde_json::json!({ "solana-core": "2.2.0" }),
                "getBalance" => serde_json::json!({ "context": { "slot": 1 }, "value": 0 }),
                "getTokenAccountsByOwner" => {
                    serde_json::json!({ "context": { "slot": 1 }, "value": [] })
                }
                _ => return StatusCode::NOT_FOUND.into_response(),
            };
            Json(serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": result })).into_response()
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{address}/")
}

async fn probe(url: &str, wallet: Option<&str>) -> super::RpcEndpointTestResult {
    let client = reqwest::Client::new();
    probe_endpoint(&client, url, "node.test", false, wallet).await
}

#[tokio::test]
async fn an_endpoint_that_answers_the_wallet_reads_passes() {
    let url = serve_node(None).await;
    let result = probe(&url, Some(WALLET)).await;
    assert!(result.success, "{:?}", result.error);
}

#[tokio::test]
async fn an_endpoint_that_blocks_a_wallet_read_is_refused_with_the_reason() {
    for blocked in ["getBalance", "getTokenAccountsByOwner"] {
        let url = serve_node(Some(blocked)).await;
        let result = probe(&url, Some(WALLET)).await;
        assert!(!result.success, "{blocked} blocked yet the endpoint passed");
        let error = result.error.unwrap_or_default();
        assert!(error.contains(blocked), "{error}");
        assert!(error.contains("403"), "{error}");
        assert!(error.contains("Request blocked"), "{error}");
        assert_eq!(result.is_mainnet, Some(true));
    }
}

#[tokio::test]
async fn without_a_wallet_only_the_node_checks_run() {
    let url = serve_node(Some("getBalance")).await;
    assert!(probe(&url, None).await.success);
}
