// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token discovery, sources, and data provider configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
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
            impact: ConfigImpact::High,
            category: ConfigCategory::DataSources,
        })]
        preferred_market_data_source: String = "dexscreener".to_owned(), // "dexscreener" or "geckoterminal"

        // Multi-source validation configuration
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        sources: TokenSourcesConfig = TokenSourcesConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        discovery: TokenDiscoveryConfig = TokenDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Updates,
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
            impact: ConfigImpact::High,
            category: ConfigCategory::Updates,
            min: 1.0,
            step: 1.0,
        })]
        open_position_seconds: u64 = 5,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Updates,
            min: 1.0,
            step: 1.0,
        })]
        pool_tracked_seconds: u64 = 7,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Updates,
            min: 1.0,
            step: 1.0,
        })]
        filter_passed_seconds: u64 = 8,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Updates,
            min: 5.0,
            step: 5.0,
        })]
        background_seconds: u64 = 30,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Updates,
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
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = true,

        /// API base URL (override the hardcoded default)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Sources,
        })]
        endpoint: String = String::new(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 300.0,
            step: 1.0,
        })]
        rate_limit_per_minute: u32 = 60,

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
    /// DexScreener source configuration (rate limit fixed in code)
    pub struct DexscreenerSourceConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = true,

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
    /// Enable/disable toggle for a specific source
    pub struct SourceToggleConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = true,
    }
}

config_struct! {
    /// Multi-source validation settings
    pub struct TokenSourcesConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        dexscreener: DexscreenerSourceConfig = DexscreenerSourceConfig {
            enabled: true,
            timeout_seconds: 10,
        },

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Sources,
        })]
        geckoterminal: SourceApiConfig = SourceApiConfig {
            enabled: true,
            endpoint: String::new(),
            rate_limit_per_minute: 30,
            timeout_seconds: 10,
        },

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Sources,
        })]
        rugcheck: SourceApiConfig = SourceApiConfig {
            enabled: true,
            endpoint: String::new(),
            rate_limit_per_minute: 30,
            timeout_seconds: 15,
        },

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        screenerbot_server: ScreenerbotServerSourceConfig =
            ScreenerbotServerSourceConfig::default(),
    }
}

config_struct! {
    /// Self-hosted ScreenerBot data server used as the preferred first-hop source
    /// for token security (Rugcheck) reports and boosted-token market identity. It
    /// serves a shared cache fast; every consumer retains a direct-provider fallback,
    /// so this is an accelerator rather than a hard dependency.
    pub struct ScreenerbotServerSourceConfig {
        /// Whether to try the ScreenerBot server as the shared first-hop cache
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Sources,
        })]
        enabled: bool = true,
        /// ScreenerBot data server base URL (no trailing slash)
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
        /// How often known tokens are checked for their resolved logo and banner
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Sources,
            min: 30.0,
            max: 3600.0,
            step: 30.0,
        })]
        media_sync_seconds: u64 = 120,
        /// How long a token's fetched logo and banner stay current before re-checking
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Sources,
            min: 1.0,
            max: 168.0,
            step: 1.0,
        })]
        media_refresh_hours: u64 = 24,
    }
}

// ----------------------------------------------------------------------------
// TOKEN DISCOVERY CONFIGURATION
// ----------------------------------------------------------------------------

config_struct! {
    pub struct TokenDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        dexscreener: DexscreenerDiscoveryConfig = DexscreenerDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        geckoterminal: GeckoDiscoveryConfig = GeckoDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        rugcheck: RugcheckDiscoveryConfig = RugcheckDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        jupiter: JupiterDiscoveryConfig = JupiterDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Discovery,
        })]
        coingecko: CoingeckoDiscoveryConfig = CoingeckoDiscoveryConfig::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Discovery,
        })]
        defillama: DefillamaDiscoveryConfig = DefillamaDiscoveryConfig::default(),
    }
}

config_struct! {
    pub struct DexscreenerDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        latest_profiles_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        latest_boosts_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        top_boosts_enabled: bool = true,
    }
}

config_struct! {
    pub struct GeckoDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        new_pools_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        recently_updated_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        trending_enabled: bool = true,
    }
}

config_struct! {
    pub struct RugcheckDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        new_tokens_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        recent_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        trending_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        verified_enabled: bool = true,
    }
}

config_struct! {
    pub struct JupiterDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        recent_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        top_organic_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        top_traded_enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        top_trending_enabled: bool = true,
    }
}

config_struct! {
    pub struct CoingeckoDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = false,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        markets_enabled: bool = false,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Discovery,
        })]
        api_key: Option<String> = None,
    }
}

config_struct! {
    pub struct DefillamaDiscoveryConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Discovery,
        })]
        enabled: bool = false,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::Discovery,
        })]
        protocols_enabled: bool = false,
    }
}
