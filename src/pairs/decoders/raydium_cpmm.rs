use super::{
    PoolDecoder,
    PoolInfo,
    PoolType,
    PoolStatus,
    PoolMetadata,
    DecoderError,
    program_ids,
    price_math,
};
use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;

/// Raydium CPMM (Constant Product Market Maker) decoder
pub struct RaydiumCpmmDecoder;

impl RaydiumCpmmDecoder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PoolDecoder for RaydiumCpmmDecoder {
    fn decode(&self, data: &[u8]) -> Result<PoolInfo> {
        // Raydium CPMM pool structure parsing - based on debug analysis
        if data.len() < 637 {
            return Err(
                (DecoderError::InvalidDataLength {
                    expected: 637,
                    actual: data.len(),
                }).into()
            );
        }

        // Based on debug analysis, we know the exact offsets:
        // amm_config at offset 8
        // token_0_vault at offset 72
        // token_1_vault at offset 104
        // lp_mint at offset 136
        // token_0_mint at offset 168
        // token_1_mint at offset 200
        // status probably around offset 54 or similar

        // Parse amm_config (pubkey) at offset 8
        let amm_config = Pubkey::try_from(&data[8..40])?;

        // Parse token vaults at known offsets
        let token_0_vault = Pubkey::try_from(&data[72..104])?;
        let token_1_vault = Pubkey::try_from(&data[104..136])?;

        // Parse lp_mint at offset 136
        let lp_mint = Pubkey::try_from(&data[136..168])?;

        // Parse token mints at known offsets
        let token_0_mint = Pubkey::try_from(&data[168..200])?;
        let token_1_mint = Pubkey::try_from(&data[200..232])?;

        // Parse status byte at offset 54 (confirmed by analysis)
        let status_byte = data[54];
        let status = match status_byte {
            0 => PoolStatus::Active,
            1 => PoolStatus::Paused,
            _ => PoolStatus::Active, // Default to Active for unknown status
        };

        // Parse auth_bump at offset 55 (from Solscan: 253)
        let auth_bump = data.get(55).copied().unwrap_or(253);

        // Parse decimals based on Solscan structure:
        // lp_mint_decimals, mint_0_decimals, mint_1_decimals should be consecutive
        // Based on analysis, these are around offset 330-332
        let lp_mint_decimals = data.get(330).copied().unwrap_or(9); // Solscan: 9
        let mint_0_decimals = data.get(331).copied().unwrap_or(9); // Solscan: 9 (SOL)
        let mint_1_decimals = data.get(332).copied().unwrap_or(6); // Solscan: 6 (CRED)

        // Try to parse some numeric values
        // LP supply might be around offset 256-288 based on JSON structure
        let lp_supply = if data.len() > 280 {
            let bytes: [u8; 8] = data[272..280].try_into().unwrap_or([0; 8]);
            u64::from_le_bytes(bytes)
        } else {
            0
        };

        let metadata = PoolMetadata {
            amm_config: Some(amm_config),
            lp_mint: Some(lp_mint),
            lp_supply: Some(lp_supply),
            auth_bump: Some(auth_bump),
            ..Default::default()
        };

        let pool_info = PoolInfo {
            pool_address: Pubkey::default(), // Will be set by caller
            program_id: self.program_id(),
            pool_type: PoolType::RaydiumCpmm,
            token_mint_0: token_0_mint,
            token_mint_1: token_1_mint,
            token_vault_0: token_0_vault,
            token_vault_1: token_1_vault,
            reserve_0: 0, // To be fetched from vault accounts
            reserve_1: 0, // To be fetched from vault accounts
            decimals_0: mint_0_decimals,
            decimals_1: mint_1_decimals,
            liquidity: None,
            sqrt_price: None,
            current_tick: None,
            fee_rate: None, // Raydium CPMM typically has fixed fees
            status,
            metadata,
        };

        Ok(pool_info)
    }

    fn calculate_price(&self, pool_info: &PoolInfo) -> Result<f64> {
        // For Raydium CPMM, use simple constant product formula: price = reserve_1 / reserve_0
        if pool_info.reserve_0 > 0 && pool_info.reserve_1 > 0 {
            Ok(
                price_math::reserves_to_price(
                    pool_info.reserve_0,
                    pool_info.reserve_1,
                    pool_info.decimals_0,
                    pool_info.decimals_1
                )
            )
        } else {
            Err(
                (DecoderError::InvalidPrice {
                    reason: "No reserves available for price calculation".to_string(),
                }).into()
            )
        }
    }

    fn program_id(&self) -> Pubkey {
        program_ids::raydium_cpmm()
    }

    fn name(&self) -> &'static str {
        "Raydium CPMM"
    }
}

impl Default for RaydiumCpmmDecoder {
    fn default() -> Self {
        Self::new()
    }
}
