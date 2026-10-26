// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the loopback-only restart check.

use super::handlers::{is_loopback_peer, restart_safety};
use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use std::net::SocketAddr;

fn peer(addr: &str) -> ConnectInfo<SocketAddr> {
    ConnectInfo(addr.parse().expect("socket address"))
}

#[test]
fn only_a_loopback_peer_is_local() {
    for addr in ["127.0.0.1:50000", "[::1]:50000", "[::ffff:127.0.0.1]:50000"] {
        assert!(is_loopback_peer(Some(&peer(addr))), "{addr}");
    }
    for addr in [
        "192.168.1.20:50000",
        "10.0.0.4:50000",
        "[2001:db8::1]:50000",
    ] {
        assert!(!is_loopback_peer(Some(&peer(addr))), "{addr}");
    }
    assert!(!is_loopback_peer(None));
}

fn request_from(addr: Option<&str>) -> Request {
    let mut request = Request::builder()
        .uri("/api/health/restart-safety")
        .body(Body::empty())
        .expect("request");
    if let Some(addr) = addr {
        request.extensions_mut().insert(peer(addr));
    }
    request
}

#[tokio::test]
async fn a_remote_peer_is_refused_without_an_answer() {
    for addr in [Some("192.168.1.20:50000"), None] {
        let response = restart_safety(request_from(addr)).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{addr:?}");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert!(json.get("safe_to_restart").is_none(), "{addr:?}");
    }
}

#[tokio::test]
async fn a_loopback_peer_gets_only_the_restart_answer() {
    let response = restart_safety(request_from(Some("127.0.0.1:50000"))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let object = json.as_object().expect("object");
    assert_eq!(object.len(), 1);
    assert!(object["safe_to_restart"].is_boolean());
}
