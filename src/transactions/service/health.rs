//! Transaction service health — periodic housekeeping metrics for the own-wallet service.
//
// Detection health (WebSocket connectivity, poll cadence) is owned by
// `wallets::watch::is_healthy()` now; this module only tracks this service's own
// remaining job -- deferred retries and expired-pending cleanup.

use chrono::{DateTime, Utc};
use std::time::Duration;

use crate::logger::{self, LogTag};

// =============================================================================
// HEALTH MONITORING
// =============================================================================

/// Service performance metrics
#[derive(Debug)]
pub struct ServiceMetrics {
    pub service_start_time: Option<DateTime<Utc>>,
    pub last_periodic_check: Option<DateTime<Utc>>,
    pub periodic_check_count: u64,
    /// Own-wallet activities consumed from `wallets::watch::subscribe_activity()`.
    pub activity_count: u64,
    pub error_count: u64,
    pub average_check_duration_ms: f64,
}

impl ServiceMetrics {
    pub fn new() -> Self {
        Self {
            service_start_time: Some(Utc::now()),
            last_periodic_check: None,
            periodic_check_count: 0,
            activity_count: 0,
            error_count: 0,
            average_check_duration_ms: 0.0,
        }
    }

    pub fn update_periodic_check(
        &mut self,
        duration: Duration,
        expired_count: usize,
        retry_count: usize,
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

        if expired_count > 0 || retry_count > 0 {
            logger::debug(
                LogTag::Transactions,
                &format!(
                    "Periodic check: {duration_ms}ms, expired={expired_count}, retries={retry_count}"
                ),
            );
        }
    }

    pub fn update_activity(&mut self) {
        self.activity_count += 1;
    }

    pub fn increment_error(&mut self) {
        self.error_count += 1;
    }
}

/// Perform service health check
pub async fn perform_health_check(metrics: &mut ServiceMetrics) -> crate::transactions::Result<()> {
    let now = Utc::now();

    if let Some(last_check) = metrics.last_periodic_check {
        let time_since_check = (now - last_check).num_seconds();
        if time_since_check > 300 {
            logger::info(
                LogTag::Transactions,
                &format!("No periodic check in {time_since_check} seconds"),
            );
        }
    }

    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        db.health_check().await?;
    }

    logger::debug(
        LogTag::Transactions,
        &format!(
            "Service health: checks: {}, activity: {}, errors: {}, avg_duration: {:.1}ms",
            metrics.periodic_check_count,
            metrics.activity_count,
            metrics.error_count,
            metrics.average_check_duration_ms
        ),
    );

    Ok(())
}
