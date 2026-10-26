// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Whether a request comes from this machine: the loopback `Host`/`Origin` check the
//! security gate applies, and the stricter direct-peer check of loopback-only routes.

use axum::extract::ConnectInfo;
use axum::http::{header, uri::Authority, HeaderMap, Uri};
use std::net::SocketAddr;

/// Headers a reverse proxy or tunnel adds when it relays a request. Their presence means
/// the loopback peer is a relay, not the caller.
const FORWARDING_HEADERS: [&str; 3] = ["forwarded", "x-forwarded-for", "x-real-ip"];

/// The host and port of `value` when it names this machine: `localhost` or a
/// loopback IP literal (which no DNS answer can rebind), with or without a port.
fn parse_loopback_authority(value: &str) -> Option<(String, Option<u16>)> {
    let authority = value.parse::<Authority>().ok()?;
    let host = authority.host().to_ascii_lowercase();
    let is_loopback = host == "localhost"
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if !is_loopback {
        return None;
    }

    Some((host, authority.port_u16()))
}

/// Whether the request targets the loopback dashboard origin of the server listening on
/// [`crate::global::get_webserver_port`]: see [`has_valid_local_request_headers_for`].
pub(crate) fn has_valid_local_request_headers(headers: &HeaderMap) -> bool {
    has_valid_local_request_headers_for(headers, crate::global::get_webserver_port())
}

/// Whether the request targets the loopback dashboard origin of a server listening on
/// `server_port`: a loopback `Host` without a port or with `server_port`, and, when an
/// `Origin` is sent, the same plain-HTTP loopback origin.
fn has_valid_local_request_headers_for(headers: &HeaderMap, server_port: u16) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_loopback_authority);
    let Some(host) = host else {
        return false;
    };
    if host.1.is_some_and(|port| port != server_port) {
        return false;
    }

    let Some(origin) = headers.get("origin") else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(origin) = origin.parse::<Uri>() else {
        return false;
    };
    if origin.scheme_str() != Some("http") {
        return false;
    }
    if origin
        .path_and_query()
        .is_some_and(|path| path.as_str() != "/")
    {
        return false;
    }

    origin
        .authority()
        .and_then(|authority| parse_loopback_authority(authority.as_str()))
        .is_some_and(|origin| origin == host)
}

/// Whether the connection's peer is this machine. A request without a recorded peer
/// address is not.
pub(crate) fn is_loopback_peer(peer: Option<&ConnectInfo<SocketAddr>>) -> bool {
    peer.is_some_and(|ConnectInfo(addr)| addr.ip().to_canonical().is_loopback())
}

/// Whether a relay forwarded the request.
fn has_forwarding_headers(headers: &HeaderMap) -> bool {
    FORWARDING_HEADERS
        .iter()
        .any(|name| headers.contains_key(*name))
}

/// Whether a process on this machine called the server directly: a loopback peer, no
/// forwarding headers, and the loopback dashboard origin. A reverse proxy or tunnel on
/// the host makes every relayed request a loopback peer, and a DNS-rebinding page sends
/// its own host name; both are refused.
pub(crate) fn is_direct_local_request(
    headers: &HeaderMap,
    peer: Option<&ConnectInfo<SocketAddr>>,
) -> bool {
    is_loopback_peer(peer)
        && !has_forwarding_headers(headers)
        && has_valid_local_request_headers(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_headers(host: Option<&str>, origin: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(host) = host {
            headers.insert(header::HOST, host.parse().unwrap());
        }
        if let Some(origin) = origin {
            headers.insert("origin", origin.parse().unwrap());
        }
        headers
    }

    fn peer(addr: &str) -> ConnectInfo<SocketAddr> {
        ConnectInfo(addr.parse().expect("socket address"))
    }

    const PORT: u16 = 54321;

    fn is_local(host: Option<&str>, origin: Option<&str>) -> bool {
        has_valid_local_request_headers_for(&request_headers(host, origin), PORT)
    }

    #[test]
    fn accepts_loopback_host_and_same_origin() {
        assert!(is_local(
            Some("127.0.0.1:54321"),
            Some("http://127.0.0.1:54321")
        ));
        assert!(is_local(Some("localhost:54321"), None));
    }

    #[test]
    fn a_loopback_host_without_a_port_is_local() {
        assert!(is_local(Some("127.0.0.1"), None));
        assert!(is_local(Some("localhost"), Some("http://localhost")));
        assert!(is_local(Some("[::1]"), None));
        assert!(!is_local(Some("127.0.0.1"), Some("http://127.0.0.1:54321")));
    }

    #[test]
    fn a_loopback_host_naming_another_port_is_refused() {
        assert!(!is_local(Some("127.0.0.1:54322"), None));
        assert!(!is_local(
            Some("localhost:8080"),
            Some("http://localhost:8080")
        ));
        assert!(
            !has_valid_local_request_headers_for(
                &request_headers(Some("127.0.0.1:54321"), None),
                0
            ),
            "a server that has not bound a port accepts no explicit port"
        );
    }

    #[test]
    fn rejects_non_loopback_or_cross_origin_requests() {
        for (host, origin) in [
            (None, None),
            (Some("example.com:54321"), None),
            (Some("example.com"), None),
            (Some("127.0.0.1:54321"), Some("null")),
            (Some("127.0.0.1:54321"), Some("https://127.0.0.1:54321")),
            (Some("127.0.0.1:54321"), Some("http://127.0.0.1:54322")),
            (Some("127.0.0.1:54321"), Some("http://example.com:54321")),
        ] {
            assert!(!is_local(host, origin), "{host:?} {origin:?}");
        }
    }

    #[test]
    fn loopback_ip_literals_are_local_hosts() {
        assert!(is_local(Some("[::1]:54321"), Some("http://[::1]:54321")));
        assert!(is_local(Some("127.0.0.2:54321"), None));
        assert!(!is_local(
            Some("[::1]:54321"),
            Some("http://127.0.0.1:54321")
        ));
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

    #[test]
    fn a_direct_local_request_needs_a_loopback_peer_host_and_no_relay() {
        crate::global::set_webserver_port(8080);
        let local = request_headers(Some("127.0.0.1:8080"), None);
        let loopback = peer("127.0.0.1:50000");
        assert!(is_direct_local_request(&local, Some(&loopback)));

        assert!(!is_direct_local_request(
            &local,
            Some(&peer("192.168.1.20:50000"))
        ));
        assert!(!is_direct_local_request(&local, None));
        assert!(!is_direct_local_request(
            &request_headers(Some("bot.example.com:443"), None),
            Some(&loopback)
        ));
        assert!(!is_direct_local_request(
            &request_headers(None, None),
            Some(&loopback)
        ));
        for name in FORWARDING_HEADERS {
            let mut relayed = local.clone();
            relayed.insert(name, "203.0.113.9".parse().unwrap());
            assert!(
                !is_direct_local_request(&relayed, Some(&loopback)),
                "{name}"
            );
        }
    }
}
