// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Redaction of credential-bearing URL parts.
//!
//! RPC and API providers authenticate with secrets embedded in the endpoint URL:
//! query parameters (`?api-key=`), userinfo (`user:pass@`), or a path segment on
//! providers that issue path tokens. `redact_url` masks exactly those parts and
//! keeps the scheme, host and the rest of the URL so the endpoint stays
//! identifiable. Every URL that is logged, put into an error message, or
//! returned by the API goes through it.

use std::borrow::Cow;

/// Replacement written in place of a credential.
const REDACTED: &str = "***";

/// Query or fragment parameter names whose values are credentials
/// (compared case-insensitively).
const CREDENTIAL_PARAMS: &[&str] = &[
    "api-key",
    "api_key",
    "apikey",
    "x-api-key",
    "key",
    "token",
    "access_token",
    "auth",
    "auth_token",
    "secret",
    "client_secret",
    "password",
];

/// Name suffixes that also mark a parameter as a credential
/// (`refresh_token`, `x-cg-demo-api-key`).
const CREDENTIAL_PARAM_SUFFIXES: &[&str] =
    &["_key", "-key", "_token", "-token", "_secret", "-secret"];

/// Hosts (and their subdomains) that carry the credential as a path segment.
const PATH_TOKEN_HOSTS: &[&str] = &[
    "quiknode.pro",
    "quicknode.pro",
    "alchemy.com",
    "rpcpool.com",
    "getblock.io",
    "api.telegram.org",
];

/// On a `PATH_TOKEN_HOSTS` host, a path segment at least this long is a token.
/// Method and version segments (`v2`, `sendMessage`) stay below it.
const PATH_TOKEN_MIN_LEN: usize = 16;

/// Return `url` with every credential-bearing part replaced by `***`:
/// userinfo, credential query/fragment parameter values, and token path
/// segments on providers that authenticate by path.
pub fn redact_url(url: &str) -> String {
    let (scheme, rest) = match url.find("://") {
        Some(index) => url.split_at(index + 3),
        None => ("", url),
    };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(authority_end);
    let (has_userinfo, host_port) = match authority.rfind('@') {
        Some(index) => (true, &authority[index + 1..]),
        None => (false, authority),
    };

    let (before_fragment, fragment) = match tail.find('#') {
        Some(index) => (&tail[..index], Some(&tail[index + 1..])),
        None => (tail, None),
    };
    let (path, query) = match before_fragment.find('?') {
        Some(index) => (
            &before_fragment[..index],
            Some(&before_fragment[index + 1..]),
        ),
        None => (before_fragment, None),
    };

    let mut out = String::with_capacity(url.len());
    out.push_str(scheme);
    if has_userinfo {
        out.push_str(REDACTED);
        out.push('@');
    }
    out.push_str(host_port);

    let host = host_port
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if is_path_token_host(&host) {
        let segments: Vec<&str> = path
            .split('/')
            .map(|segment| {
                if segment.len() >= PATH_TOKEN_MIN_LEN {
                    REDACTED
                } else {
                    segment
                }
            })
            .collect();
        out.push_str(&segments.join("/"));
    } else {
        out.push_str(path);
    }

    if let Some(query) = query {
        out.push('?');
        out.push_str(&redact_params(query));
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(&redact_params(fragment));
    }
    out
}

/// Apply `redact_url` to every `scheme://` URL embedded in free text.
///
/// Borrows the input unchanged when it contains no URL or no URL needed
/// redaction, so the common case costs one substring scan.
pub fn redact_urls_in(text: &str) -> Cow<'_, str> {
    if !text.contains("://") {
        return Cow::Borrowed(text);
    }

    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut changed = false;
    let mut cursor = 0;
    let mut search = 0;

    while let Some(relative) = text[search..].find("://") {
        let separator = search + relative;

        let mut start = separator;
        while start > cursor && is_scheme_byte(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = separator + 3;
        while end < bytes.len() && !is_url_terminator(bytes[end]) {
            end += 1;
        }

        let candidate = &text[start..end];
        let redacted = redact_url(candidate);
        if redacted != candidate {
            if !changed {
                out.reserve(text.len());
                changed = true;
            }
            out.push_str(&text[cursor..start]);
            out.push_str(&redacted);
            cursor = end;
        }
        search = end;
    }

    if !changed {
        return Cow::Borrowed(text);
    }
    out.push_str(&text[cursor..]);
    Cow::Owned(out)
}

fn is_path_token_host(host: &str) -> bool {
    PATH_TOKEN_HOSTS.iter().any(|token_host| {
        host == *token_host
            || host
                .strip_suffix(token_host)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

fn redact_params(params: &str) -> String {
    params
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, value)) if !value.is_empty() && is_credential_param(name) => {
                format!("{name}={REDACTED}")
            }
            _ => pair.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn is_credential_param(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    CREDENTIAL_PARAMS.contains(&lower.as_str())
        || CREDENTIAL_PARAM_SUFFIXES
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

/// Scheme characters per RFC 3986 (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`).
fn is_scheme_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
}

/// Bytes that end a URL embedded in prose or in an error message. Non-ASCII
/// bytes also terminate, which keeps every slice boundary on a char boundary.
fn is_url_terminator(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || !byte.is_ascii()
        || matches!(byte, b'"' | b'\'' | b'<' | b'>' | b')' | b'`')
}

#[cfg(test)]
mod tests {
    use super::{redact_url, redact_urls_in};
    use std::borrow::Cow;

    #[test]
    fn masks_credential_query_values_case_insensitively() {
        assert_eq!(
            redact_url("https://mainnet.helius-rpc.com/?api-key=secret"),
            "https://mainnet.helius-rpc.com/?api-key=***"
        );
        assert_eq!(
            redact_url("https://host/path?a=1&API_KEY=secret&b=2&refresh_token=secret#x"),
            "https://host/path?a=1&API_KEY=***&b=2&refresh_token=***#x"
        );
        assert_eq!(
            redact_url(
                "https://generativelanguage.googleapis.com/v1/models/m:generateContent?key=secret"
            ),
            "https://generativelanguage.googleapis.com/v1/models/m:generateContent?key=***"
        );
    }

    #[test]
    fn masks_userinfo_and_fragment_credentials() {
        assert_eq!(
            redact_url("http://user:secret@127.0.0.1:8080/x"),
            "http://***@127.0.0.1:8080/x"
        );
        assert_eq!(
            redact_url("https://host/cb#access_token=secret&state=1"),
            "https://host/cb#access_token=***&state=1"
        );
    }

    #[test]
    fn masks_path_tokens_only_on_path_token_hosts() {
        assert_eq!(
            redact_url("https://a-b-c.solana-mainnet.quiknode.pro/0123456789abcdef0123/"),
            "https://a-b-c.solana-mainnet.quiknode.pro/***/"
        );
        assert_eq!(
            redact_url("https://solana-mainnet.g.alchemy.com/v2/0123456789abcdef0123"),
            "https://solana-mainnet.g.alchemy.com/v2/***"
        );
        assert_eq!(
            redact_url("https://api.telegram.org/bot123456:ABCDEFGHIJKLMNOP/sendMessage"),
            "https://api.telegram.org/***/sendMessage"
        );
        let mint_url =
            "https://api.dexscreener.com/tokens/v1/solana/So11111111111111111111111111111111111111112";
        assert_eq!(redact_url(mint_url), mint_url);
        assert_eq!(
            redact_url("https://notalchemy.com/v2/0123456789abcdef0123"),
            "https://notalchemy.com/v2/0123456789abcdef0123"
        );
    }

    #[test]
    fn leaves_urls_without_credentials_unchanged() {
        for url in [
            "https://api.mainnet-beta.solana.com",
            "wss://api.mainnet-beta.solana.com/",
            "https://host/path?pubkey=abc&limit=10",
            "https://host/?api-key=",
        ] {
            assert_eq!(redact_url(url), url);
        }
    }

    #[test]
    fn redacts_urls_embedded_in_text() {
        let text = "Network timeout: error sending request for url (https://mainnet.helius-rpc.com/?api-key=secret) after retry";
        let redacted = redact_urls_in(text);
        assert_eq!(
            redacted,
            "Network timeout: error sending request for url (https://mainnet.helius-rpc.com/?api-key=***) after retry"
        );

        let two = "a wss://h/?token=s1 b http://u:s2@h/ c";
        assert_eq!(
            redact_urls_in(two),
            "a wss://h/?token=*** b http://***@h/ c"
        );

        let plain = "pool fetched from https://api.example.com/pools?limit=5 – ok";
        assert!(matches!(redact_urls_in(plain), Cow::Borrowed(_)));
        assert!(matches!(redact_urls_in("no url here"), Cow::Borrowed(_)));
    }
}
