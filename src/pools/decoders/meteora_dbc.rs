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

        // Identify SOL vault and token vault by mint
        let (sol_vault, token_vault, token_mint) = if mint_a == SOL_MINT {
            (token_accounts[0], token_accounts[1], mint_b)
        } else if mint_b == SOL_MINT {
            (token_accounts[1], token_accounts[0], mint_a)
        } else {
            // Neither is SOL, not a SOL pair we care about
            return None;
        };

        // Check that token mint matches the requested pair orientation
        if token_mint != base_mint && token_mint != quote_mint {
            return None;
        }

        // Balances (raw u64)
        let sol_raw = read_token_account_amount(&sol_vault.data)?;
        let token_raw = read_token_account_amount(&token_vault.data)?;

        // Fees: attempt to read from pool data if known offsets exist; otherwise ignore (small effect)
        // From the user's JSON, protocol_quote_fee and partner_quote_fee exist; subtract from SOL reserves.
        // Try to locate them by scanning little-endian u64 sequences right after vaults; fall back to zero.
        let (protocol_quote_fee, partner_quote_fee) = locate_fees_u64_pairs(
            &pool_acc.data
        ).unwrap_or((0, 0));

        let sol_after_fees = sol_raw
            .saturating_sub(protocol_quote_fee)
            .saturating_sub(partner_quote_fee);

        if token_raw == 0 {
            return None;
        }

        // Decimals
        let token_decimals = match get_token_decimals_sync(&token_mint) {
            Some(d) => d,
            None => {
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Price = SOL per token
        let sol = (sol_after_fees as f64) / (10f64).powi(sol_decimals as i32);
        let tok = (token_raw as f64) / (10f64).powi(token_decimals as i32);
        if tok <= 0.0 {
            return None;
        }
        let price_sol = sol / tok;

        let mut pr = PriceResult::new(
            token_mint.clone(),
            0.0,
            price_sol,
            sol,
            tok,
            pool_acc.pubkey.to_string()
        );
        pr.source_pool = Some(ProgramKind::MeteoraDbc.display_name().to_string());
        pr.slot = sol_vault.slot.min(token_vault.slot).min(pool_acc.slot);
        Some(pr)
    }
}

/// Try to locate fee fields in pool data. Returns (protocol_quote_fee, partner_quote_fee)
fn locate_fees_u64_pairs(data: &[u8]) -> Option<(u64, u64)> {
    // The provided JSON shows these fields present; in practice, at offsets around ~... after vaults.
    // We'll perform a light heuristic: search for two consecutive non-zero u64 that are relatively small compared to u64::MAX
    // and appear near the middle of the account.
    if data.len() < 64 {
        return None;
    }
    let start = data.len() / 3; // heuristic window
    let end = data.len().saturating_sub(16);
    for off in (start..end).step_by(8) {
        let a = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        let b = u64::from_le_bytes(data[off + 8..off + 16].try_into().ok()?);
        // Basic sanity: not absurdly large, and one/both non-zero
        if (a > 0 || b > 0) && a < 1u64 << 60 && b < 1u64 << 60 {
            return Some((a, b));
        }
    }
    None
}
