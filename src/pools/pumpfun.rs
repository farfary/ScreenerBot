#![allow(warnings)]
use crate::prelude::*;

use anyhow::{ anyhow, Result };
use borsh::BorshDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

#[derive(Debug, BorshDeserialize, Clone)]
pub struct PumpFunPool {
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
}

pub fn decode_pumpfun_pool(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 211 {
        return Err(anyhow!("Pump.fun account only {} B (<211)", acct.data.len()));
    }
    let pool: PumpFunPool = PumpFunPool::try_from_slice(&acct.data[8..211])?;

    // Vault balances are always at 0 offset in each account
    let base_acct = rpc.get_account(&pool.pool_base_token_account)?;
    let quote_acct = rpc.get_account(&pool.pool_quote_token_account)?;
    let base = u64::from_le_bytes(base_acct.data[64..72].try_into()?);
    let quote = u64::from_le_bytes(quote_acct.data[64..72].try_into()?);

    Ok((base, quote, pool.base_mint, pool.quote_mint))
}

pub fn decode_pumpfun_pool_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_pumpfun_pool, but account is already provided
    decode_pumpfun_pool(rpc, pool_pk, acct)
}
