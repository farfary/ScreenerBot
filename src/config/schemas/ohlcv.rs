//! OHLCV candlestick data fetching, caching, and gap detection configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// OHLCV DATA MONITORING
// ============================================================================

config_struct! {
    /// OHLCV data monitoring configuration
    pub struct OhlcvConfig {
        /// Enable OHLCV data collection
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::General,
        })]
        enabled: bool = true,
        /// Maximum number of tokens to monitor simultaneously
        #[metadata(field_metadata! {
            min: 10,
            max: 2000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::General,
        })]
        max_monitored_tokens: usize = 300,
        /// Data retention period in days
        #[metadata(field_metadata! {
            min: 1,
            max: 30,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Retention,
        })]
        retention_days: i64 = 7,
        /// Maximum consecutive empty fetches before throttling
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::General,
        })]
        max_empty_fetches: u32 = 10,
        /// Enable automatic gap filling
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::General,
        })]
        auto_fill_gaps: bool = true,
        /// Cache size (maximum number of tokens in hot cache)
        #[metadata(field_metadata! {
            min: 10,
            max: 500,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Cache,
        })]
        cache_size: usize = 100,
        /// Cache retention hours (for hot cache)
        #[metadata(field_metadata! {
            min: 1,
            max: 168,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Cache,
        })]
        cache_retention_hours: i64 = 24,

        /// Enable pool failover
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fallback,
        })]
        pool_failover_enabled: bool = true,
        /// Maximum pool failures before switching
        #[metadata(field_metadata! {
            min: 1,
            max: 20,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Fallback,
        })]
        max_pool_failures: u32 = 5,

        /// OHLCV data source configuration (independent of token sources/discovery)
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        sources: OhlcvSourcesConfig = OhlcvSourcesConfig::default(),
    }
}

// ----------------------------------------------------------------------------
// OHLCV DATA SOURCES CONFIGURATION (independent of token sources/discovery)
// ----------------------------------------------------------------------------
//
// The OHLCV module fetches candlestick data from external APIs. Historically
// it shared its GeckoTerminal client with token discovery, so turning off
// `[tokens.discovery.geckoterminal].enabled` also disabled OHLCV fetches
// (264+ errors/min in the latest log). These structs give OHLCV its own
// configuration block so it can stay on while discovery is off, and vice
// versa. Endpoint URLs are now config-driven (no hardcoded base URLs in
// code).

config_struct! {
    /// GeckoTerminal API configuration for the OHLCV fetcher.
    pub struct OhlcvGeckoConfig {
        /// Whether OHLCV fetches should use this source
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = true,
        /// GeckoTerminal API base URL
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Sources,
        })]
        endpoint: String = "https://api.geckoterminal.com/api/v2".to_owned(),
        /// Maximum API requests per minute to this source
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 300.0,
            step: 1.0,
        })]
        rate_limit_per_minute: u32 = 30,
        /// HTTP request timeout in seconds
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 60.0,
            step: 1.0,
        })]
        timeout_seconds: u64 = 10,
    }
}

config_struct! {
    /// SolanaTracker API configuration for the OHLCV fetcher (credit-based billing).
    pub struct OhlcvSolanaTrackerConfig {
        /// Whether OHLCV fetches should use this source
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = false,
        /// SolanaTracker API base URL
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Sources,
        })]
        endpoint: String = "https://data.solanatracker.io".to_owned(),
        /// SolanaTracker API key (required when enabled = true)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Sources,
        })]
        api_key: String = String::new(),
        /// Maximum API requests per minute
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 120.0,
            step: 1.0,
        })]
        rate_limit_per_minute: u32 = 30,
        /// HTTP request timeout in seconds
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 60.0,
            step: 1.0,
        })]
        timeout_seconds: u64 = 15,
    }
}

config_struct! {
    /// All OHLCV data sources — endpoint URLs and enablement per provider.
    pub struct OhlcvSourcesConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        geckoterminal: OhlcvGeckoConfig = OhlcvGeckoConfig::default(),
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Sources,
        })]
        solana_tracker: OhlcvSolanaTrackerConfig = OhlcvSolanaTrackerConfig::default(),
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        screenerbot_server: OhlcvScreenerbotConfig = OhlcvScreenerbotConfig::default(),
    }
}

config_struct! {
    /// Self-hosted ScreenerBot OHLCV server — the preferred first-hop source. It
    /// serves cached candles fast and warms itself; on a miss the fetcher falls
    /// back to GeckoTerminal/SolanaTracker as before.
    pub struct OhlcvScreenerbotConfig {
        /// Whether to try the ScreenerBot server first
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = true,
        /// ScreenerBot OHLCV server base URL (no trailing slash)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Sources,
        })]
        endpoint: String = "https://screenerbot.io/data".to_owned(),
        /// HTTP request timeout in seconds (keep short so a miss falls back fast)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 30.0,
            step: 1.0,
        })]
        timeout_seconds: u64 = 4,
    }
}
