#![allow(warnings)]
use crate::prelude::*;

use anyhow::{ anyhow, Result };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::account::Account;

pub fn decode_pumpfun2_pool(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 73 {
        return Err(anyhow!("PumpFun2 account {} too short: {} B (< 73)", pool_pk, acct.data.len()));
    }

    // skip Anchor discriminator (8 bytes)
    let data = &acct.data[8..];

    let real_token_reserves = u64::from_le_bytes(data[16..24].try_into()?);
    let real_sol_reserves = u64::from_le_bytes(data[24..32].try_into()?);

    // The pool doesn’t store its token-mint; return default / wrapped-SOL.
    let base_mint = Pubkey::default();
    let quote_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").expect(
        "hard-coded So111… pubkey"
    );

    Ok((real_token_reserves, real_sol_reserves, base_mint, quote_mint))
}

pub fn decode_pumpfun2_pool_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_pumpfun2_pool, but account is already provided
    decode_pumpfun2_pool(rpc, pool_pk, acct)
}
