//! Token filtering rules and safety check configuration.

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// DEXSCREENER FILTERING CONFIGURATION
// ============================================================================

config_struct! {
    /// DexScreener-specific filtering configuration
    pub struct DexScreenerFilters {
        // Enable/disable entire source
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::SourceControl,
        })]
        enabled: bool = true,

        // Token info checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TokenInfo,
        })]
        token_info_enabled: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TokenInfo,
        })]
        require_name_and_symbol: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::TokenInfo,
        })]
        require_logo_url: bool = false,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::TokenInfo,
        })]
        require_website_url: bool = false,

        // Liquidity checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Liquidity,
        })]
        liquidity_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 10,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Liquidity,
        })]
        min_liquidity_usd: f64 = 1.0,
        #[metadata(field_metadata! {
            min: 100,
            max: 1000000000,
            step: 100000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Liquidity,
        })]
        max_liquidity_usd: f64 = 100_000_000.0,

        // Market cap checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::MarketCap,
        })]
        market_cap_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 100,
            impact: ConfigImpact::High,
            category: ConfigCategory::MarketCap,
        })]
        min_market_cap_usd: f64 = 1000.0,
        #[metadata(field_metadata! {
            min: 1000,
            max: 1000000000,
            step: 100000,
            impact: ConfigImpact::High,
            category: ConfigCategory::MarketCap,
        })]
        max_market_cap_usd: f64 = 100_000_000.0,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fdv,
        })]
        fdv_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000000000.0,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fdv,
        })]
        min_fdv_usd: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000000000.0,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fdv,
        })]
        max_fdv_usd: f64 = 100_000_000_000.0,

        // Transaction activity checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Activity,
        })]
        transactions_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Activity,
        })]
        // Default 0: Don't require immediate activity - new tokens may have quiet periods
        min_transactions_5min: i64 = 0,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Activity,
        })]
        // Default 1: Just need some activity, not heavy trading
        min_transactions_1h: i64 = 1,

        // Volume checks (new feature)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        volume_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_5m: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_1h: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_6h: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 100,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_24h: f64 = 0.0,

        // Price change checks (new feature)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        price_change_enabled: bool = false,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_m5: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_m5: f64 = 10000.0,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_h1: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_h1: f64 = 10000.0,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_h6: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_h6: f64 = 10000.0,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_h24: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_h24: f64 = 10000.0,
    }
}

// ============================================================================
// GECKOTERMINAL FILTERING CONFIGURATION
// ============================================================================

config_struct! {
    /// GeckoTerminal-specific filtering configuration
    pub struct GeckoTerminalFilters {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::SourceControl,
        })]
        enabled: bool = true,

        // Liquidity checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Liquidity,
        })]
        liquidity_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 10,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Liquidity,
        })]
        min_liquidity_usd: f64 = 1.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000000,
            step: 10000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Liquidity,
        })]
        max_liquidity_usd: f64 = 100_000_000.0,

        // Market cap checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::MarketCap,
        })]
        market_cap_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000000,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::MarketCap,
        })]
        min_market_cap_usd: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000000,
            step: 1000,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::MarketCap,
        })]
        max_market_cap_usd: f64 = 100_000_000.0,

        // Volume checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        volume_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_5m: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 10,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_1h: f64 = 0.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 10000000,
            step: 100,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Volume,
        })]
        min_volume_24h: f64 = 0.0,

        // Price change checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        price_change_enabled: bool = false,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_m5: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_m5: f64 = 10000.0,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_h1: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_h1: f64 = 10000.0,
        #[metadata(field_metadata! {
            min: -100,
            max: 10000,
            step: 5,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        min_price_change_h24: f64 = -100.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 50,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PriceChange,
        })]
        max_price_change_h24: f64 = 10000.0,

        // Pool metrics
        #[metadata(field_metadata! {
            impact: ConfigImpact::Low,
            category: ConfigCategory::PoolMetrics,
        })]
        pool_metrics_enabled: bool = false,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PoolMetrics,
        })]
        min_pool_count: u32 = 0,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000,
            step: 1,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PoolMetrics,
        })]
        max_pool_count: u32 = 1000,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000000,
            step: 100,
            impact: ConfigImpact::Low,
            category: ConfigCategory::PoolMetrics,
        })]
        min_reserve_usd: f64 = 0.0,
    }
}

// ============================================================================
// RUGCHECK FILTERING CONFIGURATION
// ============================================================================

config_struct! {
    /// RugCheck-specific filtering configuration
    pub struct RugCheckFilters {
        // Enable/disable entire source
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::SourceControl,
        })]
        enabled: bool = true,

        // Risk score check
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::RiskScore,
        })]
        risk_score_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100000,
            step: 100,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::RiskScore,
        })]
        max_risk_score: i32 = 10000,

        // Authority checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Authorities,
        })]
        authority_checks_enabled: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Authorities,
        })]
        require_authorities_safe: bool = true,

        // Mint authority check
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Authorities,
        })]
        allow_mint_authority: bool = false,

        // Freeze authority check
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Authorities,
        })]
        allow_freeze_authority: bool = false,

        // Risk level check
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::RiskLevel,
        })]
        risk_level_enabled: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::RiskLevel,
        })]
        block_danger_level: bool = true,

        // Holder distribution checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::HolderDistribution,
        })]
        holder_distribution_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::HolderDistribution,
        })]
        // Default 40%: Most new tokens have concentrated ownership initially
        max_top_holder_pct: f64 = 40.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::HolderDistribution,
        })]
        // Default 60%: Allow reasonable concentration for newer tokens
        max_top_3_holders_pct: f64 = 60.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 1000000,
            step: 50,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::HolderDistribution,
        })]
        // Default 50: Filters very new tokens but allows most established ones to pass
        min_unique_holders: u32 = 50,

        // LP lock checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::LpLock,
        })]
        lp_lock_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            impact: ConfigImpact::High,
            category: ConfigCategory::LpLock,
        })]
        min_pumpfun_lp_lock_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            impact: ConfigImpact::High,
            category: ConfigCategory::LpLock,
        })]
        min_regular_lp_lock_pct: f64 = 50.0,

        // Rugged token check
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::SecurityFlags,
        })]
        rugged_check_enabled: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::SecurityFlags,
        })]
        block_rugged_tokens: bool = true,

        // Insider detection
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::InsiderDetection,
        })]
        graph_insiders_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 20,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::InsiderDetection,
        })]
        max_graph_insiders: i32 = 3,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::InsiderDetection,
        })]
        insider_holder_checks_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 10,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::InsiderDetection,
        })]
        max_insider_holders_in_top_10: u32 = 2,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            impact: ConfigImpact::High,
            category: ConfigCategory::InsiderDetection,
        })]
        max_insider_total_pct: f64 = 20.0,

        // Creator balance check
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CreatorChecks,
        })]
        creator_balance_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::CreatorChecks,
        })]
        max_creator_balance_pct: f64 = 10.0,

        // LP provider check
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::LpProviders,
        })]
        lp_providers_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::LpProviders,
        })]
        min_lp_providers: i32 = 3,

        // Transfer fee checks
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TransferFees,
        })]
        transfer_fee_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 1,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::TransferFees,
        })]
        max_transfer_fee_pct: f64 = 5.0,
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::TransferFees,
        })]
        block_transfer_fee_tokens: bool = false,
    }
}

// ============================================================================
// ON-CHAIN SCAM FILTERING CONFIGURATION
// ============================================================================

config_struct! {
    /// On-chain scam detection — filters tokens using blockchain data
    /// (metadata, authorities, supply) without any external API calls.
    /// Runs BEFORE DexScreener/GeckoTerminal/Rugcheck to catch obvious scams early.
    pub struct OnChainFilters {
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::SourceControl,
        })]
        enabled: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::SymbolAnalysis,
        })]
        reject_numeric_symbols: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::SymbolAnalysis,
        })]
        reject_empty_symbols: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::SymbolAnalysis,
        })]
        reject_single_char_symbols: bool = false,

        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::AuthorityAnalysis,
        })]
        reject_known_scam_authorities: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::AuthorityAnalysis,
        })]
        reject_immutable_with_freeze: bool = true,

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::RiskScoring,
        })]
        combined_risk_enabled: bool = true,

        #[metadata(field_metadata! {
            min: 0,
            max: 100,
            step: 5,
            impact: ConfigImpact::High,
            category: ConfigCategory::RiskScoring,
        })]
        max_combined_risk_score: u32 = 60,
    }
}

// ============================================================================
// MAIN FILTERING CONFIGURATION (Orchestrates All Sources)
// ============================================================================

config_struct! {
    /// Main filtering configuration - orchestrates all sources
    pub struct FilteringConfig {
        // Meta requirements (apply across all sources)
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::MetaRequirements,
        })]
        cooldown_enabled: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::MetaRequirements,
        })]
        check_cooldown: bool = true,

        // Token age
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Age,
        })]
        age_enabled: bool = true,
        #[metadata(field_metadata! {
            min: 0,
            max: 10080,
            step: 10,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Age,
        })]
        min_token_age_minutes: i64 = 60,

        // Source-specific configs (nested)
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::DataSources,
        })]
        onchain: OnChainFilters = OnChainFilters::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::DataSources,
        })]
        dexscreener: DexScreenerFilters = DexScreenerFilters::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::DataSources,
        })]
        geckoterminal: GeckoTerminalFilters = GeckoTerminalFilters::default(),

        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::DataSources,
        })]
        rugcheck: RugCheckFilters = RugCheckFilters::default(),
    }
}
