// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Recorders for system, token, wallet, security and connectivity events.

use super::super::maintenance::is_category_enabled;
use crate::events::{Event, EventCategory, Severity};
use chrono::Utc;
use serde_json::{json, Value};

// =============================================================================
// SYSTEM EVENTS
// =============================================================================

/// Record a system lifecycle event
pub async fn record_system_event(
    component: &str,
    action: &str,
    severity: Severity,
    details: Option<Value>,
) {
    if !is_category_enabled(&EventCategory::System) {
        return;
    }

    let payload = json!({
        "component": component,
        "action": action,
        "details": details,
        "timestamp": Utc::now().to_rfc3339(),
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::System,
        Some(format!("{component}_{action}")),
        severity,
        None,
        None,
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// TOKEN EVENTS
// =============================================================================

/// Record a token-related event (blacklist, metadata update, etc.)
pub async fn record_token_event(mint: &str, action: &str, severity: Severity, details: Value) {
    if !is_category_enabled(&EventCategory::Token) {
        return;
    }

    let payload = json!({
        "action": action,
        "details": details,
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::Token,
        Some(action.to_string()),
        severity,
        Some(mint.to_string()),
        None,
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// WALLET EVENTS
// =============================================================================

/// Record a wallet event (balance changes, ATA management, etc.)
pub async fn record_wallet_event(
    action: &str,
    severity: Severity,
    mint: Option<&str>,
    details: Value,
) {
    if !is_category_enabled(&EventCategory::Wallet) {
        return;
    }

    let payload = json!({
        "action": action,
        "details": details,
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::Wallet,
        Some(action.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        None,
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// SECURITY EVENTS
// =============================================================================

/// Record a security analysis event
pub async fn record_security_event(
    mint: &str,
    analysis_type: &str,
    risk_level: &str,
    findings: Value,
) {
    if !is_category_enabled(&EventCategory::Security) {
        return;
    }

    let payload = json!({
        "analysis_type": analysis_type,
        "risk_level": risk_level,
        "findings": findings,
        "event_time": Utc::now().to_rfc3339()
    });

    let severity = match risk_level {
        "high" => Severity::Warn,
        "critical" => Severity::Error,
        _ => Severity::Info,
    };

    let event = Event::new(
        EventCategory::Security,
        Some(analysis_type.to_string()),
        severity,
        Some(mint.to_string()),
        None,
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// CONNECTIVITY EVENTS
// =============================================================================

/// Record a connectivity/endpoint health event
pub async fn record_connectivity_event(
    endpoint_name: &str,
    action: &str,
    severity: Severity,
    details: Value,
) {
    if !is_category_enabled(&EventCategory::Connectivity) {
        return;
    }

    let payload = json!({
        "endpoint": endpoint_name,
        "action": action,
        "details": details,
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::Connectivity,
        Some(action.to_string()),
        severity,
        None,
        Some(endpoint_name.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}
