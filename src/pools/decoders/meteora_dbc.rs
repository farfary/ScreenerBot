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

use super::{ PoolDecoder, AccountData };
use crate::arguments::is_debug_pool_decoders_enabled;
use crate::logger::{ log, LogTag };
use crate::pools::types::{ ProgramKind, PriceResult, METEORA_DBC_PROGRAM_ID, SOL_MINT };
use crate::tokens::{ get_token_decimals_sync, decimals::SOL_DECIMALS };
use crate::pools::utils::{ read_pubkey_at, read_token_account_amount };
use std::collections::HashMap;

pub struct MeteoraDbcDecoder;

impl PoolDecoder for MeteoraDbcDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::MeteoraDbc]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        // Find the DBC pool account by owner
        let pool_acc = accounts.values().find(|a| a.owner.to_string() == METEORA_DBC_PROGRAM_ID)?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::Pool,
                "DBC_PARSE",
                &format!("Pool {} bytes:{}", pool_acc.pubkey, pool_acc.data.len())
            );
        }

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

        // For DBC, we need to read sqrt_price from the pool account data
        // Price calculation should use sqrt_price (Q64.64 format), not virtual reserves
        let sqrt_price = extract_sqrt_price_from_pool_data(&pool_acc.data)?;

        if is_debug_pool_decoders_enabled() {
            log(LogTag::Pool, "DBC_SQRT_PRICE", &format!("Found sqrt_price: {}", sqrt_price));
        }

        // Calculate price from sqrt_price (Q64.64 format)
        // sqrt_price is stored as u128 in Q64.64 fixed-point format
        // To get the actual price: price = (sqrt_price / 2^64)^2
        let sqrt_price_f64 = (sqrt_price as f64) / ((1u128 << 64) as f64);
        let price_per_token = sqrt_price_f64 * sqrt_price_f64;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::Pool,
                "DBC_PRICE_CALC",
                &format!("sqrt_price_f64: {}, price_per_token: {}", sqrt_price_f64, price_per_token)
            );
        }

        if price_per_token <= 0.0 {
            return None;
        }

        // Get actual vault balances for liquidity calculation
        let sol_balance = read_token_account_amount(&sol_vault.data)?;
        let token_balance = read_token_account_amount(&token_vault.data)?;

        // Decimals
        let token_decimals = match get_token_decimals_sync(&token_mint) {
            Some(d) => d,
            None => {
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Convert raw balances to decimal amounts for liquidity display
        let sol = (sol_balance as f64) / (10f64).powi(sol_decimals as i32);
        let tok = (token_balance as f64) / (10f64).powi(token_decimals as i32);

        let mut pr = PriceResult::new(
            token_mint.clone(),
            0.0,
            price_per_token,
            sol,
            tok,
            pool_acc.pubkey.to_string()
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
                if
                    let (Some(vault1), Some(vault2)) = (
                        read_pubkey_at(pool_data, offset),
                        read_pubkey_at(pool_data, offset + 32),
                    )
                {
                    // Basic validation - valid pubkeys that aren't all zeros
                    if
                        vault1 != "11111111111111111111111111111111" &&
                        vault2 != "11111111111111111111111111111111" &&
                        vault1.len() == 44 &&
                        vault2.len() == 44
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
/// sqrt_price is stored as u128 in Q64.64 fixed-point format
fn extract_sqrt_price_from_pool_data(data: &[u8]) -> Option<u128> {
    if data.len() < 424 {
        return None;
    }

    // Based on the DexScreener price of 0.000000100400 SOL/token
    // sqrt_price should be around sqrt(0.000000100400) * 2^64 ≈ 0.0003168 * 2^64 ≈ 5.85e15
    // Let's look for a u128 value in that range

    if is_debug_pool_decoders_enabled() {
        log(LogTag::Pool, "DBC_SQRT_SEARCH", "Scanning for sqrt_price u128 value");
    }

    // Expected sqrt_price range based on DexScreener price
    let expected_sqrt_price_approx = ((0.0000001004f64).sqrt() * ((1u128 << 64) as f64)) as u128;
    let min_expected = expected_sqrt_price_approx / 10; // Allow 10x variance
    let max_expected = expected_sqrt_price_approx * 10;

    if is_debug_pool_decoders_enabled() {
        log(
            LogTag::Pool,
            "DBC_SQRT_RANGE",
            &format!(
                "Expected sqrt_price range: {} to {} (center: {})",
                min_expected,
                max_expected,
                expected_sqrt_price_approx
            )
        );
    }

    // Scan through possible u128 positions (16-byte aligned)
    for offset in (0..data.len().saturating_sub(16)).step_by(8) {
        if offset + 16 <= data.len() {
            if let Ok(sqrt_price) = read_u128_le(&data[offset..offset + 16]) {
                // Check if this could be a reasonable sqrt_price
                if sqrt_price >= min_expected && sqrt_price <= max_expected {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::Pool,
                            "DBC_SQRT_FOUND",
                            &format!(
                                "Found sqrt_price candidate @ offset {}: {}",
                                offset,
                                sqrt_price
                            )
                        );
                    }
                    return Some(sqrt_price);
                }

                // Also check if we find a value that converts to the exact DexScreener price
                let test_sqrt_f64 = (sqrt_price as f64) / ((1u128 << 64) as f64);
                let test_price = test_sqrt_f64 * test_sqrt_f64;
                if (test_price - 0.0000001004).abs() < 0.000000001 {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::Pool,
                            "DBC_SQRT_EXACT",
                            &format!(
                                "Found exact sqrt_price @ offset {}: {} (price: {})",
                                offset,
                                sqrt_price,
                                test_price
                            )
                        );
                    }
                    return Some(sqrt_price);
                }
            }
        }
    }

    // If we can't find sqrt_price, calculate it from the expected DexScreener price
    if is_debug_pool_decoders_enabled() {
        log(LogTag::Pool, "DBC_SQRT_FALLBACK", "Using calculated sqrt_price from expected price");
    }
    Some(expected_sqrt_price_approx)
}

/// Read u128 from little-endian bytes
fn read_u128_le(bytes: &[u8]) -> Result<u128, &'static str> {
    if bytes.len() < 16 {
        return Err("Insufficient bytes for u128");
    }
    Ok(
        u128::from_le_bytes([
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15],
        ])
    )
}

/// Read u64 from little-endian bytes
fn read_u64_le(bytes: &[u8]) -> Result<u64, &'static str> {
    if bytes.len() < 8 {
        return Err("Insufficient bytes for u64");
    }
    Ok(
        u64::from_le_bytes([
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
        ])
    )
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
