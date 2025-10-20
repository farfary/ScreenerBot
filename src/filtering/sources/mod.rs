use std::fmt;

pub mod dexscreener;
pub mod meta;
pub mod rugcheck;

/// High level origin for a filtering rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterSource {
    Core,
    DexScreener,
    Rugcheck,
}

impl FilterSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterSource::Core => "core",
            FilterSource::DexScreener => "dexscreener",
            FilterSource::Rugcheck => "rugcheck",
        }
    }
}

/// Unified set of rejection reasons shared by all filtering sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterRejectionReason {
    // Core/meta checks
    NoDecimalsInDatabase,
    TokenTooNew,
    CooldownFiltered,

    // DexScreener
    DexScreenerEmptyName,
    DexScreenerEmptySymbol,
    DexScreenerEmptyLogoUrl,
    DexScreenerEmptyWebsiteUrl,
    DexScreenerInsufficientTransactions5Min,
    DexScreenerInsufficientTransactions1H,
    DexScreenerZeroLiquidity,
    DexScreenerInsufficientLiquidity,
    DexScreenerLiquidityTooHigh,
    DexScreenerMarketCapTooLow,
    DexScreenerMarketCapTooHigh,
    DexScreenerVolumeTooLow,
    DexScreenerVolumeMissing,
    DexScreenerPriceChangeTooLow,
    DexScreenerPriceChangeTooHigh,
    DexScreenerPriceChangeMissing,

    // Rugcheck
    RugcheckRuggedToken,
    RugcheckRiskScoreTooHigh,
    RugcheckRiskLevelDanger,
    RugcheckMintAuthorityBlocked,
    RugcheckFreezeAuthorityBlocked,
    RugcheckTopHolderTooHigh,
    RugcheckTop3HoldersTooHigh,
    RugcheckNotEnoughHolders,
    RugcheckInsiderHolderCount,
    RugcheckInsiderTotalPct,
    RugcheckCreatorBalanceTooHigh,
    RugcheckTransferFeePresent,
    RugcheckTransferFeeTooHigh,
    RugcheckTransferFeeMissing,
    RugcheckGraphInsidersTooHigh,
    RugcheckLpProvidersTooLow,
    RugcheckLpProvidersMissing,
    RugcheckLpLockTooLow,
    RugcheckLpLockMissing,
}

impl FilterRejectionReason {
    /// Describe the rejection reason using a machine friendly label.
    pub fn label(&self) -> &'static str {
        match self {
            FilterRejectionReason::NoDecimalsInDatabase => "no_decimals",
            FilterRejectionReason::TokenTooNew => "token_too_new",
            FilterRejectionReason::CooldownFiltered => "cooldown_filtered",
            FilterRejectionReason::DexScreenerEmptyName => "dex_empty_name",
            FilterRejectionReason::DexScreenerEmptySymbol => "dex_empty_symbol",
            FilterRejectionReason::DexScreenerEmptyLogoUrl => "dex_empty_logo",
            FilterRejectionReason::DexScreenerEmptyWebsiteUrl => "dex_empty_website",
            FilterRejectionReason::DexScreenerInsufficientTransactions5Min => "dex_txn_5m",
            FilterRejectionReason::DexScreenerInsufficientTransactions1H => "dex_txn_1h",
            FilterRejectionReason::DexScreenerZeroLiquidity => "dex_zero_liq",
            FilterRejectionReason::DexScreenerInsufficientLiquidity => "dex_liq_low",
            FilterRejectionReason::DexScreenerLiquidityTooHigh => "dex_liq_high",
            FilterRejectionReason::DexScreenerMarketCapTooLow => "dex_mcap_low",
            FilterRejectionReason::DexScreenerMarketCapTooHigh => "dex_mcap_high",
            FilterRejectionReason::DexScreenerVolumeTooLow => "dex_vol_low",
            FilterRejectionReason::DexScreenerVolumeMissing => "dex_vol_missing",
            FilterRejectionReason::DexScreenerPriceChangeTooLow => "dex_price_change_low",
            FilterRejectionReason::DexScreenerPriceChangeTooHigh => "dex_price_change_high",
            FilterRejectionReason::DexScreenerPriceChangeMissing => "dex_price_change_missing",
            FilterRejectionReason::RugcheckRuggedToken => "rug_rugged",
            FilterRejectionReason::RugcheckRiskScoreTooHigh => "rug_score",
            FilterRejectionReason::RugcheckRiskLevelDanger => "rug_level_danger",
            FilterRejectionReason::RugcheckMintAuthorityBlocked => "rug_mint_authority",
            FilterRejectionReason::RugcheckFreezeAuthorityBlocked => "rug_freeze_authority",
            FilterRejectionReason::RugcheckTopHolderTooHigh => "rug_top_holder",
            FilterRejectionReason::RugcheckTop3HoldersTooHigh => "rug_top3_holders",
            FilterRejectionReason::RugcheckNotEnoughHolders => "rug_min_holders",
            FilterRejectionReason::RugcheckInsiderHolderCount => "rug_insider_count",
            FilterRejectionReason::RugcheckInsiderTotalPct => "rug_insider_pct",
            FilterRejectionReason::RugcheckCreatorBalanceTooHigh => "rug_creator_pct",
            FilterRejectionReason::RugcheckTransferFeePresent => "rug_transfer_fee_present",
            FilterRejectionReason::RugcheckTransferFeeTooHigh => "rug_transfer_fee_high",
            FilterRejectionReason::RugcheckTransferFeeMissing => "rug_transfer_fee_missing",
            FilterRejectionReason::RugcheckGraphInsidersTooHigh => "rug_graph_insiders",
            FilterRejectionReason::RugcheckLpProvidersTooLow => "rug_lp_providers_low",
            FilterRejectionReason::RugcheckLpProvidersMissing => "rug_lp_providers_missing",
            FilterRejectionReason::RugcheckLpLockTooLow => "rug_lp_lock_low",
            FilterRejectionReason::RugcheckLpLockMissing => "rug_lp_lock_missing",
        }
    }

    /// Map rejection reason to source category for UI summaries.
    pub fn source(&self) -> FilterSource {
        match self {
            FilterRejectionReason::NoDecimalsInDatabase
            | FilterRejectionReason::TokenTooNew
            | FilterRejectionReason::CooldownFiltered => FilterSource::Core,
            FilterRejectionReason::DexScreenerEmptyName
            | FilterRejectionReason::DexScreenerEmptySymbol
            | FilterRejectionReason::DexScreenerEmptyLogoUrl
            | FilterRejectionReason::DexScreenerEmptyWebsiteUrl
            | FilterRejectionReason::DexScreenerInsufficientTransactions5Min
            | FilterRejectionReason::DexScreenerInsufficientTransactions1H
            | FilterRejectionReason::DexScreenerZeroLiquidity
            | FilterRejectionReason::DexScreenerInsufficientLiquidity
            | FilterRejectionReason::DexScreenerLiquidityTooHigh
            | FilterRejectionReason::DexScreenerMarketCapTooLow
            | FilterRejectionReason::DexScreenerMarketCapTooHigh
            | FilterRejectionReason::DexScreenerVolumeTooLow
            | FilterRejectionReason::DexScreenerVolumeMissing
            | FilterRejectionReason::DexScreenerPriceChangeTooLow
            | FilterRejectionReason::DexScreenerPriceChangeTooHigh
            | FilterRejectionReason::DexScreenerPriceChangeMissing => FilterSource::DexScreener,
            FilterRejectionReason::RugcheckRuggedToken
            | FilterRejectionReason::RugcheckRiskScoreTooHigh
            | FilterRejectionReason::RugcheckRiskLevelDanger
            | FilterRejectionReason::RugcheckMintAuthorityBlocked
            | FilterRejectionReason::RugcheckFreezeAuthorityBlocked
            | FilterRejectionReason::RugcheckTopHolderTooHigh
            | FilterRejectionReason::RugcheckTop3HoldersTooHigh
            | FilterRejectionReason::RugcheckNotEnoughHolders
            | FilterRejectionReason::RugcheckInsiderHolderCount
            | FilterRejectionReason::RugcheckInsiderTotalPct
            | FilterRejectionReason::RugcheckCreatorBalanceTooHigh
            | FilterRejectionReason::RugcheckTransferFeePresent
            | FilterRejectionReason::RugcheckTransferFeeTooHigh
            | FilterRejectionReason::RugcheckTransferFeeMissing
            | FilterRejectionReason::RugcheckGraphInsidersTooHigh
            | FilterRejectionReason::RugcheckLpProvidersTooLow
            | FilterRejectionReason::RugcheckLpProvidersMissing
            | FilterRejectionReason::RugcheckLpLockTooLow
            | FilterRejectionReason::RugcheckLpLockMissing => FilterSource::Rugcheck,
        }
    }
}

impl fmt::Display for FilterRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}
