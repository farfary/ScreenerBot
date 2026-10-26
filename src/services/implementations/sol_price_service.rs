// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! SOL price service — tracks real-time SOL/USD price for portfolio valuation.

use crate::i18n::{ids, UiArg, UiText};
use crate::native_price::NativePriceData;
use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct SolPriceService;

#[async_trait]
impl Service for SolPriceService {
    fn name(&self) -> &'static str {
        "sol_price"
    }

    fn priority(&self) -> i32 {
        120
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    /// The price comes from public HTTP sources and needs no wallet or RPC, so it runs in
    /// Explore Mode as well as after setup; entering Explore Mode at runtime starts it
    /// with the other explore-tier services.
    fn is_enabled(&self) -> bool {
        crate::global::is_explore_or_full()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle =
            crate::native_price::start_native_price_service(shutdown.clone(), monitor.clone())
                .await
                .map_err(|e| {
                    crate::Error::Service(crate::errors::ServiceError::Start {
                        service: "sol_price".to_owned(),
                        message: format!("Failed to start SOL price service: {e}"),
                    })
                })?;

        // Also mirror the full SOL/USD reference chart from the data server so the
        // bot always has SOL's own price history (all timeframes) ready for display
        // and USDC->SOL conversion, prepared during runtime (never per request).
        let chart_handle =
            crate::ohlcvs::native_usd_chart::start(shutdown.clone(), monitor.clone());

        // Return both handles so ServiceManager can wait for graceful shutdown.
        Ok(vec![handle, chart_handle])
    }

    async fn health(&self) -> ServiceHealth {
        // Check if service is running
        if !crate::native_price::is_native_price_service_running() {
            return ServiceHealth::Unhealthy(UiText::new(
                ids::SERVICES_HEALTH_SOL_PRICE_NOT_RUNNING,
            ));
        }

        price_health(
            crate::native_price::get_native_price_info(),
            crate::native_price::native_price_service_uptime(),
            crate::native_price::first_fetch_window(),
        )
    }
}

/// Health of the price cache. Until the first fetch has either stored a price or failed
/// the service is starting, for at most `window` after start: the empty cache it begins
/// with is not a stale price, and counting it as a fault showed a services issue on every
/// start. Past the window, a fetch that never settled is a fault with that cause.
fn price_health(
    info: Option<NativePriceData>,
    uptime: Option<Duration>,
    window: Duration,
) -> ServiceHealth {
    match info {
        Some(info) if info.is_fresh() => ServiceHealth::Healthy,
        Some(info) if info.fetch_count == 0 && info.error_count == 0 => {
            if uptime.is_some_and(|uptime| uptime < window) {
                ServiceHealth::Starting
            } else {
                ServiceHealth::Degraded(
                    UiText::new(ids::SERVICES_HEALTH_SOL_PRICE_FIRST_FETCH_OVERDUE)
                        .arg("seconds", UiArg::Text(window.as_secs().to_string())),
                )
            }
        }
        Some(info) if info.fetch_count > 0 => ServiceHealth::Degraded(
            UiText::new(ids::SERVICES_HEALTH_SOL_PRICE_STALE)
                .arg("seconds", UiArg::Text(info.age_seconds().to_string())),
        ),
        _ => ServiceHealth::Degraded(UiText::new(ids::SERVICES_HEALTH_SOL_PRICE_NO_DATA)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache(fetch_count: u64, error_count: u64, is_valid: bool) -> NativePriceData {
        NativePriceData {
            price_usd: if is_valid { 150.0 } else { 0.0 },
            fetch_count,
            error_count,
            is_valid,
            ..NativePriceData::default()
        }
    }

    const WINDOW: Duration = Duration::from_secs(30);
    const INSIDE: Option<Duration> = Some(Duration::from_secs(29));

    #[test]
    fn an_unfetched_cache_is_starting_only_inside_the_first_fetch_window() {
        assert!(matches!(
            price_health(Some(cache(0, 0, false)), INSIDE, WINDOW),
            ServiceHealth::Starting
        ));
        for uptime in [Some(WINDOW), Some(WINDOW * 4), None] {
            let ServiceHealth::Degraded(overdue) =
                price_health(Some(cache(0, 0, false)), uptime, WINDOW)
            else {
                panic!("a first fetch still unsettled after {uptime:?} is a fault");
            };
            assert_eq!(
                overdue.id,
                ids::SERVICES_HEALTH_SOL_PRICE_FIRST_FETCH_OVERDUE.as_str()
            );
            assert!(overdue.render_source_plain().contains("30"));
        }
    }

    #[test]
    fn a_fresh_price_is_healthy() {
        assert!(matches!(
            price_health(Some(cache(1, 0, true)), INSIDE, WINDOW),
            ServiceHealth::Healthy
        ));
    }

    #[test]
    fn a_failed_first_fetch_has_no_data_and_a_lost_price_is_stale() {
        let ServiceHealth::Degraded(no_data) =
            price_health(Some(cache(0, 1, false)), INSIDE, WINDOW)
        else {
            panic!("a first fetch that failed is degraded");
        };
        assert_eq!(no_data.id, ids::SERVICES_HEALTH_SOL_PRICE_NO_DATA.as_str());
        let ServiceHealth::Degraded(stale) =
            price_health(Some(cache(3, 10, false)), INSIDE, WINDOW)
        else {
            panic!("an invalidated price is degraded");
        };
        assert_eq!(stale.id, ids::SERVICES_HEALTH_SOL_PRICE_STALE.as_str());
        assert!(matches!(
            price_health(None, INSIDE, WINDOW),
            ServiceHealth::Degraded(_)
        ));
    }
}
