/// Pool decoders module
///
/// This module contains program-specific decoders for different DEX pool types.
/// Each decoder knows how to parse the account data for its specific pool format.

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
use super::types::{PriceResult, ProgramKind};
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
