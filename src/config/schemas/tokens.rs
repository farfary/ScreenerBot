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
            impact: "high",
            category: "Data Sources",
        })]
        preferred_market_data_source: String = "dexscreener".to_string(), // "dexscreener" or "geckoterminal"

        // Multi-source validation configuration
        #[metadata(field_metadata! {
            label: "Token Sources",
            hint: "Multi-source validation and per-source toggles",
            impact: "high",
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

        #[metadata(field_metadata! {
            label: "Update Intervals",
            hint: "Configure background update loop intervals for tokens module",
            impact: "medium",
            category: "Updates",
        })]
        update_intervals: UpdateIntervalsConfig = UpdateIntervalsConfig::default(),
    }
}

// ----------------------------------------------------------------------------
// TOKEN SOURCES CONFIGURATION (nested under TokensConfig)
// ----------------------------------------------------------------------------

config_struct! {
    /// Background update loop intervals (in seconds)
    pub struct UpdateIntervalsConfig {
        #[metadata(field_metadata! {
            label: "Critical Interval (s)",
            hint: "How often to update open positions (critical)",
            impact: "high",
            category: "Updates",
            min: 1.0,
            step: 1.0,
        })]
        critical_seconds: u64 = 5,

        #[metadata(field_metadata! {
            label: "Pool Interval (s)",
            hint: "How often to update tokens sourced from Pool Service",
            impact: "high",
            category: "Updates",
            min: 1.0,
            step: 1.0,
        })]
        pool_seconds: u64 = 7,

        #[metadata(field_metadata! {
            label: "High Interval (s)",
            hint: "How often to update filtered/watched tokens (high)",
            impact: "medium",
            category: "Updates",
            min: 1.0,
            step: 1.0,
        })]
        high_seconds: u64 = 10,

        #[metadata(field_metadata! {
            label: "Low Interval (s)",
            hint: "How often to update oldest non-blacklisted tokens (low)",
            impact: "low",
            category: "Updates",
            min: 5.0,
            step: 5.0,
        })]
        low_seconds: u64 = 30,

        #[metadata(field_metadata! {
            label: "Security Interval (s)",
            hint: "How often to attempt fetching Rugcheck data for tokens without security info",
            impact: "low",
            category: "Updates",
            min: 0.0,
            step: 1.0,
        })]
        security_seconds: u64 = 60,
    }
}

config_struct! {
    /// Full API configuration for a data source
    pub struct SourceApiConfig {
        enabled: bool = true,
        rate_limit_per_minute: u32 = 60,
        timeout_seconds: u64 = 10,
    }
}

config_struct! {
    /// DexScreener source configuration (rate limit fixed in code)
    pub struct DexscreenerSourceConfig {
        enabled: bool = true,
        timeout_seconds: u64 = 10,
    }
}

config_struct! {
    /// Enable/disable toggle for a specific source
    pub struct SourceToggleConfig {
        enabled: bool = true,
    }
}

config_struct! {
    /// Multi-source validation settings
    pub struct TokenSourcesConfig {
        #[metadata(field_metadata! {
            label: "DexScreener Source",
            hint: "DexScreener API configuration",
            impact: "high",
            category: "Sources",
        })]
        dexscreener: DexscreenerSourceConfig = DexscreenerSourceConfig {
            enabled: true,
            timeout_seconds: 10,
        },

        #[metadata(field_metadata! {
            label: "GeckoTerminal Source",
            hint: "GeckoTerminal API configuration",
            impact: "medium",
            category: "Sources",
        })]
        geckoterminal: SourceApiConfig = SourceApiConfig {
            enabled: true,
            rate_limit_per_minute: 30,
            timeout_seconds: 10,
        },

        #[metadata(field_metadata! {
            label: "Rugcheck Source",
            hint: "Rugcheck API configuration",
            impact: "medium",
            category: "Sources",
        })]
        rugcheck: SourceApiConfig = SourceApiConfig {
            enabled: true,
            rate_limit_per_minute: 30,
            timeout_seconds: 15,
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
