//! Centralized network/proxy helpers.
//!
//! Detects an available proxy once (cached) and exposes it for HTTP (reqwest),
//! Solana RPC, and WebSocket clients so the whole app works behind a system
//! proxy (e.g. v2rayN on macOS) even when the proxy is configured only in the
//! OS network settings and not exported as environment variables.
//!
//! Detection order:
//! 1. Config `[network] proxy` setting (highest priority — works for GUI-launched
//!    apps that don't inherit shell env vars).
//! 2. Environment variables: `ALL_PROXY`/`all_proxy`, `HTTPS_PROXY`/`https_proxy`,
//!    `HTTP_PROXY`/`http_proxy` (reqwest reads these itself, but we resolve them
//!    explicitly so the WebSocket path and RPC path share the same value).
//! 3. macOS system proxy via `scutil --proxy` (HTTP/HTTPS first, then SOCKS).
//!
//! GUI apps launched from Finder/Electron do not inherit shell env vars, so the
//! macOS system-proxy probe is the path that actually fires in practice.

use crate::logger::{self, LogTag};
use std::sync::LazyLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    client_async_tls_with_config, connect_async, tungstenite::protocol::WebSocketConfig,
    MaybeTlsStream, WebSocketStream,
};

/// Cached detected proxy URL (e.g. "http://127.0.0.1:1081" or "socks5://127.0.0.1:1080").
/// `None` means no proxy was detected (direct connections).
static PROXY_URL: LazyLock<Option<String>> = LazyLock::new(detect_proxy);

/// Return the detected proxy URL, if any (cached for the process lifetime).
pub fn proxy_url() -> Option<&'static str> {
    PROXY_URL.as_deref()
}

/// Apply the detected proxy (if any) to a reqwest client builder.
pub fn apply_proxy(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    match proxy_url() {
        Some(url) => match reqwest::Proxy::all(url) {
            Ok(proxy) => builder.proxy(proxy),
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!("Invalid detected proxy '{url}': {e} — using direct connection"),
                );
                builder
            }
        },
        None => builder,
    }
}

/// Default ceiling on a whole HTTP request (connect + send + read the body).
///
/// reqwest has NO request timeout by default, so a socket that opens and then
/// goes silent — a stalled proxy, a captive portal, a provider that accepts the
/// connection and never answers — parks the calling task FOREVER. That is not a
/// theoretical risk: a Jupiter `/swap/v1/swap` build request hung this way in the
/// middle of a manual sell, so the swap never returned, the position stayed in
/// "Selling" with its slot permit held, and the exit action never resolved.
///
/// Generous enough that a slow-but-live endpoint still succeeds, finite so that
/// nothing in the app can wait forever. Any caller needing longer (the updater's
/// binary download) overrides it on its own builder, and any caller needing
/// shorter (swap execution) sets a per-request `.timeout()`, which reqwest
/// applies in preference to this one.
const DEFAULT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

/// Default ceiling on establishing the TCP+TLS connection alone.
const DEFAULT_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// A reqwest client builder pre-configured with the detected proxy and finite
/// request/connect timeouts (see `DEFAULT_REQUEST_TIMEOUT`).
pub fn client_builder() -> reqwest::ClientBuilder {
    apply_proxy(
        reqwest::Client::builder()
            .timeout(DEFAULT_REQUEST_TIMEOUT)
            .connect_timeout(DEFAULT_CONNECT_TIMEOUT),
    )
}

/// A default reqwest client with the detected proxy applied. Drop-in replacement
/// for `reqwest::Client::new()`; falls back to a direct client if the build fails.
pub fn client() -> reqwest::Client {
    client_builder()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Log the detected proxy once at startup (call from boot).
pub fn log_detected_proxy() {
    match proxy_url() {
        Some(url) => logger::info(LogTag::System, &format!("Network proxy detected: {url}")),
        None => logger::info(
            LogTag::System,
            "No network proxy detected (direct connections)",
        ),
    }
}

/// Connect a WebSocket, tunnelling through the detected HTTP proxy when one is
/// set (via HTTP CONNECT). Falls back to a direct `connect_async` when there is
/// no proxy or the proxy is not an HTTP proxy (e.g. SOCKS, unsupported here).
/// Returns the same stream type as `connect_async` so callers are unaffected.
pub async fn connect_ws(
    ws_url: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    if let Some(proxy) = proxy_url() {
        if let Some((proxy_host, proxy_port)) = parse_http_proxy(proxy) {
            return connect_ws_via_http_proxy(ws_url, &proxy_host, proxy_port).await;
        }
        // Non-HTTP proxy (e.g. SOCKS): not tunnelled here; attempt direct.
        logger::warning(
            LogTag::Websocket,
            &format!("Proxy '{proxy}' is not an HTTP proxy — attempting direct WebSocket connect"),
        );
    }
    let (stream, _) = connect_async(ws_url)
        .await
        .map_err(|e| format!("Failed to connect to WebSocket: {e}"))?;
    Ok(stream)
}

/// Parse "http://host:port" → (host, port). Returns None for non-http schemes.
fn parse_http_proxy(proxy: &str) -> Option<(String, u16)> {
    let rest = proxy
        .strip_prefix("http://")
        .or_else(|| proxy.strip_prefix("https://"))?;
    let authority = rest.split('/').next().unwrap_or(rest);
    let (host, port) = authority.rsplit_once(':')?;
    Some((host.to_owned(), port.parse().ok()?))
}

async fn connect_ws_via_http_proxy(
    ws_url: &str,
    proxy_host: &str,
    proxy_port: u16,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    // Target host/port from the ws(s) URL.
    let url = url::Url::parse(ws_url).map_err(|e| format!("Invalid ws url: {e}"))?;
    let target_host = url
        .host_str()
        .ok_or_else(|| "ws url has no host".to_owned())?
        .to_owned();
    let target_port = url
        .port_or_known_default()
        .unwrap_or(if url.scheme() == "wss" { 443 } else { 80 });

    // 1) TCP to the proxy.
    let mut tcp = TcpStream::connect((proxy_host, proxy_port))
        .await
        .map_err(|e| format!("Proxy TCP connect failed: {e}"))?;

    // 2) HTTP CONNECT tunnel to the target.
    let connect_req = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Connection: keep-alive\r\n\r\n",
        host = target_host,
        port = target_port,
    );
    tcp.write_all(connect_req.as_bytes())
        .await
        .map_err(|e| format!("Proxy CONNECT write failed: {e}"))?;

    // Read the proxy response headers (until CRLFCRLF).
    let mut buf = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    loop {
        let n = tcp
            .read(&mut byte)
            .await
            .map_err(|e| format!("Proxy CONNECT read failed: {e}"))?;
        if n == 0 {
            return Err("Proxy closed connection during CONNECT".to_owned());
        }
        buf.push(byte[0]);
        if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if buf.len() > 8192 {
            return Err("Proxy CONNECT response too large".to_owned());
        }
    }
    let head = String::from_utf8_lossy(&buf);
    let status_ok = head
        .lines()
        .next()
        .map(|l| l.contains(" 200"))
        .unwrap_or(false);
    if !status_ok {
        let first = head.lines().next().unwrap_or("").to_owned();
        return Err(format!("Proxy CONNECT rejected: {first}"));
    }

    // 3) TLS + WebSocket handshake over the tunnelled stream.
    let config: Option<WebSocketConfig> = None;
    let (stream, _) = client_async_tls_with_config(ws_url, tcp, config, None)
        .await
        .map_err(|e| format!("WebSocket handshake over proxy failed: {e}"))?;
    Ok(stream)
}

fn detect_proxy() -> Option<String> {
    // 1. Config `[network] proxy` — highest priority (works for GUI-launched apps
    //    that don't inherit shell env vars).
    if let Some(url) = detect_from_config() {
        return Some(url);
    }

    // 2. Environment variables (works for terminal-launched runs).
    if let Some(url) = detect_from_env() {
        return Some(url);
    }

    // 3. macOS system proxy (scutil --proxy).
    detect_macos_system_proxy()
}

/// Read the proxy URL from the bot's config (`[network] proxy`).
/// Returns None if config isn't loaded yet or the field is empty.
/// Uses `CONFIG.get()` directly (not `with_config`) to avoid panicking
/// when the proxy is detected before config is loaded at boot.
fn detect_from_config() -> Option<String> {
    let config_lock = crate::config::utils::CONFIG.get()?;
    let config = config_lock.read().ok()?;
    let proxy = config.network.proxy.trim().to_owned();
    if proxy.is_empty() {
        return None;
    }
    Some(normalize_proxy_url(&proxy))
}

fn detect_from_env() -> Option<String> {
    for key in [
        "ALL_PROXY",
        "all_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ] {
        if let Ok(val) = std::env::var(key) {
            let val = val.trim();
            if !val.is_empty() {
                return Some(normalize_proxy_url(val));
            }
        }
    }
    None
}

/// Ensure the proxy string has a scheme reqwest accepts.
fn normalize_proxy_url(raw: &str) -> String {
    if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("http://{raw}")
    }
}

/// Read the macOS system proxy via `scutil --proxy`. Returns an HTTP proxy URL
/// when an HTTP/HTTPS proxy is enabled, otherwise a SOCKS proxy URL. No-op (None)
/// on non-macOS targets.
#[cfg(target_os = "macos")]
fn detect_macos_system_proxy() -> Option<String> {
    let output = std::process::Command::new("scutil")
        .arg("--proxy")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);

    let get = |key: &str| -> Option<String> {
        text.lines().find_map(|line| {
            let line = line.trim();
            line.strip_prefix(key)
                .and_then(|rest| rest.trim().strip_prefix(':'))
                .map(|v| v.trim().to_owned())
        })
    };
    let enabled = |key: &str| get(key).as_deref() == Some("1");

    // Prefer an HTTP/HTTPS proxy (a single CONNECT-capable proxy handles both).
    if enabled("HTTPSEnable") {
        if let (Some(host), Some(port)) = (get("HTTPSProxy"), get("HTTPSPort")) {
            return Some(format!("http://{host}:{port}"));
        }
    }
    if enabled("HTTPEnable") {
        if let (Some(host), Some(port)) = (get("HTTPProxy"), get("HTTPPort")) {
            return Some(format!("http://{host}:{port}"));
        }
    }
    if enabled("SOCKSEnable") {
        if let (Some(host), Some(port)) = (get("SOCKSProxy"), get("SOCKSPort")) {
            return Some(format!("socks5://{host}:{port}"));
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn detect_macos_system_proxy() -> Option<String> {
    None
}
