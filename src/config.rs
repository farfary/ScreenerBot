use anyhow::{ Context, Result };
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;

// Import swap types
use crate::swap::types::{ SwapConfig, JupiterConfig, RaydiumConfig, GmgnConfig };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub main_wallet_private: String,
    pub rpc_url: String,
    #[serde(default)]
    pub rpc_fallbacks: Vec<String>,
    pub discovery: DiscoveryConfig,
    pub trader: TraderConfig,
    pub database: DatabaseConfig,
    pub general: GeneralConfig,
    #[serde(default)]
    pub pricing: Option<PricingConfig>,
    #[serde(default)]
    pub wallet: WalletConfig,
    #[serde(default)]
    pub trading: TradingConfig,
    #[serde(default)]
    pub swap: SwapConfig,
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
pub struct TraderConfig {
    pub enabled: bool,
    pub trade_size_sol: f64,
    pub max_open_positions: u32,
    pub min_profit_percentage: f64,
    pub max_loss_percentage: f64,
    pub max_slippage: f64,
    pub min_confidence_score: f64,
    pub position_check_interval_secs: u64,
    pub time_based_profit: TimeProfitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeProfitConfig {
    pub enabled: bool,
    pub quick_profit_threshold_mins: u64,
    pub quick_profit_target: f64,
    pub medium_profit_threshold_hours: u64,
    pub medium_profit_target: f64,
    pub long_profit_threshold_hours: u64,
    pub long_profit_target: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradingConfig {
    pub enabled: bool,
    pub max_slippage: f64,
    pub min_liquidity_usd: f64,
    pub max_position_size_sol: f64,
    pub transaction_manager: TransactionManagerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransactionManagerConfig {
    pub cache_transactions: bool,
    pub cache_duration_hours: u64,
    pub track_pnl: bool,
    pub auto_calculate_profits: bool,
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
            trader: TraderConfig {
                enabled: false, // Start disabled for safety
                trade_size_sol: 0.001,
                max_open_positions: 10,
                min_profit_percentage: 3.0,
                max_loss_percentage: -70.0,
                max_slippage: 1.0,
                min_confidence_score: 0.7,
                position_check_interval_secs: 30,
                time_based_profit: TimeProfitConfig {
                    enabled: true,
                    quick_profit_threshold_mins: 5,
                    quick_profit_target: 100.0,
                    medium_profit_threshold_hours: 1,
                    medium_profit_target: 10.0,
                    long_profit_threshold_hours: 24,
                    long_profit_target: 3.0,
                },
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
            trading: TradingConfig {
                enabled: false, // Disabled by default for safety
                max_slippage: 0.05, // 5%
                min_liquidity_usd: 50000.0, // $50k minimum liquidity
                max_position_size_sol: 0.001, // 0.001 SOL max position
                transaction_manager: TransactionManagerConfig {
                    cache_transactions: true,
                    cache_duration_hours: 720, // 30 days
                    track_pnl: true,
                    auto_calculate_profits: true,
                },
            },
            swap: SwapConfig {
                enabled: true,
                default_dex: "jupiter".to_string(),
                is_anti_mev: false,
                max_slippage: 0.01, // 1%
                timeout_seconds: 30,
                retry_attempts: 3,
                dex_preferences: vec![
                    "jupiter".to_string(),
                    "raydium".to_string(),
                    "gmgn".to_string()
                ],
                jupiter: JupiterConfig {
                    enabled: true,
                    base_url: "https://quote-api.jup.ag/v6".to_string(),
                    timeout_seconds: 15,
                    max_accounts: 64,
                    only_direct_routes: false,
                    as_legacy_transaction: false,
                },
                raydium: RaydiumConfig {
                    enabled: true,
                    base_url: "https://api.raydium.io/v2".to_string(),
                    timeout_seconds: 15,
                    pool_type: "all".to_string(),
                },
                gmgn: GmgnConfig {
                    enabled: false, // Disabled by default since it requires API key
                    base_url: "https://gmgn.ai/defi/quoterv1".to_string(),
                    timeout_seconds: 15,
                    api_key: "".to_string(),
                    referral_account: "".to_string(),
                    referral_fee_bps: 0,
                },
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
