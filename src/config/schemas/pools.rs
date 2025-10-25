/// Pool service configuration
use crate::config_struct;
use crate::field_metadata;

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
            label: "Account Not Found Threshold",
            hint: "Failures before blacklisting account (3-5 recommended)",
            min: 1,
            max: 10,
            step: 1,
            unit: "failures",
            impact: "medium",
            category: "Fetcher",
        })]
        account_not_found_threshold: u32 = 3,
        #[metadata(field_metadata! {
            label: "Blacklist TTL Hours",
            hint: "Hours before blacklist expires (0 = permanent)",
            min: 0,
            max: 720,
            step: 24,
            unit: "hours",
            impact: "low",
            category: "Fetcher",
        })]
        blacklist_ttl_hours: u32 = 168, // 7 days
    }
}
