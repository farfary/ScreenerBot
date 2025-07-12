#![allow(warnings)]
use crate::prelude::*;

use anyhow::{ anyhow, Result };
use borsh::BorshDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::account::Account;

#[derive(Debug, BorshDeserialize, Clone)]
pub struct PumpFun2Pool {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator: Pubkey,
}

pub fn decode_pumpfun2_pool(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Check minimum size for the full structure
    // 8 bytes discriminator + 8 + 8 + 8 + 8 + 8 + 1 + 32 = 81 bytes minimum
    if acct.data.len() < 81 {
        return Err(anyhow!("PumpFun2 account {} too short: {} B (< 81)", pool_pk, acct.data.len()));
    }

    // Skip Anchor discriminator (8 bytes) and deserialize the pool data
    let pool: PumpFun2Pool = PumpFun2Pool::try_from_slice(&acct.data[8..])?;

    // Use real reserves for actual trading calculations
    // Virtual reserves might be used for bonding curve calculations
    let base_reserves = pool.real_token_reserves;
    let quote_reserves = pool.real_sol_reserves;

    // The pool doesn't store its token-mint explicitly;
    // For PumpFun2, we typically need to derive it or get it from context
    // For now, return default for base_mint and wrapped SOL for quote
    let base_mint = Pubkey::default(); // This should be the actual token mint
    let quote_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").expect(
        "hard-coded wrapped SOL pubkey"
    );

    Ok((base_reserves, quote_reserves, base_mint, quote_mint))
}

pub fn decode_pumpfun2_pool_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_pumpfun2_pool, but account is already provided
    decode_pumpfun2_pool(rpc, pool_pk, acct)
}

/// Get the full PumpFun2Pool structure for advanced operations
pub fn decode_pumpfun2_pool_full(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<PumpFun2Pool> {
    // Check minimum size for the full structure
    if acct.data.len() < 81 {
        return Err(anyhow!("PumpFun2 account {} too short: {} B (< 81)", pool_pk, acct.data.len()));
    }

    // Skip Anchor discriminator (8 bytes) and deserialize the pool data
    let pool: PumpFun2Pool = PumpFun2Pool::try_from_slice(&acct.data[8..])?;
    Ok(pool)
}
