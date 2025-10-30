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
            label: "Enable DexScreener Filters",
            hint: "Master switch for all DexScreener-based filtering",
            impact: "critical",
            category: "Source Control",
        })]
        enabled: bool = true,

        // Token info checks
        #[metadata(field_metadata! {
            label: "Enable Token Info Checks",
            hint: "Check for name, symbol, logo, website",
            impact: "high",
            category: "Token Info",
        })]
        token_info_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Require Name & Symbol",
            hint: "Recommended: true. Filters incomplete tokens",
            impact: "high",
            category: "Token Info",
        })]
        require_name_and_symbol: bool = true,
        #[metadata(field_metadata! {
            label: "Require Logo",
            hint: "Optional. Logo may indicate legitimacy",
            impact: "medium",
            category: "Token Info",
        })]
        require_logo_url: bool = false,
        #[metadata(field_metadata! {
            label: "Require Website",
            hint: "Optional. Website may indicate serious project",
            impact: "medium",
            category: "Token Info",
        })]
        require_website_url: bool = false,

        // Liquidity checks
        #[metadata(field_metadata! {
            label: "Enable Liquidity Checks",
            hint: "Check min/max liquidity from DexScreener",
            impact: "critical",
            category: "Liquidity",
        })]
        liquidity_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min Liquidity",
            hint: "$1 very low, $1000+ for serious trading",
            min: 0,
            max: 10000000,
            step: 10,
            unit: "USD",
            impact: "critical",
            category: "Liquidity",
        })]
        min_liquidity_usd: f64 = 1.0,
        #[metadata(field_metadata! {
            label: "Max Liquidity",
            hint: "High max to avoid filtering established tokens",
            min: 100,
            max: 1000000000,
            step: 100000,
            unit: "USD",
            impact: "medium",
            category: "Liquidity",
        })]
        max_liquidity_usd: f64 = 100_000_000.0,

        // Market cap checks
        #[metadata(field_metadata! {
            label: "Enable Market Cap Checks",
            hint: "Check min/max market cap from DexScreener",
            impact: "high",
            category: "Market Cap",
        })]
        market_cap_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min Market Cap",
            hint: "$1000 filters micro-cap tokens",
            min: 0,
            max: 10000000,
            step: 100,
            unit: "USD",
            impact: "high",
            category: "Market Cap",
        })]
        min_market_cap_usd: f64 = 1000.0,
        #[metadata(field_metadata! {
            label: "Max Market Cap",
            hint: "Filters out large-cap tokens",
            min: 1000,
            max: 1000000000,
            step: 100000,
            unit: "USD",
            impact: "high",
            category: "Market Cap",
        })]
        max_market_cap_usd: f64 = 100_000_000.0,

        #[metadata(field_metadata! {
            label: "Enable FDV Checks",
            hint: "Check fully diluted valuation bounds",
            impact: "medium",
            category: "FDV",
        })]
        fdv_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min FDV",
            hint: "Minimum fully diluted valuation in USD",
            min: 0,
            max: 1000000000000.0,
            step: 1000,
            unit: "USD",
            impact: "medium",
            category: "FDV",
        })]
        min_fdv_usd: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Max FDV",
            hint: "Maximum fully diluted valuation in USD",
            min: 0,
            max: 1000000000000.0,
            step: 1000,
            unit: "USD",
            impact: "medium",
            category: "FDV",
        })]
        max_fdv_usd: f64 = 100_000_000_000.0,

        // Transaction activity checks
        #[metadata(field_metadata! {
            label: "Enable Transaction Checks",
            hint: "Check transaction activity from DexScreener",
            impact: "medium",
            category: "Activity",
        })]
        transactions_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min TX (5min)",
            hint: "Min transactions in last 5 minutes (1+ is minimal)",
            min: 0,
            max: 1000,
            step: 1,
            unit: "txs",
            impact: "medium",
            category: "Activity",
        })]
        min_transactions_5min: i64 = 1,
        #[metadata(field_metadata! {
            label: "Min TX (1h)",
            hint: "Min transactions in last hour (sustained activity)",
            min: 0,
            max: 10000,
            step: 5,
            unit: "txs",
            impact: "medium",
            category: "Activity",
        })]
        min_transactions_1h: i64 = 5,

        // Volume checks (new feature)
        #[metadata(field_metadata! {
            label: "Enable Volume Checks",
            hint: "Check 24h volume from DexScreener",
            impact: "medium",
            category: "Volume",
        })]
        volume_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min Volume 5m",
            hint: "Minimum 5 minute trading volume in USD",
            min: 0,
            max: 1000000,
            step: 10,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_5m: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Min Volume 1h",
            hint: "Minimum 1 hour trading volume in USD",
            min: 0,
            max: 10000000,
            step: 10,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_1h: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Min Volume 6h",
            hint: "Minimum 6 hour trading volume in USD",
            min: 0,
            max: 10000000,
            step: 10,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_6h: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Min Volume 24h",
            hint: "Minimum 24h trading volume in USD",
            min: 0,
            max: 10000000,
            step: 100,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_24h: f64 = 0.0,

        // Price change checks (new feature)
        #[metadata(field_metadata! {
            label: "Enable Price Change Checks",
            hint: "Check price change from DexScreener",
            impact: "low",
            category: "Price Change",
        })]
        price_change_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min Price Change 5m",
            hint: "Minimum 5 minute price change %",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_m5: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 5m",
            hint: "Maximum 5 minute price change %",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        max_price_change_m5: f64 = 10000.0,
        #[metadata(field_metadata! {
            label: "Min Price Change 1h",
            hint: "Minimum 1h price change % (negative = dump filter)",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_h1: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 1h",
            hint: "Maximum 1h price change % (filter extreme pumps)",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        max_price_change_h1: f64 = 10000.0,
        #[metadata(field_metadata! {
            label: "Min Price Change 6h",
            hint: "Minimum 6h price change %",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_h6: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 6h",
            hint: "Maximum 6h price change %",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        max_price_change_h6: f64 = 10000.0,
        #[metadata(field_metadata! {
            label: "Min Price Change 24h",
            hint: "Minimum 24h price change %",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_h24: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 24h",
            hint: "Maximum 24h price change %",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
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
            label: "Enable GeckoTerminal Filters",
            hint: "Master switch for GeckoTerminal-based filtering",
            impact: "critical",
            category: "Source Control",
        })]
        enabled: bool = true,

        // Liquidity checks
        #[metadata(field_metadata! {
            label: "Enable Liquidity Checks",
            hint: "Check min/max liquidity from GeckoTerminal",
            impact: "critical",
            category: "Liquidity",
        })]
        liquidity_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min Liquidity",
            hint: "Minimum liquidity in USD",
            min: 0,
            max: 10000000,
            step: 10,
            unit: "USD",
            impact: "critical",
            category: "Liquidity",
        })]
        min_liquidity_usd: f64 = 1.0,
        #[metadata(field_metadata! {
            label: "Max Liquidity",
            hint: "Maximum liquidity in USD",
            min: 0,
            max: 1000000000,
            step: 10000,
            unit: "USD",
            impact: "medium",
            category: "Liquidity",
        })]
        max_liquidity_usd: f64 = 100_000_000.0,

        // Market cap checks
        #[metadata(field_metadata! {
            label: "Enable Market Cap Checks",
            hint: "Check min/max market cap from GeckoTerminal",
            impact: "medium",
            category: "Market Cap",
        })]
        market_cap_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min Market Cap",
            hint: "Minimum market cap in USD",
            min: 0,
            max: 1000000000,
            step: 1000,
            unit: "USD",
            impact: "medium",
            category: "Market Cap",
        })]
        min_market_cap_usd: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Max Market Cap",
            hint: "Maximum market cap in USD",
            min: 0,
            max: 1000000000,
            step: 1000,
            unit: "USD",
            impact: "medium",
            category: "Market Cap",
        })]
        max_market_cap_usd: f64 = 100_000_000.0,

        // Volume checks
        #[metadata(field_metadata! {
            label: "Enable Volume Checks",
            hint: "Check trading volume from GeckoTerminal",
            impact: "medium",
            category: "Volume",
        })]
        volume_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min Volume 5m",
            hint: "Minimum 5 minute trading volume in USD",
            min: 0,
            max: 1000000,
            step: 10,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_5m: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Min Volume 1h",
            hint: "Minimum 1 hour trading volume in USD",
            min: 0,
            max: 10000000,
            step: 10,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_1h: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "Min Volume 24h",
            hint: "Minimum 24 hour trading volume in USD",
            min: 0,
            max: 10000000,
            step: 100,
            unit: "USD",
            impact: "medium",
            category: "Volume",
        })]
        min_volume_24h: f64 = 0.0,

        // Price change checks
        #[metadata(field_metadata! {
            label: "Enable Price Change Checks",
            hint: "Check price change from GeckoTerminal",
            impact: "low",
            category: "Price Change",
        })]
        price_change_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min Price Change 5m",
            hint: "Minimum 5 minute price change %",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_m5: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 5m",
            hint: "Maximum 5 minute price change %",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        max_price_change_m5: f64 = 10000.0,
        #[metadata(field_metadata! {
            label: "Min Price Change 1h",
            hint: "Minimum 1 hour price change %",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_h1: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 1h",
            hint: "Maximum 1 hour price change %",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        max_price_change_h1: f64 = 10000.0,
        #[metadata(field_metadata! {
            label: "Min Price Change 24h",
            hint: "Minimum 24 hour price change %",
            min: -100,
            max: 10000,
            step: 5,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        min_price_change_h24: f64 = -100.0,
        #[metadata(field_metadata! {
            label: "Max Price Change 24h",
            hint: "Maximum 24 hour price change %",
            min: 0,
            max: 100000,
            step: 50,
            unit: "%",
            impact: "low",
            category: "Price Change",
        })]
        max_price_change_h24: f64 = 10000.0,

        // Pool metrics
        #[metadata(field_metadata! {
            label: "Enable Pool Metrics Checks",
            hint: "Check pool count and reserve metrics",
            impact: "low",
            category: "Pool Metrics",
        })]
        pool_metrics_enabled: bool = false,
        #[metadata(field_metadata! {
            label: "Min Pool Count",
            hint: "Minimum number of pools tracked",
            min: 0,
            max: 1000,
            step: 1,
            unit: "pools",
            impact: "low",
            category: "Pool Metrics",
        })]
        min_pool_count: u32 = 0,
        #[metadata(field_metadata! {
            label: "Max Pool Count",
            hint: "Maximum number of pools tracked",
            min: 0,
            max: 1000,
            step: 1,
            unit: "pools",
            impact: "low",
            category: "Pool Metrics",
        })]
        max_pool_count: u32 = 1000,
        #[metadata(field_metadata! {
            label: "Min Reserve USD",
            hint: "Minimum reserve liquidity across pools in USD",
            min: 0,
            max: 100000000,
            step: 100,
            unit: "USD",
            impact: "low",
            category: "Pool Metrics",
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
            label: "Enable RugCheck Filters",
            hint: "Master switch for all RugCheck-based filtering",
            impact: "critical",
            category: "Source Control",
        })]
        enabled: bool = true,

        // Risk score check
        #[metadata(field_metadata! {
            label: "Enable Risk Score Check",
            hint: "Check raw rugcheck risk score (0=safest, 100000+=highest risk)",
            impact: "critical",
            category: "Risk Score",
        })]
        risk_score_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Max Risk Score",
            hint: "Lower = safer. Max acceptable risk score (0 = safest, 100000+ = highest risk)",
            min: 0,
            max: 100000,
            step: 100,
            unit: "score",
            impact: "critical",
            category: "Risk Score",
        })]
        max_risk_score: i32 = 10000,

        // Authority checks
        #[metadata(field_metadata! {
            label: "Enable Authority Checks",
            hint: "Check if mint/freeze authorities are safe",
            impact: "critical",
            category: "Authorities",
        })]
        authority_checks_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Require Authorities Safe",
            hint: "Reject if authorities are not safe (recommended: true)",
            impact: "critical",
            category: "Authorities",
        })]
        require_authorities_safe: bool = true,

        // Mint authority check
        #[metadata(field_metadata! {
            label: "Allow Mint Authority",
            hint: "Allow tokens with mint authority (false = reject if present)",
            impact: "high",
            category: "Authorities",
        })]
        allow_mint_authority: bool = false,

        // Freeze authority check
        #[metadata(field_metadata! {
            label: "Allow Freeze Authority",
            hint: "Allow tokens with freeze authority (false = reject if present)",
            impact: "high",
            category: "Authorities",
        })]
        allow_freeze_authority: bool = false,

        // Risk level check
        #[metadata(field_metadata! {
            label: "Enable Risk Level Check",
            hint: "Check rugcheck risk level categorization",
            impact: "high",
            category: "Risk Level",
        })]
        risk_level_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Block High Risk Tokens",
            hint: "Reject tokens with 'Danger' risk level",
            impact: "high",
            category: "Risk Level",
        })]
        block_danger_level: bool = true,

        // Holder distribution checks
        #[metadata(field_metadata! {
            label: "Enable Holder Distribution Checks",
            hint: "Check holder concentration from RugCheck",
            impact: "high",
            category: "Holder Distribution",
        })]
        holder_distribution_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Max Top Holder %",
            hint: "15% means top holder can own max 15% supply",
            min: 0,
            max: 100,
            step: 1,
            unit: "%",
            impact: "critical",
            category: "Holder Distribution",
        })]
        max_top_holder_pct: f64 = 15.0,
        #[metadata(field_metadata! {
            label: "Max Top 3 Holders %",
            hint: "Combined max for top 3 holders (lower = more distributed)",
            min: 0,
            max: 100,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Holder Distribution",
        })]
        max_top_3_holders_pct: f64 = 35.0,
        #[metadata(field_metadata! {
            label: "Min Unique Holders",
            hint: "500+ indicates community adoption",
            min: 0,
            max: 1000000,
            step: 50,
            unit: "holders",
            impact: "medium",
            category: "Holder Distribution",
        })]
        min_unique_holders: u32 = 500,

        // LP lock checks
        #[metadata(field_metadata! {
            label: "Enable LP Lock Checks",
            hint: "Check liquidity pool lock percentage",
            impact: "high",
            category: "LP Lock",
        })]
        lp_lock_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min PumpFun LP Lock",
            hint: "50%+ reduces rug risk for PumpFun tokens",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            impact: "high",
            category: "LP Lock",
        })]
        min_pumpfun_lp_lock_pct: f64 = 50.0,
        #[metadata(field_metadata! {
            label: "Min Regular LP Lock",
            hint: "50%+ indicates locked liquidity for regular tokens",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            impact: "high",
            category: "LP Lock",
        })]
        min_regular_lp_lock_pct: f64 = 50.0,

        // Rugged token check
        #[metadata(field_metadata! {
            label: "Block Rugged Tokens",
            hint: "Reject tokens flagged as rugged by RugCheck",
            impact: "critical",
            category: "Security Flags",
        })]
        block_rugged_tokens: bool = true,

        // Insider detection
        #[metadata(field_metadata! {
            label: "Max Graph Insiders",
            hint: "Maximum detected insider wallets (0 = no limit)",
            min: 0,
            max: 20,
            step: 1,
            unit: "wallets",
            impact: "high",
            category: "Insider Detection",
        })]
        max_graph_insiders: i32 = 3,

        #[metadata(field_metadata! {
            label: "Enable Insider Holder Checks",
            hint: "Check for insider wallets in top holders",
            impact: "high",
            category: "Insider Detection",
        })]
        insider_holder_checks_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Max Insider Holders in Top 10",
            hint: "Maximum insider wallets allowed in top 10 holders",
            min: 0,
            max: 10,
            step: 1,
            unit: "holders",
            impact: "high",
            category: "Insider Detection",
        })]
        max_insider_holders_in_top_10: u32 = 2,
        #[metadata(field_metadata! {
            label: "Max Insider Total %",
            hint: "Maximum combined % held by all insider wallets",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            impact: "high",
            category: "Insider Detection",
        })]
        max_insider_total_pct: f64 = 20.0,

        // Creator balance check
        #[metadata(field_metadata! {
            label: "Max Creator Balance %",
            hint: "Maximum % creator can hold (0 = no limit)",
            min: 0,
            max: 100,
            step: 5,
            unit: "%",
            impact: "medium",
            category: "Creator Checks",
        })]
        max_creator_balance_pct: f64 = 10.0,

        // LP provider check
        #[metadata(field_metadata! {
            label: "Min LP Providers",
            hint: "Minimum LP providers required (0 = no limit)",
            min: 0,
            max: 100,
            step: 1,
            unit: "providers",
            impact: "medium",
            category: "LP Providers",
        })]
        min_lp_providers: i32 = 3,

        // Transfer fee checks
        #[metadata(field_metadata! {
            label: "Enable Transfer Fee Checks",
            hint: "Check for transfer fees (honeypot protection)",
            impact: "critical",
            category: "Transfer Fees",
        })]
        transfer_fee_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Max Transfer Fee %",
            hint: "Maximum acceptable transfer fee percentage (5% recommended)",
            min: 0,
            max: 100,
            step: 1,
            unit: "%",
            impact: "critical",
            category: "Transfer Fees",
        })]
        max_transfer_fee_pct: f64 = 5.0,
        #[metadata(field_metadata! {
            label: "Block Any Transfer Fee",
            hint: "Reject tokens with any transfer fee at all",
            impact: "high",
            category: "Transfer Fees",
        })]
        block_transfer_fee_tokens: bool = false,
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
            label: "Check Cooldown",
            hint: "Skip tokens in cooldown period after exit",
            impact: "high",
            category: "Meta Requirements",
        })]
        check_cooldown: bool = true,

        // Token age
        #[metadata(field_metadata! {
            label: "Min Token Age",
            hint: "60min avoids brand new tokens, lower for sniping",
            min: 0,
            max: 10080,
            step: 10,
            unit: "minutes",
            impact: "critical",
            category: "Age",
        })]
        min_token_age_minutes: i64 = 60,

        // Source-specific configs (nested)
        #[metadata(field_metadata! {
            label: "DexScreener Filters",
            hint: "Market data filtering from DexScreener",
            impact: "critical",
            category: "Data Sources",
        })]
        dexscreener: DexScreenerFilters = DexScreenerFilters::default(),

        #[metadata(field_metadata! {
            label: "GeckoTerminal Filters",
            hint: "Market data filtering from GeckoTerminal",
            impact: "high",
            category: "Data Sources",
        })]
        geckoterminal: GeckoTerminalFilters = GeckoTerminalFilters::default(),

        #[metadata(field_metadata! {
            label: "RugCheck Filters",
            hint: "Security filtering from RugCheck",
            impact: "critical",
            category: "Data Sources",
        })]
        rugcheck: RugCheckFilters = RugCheckFilters::default(),
    }
}
