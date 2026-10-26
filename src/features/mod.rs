// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Feature flags for ScreenerBot trading features and integrations.
//!
//! Provides compile-time feature flags that control which trading features are
//! available, coming soon, in beta, or disabled.
//! Used by the frontend to control UI visibility and by the backend to gate functionality.

mod types;

pub use types::*;

/// Get the current feature flags.
pub fn get_features() -> Features {
    Features::default()
}

/// Check if a trading feature is available by ID.
pub fn is_trading_feature_available(feature_id: &str) -> bool {
    get_trading_feature_status(feature_id).is_usable()
}

/// Get feature status for a trading feature.
pub fn get_trading_feature_status(feature_id: &str) -> FeatureStatus {
    TradingFeatures::default().get_status(feature_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_feature_availability() {
        assert!(is_trading_feature_available("roi-exit"));
        assert!(is_trading_feature_available("roi_exit"));
        assert!(is_trading_feature_available("stop-loss"));
        assert!(is_trading_feature_available("dca"));
        assert!(is_trading_feature_available("copy-wallet"));
        for id in ["copy-wallet", "copy_wallet"] {
            assert_eq!(get_trading_feature_status(id), FeatureStatus::Available);
        }
        let features = serde_json::to_value(get_features()).unwrap();
        assert_eq!(features["trading"]["copy_wallet"], "available");
        assert!(
            features.get("tools").is_none(),
            "the tools group is removed"
        );
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
}
