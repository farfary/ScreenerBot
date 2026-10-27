// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard label maps for service ids and health tags must resolve in the `en` catalog.

use super::health::ServiceHealth;
use crate::i18n::{format_en, ids, UiText};

/// Catalog key of each health status tag, mirrored by `HEALTH_STATUS_LABELS`
/// (pages/services.js). The match is exhaustive, so a new variant fails to
/// compile until it is mapped here.
fn status_key(health: &ServiceHealth) -> &'static str {
    match health {
        ServiceHealth::Healthy => "services-status-healthy",
        ServiceHealth::Degraded(_) => "services-status-degraded",
        ServiceHealth::Unhealthy(_) => "services-status-unhealthy",
        ServiceHealth::Starting => "services-status-starting",
        ServiceHealth::Stopping => "services-status-stopping",
        ServiceHealth::Disabled => "services-status-disabled",
    }
}

#[test]
fn health_status_labels_exist_in_the_catalog() {
    let text = || UiText::new(ids::SERVICES_HEALTH_UNAVAILABLE);
    for health in [
        ServiceHealth::Healthy,
        ServiceHealth::Degraded(text()),
        ServiceHealth::Unhealthy(text()),
        ServiceHealth::Starting,
        ServiceHealth::Stopping,
        ServiceHealth::Disabled,
    ] {
        let key = status_key(&health);
        assert_ne!(format_en(key, None), key, "missing {key}");
    }
}

/// Every id returned by a `Service::name()` implementation has a `services-name-*`
/// message, the source of `SERVICE_NAME_LABELS` (pages/services.js).
#[test]
fn service_names_exist_in_the_catalog() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/services/implementations");
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir).expect("implementations directory") {
        let source = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        let mut rest = source.as_str();
        while let Some(at) = rest.find("fn name(&self) -> &'static str") {
            rest = &rest[at..];
            let body = &rest[rest.find('{').expect("name body") + 1..];
            let body = body[..body.find('}').expect("end of name body")].trim();
            names.push(match body.strip_prefix('"') {
                Some(literal) => literal.trim_end_matches('"').to_owned(),
                // A service that names itself through a constant: read that constant's literal.
                None => {
                    let declaration = format!("const {body}: &str = \"");
                    let start = source.find(&declaration).expect("service name constant")
                        + declaration.len();
                    let end = source[start..].find('"').expect("closing quote") + start;
                    source[start..end].to_owned()
                }
            });
            rest = &rest[1..];
        }
    }
    assert!(
        names.len() >= 20,
        "expected the service names, found {names:?}"
    );
    for name in names {
        let key = format!("services-name-{}", name.replace('_', "-"));
        assert_ne!(format_en(&key, None), key, "missing {key}");
    }
}
