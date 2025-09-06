/// Meteora DAMM v2 pool decoder
/// Handles Dynamic AMM v2 pools

use crate::pools::constants::METEORA_DAMM_V2_PROGRAM_ID;
use crate::pools::decoders::PoolDecodedResult;
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

    /// Extract vault addresses from pool account data
    pub fn extract_vault_addresses(&self, pool_data: &[u8]) -> Result<Vec<String>, String> {
        if pool_data.len() < 200 {
            return Err("Insufficient pool data length".to_string());
        }

        let mut vault_addresses = Vec::new();
        let mut offset = 136; // Start after discriminator and other fields

        // Skip token mints (64 bytes)
        offset += 64;

        // Read vault A address (32 bytes)
        if let Ok(vault_a) = read_pubkey_at_offset(pool_data, &mut offset) {
            vault_addresses.push(vault_a);
        }

        // Read vault B address (32 bytes)
        if let Ok(vault_b) = read_pubkey_at_offset(pool_data, &mut offset) {
            vault_addresses.push(vault_b);
        }

        Ok(vault_addresses)
    }

    pub async fn decode_pool_data(
        &self,
        pool_data: &[u8],
        reserve_accounts_data: &HashMap<String, Vec<u8>>
    ) -> Result<PoolDecodedResult, String> {
        if pool_data.len() < 200 {
            return Err("Insufficient pool data length for Meteora DAMM v2".to_string());
        }

        log(LogTag::Pool, "METEORA_DECODE_START", "🔍 Decoding Meteora DAMM v2 pool");

        let mut offset = 8; // Skip discriminator

        // Read pool state
        let pool_state = match read_u8_at_offset(pool_data, &mut offset) {
            Ok(state) => state,
            Err(e) => {
                return Err(format!("Failed to read pool state: {}", e));
            }
        };

        if pool_state != 0 {
            return Err("Pool is not in active state".to_string());
        }

        // Skip padding (7 bytes)
        offset += 7;

        // Read bump (1 byte)
        let _bump = read_u8_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read bump: {}", e)
        )?;

        // Skip padding (7 bytes)
        offset += 7;

        // Read AMM config (32 bytes)
        let _amm_config = read_pubkey_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read AMM config: {}", e)
        )?;

        // Read pool creator (32 bytes)
        let _pool_creator = read_pubkey_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read pool creator: {}", e)
        )?;

        // Read token A mint (32 bytes)
        let token_a_mint = read_pubkey_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read token A mint: {}", e)
        )?;

        // Read token B mint (32 bytes)
        let token_b_mint = read_pubkey_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read token B mint: {}", e)
        )?;

        // Read vault A (32 bytes)
        let vault_a = read_pubkey_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read vault A: {}", e)
        )?;

        // Read vault B (32 bytes)
        let vault_b = read_pubkey_at_offset(pool_data, &mut offset).map_err(|e|
            format!("Failed to read vault B: {}", e)
        )?;

        // Get vault balances
        let (vault_a_balance, vault_b_balance) = get_vault_balances_from_data(
            &vault_a,
            &vault_b,
            reserve_accounts_data
        )?;

        // Get token decimals
        let token_a_decimals = get_cached_decimals(&token_a_mint).await.unwrap_or(9);
        let token_b_decimals = get_cached_decimals(&token_b_mint).await.unwrap_or(9);

        log(
            LogTag::Pool,
            "METEORA_DECODE_SUCCESS",
            &format!(
                "✅ Meteora pool decoded: {} {} / {} {}",
                vault_a_balance,
                &token_a_mint[..6],
                vault_b_balance,
                &token_b_mint[..6]
            )
        );

        Ok(PoolDecodedResult {
            pool_address: "unknown".to_string(), // Will be set by caller
            program_id: METEORA_DAMM_V2_PROGRAM_ID.to_string(),
            pool_type: "DAMM v2".to_string(),
            token_a_mint,
            token_b_mint,
            token_a_reserve: vault_a_balance,
            token_b_reserve: vault_b_balance,
            token_a_decimals,
            token_b_decimals,
            fee_rate: 0.0, // TODO: Extract fee rate
            liquidity: 0.0, // TODO: Calculate liquidity
            volume_24h: None,
            fees_24h: None,
            apy: None,
            last_updated: chrono::Utc::now(),
        })
    }
}

/// Get vault balances from reserve account data
fn get_vault_balances_from_data(
    vault_a_address: &str,
    vault_b_address: &str,
    reserve_accounts_data: &HashMap<String, Vec<u8>>
) -> Result<(u64, u64), String> {
    let vault_a_balance = match reserve_accounts_data.get(vault_a_address) {
        Some(vault_data) => {
            if vault_data.len() >= 72 {
                read_u64_at_offset(vault_data, &mut 64).map_err(|e|
                    format!("Failed to read vault A balance: {}", e)
                )?
            } else {
                return Err("Vault A data too short".to_string());
            }
        }
        None => {
            return Err("Vault A data not found".to_string());
        }
    };

    let vault_b_balance = match reserve_accounts_data.get(vault_b_address) {
        Some(vault_data) => {
            if vault_data.len() >= 72 {
                read_u64_at_offset(vault_data, &mut 64).map_err(|e|
                    format!("Failed to read vault B balance: {}", e)
                )?
            } else {
                return Err("Vault B data too short".to_string());
            }
        }
        None => {
            return Err("Vault B data not found".to_string());
        }
    };

    Ok((vault_a_balance, vault_b_balance))
}

/// Helper function to read u8 at offset
fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8, String> {
    if *offset >= data.len() {
        return Err("Offset out of bounds for u8".to_string());
    }

    let value = data[*offset];
    *offset += 1;

    Ok(value)
}

/// Helper function to read u64 at offset (little endian)
fn read_u64_at_offset(data: &[u8], offset: &mut usize) -> Result<u64, String> {
    if *offset + 8 > data.len() {
        return Err("Insufficient data for u64".to_string());
    }

    let bytes = &data[*offset..*offset + 8];
    *offset += 8;

    Ok(
        u64::from_le_bytes(
            bytes.try_into().map_err(|_| "Failed to convert bytes to u64".to_string())?
        )
    )
}

/// Helper function to read pubkey at offset
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
