/// Events Maintenance and Recording Functions
///
/// This module provides maintenance functions and specialized event recording
/// functions for each event category. All recording functions check the config
/// before recording to allow per-category enable/disable control.
use crate::config;
use crate::constants::SOL_MINT;
use crate::events::{Event, EventCategory, Severity};
use crate::logger::{self, LogTag};
use chrono::Utc;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use tokio::time::{interval, Duration};

// =============================================================================
// CONFIG HELPERS
// =============================================================================

/// Check if events system is globally enabled
#[inline]
fn is_events_enabled() -> bool {
    config::with_config(|c| c.events.enabled)
}

/// Check if a specific category is enabled for recording
#[inline]
fn is_category_enabled(category: &EventCategory) -> bool {
    if !is_events_enabled() {
        return false;
    }

    config::with_config(|c| match category {
        EventCategory::Swap => c.events.record_swap,
        EventCategory::Transaction => c.events.record_transaction,
        EventCategory::Pool => c.events.record_pool,
        EventCategory::Token => c.events.record_token,
        EventCategory::System => c.events.record_system,
        EventCategory::Position => c.events.record_position,
        EventCategory::Wallet => c.events.record_wallet,
        EventCategory::Trader => c.events.record_trader,
        EventCategory::Ohlcv => c.events.record_ohlcv,
        EventCategory::Rpc => c.events.record_rpc,
        EventCategory::Api => c.events.record_api,
        EventCategory::Security => c.events.record_security,
        EventCategory::Connectivity => c.events.record_connectivity,
        EventCategory::Filtering => c.events.record_filtering,
        EventCategory::ScheduledTask => c.events.record_system, // Use system flag for scheduled tasks
        EventCategory::Other(_) => true, // Always allow custom categories when enabled
    })
}

// =============================================================================
// MAINTENANCE FUNCTIONS
// =============================================================================

/// Start background maintenance task for events
/// Cleans up old events and performs database optimization
pub async fn start_maintenance_task() {
    // Only start maintenance if events are enabled
    if !is_events_enabled() {
        logger::info(
            LogTag::System,
            "Events system disabled - skipping maintenance task",
        );
        return;
    }

    let mut cleanup_interval = interval(Duration::from_secs(6 * 60 * 60)); // Every 6 hours

    tokio::spawn(async move {
        loop {
            cleanup_interval.tick().await;

            // Check if still enabled (config may have changed)
            if !is_events_enabled() {
                continue;
            }

            if let Err(e) = perform_maintenance().await {
                logger::info(LogTag::System, &format!("Events maintenance failed: {}", e));
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
        logger::info(
            LogTag::System,
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

    logger::info(
        LogTag::System,
        &format!(
            "Events DB: {} total, {} in 24h, {} MB",
            total_events, events_24h, db_size_mb
        ),
    );

    Ok(())
}

// =============================================================================
// TRANSACTION EVENTS
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
    if !is_category_enabled(&EventCategory::Transaction) {
        return;
    }

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

// =============================================================================
// SWAP EVENTS
// =============================================================================

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
    if !is_category_enabled(&EventCategory::Swap) {
        return;
    }

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
    let mint = if input_mint != SOL_MINT {
        Some(input_mint.to_string())
    } else {
        Some(output_mint.to_string())
    };

    let event = Event::new(
        EventCategory::Swap,
        Some("swap".to_string()),
        severity,
        mint,
        Some(signature.to_string()),
        payload,
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// POOL EVENTS
// =============================================================================

/// Record a pool discovery or analysis event
pub async fn record_pool_event(
    pool_address: &str,
    program_id: &str,
    pool_type: &str,
    token_mint: &str,
    action: &str,
    details: Value,
) {
    if !is_category_enabled(&EventCategory::Pool) {
        return;
    }

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

// =============================================================================
// POSITION EVENTS
// =============================================================================

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
    if !is_category_enabled(&EventCategory::Position) {
        return;
    }

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

/// Record a position event with flexible payload (for complex position operations)
pub async fn record_position_event_flexible(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Position) {
        return;
    }

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

    let event = Event::new(
        EventCategory::Position,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

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
        Some(format!("{}_{}", component, action)),
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

// =============================================================================
// OHLCV EVENTS
// =============================================================================

/// Record an OHLCV monitoring event with flexible payload metadata
pub async fn record_ohlcv_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Ohlcv) {
        return;
    }

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
// FILTERING EVENTS
// =============================================================================

/// Record a filtering event with flexible payload metadata
pub async fn record_filtering_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Filtering) {
        return;
    }

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
        .or_insert_with(|| Value::String(format!("Filtering event: {}", subtype)));

    let event = Event::new(
        EventCategory::Filtering,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// TRADER EVENTS
// =============================================================================

/// Record a trader event with flexible payload metadata
pub async fn record_trader_event(
    subtype: &str,
    severity: Severity,
    mint: Option<&str>,
    reference_id: Option<&str>,
    payload: Value,
) {
    if !is_category_enabled(&EventCategory::Trader) {
        return;
    }

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
        .or_insert_with(|| Value::String(format!("Trader event: {}", subtype)));

    let event = Event::new(
        EventCategory::Trader,
        Some(subtype.to_string()),
        severity,
        mint.map(|m| m.to_string()),
        reference_id.map(|r| r.to_string()),
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// RPC EVENTS (Solana RPC client)
// =============================================================================

/// Record an RPC client event (Solana RPC requests, responses, errors)
pub async fn record_rpc_event(method: &str, action: &str, severity: Severity, payload: Value) {
    if !is_category_enabled(&EventCategory::Rpc) {
        return;
    }

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
        .entry("method".to_string())
        .or_insert_with(|| Value::String(method.to_string()));
    payload_obj
        .entry("action".to_string())
        .or_insert_with(|| Value::String(action.to_string()));
    payload_obj
        .entry("event_time".to_string())
        .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));
    payload_obj
        .entry("message".to_string())
        .or_insert_with(|| Value::String(format!("RPC {} - {}", method, action)));

    let event = Event::new(
        EventCategory::Rpc,
        Some(action.to_string()),
        severity,
        None,
        None,
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// API EVENTS (External APIs - DexScreener, GeckoTerminal, Jupiter API, etc.)
// =============================================================================

/// Record an external API event (DexScreener, GeckoTerminal, Jupiter API, RugCheck, etc.)
pub async fn record_api_event(api_name: &str, action: &str, severity: Severity, payload: Value) {
    if !is_category_enabled(&EventCategory::Api) {
        return;
    }

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
        .entry("api".to_string())
        .or_insert_with(|| Value::String(api_name.to_string()));
    payload_obj
        .entry("action".to_string())
        .or_insert_with(|| Value::String(action.to_string()));
    payload_obj
        .entry("event_time".to_string())
        .or_insert_with(|| Value::String(Utc::now().to_rfc3339()));
    payload_obj
        .entry("message".to_string())
        .or_insert_with(|| Value::String(format!("{} - {}", api_name, action)));

    let event = Event::new(
        EventCategory::Api,
        Some(action.to_string()),
        severity,
        None,
        None,
        Value::Object(payload_obj),
    );

    crate::events::record_safe(event).await;
}

// =============================================================================
// SCHEDULED TASK EVENTS
// =============================================================================

/// Record a scheduled task event (execution, completion, failure)
pub fn record_event(category: EventCategory, title: &str, description: &str, severity: Severity) {
    let title = title.to_string();
    let description = description.to_string();

    tokio::spawn(async move {
        if !is_category_enabled(&category) {
            return;
        }

        let payload = json!({
            "title": title,
            "description": description,
            "event_time": Utc::now().to_rfc3339()
        });

        let event = Event::new(category, Some(title), severity, None, None, payload);

        crate::events::record_safe(event).await;
    });
}

/// Record a scheduled task event
pub fn record_scheduled_task_event(title: &str, description: &str, severity: Severity) {
    record_event(EventCategory::ScheduledTask, title, description, severity);
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
