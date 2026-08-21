//! Pure: WebSocket URL derivation for the shared subscription transport.
//!
//! The transport must never invent a URL. It derives one from the configured RPC
//! endpoints, so an https endpoint has to become wss, an http one ws, and an
//! endpoint that is already a WebSocket URL has to pass through untouched.

use screenerbot::chains::solana::rpc::{get_websocket_url_from_http, websocket_url_for_attempt};

#[test]
fn https_becomes_wss() {
    let ws = get_websocket_url_from_http("https://api.mainnet-beta.solana.com")
        .expect("https url should convert");
    assert_eq!(ws, "wss://api.mainnet-beta.solana.com");
}

#[test]
fn http_becomes_ws() {
    let ws = get_websocket_url_from_http("http://localhost:8899").expect("http url should convert");
    assert_eq!(ws, "ws://localhost:8899");
}

#[test]
fn wss_passes_through_unchanged() {
    let ws = get_websocket_url_from_http("wss://api.mainnet-beta.solana.com")
        .expect("wss url should pass through");
    assert_eq!(ws, "wss://api.mainnet-beta.solana.com");
}

#[test]
fn ws_passes_through_unchanged() {
    let ws =
        get_websocket_url_from_http("ws://localhost:8899").expect("ws url should pass through");
    assert_eq!(ws, "ws://localhost:8899");
}

#[test]
fn query_string_is_preserved() {
    let ws = get_websocket_url_from_http("https://mainnet.helius-rpc.com/?api-key=abc")
        .expect("url with query string should convert");
    assert_eq!(ws, "wss://mainnet.helius-rpc.com/?api-key=abc");
}

#[test]
fn garbage_input_is_an_error() {
    assert!(get_websocket_url_from_http("not a url").is_err());
}

#[test]
fn reconnect_attempts_rotate_across_providers() {
    let urls = vec![
        "wss://one.example".to_owned(),
        "wss://two.example".to_owned(),
    ];
    assert_eq!(
        websocket_url_for_attempt(&urls, 0),
        Some("wss://one.example")
    );
    assert_eq!(
        websocket_url_for_attempt(&urls, 1),
        Some("wss://two.example")
    );
    assert_eq!(
        websocket_url_for_attempt(&urls, 2),
        Some("wss://one.example")
    );
    assert_eq!(websocket_url_for_attempt(&[], 0), None);
}
