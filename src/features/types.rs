// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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

/// Feature flags for trading features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingFeatures {
    pub take_profit: FeatureStatus,
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
            take_profit: FeatureStatus::Available,
            trailing_stop: FeatureStatus::Available,
            stop_loss: FeatureStatus::Available,
            time_override: FeatureStatus::Available,
            dca: FeatureStatus::Available,
            partial_exit: FeatureStatus::Available,
            loss_blacklist: FeatureStatus::Available,
            strategies: FeatureStatus::Available,
            copy_wallet: FeatureStatus::Available,
        }
    }
}

impl TradingFeatures {
    /// Get the status of a trading feature by its ID.
    pub fn get_status(&self, feature_id: &str) -> FeatureStatus {
        match feature_id {
            "take-profit" | "take_profit" => self.take_profit,
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
    pub trading: TradingFeatures,
    pub integrations: IntegrationFeatures,
    pub version: String,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            trading: TradingFeatures::default(),
            integrations: IntegrationFeatures::default(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
