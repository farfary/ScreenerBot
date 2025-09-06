/// Meteora DAMM v2 pool decoder
/// Handles Dynamic AMM v2 pools

use crate::pools::constants::METEORA_DAMM_V2_PROGRAM_ID;
use crate::pools::decoders::PoolDecodedResult;
use crate::pools::service::PreparedPoolData;
use crate::tokens::decimals::get_cached_decimals;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::collections::HashMap;
use crate::logger::{ log, LogTag };

/// Meteora DAMM v2 pool decoder
#[derive(Debug, Clone)]
pub struct MeteoraDammV2Decoder {
    // No state needed for decoder
}

impl MeteoraDammV2Decoder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn can_decode(&self, program_id: &str) -> bool {
        program_id == METEORA_DAMM_V2_PROGRAM_ID
    }

    pub fn decode_pool_data(
        &self,
        prepared_data: &PreparedPoolData
    ) -> Result<PoolDecodedResult, String> {
        if prepared_data.program_id != METEORA_DAMM_V2_PROGRAM_ID {
            return Err(
                format!("Invalid program ID for Meteora DAMM v2: {}", prepared_data.program_id)
            );
        }

        let data = &prepared_data.pool_account_data;

        if data.len() < 1112 {
            return Err(
                format!(
                    "Invalid Meteora DAMM v2 pool account data length: {} (expected >= 1112)",
                    data.len()
                )
            );
        }

        log(
            LogTag::Pool,
            "METEORA_DECODE",
            &format!(
                "🔍 Decoding Meteora DAMM v2 pool {}, data length: {}",
                &prepared_data.pool_address[..8],
                data.len()
            )
        );

        // Decode Meteora DAMM v2 pool structure based on the provided layout
        // Skip the complex pool_fees structure (starts at offset 8, quite large)
        // Jump to token mints which start around offset 136 based on the old implementation
        let mut offset = 136;

        let token_a_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_b_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_a_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_b_vault = Self::read_pubkey_at_offset(data, &mut offset)?;

        // Skip whitelisted_vault and partner
        offset += 64;

        // Read liquidity (u128)
        let liquidity = Self::read_u128_at_offset(data, &mut offset)?;

        // Skip _padding (u128)
        offset += 16;

        // Read protocol fees
        let _protocol_a_fee = Self::read_u64_at_offset(data, &mut offset)?;
        let _protocol_b_fee = Self::read_u64_at_offset(data, &mut offset)?;

        // Skip partner fees
        offset += 16;

        // Read sqrt prices
        let _sqrt_min_price = Self::read_u128_at_offset(data, &mut offset)?;
        let _sqrt_max_price = Self::read_u128_at_offset(data, &mut offset)?;
        let _sqrt_price = Self::read_u128_at_offset(data, &mut offset)?;

        // Read activation point
        let _activation_point = Self::read_u64_at_offset(data, &mut offset)?;

        // Read status flags
        let _activation_type = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_status = Self::read_u8_at_offset(data, &mut offset)?;
        let _token_a_flag = Self::read_u8_at_offset(data, &mut offset)?;
        let _token_b_flag = Self::read_u8_at_offset(data, &mut offset)?;
        let _collect_fee_mode = Self::read_u8_at_offset(data, &mut offset)?;
        let _pool_type = Self::read_u8_at_offset(data, &mut offset)?;

        log(
            LogTag::Pool,
            "METEORA_EXTRACTED",
            &format!(
                "🔍 Extracted: Token A: {}, Token B: {}, Vaults: {} / {}",
                &token_a_mint[..8],
                &token_b_mint[..8],
                &token_a_vault[..8],
                &token_b_vault[..8]
            )
        );

        // Get token decimals
        let token_a_decimals = get_cached_decimals(&token_a_mint).ok_or_else(||
            format!("Cannot determine decimals for token A: {}", token_a_mint)
        )?;

        let token_b_decimals = get_cached_decimals(&token_b_mint).ok_or_else(||
            format!("Cannot determine decimals for token B: {}", token_b_mint)
        )?;

        // Get vault balances from prepared reserve data
        let (token_a_reserve, token_b_reserve) = self.get_vault_balances_from_prepared_data(
            &token_a_vault,
            &token_b_vault,
            &prepared_data.reserve_accounts_data
        )?;

        log(
            LogTag::Pool,
            "METEORA_RESERVES",
            &format!(
                "💰 Reserves: Token A: {} (decimals: {}), Token B: {} (decimals: {})",
                token_a_reserve,
                token_a_decimals,
                token_b_reserve,
                token_b_decimals
            )
        );

        // Create and return the decoded result
        let mut result = PoolDecodedResult::new(
            prepared_data.pool_address.clone(),
            METEORA_DAMM_V2_PROGRAM_ID.to_string(),
            "Meteora DAMM v2".to_string(),
            token_a_mint,
            token_b_mint,
            token_a_reserve,
            token_b_reserve,
            token_a_decimals,
            token_b_decimals
        );

        // Set optional fields
        result.token_a_vault = Some(token_a_vault);
        result.token_b_vault = Some(token_b_vault);
        result.lp_supply = Some(liquidity as u64);
        result.status = Some(pool_status as u32);

        Ok(result)
    }

    /// Get vault balances from prepared reserve data
    fn get_vault_balances_from_prepared_data(
        &self,
        vault_a: &str,
        vault_b: &str,
        reserve_data: &HashMap<String, Vec<u8>>
    ) -> Result<(u64, u64), String> {
        // Get vault A account data
        let vault_a_data = reserve_data
            .get(vault_a)
            .ok_or_else(|| format!("Vault A account data not found for {}", vault_a))?;

        let vault_b_data = reserve_data
            .get(vault_b)
            .ok_or_else(|| format!("Vault B account data not found for {}", vault_b))?;

        // Decode token account amounts
        let balance_a = Self::decode_token_account_amount(vault_a_data).map_err(|e|
            format!("Failed to decode vault A balance: {}", e)
        )?;

        let balance_b = Self::decode_token_account_amount(vault_b_data).map_err(|e|
            format!("Failed to decode vault B balance: {}", e)
        )?;

        log(
            LogTag::Pool,
            "METEORA_VAULT_BALANCES",
            &format!(
                "🏦 Vault balances - A ({}): {}, B ({}): {}",
                &vault_a[..8],
                balance_a,
                &vault_b[..8],
                balance_b
            )
        );

        Ok((balance_a, balance_b))
    }

    /// Decode token account amount from account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Invalid token account data length".to_string());
        }

        // Token account amount is at offset 64 (8 bytes)
        let amount_bytes = &data[64..72];
        let amount = u64::from_le_bytes(
            amount_bytes.try_into().map_err(|_| "Failed to parse token account amount".to_string())?
        );

        Ok(amount)
    }

    /// Helper functions for reading pool data
    fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
        if *offset + 32 > data.len() {
            return Err("Insufficient data for pubkey".to_string());
        }

        let pubkey_bytes = &data[*offset..*offset + 32];
        *offset += 32;

        let pubkey = Pubkey::new_from_array(
            pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey".to_string())?
        );

        Ok(pubkey.to_string())
    }

    fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8, String> {
        if *offset >= data.len() {
            return Err("Insufficient data for u8".to_string());
        }

        let value = data[*offset];
        *offset += 1;
        Ok(value)
    }

    fn read_u64_at_offset(data: &[u8], offset: &mut usize) -> Result<u64, String> {
        if *offset + 8 > data.len() {
            return Err("Insufficient data for u64".to_string());
        }

        let bytes = &data[*offset..*offset + 8];
        *offset += 8;

        let value = u64::from_le_bytes(
            bytes.try_into().map_err(|_| "Failed to parse u64".to_string())?
        );

        Ok(value)
    }

    fn read_u128_at_offset(data: &[u8], offset: &mut usize) -> Result<u128, String> {
        if *offset + 16 > data.len() {
            return Err("Insufficient data for u128".to_string());
        }

        let bytes = &data[*offset..*offset + 16];
        *offset += 16;

        let value = u128::from_le_bytes(
            bytes.try_into().map_err(|_| "Failed to parse u128".to_string())?
        );

        Ok(value)
    }
}
