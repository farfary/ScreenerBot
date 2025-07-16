use anyhow::{ Context, Result };
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub main_wallet_private: String,
    pub rpc_url: String,
    #[serde(default)]
    pub rpc_fallbacks: Vec<String>,
    pub discovery: DiscoveryConfig,
    pub database: DatabaseConfig,
    pub general: GeneralConfig,
    #[serde(default)]
    pub pricing: Option<PricingConfig>,
    #[serde(default)]
    pub wallet: WalletConfig,
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
pub struct DatabaseConfig {
    pub path: String,
    pub cleanup_interval_hours: u64,
    pub max_token_age_days: u64,
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
    pub gecko_terminal_enabled: bool,
    pub pool_calculation_enabled: bool,
    pub priority_update_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletConfig {
    pub enabled: bool,
    pub track_portfolio: bool,
    pub refresh_interval_secs: u64,
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
            database: DatabaseConfig {
                path: "cache.db".to_string(),
                cleanup_interval_hours: 24,
                max_token_age_days: 30,
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
                gecko_terminal_enabled: true,
                pool_calculation_enabled: true,
                priority_update_interval_secs: 30, // 30 seconds for priority tokens
            }),
            wallet: WalletConfig {
                enabled: true,
                track_portfolio: true,
                refresh_interval_secs: 30,
            },
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
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Self = serde_json
            ::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

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

        fs::write(path, content).with_context(|| format!("Failed to write config file: {}", path))?;

        Ok(())
    }

    pub fn reload(&mut self, path: &str) -> Result<()> {
        *self = Self::load(path)?;
        Ok(())
    }
}
