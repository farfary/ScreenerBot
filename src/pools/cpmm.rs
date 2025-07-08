#![allow(warnings)]
use crate::prelude::*;

use anyhow::Result;
use num_format::{ Locale, ToFormattedString };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

pub fn decode_cpmm(rpc: &RpcClient, pool_pk: &Pubkey) -> Result<(u64, u64, Pubkey, Pubkey)> {
    let acct = rpc.get_account(pool_pk)?;
    decode_cpmm_from_account(rpc, pool_pk, &acct)
}

/// Batch-friendly version: decode from already-fetched account
pub fn decode_cpmm_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 72 {
        println!("⚠️  CPMM account too short");
        return Ok((0, 0, Pubkey::default(), Pubkey::default()));
    }

    let vault_a = Pubkey::new_from_array(acct.data[8..40].try_into()?);
    let vault_b = Pubkey::new_from_array(acct.data[40..72].try_into()?);

    // Get the mint addresses from the vault accounts
    let mint_a = get_token_account_mint(rpc, &vault_a)?;
    let mint_b = get_token_account_mint(rpc, &vault_b)?;

    let a = rpc.get_token_account_balance(&vault_a)?.amount.parse::<u64>()?;
    let b = rpc.get_token_account_balance(&vault_b)?.amount.parse::<u64>()?;

    println!(
        "✅ CPMM pool     → A: {} | B: {}",
        a.to_formatted_string(&Locale::en),
        b.to_formatted_string(&Locale::en)
    );
    Ok((a, b, mint_a, mint_b))
}
