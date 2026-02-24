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
        preferred_market_data_source: String = "dexscreener".to_owned(), // "dexscreener" or "geckoterminal"

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
            label: "Open Position Interval (s)",
            hint: "How often to update tokens with active trading positions",
            impact: "high",
            category: "Updates",
            min: 1.0,
            step: 1.0,
        })]
        open_position_seconds: u64 = 5,

        #[metadata(field_metadata! {
            label: "Pool Tracked Interval (s)",
            hint: "How often to update tokens tracked by Pool Service",
            impact: "high",
            category: "Updates",
            min: 1.0,
            step: 1.0,
        })]
        pool_tracked_seconds: u64 = 7,

        #[metadata(field_metadata! {
            label: "Filter Passed Interval (s)",
            hint: "How often to update tokens that passed filtering criteria",
            impact: "high",
            category: "Updates",
            min: 1.0,
            step: 1.0,
        })]
        filter_passed_seconds: u64 = 8,

        #[metadata(field_metadata! {
            label: "Background Interval (s)",
            hint: "How often to update oldest tokens in background refresh",
            impact: "low",
            category: "Updates",
            min: 5.0,
            step: 5.0,
        })]
        background_seconds: u64 = 30,

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
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable or disable this data source",
            impact: "high",
            category: "Sources",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Rate Limit (req/min)",
            hint: "Maximum API requests per minute to this source",
            impact: "medium",
            category: "Sources",
            min: 1.0,
            max: 300.0,
            step: 1.0,
        })]
        rate_limit_per_minute: u32 = 60,

        #[metadata(field_metadata! {
            label: "Timeout (seconds)",
            hint: "HTTP request timeout in seconds",
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
    /// DexScreener source configuration (rate limit fixed in code)
    pub struct DexscreenerSourceConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable or disable DexScreener as a data source",
            impact: "high",
            category: "Sources",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Timeout (seconds)",
            hint: "HTTP request timeout for DexScreener API calls",
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
    /// Enable/disable toggle for a specific source
    pub struct SourceToggleConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable or disable this source",
            impact: "high",
            category: "Sources",
        })]
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
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Master toggle for DexScreener discovery endpoints",
            impact: "high",
            category: "Discovery",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Latest Profiles",
            hint: "Discover tokens with recently created DexScreener profiles",
            impact: "medium",
            category: "Discovery",
        })]
        latest_profiles_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Latest Boosts",
            hint: "Discover tokens with recent boost purchases on DexScreener",
            impact: "medium",
            category: "Discovery",
        })]
        latest_boosts_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Top Boosts",
            hint: "Discover tokens with the most active boosts on DexScreener",
            impact: "medium",
            category: "Discovery",
        })]
        top_boosts_enabled: bool = true,
    }
}

config_struct! {
    pub struct GeckoDiscoveryConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Master toggle for GeckoTerminal discovery endpoints",
            impact: "high",
            category: "Discovery",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "New Pools",
            hint: "Discover tokens from newly created liquidity pools",
            impact: "medium",
            category: "Discovery",
        })]
        new_pools_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Recently Updated",
            hint: "Discover tokens with recent price/volume activity",
            impact: "medium",
            category: "Discovery",
        })]
        recently_updated_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Trending",
            hint: "Discover trending tokens on GeckoTerminal",
            impact: "medium",
            category: "Discovery",
        })]
        trending_enabled: bool = true,
    }
}

config_struct! {
    pub struct RugcheckDiscoveryConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Master toggle for Rugcheck discovery endpoints",
            impact: "high",
            category: "Discovery",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "New Tokens",
            hint: "Discover newly listed tokens from Rugcheck",
            impact: "medium",
            category: "Discovery",
        })]
        new_tokens_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Recent",
            hint: "Discover recently analyzed tokens on Rugcheck",
            impact: "medium",
            category: "Discovery",
        })]
        recent_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Trending",
            hint: "Discover trending tokens on Rugcheck",
            impact: "medium",
            category: "Discovery",
        })]
        trending_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Verified",
            hint: "Discover verified/audited tokens from Rugcheck",
            impact: "medium",
            category: "Discovery",
        })]
        verified_enabled: bool = true,
    }
}

config_struct! {
    pub struct JupiterDiscoveryConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Master toggle for Jupiter discovery endpoints",
            impact: "high",
            category: "Discovery",
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Recent",
            hint: "Discover recently listed tokens on Jupiter",
            impact: "medium",
            category: "Discovery",
        })]
        recent_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Top Organic",
            hint: "Discover tokens with highest organic trading activity",
            impact: "medium",
            category: "Discovery",
        })]
        top_organic_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Top Traded",
            hint: "Discover most traded tokens by volume on Jupiter",
            impact: "medium",
            category: "Discovery",
        })]
        top_traded_enabled: bool = true,

        #[metadata(field_metadata! {
            label: "Top Trending",
            hint: "Discover trending tokens on Jupiter aggregator",
            impact: "medium",
            category: "Discovery",
        })]
        top_trending_enabled: bool = true,
    }
}

config_struct! {
    pub struct CoingeckoDiscoveryConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable CoinGecko as a discovery source (requires API key for higher rate limits)",
            impact: "medium",
            category: "Discovery",
        })]
        enabled: bool = false,

        #[metadata(field_metadata! {
            label: "Markets",
            hint: "Discover tokens from CoinGecko Solana markets listing",
            impact: "medium",
            category: "Discovery",
        })]
        markets_enabled: bool = false,

        #[metadata(field_metadata! {
            label: "API Key",
            hint: "CoinGecko API key for higher rate limits (optional, free tier works without key)",
            impact: "low",
            category: "Discovery",
        })]
        api_key: Option<String> = None,
    }
}

config_struct! {
    pub struct DefillamaDiscoveryConfig {
        #[metadata(field_metadata! {
            label: "Enabled",
            hint: "Enable DeFiLlama as a discovery source for established protocols",
            impact: "low",
            category: "Discovery",
        })]
        enabled: bool = false,

        #[metadata(field_metadata! {
            label: "Protocols",
            hint: "Discover tokens from DeFiLlama protocol listings",
            impact: "low",
            category: "Discovery",
        })]
        protocols_enabled: bool = false,
    }
}
