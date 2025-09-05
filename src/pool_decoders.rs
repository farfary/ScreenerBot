use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::pool_extraction::*;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

/// Pool decoder trait for different pool types
pub trait PoolDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String>;
    fn get_reserve_accounts(&self, pool_address: &Pubkey) -> Vec<Pubkey>;

    /// Extract vault/reserve addresses from pool account data without full decoding
    /// This allows fetching vault accounts in the same batch as pool accounts
    fn extract_vault_addresses(&self, data: &[u8]) -> Result<Vec<Pubkey>, String>;

    /// Check if this pool type needs additional vault/reserve account fetches
    fn needs_vault_accounts(&self) -> bool;

    /// Decode pool data with pre-fetched vault account data
    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String>;
}

/// Decoded pool data structure
#[derive(Debug, Clone)]
pub struct DecodedPoolData {
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub token_a_decimals: u8,
    pub token_b_decimals: u8,
    pub pool_type: PoolType,
}

/// Pool types supported
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PoolType {
    RaydiumCpmm,
    RaydiumLegacy,
    MeteoraDb,
    MeteoraDamm,
    Orca,
    PumpFun,
}

/// Raydium CPMM decoder placeholder
pub struct RaydiumCpmmDecoder;

impl PoolDecoder for RaydiumCpmmDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String> {
        if data.len() < 8 {
            return Err("Invalid pool data: too short".to_string());
        }

        let debug_enabled = is_debug_pool_calculator_enabled();
        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Decoding Raydium CPMM pool data");
        }

        // Extract all components using the extraction functions
        let (token_0_mint, token_1_mint) = extract_raydium_cpmm_mints(data)?;
        let (token_0_decimals, token_1_decimals) = extract_raydium_cpmm_decimals(data)?;
        let (token_a_reserve, token_b_reserve) = extract_raydium_cpmm_reserves(data)?;

        if debug_enabled {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!(
                    "Decoded Raydium CPMM: token_0={}, token_1={}, reserve_0={}, reserve_1={}",
                    token_0_mint,
                    token_1_mint,
                    token_a_reserve,
                    token_b_reserve
                )
            );
        }

        Ok(DecodedPoolData {
            token_a_mint: token_0_mint,
            token_b_mint: token_1_mint,
            token_a_reserve,
            token_b_reserve,
            token_a_decimals: token_0_decimals,
            token_b_decimals: token_1_decimals,
            pool_type: PoolType::RaydiumCpmm,
        })
    }

    fn get_reserve_accounts(&self, _pool_address: &Pubkey) -> Vec<Pubkey> {
        // For Raydium CPMM, we need to fetch the pool data to get vault addresses
        // This is a placeholder - in practice, you'd need to fetch the pool account data
        vec![]
    }

    fn extract_vault_addresses(&self, data: &[u8]) -> Result<Vec<Pubkey>, String> {
        extract_raydium_cpmm_vaults(data)
    }

    fn needs_vault_accounts(&self) -> bool {
        true // Raydium CPMM needs vault account data for reserves
    }

    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String> {
        // First decode basic pool structure
        let mut pool_info = self.decode_pool_data(pool_data)?;

        // If vault data is provided, use it to get accurate reserves
        if vault_data.len() >= 2 {
            if let (Some(vault_0_data), Some(vault_1_data)) = (&vault_data[0], &vault_data[1]) {
                // Decode token account amounts from vault data
                let vault_0_balance = decode_token_account_amount(vault_0_data)?;
                let vault_1_balance = decode_token_account_amount(vault_1_data)?;

                // Update reserves with vault balances
                pool_info.token_a_reserve = vault_0_balance;
                pool_info.token_b_reserve = vault_1_balance;
            }
        }

        Ok(pool_info)
    }
}

/// Raydium Legacy decoder placeholder
pub struct RaydiumLegacyDecoder;

impl PoolDecoder for RaydiumLegacyDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String> {
        let debug_enabled = is_debug_pool_calculator_enabled();
        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Decoding Raydium Legacy pool data");
        }

        // Extract mints using pool_extraction functions
        let (token_a_mint, token_b_mint) = extract_raydium_legacy_mints(data)?;

        // Extract decimals using pool_extraction functions
        let (token_a_decimals, token_b_decimals) = extract_raydium_legacy_decimals(data)?;

        // Extract reserves using pool_extraction functions
        let (token_a_reserve, token_b_reserve) = extract_raydium_legacy_reserves(data)?;

        if debug_enabled {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!(
                    "Decoded Raydium Legacy: token_0={}, token_1={}, reserve_0={}, reserve_1={}",
                    token_a_mint,
                    token_b_mint,
                    token_a_reserve,
                    token_b_reserve
                )
            );
        }

        Ok(DecodedPoolData {
            token_a_mint,
            token_b_mint,
            token_a_reserve,
            token_b_reserve,
            token_a_decimals,
            token_b_decimals,
            pool_type: PoolType::RaydiumLegacy,
        })
    }

    fn get_reserve_accounts(&self, _pool_address: &Pubkey) -> Vec<Pubkey> {
        // For Raydium Legacy, we need to fetch the pool data to get vault addresses
        // This is a placeholder - in practice, you'd need to fetch the pool account data
        vec![]
    }

    fn extract_vault_addresses(&self, data: &[u8]) -> Result<Vec<Pubkey>, String> {
        extract_raydium_legacy_vaults(data)
    }

    fn needs_vault_accounts(&self) -> bool {
        true // Raydium Legacy needs vault account data for reserves
    }

    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        _vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String> {
        // For now, just use the basic decode - vault integration would need actual implementation
        self.decode_pool_data(pool_data)
    }
}

/// Meteora DLMM decoder placeholder
pub struct MeteoraDbDecoder;

impl PoolDecoder for MeteoraDbDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String> {
        if data.len() < 8 {
            return Err("Invalid pool data: too short".to_string());
        }

        let debug_enabled = is_debug_pool_calculator_enabled();
        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Decoding Meteora DLMM pool data");
        }

        let mut offset = 0;

        // Skip discriminator (8 bytes)
        offset += 8;

        // Read bin_step (2 bytes)
        if offset + 2 > data.len() {
            return Err("Invalid pool data: insufficient data for bin_step".to_string());
        }
        let _bin_step = u16::from_le_bytes(
            data[offset..offset + 2].try_into().map_err(|_| "Invalid bin_step")?
        );
        offset += 2;

        // Read padding (6 bytes)
        offset += 6;

        // Read token_0_mint (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_mint".to_string());
        }
        let token_0_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_0_mint pubkey")?
        );
        offset += 32;

        // Read token_1_mint (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_mint".to_string());
        }
        let token_1_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_1_mint pubkey")?
        );
        offset += 32;

        // Read token_0_vault (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_vault".to_string());
        }
        let _token_0_vault = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_0_vault pubkey")?
        );
        offset += 32;

        // Read token_1_vault (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_vault".to_string());
        }
        let _token_1_vault = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_1_vault pubkey")?
        );
        offset += 32;

        // Read oracle (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for oracle".to_string());
        }
        let _oracle = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid oracle pubkey")?
        );
        offset += 32;

        // Read oracle_initialized (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for oracle_initialized".to_string());
        }
        let _oracle_initialized = data[offset];
        offset += 1;

        // Read padding (7 bytes)
        offset += 7;

        // Read active_id (4 bytes)
        if offset + 4 > data.len() {
            return Err("Invalid pool data: insufficient data for active_id".to_string());
        }
        let _active_id = u32::from_le_bytes(
            data[offset..offset + 4].try_into().map_err(|_| "Invalid active_id")?
        );
        offset += 4;

        // Read status (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for status".to_string());
        }
        let _status = data[offset];
        offset += 1;

        // Read padding (3 bytes)
        offset += 3;

        // Read token_0_decimals (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_decimals".to_string());
        }
        let token_0_decimals = data[offset];
        offset += 1;

        // Read token_1_decimals (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_decimals".to_string());
        }
        let token_1_decimals = data[offset];
        offset += 1;

        // Read token_0_reserve (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_reserve".to_string());
        }
        let token_0_reserve = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid token_0_reserve")?
        );
        offset += 8;

        // Read token_1_reserve (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_reserve".to_string());
        }
        let token_1_reserve = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid token_1_reserve")?
        );
        offset += 8;

        if debug_enabled {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!(
                    "Decoded Meteora DLMM: token_0={}, token_1={}, reserve_0={}, reserve_1={}",
                    token_0_mint,
                    token_1_mint,
                    token_0_reserve,
                    token_1_reserve
                )
            );
        }

        Ok(DecodedPoolData {
            token_a_mint: token_0_mint,
            token_b_mint: token_1_mint,
            token_a_reserve: token_0_reserve,
            token_b_reserve: token_1_reserve,
            token_a_decimals: token_0_decimals,
            token_b_decimals: token_1_decimals,
            pool_type: PoolType::MeteoraDb,
        })
    }

    fn get_reserve_accounts(&self, _pool_address: &Pubkey) -> Vec<Pubkey> {
        // For Meteora DLMM, we need to fetch the pool data to get vault addresses
        // This is a placeholder - in practice, you'd need to fetch the pool account data
        vec![]
    }

    fn extract_vault_addresses(&self, data: &[u8]) -> Result<Vec<Pubkey>, String> {
        // Meteora DLMM has reserve addresses in the pool data
        if data.len() < 200 {
            return Err("Not enough data to extract vault addresses".to_string());
        }
        // Placeholder - would need actual DLMM structure parsing
        Ok(vec![])
    }

    fn needs_vault_accounts(&self) -> bool {
        true // Meteora DLMM needs reserve account data
    }

    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        _vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String> {
        // For now, just use the basic decode
        self.decode_pool_data(pool_data)
    }
}

/// Meteora DAMM decoder placeholder
pub struct MeteoraDammDecoder;

impl PoolDecoder for MeteoraDammDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String> {
        if data.len() < 8 {
            return Err("Invalid pool data: too short".to_string());
        }

        let debug_enabled = is_debug_pool_calculator_enabled();
        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Decoding Meteora DAMM pool data");
        }

        let mut offset = 0;

        // Skip discriminator (8 bytes)
        offset += 8;

        // Read status (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for status".to_string());
        }
        let _status = data[offset];
        offset += 1;

        // Read padding (7 bytes)
        offset += 7;

        // Read token_0_mint (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_mint".to_string());
        }
        let token_0_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_0_mint pubkey")?
        );
        offset += 32;

        // Read token_1_mint (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_mint".to_string());
        }
        let token_1_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_1_mint pubkey")?
        );
        offset += 32;

        // Read token_0_vault (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_vault".to_string());
        }
        let _token_0_vault = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_0_vault pubkey")?
        );
        offset += 32;

        // Read token_1_vault (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_vault".to_string());
        }
        let _token_1_vault = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_1_vault pubkey")?
        );
        offset += 32;

        // Read token_0_decimals (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_decimals".to_string());
        }
        let token_0_decimals = data[offset];
        offset += 1;

        // Read token_1_decimals (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_decimals".to_string());
        }
        let token_1_decimals = data[offset];
        offset += 1;

        // Read token_0_reserve (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for token_0_reserve".to_string());
        }
        let token_0_reserve = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid token_0_reserve")?
        );
        offset += 8;

        // Read token_1_reserve (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for token_1_reserve".to_string());
        }
        let token_1_reserve = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid token_1_reserve")?
        );
        offset += 8;

        if debug_enabled {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!(
                    "Decoded Meteora DAMM: token_0={}, token_1={}, reserve_0={}, reserve_1={}",
                    token_0_mint,
                    token_1_mint,
                    token_0_reserve,
                    token_1_reserve
                )
            );
        }

        Ok(DecodedPoolData {
            token_a_mint: token_0_mint,
            token_b_mint: token_1_mint,
            token_a_reserve: token_0_reserve,
            token_b_reserve: token_1_reserve,
            token_a_decimals: token_0_decimals,
            token_b_decimals: token_1_decimals,
            pool_type: PoolType::MeteoraDamm,
        })
    }

    fn get_reserve_accounts(&self, _pool_address: &Pubkey) -> Vec<Pubkey> {
        // For Meteora DAMM, we need to fetch the pool data to get vault addresses
        // This is a placeholder - in practice, you'd need to fetch the pool account data
        vec![]
    }

    fn extract_vault_addresses(&self, _data: &[u8]) -> Result<Vec<Pubkey>, String> {
        // Meteora DAMM has reserves embedded in the pool account
        Ok(vec![]) // No additional vault accounts needed
    }

    fn needs_vault_accounts(&self) -> bool {
        false // Meteora DAMM has reserves in main pool account
    }

    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        _vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String> {
        // Just use basic decode since no vault accounts needed
        self.decode_pool_data(pool_data)
    }
}

/// Orca decoder placeholder
pub struct OrcaDecoder;

impl PoolDecoder for OrcaDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String> {
        if data.len() < 8 {
            return Err("Invalid pool data: too short".to_string());
        }

        let debug_enabled = is_debug_pool_calculator_enabled();
        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Decoding Orca Whirlpool data");
        }

        let mut offset = 0;

        // Skip discriminator (8 bytes)
        offset += 8;

        // Read whirlpools_config (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for whirlpools_config".to_string());
        }
        let _whirlpools_config = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid whirlpools_config pubkey")?
        );
        offset += 32;

        // Read whirlpool_bump (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for whirlpool_bump".to_string());
        }
        let _whirlpool_bump = data[offset];
        offset += 1;

        // Read tick_spacing (2 bytes)
        if offset + 2 > data.len() {
            return Err("Invalid pool data: insufficient data for tick_spacing".to_string());
        }
        let _tick_spacing = u16::from_le_bytes(
            data[offset..offset + 2].try_into().map_err(|_| "Invalid tick_spacing")?
        );
        offset += 2;

        // Read tick_spacing_seed (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for tick_spacing_seed".to_string());
        }
        let _tick_spacing_seed = data[offset];
        offset += 1;

        // Read fee_rate (2 bytes)
        if offset + 2 > data.len() {
            return Err("Invalid pool data: insufficient data for fee_rate".to_string());
        }
        let _fee_rate = u16::from_le_bytes(
            data[offset..offset + 2].try_into().map_err(|_| "Invalid fee_rate")?
        );
        offset += 2;

        // Read protocol_fee_rate (2 bytes)
        if offset + 2 > data.len() {
            return Err("Invalid pool data: insufficient data for protocol_fee_rate".to_string());
        }
        let _protocol_fee_rate = u16::from_le_bytes(
            data[offset..offset + 2].try_into().map_err(|_| "Invalid protocol_fee_rate")?
        );
        offset += 2;

        // Read liquidity (16 bytes)
        if offset + 16 > data.len() {
            return Err("Invalid pool data: insufficient data for liquidity".to_string());
        }
        let _liquidity = u128::from_le_bytes(
            data[offset..offset + 16].try_into().map_err(|_| "Invalid liquidity")?
        );
        offset += 16;

        // Read sqrt_price (16 bytes)
        if offset + 16 > data.len() {
            return Err("Invalid pool data: insufficient data for sqrt_price".to_string());
        }
        let _sqrt_price = u128::from_le_bytes(
            data[offset..offset + 16].try_into().map_err(|_| "Invalid sqrt_price")?
        );
        offset += 16;

        // Read tick_current_index (4 bytes)
        if offset + 4 > data.len() {
            return Err("Invalid pool data: insufficient data for tick_current_index".to_string());
        }
        let _tick_current_index = i32::from_le_bytes(
            data[offset..offset + 4].try_into().map_err(|_| "Invalid tick_current_index")?
        );
        offset += 4;

        // Read protocol_fee_owed_a (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for protocol_fee_owed_a".to_string());
        }
        let _protocol_fee_owed_a = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid protocol_fee_owed_a")?
        );
        offset += 8;

        // Read protocol_fee_owed_b (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for protocol_fee_owed_b".to_string());
        }
        let _protocol_fee_owed_b = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid protocol_fee_owed_b")?
        );
        offset += 8;

        // Read token_mint_a (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_mint_a".to_string());
        }
        let token_0_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_mint_a pubkey")?
        );
        offset += 32;

        // Read token_vault_a (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_vault_a".to_string());
        }
        let _token_vault_a = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_vault_a pubkey")?
        );
        offset += 32;

        // Read fee_growth_global_a (16 bytes)
        if offset + 16 > data.len() {
            return Err("Invalid pool data: insufficient data for fee_growth_global_a".to_string());
        }
        let _fee_growth_global_a = u128::from_le_bytes(
            data[offset..offset + 16].try_into().map_err(|_| "Invalid fee_growth_global_a")?
        );
        offset += 16;

        // Read token_mint_b (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_mint_b".to_string());
        }
        let token_1_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_mint_b pubkey")?
        );
        offset += 32;

        // Read token_vault_b (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_vault_b".to_string());
        }
        let _token_vault_b = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_vault_b pubkey")?
        );
        offset += 32;

        // Read fee_growth_global_b (16 bytes)
        if offset + 16 > data.len() {
            return Err("Invalid pool data: insufficient data for fee_growth_global_b".to_string());
        }
        let _fee_growth_global_b = u128::from_le_bytes(
            data[offset..offset + 16].try_into().map_err(|_| "Invalid fee_growth_global_b")?
        );
        offset += 16;

        // Read reward_last_updated_timestamp (8 bytes)
        if offset + 8 > data.len() {
            return Err(
                "Invalid pool data: insufficient data for reward_last_updated_timestamp".to_string()
            );
        }
        let _reward_last_updated_timestamp = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| "Invalid reward_last_updated_timestamp")?
        );
        offset += 8;

        // For Orca, we need to fetch vault balances separately
        // The pool data doesn't contain reserve amounts directly
        let token_0_reserve = 0; // Will be fetched from vault
        let token_1_reserve = 0; // Will be fetched from vault
        let token_0_decimals = 9; // Default, should be fetched from token metadata
        let token_1_decimals = 9; // Default, should be fetched from token metadata

        if debug_enabled {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!(
                    "Decoded Orca Whirlpool: token_0={}, token_1={} (reserves need to be fetched from vaults)",
                    token_0_mint,
                    token_1_mint
                )
            );
        }

        Ok(DecodedPoolData {
            token_a_mint: token_0_mint,
            token_b_mint: token_1_mint,
            token_a_reserve: token_0_reserve,
            token_b_reserve: token_1_reserve,
            token_a_decimals: token_0_decimals,
            token_b_decimals: token_1_decimals,
            pool_type: PoolType::Orca,
        })
    }

    fn get_reserve_accounts(&self, _pool_address: &Pubkey) -> Vec<Pubkey> {
        // For Orca, we need to fetch the pool data to get vault addresses
        // This is a placeholder - in practice, you'd need to fetch the pool account data
        vec![]
    }

    fn extract_vault_addresses(&self, data: &[u8]) -> Result<Vec<Pubkey>, String> {
        // Orca Whirlpool has vault addresses in the pool data
        if data.len() < 200 {
            return Err("Not enough data to extract vault addresses".to_string());
        }
        // Placeholder - would need actual Whirlpool structure parsing
        Ok(vec![])
    }

    fn needs_vault_accounts(&self) -> bool {
        true // Orca Whirlpool needs vault account data
    }

    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        _vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String> {
        // For now, just use the basic decode
        self.decode_pool_data(pool_data)
    }
}

/// Pump.fun decoder placeholder
pub struct PumpFunDecoder;

impl PoolDecoder for PumpFunDecoder {
    fn decode_pool_data(&self, data: &[u8]) -> Result<DecodedPoolData, String> {
        if data.len() < 8 {
            return Err("Invalid pool data: too short".to_string());
        }

        let debug_enabled = is_debug_pool_calculator_enabled();
        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Decoding Pump.fun AMM data");
        }

        let mut offset = 0;

        // Skip discriminator (8 bytes)
        offset += 8;

        // Read virtual_sol_reserves (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for virtual_sol_reserves".to_string());
        }
        let _virtual_sol_reserves = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid virtual_sol_reserves")?
        );
        offset += 8;

        // Read virtual_token_reserves (8 bytes)
        if offset + 8 > data.len() {
            return Err(
                "Invalid pool data: insufficient data for virtual_token_reserves".to_string()
            );
        }
        let _virtual_token_reserves = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid virtual_token_reserves")?
        );
        offset += 8;

        // Read real_sol_reserves (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for real_sol_reserves".to_string());
        }
        let token_1_reserve = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid real_sol_reserves")?
        );
        offset += 8;

        // Read real_token_reserves (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for real_token_reserves".to_string());
        }
        let token_0_reserve = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid real_token_reserves")?
        );
        offset += 8;

        // Read bonding_curve (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for bonding_curve".to_string());
        }
        let _bonding_curve = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid bonding_curve pubkey")?
        );
        offset += 32;

        // Read associated_bonding_curve (32 bytes)
        if offset + 32 > data.len() {
            return Err(
                "Invalid pool data: insufficient data for associated_bonding_curve".to_string()
            );
        }
        let _associated_bonding_curve = Pubkey::new_from_array(
            data[offset..offset + 32]
                .try_into()
                .map_err(|_| "Invalid associated_bonding_curve pubkey")?
        );
        offset += 32;

        // Read mint (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for mint".to_string());
        }
        let token_0_mint = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid mint pubkey")?
        );
        offset += 32;

        // Read sol_reserves (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for sol_reserves".to_string());
        }
        let _sol_reserves = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid sol_reserves pubkey")?
        );
        offset += 32;

        // Read token_reserves (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for token_reserves".to_string());
        }
        let _token_reserves = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid token_reserves pubkey")?
        );
        offset += 32;

        // Read complete (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for complete".to_string());
        }
        let _complete = data[offset];
        offset += 1;

        // Read padding (7 bytes)
        offset += 7;

        // Read complete_timestamp (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for complete_timestamp".to_string());
        }
        let _complete_timestamp = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid complete_timestamp")?
        );
        offset += 8;

        // Read created_timestamp (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for created_timestamp".to_string());
        }
        let _created_timestamp = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid created_timestamp")?
        );
        offset += 8;

        // Read creator (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for creator".to_string());
        }
        let _creator = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid creator pubkey")?
        );
        offset += 32;

        // Read metadata (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for metadata".to_string());
        }
        let _metadata = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid metadata pubkey")?
        );
        offset += 32;

        // Read name (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for name".to_string());
        }
        let _name = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid name pubkey")?
        );
        offset += 32;

        // Read symbol (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for symbol".to_string());
        }
        let _symbol = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid symbol pubkey")?
        );
        offset += 32;

        // Read uri (32 bytes)
        if offset + 32 > data.len() {
            return Err("Invalid pool data: insufficient data for uri".to_string());
        }
        let _uri = Pubkey::new_from_array(
            data[offset..offset + 32].try_into().map_err(|_| "Invalid uri pubkey")?
        );
        offset += 32;

        // Read total_supply (8 bytes)
        if offset + 8 > data.len() {
            return Err("Invalid pool data: insufficient data for total_supply".to_string());
        }
        let _total_supply = u64::from_le_bytes(
            data[offset..offset + 8].try_into().map_err(|_| "Invalid total_supply")?
        );
        offset += 8;

        // Read decimals (1 byte)
        if offset + 1 > data.len() {
            return Err("Invalid pool data: insufficient data for decimals".to_string());
        }
        let token_0_decimals = data[offset];
        offset += 1;

        // Read padding (7 bytes)
        offset += 7;

        // For Pump.fun, SOL is always the quote token
        let token_1_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").map_err(
            |_| "Invalid SOL mint address"
        )?;
        let token_1_decimals = 9; // SOL always has 9 decimals

        if debug_enabled {
            log(
                LogTag::Pool,
                "DEBUG",
                &format!(
                    "Decoded Pump.fun: token_0={}, token_1={}, reserve_0={}, reserve_1={}",
                    token_0_mint,
                    token_1_mint,
                    token_0_reserve,
                    token_1_reserve
                )
            );
        }

        Ok(DecodedPoolData {
            token_a_mint: token_0_mint,
            token_b_mint: token_1_mint,
            token_a_reserve: token_0_reserve,
            token_b_reserve: token_1_reserve,
            token_a_decimals: token_0_decimals,
            token_b_decimals: token_1_decimals,
            pool_type: PoolType::PumpFun,
        })
    }

    fn get_reserve_accounts(&self, _pool_address: &Pubkey) -> Vec<Pubkey> {
        // For Pump.fun, we need to fetch the pool data to get vault addresses
        // This is a placeholder - in practice, you'd need to fetch the pool account data
        vec![]
    }

    fn extract_vault_addresses(&self, data: &[u8]) -> Result<Vec<Pubkey>, String> {
        // Pump.fun has vault addresses in the pool data
        if data.len() < 200 {
            return Err("Not enough data to extract vault addresses".to_string());
        }
        // Placeholder - would need actual Pump.fun structure parsing
        Ok(vec![])
    }

    fn needs_vault_accounts(&self) -> bool {
        true // Pump.fun needs vault account data
    }

    fn decode_pool_data_with_vaults(
        &self,
        pool_data: &[u8],
        _vault_data: &[Option<Vec<u8>>]
    ) -> Result<DecodedPoolData, String> {
        // For now, just use the basic decode
        self.decode_pool_data(pool_data)
    }
}

/// Pool decoder factory
pub struct PoolDecoderFactory {
    decoders: HashMap<PoolType, Box<dyn PoolDecoder + Send + Sync>>,
}

impl PoolDecoderFactory {
    pub fn new() -> Self {
        let mut decoders: HashMap<PoolType, Box<dyn PoolDecoder + Send + Sync>> = HashMap::new();

        decoders.insert(PoolType::RaydiumCpmm, Box::new(RaydiumCpmmDecoder));
        decoders.insert(PoolType::RaydiumLegacy, Box::new(RaydiumLegacyDecoder));
        decoders.insert(PoolType::MeteoraDb, Box::new(MeteoraDbDecoder));
        decoders.insert(PoolType::MeteoraDamm, Box::new(MeteoraDammDecoder));
        decoders.insert(PoolType::Orca, Box::new(OrcaDecoder));
        decoders.insert(PoolType::PumpFun, Box::new(PumpFunDecoder));

        Self { decoders }
    }

    pub fn get_decoder(&self, pool_type: &PoolType) -> Option<&(dyn PoolDecoder + Send + Sync)> {
        self.decoders.get(pool_type).map(|d| d.as_ref())
    }
}

impl Default for PoolDecoderFactory {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get pool type from program ID
pub fn get_pool_type_from_program_id(program_id: &str) -> Option<PoolType> {
    match program_id {
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => Some(PoolType::RaydiumCpmm),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(PoolType::RaydiumLegacy),
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => Some(PoolType::MeteoraDb),
        "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" => Some(PoolType::MeteoraDamm),
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => Some(PoolType::Orca),
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => Some(PoolType::PumpFun),
        _ => None,
    }
}

/// Get program ID from pool type
pub fn get_program_id_from_pool_type(pool_type: &PoolType) -> &'static str {
    match pool_type {
        PoolType::RaydiumCpmm => "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
        PoolType::RaydiumLegacy => "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
        PoolType::MeteoraDb => "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
        PoolType::MeteoraDamm => "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG",
        PoolType::Orca => "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
        PoolType::PumpFun => "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
    }
}

/// Get display name for pool type
pub fn get_pool_type_display_name(pool_type: &PoolType) -> &'static str {
    match pool_type {
        PoolType::RaydiumCpmm => "Raydium CPMM",
        PoolType::RaydiumLegacy => "Raydium Legacy AMM",
        PoolType::MeteoraDb => "Meteora DLMM",
        PoolType::MeteoraDamm => "Meteora DAMM v2",
        PoolType::Orca => "Orca Whirlpool",
        PoolType::PumpFun => "Pump.fun AMM",
    }
}

/// Decode pool data using the appropriate decoder
pub fn decode_pool_data_by_program_id(
    program_id: &str,
    data: &[u8]
) -> Result<DecodedPoolData, String> {
    let pool_type = get_pool_type_from_program_id(program_id).ok_or_else(||
        format!("Unsupported program ID: {}", program_id)
    )?;

    let factory = PoolDecoderFactory::new();
    let decoder = factory
        .get_decoder(&pool_type)
        .ok_or_else(|| format!("No decoder found for pool type: {:?}", pool_type))?;

    decoder.decode_pool_data(data)
}

/// Get all supported program IDs
pub fn get_supported_program_ids() -> Vec<&'static str> {
    vec![
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", // Raydium CPMM
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium Legacy
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", // Meteora DLMM
        "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG", // Meteora DAMM v2
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", // Orca Whirlpool
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" // Pump.fun AMM
    ]
}
