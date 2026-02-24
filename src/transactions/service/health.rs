// Health monitoring and metrics tracking

use chrono::{DateTime, Utc};
use std::time::Duration;

use crate::logger::{self, LogTag};
use crate::transactions::utils::get_pending_transactions_count;

// =============================================================================
// HEALTH MONITORING
// =============================================================================

/// Service performance metrics
#[derive(Debug)]
pub struct ServiceMetrics {
    pub service_start_time: Option<DateTime<Utc>>,
    pub last_periodic_check: Option<DateTime<Utc>>,
    pub last_websocket_activity: Option<DateTime<Utc>>,
    pub last_websocket_message: Option<DateTime<Utc>>, // Track ANY WebSocket message (pings, pongs, etc)
    pub last_websocket_health_check: Option<DateTime<Utc>>,
    pub websocket_reconnect_count: u64,
    pub periodic_check_count: u64,
    pub websocket_transaction_count: u64,
    pub error_count: u64,
    pub average_check_duration_ms: f64,
}

impl ServiceMetrics {
    pub fn new() -> Self {
        Self {
            service_start_time: Some(Utc::now()),
            last_periodic_check: None,
            last_websocket_activity: None,
            last_websocket_message: None,
            last_websocket_health_check: None,
            websocket_reconnect_count: 0,
            periodic_check_count: 0,
            websocket_transaction_count: 0,
            error_count: 0,
            average_check_duration_ms: 0.0,
        }
    }

    pub fn update_periodic_check(
        &mut self,
        duration: Duration,
        expired_count: usize,
        retry_count: usize,
        fallback_count: usize,
    ) {
        self.last_periodic_check = Some(Utc::now());
        self.periodic_check_count += 1;

        let duration_ms = duration.as_millis() as f64;
        self.average_check_duration_ms = if self.periodic_check_count == 1 {
            duration_ms
        } else {
            (self.average_check_duration_ms * ((self.periodic_check_count - 1) as f64)
                + duration_ms)
                / (self.periodic_check_count as f64)
        };
    }

    pub fn update_websocket_activity(&mut self) {
        let now = Utc::now();
        self.last_websocket_activity = Some(now);
        self.last_websocket_message = Some(now);
        self.websocket_transaction_count += 1;
    }

    pub fn update_websocket_message(&mut self) {
        self.last_websocket_message = Some(Utc::now());
    }

    pub fn update_websocket_health_check(&mut self) {
        self.last_websocket_health_check = Some(Utc::now());
    }

    pub fn increment_websocket_reconnect(&mut self) {
        self.websocket_reconnect_count += 1;
    }

    pub fn is_websocket_healthy(&self) -> bool {
        // WebSocket health is determined by whether the receiver channel is still open
        // If the WebSocket loop crashes or connection fails, the channel closes
        // This method is called only when receiver exists, so if we get here, it's healthy
        //
        // We use a conservative timeout (5 minutes) for transaction message silence
        // to avoid false positives during idle periods when no transactions occur
        if let Some(last_msg) = self.last_websocket_message {
            let silence_duration = (Utc::now() - last_msg).num_seconds();
            silence_duration < 300 // 5 minutes instead of 30 seconds
        } else {
            // No transaction messages yet - connection is healthy if recently initialized
            if let Some(activity) = self.last_websocket_activity {
                let since_activity = (Utc::now() - activity).num_seconds();
                since_activity < 300 // 5 minutes grace period
            } else {
                // No activity at all yet - assume healthy
                true
            }
        }
    }

    pub fn increment_error(&mut self) {
        self.error_count += 1;
    }
}

/// Perform service health check
pub async fn perform_health_check(metrics: &mut ServiceMetrics) -> Result<(), String> {
    let now = Utc::now();

    // Check if periodic checks are running
    if let Some(last_check) = metrics.last_periodic_check {
        let time_since_check = (now - last_check).num_seconds();
        if time_since_check > 300 {
            // 5 minutes
            logger::info(
                LogTag::Transactions,
                &format!("No periodic check in {} seconds", time_since_check),
            );
        }
    }

    // Check database connectivity
    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        if let Err(e) = db.health_check().await {
            return Err(format!("Database health check failed: {e}"));
        }
    }

    // Log health metrics
    logger::debug(
        LogTag::Transactions,
        &format!(
            "Service health: checks: {}, websocket: {}, errors: {}, avg_duration: {:.1}ms",
            metrics.periodic_check_count,
            metrics.websocket_transaction_count,
            metrics.error_count,
            metrics.average_check_duration_ms
        ),
    );

    Ok(())
}

/// Determine if fallback check should be performed
///
/// Fallback is only performed when:
/// 1. There are pending transactions OR pending position verifications to monitor
/// 2. WebSocket has been silent for >10 seconds (or never connected with >10s uptime)
pub async fn should_perform_fallback_check(metrics: &ServiceMetrics) -> bool {
    // Step 1: Check if there's anything to monitor
    let has_pending_txs = get_pending_transactions_count().await > 0;

    // Check positions verification queue for pending work
    let (pending_verification_count, _) = crate::positions::queue::get_queue_status().await;
    let has_pending_verifications = pending_verification_count > 0;

    if !has_pending_txs && !has_pending_verifications {
        // Idle state - no need for fallback
        logger::debug(
            LogTag::Transactions,
            "Skipping fallback: no pending transactions or verifications (idle state)",
        );
        return false;
    }

    // Step 2: Check WebSocket activity with grace period
    if let Some(last_activity) = metrics.last_websocket_activity {
        let silence_duration = (Utc::now() - last_activity).num_seconds();

        // Only fallback if WS has been silent for >10 seconds
        if silence_duration > 10 {
            logger::debug(
        LogTag::Transactions,
        &format!(
          "Fallback triggered: WebSocket silent for {}s (pending txs: {}, pending verifications: {})",
          silence_duration,
          has_pending_txs,
          has_pending_verifications
        )
      );
            true
        } else {
            false
        }
    } else {
        // WebSocket never connected or failed
        // Only fallback if we've been running for >10 seconds (startup grace)
        if let Some(service_start) = metrics.service_start_time {
            let uptime = (Utc::now() - service_start).num_seconds();
            if uptime > 10 {
                logger::debug(
          LogTag::Transactions,
          &format!(
            "Fallback triggered: WebSocket never connected, uptime {}s (pending txs: {}, pending verifications: {})",
            uptime,
            has_pending_txs,
            has_pending_verifications
          )
        );
                true
            } else {
                logger::debug(
                    LogTag::Transactions,
                    &format!("Waiting for WebSocket to connect (uptime: {}s)", uptime),
                );
                false
            }
        } else {
            // Just started, give WS time to connect
            false
        }
    }
}
