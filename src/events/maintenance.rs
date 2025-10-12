use crate::events::db::EventsDatabase;
/// Events Maintenance and MCP Integration
///
/// This module provides maintenance functions and MCP server integration
/// for the events system.
use crate::events::{Event, EventCategory, Severity};
use crate::logger::{log, LogTag};
use chrono::Utc;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use tokio::time::{interval, Duration};

// =============================================================================
// MAINTENANCE FUNCTIONS
// =============================================================================

/// Start background maintenance task for events
/// Cleans up old events and performs database optimization
pub async fn start_maintenance_task() {
    let mut cleanup_interval = interval(Duration::from_secs(6 * 60 * 60)); // Every 6 hours

    tokio::spawn(async move {
        loop {
            cleanup_interval.tick().await;

            if let Err(e) = perform_maintenance().await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Events maintenance failed: {}", e),
                );
            }
        }
    });
}

/// Perform maintenance operations on events database
async fn perform_maintenance() -> Result<(), String> {
    let db = crate::events::EVENTS_DB
        .get()
        .ok_or_else(|| "Events system not initialized".to_string())?
        .clone();

    // Cleanup old events
    let deleted_count = db.cleanup_old_events().await?;
    if deleted_count > 0 {
        log(
            LogTag::System,
            "MAINT",
            &format!("Cleaned up {} old events", deleted_count),
        );
    }

    // Get database stats for monitoring
    let stats = db.get_stats().await?;
    let total_events = stats.get("total_events").unwrap_or(&0);
    let events_24h = stats.get("events_24h").unwrap_or(&0);
    let db_size_mb = stats
        .get("db_size_bytes")
        .map(|s| s / 1024 / 1024)
        .unwrap_or(0);

    log(
        LogTag::System,
        "STATS",
        &format!(
            "Events DB: {} total, {} in 24h, {} MB",
            total_events, events_24h, db_size_mb
        ),
    );

    Ok(())
}

// =============================================================================
// HELPER FUNCTIONS FOR COMMON EVENT RECORDING
// =============================================================================

/// Record a transaction event (submission/confirmation/failure)
pub async fn record_transaction_event(
    signature: &str,
    confirmation_status: &str,
    success: bool,
    fee: Option<u64>,
    slot: Option<u64>,
    error_message: Option<&str>,
) {
    let payload = json!({
        "signature": signature,
        "confirmation_status": confirmation_status,
        "fee": fee,
        "slot": slot,
        "success": success,
        "error_message": error_message,
        "event_time": Utc::now().to_rfc3339()
    });

    let severity = if success {
        Severity::Info
    } else {
        Severity::Error
    };

    let event = Event::new(
        EventCategory::Transaction,
        Some(confirmation_status.to_string()),
        severity,
        None,
        Some(signature.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

/// Record a swap event with standardized payload
pub async fn record_swap_event(
    signature: &str,
    input_mint: &str,
    output_mint: &str,
    amount_in: u64,
    amount_out: u64,
    success: bool,
    error_message: Option<&str>,
) {
    let payload = json!({
        "signature": signature,
        "input_mint": input_mint,
        "output_mint": output_mint,
        "amount_in": amount_in,
        "amount_out": amount_out,
        "success": success,
        "error_message": error_message,
        "event_time": Utc::now().to_rfc3339()
    });

    let severity = if success {
        Severity::Info
    } else {
        Severity::Error
    };
    let mint = if input_mint != "So11111111111111111111111111111111111111112" {
        Some(input_mint.to_string())
    } else {
        Some(output_mint.to_string())
    };

    let event = Event::new(
        EventCategory::Swap,
        Some("JupiterSwap".to_string()),
        severity,
        mint,
        Some(signature.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

/// Record a pool discovery or analysis event
pub async fn record_pool_event(
    pool_address: &str,
    program_id: &str,
    pool_type: &str,
    token_mint: &str,
    action: &str,
    details: Value,
) {
    let payload = json!({
        "pool_address": pool_address,
        "program_id": program_id,
        "pool_type": pool_type,
        "action": action,
        "details": details,
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::Pool,
        Some(format!("{}_{}", pool_type, action)),
        Severity::Info,
        Some(token_mint.to_string()),
        Some(pool_address.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

/// Record a position lifecycle event
pub async fn record_position_event(
    position_id: &str,
    mint: &str,
    action: &str,
    entry_signature: Option<&str>,
    exit_signature: Option<&str>,
    amount_sol: f64,
    amount_tokens: u64,
    pnl_sol: Option<f64>,
    pnl_percent: Option<f64>,
) {
    let payload = json!({
        "position_id": position_id,
        "action": action,
        "entry_signature": entry_signature,
        "exit_signature": exit_signature,
        "amount_sol": amount_sol,
        "amount_tokens": amount_tokens,
        "pnl_sol": pnl_sol,
        "pnl_percent": pnl_percent,
        "event_time": Utc::now().to_rfc3339()
    });

    let reference_id = entry_signature.or(exit_signature).map(|s| s.to_string());

    let event = Event::new(
        EventCategory::Position,
        Some(action.to_string()),
        Severity::Info,
        Some(mint.to_string()),
        reference_id,
        payload,
    );

    crate::events::record_safe(event).await;
}

/// Record an entry signal event
pub async fn record_entry_event(
    mint: &str,
    signal_type: &str,
    decision: &str,
    price_sol: f64,
    timeframe: &str,
    strength: f64,
    reason: Option<&str>,
) {
    let payload = json!({
        "signal_type": signal_type,
        "decision": decision,
        "price_sol": price_sol,
        "timeframe": timeframe,
        "strength": strength,
        "reason": reason,
        "event_time": Utc::now().to_rfc3339()
    });

    let severity = match decision {
        "buy" => Severity::Info,
        "skip" => Severity::Debug,
        _ => Severity::Info,
    };

    let event = Event::new(
        EventCategory::Entry,
        Some(signal_type.to_string()),
        severity,
        Some(mint.to_string()),
        None,
        payload,
    );

    crate::events::record_safe(event).await;
}

/// Record a system lifecycle event
pub async fn record_system_event(
    component: &str,
    action: &str,
    severity: Severity,
    details: Option<Value>,
) {
    let payload = json!({
        "component": component,
        "action": action,
        "details": details,
        "timestamp": Utc::now().to_rfc3339(),
        "event_time": Utc::now().to_rfc3339()
    });

    let event = Event::new(
        EventCategory::System,
        Some(format!("{}_{}", component, action)),
        severity,
        None,
        None,
        payload,
    );

    crate::events::record_safe(event).await;
}

/// Record a token-related event (blacklist, metadata update, etc.)
pub async fn record_token_event(mint: &str, action: &str, severity: Severity, details: Value) {
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

/// Record a security analysis event
pub async fn record_security_event(
    mint: &str,
    analysis_type: &str,
    risk_level: &str,
    findings: Value,
) {
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

/// Record an OHLCV monitoring event with flexible payload metadata
pub async fn record_ohlcv_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    let mut payload_obj: Map<String, Value> = match payload {
        Value::Object(obj) => obj,
        Value::Null => Map::new(),
        other => {
            let mut map = Map::new();
            map.insert("details".to_string(), other);
            map
        }
    };

    payload_obj
        .entry("subtype".to_string())
        .or_insert_with(|| Value::String(subtype.to_string()));
    payload_obj
        .entry("event_time".to_string())
        .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));
    payload_obj
        .entry("message".to_string())
        .or_insert_with(|| Value::String(format!("OHLCV event: {}", subtype)));

    let event = Event::new(
        EventCategory::Ohlcv,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// MCP INTEGRATION HELPERS
// =============================================================================

/// Get events summary for MCP tools
pub async fn get_events_summary(hours: u64) -> Result<HashMap<String, serde_json::Value>, String> {
    let db = crate::events::EVENTS_DB
        .get()
        .ok_or_else(|| "Events system not initialized".to_string())?
        .clone();

    // Get counts by category
    let counts = db.get_event_counts_by_category(hours).await?;

    // Get database stats
    let stats = db.get_stats().await?;

    // Get recent errors
    let recent_errors = db
        .get_recent_events(None, 50)
        .await?
        .into_iter()
        .filter(|e| matches!(e.severity, Severity::Error))
        .take(10)
        .map(|e| {
            json!({
                "category": e.category.to_string(),
                "subtype": e.subtype,
                "mint": e.mint,
                "event_time": e.event_time.to_rfc3339(),
                "payload": e.payload
            })
        })
        .collect::<Vec<_>>();

    let mut summary = HashMap::new();
    summary.insert("counts_by_category".to_string(), json!(counts));
    summary.insert("database_stats".to_string(), json!(stats));
    summary.insert("recent_errors".to_string(), json!(recent_errors));
    summary.insert("time_range_hours".to_string(), json!(hours));

    Ok(summary)
}

/// Search events by multiple criteria (for MCP tools)
pub async fn search_events(
    category: Option<&str>,
    mint: Option<&str>,
    reference_id: Option<&str>,
    since_hours: Option<u64>,
    limit: usize,
) -> Result<Vec<Event>, String> {
    let db = crate::events::EVENTS_DB
        .get()
        .ok_or_else(|| "Events system not initialized".to_string())?
        .clone();

    if let Some(ref_id) = reference_id {
        return db.get_events_by_reference(ref_id, limit).await;
    }

    if let Some(mint_addr) = mint {
        return db.get_events_by_mint(mint_addr, limit).await;
    }

    let category_enum = category.map(EventCategory::from_string);
    db.get_recent_events(category_enum, limit).await
}
