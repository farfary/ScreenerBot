use super::super::utils::{is_sol_mint, WRAPPED_SOL_MINT};
/// FluxBeam AMM decoder
///
/// This decoder handles FluxBeam pools which are Token2022-based AMM pools.
/// FluxBeam is a DEX that pioneers Token2022 standard integration on Solana.
/// Based on analysis of pool structure at 324 bytes with standard AMM vault ratio pricing.
use super::{AccountData, PoolDecoder};
use crate::constants::SOL_DECIMALS;
use crate::logger::{self, LogTag};
use crate::pools::types::{PriceResult, ProgramKind, FLUXBEAM_AMM_PROGRAM_ID};
use crate::tokens::get_cached_decimals;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

pub struct FluxbeamAmmDecoder;

impl PoolDecoder for FluxbeamAmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::FluxbeamAmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::info(LogTag::PoolDecoder, "Starting FluxBeam AMM pool decoding");

        // Find the pool account owned by FluxBeam program
        let pool_account = accounts
            .values()
            .find(|acc| acc.owner.to_string() == FLUXBEAM_AMM_PROGRAM_ID)?;

        logger::info(
            LogTag::PoolDecoder,
            &format!(
                "Found FluxBeam pool account {} with {} bytes",
                pool_account.pubkey,
                pool_account.data.len()
            ),
        );

        // FluxBeam pools are expected to be 324 bytes
        if pool_account.data.len() != 324 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "FluxBeam pool has unexpected size: {} bytes (expected 324)",
                    pool_account.data.len()
                ),
            );
            return None;
        }

        // Parse FluxBeam pool structure
        let pool_info = Self::parse_fluxbeam_pool(&pool_account.data)?;

        logger::info(
            LogTag::PoolDecoder,
            &format!(
                "FluxBeam pool parsed: token_a={}, token_b={}, vault_a={}, vault_b={}",
                pool_info.token_a_mint,
                pool_info.token_b_mint,
                pool_info.token_a_vault,
                pool_info.token_b_vault
            ),
        );

        // Determine which token is SOL and which is the base token
        let (token_mint, sol_vault, token_vault) = if is_sol_mint(&pool_info.token_b_mint) {
            // token_a is the custom token, token_b is SOL
            (
                pool_info.token_a_mint.clone(),
                pool_info.token_b_vault.clone(),
                pool_info.token_a_vault.clone(),
            )
        } else if is_sol_mint(&pool_info.token_a_mint) {
            // token_b is the custom token, token_a is SOL
            (
                pool_info.token_b_mint.clone(),
                pool_info.token_a_vault.clone(),
                pool_info.token_b_vault.clone(),
            )
        } else {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "FluxBeam pool has no SOL token: {} / {}",
                    pool_info.token_a_mint, pool_info.token_b_mint
                ),
            );
            return None;
        };

        // Verify this matches either the requested base or quote mint for bidirectional support
        if token_mint != base_mint && token_mint != quote_mint {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "FluxBeam pool token {} doesn't match requested base {} or quote {}",
                    token_mint, base_mint, quote_mint
                ),
            );
            return None;
        }

        // Get vault balances
        let sol_account = accounts.get(&sol_vault)?;
        let token_account = accounts.get(&token_vault)?;

        let sol_balance = Self::decode_token_account_amount(&sol_account.data).ok()?;
        let token_balance = Self::decode_token_account_amount(&token_account.data).ok()?;

        logger::info(
            LogTag::PoolDecoder,
            &format!(
                "FluxBeam vault balances: SOL={}, token={}",
                sol_balance, token_balance
            ),
        );

        if token_balance == 0 {
            logger::error(LogTag::PoolDecoder, "FluxBeam pool has zero token balance");
            return None;
        }

        // Get token decimals - CRITICAL: must be available, no fallback to defaults
        let token_decimals = match get_cached_decimals(&token_mint) {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "FluxBeam: Token decimals not found for {}, skipping price calculation",
                        token_mint
                    ),
                );
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        logger::info(
            LogTag::PoolDecoder,
            &format!(
                "FluxBeam decimals: token={}, sol={}",
                token_decimals, sol_decimals
            ),
        );

        // FluxBeam uses standard AMM constant product formula for pricing
        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        let sol_adjusted = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        // Validate price is reasonable
        if price_sol <= 0.0 || price_sol > 1_000_000.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("FluxBeam: Invalid price calculated: {:.12} SOL", price_sol),
            );
            return None;
        }

        logger::info(
            LogTag::PoolDecoder,
            &format!(
                "FluxBeam price calculation: {:.12} SOL per token (sol_reserves={:.6}, token_reserves={:.6})",
                price_sol,
                sol_adjusted,
                token_adjusted
            ),
        );

        Some(PriceResult {
            mint: token_mint,
            price_usd: 0.0, // We don't calculate USD prices, only SOL
            price_sol,
            sol_reserves: sol_adjusted,
            token_reserves: token_adjusted,
            confidence: 0.9,
            source_pool: Some("FLUXBEAM_AMM".to_string()),
            pool_address: pool_account.pubkey.to_string(),
            slot: 0, // Will be updated by the system
            timestamp: Instant::now(),
        })
    }
}

impl FluxbeamAmmDecoder {
    /// Extract reserve account addresses from FluxBeam pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [token_a_vault, token_b_vault]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() != 324 {
            return None;
        }

        // Extract vault pubkeys at correct offsets (derived from on-chain struct layout)
        // token_a vault @ 35, token_b vault @ 67
        let token_a_vault = Self::extract_pubkey_at_offset(pool_data, 35)?;
        let token_b_vault = Self::extract_pubkey_at_offset(pool_data, 67)?;

        Some(vec![token_a_vault, token_b_vault])
    }

    /// Parse FluxBeam pool account data to extract token mints and vault addresses
    /// Based on empirical analysis of catwifhat/SOL pool structure (324 bytes)
    ///
    /// FluxBeam Pool structure (confirmed offsets):
    /// - Offset 35:  Token A vault (32 bytes)
    /// - Offset 67:  Token B vault (32 bytes)
    /// - Offset 99:  Pool LP mint (32 bytes)
    /// - Offset 131: Token A mint (32 bytes)
    /// - Offset 163: Token B mint (32 bytes)
    /// - Offset 195: Pool fee account (32 bytes)
    pub fn parse_fluxbeam_pool(data: &[u8]) -> Option<FluxbeamPoolInfo> {
        if data.len() != 324 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "FluxBeam pool data wrong size: {} bytes: (expected 324)",
                    data.len()
                ),
            );
            return None;
        }

        // Extract token mints at confirmed offsets
        let token_a_mint = Self::extract_pubkey_at_offset(data, 131)?; // BERN mint
        let token_b_mint = Self::extract_pubkey_at_offset(data, 163)?; // SOL mint

        // Extract vault addresses at confirmed offsets
        let token_a_vault = Self::extract_pubkey_at_offset(data, 35)?;
        let token_b_vault = Self::extract_pubkey_at_offset(data, 67)?;

        logger::info(
            LogTag::PoolDecoder,
            &format!(
                "FluxBeam parsed: token_a@131={}, token_b@163={}, vault_a@35={}, vault_b@67={}",
                token_a_mint, token_b_mint, token_a_vault, token_b_vault
            ),
        );

        Some(FluxbeamPoolInfo {
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
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

/// FluxBeam pool information structure
#[derive(Debug, Clone)]
pub struct FluxbeamPoolInfo {
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_vault: String,
    pub token_b_vault: String,
}
