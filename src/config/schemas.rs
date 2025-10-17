/// Configuration schemas - all config structures defined once with defaults
///
/// This module contains all configuration structures for ScreenerBot.
/// Each struct is defined using the config_struct! macro which provides:
/// - Single-source definition (no repetition)
/// - Embedded defaults
/// - Type safety
/// - Serde support
use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// RPC CONFIGURATION
// ============================================================================

config_struct! {
    /// RPC endpoint configuration
    pub struct RpcConfig {
        /// List of RPC URLs to use (round-robin)
        #[metadata(field_metadata! {
            label: "RPC URLs",
            hint: "Comma-separated RPC endpoints (round-robin)",
            impact: "critical",
            category: "Endpoints",
        })]
        urls: Vec<String> = vec!["https://api.mainnet-beta.solana.com".to_string()],
    }
}

// ============================================================================
// TRADER CONFIGURATION
// ============================================================================

config_struct! {
    /// Trading system configuration
    pub struct TraderConfig {
        // Trader control
        enabled: bool = true,

        // Core trading parameters
        #[metadata(field_metadata! {
            label: "Max Open Positions",
            hint: "Max simultaneous positions (2-5 conservative)",
            min: 1,
            max: 100,
            unit: "positions",
            impact: "critical",
            category: "Core Trading",
        })]
        max_open_positions: usize = 2,
        #[metadata(field_metadata! {
            label: "Trade Size",
            hint: "SOL per position (0.005-0.01 for testing)",
            min: 0.001,
            max: 10,
            step: 0.001,
            unit: "SOL",
            impact: "critical",
            category: "Core Trading",
        })]
        trade_size_sol: f64 = 0.005,

        // Profit thresholds
        #[metadata(field_metadata! {
            label: "Enable Profit Threshold",
            hint: "Require minimum profit before exit",
            impact: "high",
            category: "Profit Management",
        })]
        min_profit_threshold_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Min Profit %",
            hint: "2-5% typical for volatile tokens",
            min: 0,
            max: 100,
            step: 0.1,
            unit: "%",
            impact: "high",
            category: "Profit Management",
        })]
        min_profit_threshold_percent: f64 = 2.0,

        // Time-based overrides
        #[metadata(field_metadata! {
            label: "Time Override Duration",
            hint: "Hours before forced exit (168=1 week)",
            min: 1,
            max: 720,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Time Overrides",
        })]
        time_override_duration_hours: f64 = 168.0,
        #[metadata(field_metadata! {
            label: "Time Override Loss %",
            hint: "Loss % to trigger time override (-40 = exit if down 40%)",
            min: -100,
            max: 0,
            step: 1,
            unit: "%",
            impact: "medium",
            category: "Time Overrides",
        })]
        time_override_loss_threshold_percent: f64 = -40.0,

        // Position timing
        #[metadata(field_metadata! {
            label: "Close Cooldown",
            hint: "Minutes before reopening same token",
            min: 0,
            max: 1440,
            step: 5,
            unit: "minutes",
            impact: "critical",
            category: "Timing",
        })]
        position_close_cooldown_minutes: i64 = 15,

        // Performance settings
        #[metadata(field_metadata! {
            label: "Entry Check Concurrency",
            hint: "Tokens to check concurrently (higher = faster but more CPU)",
            min: 1,
            max: 50,
            step: 1,
            unit: "concurrent",
            impact: "medium",
            category: "Performance",
        })]
        entry_check_concurrency: usize = 10,

        // Dry run mode
        dry_run_mode: bool = false,

        // Sell concurrency
        sell_concurrency: usize = 5,
    }
}

// ============================================================================
// WALLET CONFIGURATION
// ============================================================================

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

// ============================================================================
// POOLS CONFIGURATION
// ============================================================================

config_struct! {
    /// Pool service configuration
    pub struct PoolsConfig {
        #[metadata(field_metadata! {
            label: "Single Pool Mode",
            hint: "Monitor only the highest-liquidity pool per token",
            impact: "high",
            category: "Monitoring",
        })]
        enable_single_pool_mode: bool = true,
        #[metadata(field_metadata! {
            label: "DexScreener Discovery",
            hint: "Enable DexScreener API for pool discovery",
            impact: "critical",
            category: "Discovery",
        })]
        enable_dexscreener_discovery: bool = true,
        #[metadata(field_metadata! {
            label: "GeckoTerminal Discovery",
            hint: "Enable GeckoTerminal API for pool discovery",
            impact: "medium",
            category: "Discovery",
        })]
        enable_geckoterminal_discovery: bool = false,
        #[metadata(field_metadata! {
            label: "Raydium Discovery",
            hint: "Enable Raydium API for pool discovery",
            impact: "medium",
            category: "Discovery",
        })]
        enable_raydium_discovery: bool = false,
        #[metadata(field_metadata! {
            label: "Discovery Tick Interval",
            hint: "Seconds between discovery sweeps",
            min: 1,
            max: 120,
            step: 1,
            unit: "seconds",
            impact: "high",
            category: "Discovery",
        })]
        discovery_tick_interval_secs: u64 = 5,
        #[metadata(field_metadata! {
            label: "Max Watched Tokens",
            hint: "Upper bound on tokens tracked simultaneously",
            min: 100,
            max: 5000,
            step: 50,
            unit: "tokens",
            impact: "critical",
            category: "Monitoring",
        })]
        max_watched_tokens: usize = 2000,
        #[metadata(field_metadata! {
            label: "Fetcher Batch Size",
            hint: "Accounts per RPC batch (≤50 recommended)",
            min: 1,
            max: 50,
            step: 1,
            unit: "accounts",
            impact: "high",
            category: "Fetcher",
        })]
        account_batch_size: usize = 50,
        #[metadata(field_metadata! {
            label: "Fetcher Interval",
            hint: "Milliseconds between fetcher loops",
            min: 100,
            max: 5000,
            step: 50,
            unit: "ms",
            impact: "medium",
            category: "Fetcher",
        })]
        fetch_interval_ms: u64 = 500,
        #[metadata(field_metadata! {
            label: "Account Stale Threshold",
            hint: "Seconds before inactive account is refreshed",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "medium",
            category: "Fetcher",
        })]
        account_stale_threshold_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "Open Position Stale Threshold",
            hint: "Seconds before refreshing accounts backing open positions",
            min: 1,
            max: 60,
            step: 1,
            unit: "seconds",
            impact: "high",
            category: "Fetcher",
        })]
        open_position_stale_threshold_secs: u64 = 5,
        #[metadata(field_metadata! {
            label: "Price Cache TTL",
            hint: "Seconds a cached price remains fresh",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "high",
            category: "Cache",
        })]
        price_cache_ttl_secs: u64 = 30,
    }
}

// ============================================================================
// POSITIONS CONFIGURATION
// ============================================================================

config_struct! {
    /// Position management configuration
    pub struct PositionsConfig {
        /// Cooldown between position opens (seconds)
        #[metadata(field_metadata! {
            label: "Open Cooldown",
            hint: "Seconds between opening positions",
            min: 0,
            max: 300,
            step: 1,
            unit: "seconds",
            impact: "critical",
            category: "Timing",
        })]
        position_open_cooldown_secs: i64 = 5,

        /// TTL for pending open swaps (seconds)
        #[metadata(field_metadata! {
            label: "Pending Open TTL",
            hint: "Time to live for pending opens (consider failed after this)",
            min: 30,
            max: 600,
            step: 10,
            unit: "seconds",
            impact: "critical",
            category: "Timing",
        })]
        pending_open_ttl_secs: i64 = 120,

        /// Extra SOL needed for profit calculations (accounts for priority fees, etc.)
        #[metadata(field_metadata! {
            label: "Profit Extra Buffer",
            hint: "Extra SOL needed for profit calculations (priority fees)",
            min: 0,
            max: 0.01,
            step: 0.0001,
            unit: "SOL",
            impact: "high",
            category: "Profit",
        })]
        profit_extra_needed_sol: f64 = 0.0002,
    }
}

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
        // Cache settings
        #[metadata(field_metadata! {
            label: "Cache TTL",
            hint: "How long to cache filter results (lower = more current)",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Performance",
        })]
        filter_cache_ttl_secs: u64 = 15,

        // Processing limits
        #[metadata(field_metadata! {
            label: "Max Tokens to Process",
            hint: "Total tokens to evaluate per cycle (10000 = all)",
            min: 100,
            max: 100000,
            step: 100,
            unit: "tokens",
            impact: "low",
            category: "Processing",
        })]
        max_tokens_to_process: usize = 10000,
        #[metadata(field_metadata! {
            label: "Target Filtered Tokens",
            hint: "Stop after N tokens pass (500 = good pool, 0 = no limit)",
            min: 0,
            max: 10000,
            step: 10,
            unit: "tokens",
            impact: "medium",
            category: "Processing",
        })]
        target_filtered_tokens: usize = 500,

        // Meta requirements (apply across all sources)
        #[metadata(field_metadata! {
            label: "Require Decimals in Database",
            hint: "Skip tokens without cached decimal data",
            impact: "high",
            category: "Meta Requirements",
        })]
        require_decimals_in_db: bool = true,
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
            label: "RugCheck Filters",
            hint: "Security filtering from RugCheck",
            impact: "critical",
            category: "Data Sources",
        })]
        rugcheck: RugCheckFilters = RugCheckFilters::default(),
    }
}

// ============================================================================
// SWAPS CONFIGURATION
// ============================================================================

config_struct! {
    /// Swap router configuration
    pub struct SwapsConfig {
        // Router enable/disable
        #[metadata(field_metadata! {
            label: "GMGN Router",
            hint: "GMGN provides MEV protection",
            impact: "high",
            category: "Routers",
        })]
        gmgn_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Jupiter Router",
            hint: "Jupiter finds best routes across DEXes",
            impact: "high",
            category: "Routers",
        })]
        jupiter_enabled: bool = true,
        #[metadata(field_metadata! {
            label: "Raydium Direct",
            hint: "Direct Raydium swaps (bypass aggregators)",
            impact: "medium",
            category: "Routers",
        })]
        raydium_enabled: bool = false,

        // Transaction confirmation timeouts
        #[metadata(field_metadata! {
            label: "TX Confirmation Timeout",
            hint: "300s = 5 min, congestion may need more",
            min: 60,
            max: 600,
            step: 30,
            unit: "seconds",
            impact: "critical",
            category: "Confirmation",
        })]
        transaction_confirmation_timeout_secs: u64 = 300,
        #[metadata(field_metadata! {
            label: "Priority Confirm Timeout",
            hint: "Timeout for priority confirmation",
            min: 10,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Confirmation",
        })]
        priority_confirmation_timeout_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "TX Confirm Max Attempts",
            hint: "Max attempts to confirm transaction",
            min: 5,
            max: 100,
            step: 5,
            unit: "attempts",
            impact: "medium",
            category: "Confirmation",
        })]
        transaction_confirmation_max_attempts: u32 = 20,
        #[metadata(field_metadata! {
            label: "Priority Confirm Attempts",
            hint: "Max attempts for priority confirmation",
            min: 5,
            max: 50,
            step: 5,
            unit: "attempts",
            impact: "medium",
            category: "Confirmation",
        })]
        priority_confirmation_max_attempts: u32 = 15,
        #[metadata(field_metadata! {
            label: "TX Confirm Retry Delay",
            hint: "Milliseconds between confirmation retries",
            min: 1000,
            max: 10000,
            step: 500,
            unit: "ms",
            impact: "critical",
            category: "Confirmation",
        })]
        transaction_confirmation_retry_delay_ms: u64 = 3000,
        #[metadata(field_metadata! {
            label: "Priority Retry Delay",
            hint: "Milliseconds between priority retries",
            min: 500,
            max: 5000,
            step: 500,
            unit: "ms",
            impact: "critical",
            category: "Confirmation",
        })]
        priority_confirmation_retry_delay_ms: u64 = 1000,
        #[metadata(field_metadata! {
            label: "Fast Failure Threshold",
            hint: "Attempts before fast failure",
            min: 1,
            max: 20,
            step: 1,
            unit: "attempts",
            impact: "low",
            category: "Confirmation",
        })]
        fast_failure_threshold_attempts: u32 = 10,

        // Confirmation delay configuration
        #[metadata(field_metadata! {
            label: "Initial Confirm Delay",
            hint: "Initial delay before first confirmation check",
            min: 1000,
            max: 10000,
            step: 500,
            unit: "ms",
            impact: "critical",
            category: "Delays",
        })]
        initial_confirmation_delay_ms: u64 = 5000,
        #[metadata(field_metadata! {
            label: "Max Confirm Delay",
            hint: "Maximum confirmation delay",
            min: 1,
            max: 60,
            step: 1,
            unit: "seconds",
            impact: "critical",
            category: "Delays",
        })]
        max_confirmation_delay_secs: u64 = 8,
        #[metadata(field_metadata! {
            label: "Confirm Backoff Multiplier",
            hint: "Backoff multiplier for retries",
            min: 1,
            max: 5,
            step: 0.1,
            unit: "x",
            impact: "critical",
            category: "Delays",
        })]
        confirmation_backoff_multiplier: f64 = 1.5,
        #[metadata(field_metadata! {
            label: "Confirmation Timeout",
            hint: "Overall confirmation timeout",
            min: 10,
            max: 300,
            step: 10,
            unit: "seconds",
            impact: "critical",
            category: "Delays",
        })]
        confirmation_timeout_secs: u64 = 60,
        #[metadata(field_metadata! {
            label: "Priority Timeout Modifier",
            hint: "Modifier for priority confirmation timeout",
            min: 1,
            max: 30,
            step: 1,
            unit: "seconds",
            impact: "critical",
            category: "Delays",
        })]
        priority_confirmation_timeout_secs_mod: u64 = 5,

        // Rate limit handling
        #[metadata(field_metadata! {
            label: "Rate Limit Base Delay",
            hint: "Base delay for rate limiting",
            min: 1,
            max: 60,
            step: 1,
            unit: "seconds",
            impact: "critical",
            category: "Rate Limit",
        })]
        rate_limit_base_delay_secs: u64 = 5,
        #[metadata(field_metadata! {
            label: "Rate Limit Increment",
            hint: "Increment for each rate limit hit",
            min: 1,
            max: 30,
            step: 1,
            unit: "seconds",
            impact: "critical",
            category: "Rate Limit",
        })]
        rate_limit_increment_secs: u64 = 2,

        // Early attempt delays
        #[metadata(field_metadata! {
            label: "Early Attempt Delay",
            hint: "Delay for early attempts",
            min: 500,
            max: 5000,
            step: 500,
            unit: "ms",
            impact: "critical",
            category: "Delays",
        })]
        early_attempt_delay_ms: u64 = 1000,
        #[metadata(field_metadata! {
            label: "Early Attempts Count",
            hint: "Number of early attempts",
            min: 1,
            max: 10,
            step: 1,
            unit: "attempts",
            impact: "low",
            category: "Delays",
        })]
        early_attempts_count: u32 = 3,

        // GMGN specific
        #[metadata(field_metadata! {
            label: "GMGN Quote API",
            hint: "GMGN API endpoint for quotes",
            impact: "low",
            category: "GMGN",
        })]
        gmgn_quote_api: String = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route".to_string(),
        #[metadata(field_metadata! {
            label: "GMGN Partner",
            hint: "Partner identifier for GMGN",
            impact: "low",
            category: "GMGN",
        })]
        gmgn_partner: String = "screenerbot".to_string(),
        #[metadata(field_metadata! {
            label: "GMGN Anti-MEV",
            hint: "Enable GMGN MEV protection",
            impact: "medium",
            category: "GMGN",
        })]
        gmgn_anti_mev: bool = false,
        #[metadata(field_metadata! {
            label: "GMGN Fee",
            hint: "Usually 0, check GMGN docs",
            min: 0,
            max: 0.1,
            step: 0.001,
            unit: "SOL",
            impact: "low",
            category: "GMGN",
        })]
        gmgn_fee_sol: f64 = 0.0,
        #[metadata(field_metadata! {
            label: "GMGN Swap Mode",
            hint: "ExactIn or ExactOut",
            impact: "low",
            category: "GMGN",
        })]
        gmgn_default_swap_mode: String = "ExactIn".to_string(),

        // Jupiter specific
        #[metadata(field_metadata! {
            label: "Jupiter Quote API",
            hint: "Jupiter API endpoint for quotes",
            impact: "low",
            category: "Jupiter",
        })]
        jupiter_quote_api: String = "https://lite-api.jup.ag/swap/v1/quote".to_string(),
        #[metadata(field_metadata! {
            label: "Jupiter Swap API",
            hint: "Jupiter API endpoint for swaps",
            impact: "low",
            category: "Jupiter",
        })]
        jupiter_swap_api: String = "https://lite-api.jup.ag/swap/v1/swap".to_string(),
        #[metadata(field_metadata! {
            label: "Jupiter Dynamic CU Limit",
            hint: "Let Jupiter calculate compute units",
            impact: "medium",
            category: "Jupiter",
        })]
        jupiter_dynamic_compute_unit_limit: bool = false,
        #[metadata(field_metadata! {
            label: "Jupiter Priority Fee",
            hint: "1000 lamports = 0.000001 SOL, higher = faster",
            min: 0,
            max: 1000000,
            step: 100,
            unit: "lamports",
            impact: "medium",
            category: "Jupiter",
        })]
        jupiter_default_priority_fee: u64 = 1000,
        #[metadata(field_metadata! {
            label: "Jupiter Swap Mode",
            hint: "ExactIn or ExactOut",
            impact: "low",
            category: "Jupiter",
        })]
        jupiter_default_swap_mode: String = "ExactIn".to_string(),

        // Slippage configuration
        #[metadata(field_metadata! {
            label: "Default Slippage",
            hint: "1% tight, 3-5% for volatile",
            min: 0.1,
            max: 25,
            step: 0.1,
            unit: "%",
            impact: "high",
            category: "Slippage",
        })]
        slippage_quote_default_pct: f64 = 1.0,
        #[metadata(field_metadata! {
            label: "Profit Exit Slippage",
            hint: "Higher ensures exits succeed",
            min: 0,
            max: 50,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Slippage",
        })]
        slippage_exit_profit_shortfall_pct: f64 = 3.0,
        #[metadata(field_metadata! {
            label: "Loss Exit Slippage",
            hint: "Even higher to exit bad positions",
            min: 0,
            max: 50,
            step: 1,
            unit: "%",
            impact: "high",
            category: "Slippage",
        })]
        slippage_exit_loss_shortfall_pct: f64 = 5.0,
        #[metadata(field_metadata! {
            label: "Exit Retry Steps",
            hint: "Comma-separated slippage for retries",
            unit: "%",
            impact: "medium",
            category: "Slippage",
        })]
        slippage_exit_retry_steps_pct: Vec<f64> = vec![3.0, 10.0, 25.0],
    }
}

// ============================================================================
// TOKENS CONFIGURATION
// ============================================================================

config_struct! {
    /// Token management configuration
    pub struct TokensConfig {
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
            label: "GeckoTerminal Rate Limit",
            hint: "GeckoTerminal API calls per minute",
            min: 10,
            max: 120,
            step: 10,
            unit: "calls/min",
            impact: "medium",
            category: "API Limits",
        })]
        geckoterminal_rate_limit_per_minute: usize = 30,
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
        #[metadata(field_metadata! {
            label: "Cache Refresh Interval",
            hint: "Minutes between cache refreshes",
            min: 1,
            max: 60,
            step: 5,
            unit: "minutes",
            impact: "critical",
            category: "Blacklist",
        })]
        cache_refresh_interval_minutes: i64 = 5,

        // OHLCV
        #[metadata(field_metadata! {
            label: "Max OHLCV Age",
            hint: "Hours to keep OHLCV data",
            min: 24,
            max: 720,
            step: 24,
            unit: "hours",
            impact: "critical",
            category: "OHLCV",
        })]
        max_ohlcv_age_hours: i64 = 168,
        #[metadata(field_metadata! {
            label: "Max Memory Cache",
            hint: "OHLCV entries in memory cache",
            min: 100,
            max: 5000,
            step: 100,
            unit: "entries",
            impact: "medium",
            category: "OHLCV",
        })]
        max_memory_cache_entries: usize = 500,
        #[metadata(field_metadata! {
            label: "Max OHLCV Limit",
            hint: "Max OHLCV candles to fetch",
            min: 100,
            max: 5000,
            step: 100,
            unit: "candles",
            impact: "low",
            category: "OHLCV",
        })]
        max_ohlcv_limit: u32 = 2000,
        #[metadata(field_metadata! {
            label: "Default OHLCV Limit",
            hint: "Default candles to fetch",
            min: 10,
            max: 1000,
            step: 10,
            unit: "candles",
            impact: "low",
            category: "OHLCV",
        })]
        default_ohlcv_limit: u32 = 100,

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

// ============================================================================
// SOL PRICE SERVICE
// ============================================================================

config_struct! {
    /// SOL price tracking configuration
    pub struct SolPriceConfig {
        /// Price refresh interval (seconds)
        #[metadata(field_metadata! {
            label: "Price Refresh Interval",
            hint: "Seconds between SOL price updates",
            min: 10,
            max: 300,
            step: 10,
            unit: "seconds",
            impact: "critical",
            category: "Timing",
        })]
        price_refresh_interval_secs: u64 = 30,
    }
}

// ============================================================================
// SUMMARY DISPLAY
// ============================================================================

config_struct! {
    /// Summary display configuration
    pub struct SummaryConfig {
        /// Display refresh interval (seconds)
        #[metadata(field_metadata! {
            label: "Display Interval",
            hint: "Seconds between summary display updates",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Display",
        })]
        summary_display_interval_secs: u64 = 15,

        /// Maximum recent closed positions to display
        #[metadata(field_metadata! {
            label: "Max Recent Closed",
            hint: "Number of recent closed positions to display",
            min: 5,
            max: 100,
            step: 5,
            unit: "positions",
            impact: "low",
            category: "Display",
        })]
        max_recent_closed_positions: usize = 20,
    }
}

// ============================================================================
// EVENTS SYSTEM
// ============================================================================

config_struct! {
    /// Events system configuration
    pub struct EventsConfig {
        /// Batch timeout (milliseconds)
        #[metadata(field_metadata! {
            label: "Batch Timeout",
            hint: "Milliseconds for event batch timeout",
            min: 10,
            max: 1000,
            step: 10,
            unit: "ms",
            impact: "critical",
            category: "Performance",
        })]
        batch_timeout_ms: u64 = 100,
    }
}

// ============================================================================
// WEBSERVER CONFIGURATION
// ============================================================================

config_struct! {
    /// CORS configuration
    pub struct CorsConfig {
        allowed_origins: Vec<String> = vec!["http://localhost:3000".to_string()],
        allowed_methods: Vec<String> = vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
        ],
    }
}

config_struct! {
    /// Rate limiting configuration
    pub struct RateLimitConfig {
        enabled: bool = true,
        requests_per_minute: u32 = 100,
    }
}

config_struct! {
    /// Authentication configuration
    pub struct AuthConfig {
        enabled: bool = false,
        api_key: String = String::new(),
    }
}

config_struct! {
    /// WebSocket Hub snapshot limits
    pub struct WsSnapshotLimitsConfig {
        positions: usize = 100,
        tokens: usize = 200,
        events: usize = 50,
        services: usize = 50,
    }
}

config_struct! {
    /// WebSocket configuration
    pub struct WebSocketConfig {
        enabled: bool = true,
        max_connections: usize = 100,
        #[metadata(field_metadata! {
            label: "Heartbeat Interval",
            hint: "Seconds between WebSocket heartbeat messages",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "WebSocket",
        })]
        heartbeat_interval_secs: u64 = 30,

        // Central hub configuration
        central_hub_enabled: bool = false,
        per_client_buffer: usize = 256,
        #[metadata(field_metadata! {
            label: "Hub Heartbeat",
            hint: "Seconds between central hub heartbeat signals",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "WebSocket",
        })]
        heartbeat_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "Client Idle Timeout",
            hint: "Seconds before disconnecting idle WebSocket clients",
            min: 10,
            max: 600,
            step: 10,
            unit: "seconds",
            impact: "critical",
            category: "WebSocket",
        })]
        client_idle_timeout_secs: u64 = 90,
        snapshot_limits: WsSnapshotLimitsConfig = WsSnapshotLimitsConfig::default(),
    }
}

config_struct! {
    /// OHLCV data monitoring configuration
    pub struct OhlcvConfig {
        /// Enable OHLCV data collection
        enabled: bool = true,
        /// Maximum number of tokens to monitor simultaneously
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
        max_empty_fetches: u32 = 10,
        /// Enable automatic gap filling
        auto_fill_gaps: bool = true,
        /// Cache size (maximum number of tokens in hot cache)
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
        pool_failover_enabled: bool = true,
        /// Maximum pool failures before switching
        max_pool_failures: u32 = 5,
    }
}

config_struct! {
    /// Tokens tab webserver configuration
    pub struct TokensTabConfig {
        /// Default page size for token lists
        default_page_size: usize = 50,

        /// Maximum page size (enforced limit)
        max_page_size: usize = 200,

        /// Auto-refresh interval (milliseconds)
        #[metadata(field_metadata! {
            label: "Auto Refresh Interval",
            hint: "Milliseconds between token table refreshes",
            min: 500,
            max: 10000,
            step: 100,
            unit: "ms",
            impact: "critical",
            category: "Tokens Tab",
        })]
        auto_refresh_interval_ms: u64 = 2000,

        /// Price staleness warning threshold (seconds)
        #[metadata(field_metadata! {
            label: "Price Staleness Threshold",
            hint: "Seconds before highlighting stale price data",
            min: 10,
            max: 600,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Tokens Tab",
        })]
        price_staleness_threshold_seconds: u64 = 60,

        /// Security score threshold for "secure" view
        secure_token_score_threshold: i32 = 500,

        /// Recent token lookback period (hours)
        #[metadata(field_metadata! {
            label: "Recent Token Window",
            hint: "Hours considered for recent token filtering",
            min: 1,
            max: 72,
            step: 1,
            unit: "hours",
            impact: "critical",
            category: "Tokens Tab",
        })]
        recent_token_hours: i64 = 24,

        /// Enable OHLCV charts
        enable_ohlcv_charts: bool = true,

        /// Enable token detail page
        enable_detail_page: bool = true,
    }
}

config_struct! {
    /// Webserver configuration
    pub struct WebserverConfig {
        enabled: bool = true,
        host: String = "127.0.0.1".to_string(),
        port: u16 = 8080,
        cors: CorsConfig = CorsConfig::default(),
        rate_limit: RateLimitConfig = RateLimitConfig::default(),
        auth: AuthConfig = AuthConfig::default(),
        tokens_tab: TokensTabConfig = TokensTabConfig::default(),
        transactions_page_default_limit: usize = 50,
        #[metadata(field_metadata! {
            label: "Transactions Poll Interval",
            hint: "Milliseconds between transaction list refreshes",
            min: 500,
            max: 10000,
            step: 100,
            unit: "ms",
            impact: "critical",
            category: "Transactions",
        })]
        transactions_poll_interval_ms: u64 = 2000,
    }
}

// ============================================================================
// SERVICES CONFIGURATION
// ============================================================================

config_struct! {
    /// Individual service configuration
    pub struct ServiceConfig {
        enabled: bool = true,
        priority: i32 = 50,
    }
}

config_struct! {
    /// Services configuration
    pub struct ServicesConfig {
        events: ServiceConfig = ServiceConfig { enabled: true, priority: 10 },
        blacklist: ServiceConfig = ServiceConfig { enabled: true, priority: 15 },
        tokens: ServiceConfig = ServiceConfig { enabled: true, priority: 20 },
        positions: ServiceConfig = ServiceConfig { enabled: true, priority: 50 },
        pools: ServiceConfig = ServiceConfig { enabled: true, priority: 30 },
        trader: ServiceConfig = ServiceConfig { enabled: true, priority: 100 },
    }
}

// ============================================================================
// MONITORING CONFIGURATION
// ============================================================================

config_struct! {
    /// System monitoring configuration
    pub struct MonitoringConfig {
        #[metadata(field_metadata! {
            label: "Metrics Interval",
            hint: "Seconds between metrics sampling",
            min: 5,
            max: 300,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Monitoring",
        })]
        metrics_interval_secs: u64 = 30,
        #[metadata(field_metadata! {
            label: "Health Check Interval",
            hint: "Seconds between service health checks",
            min: 5,
            max: 600,
            step: 5,
            unit: "seconds",
            impact: "critical",
            category: "Monitoring",
        })]
        health_check_interval_secs: u64 = 60,
    }
}

// ============================================================================
// ROOT CONFIGURATION
// ============================================================================

config_struct! {
    /// Root configuration structure containing all sub-configurations
    pub struct Config {
        /// Main wallet private key (base58 or array format)
        main_wallet_private: String = String::new(),

        /// RPC configuration
        rpc: RpcConfig = RpcConfig::default(),

        /// Trader configuration
        trader: TraderConfig = TraderConfig::default(),

        /// Positions configuration
        positions: PositionsConfig = PositionsConfig::default(),

        /// Filtering configuration
        filtering: FilteringConfig = FilteringConfig::default(),

        /// Swaps configuration
        swaps: SwapsConfig = SwapsConfig::default(),

        /// Tokens configuration
        tokens: TokensConfig = TokensConfig::default(),

    /// Pools configuration
    pools: PoolsConfig = PoolsConfig::default(),

        /// SOL price service configuration
        sol_price: SolPriceConfig = SolPriceConfig::default(),

        /// Summary display configuration
        summary: SummaryConfig = SummaryConfig::default(),

        /// Events system configuration
        events: EventsConfig = EventsConfig::default(),

        /// Webserver configuration
        webserver: WebserverConfig = WebserverConfig::default(),

        /// Services configuration
        services: ServicesConfig = ServicesConfig::default(),

        /// Monitoring configuration
        monitoring: MonitoringConfig = MonitoringConfig::default(),

        /// OHLCV data configuration
        ohlcv: OhlcvConfig = OhlcvConfig::default(),

        /// Wallet configuration
        wallet: WalletConfig = WalletConfig::default(),
    }
}

// ============================================================================
// IMPLEMENTATIONS
// ============================================================================

impl WebserverConfig {
    /// Validate webserver configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }

        if self.port == 0 {
            return Err("Port cannot be 0".to_string());
        }

        if self.rate_limit.enabled && self.rate_limit.requests_per_minute == 0 {
            return Err("Rate limit requests_per_minute must be > 0 when enabled".to_string());
        }

        Ok(())
    }

    /// Get the full bind address (host:port)
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
