use std::fmt;

pub mod dexscreener;
pub mod geckoterminal;
pub mod meta;
pub mod rugcheck;

/// High level origin for a filtering rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterSource {
    Core,
    DexScreener,
    GeckoTerminal,
    Rugcheck,
}

impl FilterSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterSource::Core => "core",
            FilterSource::DexScreener => "dexscreener",
            FilterSource::GeckoTerminal => "geckoterminal",
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
    DexScreenerDataMissing,
    GeckoTerminalDataMissing,
    RugcheckDataMissing,

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
    DexScreenerFdvTooLow,
    DexScreenerFdvTooHigh,
    DexScreenerFdvMissing,
    DexScreenerVolume5mTooLow,
    DexScreenerVolume5mMissing,
    DexScreenerVolume1hTooLow,
    DexScreenerVolume1hMissing,
    DexScreenerVolume6hTooLow,
    DexScreenerVolume6hMissing,
    DexScreenerPriceChange5mTooLow,
    DexScreenerPriceChange5mTooHigh,
    DexScreenerPriceChange5mMissing,
    DexScreenerPriceChangeTooLow,
    DexScreenerPriceChangeTooHigh,
    DexScreenerPriceChangeMissing,
    DexScreenerPriceChange6hTooLow,
    DexScreenerPriceChange6hTooHigh,
    DexScreenerPriceChange6hMissing,
    DexScreenerPriceChange24hTooLow,
    DexScreenerPriceChange24hTooHigh,
    DexScreenerPriceChange24hMissing,

    // GeckoTerminal
    GeckoTerminalLiquidityMissing,
    GeckoTerminalLiquidityTooLow,
    GeckoTerminalLiquidityTooHigh,
    GeckoTerminalMarketCapMissing,
    GeckoTerminalMarketCapTooLow,
    GeckoTerminalMarketCapTooHigh,
    GeckoTerminalVolume5mTooLow,
    GeckoTerminalVolume5mMissing,
    GeckoTerminalVolume1hTooLow,
    GeckoTerminalVolume1hMissing,
    GeckoTerminalVolume24hTooLow,
    GeckoTerminalVolume24hMissing,
    GeckoTerminalPriceChange5mTooLow,
    GeckoTerminalPriceChange5mTooHigh,
    GeckoTerminalPriceChange5mMissing,
    GeckoTerminalPriceChange1hTooLow,
    GeckoTerminalPriceChange1hTooHigh,
    GeckoTerminalPriceChange1hMissing,
    GeckoTerminalPriceChange24hTooLow,
    GeckoTerminalPriceChange24hTooHigh,
    GeckoTerminalPriceChange24hMissing,
    GeckoTerminalPoolCountTooLow,
    GeckoTerminalPoolCountTooHigh,
    GeckoTerminalPoolCountMissing,
    GeckoTerminalReserveTooLow,
    GeckoTerminalReserveMissing,

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
            FilterRejectionReason::DexScreenerDataMissing => "dex_data_missing",
            FilterRejectionReason::GeckoTerminalDataMissing => "gecko_data_missing",
            FilterRejectionReason::RugcheckDataMissing => "rug_data_missing",
            FilterRejectionReason::DexScreenerDataMissing => "dex_data_missing",
            FilterRejectionReason::GeckoTerminalDataMissing => "gecko_data_missing",
            FilterRejectionReason::RugcheckDataMissing => "rug_data_missing",
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
            FilterRejectionReason::DexScreenerFdvTooLow => "dex_fdv_low",
            FilterRejectionReason::DexScreenerFdvTooHigh => "dex_fdv_high",
            FilterRejectionReason::DexScreenerFdvMissing => "dex_fdv_missing",
            FilterRejectionReason::DexScreenerVolume5mTooLow => "dex_vol5m_low",
            FilterRejectionReason::DexScreenerVolume5mMissing => "dex_vol5m_missing",
            FilterRejectionReason::DexScreenerVolume1hTooLow => "dex_vol1h_low",
            FilterRejectionReason::DexScreenerVolume1hMissing => "dex_vol1h_missing",
            FilterRejectionReason::DexScreenerVolume6hTooLow => "dex_vol6h_low",
            FilterRejectionReason::DexScreenerVolume6hMissing => "dex_vol6h_missing",
            FilterRejectionReason::DexScreenerPriceChange5mTooLow => "dex_price_change_5m_low",
            FilterRejectionReason::DexScreenerPriceChange5mTooHigh => "dex_price_change_5m_high",
            FilterRejectionReason::DexScreenerPriceChange5mMissing => "dex_price_change_5m_missing",
            FilterRejectionReason::DexScreenerPriceChangeTooLow => "dex_price_change_low",
            FilterRejectionReason::DexScreenerPriceChangeTooHigh => "dex_price_change_high",
            FilterRejectionReason::DexScreenerPriceChangeMissing => "dex_price_change_missing",
            FilterRejectionReason::DexScreenerPriceChange6hTooLow => "dex_price_change_6h_low",
            FilterRejectionReason::DexScreenerPriceChange6hTooHigh => "dex_price_change_6h_high",
            FilterRejectionReason::DexScreenerPriceChange6hMissing => "dex_price_change_6h_missing",
            FilterRejectionReason::DexScreenerPriceChange24hTooLow => "dex_price_change_24h_low",
            FilterRejectionReason::DexScreenerPriceChange24hTooHigh => "dex_price_change_24h_high",
            FilterRejectionReason::DexScreenerPriceChange24hMissing => {
                "dex_price_change_24h_missing"
            }
            FilterRejectionReason::GeckoTerminalLiquidityMissing => "gecko_liq_missing",
            FilterRejectionReason::GeckoTerminalLiquidityTooLow => "gecko_liq_low",
            FilterRejectionReason::GeckoTerminalLiquidityTooHigh => "gecko_liq_high",
            FilterRejectionReason::GeckoTerminalMarketCapMissing => "gecko_mcap_missing",
            FilterRejectionReason::GeckoTerminalMarketCapTooLow => "gecko_mcap_low",
            FilterRejectionReason::GeckoTerminalMarketCapTooHigh => "gecko_mcap_high",
            FilterRejectionReason::GeckoTerminalVolume5mTooLow => "gecko_vol5m_low",
            FilterRejectionReason::GeckoTerminalVolume5mMissing => "gecko_vol5m_missing",
            FilterRejectionReason::GeckoTerminalVolume1hTooLow => "gecko_vol1h_low",
            FilterRejectionReason::GeckoTerminalVolume1hMissing => "gecko_vol1h_missing",
            FilterRejectionReason::GeckoTerminalVolume24hTooLow => "gecko_vol24h_low",
            FilterRejectionReason::GeckoTerminalVolume24hMissing => "gecko_vol24h_missing",
            FilterRejectionReason::GeckoTerminalPriceChange5mTooLow => "gecko_price_change_5m_low",
            FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh => {
                "gecko_price_change_5m_high"
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mMissing => {
                "gecko_price_change_5m_missing"
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooLow => "gecko_price_change_1h_low",
            FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh => {
                "gecko_price_change_1h_high"
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hMissing => {
                "gecko_price_change_1h_missing"
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooLow => {
                "gecko_price_change_24h_low"
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh => {
                "gecko_price_change_24h_high"
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hMissing => {
                "gecko_price_change_24h_missing"
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooLow => "gecko_pool_count_low",
            FilterRejectionReason::GeckoTerminalPoolCountTooHigh => "gecko_pool_count_high",
            FilterRejectionReason::GeckoTerminalPoolCountMissing => "gecko_pool_count_missing",
            FilterRejectionReason::GeckoTerminalReserveTooLow => "gecko_reserve_low",
            FilterRejectionReason::GeckoTerminalReserveMissing => "gecko_reserve_missing",
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
            | FilterRejectionReason::CooldownFiltered
            | FilterRejectionReason::DexScreenerDataMissing
            | FilterRejectionReason::GeckoTerminalDataMissing
            | FilterRejectionReason::RugcheckDataMissing => FilterSource::Core,
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
            | FilterRejectionReason::DexScreenerFdvTooLow
            | FilterRejectionReason::DexScreenerFdvTooHigh
            | FilterRejectionReason::DexScreenerFdvMissing
            | FilterRejectionReason::DexScreenerVolumeTooLow
            | FilterRejectionReason::DexScreenerVolumeMissing
            | FilterRejectionReason::DexScreenerVolume5mTooLow
            | FilterRejectionReason::DexScreenerVolume5mMissing
            | FilterRejectionReason::DexScreenerVolume1hTooLow
            | FilterRejectionReason::DexScreenerVolume1hMissing
            | FilterRejectionReason::DexScreenerVolume6hTooLow
            | FilterRejectionReason::DexScreenerVolume6hMissing
            | FilterRejectionReason::DexScreenerPriceChangeTooLow
            | FilterRejectionReason::DexScreenerPriceChangeTooHigh
            | FilterRejectionReason::DexScreenerPriceChangeMissing
            | FilterRejectionReason::DexScreenerPriceChange5mTooLow
            | FilterRejectionReason::DexScreenerPriceChange5mTooHigh
            | FilterRejectionReason::DexScreenerPriceChange5mMissing
            | FilterRejectionReason::DexScreenerPriceChange6hTooLow
            | FilterRejectionReason::DexScreenerPriceChange6hTooHigh
            | FilterRejectionReason::DexScreenerPriceChange6hMissing
            | FilterRejectionReason::DexScreenerPriceChange24hTooLow
            | FilterRejectionReason::DexScreenerPriceChange24hTooHigh
            | FilterRejectionReason::DexScreenerPriceChange24hMissing => FilterSource::DexScreener,
            FilterRejectionReason::GeckoTerminalLiquidityMissing
            | FilterRejectionReason::GeckoTerminalLiquidityTooLow
            | FilterRejectionReason::GeckoTerminalLiquidityTooHigh
            | FilterRejectionReason::GeckoTerminalMarketCapMissing
            | FilterRejectionReason::GeckoTerminalMarketCapTooLow
            | FilterRejectionReason::GeckoTerminalMarketCapTooHigh
            | FilterRejectionReason::GeckoTerminalVolume5mTooLow
            | FilterRejectionReason::GeckoTerminalVolume5mMissing
            | FilterRejectionReason::GeckoTerminalVolume1hTooLow
            | FilterRejectionReason::GeckoTerminalVolume1hMissing
            | FilterRejectionReason::GeckoTerminalVolume24hTooLow
            | FilterRejectionReason::GeckoTerminalVolume24hMissing
            | FilterRejectionReason::GeckoTerminalPriceChange5mTooLow
            | FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh
            | FilterRejectionReason::GeckoTerminalPriceChange5mMissing
            | FilterRejectionReason::GeckoTerminalPriceChange1hTooLow
            | FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh
            | FilterRejectionReason::GeckoTerminalPriceChange1hMissing
            | FilterRejectionReason::GeckoTerminalPriceChange24hTooLow
            | FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh
            | FilterRejectionReason::GeckoTerminalPriceChange24hMissing
            | FilterRejectionReason::GeckoTerminalPoolCountTooLow
            | FilterRejectionReason::GeckoTerminalPoolCountTooHigh
            | FilterRejectionReason::GeckoTerminalPoolCountMissing
            | FilterRejectionReason::GeckoTerminalReserveTooLow
            | FilterRejectionReason::GeckoTerminalReserveMissing => FilterSource::GeckoTerminal,
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
