// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Service health types — running state, error counts, and health check results.

use crate::i18n::UiText;
use serde::{Deserialize, Serialize};

/// Service health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "message")]
pub enum ServiceHealth {
    /// Service is operating normally
    #[serde(rename = "healthy")]
    Healthy,

    /// Service is operating but with degraded performance
    #[serde(rename = "degraded")]
    Degraded(UiText),

    /// Service has failed
    #[serde(rename = "unhealthy")]
    Unhealthy(UiText),

    /// Service is starting up
    #[serde(rename = "starting")]
    Starting,

    /// Service is shutting down
    #[serde(rename = "stopping")]
    Stopping,

    /// Service is intentionally not running (disabled by config or mode, e.g. wallet/RPC
    /// services while in Explore Mode). This is a normal state, never an error/issue.
    #[serde(rename = "disabled")]
    Disabled,
}

impl ServiceHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, ServiceHealth::Healthy)
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, ServiceHealth::Degraded(_))
    }

    pub fn is_unhealthy(&self) -> bool {
        matches!(self, ServiceHealth::Unhealthy(_))
    }

    pub fn is_disabled(&self) -> bool {
        matches!(self, ServiceHealth::Disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{format_en, ids, UiArg};

    fn arg(name: &'static str, value: &str) -> (&'static str, UiArg) {
        (name, UiArg::Text(value.to_owned()))
    }

    /// Every service health message with the English text the services reported
    /// before they moved to the catalog (the filtering snapshot sentence excepted).
    fn messages() -> Vec<(UiText, &'static str)> {
        let with = |id, args: Vec<(&'static str, UiArg)>| {
            args.into_iter()
                .fold(UiText::new(id), |text, (name, value)| text.arg(name, value))
        };
        vec![
            (
                with(
                    ids::SERVICES_HEALTH_COMPONENT_UNAVAILABLE,
                    vec![arg("component", "PoolDiscovery")],
                ),
                "PoolDiscovery component not available",
            ),
            (
                with(ids::SERVICES_HEALTH_UNAVAILABLE, vec![]),
                "Health status unavailable",
            ),
            (
                with(ids::SERVICES_HEALTH_POOLS_NOT_RUNNING, vec![]),
                "Pool service not running",
            ),
            (
                with(ids::SERVICES_HEALTH_EVENTS_DB_UNINITIALIZED, vec![]),
                "Events database not initialized",
            ),
            (
                with(ids::SERVICES_HEALTH_SOL_PRICE_NOT_RUNNING, vec![]),
                "SOL price service is not running",
            ),
            (
                with(
                    ids::SERVICES_HEALTH_SOL_PRICE_STALE,
                    vec![arg("seconds", "95")],
                ),
                "SOL price data is stale (95s old)",
            ),
            (
                with(ids::SERVICES_HEALTH_SOL_PRICE_NO_DATA, vec![]),
                "No SOL price data available yet",
            ),
            (
                with(ids::SERVICES_HEALTH_TELEGRAM_DISCOVERY, vec![]),
                "Discovery mode",
            ),
            (
                with(ids::SERVICES_HEALTH_TELEGRAM_DISCONNECTED, vec![]),
                "Disconnected",
            ),
            (
                with(ids::SERVICES_HEALTH_WALLET_WATCH_POLLING_ONLY, vec![]),
                "Detection running on polling alone",
            ),
            (
                with(ids::SERVICES_HEALTH_ASSISTANT_TASKS_DISABLED, vec![]),
                "Disabled in config",
            ),
            (
                with(
                    ids::SERVICES_HEALTH_CONNECTIVITY_CRITICAL_UNHEALTHY,
                    vec![arg("endpoints", "[\"internet\", \"rpc\"]")],
                ),
                "Critical endpoints unhealthy: [\"internet\", \"rpc\"]",
            ),
            (
                with(
                    ids::SERVICES_HEALTH_FILTERING_SNAPSHOT_STALE,
                    vec![arg("seconds", "412")],
                ),
                "Filtering snapshot is 412s old",
            ),
        ]
    }

    #[test]
    fn every_health_message_exists_and_keeps_its_wording() {
        for (text, expected) in messages() {
            assert_ne!(format_en(&text.id, None), text.id, "missing {}", text.id);
            assert_eq!(text.render_source_plain(), expected, "{}", text.id);
        }
    }

    #[test]
    fn catalog_holds_no_health_message_the_tests_do_not_cover() {
        let covered: Vec<String> = messages().iter().map(|(t, _)| t.id.to_string()).collect();
        let catalog = include_str!("../../locales/en/services.ftl");
        for line in catalog
            .lines()
            .filter(|l| l.starts_with("services-health-"))
        {
            let id = line.split('=').next().unwrap().trim();
            assert!(covered.iter().any(|c| c == id), "untested message {id}");
        }
    }

    #[test]
    fn health_message_serializes_as_ui_text() {
        let health = ServiceHealth::Degraded(UiText::new(ids::SERVICES_HEALTH_TELEGRAM_DISCOVERY));
        assert_eq!(
            serde_json::to_value(&health).unwrap(),
            serde_json::json!({
                "status": "degraded",
                "message": { "id": "services-health-telegram-discovery" }
            })
        );
    }
}
