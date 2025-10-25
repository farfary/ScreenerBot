/// Meteora Dynamic Bonding Curve (DBC) decoder
///
/// Program ID: METEORA_DBC_PROGRAM_ID (dbcij3LW...)
///
/// The DBC pool account stores:
/// - base_mint (Pubkey)
/// - base_vault (Pubkey)
/// - quote_vault (Pubkey)
/// - base_reserve (u64)
/// - quote_reserve (u64)
/// - protocol_base_fee (u64)
/// - protocol_quote_fee (u64)
/// - partner_base_fee (u64)
/// - partner_quote_fee (u64)
/// - sqrt_price (u128)
/// ... plus metadata. We compute price from live vault balances minus fees.
use super::{AccountData, PoolDecoder};
use crate::constants::{METEORA_DBC_PROGRAM_ID, SOL_DECIMALS, SOL_MINT};
use crate::logger::{self, LogTag};
use crate::pools::types::{PriceResult, ProgramKind};
use crate::pools::utils::{read_pubkey_at, read_token_account_amount};
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;

pub struct MeteoraDbcDecoder;

impl PoolDecoder for MeteoraDbcDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::MeteoraDbc]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        // Find the DBC pool account by owner
        let pool_acc = accounts
            .values()
            .find(|a| a.owner.to_string() == METEORA_DBC_PROGRAM_ID)?;

        logger::debug(
            LogTag::Pool,
            &format!("Pool {} bytes:{}", pool_acc.pubkey, pool_acc.data.len()),
        );

        // Parse key fields from the provided on-chain JSON (stable offsets not guaranteed across versions),
        // but the vault accounts themselves are SPL Token accounts; we can rely on them for balances.
        // We'll try common offsets observed empirically:
        // base_mint @ 32*? Unknown; instead, detect SOL orientation using vault mints directly.

        // Heuristic: scan all accounts to find two SPL token accounts owned by Token program that match pool's vaults.
        // Simpler: attempt to read first 32 bytes of token accounts for mint.
        // We need to identify which vault holds SOL (wrapped) vs the token.

        // Find two candidate token accounts (exclude the pool account itself)
        let mut token_accounts: Vec<&AccountData> = accounts
            .values()
            .filter(|a| a.pubkey != pool_acc.pubkey && a.data.len() >= 72) // SPL token acc length >= 72
            .collect();

        if token_accounts.len() < 2 {
            return None;
        }

        // Read mint pubkeys of first two token accounts
        let mint_a = read_pubkey_at(&token_accounts[0].data, 0).unwrap_or_default();
        let mint_b = read_pubkey_at(&token_accounts[1].data, 0).unwrap_or_default();

        // Store for later use before moving
        let mint_a_clone = mint_a.clone();
        let mint_b_clone = mint_b.clone();

        // Identify SOL vault and token vault by mint
        let (sol_vault, token_vault, token_mint) = if mint_a == SOL_MINT {
            (token_accounts[0], token_accounts[1], mint_b)
        } else if mint_b == SOL_MINT {
            (token_accounts[1], token_accounts[0], mint_a)
        } else {
            // Neither is SOL, not a SOL pair we care about
            return None;
        };

        // For DBC, read sqrt_price from the pool account data at the known offset
        // Layout (bytes) with Anchor discriminator (8 bytes) at start:
        // 0..8    Anchor discriminator
        // 8..72   VolatilityTracker
        // 64..96  config (Pubkey)
        // 96..128 creator (Pubkey)
        // 128..160 base_mint (Pubkey)
        // 160..192 base_vault (Pubkey)
        // 192..224 quote_vault (Pubkey)
        // 224..232 base_reserve (u64)
        // 232..240 quote_reserve (u64)
        // 240..248 protocol_base_fee (u64)
        // 248..256 protocol_quote_fee (u64)
        // 256..264 partner_base_fee (u64)
        // 264..272 partner_quote_fee (u64)
        // 272..288 sqrt_price (u128, Q64.64) — without discriminator
        // With discriminator, sqrt_price sits at 280..296
        let sqrt_price = extract_sqrt_price_from_pool_data(&pool_acc.data)?;

        // Decimals
        let token_decimals = match get_cached_decimals(&token_mint) {
            Some(d) => d,
            None => {
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Calculate price from sqrt_price (Q64.64 format)
        // sqrt_price is stored as u128 in Q64.64 fixed-point format
        // Raw ratio (quote/base in raw units) = (sqrt_price / 2^64)^2
        // Convert to human units: divide by 10^(quote_decimals - base_decimals)
        let sqrt_price_f64 = (sqrt_price as f64) / ((1u128 << 64) as f64);
        let raw_ratio = sqrt_price_f64 * sqrt_price_f64;
        let decimals_scale = (10f64).powi((sol_decimals as i32) - (token_decimals as i32));
        let price_per_token = raw_ratio / decimals_scale;

        logger::debug(
            LogTag::Pool,
            &format!(
                "sqrt_price_f64: {}, raw_ratio: {}, decimals_scale: {}, price_per_token: {}",
                sqrt_price_f64, raw_ratio, decimals_scale, price_per_token
            ),
        );

        if price_per_token <= 0.0 || !price_per_token.is_finite() {
            return None;
        }

        // Get actual vault balances for liquidity calculation
        let sol_balance = read_token_account_amount(&sol_vault.data)?;
        let token_balance = read_token_account_amount(&token_vault.data)?;

        // Convert raw balances to decimal amounts for liquidity display
        let sol = (sol_balance as f64) / (10f64).powi(sol_decimals as i32);
        let tok = (token_balance as f64) / (10f64).powi(token_decimals as i32);

        let mut pr = PriceResult::new(
            token_mint.clone(),
            0.0,
            price_per_token,
            sol,
            tok,
            pool_acc.pubkey.to_string(),
        );
        pr.source_pool = Some(ProgramKind::MeteoraDbc.display_name().to_string());
        pr.slot = sol_vault.slot.min(token_vault.slot).min(pool_acc.slot);
        Some(pr)
    }
}

impl MeteoraDbcDecoder {
    /// Extract reserve account addresses from DBC pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [base_vault, quote_vault]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() < 424 {
            return None;
        }

        // DBC pools store vault addresses after the mints
        // Try to extract from known potential offsets
        // Look for two consecutive pubkeys that look like vault addresses
        for offset in (32..200).step_by(32) {
            if offset + 64 <= pool_data.len() {
                if let (Some(vault1), Some(vault2)) = (
                    read_pubkey_at(pool_data, offset),
                    read_pubkey_at(pool_data, offset + 32),
                ) {
                    // Basic validation - valid pubkeys that aren't all zeros
                    if vault1 != "11111111111111111111111111111111"
                        && vault2 != "11111111111111111111111111111111"
                        && vault1.len() == 44
                        && vault2.len() == 44
                    {
                        return Some(vec![vault1, vault2]);
                    }
                }
            }
        }

        None
    }
}

/// Extract sqrt_price from pool account data
/// sqrt_price is stored as u128 in Q64.64 fixed-point format.
fn extract_sqrt_price_from_pool_data(data: &[u8]) -> Option<u128> {
    // Primary: read from known offset (Anchor discriminator +8 bytes)
    if data.len() >= 296 {
        if let Ok(val) = read_u128_le(&data[280..296]) {
            logger::debug(LogTag::Pool, &format!("Found sqrt_price: {}", val));
            return Some(val);
        }
    }

    // Fallback: scan entire account for any plausible u128 (very permissive)
    logger::debug(
        LogTag::Pool,
        "Scanning for sqrt_price u128 value (fallback)",
    );
    for offset in (0..data.len().saturating_sub(16)).step_by(8) {
        if let Ok(val) = read_u128_le(&data[offset..offset + 16]) {
            // Basic plausibility: value should be non-zero and < 2^80 (to avoid random big numbers)
            if val > 0 && val < 1u128 << 80 {
                logger::debug(
                    LogTag::Pool,
                    &format!("Candidate sqrt_price @{}: {}", offset, val),
                );
                return Some(val);
            }
        }
    }
    None
}

/// Read u128 from little-endian bytes
fn read_u128_le(bytes: &[u8]) -> Result<u128, &'static str> {
    if bytes.len() < 16 {
        return Err("Insufficient bytes for u128");
    }
    Ok(u128::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]))
}

/// Read u64 from little-endian bytes
fn read_u64_le(bytes: &[u8]) -> Result<u64, &'static str> {
    if bytes.len() < 8 {
        return Err("Insufficient bytes for u64");
    }
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Try to locate fee fields in pool data. Returns (protocol_quote_fee, partner_quote_fee)
fn locate_fees_u64_pairs(data: &[u8]) -> Option<(u64, u64)> {
    // From the user's JSON: protocol_quote_fee: 3509385, partner_quote_fee: 0
    // Look for consecutive u64 values that are reasonable fee amounts (< 1 billion raw units)
    if data.len() < 64 {
        return None;
    }

    // Scan for reasonable fee pairs - fees should be much smaller than reserves
    for off in (100..data.len().saturating_sub(16)).step_by(8) {
        if let Ok(bytes1) = data[off..off + 8].try_into() {
            if let Ok(bytes2) = data[off + 8..off + 16].try_into() {
                let a = u64::from_le_bytes(bytes1);
                let b = u64::from_le_bytes(bytes2);

                // Look for fee-like values: non-zero but < 100M (reasonable for quote fees)
                // and the pair where one might be zero (partner fees often zero)
                if a > 0 && a < 100_000_000 && b < 100_000_000 {
                    return Some((a, b));
                }
                // Also try the reverse pattern
                if b > 0 && b < 100_000_000 && a < 100_000_000 {
                    return Some((b, a));
                }
            }
        }
    }

    // Fallback: no fees
    Some((0, 0))
}
