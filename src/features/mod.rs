//! Feature flags for ScreenerBot tools and trading features.
//!
//! Provides compile-time feature flags that control which tools and trading
//! features are available, coming soon, in beta, or disabled.
//! Used by the frontend to control UI visibility and by the backend to gate functionality.

mod types;

pub use types::*;

/// Get the current feature flags.
pub fn get_features() -> Features {
    Features::default()
}

/// Check if a specific tool is available by ID.
pub fn is_tool_available(tool_id: &str) -> bool {
    get_tool_status(tool_id).is_usable()
}

/// Check if a trading feature is available by ID.
pub fn is_trading_feature_available(feature_id: &str) -> bool {
    get_trading_feature_status(feature_id).is_usable()
}

/// Get feature status for a tool.
pub fn get_tool_status(tool_id: &str) -> FeatureStatus {
    ToolFeatures::default().get_status(tool_id)
}

/// Get feature status for a trading feature.
pub fn get_trading_feature_status(feature_id: &str) -> FeatureStatus {
    TradingFeatures::default().get_status(feature_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_availability() {
        assert!(is_tool_available("wallet-cleanup"));
        assert!(!is_tool_available("burn-tokens"));
        assert!(!is_tool_available("create-token"));
        assert!(!is_tool_available("airdrop-checker"));
        assert!(!is_tool_available("unknown-tool"));
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
