// pool_raydium_launchpad.rs

use anyhow::{ anyhow, Result };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

pub fn decode_raydium_launchpad(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Known pubkey fields: base_mint, quote_mint, base_vault, quote_vault
    // Typical Anchor padding is 208, so test at 208, fallback to 205

    let data = &acct.data;

    // Try at offset 208 (most Anchor structs are 8-byte aligned)
    let base_mint_off = if data.len() >= 208 + 32 * 4 {
        208
    } else if data.len() >= 205 + 32 * 4 {
        205
    } else {
        return Err(anyhow!(
            "Launchpad account too short: got {} bytes, expected at least 205 + 128",
            data.len()
        ));
    };

    let base_mint = Pubkey::new_from_array(data[base_mint_off..base_mint_off + 32].try_into()?);
    let quote_mint = Pubkey::new_from_array(data[base_mint_off + 32..base_mint_off + 64].try_into()?);
    let base_vault = Pubkey::new_from_array(data[base_mint_off + 64..base_mint_off + 96].try_into()?);
    let quote_vault = Pubkey::new_from_array(data[base_mint_off + 96..base_mint_off + 128].try_into()?);

    let base = rpc.get_token_account_balance(&base_vault)?.amount.parse::<u64>().unwrap_or(0);
    let quote = rpc.get_token_account_balance(&quote_vault)?.amount.parse::<u64>().unwrap_or(0);

    println!(
        "✅ Raydium Launchpad   → Base: {} | Quote: {}",
        base, quote
    );
    Ok((base, quote, base_mint, quote_mint))
}


// Optional: batch-friendly version
pub fn decode_raydium_launchpad_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    decode_raydium_launchpad(rpc, pool_pk, acct)
}
