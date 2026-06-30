//! OHLCV candlestick data fetching, caching, and gap detection configuration.

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
            label: "Enabled",
            hint: "Enable OHLCV candlestick data collection for technical analysis",
            impact: "high",
            category: "General",
        })]
        enabled: bool = true,
        /// Maximum number of tokens to monitor simultaneously
        #[metadata(field_metadata! {
            label: "Max Monitored Tokens",
            hint: "Maximum tokens to track OHLCV data for (higher uses more memory)",
            min: 10,
            max: 500,
            step: 10,
            unit: "tokens",
            impact: "medium",
            category: "General",
        })]
        max_monitored_tokens: usize = 100,
        /// Data retention period in days
        #[metadata(field_metadata! {
            label: "Retention Days",
            hint: "Days to retain historical OHLCV data",
            min: 1,
            max: 30,
            step: 1,
            unit: "days",
            impact: "critical",
            category: "Retention",
        })]
        retention_days: i64 = 7,
        /// Maximum consecutive empty fetches before throttling
        #[metadata(field_metadata! {
            label: "Empty Response Threshold",
            hint: "Consecutive empty API responses before throttling requests",
            min: 1,
            max: 50,
            step: 1,
            impact: "low",
            category: "General",
        })]
        max_empty_fetches: u32 = 10,
        /// Enable automatic gap filling
        #[metadata(field_metadata! {
            label: "Auto Fetch Gaps",
            hint: "Automatically fetch missing candles when gaps are detected",
            impact: "medium",
            category: "General",
        })]
        auto_fill_gaps: bool = true,
        /// Cache size (maximum number of tokens in hot cache)
        #[metadata(field_metadata! {
            label: "Cache Max Tokens",
            hint: "Maximum tokens to keep in hot memory cache",
            min: 10,
            max: 500,
            step: 10,
            unit: "tokens",
            impact: "medium",
            category: "Cache",
        })]
        cache_size: usize = 100,
        /// Cache retention hours (for hot cache)
        #[metadata(field_metadata! {
            label: "Cache Retention",
            hint: "Hours to keep tokens in hot cache",
            min: 1,
            max: 168,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Cache",
        })]
        cache_retention_hours: i64 = 24,

        /// Enable pool failover
        #[metadata(field_metadata! {
            label: "Enable Fallback",
            hint: "Switch to alternative data source when primary fails",
            impact: "medium",
            category: "Fallback",
        })]
        pool_failover_enabled: bool = true,
        /// Maximum pool failures before switching
        #[metadata(field_metadata! {
            label: "Fallback Threshold",
            hint: "Consecutive failures before switching to backup source",
            min: 1,
            max: 20,
            step: 1,
            impact: "low",
            category: "Fallback",
        })]
        max_pool_failures: u32 = 5,

        /// OHLCV data source configuration (independent of token sources/discovery)
        #[metadata(field_metadata! {
            label: "Data Sources",
            hint: "API sources used by the OHLCV fetcher (independent of token discovery)",
            impact: "high",
            category: "Sources",
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
            label: "Enabled",
            hint: "Enable GeckoTerminal as an OHLCV data source",
            impact: "high",
            category: "Sources",
        })]
        enabled: bool = true,
        /// GeckoTerminal API base URL
        #[metadata(field_metadata! {
            label: "Endpoint",
            hint: "GeckoTerminal API base URL",
            impact: "critical",
            category: "Sources",
        })]
        endpoint: String = "https://api.geckoterminal.com/api/v2".to_owned(),
        /// Maximum API requests per minute to this source
        #[metadata(field_metadata! {
            label: "Rate Limit (req/min)",
            hint: "Maximum API requests per minute (GeckoTerminal enforces strict limits)",
            impact: "medium",
            category: "Sources",
            min: 1.0,
            max: 300.0,
            step: 1.0,
        })]
        rate_limit_per_minute: u32 = 30,
        /// HTTP request timeout in seconds
        #[metadata(field_metadata! {
            label: "Timeout (seconds)",
            hint: "HTTP request timeout for GeckoTerminal calls",
            impact: "low",
            category: "Sources",
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
            label: "Enabled",
            hint: "Enable SolanaTracker as an OHLCV fallback source (credit-based, requires API key)",
            impact: "high",
            category: "Sources",
        })]
        enabled: bool = false,
        /// SolanaTracker API base URL
        #[metadata(field_metadata! {
            label: "Endpoint",
            hint: "SolanaTracker API base URL",
            impact: "critical",
            category: "Sources",
        })]
        endpoint: String = "https://data.solanatracker.io".to_owned(),
        /// SolanaTracker API key (required when enabled = true)
        #[metadata(field_metadata! {
            label: "API Key",
            hint: "SolanaTracker API key from solanatracker.io",
            impact: "critical",
            category: "Sources",
        })]
        api_key: String = String::new(),
        /// Maximum API requests per minute
        #[metadata(field_metadata! {
            label: "Rate Limit (req/min)",
            hint: "Maximum API requests per minute (credit-based, be conservative)",
            impact: "medium",
            category: "Sources",
            min: 1.0,
            max: 120.0,
            step: 1.0,
        })]
        rate_limit_per_minute: u32 = 30,
        /// HTTP request timeout in seconds
        #[metadata(field_metadata! {
            label: "Timeout (seconds)",
            hint: "HTTP request timeout for SolanaTracker calls",
            impact: "low",
            category: "Sources",
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
            label: "GeckoTerminal Source",
            hint: "GeckoTerminal endpoint used exclusively by the OHLCV fetcher (independent of token discovery)",
            impact: "high",
            category: "Sources",
        })]
        geckoterminal: OhlcvGeckoConfig = OhlcvGeckoConfig::default(),
        #[metadata(field_metadata! {
            label: "SolanaTracker Source",
            hint: "SolanaTracker fallback endpoint used exclusively by the OHLCV fetcher (when enabled + API key set)",
            impact: "medium",
            category: "Sources",
        })]
        solana_tracker: OhlcvSolanaTrackerConfig = OhlcvSolanaTrackerConfig::default(),
        #[metadata(field_metadata! {
            label: "ScreenerBot Server Source",
            hint: "Self-hosted ScreenerBot OHLCV cache — tried FIRST (fast, shared cache); falls back to the providers below on a miss",
            impact: "high",
            category: "Sources",
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
            label: "Enabled",
            hint: "Try the self-hosted ScreenerBot OHLCV server before external providers",
            impact: "high",
            category: "Sources",
        })]
        enabled: bool = true,
        /// ScreenerBot OHLCV server base URL (no trailing slash)
        #[metadata(field_metadata! {
            label: "Endpoint",
            hint: "Base URL of the self-hosted OHLCV server",
            impact: "critical",
            category: "Sources",
        })]
        endpoint: String = "https://screenerbot.io/ohlcv".to_owned(),
        /// HTTP request timeout in seconds (keep short so a miss falls back fast)
        #[metadata(field_metadata! {
            label: "Timeout (seconds)",
            hint: "HTTP request timeout for the ScreenerBot server (short so misses fall back quickly)",
            impact: "low",
            category: "Sources",
            min: 1.0,
            max: 30.0,
            step: 1.0,
        })]
        timeout_seconds: u64 = 4,
    }
}
