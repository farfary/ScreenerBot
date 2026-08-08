//! Feature flag types — status enum and feature group structs.

use serde::{Deserialize, Serialize};

/// Status of a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    /// Fully functional and available to all users.
    Available,
    /// Shows in UI but disabled — functionality not yet implemented.
    ComingSoon,
    /// Available but experimental — may have issues.
    Beta,
    /// Completely hidden/disabled — not shown in UI.
    Disabled,
}

impl FeatureStatus {
    /// Returns true if the feature is usable (Available or Beta).
    pub fn is_usable(&self) -> bool {
        matches!(self, FeatureStatus::Available | FeatureStatus::Beta)
    }

    /// Returns true if the feature should be shown in the UI.
    pub fn is_visible(&self) -> bool {
        !matches!(self, FeatureStatus::Disabled)
    }
}

/// Feature flags for all tools in the Tools page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFeatures {
    pub wallet_cleanup: FeatureStatus,
    pub burn_tokens: FeatureStatus,
    pub token_analyzer: FeatureStatus,
    pub create_token: FeatureStatus,
    pub trade_watcher: FeatureStatus,
    pub holder_watch: FeatureStatus,
    pub multi_buy: FeatureStatus,
    pub multi_sell: FeatureStatus,
    pub wallet_consolidation: FeatureStatus,
    pub airdrop_checker: FeatureStatus,
    pub wallet_generator: FeatureStatus,
}

impl Default for ToolFeatures {
    fn default() -> Self {
        Self {
            wallet_cleanup: FeatureStatus::Available,
            burn_tokens: FeatureStatus::ComingSoon,
            token_analyzer: FeatureStatus::ComingSoon,
            create_token: FeatureStatus::ComingSoon,
            trade_watcher: FeatureStatus::ComingSoon,
            holder_watch: FeatureStatus::ComingSoon,
            multi_buy: FeatureStatus::ComingSoon,
            multi_sell: FeatureStatus::ComingSoon,
            wallet_consolidation: FeatureStatus::ComingSoon,
            airdrop_checker: FeatureStatus::ComingSoon,
            wallet_generator: FeatureStatus::ComingSoon,
        }
    }
}

impl ToolFeatures {
    /// Get the status of a tool by its ID.
    pub fn get_status(&self, tool_id: &str) -> FeatureStatus {
        match tool_id {
            "wallet-cleanup" => self.wallet_cleanup,
            "burn-tokens" => self.burn_tokens,
            "token-analyzer" => self.token_analyzer,
            "create-token" => self.create_token,
            "trade-watcher" => self.trade_watcher,
            "token-watch" | "holder-watch" => self.holder_watch,
            "buy-multi-wallets" => self.multi_buy,
            "sell-multi-wallets" => self.multi_sell,
            "wallet-consolidation" => self.wallet_consolidation,
            "airdrop-checker" => self.airdrop_checker,
            "wallet-generator" => self.wallet_generator,
            _ => FeatureStatus::Disabled,
        }
    }
}

/// Feature flags for trading features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingFeatures {
    pub roi_exit: FeatureStatus,
    pub trailing_stop: FeatureStatus,
    pub stop_loss: FeatureStatus,
    pub time_override: FeatureStatus,
    pub dca: FeatureStatus,
    pub partial_exit: FeatureStatus,
    pub loss_blacklist: FeatureStatus,
    pub strategies: FeatureStatus,
    pub copy_wallet: FeatureStatus,
}

impl Default for TradingFeatures {
    fn default() -> Self {
        Self {
            roi_exit: FeatureStatus::Available,
            trailing_stop: FeatureStatus::Available,
            stop_loss: FeatureStatus::Available,
            time_override: FeatureStatus::Available,
            dca: FeatureStatus::Available,
            partial_exit: FeatureStatus::Available,
            loss_blacklist: FeatureStatus::Available,
            strategies: FeatureStatus::Available,
            copy_wallet: FeatureStatus::Beta,
        }
    }
}

impl TradingFeatures {
    /// Get the status of a trading feature by its ID.
    pub fn get_status(&self, feature_id: &str) -> FeatureStatus {
        match feature_id {
            "roi-exit" | "roi_exit" => self.roi_exit,
            "trailing-stop" | "trailing_stop" => self.trailing_stop,
            "stop-loss" | "stop_loss" => self.stop_loss,
            "time-override" | "time_override" => self.time_override,
            "dca" => self.dca,
            "partial-exit" | "partial_exit" => self.partial_exit,
            "loss-blacklist" | "loss_blacklist" => self.loss_blacklist,
            "strategies" => self.strategies,
            "copy-wallet" | "copy_wallet" => self.copy_wallet,
            _ => FeatureStatus::Disabled,
        }
    }
}

/// Feature flags for external integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationFeatures {
    pub telegram: FeatureStatus,
}

impl Default for IntegrationFeatures {
    fn default() -> Self {
        Self {
            telegram: FeatureStatus::Available,
        }
    }
}

/// All feature flags for ScreenerBot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    pub tools: ToolFeatures,
    pub trading: TradingFeatures,
    pub integrations: IntegrationFeatures,
    pub version: String,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            tools: ToolFeatures::default(),
            trading: TradingFeatures::default(),
            integrations: IntegrationFeatures::default(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
