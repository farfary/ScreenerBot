//! Filter rejection reasons, their stored codes and their catalog text.
//!
//! One table maps each variant to its stable code (persisted in
//! `rejection_history.reason`, `rejection_stats.reason` and
//! `tokens.last_rejection_reason`) and to its catalog message. The same table
//! serves both directions: a variant produces its code, and a stored code is
//! parsed back to its variant for display.

use crate::i18n::{ids, MessageId, UiArg, UiText};

macro_rules! rejection_table {
    ($( $variant:ident => ($code:literal, $id:expr) ),+ $(,)?) => {
        /// Unified set of rejection reasons shared by all filtering sources.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum FilterRejectionReason {
            $( $variant, )+
            /// Rejection by the LLM analysis stage, carrying its reasoning.
            LlmAnalysisRejected {
                reason: String,
                confidence: u8,
                provider: String,
            },
        }

        impl FilterRejectionReason {
            /// Every variant that carries no data.
            pub const UNIT_VARIANTS: &'static [FilterRejectionReason] =
                &[ $( FilterRejectionReason::$variant, )+ ];

            /// Stable machine code persisted with rejections.
            pub fn label(&self) -> String {
                match self {
                    $( FilterRejectionReason::$variant => $code.to_owned(), )+
                    FilterRejectionReason::LlmAnalysisRejected { .. } => LLM_CODE.to_owned(),
                }
            }

            /// Catalog text for this reason.
            pub fn ui_text(&self) -> UiText {
                match self {
                    $( FilterRejectionReason::$variant => UiText::new($id), )+
                    FilterRejectionReason::LlmAnalysisRejected {
                        reason,
                        confidence,
                        provider,
                    } => UiText::new(ids::FILTERING_REJECT_LLM_ANALYSIS_REJECTED)
                        .arg("reason", UiArg::Text(reason.clone()))
                        .arg("confidence", UiArg::Count(i64::from(*confidence)))
                        .arg("provider", UiArg::Text(provider.clone())),
                }
            }

            /// Variant for a stored code. The LLM code maps to no variant: its
            /// data is not persisted with the code.
            fn from_code(code: &str) -> Option<FilterRejectionReason> {
                match code {
                    $( $code => Some(FilterRejectionReason::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

/// Stored code of [`FilterRejectionReason::LlmAnalysisRejected`].
const LLM_CODE: &str = "llm_analysis_rejected";

rejection_table! {
        NoDecimalsInDatabase => ("no_decimals", ids::FILTERING_REJECT_NO_DECIMALS),
        TokenTooNew => ("token_too_new", ids::FILTERING_REJECT_TOKEN_TOO_NEW),
        CooldownFiltered => ("cooldown_filtered", ids::FILTERING_REJECT_COOLDOWN_FILTERED),
        DexScreenerDataMissing => ("dex_data_missing", ids::FILTERING_REJECT_DEX_DATA_MISSING),
        GeckoTerminalDataMissing => ("gecko_data_missing", ids::FILTERING_REJECT_GECKO_DATA_MISSING),
        RugcheckDataMissing => ("rug_data_missing", ids::FILTERING_REJECT_RUG_DATA_MISSING),
        OnChainNumericSymbol => ("onchain_numeric_symbol", ids::FILTERING_REJECT_ONCHAIN_NUMERIC_SYMBOL),
        OnChainEmptySymbol => ("onchain_empty_symbol", ids::FILTERING_REJECT_ONCHAIN_EMPTY_SYMBOL),
        OnChainSuspiciousSymbol => ("onchain_suspicious_symbol", ids::FILTERING_REJECT_ONCHAIN_SUSPICIOUS_SYMBOL),
        OnChainKnownScamAuthority => ("onchain_known_scam_authority", ids::FILTERING_REJECT_ONCHAIN_KNOWN_SCAM_AUTHORITY),
        OnChainImmutableWithFreeze => ("onchain_immutable_with_freeze", ids::FILTERING_REJECT_ONCHAIN_IMMUTABLE_WITH_FREEZE),
        OnChainHighRiskScore => ("onchain_high_risk_score", ids::FILTERING_REJECT_ONCHAIN_HIGH_RISK_SCORE),
        DexScreenerEmptyName => ("dex_empty_name", ids::FILTERING_REJECT_DEX_EMPTY_NAME),
        DexScreenerEmptySymbol => ("dex_empty_symbol", ids::FILTERING_REJECT_DEX_EMPTY_SYMBOL),
        DexScreenerEmptyLogoUrl => ("dex_empty_logo", ids::FILTERING_REJECT_DEX_EMPTY_LOGO),
        DexScreenerEmptyWebsiteUrl => ("dex_empty_website", ids::FILTERING_REJECT_DEX_EMPTY_WEBSITE),
        DexScreenerInsufficientTransactions5Min => ("dex_txn_5m", ids::FILTERING_REJECT_DEX_TXN_5M),
        DexScreenerInsufficientTransactions1H => ("dex_txn_1h", ids::FILTERING_REJECT_DEX_TXN_1H),
        DexScreenerZeroLiquidity => ("dex_zero_liq", ids::FILTERING_REJECT_DEX_ZERO_LIQ),
        DexScreenerInsufficientLiquidity => ("dex_liq_low", ids::FILTERING_REJECT_DEX_LIQ_LOW),
        DexScreenerLiquidityTooHigh => ("dex_liq_high", ids::FILTERING_REJECT_DEX_LIQ_HIGH),
        DexScreenerMarketCapTooLow => ("dex_mcap_low", ids::FILTERING_REJECT_DEX_MCAP_LOW),
        DexScreenerMarketCapTooHigh => ("dex_mcap_high", ids::FILTERING_REJECT_DEX_MCAP_HIGH),
        DexScreenerVolumeTooLow => ("dex_vol_low", ids::FILTERING_REJECT_DEX_VOL_LOW),
        DexScreenerVolumeMissing => ("dex_vol_missing", ids::FILTERING_REJECT_DEX_VOL_MISSING),
        DexScreenerFdvTooLow => ("dex_fdv_low", ids::FILTERING_REJECT_DEX_FDV_LOW),
        DexScreenerFdvTooHigh => ("dex_fdv_high", ids::FILTERING_REJECT_DEX_FDV_HIGH),
        DexScreenerVolume5mTooLow => ("dex_vol5m_low", ids::FILTERING_REJECT_DEX_VOL5M_LOW),
        DexScreenerVolume5mMissing => ("dex_vol5m_missing", ids::FILTERING_REJECT_DEX_VOL5M_MISSING),
        DexScreenerVolume1hTooLow => ("dex_vol1h_low", ids::FILTERING_REJECT_DEX_VOL1H_LOW),
        DexScreenerVolume1hMissing => ("dex_vol1h_missing", ids::FILTERING_REJECT_DEX_VOL1H_MISSING),
        DexScreenerVolume6hTooLow => ("dex_vol6h_low", ids::FILTERING_REJECT_DEX_VOL6H_LOW),
        DexScreenerVolume6hMissing => ("dex_vol6h_missing", ids::FILTERING_REJECT_DEX_VOL6H_MISSING),
        DexScreenerPriceChange5mTooLow => ("dex_price_change_5m_low", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_5M_LOW),
        DexScreenerPriceChange5mTooHigh => ("dex_price_change_5m_high", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_5M_HIGH),
        DexScreenerPriceChangeTooLow => ("dex_price_change_low", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_LOW),
        DexScreenerPriceChangeTooHigh => ("dex_price_change_high", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_HIGH),
        DexScreenerPriceChange6hTooLow => ("dex_price_change_6h_low", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_6H_LOW),
        DexScreenerPriceChange6hTooHigh => ("dex_price_change_6h_high", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_6H_HIGH),
        DexScreenerPriceChange24hTooLow => ("dex_price_change_24h_low", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_24H_LOW),
        DexScreenerPriceChange24hTooHigh => ("dex_price_change_24h_high", ids::FILTERING_REJECT_DEX_PRICE_CHANGE_24H_HIGH),
        GeckoTerminalLiquidityTooLow => ("gecko_liq_low", ids::FILTERING_REJECT_GECKO_LIQ_LOW),
        GeckoTerminalLiquidityTooHigh => ("gecko_liq_high", ids::FILTERING_REJECT_GECKO_LIQ_HIGH),
        GeckoTerminalMarketCapTooLow => ("gecko_mcap_low", ids::FILTERING_REJECT_GECKO_MCAP_LOW),
        GeckoTerminalMarketCapTooHigh => ("gecko_mcap_high", ids::FILTERING_REJECT_GECKO_MCAP_HIGH),
        GeckoTerminalVolume5mTooLow => ("gecko_vol5m_low", ids::FILTERING_REJECT_GECKO_VOL5M_LOW),
        GeckoTerminalVolume5mMissing => ("gecko_vol5m_missing", ids::FILTERING_REJECT_GECKO_VOL5M_MISSING),
        GeckoTerminalVolume1hTooLow => ("gecko_vol1h_low", ids::FILTERING_REJECT_GECKO_VOL1H_LOW),
        GeckoTerminalVolume1hMissing => ("gecko_vol1h_missing", ids::FILTERING_REJECT_GECKO_VOL1H_MISSING),
        GeckoTerminalVolume24hTooLow => ("gecko_vol24h_low", ids::FILTERING_REJECT_GECKO_VOL24H_LOW),
        GeckoTerminalVolume24hMissing => ("gecko_vol24h_missing", ids::FILTERING_REJECT_GECKO_VOL24H_MISSING),
        GeckoTerminalPriceChange5mTooLow => ("gecko_price_change_5m_low", ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_5M_LOW),
        GeckoTerminalPriceChange5mTooHigh => ("gecko_price_change_5m_high", ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_5M_HIGH),
        GeckoTerminalPriceChange1hTooLow => ("gecko_price_change_1h_low", ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_1H_LOW),
        GeckoTerminalPriceChange1hTooHigh => ("gecko_price_change_1h_high", ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_1H_HIGH),
        GeckoTerminalPriceChange24hTooLow => ("gecko_price_change_24h_low", ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_24H_LOW),
        GeckoTerminalPriceChange24hTooHigh => ("gecko_price_change_24h_high", ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_24H_HIGH),
        GeckoTerminalPoolCountTooLow => ("gecko_pool_count_low", ids::FILTERING_REJECT_GECKO_POOL_COUNT_LOW),
        GeckoTerminalPoolCountTooHigh => ("gecko_pool_count_high", ids::FILTERING_REJECT_GECKO_POOL_COUNT_HIGH),
        GeckoTerminalPoolCountMissing => ("gecko_pool_count_missing", ids::FILTERING_REJECT_GECKO_POOL_COUNT_MISSING),
        GeckoTerminalReserveTooLow => ("gecko_reserve_low", ids::FILTERING_REJECT_GECKO_RESERVE_LOW),
        GeckoTerminalReserveMissing => ("gecko_reserve_missing", ids::FILTERING_REJECT_GECKO_RESERVE_MISSING),
        RugcheckRuggedToken => ("rug_rugged", ids::FILTERING_REJECT_RUG_RUGGED),
        RugcheckRiskScoreTooHigh => ("rug_score", ids::FILTERING_REJECT_RUG_SCORE),
        RugcheckRiskLevelDanger => ("rug_level_danger", ids::FILTERING_REJECT_RUG_LEVEL_DANGER),
        RugcheckMintAuthorityBlocked => ("rug_mint_authority", ids::FILTERING_REJECT_RUG_MINT_AUTHORITY),
        RugcheckFreezeAuthorityBlocked => ("rug_freeze_authority", ids::FILTERING_REJECT_RUG_FREEZE_AUTHORITY),
        RugcheckTopHolderTooHigh => ("rug_top_holder", ids::FILTERING_REJECT_RUG_TOP_HOLDER),
        RugcheckTop3HoldersTooHigh => ("rug_top3_holders", ids::FILTERING_REJECT_RUG_TOP3_HOLDERS),
        RugcheckNotEnoughHolders => ("rug_min_holders", ids::FILTERING_REJECT_RUG_MIN_HOLDERS),
        RugcheckInsiderHolderCount => ("rug_insider_count", ids::FILTERING_REJECT_RUG_INSIDER_COUNT),
        RugcheckInsiderTotalPct => ("rug_insider_pct", ids::FILTERING_REJECT_RUG_INSIDER_PCT),
        RugcheckCreatorBalanceTooHigh => ("rug_creator_pct", ids::FILTERING_REJECT_RUG_CREATOR_PCT),
        RugcheckTransferFeePresent => ("rug_transfer_fee_present", ids::FILTERING_REJECT_RUG_TRANSFER_FEE_PRESENT),
        RugcheckTransferFeeTooHigh => ("rug_transfer_fee_high", ids::FILTERING_REJECT_RUG_TRANSFER_FEE_HIGH),
        RugcheckGraphInsidersTooHigh => ("rug_graph_insiders", ids::FILTERING_REJECT_RUG_GRAPH_INSIDERS),
        RugcheckLpProvidersTooLow => ("rug_lp_providers_low", ids::FILTERING_REJECT_RUG_LP_PROVIDERS_LOW),
        RugcheckLpProvidersMissing => ("rug_lp_providers_missing", ids::FILTERING_REJECT_RUG_LP_PROVIDERS_MISSING),
        RugcheckLpLockTooLow => ("rug_lp_lock_low", ids::FILTERING_REJECT_RUG_LP_LOCK_LOW),
        RugcheckLpLockMissing => ("rug_lp_lock_missing", ids::FILTERING_REJECT_RUG_LP_LOCK_MISSING),
}

/// Codes that no current variant emits. They appear only in stored rows
/// (`rejection_history`, `rejection_stats`, `tokens.last_rejection_reason`) and
/// keep the wording they were displayed with.
pub const LEGACY_REJECTION_CODES: &[(&str, MessageId)] = &[
    ("dex_fdv_missing", ids::FILTERING_REJECT_DEX_FDV_MISSING),
    (
        "dex_price_change_5m_missing",
        ids::FILTERING_REJECT_DEX_PRICE_CHANGE_5M_MISSING,
    ),
    (
        "dex_price_change_missing",
        ids::FILTERING_REJECT_DEX_PRICE_CHANGE_MISSING,
    ),
    (
        "dex_price_change_6h_missing",
        ids::FILTERING_REJECT_DEX_PRICE_CHANGE_6H_MISSING,
    ),
    (
        "dex_price_change_24h_missing",
        ids::FILTERING_REJECT_DEX_PRICE_CHANGE_24H_MISSING,
    ),
    ("gecko_liq_missing", ids::FILTERING_REJECT_GECKO_LIQ_MISSING),
    (
        "gecko_mcap_missing",
        ids::FILTERING_REJECT_GECKO_MCAP_MISSING,
    ),
    (
        "gecko_price_change_5m_missing",
        ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_5M_MISSING,
    ),
    (
        "gecko_price_change_1h_missing",
        ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_1H_MISSING,
    ),
    (
        "gecko_price_change_24h_missing",
        ids::FILTERING_REJECT_GECKO_PRICE_CHANGE_24H_MISSING,
    ),
    (
        "rug_transfer_fee_missing",
        ids::FILTERING_REJECT_RUG_TRANSFER_FEE_MISSING,
    ),
];

/// Catalog text for a stored rejection code. The LLM code uses its no-argument
/// message because rows persist only the code. Live codes resolve first, then
/// legacy codes; anything else renders as the code itself.
pub fn rejection_text(code: &str) -> UiText {
    if code == LLM_CODE {
        return UiText::new(ids::FILTERING_REJECT_LLM_ANALYSIS_REJECTED_GENERIC);
    }
    match FilterRejectionReason::from_code(code) {
        Some(reason) => reason.ui_text(),
        None => match LEGACY_REJECTION_CODES
            .iter()
            .find(|(legacy, _)| *legacy == code)
        {
            Some((_, id)) => UiText::new(*id),
            None => {
                UiText::new(ids::FILTERING_REJECT_UNKNOWN).arg("code", UiArg::Text(code.to_owned()))
            }
        },
    }
}

#[cfg(test)]
mod tests;
