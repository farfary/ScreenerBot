//! Shared HTTP transport for the aggregator routers.
//!
//! Jupiter and Raptor are both "ask a hosted API for a quote, then ask it to
//! build a transaction" routers, and both are served from free endpoints that
//! rate-limit or fail transiently. The retry loop, the backoff curve and the
//! shape of a captured failure are therefore identical for both and live here
//! exactly once.
//!
//! What is NOT shared is classification. Turning a status code and a response
//! body into a [`crate::swaps::error::QuoteError`] depends on the provider's own
//! wire vocabulary, so each router owns that translation next to the API it
//! tracks — see `jupiter::jupiter_quote_error` and `raptor::raptor_quote_error`.
//! Sharing the transport and splitting the meaning is the whole point: a
//! provider rewording a response breaks one function that exists to track it.

use crate::logger::{self, LogTag};
use std::time::Duration;

/// A router's HTTP call that failed, kept structured.
///
/// The quote path and the swap path need different things from the same
/// failure — one decides whether to retire a token, the other only reports — so
/// the transport hands back the status and the raw body and lets each caller
/// classify for its own channel. Rendering a message here and having callers
/// search it is what silently broke no-route blacklisting once already.
pub(crate) struct RouterHttpFailure {
    pub(crate) label: String,
    /// `None` when the request never got a response (DNS, TCP, TLS, timeout).
    pub(crate) status: Option<u16>,
    /// Response body, or the transport error when `status` is `None`.
    pub(crate) body: String,
    /// `Retry-After`, when the endpoint sent one.
    pub(crate) retry_after: Option<Duration>,
    /// The request never completed rather than completing unsuccessfully.
    pub(crate) timed_out: bool,
}

/// Compute a backoff delay. Honors a server `Retry-After` (seconds) when
/// present, otherwise exponential (~0.4s, 0.8s, 1.6s, 3.2s) capped at 4s, plus
/// small jitter.
pub(crate) fn backoff_delay(attempt: u32, retry_after_secs: Option<u64>) -> Duration {
    if let Some(secs) = retry_after_secs {
        return Duration::from_millis(secs.clamp(1, 5) * 1000);
    }
    let exp = 400u64.saturating_mul(1u64 << attempt.saturating_sub(1).min(4));
    let capped = exp.min(4000);
    let jitter = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.subsec_nanos() as u64) % 200)
        .unwrap_or(0);
    Duration::from_millis(capped + jitter)
}

/// Send a router HTTP request with retry+backoff on transient failures.
///
/// `build` is invoked fresh per attempt (a `RequestBuilder` is single-use), and
/// the caller applies its own per-call timeout there. Returns the success body
/// text on 2xx. Retries only on HTTP 429, 5xx and network/transport errors —
/// never on 4xx (e.g. 400 = no route), which are deterministic and surfaced
/// immediately so the caller can classify them.
///
/// Retrying is safe for both call sites it serves: a quote has no side effect,
/// and a swap-build only produces an UNSIGNED transaction — submission happens
/// later, separately.
pub(crate) async fn send_with_retry<F>(
    provider: &str,
    label: &str,
    max_attempts: u32,
    build: F,
) -> std::result::Result<String, RouterHttpFailure>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match build().send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return resp.text().await.map_err(|e| RouterHttpFailure {
                        label: label.to_owned(),
                        status: None,
                        body: format!("failed to read response: {e}"),
                        retry_after: None,
                        timed_out: e.is_timeout(),
                    });
                }
                let is_transient = status.as_u16() == 429 || status.is_server_error();
                let retry_after = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok());
                let body = resp.text().await.unwrap_or_else(|_| "Unknown".to_owned());
                if is_transient && attempt < max_attempts {
                    let delay = backoff_delay(attempt, retry_after);
                    logger::warning(
                        LogTag::Swap,
                        &format!(
                            "{provider} {label} transient {} (attempt {}/{}), retrying in {}ms",
                            status,
                            attempt,
                            max_attempts,
                            delay.as_millis()
                        ),
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(RouterHttpFailure {
                    label: label.to_owned(),
                    status: Some(status.as_u16()),
                    body,
                    retry_after: retry_after.map(Duration::from_secs),
                    timed_out: false,
                });
            }
            Err(e) => {
                if attempt < max_attempts {
                    let delay = backoff_delay(attempt, None);
                    logger::warning(
                        LogTag::Swap,
                        &format!(
                            "{provider} {label} network error (attempt {}/{}): {} - retrying in {}ms",
                            attempt,
                            max_attempts,
                            e,
                            delay.as_millis()
                        ),
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(RouterHttpFailure {
                    label: label.to_owned(),
                    status: None,
                    body: format!("request failed after {attempt} attempts: {e}"),
                    retry_after: None,
                    timed_out: e.is_timeout(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_delay_honors_retry_after_and_otherwise_grows_exponentially_with_a_cap() {
        assert_eq!(backoff_delay(1, Some(2)), Duration::from_millis(2000));
        // Clamped into [1, 5] seconds even for an extreme server value.
        assert_eq!(backoff_delay(1, Some(999)), Duration::from_millis(5000));

        let d1 = backoff_delay(1, None);
        let d2 = backoff_delay(2, None);
        let d4 = backoff_delay(4, None);
        assert!(
            d1 <= d2 && d2 <= d4,
            "delay must not shrink as attempts grow"
        );
        assert!(
            d4 <= Duration::from_millis(4000 + 200),
            "delay is capped near 4s plus jitter, got {d4:?}"
        );
    }
}
