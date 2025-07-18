use anyhow::{ Context, Result };
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;
use crate::rug_detection::RugDetectionConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub main_wallet_private: String,
    pub rpc_url: String,
    #[serde(default)]
    pub rpc_fallbacks: Vec<String>,
    pub discovery: DiscoveryConfig,
    pub general: GeneralConfig,
    #[serde(default)]
    pub pricing: Option<PricingConfig>,
    #[serde(default)]
    pub swap: SwapConfig,
    #[serde(default)]
    pub rpc: RpcConfig,
    #[serde(default)]
    pub trader: TraderConfig,
    #[serde(default)]
    pub rug_detection: RugDetectionConfig,
    #[serde(default)]
    pub dexscreener: DexScreenerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub min_liquidity: f64,
    pub min_volume_24h: f64,
    pub max_market_cap: Option<f64>,
    pub min_market_cap: Option<f64>,
    pub blacklisted_tokens: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub log_level: String,
    pub update_interval_seconds: u64,
    pub ui_refresh_rate_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    pub enabled: bool,
    pub update_interval_secs: u64,
    pub top_tokens_count: usize,
    pub cache_ttl_secs: u64,
    pub max_cache_size: usize,
    pub pool_calculation_enabled: bool,
    pub priority_update_interval_secs: u64,
    pub enable_dynamic_pricing: bool,
    // Dynamic pricing configuration
    pub dynamic_pricing: DynamicPricingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPricingConfig {
    pub enabled: bool,
    pub fastest_interval_secs: u64, // 5 seconds
    pub slowest_interval_secs: u64, // 5 minutes (300 seconds)
    pub high_liquidity_threshold: f64, // 1 million USD
    pub low_liquidity_threshold: f64, // 100 USD
    pub dead_token_threshold: f64, // Near zero liquidity
    pub dead_token_timeout_hours: u64, // 6 hours
    pub rate_limit_usage_threshold: f64, // 90% of available rate limit
    pub blacklist_cleanup_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapConfig {
    pub enabled: bool,
    pub default_slippage_bps: u16,
    pub max_slippage_bps: u16,
    pub min_amount_sol: f64,
    pub max_amount_sol: f64,
    pub default_priority_fee: u64,
    pub max_priority_fee: u64,
    pub compute_unit_price_micro_lamports: Option<u64>,
    pub wrap_unwrap_sol: bool,
    pub use_shared_accounts: bool,
    pub jupiter: JupiterConfig,
    pub gmgn: GmgnConfig,
    pub raydium: RaydiumConfig,
}

impl Default for SwapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_slippage_bps: 50,
            max_slippage_bps: 100,
            min_amount_sol: 0.001,
            max_amount_sol: 100.0,
            default_priority_fee: 1000,
            max_priority_fee: 5000,
            compute_unit_price_micro_lamports: Some(5000),
            wrap_unwrap_sol: true,
            use_shared_accounts: true,
            jupiter: JupiterConfig::default(),
            gmgn: GmgnConfig::default(),
            raydium: RaydiumConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterConfig {
    pub enabled: bool,
    pub api_url: String,
    pub timeout_seconds: u64,
    pub use_token_ledger: bool,
    pub as_legacy_transaction: bool,
}

impl Default for JupiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_url: "https://quote-api.jup.ag".to_string(),
            timeout_seconds: 10,
            use_token_ledger: false,
            as_legacy_transaction: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnConfig {
    pub enabled: bool,
    pub api_url: String,
    pub timeout_seconds: u64,
    pub swap_mode: String,
    pub fee: f64,
    pub anti_mev: bool,
}

impl Default for GmgnConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_url: "https://gmgn.ai/defi/router/v1/sol/tx".to_string(),
            timeout_seconds: 10,
            swap_mode: "ExactIn".to_string(),
            fee: 0.001,
            anti_mev: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumConfig {
    pub enabled: bool,
    pub api_url: String,
    pub timeout_seconds: u64,
    pub compute_unit_price_micro_lamports: u64,
}

impl Default for RaydiumConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_url: "https://api-v3.raydium.io".to_string(),
            timeout_seconds: 10,
            compute_unit_price_micro_lamports: 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub timeout_seconds: u64,
    pub commitment: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub fallback_enabled: bool,
    pub health_check_interval_seconds: u64,
    pub max_concurrent_requests: usize,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            commitment: "confirmed".to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            fallback_enabled: true,
            health_check_interval_seconds: 60,
            max_concurrent_requests: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderConfig {
    pub enabled: bool,
    pub dry_run: bool,
    pub trade_size_sol: f64,
    pub buy_trigger_percent: f64,
    pub sell_trigger_percent: f64,
    pub stop_loss_percent: f64,
    pub dca_enabled: bool,
    pub dca_min_loss_percent: f64,
    pub dca_max_loss_percent: f64,
    pub dca_levels: u32,
    pub max_positions: u32,
    pub position_check_interval_seconds: u64,
    pub price_check_interval_seconds: u64,
    pub database_path: String,
    #[serde(default)]
    pub rug_detection: RugDetectionConfig,
}

impl Default for TraderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dry_run: true,
            trade_size_sol: 0.001,
            buy_trigger_percent: -5.0,
            sell_trigger_percent: 5.0,
            stop_loss_percent: -50.0,
            dca_enabled: true,
            dca_min_loss_percent: -20.0,
            dca_max_loss_percent: -50.0,
            dca_levels: 3,
            max_positions: 20,
            position_check_interval_seconds: 30,
            price_check_interval_seconds: 10,
            database_path: "trader.db".to_string(),
            rug_detection: RugDetectionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerConfig {
    pub enabled: bool,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub rate_limit_requests_per_minute: u32,
    pub rate_limit_burst_size: u32,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
    pub retry_exponential_backoff: bool,
    pub max_retry_delay_ms: u64,
    pub user_agent: String,
}

impl Default for DexScreenerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "https://api.dexscreener.com".to_string(),
            timeout_seconds: 10,
            rate_limit_requests_per_minute: 280, // Conservative, below 300 limit
            rate_limit_burst_size: 5, // Allow small burst for urgent requests
            retry_attempts: 3,
            retry_delay_ms: 1000,
            retry_exponential_backoff: true,
            max_retry_delay_ms: 10000, // 10 seconds max
            user_agent: "ScreenerBot/1.0".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            main_wallet_private: String::new(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            rpc_fallbacks: vec![],
            discovery: DiscoveryConfig {
                enabled: true,
                interval_seconds: 300, // 5 minutes
                min_liquidity: 10000.0,
                min_volume_24h: 50000.0,
                max_market_cap: Some(1000000.0), // 1M
                min_market_cap: Some(10000.0), // 10K
                blacklisted_tokens: vec![],
                sources: vec!["raydium".to_string(), "jupiter".to_string(), "orca".to_string()],
            },
            general: GeneralConfig {
                log_level: "info".to_string(),
                update_interval_seconds: 30,
                ui_refresh_rate_ms: 1000,
            },
            pricing: Some(PricingConfig {
                enabled: true,
                update_interval_secs: 300, // 5 minutes
                top_tokens_count: 100,
                cache_ttl_secs: 300, // 5 minutes
                max_cache_size: 10000,
                pool_calculation_enabled: true,
                priority_update_interval_secs: 30, // 30 seconds for priority tokens
                enable_dynamic_pricing: true,
                dynamic_pricing: DynamicPricingConfig {
                    enabled: true,
                    fastest_interval_secs: 5,
                    slowest_interval_secs: 300,
                    high_liquidity_threshold: 1_000_000.0,
                    low_liquidity_threshold: 100.0,
                    dead_token_threshold: 0.0,
                    dead_token_timeout_hours: 6,
                    rate_limit_usage_threshold: 0.9,
                    blacklist_cleanup_interval_hours: 24,
                },
            }),
            swap: SwapConfig::default(),
            rpc: RpcConfig::default(),
            trader: TraderConfig::default(),
            rug_detection: RugDetectionConfig::default(),
            dexscreener: DexScreenerConfig::default(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            let default_config = Self::default();
            default_config.save(path)?;
            return Ok(default_config);
        }

        let content = fs
            ::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {path}"))?;

        let config: Self = serde_json
            ::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {path}"))?;

        // Validate required fields
        if config.main_wallet_private.is_empty() {
            return Err(anyhow::anyhow!("main_wallet_private is required in config"));
        }

        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = serde_json
            ::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;

        fs::write(path, content).with_context(|| format!("Failed to write config file: {path}"))?;

        Ok(())
    }

    pub fn reload(&mut self, path: &str) -> Result<()> {
        *self = Self::load(path)?;
        Ok(())
    }
}
