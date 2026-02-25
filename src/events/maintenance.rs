//! Events maintenance, config helpers, and MCP integration.
//!
//! Maintenance tasks and configuration checks for the events system.
//! Event recording functions live in the sibling `recorders` module.
use crate::config;
use crate::events::{Event, EventCategory, Severity};
use crate::logger::{self, LogTag};
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{interval, Duration};

// =============================================================================
// CONFIG HELPERS
// =============================================================================

/// Check if events system is globally enabled
#[inline]
pub(crate) fn is_events_enabled() -> bool {
    config::with_config(|c| c.events.enabled)
}

/// Check if a specific category is enabled for recording
#[inline]
pub(crate) fn is_category_enabled(category: &EventCategory) -> bool {
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
                logger::info(LogTag::System, &format!("Events maintenance failed: {e}"));
            }
        }
    });
}

/// Perform maintenance operations on events database
async fn perform_maintenance() -> Result<(), String> {
    let db = crate::events::EVENTS_DB
        .get()
        .ok_or_else(|| "Events system not initialized".to_owned())?
        .clone();

    // Cleanup old events
    let deleted_count = db.cleanup_old_events().await?;
    if deleted_count > 0 {
        logger::info(
            LogTag::System,
            &format!("Cleaned up {deleted_count} old events"),
        );
    }

    // Get database stats for monitoring
    let stats = db.get_stats().await?;
    let total_events = stats.get("total_events").unwrap_or(&0);
    let events_24h = stats.get("events_24h").unwrap_or(&0);
    let db_size_mb = stats
        .get("db_size_bytes")
        .map(|s| s / 1024 / 1024)
        .unwrap_or_default();

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
// MCP INTEGRATION HELPERS
// =============================================================================

/// Get events summary for MCP tools
pub async fn get_events_summary(hours: u64) -> Result<HashMap<String, serde_json::Value>, String> {
    let db = crate::events::EVENTS_DB
        .get()
        .ok_or_else(|| "Events system not initialized".to_owned())?
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
    summary.insert("counts_by_category".to_owned(), json!(counts));
    summary.insert("database_stats".to_owned(), json!(stats));
    summary.insert("recent_errors".to_owned(), json!(recent_errors));
    summary.insert("time_range_hours".to_owned(), json!(hours));

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
        .ok_or_else(|| "Events system not initialized".to_owned())?
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
