use anyhow::Result;
use serde::{ Deserialize, Serialize };
use std::fs;

/// Main bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    // Network settings
    pub network: String,
    pub rpc_url: String,
    pub helius_api_key: String,

    // Wallet settings
    pub main_wallet_public: String,
    pub main_wallet_private: String,

    // Trading settings
    pub trading: TradingConfig,

    // Screener settings
    pub screener_config: ScreenerConfig,

    // Trader settings
    pub trader_config: TraderConfig,

    // Portfolio settings
    pub portfolio_config: PortfolioConfig,

    // Cache settings
    pub cache_settings: CacheConfig,

    // General settings
    pub threads: u32,
    pub loop_delay_secs: u64,

    // Notification settings
    pub telegram_chat_id: Option<i64>,
    pub bot_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub wsol: String,
    pub raydium_swap_host: String,
    pub priority_fee_microlamports: u64,
    pub slippage_bps: u16,
    pub sol_threshold: u64,
    pub max_token: u64,
    pub max_sol_buy_lamports: u64,
    pub aggregator_buy_sol_min: f64,
    pub aggregator_buy_sol_max: f64,
    pub aggregator_buy_sol_decimal: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerConfig {
    pub gecko_api_base: String,
    pub trade_volume_usd_threshold: f64,
    pub sources: ScreenerSources,
    pub filters: ScreenerFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerSources {
    pub dexscreener_enabled: bool,
    pub geckoterminal_enabled: bool,
    pub raydium_enabled: bool,
    pub rugcheck_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerFilters {
    pub min_volume_24h: f64,
    pub min_liquidity: f64,
    pub max_age_hours: u64,
    pub require_verified: bool,
    pub require_profile: bool,
    pub allow_boosted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderConfig {
    pub entry_amount_sol: f64,
    pub max_positions: u32,
    pub dca_enabled: bool,
    pub dca_percentage: f64,
    pub stop_loss_enabled: bool,
    pub stop_loss_percentage: f64,
    pub take_profit_percentage: f64,
    pub max_slippage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    pub update_interval_seconds: u64,
    pub track_unrealized_pnl: bool,
    pub display_format: String,
    pub show_detailed_breakdown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub database_path: String,
    pub cache_transactions: bool,
    pub cache_token_metadata: bool,
    pub max_cache_age_hours: u64,
    pub cleanup_interval_hours: u64,
}

impl BotConfig {
    /// Load configuration from JSON file
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let raw_config: serde_json::Value = serde_json::from_str(&content)?;

        // Convert old config format to new structured format
        let config = Self {
            network: raw_config["network"].as_str().unwrap_or("mainnet").to_string(),
            rpc_url: raw_config["rpc_url"].as_str().unwrap_or_default().to_string(),
            helius_api_key: raw_config["helius_api_key"].as_str().unwrap_or_default().to_string(),

            main_wallet_public: raw_config["main_wallet_public"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            main_wallet_private: raw_config["main_wallet_private"]
                .as_str()
                .unwrap_or_default()
                .to_string(),

            trading: TradingConfig {
                wsol: raw_config["wsol"]
                    .as_str()
                    .unwrap_or("So11111111111111111111111111111111111111112")
                    .to_string(),
                raydium_swap_host: raw_config["raydium_swap_host"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                priority_fee_microlamports: raw_config["priority_fee_microlamports"]
                    .as_u64()
                    .unwrap_or(0),
                slippage_bps: raw_config["slippage_bps"].as_u64().unwrap_or(50) as u16,
                sol_threshold: raw_config["sol_threshold"].as_u64().unwrap_or(20000000),
                max_token: raw_config["max_token"].as_u64().unwrap_or(2000000000000),
                max_sol_buy_lamports: raw_config["max_sol_buy_lamports"]
                    .as_u64()
                    .unwrap_or(100000000),
                aggregator_buy_sol_min: raw_config["aggregator_buy_sol_min"]
                    .as_f64()
                    .unwrap_or(0.001),
                aggregator_buy_sol_max: raw_config["aggregator_buy_sol_max"]
                    .as_f64()
                    .unwrap_or(0.01),
                aggregator_buy_sol_decimal: raw_config["aggregator_buy_sol_decimal"]
                    .as_u64()
                    .unwrap_or(3) as u8,
            },

            screener_config: ScreenerConfig {
                gecko_api_base: raw_config["gecko_api_base"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                trade_volume_usd_threshold: raw_config["trade_volume_usd_threshold"]
                    .as_f64()
                    .unwrap_or(0.0),
                sources: ScreenerSources {
                    dexscreener_enabled: true,
                    geckoterminal_enabled: true,
                    raydium_enabled: true,
                    rugcheck_enabled: true,
                },
                filters: ScreenerFilters {
                    min_volume_24h: 1000.0,
                    min_liquidity: 5000.0,
                    max_age_hours: 24,
                    require_verified: false,
                    require_profile: false,
                    allow_boosted: true,
                },
            },

            trader_config: TraderConfig {
                entry_amount_sol: 0.001,
                max_positions: 50,
                dca_enabled: true,
                dca_percentage: 50.0,
                stop_loss_enabled: false, // Never do loss as per requirement
                stop_loss_percentage: 0.0,
                take_profit_percentage: 20.0,
                max_slippage: 5.0,
            },

            portfolio_config: PortfolioConfig {
                update_interval_seconds: 30,
                track_unrealized_pnl: true,
                display_format: "table".to_string(),
                show_detailed_breakdown: true,
            },

            cache_settings: CacheConfig {
                database_path: "./cache.db".to_string(),
                cache_transactions: true,
                cache_token_metadata: true,
                max_cache_age_hours: 24,
                cleanup_interval_hours: 6,
            },

            threads: raw_config["threads"].as_u64().unwrap_or(20) as u32,
            loop_delay_secs: raw_config["loop_delay_secs"].as_u64().unwrap_or(3600),

            telegram_chat_id: raw_config["telegram_chat_id"].as_i64(),
            bot_token: raw_config["bot_token"].as_str().map(|s| s.to_string()),
        };

        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn save(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.main_wallet_private.is_empty() {
            return Err(anyhow::anyhow!("main_wallet_private is required"));
        }

        if self.main_wallet_public.is_empty() {
            return Err(anyhow::anyhow!("main_wallet_public is required"));
        }

        if self.rpc_url.is_empty() {
            return Err(anyhow::anyhow!("rpc_url is required"));
        }

        if self.trading.entry_amount_sol <= 0.0 {
            return Err(anyhow::anyhow!("entry_amount_sol must be positive"));
        }

        Ok(())
    }
}
