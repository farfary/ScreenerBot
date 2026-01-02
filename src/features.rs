//! Feature flags for ScreenerBot tools and trading features
//!
//! Provides compile-time feature flags that control which tools and trading
//! features are available, coming soon, in beta, or disabled.
//! Used by the frontend to control UI visibility and by the backend to gate functionality.

use serde::{Deserialize, Serialize};

// =============================================================================
// Types
// =============================================================================

/// Status of a feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    /// Fully functional and available to all users
    Available,
    /// Shows in UI but disabled - functionality not yet implemented
    ComingSoon,
    /// Available but experimental - may have issues
    Beta,
    /// Completely hidden/disabled - not shown in UI
    Disabled,
}

impl FeatureStatus {
    /// Returns true if the feature is usable (Available or Beta)
    pub fn is_usable(&self) -> bool {
        matches!(self, FeatureStatus::Available | FeatureStatus::Beta)
    }

    /// Returns true if the feature should be shown in the UI
    pub fn is_visible(&self) -> bool {
        !matches!(self, FeatureStatus::Disabled)
    }
}

// =============================================================================
// Tool Features
// =============================================================================

/// Feature flags for all tools in the Tools page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFeatures {
    /// Wallet cleanup tool - closes empty ATAs
    pub wallet_cleanup: FeatureStatus,
    /// Burn tokens tool - permanently destroys tokens
    pub burn_tokens: FeatureStatus,
    /// Token analyzer tool - detailed token analysis
    pub token_analyzer: FeatureStatus,
    /// Create token tool - launch new tokens
    pub create_token: FeatureStatus,
    /// Trade watcher tool - monitor wallet trades
    pub trade_watcher: FeatureStatus,
    /// Holder watch tool - monitor token holders
    pub holder_watch: FeatureStatus,
    /// Volume aggregator tool - aggregate trading volume
    pub volume_aggregator: FeatureStatus,
    /// Multi-buy tool - buy from multiple wallets
    pub multi_buy: FeatureStatus,
    /// Multi-sell tool - sell from multiple wallets
    pub multi_sell: FeatureStatus,
    /// Wallet consolidation tool - merge wallet balances
    pub wallet_consolidation: FeatureStatus,
    /// Airdrop checker tool - check wallet eligibility
    pub airdrop_checker: FeatureStatus,
    /// Wallet generator tool - create new wallets
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
            volume_aggregator: FeatureStatus::ComingSoon,
            multi_buy: FeatureStatus::ComingSoon,
            multi_sell: FeatureStatus::ComingSoon,
            wallet_consolidation: FeatureStatus::ComingSoon,
            airdrop_checker: FeatureStatus::ComingSoon,
            wallet_generator: FeatureStatus::ComingSoon,
        }
    }
}

impl ToolFeatures {
    /// Get the status of a tool by its ID
    pub fn get_status(&self, tool_id: &str) -> FeatureStatus {
        match tool_id {
            "wallet-cleanup" => self.wallet_cleanup,
            "burn-tokens" => self.burn_tokens,
            "token-analyzer" => self.token_analyzer,
            "create-token" => self.create_token,
            "trade-watcher" => self.trade_watcher,
            "token-watch" | "holder-watch" => self.holder_watch,
            "volume-aggregator" => self.volume_aggregator,
            "buy-multi-wallets" => self.multi_buy,
            "sell-multi-wallets" => self.multi_sell,
            "wallet-consolidation" => self.wallet_consolidation,
            "airdrop-checker" => self.airdrop_checker,
            "wallet-generator" => self.wallet_generator,
            _ => FeatureStatus::Disabled,
        }
    }
}

// =============================================================================
// Trading Features
// =============================================================================

/// Feature flags for trading features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingFeatures {
    /// ROI-based exit - sell at target profit percentage
    pub roi_exit: FeatureStatus,
    /// Trailing stop - dynamic stop loss that follows price up
    pub trailing_stop: FeatureStatus,
    /// Stop loss - sell at fixed loss percentage
    pub stop_loss: FeatureStatus,
    /// Time override - force exit after time limit
    pub time_override: FeatureStatus,
    /// DCA - dollar cost averaging into positions
    pub dca: FeatureStatus,
    /// Partial exit - sell portions of position at targets
    pub partial_exit: FeatureStatus,
    /// Loss blacklist - blacklist tokens that hit loss threshold
    pub loss_blacklist: FeatureStatus,
    /// Strategies - custom entry/exit strategy conditions
    pub strategies: FeatureStatus,
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
        }
    }
}

impl TradingFeatures {
    /// Get the status of a trading feature by its ID
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
            _ => FeatureStatus::Disabled,
        }
    }
}

// =============================================================================
// Integration Features
// =============================================================================

/// Feature flags for external integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationFeatures {
    /// Telegram notifications
    pub telegram: FeatureStatus,
}

impl Default for IntegrationFeatures {
    fn default() -> Self {
        Self {
            telegram: FeatureStatus::Available,
        }
    }
}

// =============================================================================
// Main Features Struct
// =============================================================================

/// All feature flags for ScreenerBot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    /// Tool features
    pub tools: ToolFeatures,
    /// Trading features
    pub trading: TradingFeatures,
    /// Integration features
    pub integrations: IntegrationFeatures,
    /// Current version of ScreenerBot
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

// =============================================================================
// Public API
// =============================================================================

/// Get the current feature flags
pub fn get_features() -> Features {
    Features::default()
}

/// Check if a specific tool is available by ID
///
/// Tool IDs:
/// - "wallet-cleanup", "burn-tokens", "token-analyzer", "create-token"
/// - "trade-watcher", "token-watch", "holder-watch", "volume-aggregator"
/// - "buy-multi-wallets", "sell-multi-wallets", "wallet-consolidation"
/// - "airdrop-checker", "wallet-generator"
pub fn is_tool_available(tool_id: &str) -> bool {
    get_tool_status(tool_id).is_usable()
}

/// Check if a trading feature is available by ID
///
/// Feature IDs (supports both kebab-case and snake_case):
/// - "roi-exit" / "roi_exit"
/// - "trailing-stop" / "trailing_stop"
/// - "stop-loss" / "stop_loss"
/// - "time-override" / "time_override"
/// - "dca"
/// - "partial-exit" / "partial_exit"
/// - "loss-blacklist" / "loss_blacklist"
/// - "strategies"
pub fn is_trading_feature_available(feature_id: &str) -> bool {
    get_trading_feature_status(feature_id).is_usable()
}

/// Get feature status for a tool
pub fn get_tool_status(tool_id: &str) -> FeatureStatus {
    ToolFeatures::default().get_status(tool_id)
}

/// Get feature status for a trading feature
pub fn get_trading_feature_status(feature_id: &str) -> FeatureStatus {
    TradingFeatures::default().get_status(feature_id)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_availability() {
        assert!(is_tool_available("wallet-cleanup"));
        assert!(is_tool_available("burn-tokens"));
        assert!(!is_tool_available("create-token")); // ComingSoon
        assert!(!is_tool_available("airdrop-checker")); // ComingSoon
        assert!(!is_tool_available("unknown-tool")); // Disabled
    }

    #[test]
    fn test_trading_feature_availability() {
        assert!(is_trading_feature_available("roi-exit"));
        assert!(is_trading_feature_available("roi_exit"));
        assert!(is_trading_feature_available("stop-loss"));
        assert!(is_trading_feature_available("dca"));
        assert!(!is_trading_feature_available("unknown-feature"));
    }

    #[test]
    fn test_feature_status_methods() {
        assert!(FeatureStatus::Available.is_usable());
        assert!(FeatureStatus::Beta.is_usable());
        assert!(!FeatureStatus::ComingSoon.is_usable());
        assert!(!FeatureStatus::Disabled.is_usable());

        assert!(FeatureStatus::Available.is_visible());
        assert!(FeatureStatus::ComingSoon.is_visible());
        assert!(!FeatureStatus::Disabled.is_visible());
    }

    #[test]
    fn test_holder_watch_aliases() {
        assert_eq!(
            get_tool_status("token-watch"),
            get_tool_status("holder-watch")
        );
    }
}
