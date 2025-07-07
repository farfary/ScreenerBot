//! Raydium CPMM pool decoder
//! Program id: CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C
//!
/// Layout (32-byte Pubkeys, little-endian u64 counters)
///  0 ..  32  amm_config
/// 32 ..  64  pool_creator
/// 64 ..  96  token_0_vault
/// 96 .. 128  token_1_vault
///128 .. 160  lp_mint
///160 .. 192  token_0_mint
///192 .. 224  token_1_mint
///
/// *Reserves are **not** stored in the pool; we read vault balances.*

use anyhow::{ anyhow, Result };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;

/// Returns `(reserve_token0, reserve_token1, token0_mint, token1_mint)`
pub fn decode_raydium_cpmm(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 224 {
        return Err(anyhow!("CPMM account too short: {}", acct.data.len()));
    }

    let token0_vault = Pubkey::new_from_array(acct.data[64..96].try_into()?);
    let token1_vault = Pubkey::new_from_array(acct.data[96..128].try_into()?);

    let token0_mint = Pubkey::new_from_array(acct.data[160..192].try_into()?);
    let token1_mint = Pubkey::new_from_array(acct.data[192..224].try_into()?);

    // use vault token-account balances as reserves
    let reserve0 = rpc
        .get_token_account_balance(&token0_vault)
        .map(|b| b.amount.parse::<u64>().unwrap_or(0))?;
    let reserve1 = rpc
        .get_token_account_balance(&token1_vault)
        .map(|b| b.amount.parse::<u64>().unwrap_or(0))?;

    Ok((reserve0, reserve1, token0_mint, token1_mint))
}

/// Batch-friendly version: decode from already-fetched account
pub fn decode_raydium_cpmm_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_raydium_cpmm, but account is already provided
    decode_raydium_cpmm(rpc, pool_pk, acct)
}
