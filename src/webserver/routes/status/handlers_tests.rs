// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the loopback-only restart check.

use super::handlers::restart_safety;
use axum::body::{to_bytes, Body};
use axum::extract::{ConnectInfo, Request};
use axum::http::StatusCode;
use std::net::SocketAddr;

fn peer(addr: &str) -> ConnectInfo<SocketAddr> {
    ConnectInfo(addr.parse().expect("socket address"))
}

/// A restart check as the local service manager sends it, with `extra` headers added.
fn request_from(addr: Option<&str>, host: &str, extra: &[(&str, &str)]) -> Request {
    let mut builder = Request::builder()
        .uri("/api/health/restart-safety")
        .header("host", host);
    for (name, value) in extra {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).expect("request");
    if let Some(addr) = addr {
        request.extensions_mut().insert(peer(addr));
    }
    request
}

async fn assert_refused(request: Request, case: &str) {
    let response = restart_safety(request).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN, "{case}");
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert!(json.get("safe_to_restart").is_none(), "{case}");
}

#[tokio::test]
async fn a_remote_peer_is_refused_without_an_answer() {
    for addr in [Some("192.168.1.20:50000"), None] {
        assert_refused(
            request_from(addr, "127.0.0.1:8080", &[]),
            &format!("{addr:?}"),
        )
        .await;
    }
}

#[tokio::test]
async fn a_relayed_request_is_refused_although_its_peer_is_loopback() {
    for (name, value) in [
        ("forwarded", "for=203.0.113.9"),
        ("x-forwarded-for", "203.0.113.9"),
        ("x-real-ip", "203.0.113.9"),
    ] {
        assert_refused(
            request_from(Some("127.0.0.1:50000"), "127.0.0.1:8080", &[(name, value)]),
            name,
        )
        .await;
    }
}

#[tokio::test]
async fn a_loopback_peer_naming_another_host_is_refused() {
    for host in ["bot.example.com:443", "rebind.example:8080"] {
        assert_refused(request_from(Some("127.0.0.1:50000"), host, &[]), host).await;
    }
}

#[tokio::test]
async fn a_loopback_peer_gets_only_the_restart_answer() {
    crate::global::set_webserver_port(8080);
    let response =
        restart_safety(request_from(Some("127.0.0.1:50000"), "127.0.0.1:8080", &[])).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let object = json.as_object().expect("object");
    assert_eq!(object.len(), 1);
    assert!(object["safe_to_restart"].is_boolean());
}
