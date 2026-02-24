//! Database maintenance, cleanup, and retention policy configuration.

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// MAINTENANCE CONFIGURATION
// ============================================================================

config_struct! {
    /// Automatic maintenance and data retention configuration.
    ///
    /// Controls how long historical data is kept and when heavy
    /// maintenance operations (VACUUM, WAL checkpoint) run.
    pub struct MaintenanceConfig {
        #[metadata(field_metadata! {
            label: "Events Retention",
            hint: "How many days to keep event history (0 = keep forever)",
            impact: "medium",
            category: "Retention",
            min: 0.0,
            step: 1.0,
            unit: "days",
        })]
        events_retention_days: u32 = 30,

        #[metadata(field_metadata! {
            label: "Actions Retention",
            hint: "How many days to keep action history (0 = keep forever)",
            impact: "medium",
            category: "Retention",
            min: 0.0,
            step: 1.0,
            unit: "days",
        })]
        actions_retention_days: u32 = 30,

        #[metadata(field_metadata! {
            label: "RPC Stats Retention",
            hint: "How many hours to keep RPC statistics (0 = keep forever)",
            impact: "low",
            category: "Retention",
            min: 0.0,
            step: 1.0,
            unit: "hours",
        })]
        rpc_stats_retention_hours: u64 = 72,

        #[metadata(field_metadata! {
            label: "OHLCV Retention",
            hint: "How many days to keep OHLCV candle data (0 = keep forever)",
            impact: "medium",
            category: "Retention",
            min: 0.0,
            step: 1.0,
            unit: "days",
        })]
        ohlcv_retention_days: u32 = 90,

        #[metadata(field_metadata! {
            label: "Stale Token Cutoff",
            hint: "Tokens without market data updates older than this are excluded from filtering to save memory. 0 = include all. At 7 days, typically reduces token count by ~90%.",
            impact: "high",
            category: "Optimization",
            min: 0.0,
            step: 1.0,
            unit: "days",
        })]
        stale_token_days: u32 = 7,

        #[metadata(field_metadata! {
            label: "WAL Checkpoint Interval",
            hint: "How often to run SQLite WAL checkpoint (consolidates write-ahead log)",
            impact: "low",
            category: "Database",
            min: 300.0,
            step: 300.0,
            unit: "seconds",
        })]
        wal_checkpoint_interval_secs: u64 = 3600,

        #[metadata(field_metadata! {
            label: "VACUUM Interval",
            hint: "How often to run SQLite VACUUM (reclaims disk space, heavy operation)",
            impact: "low",
            category: "Database",
            min: 3600.0,
            step: 3600.0,
            unit: "seconds",
        })]
        vacuum_interval_secs: u64 = 86400,

        #[metadata(field_metadata! {
            label: "Maintenance Window Start",
            hint: "Start time for heavy maintenance (HH:MM local time). Empty = anytime.",
            impact: "low",
            category: "Scheduling",
        })]
        maintenance_window_start: String = String::new(),

        #[metadata(field_metadata! {
            label: "Skip During Active Trades",
            hint: "Postpone heavy maintenance (VACUUM) while positions are open",
            impact: "medium",
            category: "Scheduling",
        })]
        skip_during_active_trades: bool = true,
    }
}
