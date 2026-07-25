//! Token data sources and rejection origins for the filtering pipeline.
use std::fmt;

pub mod ai;
pub(super) mod bounds;
pub mod dexscreener;
pub mod geckoterminal;
pub mod meta;
pub mod onchain;
pub mod rugcheck;

/// High level origin for a filtering rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterSource {
    Core,
    OnChain,
    DexScreener,
    GeckoTerminal,
    Rugcheck,
    Ai,
}

impl FilterSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterSource::Core => "core",
            FilterSource::OnChain => "onchain",
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

    // On-chain scam detection (no external API needed)
    OnChainNumericSymbol,
    OnChainEmptySymbol,
    OnChainSuspiciousSymbol,
    OnChainKnownScamAuthority,
    OnChainImmutableWithFreeze,
    OnChainHighRiskScore,

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
    DexScreenerVolume5mTooLow,
    DexScreenerVolume5mMissing,
    DexScreenerVolume1hTooLow,
    DexScreenerVolume1hMissing,
    DexScreenerVolume6hTooLow,
    DexScreenerVolume6hMissing,
    DexScreenerPriceChange5mTooLow,
    DexScreenerPriceChange5mTooHigh,
    DexScreenerPriceChangeTooLow,
    DexScreenerPriceChangeTooHigh,
    DexScreenerPriceChange6hTooLow,
    DexScreenerPriceChange6hTooHigh,
    DexScreenerPriceChange24hTooLow,
    DexScreenerPriceChange24hTooHigh,

    // GeckoTerminal
    GeckoTerminalLiquidityTooLow,
    GeckoTerminalLiquidityTooHigh,
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
    GeckoTerminalPriceChange1hTooLow,
    GeckoTerminalPriceChange1hTooHigh,
    GeckoTerminalPriceChange24hTooLow,
    GeckoTerminalPriceChange24hTooHigh,
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
            FilterRejectionReason::NoDecimalsInDatabase => "no_decimals".to_owned(),
            FilterRejectionReason::TokenTooNew => "token_too_new".to_owned(),
            FilterRejectionReason::CooldownFiltered => "cooldown_filtered".to_owned(),
            FilterRejectionReason::DexScreenerDataMissing => "dex_data_missing".to_owned(),
            FilterRejectionReason::GeckoTerminalDataMissing => "gecko_data_missing".to_owned(),
            FilterRejectionReason::RugcheckDataMissing => "rug_data_missing".to_owned(),
            FilterRejectionReason::OnChainNumericSymbol => "onchain_numeric_symbol".to_owned(),
            FilterRejectionReason::OnChainEmptySymbol => "onchain_empty_symbol".to_owned(),
            FilterRejectionReason::OnChainSuspiciousSymbol => {
                "onchain_suspicious_symbol".to_owned()
            }
            FilterRejectionReason::OnChainKnownScamAuthority => {
                "onchain_known_scam_authority".to_owned()
            }
            FilterRejectionReason::OnChainImmutableWithFreeze => {
                "onchain_immutable_with_freeze".to_owned()
            }
            FilterRejectionReason::OnChainHighRiskScore => "onchain_high_risk_score".to_owned(),
            FilterRejectionReason::AiRejected { .. } => "ai_rejected".to_owned(),
            FilterRejectionReason::DexScreenerEmptyName => "dex_empty_name".to_owned(),
            FilterRejectionReason::DexScreenerEmptySymbol => "dex_empty_symbol".to_owned(),
            FilterRejectionReason::DexScreenerEmptyLogoUrl => "dex_empty_logo".to_owned(),
            FilterRejectionReason::DexScreenerEmptyWebsiteUrl => "dex_empty_website".to_owned(),
            FilterRejectionReason::DexScreenerInsufficientTransactions5Min => {
                "dex_txn_5m".to_owned()
            }
            FilterRejectionReason::DexScreenerInsufficientTransactions1H => "dex_txn_1h".to_owned(),
            FilterRejectionReason::DexScreenerZeroLiquidity => "dex_zero_liq".to_owned(),
            FilterRejectionReason::DexScreenerInsufficientLiquidity => "dex_liq_low".to_owned(),
            FilterRejectionReason::DexScreenerLiquidityTooHigh => "dex_liq_high".to_owned(),
            FilterRejectionReason::DexScreenerMarketCapTooLow => "dex_mcap_low".to_owned(),
            FilterRejectionReason::DexScreenerMarketCapTooHigh => "dex_mcap_high".to_owned(),
            FilterRejectionReason::DexScreenerVolumeTooLow => "dex_vol_low".to_owned(),
            FilterRejectionReason::DexScreenerVolumeMissing => "dex_vol_missing".to_owned(),
            FilterRejectionReason::DexScreenerFdvTooLow => "dex_fdv_low".to_owned(),
            FilterRejectionReason::DexScreenerFdvTooHigh => "dex_fdv_high".to_owned(),
            FilterRejectionReason::DexScreenerVolume5mTooLow => "dex_vol5m_low".to_owned(),
            FilterRejectionReason::DexScreenerVolume5mMissing => "dex_vol5m_missing".to_owned(),
            FilterRejectionReason::DexScreenerVolume1hTooLow => "dex_vol1h_low".to_owned(),
            FilterRejectionReason::DexScreenerVolume1hMissing => "dex_vol1h_missing".to_owned(),
            FilterRejectionReason::DexScreenerVolume6hTooLow => "dex_vol6h_low".to_owned(),
            FilterRejectionReason::DexScreenerVolume6hMissing => "dex_vol6h_missing".to_owned(),
            FilterRejectionReason::DexScreenerPriceChange5mTooLow => {
                "dex_price_change_5m_low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange5mTooHigh => {
                "dex_price_change_5m_high".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooLow => {
                "dex_price_change_low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooHigh => {
                "dex_price_change_high".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooLow => {
                "dex_price_change_6h_low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooHigh => {
                "dex_price_change_6h_high".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooLow => {
                "dex_price_change_24h_low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooHigh => {
                "dex_price_change_24h_high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalLiquidityTooLow => "gecko_liq_low".to_owned(),
            FilterRejectionReason::GeckoTerminalLiquidityTooHigh => "gecko_liq_high".to_owned(),
            FilterRejectionReason::GeckoTerminalMarketCapTooLow => "gecko_mcap_low".to_owned(),
            FilterRejectionReason::GeckoTerminalMarketCapTooHigh => "gecko_mcap_high".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume5mTooLow => "gecko_vol5m_low".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume5mMissing => "gecko_vol5m_missing".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume1hTooLow => "gecko_vol1h_low".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume1hMissing => "gecko_vol1h_missing".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume24hTooLow => "gecko_vol24h_low".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume24hMissing => {
                "gecko_vol24h_missing".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooLow => {
                "gecko_price_change_5m_low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh => {
                "gecko_price_change_5m_high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooLow => {
                "gecko_price_change_1h_low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh => {
                "gecko_price_change_1h_high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooLow => {
                "gecko_price_change_24h_low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh => {
                "gecko_price_change_24h_high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooLow => {
                "gecko_pool_count_low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooHigh => {
                "gecko_pool_count_high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPoolCountMissing => {
                "gecko_pool_count_missing".to_owned()
            }
            FilterRejectionReason::GeckoTerminalReserveTooLow => "gecko_reserve_low".to_owned(),
            FilterRejectionReason::GeckoTerminalReserveMissing => {
                "gecko_reserve_missing".to_owned()
            }
            FilterRejectionReason::RugcheckRuggedToken => "rug_rugged".to_owned(),
            FilterRejectionReason::RugcheckRiskScoreTooHigh => "rug_score".to_owned(),
            FilterRejectionReason::RugcheckRiskLevelDanger => "rug_level_danger".to_owned(),
            FilterRejectionReason::RugcheckMintAuthorityBlocked => "rug_mint_authority".to_owned(),
            FilterRejectionReason::RugcheckFreezeAuthorityBlocked => {
                "rug_freeze_authority".to_owned()
            }
            FilterRejectionReason::RugcheckTopHolderTooHigh => "rug_top_holder".to_owned(),
            FilterRejectionReason::RugcheckTop3HoldersTooHigh => "rug_top3_holders".to_owned(),
            FilterRejectionReason::RugcheckNotEnoughHolders => "rug_min_holders".to_owned(),
            FilterRejectionReason::RugcheckInsiderHolderCount => "rug_insider_count".to_owned(),
            FilterRejectionReason::RugcheckInsiderTotalPct => "rug_insider_pct".to_owned(),
            FilterRejectionReason::RugcheckCreatorBalanceTooHigh => "rug_creator_pct".to_owned(),
            FilterRejectionReason::RugcheckTransferFeePresent => {
                "rug_transfer_fee_present".to_owned()
            }
            FilterRejectionReason::RugcheckTransferFeeTooHigh => "rug_transfer_fee_high".to_owned(),
            FilterRejectionReason::RugcheckGraphInsidersTooHigh => "rug_graph_insiders".to_owned(),
            FilterRejectionReason::RugcheckLpProvidersTooLow => "rug_lp_providers_low".to_owned(),
            FilterRejectionReason::RugcheckLpProvidersMissing => {
                "rug_lp_providers_missing".to_owned()
            }
            FilterRejectionReason::RugcheckLpLockTooLow => "rug_lp_lock_low".to_owned(),
            FilterRejectionReason::RugcheckLpLockMissing => "rug_lp_lock_missing".to_owned(),
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
            FilterRejectionReason::NoDecimalsInDatabase => "No decimals in database".to_owned(),
            FilterRejectionReason::TokenTooNew => "Token too new".to_owned(),
            FilterRejectionReason::CooldownFiltered => "Cooldown filtered".to_owned(),
            FilterRejectionReason::DexScreenerDataMissing => "DexScreener data missing".to_owned(),
            FilterRejectionReason::GeckoTerminalDataMissing => {
                "GeckoTerminal data missing".to_owned()
            }
            FilterRejectionReason::RugcheckDataMissing => "Rugcheck data missing".to_owned(),
            FilterRejectionReason::OnChainNumericSymbol => "Numeric-only symbol (scam)".to_owned(),
            FilterRejectionReason::OnChainEmptySymbol => "Empty symbol (scam)".to_owned(),
            FilterRejectionReason::OnChainSuspiciousSymbol => "Suspicious symbol (scam)".to_owned(),
            FilterRejectionReason::OnChainKnownScamAuthority => "Known scam authority".to_owned(),
            FilterRejectionReason::OnChainImmutableWithFreeze => {
                "Immutable + freeze authority (scam)".to_owned()
            }
            FilterRejectionReason::OnChainHighRiskScore => "On-chain high risk score".to_owned(),
            FilterRejectionReason::DexScreenerEmptyName => "Empty name".to_owned(),
            FilterRejectionReason::DexScreenerEmptySymbol => "Empty symbol".to_owned(),
            FilterRejectionReason::DexScreenerEmptyLogoUrl => "Empty logo URL".to_owned(),
            FilterRejectionReason::DexScreenerEmptyWebsiteUrl => "Empty website URL".to_owned(),
            FilterRejectionReason::DexScreenerInsufficientTransactions5Min => {
                "Low 5m transactions".to_owned()
            }
            FilterRejectionReason::DexScreenerInsufficientTransactions1H => {
                "Low 1h transactions".to_owned()
            }
            FilterRejectionReason::DexScreenerZeroLiquidity => "Zero liquidity".to_owned(),
            FilterRejectionReason::DexScreenerInsufficientLiquidity => {
                "Liquidity too low".to_owned()
            }
            FilterRejectionReason::DexScreenerLiquidityTooHigh => "Liquidity too high".to_owned(),
            FilterRejectionReason::DexScreenerMarketCapTooLow => "Market cap too low".to_owned(),
            FilterRejectionReason::DexScreenerMarketCapTooHigh => "Market cap too high".to_owned(),
            FilterRejectionReason::DexScreenerVolumeTooLow => "Volume too low".to_owned(),
            FilterRejectionReason::DexScreenerVolumeMissing => "Volume missing".to_owned(),
            FilterRejectionReason::DexScreenerFdvTooLow => "FDV too low".to_owned(),
            FilterRejectionReason::DexScreenerFdvTooHigh => "FDV too high".to_owned(),
            FilterRejectionReason::DexScreenerVolume5mTooLow => "5m volume too low".to_owned(),
            FilterRejectionReason::DexScreenerVolume5mMissing => "5m volume missing".to_owned(),
            FilterRejectionReason::DexScreenerVolume1hTooLow => "1h volume too low".to_owned(),
            FilterRejectionReason::DexScreenerVolume1hMissing => "1h volume missing".to_owned(),
            FilterRejectionReason::DexScreenerVolume6hTooLow => "6h volume too low".to_owned(),
            FilterRejectionReason::DexScreenerVolume6hMissing => "6h volume missing".to_owned(),
            FilterRejectionReason::DexScreenerPriceChange5mTooLow => {
                "5m price change too low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange5mTooHigh => {
                "5m price change too high".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooLow => {
                "Price change too low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChangeTooHigh => {
                "Price change too high".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooLow => {
                "6h price change too low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange6hTooHigh => {
                "6h price change too high".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooLow => {
                "24h price change too low".to_owned()
            }
            FilterRejectionReason::DexScreenerPriceChange24hTooHigh => {
                "24h price change too high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalLiquidityTooLow => "Liquidity too low".to_owned(),
            FilterRejectionReason::GeckoTerminalLiquidityTooHigh => "Liquidity too high".to_owned(),
            FilterRejectionReason::GeckoTerminalMarketCapTooLow => "Market cap too low".to_owned(),
            FilterRejectionReason::GeckoTerminalMarketCapTooHigh => {
                "Market cap too high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalVolume5mTooLow => "5m volume too low".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume5mMissing => "5m volume missing".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume1hTooLow => "1h volume too low".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume1hMissing => "1h volume missing".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume24hTooLow => "24h volume too low".to_owned(),
            FilterRejectionReason::GeckoTerminalVolume24hMissing => "24h volume missing".to_owned(),
            FilterRejectionReason::GeckoTerminalPriceChange5mTooLow => {
                "5m price change too low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange5mTooHigh => {
                "5m price change too high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooLow => {
                "1h price change too low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh => {
                "1h price change too high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooLow => {
                "24h price change too low".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh => {
                "24h price change too high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPoolCountTooLow => "Pool count too low".to_owned(),
            FilterRejectionReason::GeckoTerminalPoolCountTooHigh => {
                "Pool count too high".to_owned()
            }
            FilterRejectionReason::GeckoTerminalPoolCountMissing => "Pool count missing".to_owned(),
            FilterRejectionReason::GeckoTerminalReserveTooLow => "Reserve too low".to_owned(),
            FilterRejectionReason::GeckoTerminalReserveMissing => "Reserve missing".to_owned(),
            FilterRejectionReason::RugcheckRuggedToken => "Rugged token".to_owned(),
            FilterRejectionReason::RugcheckRiskScoreTooHigh => "Risk score too high".to_owned(),
            FilterRejectionReason::RugcheckRiskLevelDanger => "Danger risk level".to_owned(),
            FilterRejectionReason::RugcheckMintAuthorityBlocked => {
                "Mint authority present".to_owned()
            }
            FilterRejectionReason::RugcheckFreezeAuthorityBlocked => {
                "Freeze authority present".to_owned()
            }
            FilterRejectionReason::RugcheckTopHolderTooHigh => "Top holder % too high".to_owned(),
            FilterRejectionReason::RugcheckTop3HoldersTooHigh => {
                "Top 3 holders % too high".to_owned()
            }
            FilterRejectionReason::RugcheckNotEnoughHolders => "Not enough holders".to_owned(),
            FilterRejectionReason::RugcheckInsiderHolderCount => {
                "Too many insider holders".to_owned()
            }
            FilterRejectionReason::RugcheckInsiderTotalPct => "Insider % too high".to_owned(),
            FilterRejectionReason::RugcheckCreatorBalanceTooHigh => {
                "Creator balance too high".to_owned()
            }
            FilterRejectionReason::RugcheckTransferFeePresent => "Transfer fee present".to_owned(),
            FilterRejectionReason::RugcheckTransferFeeTooHigh => "Transfer fee too high".to_owned(),
            FilterRejectionReason::RugcheckGraphInsidersTooHigh => {
                "Graph insiders too high".to_owned()
            }
            FilterRejectionReason::RugcheckLpProvidersTooLow => "LP providers too low".to_owned(),
            FilterRejectionReason::RugcheckLpProvidersMissing => "LP providers missing".to_owned(),
            FilterRejectionReason::RugcheckLpLockTooLow => "LP lock too low".to_owned(),
            FilterRejectionReason::RugcheckLpLockMissing => "LP lock missing".to_owned(),
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
            FilterRejectionReason::OnChainNumericSymbol
            | FilterRejectionReason::OnChainEmptySymbol
            | FilterRejectionReason::OnChainSuspiciousSymbol
            | FilterRejectionReason::OnChainKnownScamAuthority
            | FilterRejectionReason::OnChainImmutableWithFreeze
            | FilterRejectionReason::OnChainHighRiskScore => FilterSource::OnChain,
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
            | FilterRejectionReason::DexScreenerPriceChange5mTooLow
            | FilterRejectionReason::DexScreenerPriceChange5mTooHigh
            | FilterRejectionReason::DexScreenerPriceChange6hTooLow
            | FilterRejectionReason::DexScreenerPriceChange6hTooHigh
            | FilterRejectionReason::DexScreenerPriceChange24hTooLow
            | FilterRejectionReason::DexScreenerPriceChange24hTooHigh => FilterSource::DexScreener,
            FilterRejectionReason::GeckoTerminalLiquidityTooLow
            | FilterRejectionReason::GeckoTerminalLiquidityTooHigh
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
            | FilterRejectionReason::GeckoTerminalPriceChange1hTooLow
            | FilterRejectionReason::GeckoTerminalPriceChange1hTooHigh
            | FilterRejectionReason::GeckoTerminalPriceChange24hTooLow
            | FilterRejectionReason::GeckoTerminalPriceChange24hTooHigh
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
