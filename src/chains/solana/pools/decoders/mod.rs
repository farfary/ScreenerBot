//! Pool decoders module
//!
//! This module contains program-specific decoders for different DEX pool types.
//! Each decoder knows how to parse the account data for its specific pool format.

pub mod fluxbeam_amm;
pub mod meteora_damm;
pub mod meteora_dbc;
pub mod meteora_dlmm;
pub mod moonit_amm;
pub mod orca_whirlpool;
pub mod pumpfun_amm;
pub mod pumpfun_legacy;
pub mod raydium_clmm;
pub mod raydium_cpmm;
pub mod raydium_legacy_amm;

pub use raydium_cpmm::{RaydiumCpmmDecoder, RaydiumCpmmPoolInfo};

use super::fetcher::AccountData;
use super::types::ProgramKind;
use crate::pools::types::PriceResult;
use std::collections::HashMap;

/// Trait for pool decoders
pub trait PoolDecoder {
    /// Get the program kinds this decoder supports
    fn supported_programs() -> Vec<ProgramKind>;

    /// Decode pool data and calculate price
    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult>;
}

/// Main decoder dispatch function
pub fn decode_pool(
    program_kind: ProgramKind,
    accounts: &HashMap<String, AccountData>,
    base_mint: &str,
    quote_mint: &str,
) -> Option<PriceResult> {
    match program_kind {
        ProgramKind::RaydiumCpmm => {
            raydium_cpmm::RaydiumCpmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::RaydiumClmm => {
            raydium_clmm::RaydiumClmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::PumpFunAmm => {
            pumpfun_amm::PumpFunAmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::PumpFunLegacy => pumpfun_legacy::PumpFunLegacyDecoder::decode_and_calculate(
            accounts, base_mint, quote_mint,
        ),
        ProgramKind::RaydiumLegacyAmm => {
            raydium_legacy_amm::RaydiumLegacyAmmDecoder::decode_and_calculate(
                accounts, base_mint, quote_mint,
            )
        }
        ProgramKind::MeteoraDlmm => {
            meteora_dlmm::MeteoraDlmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::MeteoraDamm => {
            meteora_damm::MeteoraDammDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::MeteoraDbc => {
            meteora_dbc::MeteoraDbcDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::OrcaWhirlpool => orca_whirlpool::OrcaWhirlpoolDecoder::decode_and_calculate(
            accounts, base_mint, quote_mint,
        ),
        ProgramKind::Moonit => {
            moonit_amm::MoonitAmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::FluxbeamAmm => {
            fluxbeam_amm::FluxbeamAmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        _ => {
            // TODO: Add other decoders as needed
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pools::types::ProtocolId;

    /// Every canonical `ProtocolId` slug must resolve, through the real
    /// `ProgramKind::from_protocol_id` boundary, to the decoder family that
    /// actually declares support for it — proving decoder selection still
    /// reaches the intended known family after the slug/display-name split.
    #[test]
    fn from_protocol_id_boundary_reaches_the_intended_decoder_family() {
        let cases: [(ProgramKind, Vec<ProgramKind>); 11] = [
            (
                ProgramKind::RaydiumCpmm,
                raydium_cpmm::RaydiumCpmmDecoder::supported_programs(),
            ),
            (
                ProgramKind::RaydiumLegacyAmm,
                raydium_legacy_amm::RaydiumLegacyAmmDecoder::supported_programs(),
            ),
            (
                ProgramKind::RaydiumClmm,
                raydium_clmm::RaydiumClmmDecoder::supported_programs(),
            ),
            (
                ProgramKind::OrcaWhirlpool,
                orca_whirlpool::OrcaWhirlpoolDecoder::supported_programs(),
            ),
            (
                ProgramKind::MeteoraDamm,
                meteora_damm::MeteoraDammDecoder::supported_programs(),
            ),
            (
                ProgramKind::MeteoraDlmm,
                meteora_dlmm::MeteoraDlmmDecoder::supported_programs(),
            ),
            (
                ProgramKind::MeteoraDbc,
                meteora_dbc::MeteoraDbcDecoder::supported_programs(),
            ),
            (
                ProgramKind::PumpFunAmm,
                pumpfun_amm::PumpFunAmmDecoder::supported_programs(),
            ),
            (
                ProgramKind::PumpFunLegacy,
                pumpfun_legacy::PumpFunLegacyDecoder::supported_programs(),
            ),
            (
                ProgramKind::Moonit,
                moonit_amm::MoonitAmmDecoder::supported_programs(),
            ),
            (
                ProgramKind::FluxbeamAmm,
                fluxbeam_amm::FluxbeamAmmDecoder::supported_programs(),
            ),
        ];

        for (kind, supported_by_owning_decoder) in cases {
            let slug_id = kind.protocol_id();
            assert_eq!(
                ProgramKind::from_protocol_id(&slug_id),
                kind,
                "canonical slug for {kind:?} must route back to itself"
            );
            assert!(
                supported_by_owning_decoder.contains(&kind),
                "{kind:?}'s own decoder module must declare support for it"
            );

            // The exact historical display-name identity must dispatch to
            // the same decoder family during the compatibility window.
            let legacy_id = ProtocolId::new(kind.display_name());
            assert_eq!(ProgramKind::from_protocol_id(&legacy_id), kind);
        }
    }
}
