//! Observation-loop state other modules read: which addresses the loop is actually
//! observing, which targets it gave up on as saturated, and the short WS retry
//! schedule for signatures the RPC has not indexed yet.
//!
//! Kept out of `service.rs` (module-size limit) and behind plain accessors so the
//! copy-trading reconciler and the status API never reach into the loop itself.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

/// A WS notification whose transaction was not decodable yet.
#[derive(Debug, Clone)]
pub(super) struct WsRetry {
    pub(super) address: String,
    pub(super) signature: String,
    /// When the notification originally arrived; retries never reset it, so the
    /// indexing wait is charged to the observation instead of hidden.
    pub(super) detected_at: DateTime<Utc>,
    pub(super) attempt: u32,
}

/// Decode retries a WS notification gets before the baseline poll takes over.
pub(super) const WS_RETRY_ATTEMPTS: u32 = 4;
const WS_RETRY_BASE: Duration = Duration::from_millis(400);

/// Re-feed `retry` into the loop after an exponential backoff (400ms, 800ms, ...).
/// Returns false once the attempts are spent, leaving the signature to the poll.
pub(super) fn schedule_ws_retry(tx: &mpsc::UnboundedSender<WsRetry>, retry: WsRetry) -> bool {
    let Some(delay) = ws_retry_delay(retry.attempt) else {
        return false;
    };
    let tx = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        let _ = tx.send(WsRetry {
            attempt: retry.attempt + 1,
            ..retry
        });
    });
    true
}

fn ws_retry_delay(attempt: u32) -> Option<Duration> {
    (attempt < WS_RETRY_ATTEMPTS).then(|| WS_RETRY_BASE * 2u32.pow(attempt))
}

/// `None` until the loop has built its target set once, so a consumer never mistakes
/// "not started yet" for "not observed".
static OBSERVED: LazyLock<RwLock<Option<HashSet<String>>>> = LazyLock::new(|| RwLock::new(None));

pub(super) fn set_observed(addresses: impl IntoIterator<Item = String>) {
    *OBSERVED.write().unwrap_or_else(|p| p.into_inner()) = Some(addresses.into_iter().collect());
}

pub(super) fn clear_observed() {
    *OBSERVED.write().unwrap_or_else(|p| p.into_inner()) = None;
}

/// Whether the running loop observes `address`; `None` while the loop is not
/// running or has not loaded its targets yet.
pub(super) fn is_observing(address: &str) -> Option<bool> {
    OBSERVED
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .map(|set| set.contains(address))
}

/// Why the loop disabled a target, by address. In memory on purpose: a restart
/// or re-enable is a fresh attempt, and the persisted `enabled = 0` is the state.
static SATURATED: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub(super) fn mark_saturated(address: &str, reason: String) {
    SATURATED
        .write()
        .unwrap_or_else(|p| p.into_inner())
        .insert(address.to_owned(), reason);
}

pub(super) fn clear_saturation(address: &str) {
    SATURATED
        .write()
        .unwrap_or_else(|p| p.into_inner())
        .remove(address);
}

pub(super) fn saturation_reason(address: &str) -> Option<String> {
    SATURATED
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .get(address)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_retry_backoff_doubles_and_stops_at_the_attempt_budget() {
        assert_eq!(ws_retry_delay(0), Some(Duration::from_millis(400)));
        assert_eq!(ws_retry_delay(1), Some(Duration::from_millis(800)));
        assert_eq!(
            ws_retry_delay(WS_RETRY_ATTEMPTS - 1),
            Some(Duration::from_millis(3200))
        );
        assert_eq!(ws_retry_delay(WS_RETRY_ATTEMPTS), None);
    }

    #[test]
    fn saturation_reason_round_trips_and_clears() {
        mark_saturated("SatAddr", "too busy".to_owned());
        assert_eq!(saturation_reason("SatAddr").as_deref(), Some("too busy"));
        clear_saturation("SatAddr");
        assert_eq!(saturation_reason("SatAddr"), None);
    }
}
