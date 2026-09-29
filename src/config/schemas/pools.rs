//! Pool service configuration

use crate::config::metadata::{ConfigCategory, ConfigImpact};
use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Pool service configuration
    pub struct PoolsConfig {
        #[metadata(field_metadata! {
            impact: ConfigImpact::High,
            category: ConfigCategory::Monitoring,
        })]
        enable_single_pool_mode: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Discovery,
        })]
        enable_dexscreener_discovery: bool = true,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        enable_geckoterminal_discovery: bool = false,
        #[metadata(field_metadata! {
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Discovery,
        })]
        enable_raydium_discovery: bool = false,
        #[metadata(field_metadata! {
            min: 100,
            max: 5000,
            step: 50,
            impact: ConfigImpact::Critical,
            category: ConfigCategory::Monitoring,
        })]
        max_watched_tokens: usize = 2000,
        #[metadata(field_metadata! {
            min: 1,
            max: 50,
            step: 1,
            impact: ConfigImpact::High,
            category: ConfigCategory::Fetcher,
        })]
        account_batch_size: usize = 50,
        #[metadata(field_metadata! {
            min: 10,
            max: 120,
            step: 5,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Cache,
        })]
        price_cache_ttl_secs: u64 = 30,
        #[metadata(field_metadata! {
            min: 1,
            max: 10,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fetcher,
        })]
        account_blacklist_threshold: u32 = 3,
        #[metadata(field_metadata! {
            min: 1,
            max: 10,
            step: 1,
            impact: ConfigImpact::Medium,
            category: ConfigCategory::Fetcher,
        })]
        pool_blacklist_threshold: u32 = 2,
        #[metadata(field_metadata! {
            min: 60,
            max: 600,
            step: 30,
            impact: ConfigImpact::Low,
            category: ConfigCategory::Fetcher,
        })]
        failure_window_secs: u64 = 300,
    }
}
