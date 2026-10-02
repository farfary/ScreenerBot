// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Rejection categories: grouping of stored reason codes for the analytics views.

use crate::i18n::{ids, UiText};

/// High-level grouping of rejection reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionCategory {
    Security,
    Distribution,
    LiquidityLock,
    Fees,
    Liquidity,
    Volume,
    MarketCap,
    PriceAction,
    Activity,
    DataQuality,
    Timing,
    Market,
    Other,
}

impl RejectionCategory {
    #[cfg(test)]
    pub const ALL: [RejectionCategory; 13] = [
        RejectionCategory::Security,
        RejectionCategory::Distribution,
        RejectionCategory::LiquidityLock,
        RejectionCategory::Fees,
        RejectionCategory::Liquidity,
        RejectionCategory::Volume,
        RejectionCategory::MarketCap,
        RejectionCategory::PriceAction,
        RejectionCategory::Activity,
        RejectionCategory::DataQuality,
        RejectionCategory::Timing,
        RejectionCategory::Market,
        RejectionCategory::Other,
    ];

    /// Categorize a stored rejection code.
    pub fn of_code(reason: &str) -> Self {
        if reason.starts_with("rug_") {
            if reason.contains("authority")
                || reason.contains("rugged")
                || reason.contains("level_danger")
            {
                Self::Security
            } else if reason.contains("holder")
                || reason.contains("insider")
                || reason.contains("creator")
            {
                Self::Distribution
            } else if reason.contains("lp_") {
                Self::LiquidityLock
            } else if reason.contains("transfer_fee") {
                Self::Fees
            } else {
                Self::Security
            }
        } else if reason.starts_with("dex_") || reason.starts_with("gecko_") {
            if reason.contains("liq") || reason.contains("reserve") {
                Self::Liquidity
            } else if reason.contains("vol") {
                Self::Volume
            } else if reason.contains("mcap") || reason.contains("fdv") {
                Self::MarketCap
            } else if reason.contains("price_change") {
                Self::PriceAction
            } else if reason.contains("txn") {
                Self::Activity
            } else if reason.contains("empty") || reason.contains("missing") {
                Self::DataQuality
            } else {
                Self::Market
            }
        } else if reason.contains("decimals") || reason.contains("data_missing") {
            Self::DataQuality
        } else if reason.contains("cooldown") || reason.contains("new") {
            Self::Timing
        } else {
            Self::Other
        }
    }

    /// Stable id sent to the dashboard.
    pub fn id(&self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::Distribution => "distribution",
            Self::LiquidityLock => "liquidity_lock",
            Self::Fees => "fees",
            Self::Liquidity => "liquidity",
            Self::Volume => "volume",
            Self::MarketCap => "market_cap",
            Self::PriceAction => "price_action",
            Self::Activity => "activity",
            Self::DataQuality => "data_quality",
            Self::Timing => "timing",
            Self::Market => "market",
            Self::Other => "other",
        }
    }

    /// Catalog text for the category name.
    pub fn text(&self) -> UiText {
        UiText::new(match self {
            Self::Security => ids::FILTERING_REJECT_CATEGORY_SECURITY,
            Self::Distribution => ids::FILTERING_REJECT_CATEGORY_DISTRIBUTION,
            Self::LiquidityLock => ids::FILTERING_REJECT_CATEGORY_LIQUIDITY_LOCK,
            Self::Fees => ids::FILTERING_REJECT_CATEGORY_FEES,
            Self::Liquidity => ids::FILTERING_REJECT_CATEGORY_LIQUIDITY,
            Self::Volume => ids::FILTERING_REJECT_CATEGORY_VOLUME,
            Self::MarketCap => ids::FILTERING_REJECT_CATEGORY_MARKET_CAP,
            Self::PriceAction => ids::FILTERING_REJECT_CATEGORY_PRICE_ACTION,
            Self::Activity => ids::FILTERING_REJECT_CATEGORY_ACTIVITY,
            Self::DataQuality => ids::FILTERING_REJECT_CATEGORY_DATA_QUALITY,
            Self::Timing => ids::FILTERING_REJECT_CATEGORY_TIMING,
            Self::Market => ids::FILTERING_REJECT_CATEGORY_MARKET,
            Self::Other => ids::FILTERING_REJECT_CATEGORY_OTHER,
        })
    }

    /// Icon name for the category.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Security => "shield",
            Self::Distribution => "users",
            Self::LiquidityLock => "lock",
            Self::Fees => "percent",
            Self::Liquidity => "droplet",
            Self::Volume => "chart-bar",
            Self::MarketCap => "dollar-sign",
            Self::PriceAction => "trending-up",
            Self::Activity => "activity",
            Self::DataQuality => "circle-alert",
            Self::Timing => "clock",
            Self::Market => "trending-up",
            Self::Other => "info",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::source_message_ids;

    #[test]
    fn every_category_has_a_message_and_a_distinct_id() {
        let mut ids_seen = std::collections::HashSet::new();
        for category in RejectionCategory::ALL {
            let id = category.text().id;
            assert!(
                source_message_ids().binary_search(&id.as_ref()).is_ok(),
                "{} has no catalog message {id}",
                category.id()
            );
            assert!(ids_seen.insert(category.id()));
        }
    }

    #[test]
    fn codes_map_to_their_category() {
        assert_eq!(
            RejectionCategory::of_code("rug_lp_lock_low"),
            RejectionCategory::LiquidityLock
        );
        assert_eq!(
            RejectionCategory::of_code("dex_txn_5m"),
            RejectionCategory::Activity
        );
        assert_eq!(
            RejectionCategory::of_code("token_too_new"),
            RejectionCategory::Timing
        );
        assert_eq!(
            RejectionCategory::of_code("llm_analysis_rejected"),
            RejectionCategory::Other
        );
    }
}
