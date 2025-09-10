/// Raydium CLMM (Concentrated Liquidity Market Maker) decoder
///
/// This decoder handles Raydium Concentrated Liquidity pools.
/// CLMM uses a sqrt_price_x64 format (Q64.64) and token vaults for pricing.
/// Based on Uniswap v3 math principles but with Raydium-specific implementation.

use super::{ PoolDecoder, AccountData };
use crate::arguments::is_debug_pool_decoders_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::{ get_token_decimals_sync, decimals::SOL_DECIMALS };
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT, RAYDIUM_CLMM_PROGRAM_ID };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

pub struct RaydiumClmmDecoder;

impl PoolDecoder for RaydiumClmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::RaydiumClmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_decoders_enabled() {
            log(LogTag::PoolDecoder, "INFO", "Starting Raydium CLMM pool decoding");
        }

        // Find the pool account
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Raydium CLMM program as owner
            acc.owner.to_string() == RAYDIUM_CLMM_PROGRAM_ID
        })?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "Found CLMM pool account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse CLMM pool structure
        let clmm_info = Self::parse_clmm_pool(&pool_account.data)?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "CLMM pool parsed: token_mint_0={}, token_mint_1={}, vault_0={}, vault_1={}, sqrt_price_x64={}",
                    clmm_info.token_mint_0,
                    clmm_info.token_mint_1,
                    clmm_info.token_vault_0,
                    clmm_info.token_vault_1,
                    clmm_info.sqrt_price_x64
                )
            );
        }

        // Determine which token is SOL and which is the base token
        // Handle both orientations: TOKEN/SOL and SOL/TOKEN
        let (token_mint, sol_vault, token_vault, is_token_0) = if
            clmm_info.token_mint_1 == SOL_MINT
        {
            // token_mint_0 is the custom token, token_mint_1 is SOL
            (
                clmm_info.token_mint_0.clone(),
                clmm_info.token_vault_1.clone(),
                clmm_info.token_vault_0.clone(),
                true,
            )
        } else if clmm_info.token_mint_0 == SOL_MINT {
            // token_mint_1 is the custom token, token_mint_0 is SOL
            (
                clmm_info.token_mint_1.clone(),
                clmm_info.token_vault_0.clone(),
                clmm_info.token_vault_1.clone(),
                false,
            )
        } else {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "CLMM pool has no SOL token: {} / {}",
                        clmm_info.token_mint_0,
                        clmm_info.token_mint_1
                    )
                );
            }
            return None;
        };

        // Verify the token mint matches one of the requested mints (base or quote)
        // This handles both TOKEN/SOL and SOL/TOKEN orientations
        if token_mint != base_mint && token_mint != quote_mint {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "CLMM pool token {} doesn't match either requested mint: base={}, quote={}",
                        token_mint,
                        base_mint,
                        quote_mint
                    )
                );
            }
            return None;
        }

        // Get vault balances
        let sol_account = accounts.get(&sol_vault)?;
        let token_account = accounts.get(&token_vault)?;

        let sol_balance = Self::decode_token_account_amount(&sol_account.data).ok()?;
        let token_balance = Self::decode_token_account_amount(&token_account.data).ok()?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "CLMM vault balances: SOL={}, token={}, is_token_0={}",
                    sol_balance,
                    token_balance,
                    is_token_0
                )
            );
        }

        // Note: In CLMM pools, zero vault balances are normal when liquidity is concentrated
        // outside the current price range. We can still calculate price using sqrt_price_x64.
        if token_balance == 0 && sol_balance == 0 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "INFO",
                    "CLMM pool has zero vault balances, using sqrt_price for calculation"
                );
            }
        }

        // Get token decimals - CRITICAL: must be available, no fallback to defaults
        let token_decimals = match get_token_decimals_sync(&token_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!("CLMM: Token decimals not found for {}, skipping price calculation", token_mint)
                    );
                }
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!("CLMM decimals: token={}, sol={}", token_decimals, sol_decimals)
            );
        }

        // Calculate price using sqrt_price_x64
        // sqrt_price_x64 is in Q64.64 format, so we divide by 2^64 to get the actual sqrt_price
        // price = sqrt_price^2, and it represents token_1/token_0 price

        let sqrt_price = (clmm_info.sqrt_price_x64 as f64) / (2_f64).powi(64);
        let raw_price = sqrt_price * sqrt_price; // price = sqrt_price^2

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "CLMM sqrt_price_x64={}, sqrt_price={}, raw_price={}",
                    clmm_info.sqrt_price_x64,
                    sqrt_price,
                    raw_price
                )
            );
        }

        // Apply decimal adjustments and determine final price
        // raw_price represents token_1/token_0 ratio
        let price_sol = if is_token_0 {
            // Custom token is token_0, SOL is token_1
            // raw_price = SOL/token, so this is what we want
            raw_price * (10_f64).powi((token_decimals as i32) - (sol_decimals as i32))
        } else {
            // Custom token is token_1, SOL is token_0
            // raw_price = token/SOL, so we need to invert it
            (1.0 / raw_price) * (10_f64).powi((token_decimals as i32) - (sol_decimals as i32))
        };

        // Convert reserves to human-readable format for display
        let sol_reserves = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
        let token_reserves = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "CLMM price calculation: {:.12} SOL per token (sol_reserves={:.6}, token_reserves={:.6})",
                    price_sol,
                    sol_reserves,
                    token_reserves
                )
            );
        }

        Some(PriceResult {
            mint: token_mint,
            price_usd: 0.0, // We don't calculate USD prices, only SOL
            price_sol,
            sol_reserves,
            token_reserves,
            confidence: 0.9,
            source_pool: Some("RAYDIUM_CLMM".to_string()),
            pool_address: pool_account.pubkey.to_string(),
            slot: 0, // Will be updated by the system
            timestamp: Instant::now(),
        })
    }
}

impl RaydiumClmmDecoder {
    /// Extract reserve account addresses from CLMM pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [token_vault_0, token_vault_1]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() < 800 {
            return None;
        }

        // Based on Raydium CLMM PoolState struct layout
        // Skip discriminator (8 bytes), bump (1 byte), amm_config (32 bytes), owner (32 bytes)
        let base_offset = 8 + 1 + 32 + 32;

        // Skip token_mint_0 (32 bytes) and token_mint_1 (32 bytes)
        let vault_offset = base_offset + 32 + 32;

        // Extract vault pubkeys at calculated offsets
        let token_vault_0 = Self::extract_pubkey_at_offset(pool_data, vault_offset)?;
        let token_vault_1 = Self::extract_pubkey_at_offset(pool_data, vault_offset + 32)?;

        Some(vec![token_vault_0, token_vault_1])
    }

    /// Parse CLMM pool account data to extract complete PoolState structure
    /// Based on Raydium CLMM PoolState struct from official GitHub source code
    fn parse_clmm_pool(data: &[u8]) -> Option<ClmmPoolInfo> {
        // Minimum size check - CLMM pools are quite large (1000+ bytes)
        if data.len() < 1200 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("CLMM pool data too short: {} bytes (expected >= 1200)", data.len())
                );
            }
            return None;
        }

        // Skip discriminator (8 bytes)
        let mut offset = 8;

        // Extract bump (1 byte)
        let bump = data[offset];
        offset += 1;

        // Extract amm_config (32 bytes)
        let amm_config = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Extract owner (32 bytes)
        let owner = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Extract token mints
        let token_mint_0 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;
        let token_mint_1 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Extract token vaults
        let token_vault_0 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;
        let token_vault_1 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Extract observation_key (32 bytes)
        let observation_key = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Extract mint decimals
        let mint_decimals_0 = data[offset];
        offset += 1;
        let mint_decimals_1 = data[offset];
        offset += 1;

        // Extract tick_spacing (2 bytes)
        let tick_spacing = Self::extract_u16_at_offset(data, offset)?;
        offset += 2;

        // Extract liquidity (16 bytes, u128)
        let liquidity = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;

        // Extract sqrt_price_x64 (16 bytes, u128)
        let sqrt_price_x64 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;

        // Extract tick_current (4 bytes, i32)
        let tick_current = Self::extract_i32_at_offset(data, offset)?;
        offset += 4;

        // Extract padding fields
        let padding3 = Self::extract_u16_at_offset(data, offset)?;
        offset += 2;
        let padding4 = Self::extract_u16_at_offset(data, offset)?;
        offset += 2;

        // Extract fee growth tracking (2 x 16 bytes)
        let fee_growth_global_0_x64 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;
        let fee_growth_global_1_x64 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;

        // Extract protocol fees (2 x 8 bytes)
        let protocol_fees_token_0 = Self::extract_u64_at_offset(data, offset)?;
        offset += 8;
        let protocol_fees_token_1 = Self::extract_u64_at_offset(data, offset)?;
        offset += 8;

        // Extract swap amounts tracking (4 x 16 bytes)
        let swap_in_amount_token_0 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;
        let swap_out_amount_token_1 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;
        let swap_in_amount_token_1 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;
        let swap_out_amount_token_0 = Self::extract_u128_at_offset(data, offset)?;
        offset += 16;

        // Extract status (1 byte)
        let status = data[offset];
        offset += 1;

        // Extract padding (7 bytes)
        let mut padding = [0u8; 7];
        padding.copy_from_slice(&data[offset..offset + 7]);
        offset += 7;

        // Extract reward_infos (3 rewards, calculate size more carefully)
        let mut reward_infos = Vec::new();
        for i in 0..3 {
            // Calculate available space for this reward
            let remaining_space = data.len() - offset;
            if remaining_space < 100 {
                // Not enough space for a reward, add empty rewards
                reward_infos.push(RewardInfo {
                    reward_state: 0,
                    open_time: 0,
                    end_time: 0,
                    last_update_time: 0,
                    emissions_per_second_x64: 0,
                    reward_total_emissioned: 0,
                    reward_claimed: 0,
                    token_mint: "11111111111111111111111111111111".to_string(),
                    token_vault: "11111111111111111111111111111111".to_string(),
                    authority: "11111111111111111111111111111111".to_string(),
                    reward_growth_global_x64: 0,
                });
                continue;
            }

            if let Some(reward_info) = Self::extract_reward_info_at_offset(data, offset) {
                reward_infos.push(reward_info);
                offset += 176; // Use the calculated size but be more flexible
            } else {
                // If parsing fails, create a default reward and estimate offset
                reward_infos.push(RewardInfo {
                    reward_state: 0,
                    open_time: 0,
                    end_time: 0,
                    last_update_time: 0,
                    emissions_per_second_x64: 0,
                    reward_total_emissioned: 0,
                    reward_claimed: 0,
                    token_mint: "11111111111111111111111111111111".to_string(),
                    token_vault: "11111111111111111111111111111111".to_string(),
                    authority: "11111111111111111111111111111111".to_string(),
                    reward_growth_global_x64: 0,
                });
                offset += 176;
            }
        }

        // After reward parsing, we know from data analysis that:
        // total_fees_token_0 is at offset 1032
        // total_fees_token_1 is at offset 1048
        // recent_epoch is at offset 1088

        // Skip calculating offset through rewards and jump to known positions
        let total_fees_token_0 = Self::extract_u64_at_offset(data, 1032).unwrap_or(0);
        let total_fees_claimed_token_0 = Self::extract_u64_at_offset(data, 1040).unwrap_or(0);
        let total_fees_token_1 = Self::extract_u64_at_offset(data, 1048).unwrap_or(0);
        let total_fees_claimed_token_1 = Self::extract_u64_at_offset(data, 1056).unwrap_or(0);

        // Fund fees should be right after
        let fund_fees_token_0 = Self::extract_u64_at_offset(data, 1064).unwrap_or(0);
        let fund_fees_token_1 = Self::extract_u64_at_offset(data, 1072).unwrap_or(0);

        // Timing information
        let open_time = Self::extract_u64_at_offset(data, 1080).unwrap_or(0);
        let recent_epoch = Self::extract_u64_at_offset(data, 1088).unwrap_or(0);

        // For tick_array_bitmap, it should be before the fees section
        // Based on the structure, it's likely around offset 904 (after rewards)
        let mut tick_array_bitmap = [0u64; 16];
        let bitmap_start = 904; // Estimate based on structure
        for i in 0..16 {
            tick_array_bitmap[i] = Self::extract_u64_at_offset(
                data,
                bitmap_start + i * 8
            ).unwrap_or(0);
        }

        // Skip padding arrays for now (not critical for functionality)
        let padding1 = [0u64; 24];
        let padding2 = [0u64; 32];

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "CLMM complete pool parsed: bump={}, token_mint_0={}, token_mint_1={}, sqrt_price_x64={}, liquidity={}, tick_current={}",
                    bump,
                    token_mint_0,
                    token_mint_1,
                    sqrt_price_x64,
                    liquidity,
                    tick_current
                )
            );
        }

        Some(ClmmPoolInfo {
            bump,
            amm_config,
            owner,
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            observation_key,
            mint_decimals_0,
            mint_decimals_1,
            tick_spacing,
            liquidity,
            sqrt_price_x64,
            tick_current,
            padding3,
            padding4,
            fee_growth_global_0_x64,
            fee_growth_global_1_x64,
            protocol_fees_token_0,
            protocol_fees_token_1,
            swap_in_amount_token_0,
            swap_out_amount_token_1,
            swap_in_amount_token_1,
            swap_out_amount_token_0,
            status,
            padding,
            reward_infos,
            tick_array_bitmap,
            total_fees_token_0,
            total_fees_claimed_token_0,
            total_fees_token_1,
            total_fees_claimed_token_1,
            fund_fees_token_0,
            fund_fees_token_1,
            open_time,
            recent_epoch,
            padding1,
            padding2,
        })
    }

    /// Extract a pubkey from raw data at a fixed offset
    fn extract_pubkey_at_offset(data: &[u8], offset: usize) -> Option<String> {
        if data.len() < offset + 32 {
            return None;
        }

        let pubkey_bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
        let pubkey = Pubkey::new_from_array(pubkey_bytes);
        Some(pubkey.to_string())
    }

    /// Extract a u128 value from raw data at a fixed offset
    fn extract_u128_at_offset(data: &[u8], offset: usize) -> Option<u128> {
        if data.len() < offset + 16 {
            return None;
        }

        let bytes: [u8; 16] = data[offset..offset + 16].try_into().ok()?;
        Some(u128::from_le_bytes(bytes))
    }

    /// Extract a u64 value from raw data at a fixed offset
    fn extract_u64_at_offset(data: &[u8], offset: usize) -> Option<u64> {
        if data.len() < offset + 8 {
            return None;
        }

        let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    }

    /// Extract a u32 value from raw data at a fixed offset
    fn extract_u32_at_offset(data: &[u8], offset: usize) -> Option<u32> {
        if data.len() < offset + 4 {
            return None;
        }

        let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }

    /// Extract an i32 value from raw data at a fixed offset
    fn extract_i32_at_offset(data: &[u8], offset: usize) -> Option<i32> {
        if data.len() < offset + 4 {
            return None;
        }

        let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }

    /// Extract a u16 value from raw data at a fixed offset
    fn extract_u16_at_offset(data: &[u8], offset: usize) -> Option<u16> {
        if data.len() < offset + 2 {
            return None;
        }

        let bytes: [u8; 2] = data[offset..offset + 2].try_into().ok()?;
        Some(u16::from_le_bytes(bytes))
    }

    /// Extract RewardInfo structure from raw data at a fixed offset
    fn extract_reward_info_at_offset(data: &[u8], offset: usize) -> Option<RewardInfo> {
        let mut current_offset = offset;

        if data.len() < current_offset + 176 {
            return None; // Not enough data for a full RewardInfo (176 bytes)
        }

        let reward_state = data[current_offset];
        current_offset += 1;

        // Add 7 bytes padding to align to 8 bytes (Rust struct alignment)
        current_offset += 7;

        let open_time = Self::extract_u64_at_offset(data, current_offset)?;
        current_offset += 8;

        let end_time = Self::extract_u64_at_offset(data, current_offset)?;
        current_offset += 8;

        let last_update_time = Self::extract_u64_at_offset(data, current_offset)?;
        current_offset += 8;

        let emissions_per_second_x64 = Self::extract_u128_at_offset(data, current_offset)?;
        current_offset += 16;

        let reward_total_emissioned = Self::extract_u64_at_offset(data, current_offset)?;
        current_offset += 8;

        let reward_claimed = Self::extract_u64_at_offset(data, current_offset)?;
        current_offset += 8;

        let token_mint = Self::extract_pubkey_at_offset(data, current_offset)?;
        current_offset += 32;

        let token_vault = Self::extract_pubkey_at_offset(data, current_offset)?;
        current_offset += 32;

        let authority = Self::extract_pubkey_at_offset(data, current_offset)?;
        current_offset += 32;

        let reward_growth_global_x64 = Self::extract_u128_at_offset(data, current_offset)?;

        Some(RewardInfo {
            reward_state,
            open_time,
            end_time,
            last_update_time,
            emissions_per_second_x64,
            reward_total_emissioned,
            reward_claimed,
            token_mint,
            token_vault,
            authority,
            reward_growth_global_x64,
        })
    }

    /// Decode token account amount from token account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Token account data too short".to_string());
        }

        // Token account amount is at offset 64 (8 bytes, little-endian)
        let amount_bytes: [u8; 8] = data[64..72]
            .try_into()
            .map_err(|_| "Failed to read amount bytes".to_string())?;

        Ok(u64::from_le_bytes(amount_bytes))
    }
}

/// Raydium CLMM pool information structure
/// Based on complete PoolState from raydium-io/raydium-clmm GitHub
#[derive(Debug, Clone)]
pub struct ClmmPoolInfo {
    // Core pool identification
    pub bump: u8,
    pub amm_config: String,
    pub owner: String,

    // Token pair information
    pub token_mint_0: String,
    pub token_mint_1: String,
    pub token_vault_0: String,
    pub token_vault_1: String,
    pub observation_key: String,

    // Token decimals
    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,

    // Tick and price information
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,

    // Padding fields
    pub padding3: u16,
    pub padding4: u16,

    // Fee growth tracking
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,

    // Protocol fees
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,

    // Swap amounts tracking
    pub swap_in_amount_token_0: u128,
    pub swap_out_amount_token_1: u128,
    pub swap_in_amount_token_1: u128,
    pub swap_out_amount_token_0: u128,

    // Pool status and padding
    pub status: u8,
    pub padding: [u8; 7],

    // Reward information (3 rewards)
    pub reward_infos: Vec<RewardInfo>,

    // Tick array bitmap
    pub tick_array_bitmap: [u64; 16],

    // Fee tracking
    pub total_fees_token_0: u64,
    pub total_fees_claimed_token_0: u64,
    pub total_fees_token_1: u64,
    pub total_fees_claimed_token_1: u64,

    // Fund fees
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,

    // Timing information
    pub open_time: u64,
    pub recent_epoch: u64,

    // Future padding
    pub padding1: [u64; 24],
    pub padding2: [u64; 32],
}

#[derive(Debug, Clone)]
pub struct RewardInfo {
    pub reward_state: u8,
    pub open_time: u64,
    pub end_time: u64,
    pub last_update_time: u64,
    pub emissions_per_second_x64: u128,
    pub reward_total_emissioned: u64,
    pub reward_claimed: u64,
    pub token_mint: String,
    pub token_vault: String,
    pub authority: String,
    pub reward_growth_global_x64: u128,
}

impl RaydiumClmmDecoder {
    /// Extract raw CLMM pool data without price calculations
    /// This is for use by swap modules that need raw pool data
    /// Returns complete PoolState structure with all fields
    pub fn extract_pool_data(accounts: &HashMap<String, AccountData>) -> Option<ClmmPoolInfo> {
        // Find the pool account
        let pool_account = accounts
            .values()
            .find(|acc| { acc.owner.to_string() == RAYDIUM_CLMM_PROGRAM_ID })?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "Extracting complete CLMM pool data from account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse complete CLMM pool structure
        Self::parse_clmm_pool(&pool_account.data)
    }

    /// Get basic trading info for CLMM pools (backward compatibility)
    /// Returns only essential fields needed for trading calculations
    pub fn get_basic_pool_info(pool_data: &ClmmPoolInfo) -> ClmmBasicInfo {
        ClmmBasicInfo {
            token_mint_0: pool_data.token_mint_0.clone(),
            token_mint_1: pool_data.token_mint_1.clone(),
            token_vault_0: pool_data.token_vault_0.clone(),
            token_vault_1: pool_data.token_vault_1.clone(),
            sqrt_price_x64: pool_data.sqrt_price_x64,
            liquidity: pool_data.liquidity,
            tick_current: pool_data.tick_current,
            tick_spacing: pool_data.tick_spacing,
        }
    }
}

/// Basic CLMM pool info for backward compatibility
#[derive(Debug, Clone)]
pub struct ClmmBasicInfo {
    pub token_mint_0: String,
    pub token_mint_1: String,
    pub token_vault_0: String,
    pub token_vault_1: String,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub tick_spacing: u16,
}
