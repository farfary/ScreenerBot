//! Token data sources and rejection origins for the filtering pipeline.
use std::fmt;

pub(super) mod bounds;
pub mod dexscreener;
pub mod geckoterminal;
pub mod llm_analysis;
pub mod meta;
pub mod onchain;
pub mod rejection;
pub mod rugcheck;

pub use rejection::{rejection_text, FilterRejectionReason};

/// High level origin for a filtering rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterSource {
    Core,
    OnChain,
    DexScreener,
    GeckoTerminal,
    Rugcheck,
    LlmAnalysis,
}

impl FilterSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterSource::Core => "core",
            FilterSource::OnChain => "onchain",
            FilterSource::DexScreener => "dexscreener",
            FilterSource::GeckoTerminal => "geckoterminal",
            FilterSource::Rugcheck => "rugcheck",
            FilterSource::LlmAnalysis => "llm_analysis",
        }
    }
}

impl FilterRejectionReason {
    /// Map rejection reason to source category for UI summaries.
    pub fn source(&self) -> FilterSource {
        match self {
            FilterRejectionReason::LlmAnalysisRejected { .. } => FilterSource::LlmAnalysis,
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
