/// Wallet monitoring and caching configuration
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Wallet monitoring and caching configuration
    pub struct WalletConfig {
        #[metadata(field_metadata! {
            label: "Snapshot Interval",
            hint: "Seconds between wallet balance snapshots",
            min: 15,
            max: 600,
            step: 5,
            unit: "seconds",
            impact: "medium",
            category: "Wallet",
        })]
        snapshot_interval_secs: u64 = 60,

        #[metadata(field_metadata! {
            label: "Flow Cache Update",
            hint: "Seconds between SOL flow cache syncs from transactions DB",
            min: 1,
            max: 60,
            step: 1,
            unit: "seconds",
            impact: "high",
            category: "Wallet",
        })]
        flow_cache_update_secs: u64 = 5,

        #[metadata(field_metadata! {
            label: "Flow Cache Batch Size",
            hint: "Max new transactions to process per sync",
            min: 100,
            max: 20000,
            step: 100,
            unit: "rows",
            impact: "medium",
            category: "Wallet",
        })]
        flow_cache_backfill_batch: usize = 2000,

        #[metadata(field_metadata! {
            label: "Flow Cache Lookback",
            hint: "Safety lookback when resuming sync (seconds)",
            min: 0,
            max: 86400,
            step: 60,
            unit: "seconds",
            impact: "medium",
            category: "Wallet",
        })]
        flow_cache_lookback_secs: u64 = 3600,

        #[metadata(field_metadata! {
            label: "Max Daily Flow Days",
            hint: "Maximum days of daily flow data to return (hard cap)",
            min: 30,
            max: 1825,
            step: 30,
            unit: "days",
            impact: "medium",
            category: "Wallet",
        })]
        max_daily_flow_days: usize = 730,

        #[metadata(field_metadata! {
            label: "Daily Flow Decimation Threshold",
            hint: "Days threshold beyond which older data is decimated",
            min: 30,
            max: 730,
            step: 30,
            unit: "days",
            impact: "low",
            category: "Wallet",
        })]
        daily_flow_decimate_threshold_days: usize = 365,

        #[metadata(field_metadata! {
            label: "Dashboard Metrics Update (24h)",
            hint: "Seconds between pre-computing 24h dashboard metrics",
            min: 30,
            max: 300,
            step: 10,
            unit: "seconds",
            impact: "high",
            category: "Wallet",
        })]
        dashboard_metrics_24h_interval_secs: u64 = 60,

        #[metadata(field_metadata! {
            label: "Dashboard Metrics Update (7d)",
            hint: "Seconds between pre-computing 7d dashboard metrics",
            min: 60,
            max: 600,
            step: 30,
            unit: "seconds",
            impact: "medium",
            category: "Wallet",
        })]
        dashboard_metrics_7d_interval_secs: u64 = 300,

        #[metadata(field_metadata! {
            label: "Dashboard Metrics Update (30d)",
            hint: "Seconds between pre-computing 30d dashboard metrics",
            min: 300,
            max: 1800,
            step: 60,
            unit: "seconds",
            impact: "medium",
            category: "Wallet",
        })]
        dashboard_metrics_30d_interval_secs: u64 = 900,

        #[metadata(field_metadata! {
            label: "Dashboard Metrics Update (All Time)",
            hint: "Seconds between pre-computing all-time dashboard metrics",
            min: 600,
            max: 3600,
            step: 60,
            unit: "seconds",
            impact: "low",
            category: "Wallet",
        })]
        dashboard_metrics_alltime_interval_secs: u64 = 1800,

        #[metadata(field_metadata! {
            label: "API Response Cache TTL",
            hint: "Seconds to cache wallet dashboard responses in memory",
            min: 10,
            max: 300,
            step: 10,
            unit: "seconds",
            impact: "low",
            category: "Wallet",
        })]
        api_response_cache_ttl_secs: u64 = 30,
    }
}
