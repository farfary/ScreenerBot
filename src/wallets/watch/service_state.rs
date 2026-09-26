//! Observation-loop state other modules read: per-target subscription liveness
//! and the short WS retry schedule for signatures the RPC has not
//! indexed yet.
//!
//! Kept out of `service.rs` (module-size limit) and behind plain accessors so the
//! status API never reaches into the loop itself.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::mpsc;

use super::types::WatchMode;

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

pub(super) fn ws_retry_delay(attempt: u32) -> Option<Duration> {
    (attempt < WS_RETRY_ATTEMPTS).then(|| WS_RETRY_BASE * 2u32.pow(attempt))
}

/// Per-address forwarder liveness. Transport connectivity alone is not enough to
/// claim a target streams: its subscription can fail or end independently.
static SUBSCRIPTIONS: LazyLock<RwLock<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Clone)]
struct RuntimeStatus {
    mode: WatchMode,
    catching_up: bool,
    last_checked_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

static RUNTIME_STATUS: LazyLock<RwLock<HashMap<String, RuntimeStatus>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub(super) fn update_runtime_status(
    address: &str,
    mode: WatchMode,
    catching_up: bool,
    completed_successfully: bool,
) {
    let mut statuses = RUNTIME_STATUS.write().unwrap_or_else(|p| p.into_inner());
    let last_error = if completed_successfully {
        None
    } else {
        statuses
            .get(address)
            .and_then(|status| status.last_error.clone())
    };
    statuses.insert(
        address.to_owned(),
        RuntimeStatus {
            mode,
            catching_up,
            last_checked_at: Some(Utc::now()),
            last_error,
        },
    );
}

pub(super) fn remove_runtime_status(address: &str) {
    RUNTIME_STATUS
        .write()
        .unwrap_or_else(|p| p.into_inner())
        .remove(address);
}

pub(super) fn watch_mode(address: &str) -> WatchMode {
    RUNTIME_STATUS
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .get(address)
        .map(|status| status.mode)
        .unwrap_or(WatchMode::Standard)
}

pub(super) fn catching_up(address: &str) -> bool {
    RUNTIME_STATUS
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .get(address)
        .is_some_and(|status| status.catching_up)
}

pub(super) fn last_checked_at(address: &str) -> Option<DateTime<Utc>> {
    RUNTIME_STATUS
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .get(address)
        .and_then(|status| status.last_checked_at)
}

pub(super) fn set_runtime_error(address: &str, summary: &'static str) {
    let mut statuses = RUNTIME_STATUS.write().unwrap_or_else(|p| p.into_inner());
    let status = statuses.entry(address.to_owned()).or_insert(RuntimeStatus {
        mode: WatchMode::Standard,
        catching_up: false,
        last_checked_at: None,
        last_error: None,
    });
    status.last_error = Some(summary.to_owned());
}

pub(super) fn runtime_error(address: &str) -> Option<String> {
    RUNTIME_STATUS
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .get(address)
        .and_then(|status| status.last_error.clone())
}

pub(super) fn set_subscription(address: String, active: Arc<AtomicBool>) {
    SUBSCRIPTIONS
        .write()
        .unwrap_or_else(|p| p.into_inner())
        .insert(address, active);
}

pub(super) fn remove_subscription(address: &str) {
    SUBSCRIPTIONS
        .write()
        .unwrap_or_else(|p| p.into_inner())
        .remove(address);
}

pub(super) fn subscription_active(address: &str) -> bool {
    SUBSCRIPTIONS
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .get(address)
        .is_some_and(|active| active.load(std::sync::atomic::Ordering::Relaxed))
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
    fn subscription_status_tracks_the_per_address_forwarder() {
        let active = Arc::new(AtomicBool::new(false));
        set_subscription("SatAddr".to_owned(), Arc::clone(&active));
        assert!(!subscription_active("SatAddr"));
        active.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(subscription_active("SatAddr"));
        remove_subscription("SatAddr");
        assert!(!subscription_active("SatAddr"));
    }

    #[test]
    fn failed_status_update_keeps_safe_runtime_error_until_a_successful_check() {
        let address = "status-preserve";
        set_runtime_error(address, "High-activity provider check failed; retrying");
        update_runtime_status(address, WatchMode::HeliusHighActivity, true, false);
        assert_eq!(
            runtime_error(address).as_deref(),
            Some("High-activity provider check failed; retrying")
        );
        update_runtime_status(address, WatchMode::HeliusHighActivity, false, true);
        assert!(runtime_error(address).is_none());
        remove_runtime_status(address);
    }
}
