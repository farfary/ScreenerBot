use std::fmt;

pub mod ai;
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
    Ai,
}

impl FilterSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterSource::Core => "core",
            FilterSource::DexScreener => "dexscreener",
            FilterSource::GeckoTerminal => "geckoterminal",
            FilterSource::Rugcheck => "rugcheck",
            FilterSource::Ai => "ai",
        }
    }
}

/// Unified set of rejection reasons shared by all filtering sources.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FilterRejectionReason {
    // Core/meta checks
    NoDecimalsInDatabase,
    TokenTooNew,
    CooldownFiltered,
    DexScreenerDataMissing,
    GeckoTerminalDataMissing,
    RugcheckDataMissing,

    // AI filtering
    AiRejected {
        reason: String,
        confidence: u8,
        provider: String,
    },

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
    pub fn label(&self) -> String {
        match self {
            FilterRejectionReason::NoDecimalsInDatabase => "no_decimals".to_string(),
            FilterRejectionReason::TokenTooNew => "token_too_new".to_string(),
            FilterRejectionReason::CooldownFiltered => "cooldown_filtered".to_string(),
            FilterRejectionReason::DexScreenerDataMissing => "dex_data_missing".to_string(),
            FilterRejectionReason::GeckoTerminalDataMissing => "gecko_data_missing".to_string(),
            FilterRejectionReason::RugcheckDataMissing => "rug_data_missing".to_string(),
            FilterRejectionReason::AiRejected { .. } => "ai_rejected".to_string(),
            FilterRejectionReason::DexScreenerEmptyName => "dex_empty_name".to_string(),
            FilterRejectionReason::DexScreenerEmptySymbol => "dex_empty_symbol".to_string(),
            FilterRejectionReason::DexScreenerEmptyLogoUrl => "dex_empty_logo".to_string(),
            FilterRejectionReason::DexScreenerEmptyWebsiteUrl => "dex_empty_website".to_string(),
            FilterRejectionReason::DexScreenerInsufficientTransactions5Min => {
                "dex_txn_5m".to_string()
            }
            FilterRejectionReason::DexScreenerInsufficientTransactions1H => {
                "dex_txn_1h".to_string()
            }
            FilterRejectionReason::DexScreenerZeroLiquidity => "dex_zero_liq".to_string(),
            FilterRejectionReason::DexScreenerInsufficientLiquidity => "dex_liq_low".to_string(),
            FilterRejectionReason::DexScreenerLiquidityTooHigh => "dex_liq_high".to_string(),
            FilterRejectionReason::DexScreenerMarketCapTooLow => "dex_mcap_low".to_string(),
            FilterRejectionReason::DexScreenerMarketCapTooHigh => "dex_mcap_high".to_string(),
            FilterRejectionReason::DexScreenerVolumeTooLow => "dex_vol_low".to_string(),
            FilterRejectionReason::DexScreenerVolumeMissing => "dex_vol_missing".to_string(),
            FilterRejectionReason::DexScreenerFdvTooLow => "dex_fdv_low".to_string(),
            FilterRejectionReason::DexScreenerFdvTooHigh => "dex_fdv_high".to_string(),
            FilterRejectionReason::DexScreenerFdvMissing => "dex_fdv_missing".to_string(),
            FilterRejectionReason::DexScreenerVolume5mTooLow => "dex_vol5m_low".to_string(),
            FilterRejectionReason::DexScreenerVolume5mMissing => "dex_vol5m_missing".to_string(),
            FilterRejectionReason::DexScreenerVolume1hTooLow => "dex_vol1h_low".to_string(),
            FilterRejectionReason::DexScreenerVolume1hMissing => "dex_vol1h_missing".to_string(),
            FilterRejectionReason::DexScreenerVolume6hTooLow => "dex_vol6h_low".to_string(),
            FilterRejectionReason::DexScreenerVolume6hMissing => "dex_vol6h_missing".to_string(),
            FilterRejectionReason::DexScreenerPriceChange5mTooLow => {
                "dex_price_change_5m_low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange5mTooHigh => {
                "dex_price_change_5m_high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange5mMissing => {
                "dex_price_change_5m_missing".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooLow => {
                "dex_price_change_low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooHigh => {
                "dex_price_change_high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChangeMissing => {
                "dex_price_change_missing".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooLow => {
                "dex_price_change_6h_low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooHigh => {
                "dex_price_change_6h_high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange6hMissing => {
                "dex_price_change_6h_missing".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooLow => {
                "dex_price_change_24h_low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooHigh => {
                "dex_price_change_24h_high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange24hMissing => {
                "dex_price_change_24h_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalLiquidityMissing => "gecko_liq_missing".to_string(),
            FilterRejectionReason::GeckoTerminalLiquidityTooLow => "gecko_liq_low".to_string(),
            FilterRejectionReason::GeckoTerminalLiquidityTooHigh => "gecko_liq_high".to_string(),
            FilterRejectionReason::GeckoTerminalMarketCapMissing => {
                "gecko_mcap_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalMarketCapTooLow => "gecko_mcap_low".to_string(),
            FilterRejectionReason::GeckoTerminalMarketCapTooHigh => "gecko_mcap_high".to_string(),
            FilterRejectionReason::GeckoTerminalVolume5mTooLow => "gecko_vol5m_low".to_string(),
            FilterRejectionReason::GeckoTerminalVolume5mMissing => {
                "gecko_vol5m_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalVolume1hTooLow => "gecko_vol1h_low".to_string(),
            FilterRejectionReason::GeckoTerminalVolume1hMissing => {
                "gecko_vol1h_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalVolume24hTooLow => "gecko_vol24h_low".to_string(),
            FilterRejectionReason::GeckoTerminalVolume24hMissing => {
                "gecko_vol24h_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooLow => {
                "gecko_price_change_5m_low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh => {
                "gecko_price_change_5m_high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mMissing => {
                "gecko_price_change_5m_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooLow => {
                "gecko_price_change_1h_low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh => {
                "gecko_price_change_1h_high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hMissing => {
                "gecko_price_change_1h_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooLow => {
                "gecko_price_change_24h_low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh => {
                "gecko_price_change_24h_high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hMissing => {
                "gecko_price_change_24h_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooLow => {
                "gecko_pool_count_low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooHigh => {
                "gecko_pool_count_high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPoolCountMissing => {
                "gecko_pool_count_missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalReserveTooLow => "gecko_reserve_low".to_string(),
            FilterRejectionReason::GeckoTerminalReserveMissing => {
                "gecko_reserve_missing".to_string()
            }
            FilterRejectionReason::RugcheckRuggedToken => "rug_rugged".to_string(),
            FilterRejectionReason::RugcheckRiskScoreTooHigh => "rug_score".to_string(),
            FilterRejectionReason::RugcheckRiskLevelDanger => "rug_level_danger".to_string(),
            FilterRejectionReason::RugcheckMintAuthorityBlocked => "rug_mint_authority".to_string(),
            FilterRejectionReason::RugcheckFreezeAuthorityBlocked => {
                "rug_freeze_authority".to_string()
            }
            FilterRejectionReason::RugcheckTopHolderTooHigh => "rug_top_holder".to_string(),
            FilterRejectionReason::RugcheckTop3HoldersTooHigh => "rug_top3_holders".to_string(),
            FilterRejectionReason::RugcheckNotEnoughHolders => "rug_min_holders".to_string(),
            FilterRejectionReason::RugcheckInsiderHolderCount => "rug_insider_count".to_string(),
            FilterRejectionReason::RugcheckInsiderTotalPct => "rug_insider_pct".to_string(),
            FilterRejectionReason::RugcheckCreatorBalanceTooHigh => "rug_creator_pct".to_string(),
            FilterRejectionReason::RugcheckTransferFeePresent => {
                "rug_transfer_fee_present".to_string()
            }
            FilterRejectionReason::RugcheckTransferFeeTooHigh => {
                "rug_transfer_fee_high".to_string()
            }
            FilterRejectionReason::RugcheckTransferFeeMissing => {
                "rug_transfer_fee_missing".to_string()
            }
            FilterRejectionReason::RugcheckGraphInsidersTooHigh => "rug_graph_insiders".to_string(),
            FilterRejectionReason::RugcheckLpProvidersTooLow => "rug_lp_providers_low".to_string(),
            FilterRejectionReason::RugcheckLpProvidersMissing => {
                "rug_lp_providers_missing".to_string()
            }
            FilterRejectionReason::RugcheckLpLockTooLow => "rug_lp_lock_low".to_string(),
            FilterRejectionReason::RugcheckLpLockMissing => "rug_lp_lock_missing".to_string(),
        }
    }

    /// Human-readable display label for UI
    pub fn display_label(&self) -> String {
        match self {
            FilterRejectionReason::AiRejected {
                reason,
                confidence,
                provider,
            } => {
                format!(
                    "AI Rejected: {} ({}% conf, {})",
                    reason, confidence, provider
                )
            }
            FilterRejectionReason::NoDecimalsInDatabase => "No decimals in database".to_string(),
            FilterRejectionReason::TokenTooNew => "Token too new".to_string(),
            FilterRejectionReason::CooldownFiltered => "Cooldown filtered".to_string(),
            FilterRejectionReason::DexScreenerDataMissing => "DexScreener data missing".to_string(),
            FilterRejectionReason::GeckoTerminalDataMissing => {
                "GeckoTerminal data missing".to_string()
            }
            FilterRejectionReason::RugcheckDataMissing => "Rugcheck data missing".to_string(),
            FilterRejectionReason::DexScreenerEmptyName => "Empty name".to_string(),
            FilterRejectionReason::DexScreenerEmptySymbol => "Empty symbol".to_string(),
            FilterRejectionReason::DexScreenerEmptyLogoUrl => "Empty logo URL".to_string(),
            FilterRejectionReason::DexScreenerEmptyWebsiteUrl => "Empty website URL".to_string(),
            FilterRejectionReason::DexScreenerInsufficientTransactions5Min => {
                "Low 5m transactions".to_string()
            }
            FilterRejectionReason::DexScreenerInsufficientTransactions1H => {
                "Low 1h transactions".to_string()
            }
            FilterRejectionReason::DexScreenerZeroLiquidity => "Zero liquidity".to_string(),
            FilterRejectionReason::DexScreenerInsufficientLiquidity => {
                "Liquidity too low".to_string()
            }
            FilterRejectionReason::DexScreenerLiquidityTooHigh => "Liquidity too high".to_string(),
            FilterRejectionReason::DexScreenerMarketCapTooLow => "Market cap too low".to_string(),
            FilterRejectionReason::DexScreenerMarketCapTooHigh => "Market cap too high".to_string(),
            FilterRejectionReason::DexScreenerVolumeTooLow => "Volume too low".to_string(),
            FilterRejectionReason::DexScreenerVolumeMissing => "Volume missing".to_string(),
            FilterRejectionReason::DexScreenerFdvTooLow => "FDV too low".to_string(),
            FilterRejectionReason::DexScreenerFdvTooHigh => "FDV too high".to_string(),
            FilterRejectionReason::DexScreenerFdvMissing => "FDV missing".to_string(),
            FilterRejectionReason::DexScreenerVolume5mTooLow => "5m volume too low".to_string(),
            FilterRejectionReason::DexScreenerVolume5mMissing => "5m volume missing".to_string(),
            FilterRejectionReason::DexScreenerVolume1hTooLow => "1h volume too low".to_string(),
            FilterRejectionReason::DexScreenerVolume1hMissing => "1h volume missing".to_string(),
            FilterRejectionReason::DexScreenerVolume6hTooLow => "6h volume too low".to_string(),
            FilterRejectionReason::DexScreenerVolume6hMissing => "6h volume missing".to_string(),
            FilterRejectionReason::DexScreenerPriceChange5mTooLow => {
                "5m price change too low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange5mTooHigh => {
                "5m price change too high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange5mMissing => {
                "5m price change missing".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooLow => {
                "Price change too low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooHigh => {
                "Price change too high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChangeMissing => {
                "Price change missing".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooLow => {
                "6h price change too low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooHigh => {
                "6h price change too high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange6hMissing => {
                "6h price change missing".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooLow => {
                "24h price change too low".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooHigh => {
                "24h price change too high".to_string()
            }
            FilterRejectionReason::DexScreenerPriceChange24hMissing => {
                "24h price change missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalLiquidityMissing => "Liquidity missing".to_string(),
            FilterRejectionReason::GeckoTerminalLiquidityTooLow => "Liquidity too low".to_string(),
            FilterRejectionReason::GeckoTerminalLiquidityTooHigh => {
                "Liquidity too high".to_string()
            }
            FilterRejectionReason::GeckoTerminalMarketCapMissing => {
                "Market cap missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalMarketCapTooLow => "Market cap too low".to_string(),
            FilterRejectionReason::GeckoTerminalMarketCapTooHigh => {
                "Market cap too high".to_string()
            }
            FilterRejectionReason::GeckoTerminalVolume5mTooLow => "5m volume too low".to_string(),
            FilterRejectionReason::GeckoTerminalVolume5mMissing => "5m volume missing".to_string(),
            FilterRejectionReason::GeckoTerminalVolume1hTooLow => "1h volume too low".to_string(),
            FilterRejectionReason::GeckoTerminalVolume1hMissing => "1h volume missing".to_string(),
            FilterRejectionReason::GeckoTerminalVolume24hTooLow => "24h volume too low".to_string(),
            FilterRejectionReason::GeckoTerminalVolume24hMissing => {
                "24h volume missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooLow => {
                "5m price change too low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh => {
                "5m price change too high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mMissing => {
                "5m price change missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooLow => {
                "1h price change too low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh => {
                "1h price change too high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hMissing => {
                "1h price change missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooLow => {
                "24h price change too low".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh => {
                "24h price change too high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hMissing => {
                "24h price change missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooLow => "Pool count too low".to_string(),
            FilterRejectionReason::GeckoTerminalPoolCountTooHigh => {
                "Pool count too high".to_string()
            }
            FilterRejectionReason::GeckoTerminalPoolCountMissing => {
                "Pool count missing".to_string()
            }
            FilterRejectionReason::GeckoTerminalReserveTooLow => "Reserve too low".to_string(),
            FilterRejectionReason::GeckoTerminalReserveMissing => "Reserve missing".to_string(),
            FilterRejectionReason::RugcheckRuggedToken => "Rugged token".to_string(),
            FilterRejectionReason::RugcheckRiskScoreTooHigh => "Risk score too high".to_string(),
            FilterRejectionReason::RugcheckRiskLevelDanger => "Danger risk level".to_string(),
            FilterRejectionReason::RugcheckMintAuthorityBlocked => {
                "Mint authority present".to_string()
            }
            FilterRejectionReason::RugcheckFreezeAuthorityBlocked => {
                "Freeze authority present".to_string()
            }
            FilterRejectionReason::RugcheckTopHolderTooHigh => "Top holder % too high".to_string(),
            FilterRejectionReason::RugcheckTop3HoldersTooHigh => {
                "Top 3 holders % too high".to_string()
            }
            FilterRejectionReason::RugcheckNotEnoughHolders => "Not enough holders".to_string(),
            FilterRejectionReason::RugcheckInsiderHolderCount => {
                "Too many insider holders".to_string()
            }
            FilterRejectionReason::RugcheckInsiderTotalPct => "Insider % too high".to_string(),
            FilterRejectionReason::RugcheckCreatorBalanceTooHigh => {
                "Creator balance too high".to_string()
            }
            FilterRejectionReason::RugcheckTransferFeePresent => "Transfer fee present".to_string(),
            FilterRejectionReason::RugcheckTransferFeeTooHigh => {
                "Transfer fee too high".to_string()
            }
            FilterRejectionReason::RugcheckTransferFeeMissing => {
                "Transfer fee data missing".to_string()
            }
            FilterRejectionReason::RugcheckGraphInsidersTooHigh => {
                "Graph insiders too high".to_string()
            }
            FilterRejectionReason::RugcheckLpProvidersTooLow => "LP providers too low".to_string(),
            FilterRejectionReason::RugcheckLpProvidersMissing => "LP providers missing".to_string(),
            FilterRejectionReason::RugcheckLpLockTooLow => "LP lock too low".to_string(),
            FilterRejectionReason::RugcheckLpLockMissing => "LP lock missing".to_string(),
        }
    }

    /// Map rejection reason to source category for UI summaries.
    pub fn source(&self) -> FilterSource {
        match self {
            FilterRejectionReason::AiRejected { .. } => FilterSource::Ai,
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
