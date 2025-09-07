/// Pool decoders module
///
/// This module contains program-specific decoders for different DEX pool types.
/// Each decoder knows how to parse the account data for its specific pool format.

pub mod raydium_cpmm;
pub mod raydium_clmm;
pub mod pumpfun_amm;
pub mod raydium_legacy_amm;
pub mod meteora_dlmm;
pub mod meteora_damm;
pub mod orca_whirlpool;

use super::fetcher::AccountData;
use super::types::{ ProgramKind, PriceResult };
use std::collections::HashMap;

/// Trait for pool decoders
pub trait PoolDecoder {
    /// Get the program kinds this decoder supports
    fn supported_programs() -> Vec<ProgramKind>;

    /// Decode pool data and calculate price
    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult>;
}

/// Main decoder dispatch function
pub fn decode_pool(
    program_kind: ProgramKind,
    accounts: &HashMap<String, AccountData>,
    base_mint: &str,
    quote_mint: &str
) -> Option<PriceResult> {
    match program_kind {
        ProgramKind::RaydiumCpmm => {
            raydium_cpmm::RaydiumCpmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint)
        }
        ProgramKind::RaydiumClmm =>
            raydium_clmm::RaydiumClmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint),
        ProgramKind::PumpFun =>
            pumpfun_amm::PumpFunAmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint),
        ProgramKind::RaydiumLegacyAmm =>
            raydium_legacy_amm::RaydiumLegacyAmmDecoder::decode_and_calculate(
                accounts,
                base_mint,
                quote_mint
            ),
        ProgramKind::MeteoraDlmm =>
            meteora_dlmm::MeteoraDlmmDecoder::decode_and_calculate(accounts, base_mint, quote_mint),
        ProgramKind::MeteoraDamm =>
            meteora_damm::MeteoraDammDecoder::decode_and_calculate(accounts, base_mint, quote_mint),
        ProgramKind::OrcaWhirlpool =>
            orca_whirlpool::OrcaWhirlpoolDecoder::decode_and_calculate(accounts, base_mint, quote_mint),
        _ => {
            // TODO: Add other decoders as needed
            None
        }
    }
}
