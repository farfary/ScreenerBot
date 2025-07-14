use anyhow::Result;
use serde::{ Deserialize, Serialize };
use std::fs;

/// Main bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
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

    // Notification settings
    pub telegram_chat_id: Option<i64>,
    pub telegram_bot_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub entry_amount_sol: f64,
    pub max_positions: u32,
    pub dca_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenerConfig {
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
        let config: Self = serde_json::from_str(&content)?;

        // Validate the configuration
        config.validate()?;

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

        if self.trading.entry_amount_sol <= 0.0 {
            return Err(anyhow::anyhow!("trading.entry_amount_sol must be positive"));
        }

        if self.trader_config.entry_amount_sol <= 0.0 {
            return Err(anyhow::anyhow!("trader_config.entry_amount_sol must be positive"));
        }

        Ok(())
    }
}
