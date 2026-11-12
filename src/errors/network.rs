// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Network errors — HTTP request failures, timeouts, and connection issues.
//!
//! `endpoint` is always rendered through `redact_url`: provider endpoints carry
//! API keys in their URL, and these messages reach logs and API responses.

use crate::logger::redact_url;

const HTTP_BODY_MAX_CHARS: usize = 300;

/// Opening words of the summary of an HTML response body (`summarize_http_body`).
const HTML_PAGE_SUMMARY: &str = "HTML page";

/// Summarizes an HTTP response body before it reaches logs or error messages.
pub fn summarize_http_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lower = trimmed.to_ascii_lowercase();
    if trimmed.starts_with('<') || lower.contains("<html") || lower.contains("<!doctype") {
        let title = lower.find("<title").and_then(|start| {
            let open_end = trimmed[start..].find('>')? + start + 1;
            let close = lower[open_end..].find("</title>")? + open_end;
            let title = trimmed[open_end..close]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            (!title.is_empty() && title.chars().count() <= 80).then_some(title)
        });
        return match title {
            Some(title) => format!("{HTML_PAGE_SUMMARY} \"{title}\", {} bytes", body.len()),
            None => format!("{HTML_PAGE_SUMMARY}, {} bytes", body.len()),
        };
    }

    let single_line = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= HTTP_BODY_MAX_CHARS {
        return single_line;
    }
    let mut end = 0;
    for (index, character) in single_line.char_indices().take(HTTP_BODY_MAX_CHARS) {
        end = index + character.len_utf8();
    }
    format!("{}…, {} bytes", &single_line[..end], body.len())
}

impl NetworkError {
    /// Creates an HTTP status error with a bounded, single-line response body.
    pub fn http_status(endpoint: impl Into<String>, status: u16, body: Option<String>) -> Self {
        Self::HttpStatus {
            endpoint: endpoint.into(),
            status,
            body: body
                .map(|body| summarize_http_body(&body))
                .filter(|body| !body.is_empty()),
        }
    }

    /// Whether the endpoint refused with a 403 whose body is an HTML page: the challenge or
    /// error page of the CDN in front of an API, which blocks the client as a whole and says
    /// nothing about the request it answers.
    pub fn is_html_page_refusal(&self) -> bool {
        matches!(
            self,
            Self::HttpStatus { status: 403, body: Some(body), .. }
                if body.starts_with(HTML_PAGE_SUMMARY)
        )
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum NetworkError {
    /// The request could not be sent or the connection failed outright
    /// (DNS, TCP, TLS, or the transport dropping mid-request).
    #[error("request to {} failed: {detail}", redact_url(endpoint))]
    RequestFailed { endpoint: String, detail: String },
    /// The endpoint responded, but with a non-success HTTP status.
    #[error("{} returned HTTP {status}{}", redact_url(endpoint), body.as_deref().map(|b| format!(": {b}")).unwrap_or_default())]
    HttpStatus {
        endpoint: String,
        status: u16,
        body: Option<String>,
    },
    /// The request did not complete within its deadline.
    #[error("request to {} timed out after {timeout_ms}ms", redact_url(endpoint))]
    Timeout { endpoint: String, timeout_ms: u64 },
    /// The endpoint rejected the request for exceeding its rate limit.
    #[error("rate limited by {}", redact_url(endpoint))]
    RateLimited {
        endpoint: String,
        retry_after_ms: Option<u64>,
    },
}

#[cfg(test)]
mod tests {
    use super::{summarize_http_body, NetworkError};

    #[test]
    fn html_error_pages_are_summarized_on_one_line() {
        let body = "<!DOCTYPE html><html><head><title>Just a moment...</title></head>\n<body>challenge</body></html>";
        let summary = summarize_http_body(body);
        assert_eq!(
            summary,
            format!("HTML page \"Just a moment...\", {} bytes", body.len())
        );
        assert!(!summary.contains('\n'));
    }

    #[test]
    fn only_a_403_with_an_html_body_is_an_html_page_refusal() {
        let page = "<html><head><title>Challenge Verification</title></head></html>";
        let json = r#"{"errors":[{"status":"403"}]}"#;
        let cases = [
            ("403 with an HTML page", 403, Some(page), true),
            ("403 with JSON", 403, Some(json), false),
            ("403 without a body", 403, None, false),
            ("429 with an HTML page", 429, Some(page), false),
            ("503 with an HTML page", 503, Some(page), false),
        ];
        for (case, status, body, refusal) in cases {
            let error = NetworkError::http_status(
                "https://api.example.com/v2",
                status,
                body.map(String::from),
            );
            assert_eq!(error.is_html_page_refusal(), refusal, "{case}");
        }
    }

    #[test]
    fn long_json_is_bounded_and_reports_original_size() {
        let body = format!("{{\"data\":\"{}\"}}", "x".repeat(400));
        let summary = summarize_http_body(&body);
        assert!(summary.starts_with('{'));
        assert!(summary.contains(&format!("…, {} bytes", body.len())));
        assert!(summary.chars().count() < 330);
    }

    #[test]
    fn truncation_preserves_utf8_boundaries() {
        let body = "界".repeat(340);
        let summary = summarize_http_body(&body);
        let truncation = summary.find('…').unwrap();
        assert!(summary.is_char_boundary(truncation));
        assert!(summary.starts_with(&"界".repeat(super::HTTP_BODY_MAX_CHARS)));
        assert!(summary.ends_with(&format!("…, {} bytes", body.len())));
    }

    #[test]
    fn empty_body_has_no_summary() {
        assert_eq!(summarize_http_body(" \n\t "), "");
    }

    #[test]
    fn response_status_literals_use_the_constructor() {
        fn visit(directory: &std::path::Path) {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path);
                } else if path.extension().is_some_and(|extension| extension == "rs")
                    && path.file_name().is_some_and(|name| name != "network.rs")
                {
                    let source = std::fs::read_to_string(&path).unwrap();
                    for (index, _) in source.match_indices("HttpStatus {") {
                        let tail = &source[index..source.len().min(index + 160)];
                        assert!(
                            !tail.contains("endpoint:") && !tail.contains("endpoint,"),
                            "{}",
                            path.display()
                        );
                    }
                }
            }
        }
        visit(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    }
}
