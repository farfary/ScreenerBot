//! Raydium AMM V4  (program RVKd6…)
use anyhow::{ anyhow, Result };
use num_format::{ Locale, ToFormattedString };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

pub fn decode_raydium_amm(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 264 {
        return Err(anyhow!("AMM account too short"));
    }

    // Extract mint addresses from pool account
    let base_mint = Pubkey::new_from_array(acct.data[168..200].try_into()?);
    let quote_mint = Pubkey::new_from_array(acct.data[216..248].try_into()?);

    let base_vault = Pubkey::new_from_array(acct.data[200..232].try_into()?);
    let quote_vault = Pubkey::new_from_array(acct.data[232..264].try_into()?);
    let base = rpc.get_token_account_balance(&base_vault)?.amount.parse::<u64>().unwrap_or(0);
    let quote = rpc.get_token_account_balance(&quote_vault)?.amount.parse::<u64>().unwrap_or(0);

    println!(
        "✅ Raydium AMM   → Base: {} | Quote: {}",
        base.to_formatted_string(&Locale::en),
        quote.to_formatted_string(&Locale::en)
    );
    Ok((base, quote, base_mint, quote_mint))
}

/// Batch-friendly version: decode from already-fetched account
pub fn decode_raydium_amm_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_raydium_amm, but account is already provided
    decode_raydium_amm(rpc, pool_pk, acct)
}
