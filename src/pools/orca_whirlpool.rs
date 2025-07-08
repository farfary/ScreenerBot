//! Orca Whirlpool (program whirLb…)

use anyhow::{ anyhow, Result };
use num_format::{ Locale, ToFormattedString };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

pub fn decode_orca_whirlpool(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 211 {
        return Err(anyhow!("Pump.fun account only {} B (<211)", acct.data.len()));
    }

    // Extract mint addresses from pool account
    let mint_a = Pubkey::new_from_array(acct.data[8..40].try_into()?);
    let mint_b = Pubkey::new_from_array(acct.data[40..72].try_into()?);

    // Extract vault addresses (different offsets than mints)
    let vault_a = Pubkey::new_from_array(acct.data[72..104].try_into()?);
    let vault_b = Pubkey::new_from_array(acct.data[104..136].try_into()?);
    let a = rpc.get_token_account_balance(&vault_a)?.amount.parse::<u64>().unwrap_or(0);
    let b = rpc.get_token_account_balance(&vault_b)?.amount.parse::<u64>().unwrap_or(0);

    println!(
        "✅ Orca Whirlpool → A: {} | B: {}",
        a.to_formatted_string(&Locale::en),
        b.to_formatted_string(&Locale::en)
    );
    Ok((a, b, mint_a, mint_b))
}

/// Batch-friendly version: decode from already-fetched account
pub fn decode_orca_whirlpool_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_orca_whirlpool, but account is already provided
    decode_orca_whirlpool(rpc, pool_pk, acct)
}
