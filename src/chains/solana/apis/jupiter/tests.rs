// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the Jupiter token-list transport: a failed request names its cause and counts as a failed request.

use super::JupiterClient;

#[tokio::test]
async fn a_request_that_cannot_be_sent_names_its_cause_and_counts_as_failed() {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        listener.local_addr().expect("local address").port()
    };
    let client = JupiterClient::new(true).expect("client");

    let error = client
        .fetch_list(format!("http://127.0.0.1:{port}/recent"))
        .await
        .expect_err("nothing listens on a closed port");

    let message = error.to_string().to_lowercase();
    assert!(
        message.contains("connection refused"),
        "the transport cause is part of the error: {message}"
    );
    let stats = client.get_stats().await;
    assert_eq!(
        (
            stats.total_requests,
            stats.failed_requests,
            stats.cache_misses
        ),
        (1, 1, 0)
    );
}
