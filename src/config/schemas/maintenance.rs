use crate::config_struct;

// ============================================================================
// MAINTENANCE CONFIGURATION
// ============================================================================

config_struct! {
    /// Automatic maintenance and data retention configuration.
    ///
    /// Controls how long historical data is kept and when heavy
    /// maintenance operations (VACUUM, WAL checkpoint) run.
    pub struct MaintenanceConfig {
        /// Events retention in days (0 = keep forever).
        events_retention_days: u32 = 30,

        /// Actions retention in days (0 = keep forever).
        actions_retention_days: u32 = 30,

        /// RPC stats retention in hours (0 = keep forever).
        rpc_stats_retention_hours: u64 = 72,

        /// OHLCV candle retention in days (0 = keep forever).
        ohlcv_retention_days: u32 = 90,

        /// Tokens without market data updates older than this are excluded
        /// from the filter engine to save memory. 0 = include all tokens.
        /// At 7 days, this typically reduces token count by ~90%.
        stale_token_days: u32 = 7,

        /// WAL checkpoint interval in seconds.
        wal_checkpoint_interval_secs: u64 = 3600,

        /// VACUUM interval in seconds (heavy operation).
        vacuum_interval_secs: u64 = 86400,

        /// Maintenance window start time (HH:MM, local time).
        /// Empty string means anytime.
        maintenance_window_start: String = String::new(),

        /// Skip heavy maintenance during active trades.
        skip_during_active_trades: bool = true,
    }
}
