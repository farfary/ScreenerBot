use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// TOKENS CONFIGURATION
// ============================================================================

config_struct! {
    /// Token management configuration
    pub struct TokensConfig {
        // Market data source selection
        #[metadata(field_metadata! {
            label: "Preferred Market Data Source",
            hint: "Choose DexScreener or GeckoTerminal for price/volume/market data. Rugcheck always fetched for security.",
            impact: "critical",
            category: "Data Sources",
        })]
        preferred_market_data_source: String = "dexscreener".to_string(), // "dexscreener" or "geckoterminal"

        // API rate limits
        #[metadata(field_metadata! {
            label: "DexScreener Rate Limit",
            hint: "API calls per minute",
            min: 10,
            max: 300,
            step: 10,
            unit: "calls/min",
            impact: "medium",
            category: "API Limits",
        })]
        dexscreener_rate_limit_per_minute: usize = 100,
        #[metadata(field_metadata! {
            label: "DexScreener Discovery Limit",
            hint: "Discovery API calls per minute",
            min: 10,
            max: 300,
            step: 10,
            unit: "calls/min",
            impact: "medium",
            category: "API Limits",
        })]
        dexscreener_discovery_rate_limit: usize = 60,
        #[metadata(field_metadata! {
            label: "Max Tokens Per Call",
            hint: "Tokens per API request",
            min: 10,
            max: 100,
            step: 10,
            unit: "tokens",
            impact: "low",
            category: "API Limits",
        })]
        max_tokens_per_api_call: usize = 30,
        #[metadata(field_metadata! {
            label: "Raydium Rate Limit",
            hint: "Raydium API calls per minute",
            min: 10,
            max: 300,
            step: 10,
            unit: "calls/min",
            impact: "medium",
            category: "API Limits",
        })]
        raydium_rate_limit_per_minute: usize = 120,
        #[metadata(field_metadata! {
            label: "Max Tokens Per Batch",
            hint: "Tokens per batch operation",
            min: 10,
            max: 100,
            step: 10,
            unit: "tokens",
            impact: "low",
            category: "API Limits",
        })]
        max_tokens_per_batch: usize = 30,

        // Decimals
        #[metadata(field_metadata! {
            label: "Max Accounts Per RPC Call",
            hint: "Accounts per get_multiple_accounts (max 100)",
            min: 10,
            max: 100,
            step: 10,
            unit: "accounts",
            impact: "medium",
            category: "RPC",
        })]
        max_accounts_per_call: usize = 100,
        #[metadata(field_metadata! {
            label: "Max Decimal Retry",
            hint: "Retries for fetching token decimals",
            min: 1,
            max: 10,
            step: 1,
            unit: "attempts",
            impact: "low",
            category: "RPC",
        })]
        max_decimal_retry_attempts: i32 = 3,

        // Blacklist
        #[metadata(field_metadata! {
            label: "Low Liquidity Threshold",
            hint: "USD threshold for low liquidity blacklist",
            min: 10,
            max: 10000,
            step: 10,
            unit: "USD",
            impact: "high",
            category: "Blacklist",
        })]
        low_liquidity_threshold: f64 = 100.0,
        #[metadata(field_metadata! {
            label: "Min Age for Blacklist",
            hint: "Hours before token can be blacklisted",
            min: 0,
            max: 168,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Blacklist",
        })]
        min_age_hours: i64 = 2,
        #[metadata(field_metadata! {
            label: "Max Low Liq Count",
            hint: "Times seen with low liquidity before blacklist",
            min: 1,
            max: 20,
            step: 1,
            unit: "times",
            impact: "medium",
            category: "Blacklist",
        })]
        max_low_liquidity_count: u32 = 5,
        #[metadata(field_metadata! {
            label: "Max No Route Failures",
            hint: "Route failures before blacklist",
            min: 1,
            max: 20,
            step: 1,
            unit: "failures",
            impact: "medium",
            category: "Blacklist",
        })]
        max_no_route_failures: u32 = 5,

        // Token monitor
        #[metadata(field_metadata! {
            label: "Max Update Interval",
            hint: "Hours between token updates",
            min: 1,
            max: 24,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Monitoring",
        })]
        max_update_interval_hours: i64 = 2,
        #[metadata(field_metadata! {
            label: "New Token Boost Age",
            hint: "Minutes to boost new tokens",
            min: 10,
            max: 240,
            step: 10,
            unit: "minutes",
            impact: "critical",
            category: "Monitoring",
        })]
        new_token_boost_max_age_minutes: i64 = 60,

        // Patterns
        #[metadata(field_metadata! {
            label: "Max Pattern Length",
            hint: "Max length for pattern detection",
            min: 3,
            max: 20,
            step: 1,
            unit: "chars",
            impact: "low",
            category: "Patterns",
        })]
        max_pattern_length: usize = 8,

        // Multi-source validation configuration
        #[metadata(field_metadata! {
            label: "Token Sources",
            hint: "Multi-source validation and per-source toggles",
            impact: "critical",
            category: "Sources",
        })]
        sources: TokenSourcesConfig = TokenSourcesConfig::default(),

        #[metadata(field_metadata! {
            label: "Token Discovery",
            hint: "Configure discovery endpoints per provider",
            impact: "high",
            category: "Discovery",
        })]
        discovery: TokenDiscoveryConfig = TokenDiscoveryConfig::default(),
    }
}

// ----------------------------------------------------------------------------
// TOKEN SOURCES CONFIGURATION (nested under TokensConfig)
// ----------------------------------------------------------------------------

config_struct! {
    /// Enable/priority toggle for a specific source
    pub struct SourceToggleConfig {
        enabled: bool = true,
        priority: i32 = 1,
    }
}

config_struct! {
    /// Full API configuration for a data source
    pub struct SourceApiConfig {
        enabled: bool = true,
        priority: i32 = 1,
        rate_limit_per_minute: u32 = 60,
        timeout_seconds: u64 = 10,
        cache_ttl_seconds: u64 = 60,
    }
}

config_struct! {
    /// Multi-source validation settings
    pub struct TokenSourcesConfig {
        #[metadata(field_metadata! {
            label: "Enable Multi-Source",
            hint: "Route validation through multi-source consensus",
            impact: "critical",
            category: "Sources",
        })]
        enable_multi_source: bool = true,

        #[metadata(field_metadata! {
            label: "Min Sources",
            hint: "Minimum agreeing sources required",
            min: 1,
            max: 5,
            step: 1,
            unit: "sources",
            impact: "high",
            category: "Sources",
        })]
        min_sources: usize = 2,

        #[metadata(field_metadata! {
            label: "Max Inter-Source Deviation",
            hint: "Maximum allowed deviation between sources",
            min: 1,
            max: 100,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Sources",
        })]
        max_inter_source_deviation: f64 = 20.0,

        #[metadata(field_metadata! {
            label: "DexScreener Source",
            hint: "DexScreener API configuration",
            impact: "critical",
            category: "Sources",
        })]
        dexscreener: SourceApiConfig = SourceApiConfig {
            enabled: true,
            priority: 1,
            rate_limit_per_minute: 60,
            timeout_seconds: 10,
            cache_ttl_seconds: 60,
        },

        #[metadata(field_metadata! {
            label: "GeckoTerminal Source",
            hint: "GeckoTerminal API configuration",
            impact: "high",
            category: "Sources",
        })]
        geckoterminal: SourceApiConfig = SourceApiConfig {
            enabled: true,
            priority: 2,
            rate_limit_per_minute: 30,
            timeout_seconds: 10,
            cache_ttl_seconds: 300,
        },

        #[metadata(field_metadata! {
            label: "Rugcheck Source",
            hint: "Rugcheck API configuration",
            impact: "high",
            category: "Sources",
        })]
        rugcheck: SourceApiConfig = SourceApiConfig {
            enabled: true,
            priority: 3,
            rate_limit_per_minute: 30,
            timeout_seconds: 15,
            cache_ttl_seconds: 86400,
        },
    }
}

// ----------------------------------------------------------------------------
// TOKEN DISCOVERY CONFIGURATION
// ----------------------------------------------------------------------------

config_struct! {
    pub struct TokenDiscoveryConfig {
        #[metadata(field_metadata! {
            label: "Discovery Enabled",
            hint: "Master toggle for token discovery endpoints",
            impact: "critical",
            category: "Discovery",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "DexScreener Discovery",
            hint: "Per-endpoint toggles for DexScreener discovery",
            impact: "high",
            category: "Discovery",
        })]
        dexscreener: DexscreenerDiscoveryConfig = DexscreenerDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            label: "GeckoTerminal Discovery",
            hint: "Per-endpoint toggles for GeckoTerminal discovery",
            impact: "high",
            category: "Discovery",
        })]
        geckoterminal: GeckoDiscoveryConfig = GeckoDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            label: "Rugcheck Discovery",
            hint: "Per-endpoint toggles for Rugcheck discovery",
            impact: "high",
            category: "Discovery",
        })]
        rugcheck: RugcheckDiscoveryConfig = RugcheckDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            label: "Jupiter Discovery",
            hint: "Per-endpoint toggles for Jupiter discovery",
            impact: "medium",
            category: "Discovery",
        })]
        jupiter: JupiterDiscoveryConfig = JupiterDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            label: "CoinGecko Discovery",
            hint: "Toggle CoinGecko Solana markets discovery",
            impact: "low",
            category: "Discovery",
        })]
        coingecko: CoingeckoDiscoveryConfig = CoingeckoDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            label: "DeFiLlama Discovery",
            hint: "Toggle DeFiLlama protocol discovery",
            impact: "low",
            category: "Discovery",
        })]
        defillama: DefillamaDiscoveryConfig = DefillamaDiscoveryConfig::default(),
    }
}

config_struct! {
    pub struct DexscreenerDiscoveryConfig {
        enabled: bool = true,
        latest_profiles_enabled: bool = true,
        latest_boosts_enabled: bool = true,
        top_boosts_enabled: bool = true,
    }
}

config_struct! {
    pub struct GeckoDiscoveryConfig {
        enabled: bool = true,
        new_pools_enabled: bool = true,
        recently_updated_enabled: bool = true,
        trending_enabled: bool = true,
    }
}

config_struct! {
    pub struct RugcheckDiscoveryConfig {
        enabled: bool = true,
        new_tokens_enabled: bool = true,
        recent_enabled: bool = true,
        trending_enabled: bool = true,
        verified_enabled: bool = true,
    }
}

config_struct! {
    pub struct JupiterDiscoveryConfig {
        enabled: bool = true,
        recent_enabled: bool = true,
        top_organic_enabled: bool = true,
        top_traded_enabled: bool = true,
        top_trending_enabled: bool = true,
    }
}

config_struct! {
    pub struct CoingeckoDiscoveryConfig {
        enabled: bool = false,
        markets_enabled: bool = false,
        api_key: Option<String> = None,
    }
}

config_struct! {
    pub struct DefillamaDiscoveryConfig {
        enabled: bool = false,
        protocols_enabled: bool = false,
    }
}
